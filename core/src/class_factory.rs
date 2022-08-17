use std::mem::transmute;
use tracing::{debug, instrument};
use windows::{
    core::{implement, IUnknown, Result, GUID},
    Win32::{
        Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_NOINTERFACE},
        System::Com::{IClassFactory, IClassFactory_Impl},
    },
};
use windows::{
    core::{Error, Interface},
    Win32::System::RemoteDesktop::IWTSPlugin,
};

use crate::rd_pipe_plugin::RdPipePlugin;

pub const IID_I_RD_PIPE_PLUGIN: GUID = GUID::from_u128(0xD1F74DC79FDE45BE9251FA72D4064DA3);

#[implement(IClassFactory)]
#[derive(Debug)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    #[instrument]
    fn CreateInstance(
        &self,
        outer: &Option<IUnknown>,
        iid: *const GUID,
        object: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        let iid = unsafe { *iid };
        let object = unsafe { &mut *object };
        *object = std::ptr::null_mut();
        debug!("Object with type {:?} requested", iid);
        if outer.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        debug!("Creating plugin instance");
        let plugin = RdPipePlugin::new();
        match iid {
            IUnknown::IID => {
                debug!("Requested IUnknown");
                let plugin: IUnknown = plugin.into();
                *object = unsafe { transmute(plugin) };
            }
            IWTSPlugin::IID => {
                debug!("Requested IWTSPlugin");
                let plugin: IWTSPlugin = plugin.into();
                *object = unsafe { transmute(plugin) };
            }
            _ => return Err(Error::from(E_NOINTERFACE)),
        }
        Ok(())
    }

    #[instrument]
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        assert!(lock.as_bool());
        Ok(())
    }
}
