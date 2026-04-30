// Copyright (C) 2026 Leonard de Ruijter
// Integration tests that verify the DLL loads and exposes its exports.

mod common;

use serial_test::serial;

#[test]
fn dll_loads_and_exports_present() {
    // Use the shared OnceLock loader so DllMain runs once per process; this
    // also keeps the Library alive for the entire test binary, avoiding a
    // FreeLibrary/tracing-subscriber TLS race on detach.
    let dll = common::DllHandle::load();
    let lib: &libloading::Library = dll.lib();

    // Resolve the standard COM in-proc server exports this DLL provides.
    // `DllGetClassObject` and `DllCanUnloadNow` are required by the COM
    // in-proc-server contract; `DllInstall` is the regsvr32 /i hook this
    // crate uses for registration. `DllMain` is the module entry point.
    unsafe {
        let _: libloading::Symbol<unsafe extern "system" fn()> = lib
            .get(b"DllGetClassObject\0")
            .expect("DllGetClassObject export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllInstall\0").expect("DllInstall export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllMain\0").expect("DllMain export missing");
    }
}

use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_UNEXPECTED};
use windows::Win32::System::Com::IClassFactory;
use windows::core::{GUID, Interface};
use windows_core::{OutRef, Ref};

#[test]
fn bad_clsid_returns_class_e_classnotavailable() {
    let dll = common::DllHandle::load();

    // A CLSID we made up — guaranteed not to match the plugin's.
    let bad_clsid = GUID::from_u128(0xDEAD_BEEF_DEAD_BEEF_DEAD_BEEF_DEAD_BEEFu128);
    let mut out: Option<IClassFactory> = None;

    let hr = unsafe {
        (dll.get_class_object)(
            Ref::from(&bad_clsid),
            Ref::from(&IClassFactory::IID),
            OutRef::from(&mut out),
        )
    };

    assert_eq!(
        hr, CLASS_E_CLASSNOTAVAILABLE,
        "expected CLASS_E_CLASSNOTAVAILABLE, got {hr:?}"
    );
    assert!(
        out.is_none(),
        "ppv should have been written to None on rejection"
    );
}

#[test]
fn bad_iid_returns_e_unexpected() {
    let dll = common::DllHandle::load();

    // Correct CLSID, made-up IID — plugin must reject.
    let bad_iid = GUID::from_u128(0xCAFEBABE_CAFE_BABE_CAFE_BABECAFEBABEu128);
    let mut out: Option<IClassFactory> = None;

    let hr = unsafe {
        (dll.get_class_object)(
            Ref::from(&common::CLSID_RD_PIPE_PLUGIN),
            Ref::from(&bad_iid),
            OutRef::from(&mut out),
        )
    };

    assert_eq!(hr, E_UNEXPECTED, "expected E_UNEXPECTED, got {hr:?}");
    assert!(
        out.is_none(),
        "ppv should have been written to None on rejection"
    );
}

#[test]
#[serial]
fn hkcu_override_smoke() {
    // Serial: RegOverridePredefKey is process-wide and would race with any
    // other test that loads the DLL or reads HKCU concurrently.
    let guard = common::HkcuOverride::new().expect("override hkcu");
    guard
        .write_channel_names(&["smoke"])
        .expect("write channel names");
    drop(guard);
}

#[test]
fn fake_virtual_channel_records_writes() {
    let (chan, state) = common::FakeVirtualChannel::new();
    let payload = b"abc";
    unsafe {
        chan.Write(payload, None).unwrap();
    }
    assert_eq!(state.flat_writes(), b"abc");
}
