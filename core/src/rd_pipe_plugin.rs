use core::slice;
use std::{
    io::{self, ErrorKind::WouldBlock},
    sync::Arc,
};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    runtime::{Builder, Runtime},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace, warn};
use windows::{
    core::{implement, AgileReference, Error, Interface, Result},
    Win32::{
        Foundation::{BOOL, BSTR, E_UNEXPECTED},
        System::RemoteDesktop::{
            IWTSListener, IWTSListenerCallback, IWTSListenerCallback_Impl, IWTSPlugin,
            IWTSPlugin_Impl, IWTSVirtualChannel, IWTSVirtualChannelCallback,
            IWTSVirtualChannelCallback_Impl, IWTSVirtualChannelManager,
        },
    },
};

#[derive(Debug)]
#[implement(IWTSPlugin)]
pub struct RdPipePlugin {
    async_runtime: Arc<Runtime>,
}

impl RdPipePlugin {
    #[instrument]
    pub fn new() -> RdPipePlugin {
        trace!("Constructing runtime");
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
        trace!("Constructing plugin");
        RdPipePlugin {
            async_runtime: Arc::new(runtime),
        }
    }

    #[instrument]
    fn create_listener(
        &self,
        channel_mgr: &IWTSVirtualChannelManager,
        channel_name: &str,
    ) -> Result<IWTSListener> {
        debug!("Creating listener with name {}", channel_name);
        let callback: IWTSListenerCallback =
            RdPipeListenerCallback::new(channel_name, self.async_runtime.clone()).into();
        unsafe {
            channel_mgr.CreateListener(&*format!("{}\0", channel_name).as_ptr(), 0, &callback)
        }
    }
}

impl IWTSPlugin_Impl for RdPipePlugin {
    #[instrument]
    fn Initialize(&self, pchannelmgr: &Option<IWTSVirtualChannelManager>) -> Result<()> {
        let channel_mgr = match pchannelmgr {
            Some(m) => m,
            None => {
                error!("No pchannelmgr given when initializing");
                return Err(Error::from(E_UNEXPECTED));
            }
        };
        self.create_listener(channel_mgr, "UnicornDVC")?;
        Ok(())
    }

    #[instrument]
    fn Connected(&self) -> Result<()> {
        info!("Client connected");
        Ok(())
    }

    #[instrument]
    fn Disconnected(&self, dwdisconnectcode: u32) -> Result<()> {
        info!("Client connected with {}", dwdisconnectcode);
        Ok(())
    }

    #[instrument]
    fn Terminated(&self) -> Result<()> {
        info!("Client terminated");
        Ok(())
    }
}

#[derive(Debug)]
#[implement(IWTSListenerCallback)]
pub struct RdPipeListenerCallback {
    name: String,
    async_runtime: Arc<Runtime>,
}

impl RdPipeListenerCallback {
    #[instrument]
    pub fn new(name: &str, async_runtime: Arc<Runtime>) -> RdPipeListenerCallback {
        RdPipeListenerCallback {
            name: name.to_string(),
            async_runtime: async_runtime,
        }
    }
}

impl IWTSListenerCallback_Impl for RdPipeListenerCallback {
    #[instrument]
    fn OnNewChannelConnection(
        &self,
        pchannel: &Option<IWTSVirtualChannel>,
        data: &BSTR,
        pbaccept: *mut BOOL,
        ppcallback: *mut Option<IWTSVirtualChannelCallback>,
    ) -> Result<()> {
        debug!(
            "Creating new callback for channel {:?} with name {}",
            pchannel, &self.name
        );
        let channel = match pchannel {
            Some(c) => c.to_owned(),
            None => return Err(Error::from(E_UNEXPECTED)),
        };
        let pbaccept = unsafe { &mut *pbaccept };
        let ppcallback = unsafe { &mut *ppcallback };
        *pbaccept = BOOL::from(true);
        debug!("Creating callback");
        let callback: IWTSVirtualChannelCallback =
            RdPipeChannelCallback::new(channel, &self.name, self.async_runtime.clone()).into();
        trace!("Callback {:?} created", callback);
        *ppcallback = Some(callback);
        Ok(())
    }
}

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\RdPipe";

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
    server_task_handle: JoinHandle<io::Result<()>>,
    data_sender: Option<UnboundedSender<Vec<u8>>>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(
        channel: IWTSVirtualChannel,
        channel_name: &str,
        async_runtime: Arc<Runtime>,
    ) -> RdPipeChannelCallback {
        trace!("Constructing unbounded mpsc");
        let (sender, receiver) = unbounded_channel::<Vec<u8>>();
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        debug!("Creating agile reference to channel");
        let channel_agile = AgileReference::new(&channel).unwrap();
        debug!("Spawning process_messages task");
        let processor_handle = async_runtime.spawn(async move {
            RdPipeChannelCallback::process_pipe(channel_agile, &addr, receiver).await
        });
        debug!("Constructing the callback");
        let callback = RdPipeChannelCallback {
            server_task_handle: processor_handle,
            data_sender: Some(sender),
        };
        callback
    }

    #[instrument]
    async fn process_pipe(
        channel_agile: AgileReference<IWTSVirtualChannel>,
        pipe_addr: &str,
        mut data_receiver: UnboundedReceiver<Vec<u8>>,
    ) -> io::Result<()> {
        trace!("Creating first pipe server with address {}", pipe_addr);
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .max_instances(1)
            .create(pipe_addr)?;
        loop {
            trace!("Initiate connection to pipe client");
            server.connect().await.unwrap();
            let (server_reader, mut server_writer) = split(server);
            trace!("Pipe client connected. Replacing server for new clients, if any");
            server = ServerOptions::new().max_instances(1).create(pipe_addr)?;
            trace!("Spawning channel writer task");
            let pipe_reader_handle = tokio::spawn(RdPipeChannelCallback::write_channel(
                server_reader,
                channel_agile.clone(),
            ));
            trace!("Entering pipe writer loop");
            loop {
                tokio::select! {
                    _ = pipe_reader_handle => {
                        return Ok(());
                    }
                    val = data_receiver.recv() => {
                        match val {
                            Some(b) => {
                                server_writer.write(&b).await?;
                            },
                            None => {
                                debug!("Receiver closed");
                                server_writer.shutdown().await?;
                                break;
                            }
                        }
                    }
                }
            }
            trace!("Exiting pipe writer loop");
        }
    }

    #[instrument]
    async fn write_channel(
        mut server_reader: ReadHalf<NamedPipeServer>,
        channel_agile: AgileReference<IWTSVirtualChannel>,
    ) -> io::Result<()> {
        trace!("Initiating writer loop");
        loop {
            let mut buf = Vec::with_capacity(4096);
            match server_reader.read(&mut buf).await {
                Ok(n) if n == 0 => {
                    info!("Received 0 bytes, pipe closed by client");
                    return Ok(());
                }
                Ok(n) => {
                    trace!("read {} bytes", n);
                    let channel = channel_agile.resolve().unwrap();
                    unsafe { channel.Write(&mut buf, None) }.unwrap();
                }
                Err(e) if e.kind() == WouldBlock => {
                    continue;
                }
                Err(e) => {
                    error!("Error reading from pipe server: {}", e);
                    return Err(e);
                }
            }
        }
    }
}

impl Drop for RdPipeChannelCallback {
    #[instrument]
    fn drop(&mut self) {
        self.OnClose().unwrap_or_default();
    }
}

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback {
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        if let Some(sender) = &self.data_sender {
            let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
            trace!("Writing received data to sender");
            sender.send(slice.to_owned()).unwrap();
        }
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        // todo, drop(self.data_sender);
        Ok(())
    }
}
