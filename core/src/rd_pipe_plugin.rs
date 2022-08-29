use core::slice;
use std::{
    io::{self, ErrorKind::WouldBlock},
    sync::{Arc, Mutex},
};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, WriteHalf},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    runtime::{Builder, Runtime},
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
    async_runtime: Arc<Runtime>,
    name: String,
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
    async_runtime: Arc<Runtime>,
    pipe_writer: Arc<Mutex<Option<WriteHalf<NamedPipeServer>>>>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(
        channel: IWTSVirtualChannel,
        channel_name: &str,
        async_runtime: Arc<Runtime>,
    ) -> RdPipeChannelCallback {
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        debug!("Creating agile reference to channel");
        let channel_agile = AgileReference::new(&channel).unwrap();
        debug!("Constructing the callback");
        let callback = RdPipeChannelCallback {
            async_runtime: async_runtime.clone(),
            pipe_writer: Arc::new(Mutex::new(None)),
        };
        debug!("Spawning process_messages task");
        callback.process_pipe(channel_agile, addr.to_string());
        callback
    }

    #[instrument]
    fn process_pipe(
        &self,
        channel_agile: AgileReference<IWTSVirtualChannel>,
        pipe_addr: String,
    ) -> JoinHandle<io::Result<()>> {
        let writer = self.pipe_writer.clone();
        self.async_runtime.spawn(async move {
            let mut first_pipe_instance = true;
            loop {
                trace!("Creating pipe server with address {}", pipe_addr);
                let server = ServerOptions::new()
                    .first_pipe_instance(first_pipe_instance)
                    .max_instances(1)
                    .create(&pipe_addr)
                    .unwrap();
                first_pipe_instance = false;
                trace!("Initiate connection to pipe client");
                server.connect().await.unwrap();
                let (mut server_reader, server_writer) = split(server);
                {
                    let mut writer_guard = writer.lock().unwrap();
                    *writer_guard = Some(server_writer);
                }
                trace!("Pipe client connected. initiating pipe_reader loop");
                loop {
                    let mut buf = Vec::with_capacity(4096);
                    match server_reader.read(&mut buf).await {
                        Ok(0) => {
                            info!("Received 0 bytes, pipe closed by client");
                            break;
                        }
                        Ok(n) => {
                            trace!("read {} bytes", n);
                            let channel = channel_agile.resolve().unwrap();
                            unsafe { channel.Write(&mut buf, None) }.unwrap();
                        }
                        Err(e) if e.kind() == WouldBlock => {
                            warn!("Reading pipe would block: {}", e);
                            continue;
                        }
                        Err(e) => {
                            error!("Error reading from pipe server: {}", e);
                            break;
                        }
                    }
                }
                trace!("End of pipe_reader loop, releasing writer");
                {
                    let mut writer_guard = writer.lock().unwrap();
                    *writer_guard = None;
                }
                trace!("Writer released");
            }
        })
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
        debug!("Data received, buffer has size {}", cbsize);
        let mut writer_lock = self.pipe_writer.lock().unwrap();
        match writer_lock.as_mut() {
            Some(writer) => {
                let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
                trace!("Writing received data to pipe");
                self.async_runtime.block_on(writer.write(slice)).unwrap();
            }
            None => {
                debug!("Data received without an open named pipe");
            }
        }
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        let mut writer_lock = self.pipe_writer.lock().unwrap();
        match writer_lock.as_mut() {
            Some(writer) => {
                self.async_runtime.block_on(writer.shutdown()).unwrap();
            }
            None => {}
        }
        Ok(())
    }
}
