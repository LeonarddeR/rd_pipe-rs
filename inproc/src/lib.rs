use std::{ffi::c_void, mem::transmute};

use rd_pipe_core::{class_factory::ClassFactory, rd_pipe_plugin::RdPipePlugin};
use tracing::{debug, instrument};
use windows::Win32::{System::{SystemServices::DLL_PROCESS_ATTACH, LibraryLoader::DisableThreadLibraryCalls}, Foundation::BOOL};
use windows::{
    core::{Interface, GUID, HRESULT},
    Win32::{
        Foundation::{E_UNEXPECTED, HINSTANCE, S_OK},
        System::{Com::IClassFactory, RemoteDesktop::IWTSPlugin},
    },
};

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        // Set up logging
        let file_appender = tracing_appender::rolling::never("d:", "RdPipe");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt().with_writer(non_blocking).init();
        debug!("DllMain: DLL_PROCESS_ATTACH");
        unsafe { DisableThreadLibraryCalls(hinst) };
    }
    BOOL::from(true)
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllGetClassObject(
    _rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let riid = unsafe { *riid };
    let ppv = unsafe { &mut *ppv };
    // ppv must be null if we fail so set it here for safety
    *ppv = std::ptr::null_mut();

    if riid != IClassFactory::IID {
        return E_UNEXPECTED;
    }

    let factory = ClassFactory;
    let factory: IClassFactory = factory.into();
    *ppv = unsafe { transmute(factory) };

    S_OK
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn VirtualChannelGetInstance(
    riid: *const GUID,
    pnumobjs: *mut u32,
    ppo: *mut *mut c_void,
) -> HRESULT {
    let riid = unsafe { *riid };
    if riid != IWTSPlugin::IID {
        return E_UNEXPECTED;
    }

    let pnumobjs = unsafe { &mut *pnumobjs };
    let ppo = unsafe { &mut *ppo };
    *pnumobjs = 1;
    let plugin: IWTSPlugin = RdPipePlugin::new().into();
    *ppo = unsafe { transmute(plugin) };
    S_OK
}
