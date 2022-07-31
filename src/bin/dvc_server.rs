use dvc::class_factory::{ClassFactory, IID_I_DVC_PLUGIN};

use std::time::Duration;
use std::thread::sleep;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, IClassFactory};
use windows::{
    core::IUnknown,
    Win32::System::Com::{
        CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CLSCTX_LOCAL_SERVER,
        REGCLS_MULTIPLEUSE,
    },
};

fn main() {
    unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED) }.unwrap();
    let factory: IClassFactory = ClassFactory.into();
    let res = unsafe {
        CoRegisterClassObject(
            &IID_I_DVC_PLUGIN,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )
    };
    let cookie: u32 = res.unwrap();
    sleep(Duration::from_secs(120));
    unsafe {
        CoRevokeClassObject(cookie)
    }.unwrap();
}
