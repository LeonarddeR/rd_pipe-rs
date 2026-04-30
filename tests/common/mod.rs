// Copyright (C) 2026 Leonard de Ruijter
// Shared helpers for integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

/// Resolve the path to the built `rd_pipe.dll` for the current Cargo profile.
///
/// Honors `CARGO_TARGET_DIR` when set; falls back to `<manifest>/target`.
/// Profile is derived from `cfg!(debug_assertions)`.
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

    // `cargo test` builds the cdylib for the host triple under
    // `target/<profile>/`; cross-compiled runs use `target/<triple>/<profile>/`.
    // Try the host path first, then the most-recent triple-scoped path.
    let host_path = target_dir.join(profile).join("rd_pipe.dll");
    if host_path.is_file() {
        return host_path;
    }

    // Fallback: scan `target/*/profile/rd_pipe.dll` for the first match.
    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join(profile).join("rd_pipe.dll");
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    // Give up gracefully — return the host path so the eventual
    // `Library::new` error message is informative.
    host_path
}
