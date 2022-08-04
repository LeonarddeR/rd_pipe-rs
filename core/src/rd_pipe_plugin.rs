use tracing::instrument;
use windows::{
    core::{implement, Error, Result},
    Win32::{
        Foundation::{BOOL, BSTR, E_UNEXPECTED},
        System::RemoteDesktop::{
            IWTSListenerCallback, IWTSListenerCallback_Impl, IWTSPlugin, IWTSPlugin_Impl,
            IWTSVirtualChannel, IWTSVirtualChannelCallback, IWTSVirtualChannelCallback_Impl,
            IWTSVirtualChannelManager,
        },
    },
};

#[derive(Debug)]
#[implement(IWTSPlugin)]
pub struct RdPipePlugin(Option<IWTSVirtualChannelManager>);

impl RdPipePlugin {
    pub fn new() -> RdPipePlugin {
        RdPipePlugin(None)
    }
}

impl IWTSPlugin_Impl for RdPipePlugin {
    #[instrument]
    fn Initialize(&self, pchannelmgr: &Option<IWTSVirtualChannelManager>) -> Result<()> {
        if pchannelmgr.is_none() {
            return Err(Error::from(E_UNEXPECTED));
        }
        self.0 = *pchannelmgr;
        Ok(())
    }

    fn Connected(&self) -> Result<()> {
        Ok(())
    }

    fn Disconnected(&self, _dwdisconnectcode: u32) -> Result<()> {
        Ok(())
    }

    fn Terminated(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
#[implement(IWTSListenerCallback)]
pub struct RdPipeListenerCallback;

impl IWTSListenerCallback_Impl for RdPipeListenerCallback {
    #[instrument]
    fn OnNewChannelConnection(
        &self,
        pchannel: &Option<IWTSVirtualChannel>,
        data: &BSTR,
        pbaccept: *mut BOOL,
        ppcallback: *mut Option<IWTSVirtualChannelCallback>,
    ) -> Result<()> {
        if pchannel.is_none() {
            return Err(Error::from(E_UNEXPECTED));
        }
        Ok(())
    }
}

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback;

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback {
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        Ok(())
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        Ok(())
    }
}
