use std::mem::transmute;

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

use crate::dvc_plugin::DvcPlugin;

pub const IID_I_DVC_PLUGIN: GUID = GUID::from_u128(0xD1F74DC79FDE45BE9251FA72D4064DA3);

#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        outer: &Option<IUnknown>,
        iid: *const GUID,
        object: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        let iid = unsafe { *iid };
        unsafe { *object = std::ptr::null_mut() };
        if outer.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        let plugin = DvcPlugin;
        if iid == IUnknown::IID {
            let plugin: IUnknown = plugin.into();
            unsafe {
                *object = transmute(plugin);
            }
        } else if iid == IWTSPlugin::IID {
            let plugin: IWTSPlugin = plugin.into();
            unsafe {
                *object = transmute(plugin);
            }
        } else {
            return Err(Error::from(E_NOINTERFACE));
        }
        Ok(())
    }

    fn LockServer(&self, lock: BOOL) -> Result<()> {
        assert!(lock.as_bool());
        Ok(())
    }
}