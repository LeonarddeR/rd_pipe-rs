// Copyright (C) 2026 Leonard de Ruijter
// End-to-end integration tests for the rd_pipe COM plugin.

mod common;

use serial_test::serial;

#[test]
#[serial]
fn factory_creates_plugin() {
    let _hkcu = common::HkcuOverride::new().expect("override hkcu");
    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(&dll);
    // create_plugin succeeds => DllGetClassObject + CreateInstance(IWTSPlugin) both worked.
    drop(plugin);
}

#[test]
#[serial]
fn initialize_creates_listeners_per_channel() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"]).expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(&dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize failed");
    }

    let events = mgr_state.events.lock().clone();
    // Plugin reads both HKCU (redirected to hive) and HKLM (not redirected).
    // HKLM may contribute empty or extra names; assert the expected name is present
    // and no unexpected non-empty names appear.
    let names: std::collections::HashSet<String> = events
        .iter()
        .map(|e| match e {
            common::MgrEvent::CreateListener { name } => name.clone(),
        })
        .filter(|n| !n.is_empty())
        .collect();
    assert!(
        names.contains("RdPipeTest"),
        "expected CreateListener(\"RdPipeTest\"), got {names:?}"
    );
    assert_eq!(names.len(), 1, "unexpected extra channel names: {names:?}");

    drop(plugin);
    drop(dll);
}
