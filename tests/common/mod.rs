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
