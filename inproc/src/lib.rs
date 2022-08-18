use lazy_static::lazy_static;
use rd_pipe_core::{class_factory::ClassFactory, rd_pipe_plugin::RdPipePlugin};
use std::{ffi::c_void, mem::transmute};
use tokio::runtime::Runtime;
use tracing::{debug, error, instrument};
use windows::Win32::{
    Foundation::BOOL,
    System::{LibraryLoader::DisableThreadLibraryCalls, SystemServices::DLL_PROCESS_ATTACH},
};
use windows::{
    core::{Interface, GUID, HRESULT},
    Win32::{
        Foundation::{E_UNEXPECTED, HINSTANCE, S_OK},
        System::{Com::IClassFactory, RemoteDesktop::IWTSPlugin},
    },
};

lazy_static! {
    static ref RUNTIME: Runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        // Set up logging
        let file_appender = tracing_appender::rolling::never("d:", "RdPipe.log");
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .init();
        debug!("DllMain: DLL_PROCESS_ATTACH");
        unsafe { DisableThreadLibraryCalls(hinst) };
        debug!("Disabled thread library calls");
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
    debug!("DllGetClassObject called");
    let riid = unsafe { *riid };
    let ppv = unsafe { &mut *ppv };
    // ppv must be null if we fail so set it here for safety
    *ppv = std::ptr::null_mut();

    if riid != IClassFactory::IID {
        debug!("DllGetClassObject called for unknown interface: {:?}", riid);
        return E_UNEXPECTED;
    }
    debug!("Constructing class factory");
    let factory = ClassFactory;
    let factory: IClassFactory = factory.into();
    debug!("Setting result pointer to class factory");
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
    debug!("VirtualChannelGetInstance called");
    let riid = unsafe { *riid };
    if riid != IWTSPlugin::IID {
        debug!(
            "VirtualChannelGetInstance called for unknown interface: {:?}",
            riid
        );
        return E_UNEXPECTED;
    }
    let pnumobjs = unsafe { &mut *pnumobjs };
    debug!("Checking whether result pointer is null (i.e. whether this call is a query for number of plugins or a query for the plugins itself)");
    if ppo.is_null() {
        debug!("Result pointer is null, client is querying for number of objects. Setting pnumobjs to 1, since we only support one plugin");
        *pnumobjs = 1;
    } else {
        debug!("{} plugins requested", *pnumobjs);
        if *pnumobjs != 1 {
            error!("Invalid number of plugins requested: {}", *pnumobjs);
            return E_UNEXPECTED;
        }
        let ppo = unsafe { &mut *ppo };
        debug!("Constructing the plugin");
        let plugin: IWTSPlugin = RdPipePlugin::new().into();
        debug!("Setting result pointer to plugin");
        *ppo = unsafe { transmute(plugin) };
    }
    S_OK
}
