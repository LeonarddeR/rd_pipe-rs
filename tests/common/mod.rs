// Copyright (C) 2026 Leonard de Ruijter
// Shared helpers for integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

/// Resolve the path to the built `rd_pipe.dll` for the current Cargo profile and target.
///
/// Honors `CARGO_TARGET_DIR` when set; falls back to `<manifest>/target`.
/// Profile is derived from `cfg!(debug_assertions)`.
/// Resolves to the DLL matching the compile-time target architecture.
pub fn dll_path() -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
        });

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    // Use compile-time cfg! to determine which target triple we need
    #[cfg(target_arch = "x86_64")]
    let target_triple = "x86_64-pc-windows-msvc";
    #[cfg(target_arch = "aarch64")]
    let target_triple = "aarch64-pc-windows-msvc";
    #[cfg(target_arch = "arm64ec")]
    let target_triple = "arm64ec-pc-windows-msvc";
    #[cfg(target_arch = "x86")]
    let target_triple = "i686-pc-windows-msvc";

    // First, try the target-specific path
    let target_path = target_dir.join(target_triple).join(profile).join("rd_pipe.dll");
    if target_path.is_file() {
        return target_path;
    }

    // Fallback: try the default (host arch) path
    let default_path = target_dir.join(profile).join("rd_pipe.dll");
    if default_path.is_file() {
        return default_path;
    }

    // Last resort: scan for any matching DLL and return first match
    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join(profile).join("rd_pipe.dll");
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    // If nothing found, return the target-specific path (so Library::new error is informative)
    target_path
}

use std::io;
use tempfile::NamedTempFile;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, RegCloseKey, RegLoadAppKeyW, RegOverridePredefKey,
};
use windows::core::PCWSTR;

/// RAII guard that:
/// 1. Loads a private hive file via `RegLoadAppKeyW`.
/// 2. Redirects `HKEY_CURRENT_USER` for this process to that hive via
///    `RegOverridePredefKey`.
/// 3. On drop: clears the override, closes the hive, deletes the file.
///
/// The live registry is never read or written.
pub struct HkcuOverride {
    hive: HKEY,
    // Field order: hive closed first in Drop, then NamedTempFile drops and deletes the file.
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

        let mut hive = HKEY::default();
        let rc = unsafe {
            RegLoadAppKeyW(PCWSTR(path_w.as_ptr()), &mut hive, KEY_ALL_ACCESS.0, 0, None)
        };
        if rc != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(rc.0 as i32));
        }

        let rc = unsafe { RegOverridePredefKey(HKEY_CURRENT_USER, Some(hive)) };
        if rc != ERROR_SUCCESS {
            unsafe {
                let _ = RegCloseKey(hive);
            }
            return Err(io::Error::from_raw_os_error(rc.0 as i32));
        }

        Ok(Self { hive, _file: temp })
    }

    /// Write `ChannelNames` (REG_MULTI_SZ) at the plugin's CLSID subkey
    /// inside the redirected hive, using the `windows-registry` crate.
    pub fn write_channel_names(&self, names: &[&str]) -> windows_core::Result<()> {
        const SUBKEY: &str =
            r"Software\Classes\CLSID\{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}";
        // Wrap the raw HKEY in a windows_registry::Key. Drop of `root` calls
        // RegCloseKey on the duplicated handle — but Key::from_raw stores the
        // handle by-value. To avoid double-close on `self.hive` at Drop,
        // we forget `root` before returning.
        let root = unsafe { windows_registry::Key::from_raw(self.hive.0 as _) };
        let result = (|| -> windows_core::Result<()> {
            let sub = root.create(SUBKEY)?;
            sub.set_multi_string("ChannelNames", names)
        })();
        // Don't let `root` close `self.hive`.
        std::mem::forget(root);
        result
    }
}

impl Drop for HkcuOverride {
    fn drop(&mut self) {
        unsafe {
            let _ = RegOverridePredefKey(HKEY_CURRENT_USER, None);
            let _ = RegCloseKey(self.hive);
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
    pub fn new() -> (IWTSVirtualChannel, Arc<FakeChannelState>) {
        let state = Arc::new(FakeChannelState::default());
        let iface: IWTSVirtualChannel = FakeVirtualChannel { state: state.clone() }.into();
        (iface, state)
    }
}

impl windows::Win32::System::RemoteDesktop::IWTSVirtualChannel_Impl
    for FakeVirtualChannel_Impl
{
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
