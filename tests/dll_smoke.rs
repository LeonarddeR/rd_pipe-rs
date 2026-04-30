// Copyright (C) 2026 Leonard de Ruijter
// Integration tests that verify the DLL loads and exposes its exports.

mod common;

use libloading::Library;

#[test]
fn dll_loads_and_exports_present() {
    let path = common::dll_path();
    assert!(
        path.is_file(),
        "rd_pipe.dll not found at {} — run `cargo test` (which builds the cdylib) instead of `cargo run`",
        path.display()
    );

    // Library::new on Windows calls LoadLibraryW + DllMain(DLL_PROCESS_ATTACH).
    // SAFETY: we control the DLL we are loading; it is the artifact this very
    // test crate produced. Required-export resolution below confirms identity.
    let lib = unsafe { Library::new(&path) }.expect("LoadLibrary failed");

    // Resolve the three exports the COM in-proc server contract requires.
    // Each `lib.get` is unsafe because the function-pointer signature is
    // declared by the caller; we only verify presence here, not call them.
    unsafe {
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllMain\0").expect("DllMain export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllGetClassObject\0").expect("DllGetClassObject export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllInstall\0").expect("DllInstall export missing");
    }

    // Library drops here -> FreeLibrary -> DllMain(DLL_PROCESS_DETACH).
}

use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_UNEXPECTED};
use windows::Win32::System::Com::IClassFactory;
use windows::core::{GUID, HRESULT, Interface};
use windows_core::{OutRef, Ref};

/// CLSID published by the production crate. Hard-coded here so the test does
/// not depend on the crate's Rust API surface; the value is the same string
/// used in `src/registry.rs`.
const CLSID_RD_PIPE_PLUGIN: GUID = GUID::from_u128(0xD1F74DC7_9FDE_45BE_9251_FA72D4064DA3);

type DllGetClassObjectFn = unsafe extern "system" fn(
    rclsid: Ref<GUID>,
    riid: Ref<GUID>,
    ppv: OutRef<IClassFactory>,
) -> HRESULT;

#[test]
fn bad_clsid_returns_class_e_classnotavailable() {
    let lib = unsafe { Library::new(common::dll_path()) }.expect("load DLL");
    let get_class_object: libloading::Symbol<DllGetClassObjectFn> =
        unsafe { lib.get(b"DllGetClassObject\0") }.expect("resolve DllGetClassObject");

    // A CLSID we made up — guaranteed not to match the plugin's.
    let bad_clsid = GUID::from_u128(0xDEAD_BEEF_DEAD_BEEF_DEAD_BEEF_DEAD_BEEFu128);
    let mut out: Option<IClassFactory> = None;

    let hr = unsafe {
        get_class_object(
            Ref::from(&bad_clsid),
            Ref::from(&IClassFactory::IID),
            OutRef::from(&mut out),
        )
    };

    assert_eq!(hr, CLASS_E_CLASSNOTAVAILABLE, "expected CLASS_E_CLASSNOTAVAILABLE, got {hr:?}");
    assert!(out.is_none(), "ppv should have been written to None on rejection");
}

#[test]
fn bad_iid_returns_e_unexpected() {
    let lib = unsafe { Library::new(common::dll_path()) }.expect("load DLL");
    let get_class_object: libloading::Symbol<DllGetClassObjectFn> =
        unsafe { lib.get(b"DllGetClassObject\0") }.expect("resolve DllGetClassObject");

    // Correct CLSID, made-up IID — plugin must reject.
    let bad_iid = GUID::from_u128(0xCAFEBABE_CAFE_BABE_CAFE_BABECAFEBABEu128);
    let mut out: Option<IClassFactory> = None;

    let hr = unsafe {
        get_class_object(
            Ref::from(&CLSID_RD_PIPE_PLUGIN),
            Ref::from(&bad_iid),
            OutRef::from(&mut out),
        )
    };

    assert_eq!(hr, E_UNEXPECTED, "expected E_UNEXPECTED, got {hr:?}");
    assert!(out.is_none(), "ppv should have been written to None on rejection");
}

#[test]
fn hkcu_override_smoke() {
    let guard = common::HkcuOverride::new().expect("override hkcu");
    guard.write_channel_names(&["smoke"]).expect("write channel names");
    drop(guard);
}
