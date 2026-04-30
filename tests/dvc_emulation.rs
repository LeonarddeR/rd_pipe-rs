// Copyright (C) 2026 Leonard de Ruijter
// End-to-end integration tests for the rd_pipe COM plugin.

mod common;

use serial_test::serial;

#[test]
#[serial]
fn factory_creates_plugin() {
    let _hkcu = common::HkcuOverride::new().expect("override hkcu");
    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);
    // create_plugin succeeds => DllGetClassObject + CreateInstance(IWTSPlugin) both worked.
    drop(plugin);
}

#[test]
#[serial]
fn initialize_creates_listeners_per_channel() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize failed");
    }

    let events = mgr_state.events.lock().clone();
    // Plugin reads both HKCU (redirected to hive) and HKLM (not redirected).
    // HKLM may contribute empty or extra names on registered machines; only
    // assert that the expected name is present.
    let names: std::collections::HashSet<String> = events
        .iter()
        .map(|e| match e {
            common::MgrEvent::CreateListener { name } => name.clone(),
        })
        .collect();
    assert!(
        names.contains("RdPipeTest"),
        "expected CreateListener(\"RdPipeTest\"), got {names:?}"
    );

    drop(plugin);
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
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, _chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);

    let addr = common::channel_addr(&channel_iface);
    let _client = common::block_on(common::connect_pipe_client(
        "RdPipeTest",
        addr,
        std::time::Duration::from_secs(5),
    ));

    unsafe {
        chan_cb.OnClose().expect("OnClose");
    }
    drop(plugin);
}

#[test]
#[serial]
fn channel_to_pipe_round_trip() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);
    let addr = common::channel_addr(&channel_iface);

    common::block_on(async {
        use tokio::io::AsyncReadExt;

        let mut client =
            common::connect_pipe_client("RdPipeTest", addr, std::time::Duration::from_secs(5))
                .await;

        // Wait for XON so the plugin's pipe writer half is registered.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while chan_state.snapshot_writes().is_empty() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            !chan_state.snapshot_writes().is_empty(),
            "timed out waiting for XON"
        );

        // Push data via OnDataReceived -> plugin writes to pipe -> client reads.
        let payload = b"world";
        unsafe {
            chan_cb.OnDataReceived(payload).expect("OnDataReceived");
        }

        let mut got = [0u8; 5];
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.read_exact(&mut got),
        )
        .await
        .expect("read timeout")
        .expect("read");
        assert_eq!(&got, b"world");
    });

    unsafe {
        chan_cb.OnClose().expect("OnClose");
    }
    drop(plugin);
}

#[test]
#[serial]
fn pipe_close_writes_xoff_to_channel() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);
    let addr = common::channel_addr(&channel_iface);

    common::block_on(async {
        let client =
            common::connect_pipe_client("RdPipeTest", addr, std::time::Duration::from_secs(5))
                .await;

        // Wait for XON.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while chan_state.snapshot_writes().is_empty() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            !chan_state.snapshot_writes().is_empty(),
            "timed out waiting for XON"
        );

        // Drop client -> plugin reads 0 bytes -> writes XOFF (0x13).
        drop(client);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let flat = chan_state.flat_writes();
            if flat.contains(&0x13u8) {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("XOFF never written; got {flat:?}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    });

    unsafe {
        chan_cb.OnClose().expect("OnClose");
    }
    drop(plugin);
}

#[test]
#[serial]
fn pipe_to_channel_round_trip() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);
    let addr = common::channel_addr(&channel_iface);

    common::block_on(async {
        use tokio::io::AsyncWriteExt;

        let mut client =
            common::connect_pipe_client("RdPipeTest", addr, std::time::Duration::from_secs(5))
                .await;

        // Wait for plugin to write XON (0x11).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while chan_state.snapshot_writes().is_empty() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let first_writes = chan_state.snapshot_writes();
        assert!(!first_writes.is_empty(), "timed out waiting for XON");
        assert_eq!(first_writes[0], vec![0x11u8], "first write must be XON");

        // Write payload to pipe; assert it arrives on the channel.
        client.write_all(b"hello").await.expect("pipe write");
        client.flush().await.expect("pipe flush");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let flat = chan_state.flat_writes();
            // flat[0] is XON; rest should accumulate "hello".
            if flat.len() > b"hello".len() {
                assert_eq!(&flat[1..1 + b"hello".len()], b"hello");
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("payload never arrived on channel; got {flat:?}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    });

    unsafe {
        chan_cb.OnClose().expect("OnClose");
    }
    drop(plugin);
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
    let plugin = common::create_plugin(dll);

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
}

#[test]
#[serial]
fn on_close_releases_pipe_writer() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, _chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);
    let addr = common::channel_addr(&channel_iface);

    common::block_on(async {
        let client =
            common::connect_pipe_client("RdPipeTest", addr, std::time::Duration::from_secs(5))
                .await;

        // Drop client so plugin's reader hits EOF; this is the path the
        // plugin's writer release uses (end-of-reader-loop sets writer=None).
        drop(client);

        // Call OnClose -> plugin aborts the pipe task and shuts down the writer.
        unsafe {
            chan_cb.OnClose().expect("OnClose");
        }

        // Verify the writer is released: subsequent OnDataReceived must
        // return ERROR_PIPE_NOT_CONNECTED because pipe_writer is None.
        // This is the direct contract that "OnClose releases the writer";
        // probing the named pipe path is unreliable because the plugin
        // respawns a fresh server in its reader loop, so client open may
        // race with the abort and yield PIPE_BUSY rather than FILE_NOT_FOUND.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let result = unsafe { chan_cb.OnDataReceived(b"after-close") };
            match result {
                Err(e)
                    if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into() =>
                {
                    break;
                }
                Err(e) if std::time::Instant::now() < deadline => {
                    // Different error — wait and retry briefly.
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    if std::time::Instant::now() >= deadline {
                        panic!("unexpected error from OnDataReceived after OnClose: {e:?}");
                    }
                }
                Err(e) => {
                    panic!("unexpected error from OnDataReceived after OnClose: {e:?}")
                }
                Ok(()) if std::time::Instant::now() < deadline => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Ok(()) => panic!("OnDataReceived after OnClose unexpectedly succeeded"),
            }
        }
    });

    drop(plugin);
}

#[test]
#[serial]
fn multiple_channels_produce_multiple_listeners() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["ChanA", "ChanB"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let names: std::collections::HashSet<String> = mgr_state
        .events
        .lock()
        .iter()
        .map(|e| match e {
            common::MgrEvent::CreateListener { name } => name.clone(),
        })
        .collect();

    let expected: std::collections::HashSet<String> = ["ChanA".to_string(), "ChanB".to_string()]
        .into_iter()
        .collect();
    // HKLM is not redirected, so machines with rd_pipe registered in HKLM
    // contribute extra channel names. Only assert expected ⊆ names.
    assert!(
        expected.is_subset(&names),
        "missing expected listener names. expected subset: {expected:?}, actual: {names:?}"
    );

    drop(plugin);
}

/// Regression test for issue #57: `OnClose` must terminate the reader task
/// while the client is still connected and any subsequent `OnDataReceived`
/// must fail with `ERROR_PIPE_NOT_CONNECTED`. Shutdown is driven cooperatively
/// via `CancellationToken` + `tokio::select!`, with `OnClose` synchronously
/// dropping the write half so this assertion holds without polling.
#[test]
#[serial]
fn on_close_terminates_reader_cooperatively_while_client_connected() {
    let hkcu = common::HkcuOverride::new().expect("override hkcu");
    hkcu.write_channel_names(&["RdPipeTest"])
        .expect("write channel names");

    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(dll);

    let (mgr_iface, mgr_state) = common::FakeChannelMgr::new();
    unsafe {
        plugin.Initialize(&mgr_iface).expect("Initialize");
    }

    let listener_cb = get_listener_cb(&mgr_state, "RdPipeTest");
    let (channel_iface, chan_state) = common::FakeVirtualChannel::new();
    let chan_cb = common::trigger_new_channel(&listener_cb, &channel_iface);
    let addr = common::channel_addr(&channel_iface);

    common::block_on(async {
        let _client =
            common::connect_pipe_client("RdPipeTest", addr, std::time::Duration::from_secs(5))
                .await;

        // Wait for XON, confirming the pipe connection handshake completed.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while chan_state.snapshot_writes().is_empty() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            !chan_state.snapshot_writes().is_empty(),
            "timed out waiting for XON"
        );

        // Cooperative shutdown: OnClose while the client is still connected.
        unsafe { chan_cb.OnClose().expect("OnClose") };

        // Subsequent OnDataReceived must fail with ERROR_PIPE_NOT_CONNECTED
        // (writer released synchronously by OnClose).
        let r = unsafe { chan_cb.OnDataReceived(b"\xab") };
        assert!(
            matches!(
                r,
                Err(ref e) if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into()
            ),
            "expected ERROR_PIPE_NOT_CONNECTED after OnClose, got {:?}",
            r
        );
    });

    drop(plugin);
}
