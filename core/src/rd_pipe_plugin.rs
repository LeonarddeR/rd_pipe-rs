use bytes::{Bytes, BytesMut};
use core::slice;

use std::io::ErrorKind::WouldBlock;
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::ServerOptions,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace};
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
pub struct RdPipePlugin;

impl RdPipePlugin {
    #[instrument]
    pub fn new() -> RdPipePlugin {
        RdPipePlugin
    }

    #[instrument]
    fn create_listener(
        &self,
        channel_mgr: &IWTSVirtualChannelManager,
        channel_name: &str,
    ) -> Result<IWTSListener> {
        debug!("Creating listener with name {}", channel_name);
        let callback: IWTSListenerCallback = RdPipeListenerCallback::new(channel_name).into();
        unsafe { channel_mgr.CreateListener(format!("{}\0", channel_name).as_ptr(), 0, &callback) }
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
}

impl RdPipeListenerCallback {
    #[instrument]
    pub fn new(name: &str) -> RdPipeListenerCallback {
        RdPipeListenerCallback {
            name: name.to_string(),
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
            RdPipeChannelCallback::new(channel, &self.name).into();
        trace!("Callback {:?} created", callback);
        *ppcallback = Some(callback);
        Ok(())
    }
}

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\RdPipe";

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
    server_task_handle: JoinHandle<()>,
    data_sender: Option<UnboundedSender<Bytes>>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(channel: IWTSVirtualChannel, channel_name: &str) -> RdPipeChannelCallback {
        trace!("Constructing unbounded mpsc");
        let (sender, mut receiver) = unbounded_channel::<Bytes>();
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        debug!("Creating agile reference to channel");
        let channel_agile = AgileReference::new(&channel).unwrap();
        debug!("Spawning process_messages task");
        let handle = tokio::spawn(async move {
            RdPipeChannelCallback::process_messages(channel_agile, &addr, &mut receiver).await
        });
        debug!("Constructing the callback");
        let callback = RdPipeChannelCallback {
            server_task_handle: handle,
            data_sender: Some(sender),
        };
        callback
    }

    async fn process_messages(
        channel_agile: AgileReference<IWTSVirtualChannel>,
        pipe_addr: &str,
        data_receiver: &mut UnboundedReceiver<Bytes>,
    ) {
        trace!("Creating pipe server");
        let server = ServerOptions::new()
            .max_instances(1)
            .create(pipe_addr)
            .unwrap();
        trace!("Initiate connection to pipe client");
        server.connect().await;
        let (mut server_reader, mut server_writer) = split(server);
        let pipe_reader_handle = tokio::spawn(async move {
            loop {
                let mut buf = BytesMut::with_capacity(4096);
                match server_reader.read(&mut buf).await {
                    Ok(n) => {
                        trace!("read {} bytes", n);
                        let channel = channel_agile.resolve().unwrap();
                        unsafe { channel.Write(&mut buf, None) };
                    }
                    Err(e) if e.kind() == WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        error!("Error reading from pipe server: {}", e);
                        break;
                    }
                }
            }
        });
        loop {
            let buf = data_receiver.recv().await.unwrap();
            server_writer.write(&buf).await.unwrap();
        }
    }
}

impl Drop for RdPipeChannelCallback {
    #[instrument]
    fn drop(&mut self) {
        self.OnClose();
    }
}

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback {
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        match &self.data_sender {
            Some(sender) => {
                let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
                let bytes = bytes::Bytes::copy_from_slice(slice);
                trace!("Writing received data to pipe");
                sender.send(bytes);
            }
            None => return Ok(()),
        };
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        // todo, drop(self.data_sender);
        Ok(())
    }
}
