use std::{ffi::c_void, mem::transmute};

use dvc_core::class_factory::ClassFactory;
use windows::{core::{GUID, HRESULT, Interface}, Win32::{System::Com::IClassFactory, Foundation::{E_UNEXPECTED, S_OK}}};

#[no_mangle]
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
