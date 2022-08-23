use bytes::BytesMut;
use core::slice;

use std::{io::ErrorKind::WouldBlock, sync::Arc};
use tokio::{
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
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

use tokio::runtime::Builder;

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
    data_sender: UnboundedSender<()>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(channel: IWTSVirtualChannel, channel_name: &str) -> RdPipeChannelCallback {
        trace!("Constructing unbounded mpsc");
        let (sender, receiver) = unbounded_channel();
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        let handle = tokio::spawn(async move {});
        debug!("Constructing the callback");
        let callback = RdPipeChannelCallback {
            server_task_handle: handle,
            data_sender: sender,
        };
        callback.process_messages(&addr, receiver);
        callback
    }

    #[instrument]
    fn process_messages(&self, pipe_addr: &str, data_receiver: UnboundedReceiver<()>) {
        trace!("Cloning pipe server");
        let server_arc = Arc::clone(&(self.pipe_server));
        runtime.block_on(async move {
            {
                let server = server_arc.read().await;
                trace!("Initiate connection to pipe client");
                server.connect().await;
            }
            trace!("Entering pipe writing loop");
            loop {
                let mut buf = BytesMut::with_capacity(4096);
                let server = server_arc.read().await;
                match server.try_read(&mut buf) {
                    Ok(n) => {
                        trace!("read {} bytes", n);
                        let channel = channel_agile.resolve().unwrap();
                        trace!("Writing buffer to channel");
                        unsafe { channel.Write(&mut buf, None) };
                    }
                    Err(e) if e.kind() == WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        error!("Error reading from pipe server: {}", e);
                        return e;
                    }
                }
            }
        });
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
        let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
        trace!("Writing received data to pipe");
        self.pipe_server.blocking_read().try_write(slice);
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        info!("Closing channel {}", self.channel.as_raw() as usize);
        self.pipe_server.blocking_read().disconnect().unwrap();
        unsafe { self.channel.Close() }
    }
}
