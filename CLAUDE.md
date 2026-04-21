# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`rd_pipe` is a Windows-only Rust crate built as `cdylib` + `staticlib`. It implements the Windows Remote Desktop Services **Dynamic Virtual Channel (DVC) client-side COM server** and bridges each DVC to a **named pipe**, so non-native consumers (Python, C#, etc.) can read/write virtual-channel data without implementing COM themselves.

The DLL is loaded into the RDP/Citrix client process and registered as an in-proc COM server (CLSID `{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}`).

## Build & Test

Windows + MSVC toolchain required. Cross-platform builds fail (`windows-future` / `windows-core` incompat).

```
cargo build                                        # debug
cargo build --release --target x86_64-pc-windows-msvc
cargo test                                         # all unit tests
cargo test --lib registry                          # one module
cargo test -- --nocapture                          # show println!/trace
cargo fmt --all -- --check                         # CI style check
```

CI matrix targets: `i686`, `x86_64`, `aarch64`, `arm64ec` — all `*-pc-windows-msvc`. Citrix registration code (`ctx_*`) is `#[cfg(target_arch = "x86")]` only.

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
