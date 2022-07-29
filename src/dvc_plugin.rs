use windows::{
    core::implement,
    Win32::System::RemoteDesktop::{IWTSPlugin, IWTSPlugin_Impl, IWTSVirtualChannelManager},
};

#[implement(IWTSPlugin)]
struct DvcPlugin();

impl IWTSPlugin_Impl for DvcPlugin {
    fn Initialize(
        &self,
        pchannelmgr: &core::option::Option<
            IWTSVirtualChannelManager,
        >,
    ) -> windows::core::Result<()> {
        todo!()
    }

    fn Connected(&self) -> windows::core::Result<()> {
        todo!()
    }

    fn Disconnected(&self, dwdisconnectcode: u32) -> windows::core::Result<()> {
        todo!()
    }

    fn Terminated(&self) -> windows::core::Result<()> {
        todo!()
    }
}
