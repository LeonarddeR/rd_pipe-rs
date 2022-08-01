use windows::{
    core::{implement, Result},
    Win32::System::RemoteDesktop::{IWTSPlugin, IWTSPlugin_Impl, IWTSVirtualChannelManager},
};

#[implement(IWTSPlugin)]
pub struct DvcPlugin;

impl IWTSPlugin_Impl for DvcPlugin {
    fn Initialize(
        &self,
        _pchannelmgr: &Option<
            IWTSVirtualChannelManager,
        >,
    ) -> Result<()> {
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
