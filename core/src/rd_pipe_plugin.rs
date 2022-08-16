use bytes::BytesMut;
use core::slice;
use std::{future, io::ErrorKind::WouldBlock, sync::Arc};
use tokio::{
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
    sync::RwLock,
};
use tracing::{debug, error, info, instrument};
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
        self.create_listener(channel_mgr, "echo")?;
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
    pub name: String,
}

impl RdPipeListenerCallback {
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
            "Creating new callback for channel {:?} with name {} and data {}",
            pchannel, &self.name, data
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
        debug!("Callback {:?} created", callback);
        *ppcallback = Some(callback);
        Ok(())
    }
}

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\RdPipe";

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
    channel: IWTSVirtualChannel,
    pipe_server: Arc<RwLock<NamedPipeServer>>,
}

impl RdPipeChannelCallback {
    pub fn new(channel: IWTSVirtualChannel, channel_name: &str) -> RdPipeChannelCallback {
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        let pipe_server = ServerOptions::new()
            .max_instances(1)
            .pipe_mode(PipeMode::Message)
            .create(addr)
            .unwrap();
        let callback = RdPipeChannelCallback {
            channel,
            pipe_server: Arc::new(RwLock::new(pipe_server)),
        };
        callback.run_pipe_reader();
        callback
    }

    fn run_pipe_reader(&self) {
        let server = Arc::clone(&(self.pipe_server));
        let channel = AgileReference::new(&(self.channel)).unwrap();
        tokio::spawn(async move {
            {
                let reader = server.read().await;
                reader.connect().await;
            }
            loop {
                let mut buf = BytesMut::with_capacity(4096);
                let reader = server.read().await;
                match reader.try_read(&mut buf) {
                    Ok(n) => {
                        debug!("read {} bytes", n);
                        let channel_inst = channel.resolve().unwrap();
                        unsafe { channel_inst.Write(&mut buf, None) };
                    }
                    Err(e) if e.kind() == WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        return e;
                    }
                }
            }
        });
    }
}

impl Drop for RdPipeChannelCallback {
    fn drop(&mut self) {
        self.OnClose();
    }
}

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback {
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
        tokio::future::block_on(self.pipe_server.read())
            .unwrap()
            .try_write(slice);
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        debug!("Closing channel {}", self.channel.as_raw() as usize);
        self.pipe_server.read().unwrap().disconnect().unwrap();
        unsafe { self.channel.Close() }
    }
}
