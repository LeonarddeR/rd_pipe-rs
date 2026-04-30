// Copyright (C) 2026 Leonard de Ruijter
// Shared helpers for integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

/// Resolve the path to the built `rd_pipe.dll` for the current Cargo profile and target.
///
/// Honors `CARGO_TARGET_DIR` when set; falls back to `<manifest>/target`.
/// Profile is derived from `cfg!(debug_assertions)`.
/// Resolves to the DLL matching the compile-time target triple.
///
/// `cargo test` does NOT build the cdylib for these integration tests
/// (libloading uses it at runtime, no link dependency exists). Run
/// `cargo build --target <triple>` before `cargo test --target <triple>`.
pub fn dll_path() -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    // Compile-time target triple. Rust reports `target_arch = "arm64ec"` for
    // arm64ec-pc-windows-msvc on supported toolchains.
    let target_triple = if cfg!(target_arch = "arm64ec") {
        "arm64ec-pc-windows-msvc"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-pc-windows-msvc"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_arch = "x86") {
        "i686-pc-windows-msvc"
    } else {
        ""
    };

    if !target_triple.is_empty() {
        let target_path = target_dir
            .join(target_triple)
            .join(profile)
            .join("rd_pipe.dll");
        if target_path.is_file() {
            return target_path;
        }
    }

    // Fallback: try the default (host arch) path.
    let default_path = target_dir.join(profile).join("rd_pipe.dll");
    if default_path.is_file() {
        return default_path;
    }

    // Give up gracefully — return the most likely path so the eventual
    // `Library::new` error is informative. Do NOT scan target subdirs
    // (could pick a wrong-arch DLL and trigger STATUS_BAD_IMAGE_FORMAT).
    if !target_triple.is_empty() {
        target_dir
            .join(target_triple)
            .join(profile)
            .join("rd_pipe.dll")
    } else {
        default_path
    }
}

use std::io;
use tempfile::NamedTempFile;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, RegLoadAppKeyW, RegOverridePredefKey,
};
use windows::core::PCWSTR;

/// RAII guard that:
/// 1. Loads a private hive file via `RegLoadAppKeyW`.
/// 2. Redirects `HKEY_CURRENT_USER` for this process to that hive via
///    `RegOverridePredefKey`.
/// 3. On drop: clears the override, closes the hive (via `Key`'s Drop),
///    deletes the temp file.
///
/// Isolates `HKEY_CURRENT_USER` reads and writes for the current process to
/// the private hive. Other predefined hives (notably `HKEY_LOCAL_MACHINE`)
/// are NOT overridden by this helper and may still be read by code under
/// test. The plugin's `Initialize` for example reads `ChannelNames` from
/// both HKCU and HKLM; only HKCU is isolated here.
pub struct HkcuOverride {
    // Field order: `hive` Drop runs before `_file` Drop. `Key`'s Drop calls
    // `RegCloseKey`. NamedTempFile's Drop deletes the file.
    hive: windows_registry::Key,
    _file: NamedTempFile,
}

impl HkcuOverride {
    pub fn new() -> io::Result<Self> {
        // Reserve a unique filename, then delete the file so RegLoadAppKey
        // can create a hive at that path. NamedTempFile keeps the path
        // reserved for cleanup-on-drop.
        let temp = tempfile::Builder::new()
            .prefix("rd_pipe_test_hive_")
            .suffix(".dat")
            .tempfile()?;
        let path = temp.path().to_owned();
        // Remove the empty file; RegLoadAppKey requires no file at the path.
        std::fs::remove_file(&path)?;

        let path_w: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut raw_hive = HKEY::default();
        let rc = unsafe {
            RegLoadAppKeyW(
                PCWSTR(path_w.as_ptr()),
                &mut raw_hive,
                KEY_ALL_ACCESS.0,
                0,
                None,
            )
        };
        if rc != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(rc.0 as i32));
        }
        // Take ownership of the handle via windows_registry::Key — its Drop
        // will RegCloseKey it for us.
        let hive = unsafe { windows_registry::Key::from_raw(raw_hive.0 as _) };

        let rc = unsafe { RegOverridePredefKey(HKEY_CURRENT_USER, Some(raw_hive)) };
        if rc != ERROR_SUCCESS {
            // hive's Drop will close the handle.
            return Err(io::Error::from_raw_os_error(rc.0 as i32));
        }

        Ok(Self { hive, _file: temp })
    }

    /// Write `ChannelNames` (REG_MULTI_SZ) at the plugin's CLSID subkey
    /// inside the redirected hive.
    pub fn write_channel_names(&self, names: &[&str]) -> windows_core::Result<()> {
        const SUBKEY: &str = r"Software\Classes\CLSID\{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}";
        let sub = self.hive.create(SUBKEY)?;
        sub.set_multi_string("ChannelNames", names)
    }
}

impl Drop for HkcuOverride {
    fn drop(&mut self) {
        // Restore HKCU first so subsequent code in this process sees the
        // real hive. The `hive` Key's Drop closes the handle; NamedTempFile
        // deletes the file.
        unsafe {
            let _ = RegOverridePredefKey(HKEY_CURRENT_USER, None);
        }
    }
}

use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::System::RemoteDesktop::IWTSVirtualChannel;
use windows::core::implement;

/// Shared state exposed to the test for assertion after plugin calls.
#[derive(Default)]
pub struct FakeChannelState {
    pub writes: Mutex<Vec<Vec<u8>>>,
    pub closed: AtomicBool,
}

impl FakeChannelState {
    pub fn snapshot_writes(&self) -> Vec<Vec<u8>> {
        self.writes.lock().clone()
    }

    pub fn flat_writes(&self) -> Vec<u8> {
        self.writes.lock().iter().flatten().copied().collect()
    }
}

/// Fake `IWTSVirtualChannel` that captures every `Write` payload and
/// records `Close` — no real RDS session involved.
#[implement(IWTSVirtualChannel)]
pub struct FakeVirtualChannel {
    pub state: Arc<FakeChannelState>,
}

impl FakeVirtualChannel {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> (IWTSVirtualChannel, Arc<FakeChannelState>) {
        let state = Arc::new(FakeChannelState::default());
        let iface: IWTSVirtualChannel = FakeVirtualChannel {
            state: state.clone(),
        }
        .into();
        (iface, state)
    }
}

impl windows::Win32::System::RemoteDesktop::IWTSVirtualChannel_Impl for FakeVirtualChannel_Impl {
    fn Write(
        &self,
        cbsize: u32,
        pbuffer: *const u8,
        _preserved: windows_core::Ref<windows_core::IUnknown>,
    ) -> windows_core::Result<()> {
        let buf = unsafe { std::slice::from_raw_parts(pbuffer, cbsize as usize) }.to_vec();
        self.state.writes.lock().push(buf);
        Ok(())
    }

    fn Close(&self) -> windows_core::Result<()> {
        self.state.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

use windows::Win32::System::RemoteDesktop::{
    IWTSListener, IWTSListenerCallback, IWTSVirtualChannelManager,
};

/// Stub listener — the plugin never calls `GetConfiguration` in our tests,
/// but the interface requires an impl.
#[implement(IWTSListener)]
pub struct FakeListener;

impl windows::Win32::System::RemoteDesktop::IWTSListener_Impl for FakeListener_Impl {
    fn GetConfiguration(
        &self,
    ) -> windows_core::Result<windows::Win32::System::Com::StructuredStorage::IPropertyBag> {
        Err(windows_core::Error::from_hresult(
            windows::Win32::Foundation::E_NOTIMPL,
        ))
    }
}

#[derive(Debug, Clone)]
pub enum MgrEvent {
    CreateListener { name: String },
}

#[derive(Default)]
pub struct FakeMgrState {
    pub events: Mutex<Vec<MgrEvent>>,
    pub listeners: Mutex<Vec<(String, IWTSListenerCallback)>>,
}

/// Fake `IWTSVirtualChannelManager` that records `CreateListener` calls so
/// tests can later retrieve the callbacks and drive `OnNewChannelConnection`.
#[implement(IWTSVirtualChannelManager)]
pub struct FakeChannelMgr {
    pub state: Arc<FakeMgrState>,
}

impl FakeChannelMgr {
    #[allow(clippy::new_ret_no_self, clippy::arc_with_non_send_sync)]
    pub fn new() -> (IWTSVirtualChannelManager, Arc<FakeMgrState>) {
        let state = Arc::new(FakeMgrState::default());
        let iface: IWTSVirtualChannelManager = FakeChannelMgr {
            state: state.clone(),
        }
        .into();
        (iface, state)
    }
}

impl windows::Win32::System::RemoteDesktop::IWTSVirtualChannelManager_Impl for FakeChannelMgr_Impl {
    fn CreateListener(
        &self,
        pszchannelname: &windows_core::PCSTR,
        _uflags: u32,
        plistenercallback: windows_core::Ref<IWTSListenerCallback>,
    ) -> windows_core::Result<IWTSListener> {
        let name = unsafe { pszchannelname.to_string() }.map_err(|_| {
            windows_core::Error::from_hresult(windows::Win32::Foundation::E_UNEXPECTED)
        })?;
        let cb = plistenercallback
            .as_ref()
            .ok_or_else(|| {
                windows_core::Error::from_hresult(windows::Win32::Foundation::E_UNEXPECTED)
            })?
            .clone();

        self.state
            .events
            .lock()
            .push(MgrEvent::CreateListener { name: name.clone() });
        self.state.listeners.lock().push((name, cb));

        Ok(FakeListener.into())
    }
}

use libloading::Library;
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::RemoteDesktop::IWTSPlugin;
use windows::core::{GUID, HRESULT, Interface};
use windows_core::{OutRef, Ref};

pub const CLSID_RD_PIPE_PLUGIN: GUID = GUID::from_u128(0xD1F74DC7_9FDE_45BE_9251_FA72D4064DA3);

pub type DllGetClassObjectFn = unsafe extern "system" fn(
    rclsid: Ref<GUID>,
    riid: Ref<GUID>,
    ppv: OutRef<IClassFactory>,
) -> HRESULT;

/// Owns the loaded `rd_pipe.dll`. Loaded exactly once per test process via
/// a `OnceLock`; `Library` and `Symbol` outlive the process.
pub struct DllHandle {
    pub get_class_object: libloading::Symbol<'static, DllGetClassObjectFn>,
    // Keep alive to prevent unload; `DllMain(DLL_PROCESS_DETACH)` racing
    // with tracing-subscriber TLS teardown would otherwise abort the
    // test process.
    _lib: &'static Library,
}

// SAFETY: libloading::Symbol is !Send by default but the underlying function
// pointer is fine to call from any thread; we never re-bind the Library and
// process-exit cleanup is the only release path.
unsafe impl Send for DllHandle {}
unsafe impl Sync for DllHandle {}

impl DllHandle {
    /// Returns a process-global handle to the loaded DLL. First call loads
    /// the DLL; subsequent calls return the same reference. This guarantees
    /// `DllMain(DLL_PROCESS_ATTACH)` runs exactly once (so
    /// `tracing_subscriber::fmt().init()` doesn't panic on re-init) and
    /// avoids per-test handle leaks.
    pub fn load() -> &'static DllHandle {
        static HANDLE: std::sync::OnceLock<DllHandle> = std::sync::OnceLock::new();
        HANDLE.get_or_init(|| {
            let path = dll_path();
            let lib = unsafe { Library::new(&path) }
                .unwrap_or_else(|e| panic!("LoadLibrary {path:?} failed: {e}"));
            let lib: &'static Library = Box::leak(Box::new(lib));
            let get_class_object: libloading::Symbol<'static, DllGetClassObjectFn> =
                unsafe { lib.get(b"DllGetClassObject\0") }
                    .expect("DllGetClassObject export missing");
            DllHandle {
                get_class_object,
                _lib: lib,
            }
        })
    }

    /// Access the underlying `libloading::Library` for resolving additional
    /// exports beyond `DllGetClassObject`.
    pub fn lib(&self) -> &'static Library {
        self._lib
    }
}

/// Calls `DllGetClassObject` → `IClassFactory`, then `CreateInstance` →
/// `IWTSPlugin`.
pub fn create_plugin(dll: &DllHandle) -> IWTSPlugin {
    let mut factory: Option<IClassFactory> = None;
    let hr = unsafe {
        (dll.get_class_object)(
            Ref::from(&CLSID_RD_PIPE_PLUGIN),
            Ref::from(&IClassFactory::IID),
            OutRef::from(&mut factory),
        )
    };
    assert!(hr.is_ok(), "DllGetClassObject returned {hr:?}");
    let factory = factory.expect("factory is None after DllGetClassObject");

    unsafe {
        factory
            .CreateInstance::<Option<&windows_core::IUnknown>, IWTSPlugin>(None)
            .expect("CreateInstance(IWTSPlugin) failed")
    }
}

use std::time::{Duration, Instant};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

/// Build the pipe path used by the plugin for a given channel name and
/// channel COM interface address (matches `PIPE_NAME_PREFIX` in rd_pipe_plugin.rs).
pub fn pipe_address(channel_name: &str, channel_addr: usize) -> String {
    format!(r"\\.\pipe\RDPipe_{channel_name}_{channel_addr}")
}

/// Poll `pipe_address(name, addr)` every 25 ms until the pipe is connectable
/// or `deadline` elapses. Returns the connected client.
pub async fn connect_pipe_client(
    channel_name: &str,
    channel_addr: usize,
    deadline: Duration,
) -> NamedPipeClient {
    let addr = pipe_address(channel_name, channel_addr);
    let start = Instant::now();
    loop {
        match ClientOptions::new().open(&addr) {
            Ok(client) => return client,
            Err(_) if start.elapsed() < deadline => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(e) => panic!(
                "pipe {addr} never accepted within {deadline:?}: {e}\n\n--- RdPipe.log tail ---\n{}",
                read_rdpipe_log_tail()
            ),
        }
    }
}

/// Read the tail of `%TEMP%\RdPipe.log` for diagnostics. Returns a placeholder
/// if the log is missing or unreadable.
pub fn read_rdpipe_log_tail() -> String {
    let path = std::env::temp_dir().join("RdPipe.log");
    match std::fs::read_to_string(&path) {
        Ok(s) => {
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.len().saturating_sub(80);
            lines[start..].join("\n")
        }
        Err(e) => format!("(could not read {}: {e})", path.display()),
    }
}

/// Build a single-thread Tokio runtime, run `f` to completion, return the result.
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(f)
}

use windows::Win32::System::RemoteDesktop::IWTSVirtualChannelCallback;
use windows_core::{BOOL, BSTR};

/// Drive `OnNewChannelConnection` on a captured listener callback and return
/// the `IWTSVirtualChannelCallback` the plugin produced.
pub fn trigger_new_channel(
    listener_cb: &IWTSListenerCallback,
    channel: &IWTSVirtualChannel,
) -> IWTSVirtualChannelCallback {
    let bstr = BSTR::new();
    let mut accept = BOOL::default();
    let mut chan_cb: Option<IWTSVirtualChannelCallback> = None;

    unsafe {
        listener_cb
            .OnNewChannelConnection(channel, &bstr, &mut accept, &mut chan_cb)
            .expect("OnNewChannelConnection failed");
    }
    assert!(accept.as_bool(), "plugin refused channel");
    chan_cb.expect("plugin did not return a channel callback")
}

/// Return the COM vtable pointer of an `IWTSVirtualChannel` as a `usize` —
/// the same computation the plugin uses to build the pipe path.
pub fn channel_addr(chan: &IWTSVirtualChannel) -> usize {
    chan.as_raw() as usize
}
