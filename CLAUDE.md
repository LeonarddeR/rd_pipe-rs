# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`rd_pipe` is a Windows-only Rust crate built as `cdylib` + `staticlib`. It implements the Windows Remote Desktop Services **Dynamic Virtual Channel (DVC) client-side COM server** and bridges each DVC to a **named pipe**, so non-native consumers (Python, C#, etc.) can read/write virtual-channel data without implementing COM themselves.

The DLL is loaded into the RDP/Citrix client process and registered as an in-proc COM server (CLSID `{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}`).

## Build & Test

Windows + MSVC toolchain required. Cross-platform builds fail (`windows-core` is Windows-only).

```
cargo build                                        # debug
cargo build --release --target x86_64-pc-windows-msvc
cargo test                                         # all unit tests
cargo test --lib registry                          # one module
cargo test -- --nocapture                          # show println!/trace
cargo fmt --all -- --check                         # CI style check
```

CI matrix targets: `i686`, `x86_64`, `aarch64`, `arm64ec` — all `*-pc-windows-msvc`. Citrix registration code (`ctx_*`) is `#[cfg(target_arch = "x86")]` only.

## ARM64X merged DLL (`arm64x/build_merged.ps1`)

The `aarch64` + `arm64ec` staticlibs are linked into one ARM64X (hybrid) DLL by `arm64x/build_merged.ps1`, using `rust-lld` (`/machine:arm64x`) and `llvm-readobj` from the active rustc toolchain (needs the `llvm-tools` component). Per-arch export tables are generated on the fly from each DLL via `llvm-readobj --coff-exports`.

Two hard requirements, both learned the hard way:

- **Toolchain must be `nightly`.** `arm64ec` staticlibs built on stable/beta crash at `0xc0000096` when an ARM64X DLL is loaded from an x64 process on ARM64 Windows (rust-lang/rust#145154). Fixed by #148799 (TLS dtors → FLS), first in nightly `1.98.0` (2026-06-03). The `Test (arm64x-on-arm64ec)` job is the gate; it only runs on the `windows-11-arm` runner.
- **Windows SDK >= 10.0.28000 on the linking runner.** With SDK 10.0.26100 the linker pulls `msvcrt.lib`'s vcstartup glue (`utility.obj`), which references static-only `__vcrt_initialize` / `__acrt_initialize` and fails the link. Pulling in the static `libvcruntime`/`libucrt` makes it link but double-initialises the CRT → merged DLL crashes at load (`0xc0000005`). SDK >= 28000 doesn't pull `utility.obj`, so a **dynamic-CRT-only** link (kernel32/msvcrt/ucrt/vcruntime import libs, both arches by full path, no ambient `LIB`) is correct and runtime-safe. `Get-SdkLibRoot` enforces the version; the `build-arm64x` CI job installs it via `winget`.

The script can be exercised on a Windows ARM64 host: build both staticlibs+DLLs (`cargo +nightly build --release --target {aarch64,arm64ec}-pc-windows-msvc`), run the script with `$env:LIB=''`, then validate the merged DLL with `RD_PIPE_DLL_PATH=<dll> cargo +nightly nextest run --target aarch64-pc-windows-msvc -E 'binary(dll_smoke) or binary(dvc_emulation)'`.

## Registration (DllInstall)

`regsvr32 /i:"<flags> <ChannelName1> <ChannelName2> ..." rd_pipe.dll` drives registration via `DllInstall`. Flag chars parsed from arg[0]:

- `c` — COM in-proc server (requires channel names as remaining args)
- `r` — RDP/MSTS Add-In registration
- `x` — Citrix (x86 only)
- `m` — write to `HKLM` instead of `HKCU`

Uninstall: `regsvr32 /u /i:"<flags>"`. See `CMD_*` constants in `src/lib.rs`.

Log level read from `HKCU\...\CLSID\{...}\LogLevel` (fallback HKLM), values 1–5 → tracing `Level`. Logs written to `%TEMP%\RdPipe.log`.

## Architecture

Entry points live in `src/lib.rs`: `DllMain` (init tracing + async runtime), `DllGetClassObject` (hand out `ClassFactory`), `DllInstall` (registry setup).

Call flow inside an RDS client:

1. RDS loads DLL, calls `DllGetClassObject` for `CLSID_RD_PIPE_PLUGIN`.
2. `class_factory::ClassFactory::CreateInstance` returns an `IWTSPlugin` (`RdPipePlugin`).
3. On `Initialize`, `RdPipePlugin` reads channel names from registry (`ChannelNames` multi-string) and calls `CreateListener` per channel, attaching an `IWTSListenerCallback`.
4. When the server opens a channel, `OnNewChannelConnection` builds an `IWTSVirtualChannelCallback` and spawns — on the global Tokio runtime (`ASYNC_RUNTIME` in `lib.rs`) — a named-pipe server at `\\.\pipe\RdPipe\<session>\<channel>` (see `rd_pipe_plugin.rs`).
5. Pipe ↔ channel pump runs via `tokio::io::split`; writes to channel use the COM `IWTSVirtualChannel`, reads from channel delivered via `OnDataReceived` forwarded to pipe write half (held behind `parking_lot::Mutex<Arc<...>>`).
6. XON/XOFF byte constants gate flow control.

Named-pipe ACL built from caller's **logon SID** via SDDL in `security_descriptor.rs` (`get_logon_sid` + `security_attributes_from_sddl`), so only the interactive user can connect.

Module map:

- `lib.rs` — DLL exports, runtime, logging, install dispatcher.
- `class_factory.rs` — `IClassFactory` impl producing `IWTSPlugin`.
- `rd_pipe_plugin.rs` — plugin, listener callback, channel callback, named-pipe pump. **Core of the crate.**
- `registry.rs` — CLSID constant, registry path constants, add/delete helpers for InprocServer, MSTS AddIns, Citrix modules.
- `security_descriptor.rs` — logon SID lookup and SDDL → `SECURITY_ATTRIBUTES` conversion (caller must `LocalFree` the descriptor).

Concurrency: single global multi-thread Tokio runtime (`LazyLock<Runtime>`). COM interfaces crossed between threads are wrapped in `AgileReference`. Shared pipe write-halves guarded by `parking_lot::Mutex`.

## Testing Notes

Unit tests across modules. Tests never mutate the live registry, never open real RDS channels. Full plugin lifecycle (`DllMain`, `DllInstall`, `IWTSPlugin::Initialize`) requires a live RDS session and is not covered by unit tests. `security_descriptor` tests may fail in restricted/CI contexts lacking a logon session — they're written to degrade gracefully. See `TESTING.md` for per-module breakdown before adding tests.
