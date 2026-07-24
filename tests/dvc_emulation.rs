// Copyright (C) 2026 Leonard de Ruijter
// End-to-end integration tests for the rd_pipe COM plugin.

mod common;

use serial_test::serial;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::System::RemoteDesktop::{
	IWTSPlugin, IWTSVirtualChannel, IWTSVirtualChannelCallback,
};

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
	hkcu.write_channel_names(&["RdPipeTest"]).expect("write channel names");

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
	assert!(names.contains("RdPipeTest"), "expected CreateListener(\"RdPipeTest\"), got {names:?}");

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

/// Poll `pred` every 25 ms until it returns true or `deadline` elapses.
fn wait_until(deadline: Duration, mut pred: impl FnMut() -> bool) -> bool {
	let end = Instant::now() + deadline;
	loop {
		if pred() {
			return true;
		}
		if Instant::now() >= end {
			return false;
		}
		std::thread::sleep(Duration::from_millis(25));
	}
}

/// Wait for the plugin's first channel write after a client connects (XON),
/// which also means the pipe writer is registered.
fn wait_for_xon(chan_state: &common::FakeChannelState) {
	assert!(
		wait_until(Duration::from_secs(5), || !chan_state.snapshot_writes().is_empty()),
		"timed out waiting for XON"
	);
}

/// Wait for XOFF (0x13) to appear among the channel writes.
fn wait_for_xoff(chan_state: &common::FakeChannelState) {
	assert!(
		wait_until(Duration::from_secs(5), || chan_state.flat_writes().contains(&0x13u8)),
		"XOFF never written; got {:?}",
		chan_state.flat_writes()
	);
}

/// Everything the single-channel lifecycle tests share: registry override,
/// loaded plugin, an initialized "RdPipeTest" channel and its callback.
/// Fields drop in declaration order: plugin first, registry override last.
struct ChannelFixture {
	_plugin: IWTSPlugin,
	chan_cb: IWTSVirtualChannelCallback,
	chan_state: Arc<common::FakeChannelState>,
	_channel: IWTSVirtualChannel,
	addr: usize,
	_hkcu: common::HkcuOverride,
}

fn setup_channel() -> ChannelFixture {
	let hkcu = common::HkcuOverride::new().expect("override hkcu");
	hkcu.write_channel_names(&["RdPipeTest"]).expect("write channel names");

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

	ChannelFixture {
		_plugin: plugin,
		chan_cb,
		chan_state,
		_channel: channel_iface,
		addr,
		_hkcu: hkcu,
	}
}

impl ChannelFixture {
	/// Connects a pipe client and returns it once the plugin has written XON.
	fn connect_client_and_wait_for_xon(&self) -> std::fs::File {
		let client = common::connect_pipe_client("RdPipeTest", self.addr, Duration::from_secs(5));
		wait_for_xon(&self.chan_state);
		client
	}
}

#[test]
#[serial]
fn new_channel_connection_opens_named_pipe() {
	let fx = setup_channel();
	let _client = common::connect_pipe_client("RdPipeTest", fx.addr, Duration::from_secs(5));

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

#[test]
#[serial]
fn channel_to_pipe_round_trip() {
	let fx = setup_channel();
	let client = fx.connect_client_and_wait_for_xon();

	// Push data via OnDataReceived -> plugin writes to pipe -> client reads.
	let payload = b"world";
	unsafe {
		fx.chan_cb.OnDataReceived(payload).expect("OnDataReceived");
	}

	let got = common::read_exact_with_timeout(&client, payload.len(), Duration::from_secs(5))
		.expect("read");
	assert_eq!(&got, b"world");

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

#[test]
#[serial]
fn pipe_close_writes_xoff_to_channel() {
	let fx = setup_channel();
	let client = fx.connect_client_and_wait_for_xon();

	// Drop client -> plugin read fails -> writes XOFF (0x13).
	drop(client);
	wait_for_xoff(&fx.chan_state);

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

#[test]
#[serial]
fn pipe_to_channel_round_trip() {
	let fx = setup_channel();
	let mut client = fx.connect_client_and_wait_for_xon();

	let first_writes = fx.chan_state.snapshot_writes();
	assert_eq!(first_writes[0], vec![0x11u8], "first write must be XON");

	// Write payload to pipe; assert it arrives on the channel.
	client.write_all(b"hello").expect("pipe write");
	client.flush().expect("pipe flush");

	assert!(
		wait_until(Duration::from_secs(5), || {
			let flat = fx.chan_state.flat_writes();
			// flat[0] is XON; rest should accumulate "hello".
			flat.len() > b"hello".len() && &flat[1..1 + b"hello".len()] == b"hello"
		}),
		"payload never arrived on channel; got {:?}",
		fx.chan_state.flat_writes()
	);

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
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
	let fx = setup_channel();
	let client = common::connect_pipe_client("RdPipeTest", fx.addr, Duration::from_secs(5));

	// Drop client so the plugin's reader observes the disconnect; this is the
	// path the plugin's writer release uses (end-of-reader-loop clears the slot).
	drop(client);

	// Call OnClose -> plugin signals shutdown and releases the writer slot.
	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}

	// Verify the writer is released: subsequent OnDataReceived must
	// return ERROR_PIPE_NOT_CONNECTED because the writer slot is None.
	let mut last = None;
	assert!(
		wait_until(Duration::from_secs(5), || {
			let result = unsafe { fx.chan_cb.OnDataReceived(b"after-close") };
			let released = matches!(
				&result,
				Err(e) if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into()
			);
			last = Some(result);
			released
		}),
		"OnDataReceived after OnClose never returned ERROR_PIPE_NOT_CONNECTED; last: {last:?}"
	);
}

#[test]
#[serial]
fn multiple_channels_produce_multiple_listeners() {
	let hkcu = common::HkcuOverride::new().expect("override hkcu");
	hkcu.write_channel_names(&["ChanA", "ChanB"]).expect("write channel names");

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

	let expected: std::collections::HashSet<String> =
		["ChanA".to_string(), "ChanB".to_string()].into_iter().collect();
	// HKLM is not redirected, so machines with rd_pipe registered in HKLM
	// contribute extra channel names. Only assert expected ⊆ names.
	assert!(
		expected.is_subset(&names),
		"missing expected listener names. expected subset: {expected:?}, actual: {names:?}"
	);

	drop(plugin);
}

/// After a client disconnects, the plugin keeps the pipe name claimed
/// (same instance disconnected and reused), so a second client can connect
/// and the channel pump keeps working — in both directions.
#[test]
#[serial]
fn pipe_client_can_reconnect_after_disconnect() {
	let fx = setup_channel();
	let client = fx.connect_client_and_wait_for_xon();

	// Disconnect and wait for XOFF so the plugin has observed it.
	drop(client);
	wait_for_xoff(&fx.chan_state);

	// Second client connects to the same, still-claimed pipe name.
	let mut client2 = common::connect_pipe_client("RdPipeTest", fx.addr, Duration::from_secs(5));

	// Wait for the second XON.
	assert!(
		wait_until(Duration::from_secs(5), || {
			fx.chan_state.flat_writes().iter().filter(|&&b| b == 0x11u8).count() >= 2
		}),
		"second XON never written; got {:?}",
		fx.chan_state.flat_writes()
	);

	// Pipe -> channel still pumps after the reconnect.
	client2.write_all(b"again").expect("pipe write");
	client2.flush().expect("pipe flush");
	assert!(
		wait_until(Duration::from_secs(5), || {
			fx.chan_state.flat_writes().windows(b"again".len()).any(|w| w == b"again")
		}),
		"payload never arrived on channel after reconnect; got {:?}",
		fx.chan_state.flat_writes()
	);

	// Channel -> pipe still pumps after the reconnect.
	unsafe {
		fx.chan_cb.OnDataReceived(b"back").expect("OnDataReceived after reconnect");
	}
	let got = common::read_exact_with_timeout(&client2, 4, Duration::from_secs(5))
		.expect("read after reconnect");
	assert_eq!(&got, b"back");

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

/// A pipe client that stops reading must never block the RDS callback
/// thread: `OnDataReceived` queues via `try_send` and, once the bounded
/// queue overflows, the stalled client is disconnected.
#[test]
#[serial]
fn stalled_client_disconnected_at_cap() {
	let fx = setup_channel();

	// Client connects and never reads.
	let client = fx.connect_client_and_wait_for_xon();

	// Flood the channel with max-size chunks. Every call must return
	// promptly; once the queue cap trips, the plugin disconnects the
	// stalled client and later calls fail with ERROR_PIPE_NOT_CONNECTED.
	let chunk = vec![0xA5u8; 64 * 1024];
	let mut disconnected = false;
	for _ in 0..1000 {
		let started = Instant::now();
		let result = unsafe { fx.chan_cb.OnDataReceived(&chunk) };
		let elapsed = started.elapsed();
		assert!(
			elapsed < Duration::from_secs(2),
			"OnDataReceived blocked for {elapsed:?} with a stalled client"
		);
		match result {
			Ok(()) => {}
			Err(e) if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into() => {
				disconnected = true;
				break;
			}
			Err(e) => panic!("unexpected OnDataReceived error: {e:?}"),
		}
	}
	assert!(disconnected, "stalled client was never disconnected");

	// The plugin observed the disconnect: XOFF went to the channel.
	wait_for_xoff(&fx.chan_state);

	// The stalled client's connection is dead.
	let read_result = common::read_exact_with_timeout(&client, 1, Duration::from_secs(5));
	assert!(read_result.is_err(), "expected read failure on disconnected client");

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

/// A single chunk larger than the pipe buffer is forwarded intact: the DVC
/// framework places no upper bound on `OnDataReceived` size, and the blocking
/// pipe write drains fully as the client reads. The channel keeps working
/// afterwards.
#[test]
#[serial]
fn oversized_chunk_forwarded() {
	let fx = setup_channel();
	let client = fx.connect_client_and_wait_for_xon();

	// Four times the pipe buffer, so the write spans several buffer fills and
	// an index-based pattern catches any truncation or reordering.
	let oversized: Vec<u8> = (0..256 * 1024).map(|i| (i % 256) as u8).collect();
	unsafe {
		fx.chan_cb.OnDataReceived(&oversized).expect("OnDataReceived for oversized chunk");
	}
	let got = common::read_exact_with_timeout(&client, oversized.len(), Duration::from_secs(10))
		.expect("read oversized chunk");
	assert_eq!(got, oversized, "oversized chunk was not forwarded intact");

	// Channel still pumps normally after the large write.
	unsafe {
		fx.chan_cb.OnDataReceived(b"still-alive").expect("OnDataReceived after oversized");
	}
	let tail =
		common::read_exact_with_timeout(&client, b"still-alive".len(), Duration::from_secs(5))
			.expect("read after oversized");
	assert_eq!(&tail, b"still-alive");

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}
}

/// Teardown discards queued writer data rather than draining it: `OnClose`
/// with unwritten chunks queued to a non-reading client severs the
/// connection promptly instead of hanging on the parked writer.
#[test]
#[serial]
fn on_close_with_queued_data_discards_and_disconnects() {
	let fx = setup_channel();

	// Client connects and never reads.
	let mut client = fx.connect_client_and_wait_for_xon();

	// Two max-size chunks: the first fills the outbound pipe buffer, the
	// second parks the writer thread in a pending overlapped write.
	let chunk = vec![0xC3u8; 64 * 1024];
	unsafe {
		fx.chan_cb.OnDataReceived(&chunk).expect("first OnDataReceived");
		fx.chan_cb.OnDataReceived(&chunk).expect("second OnDataReceived");
	}
	// Give the writer time to reach the pending write before closing.
	std::thread::sleep(Duration::from_millis(100));

	unsafe {
		fx.chan_cb.OnClose().expect("OnClose");
	}

	// Shutdown must cut the parked writer off and disconnect the instance;
	// the client observes that as failing writes. A hang in teardown would
	// leave the connection alive and trip this timeout.
	assert!(
		wait_until(Duration::from_secs(5), || client.write(b"x").is_err()),
		"pipe never disconnected after OnClose with queued data"
	);

	// The writer slot is gone as well.
	let r = unsafe { fx.chan_cb.OnDataReceived(b"\xab") };
	assert!(
		matches!(
			r,
			Err(ref e) if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into()
		),
		"expected ERROR_PIPE_NOT_CONNECTED after OnClose, got {r:?}"
	);
}

/// A channel whose `Write` starts failing (transport died before `OnClose`)
/// must tear down like a close: the connected client observes a graceful
/// end-of-pipe, not a forced disconnect.
#[test]
#[serial]
fn channel_write_failure_closes_pipe_gracefully() {
	let fx = setup_channel();
	let mut client = fx.connect_client_and_wait_for_xon();

	fx.chan_state.fail_writes.store(true, std::sync::atomic::Ordering::SeqCst);
	client.write_all(b"boom").expect("pipe write");
	client.flush().expect("pipe flush");

	// Graceful instance close = EOF (read_exact reports UnexpectedEof);
	// a forced DisconnectNamedPipe surfaces as raw OS error 233.
	let err = common::read_exact_with_timeout(&client, 1, Duration::from_secs(5))
		.expect_err("expected end-of-pipe");
	assert_ne!(err.raw_os_error(), Some(233), "client saw a forced disconnect: {err:?}");
	assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof, "expected graceful EOF, got {err:?}");
}

/// Regression test for issue #57: `OnClose` must terminate the reader
/// while the client is still connected and any subsequent `OnDataReceived`
/// must fail with `ERROR_PIPE_NOT_CONNECTED`. `OnClose` synchronously takes
/// the writer slot and signals the shutdown event, so this assertion holds
/// without polling.
#[test]
#[serial]
fn on_close_terminates_reader_cooperatively_while_client_connected() {
	let fx = setup_channel();
	let _client = fx.connect_client_and_wait_for_xon();

	// Cooperative shutdown: OnClose while the client is still connected.
	unsafe { fx.chan_cb.OnClose().expect("OnClose") };

	// Subsequent OnDataReceived must fail with ERROR_PIPE_NOT_CONNECTED
	// (writer slot released synchronously by OnClose).
	let r = unsafe { fx.chan_cb.OnDataReceived(b"\xab") };
	assert!(
		matches!(
			r,
			Err(ref e) if e.code() == windows::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED.into()
		),
		"expected ERROR_PIPE_NOT_CONNECTED after OnClose, got {:?}",
		r
	);
}
