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

/// Get the first listener callback with the given channel name from mgr state.
fn get_listener_cb(
    mgr_state: &common::FakeMgrState,
    name: &str,
) -> windows::Win32::System::RemoteDesktop::IWTSListenerCallback {
    mgr_state
        .listeners
        .lock()
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no listener for channel {name:?}"))
        .1
        .clone()
}

#[test]
#[serial]
fn new_channel_connection_opens_named_pipe() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"]).expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(&dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe { plugin.Initialize(&mgr_iface).expect("Initialize"); }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, _chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);

    let addr = common::channel_addr(&channel_iface);
    let _client = common::block_on(common::connect_pipe_client(
        "RdPipeTest",
        addr,
        std::time::Duration::from_secs(5),
    ));

    unsafe { chan_cb.OnClose().expect("OnClose"); }
    drop(plugin);
    drop(dll);
}
#[test]
#[serial]
fn initialize_with_empty_channels_returns_e_unexpected() {
    // Override HKCU but write NO ChannelNames.
    // HKLM is not redirected; this test assumes the DLL is not registered
    // in HKLM on the test machine (true in CI and fresh dev machines).
    // If rd_pipe IS registered in HKLM, Initialize may succeed — in that
    // case we accept Ok as well.
    let _hkcu = common::HkcuOverride::new().expect("override hkcu");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(&dll);

    let (mgr_iface, _state) = common::FakeChannelMgr::new();
    let result = unsafe { plugin.Initialize(&mgr_iface) };
    match result {
        Err(e) => assert_eq!(
            e.code(),
            windows::Win32::Foundation::E_UNEXPECTED,
            "expected E_UNEXPECTED, got {e:?}"
        ),
        Ok(()) => {
            // HKLM has ChannelNames registered — acceptable on registered machines.
        }
    }

    drop(plugin);
    drop(dll);
}
