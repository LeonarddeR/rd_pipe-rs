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

The `aarch64` + `arm64ec` staticlibs are linked into one ARM64X (hybrid) DLL by `arm64x/build_merged.ps1`, using **MSVC link.exe** (`/machine:arm64x`, discovered via vswhere) and `llvm-readobj` from the active rustc toolchain (needs the `llvm-tools` component). Per-arch export tables are generated on the fly from each DLL via `llvm-readobj --coff-exports`.

**Why MSVC link.exe, not rust-lld (`lld-link.exe`)**: lld corrupts the EC view's TLS directory (`_tls_used`/`_tls_index`/TLS-callback table) when merging two full Rust staticlibs into one ARM64X image, causing `fatal runtime error: the System allocator may not use TLS with destructors` (0xc0000409) on the EC view at test time. MSVC link.exe handles the ARM64X TLS merge correctly.

One hard requirement:

- **Toolchain must be `nightly`.** `arm64ec` staticlibs built on stable/beta crash at `0xc0000096` when an ARM64X DLL is loaded from an x64 process on ARM64 Windows (rust-lang/rust#145154). Fixed by #148799 (TLS dtors → FLS), first in nightly `1.98.0` (2026-06-03). The `Test (arm64x-on-arm64ec)` job is the gate; it only runs on the `windows-11-arm` runner.

Link recipe (dynamic CRT, MSVC link.exe):
- Both per-arch staticlibs as explicit inputs (`rd_pipe.lib` arm64 + arm64ec)
- `arm64\vcruntime.lib` + `arm64\msvcrt.lib` (arm64 MSVC CRT libs serve both views; arm64\msvcrt.lib provides `__icall_helper_arm64ec` for the EC view's raw_dylib import stubs)
- `um\arm64\{kernel32,ntdll,userenv,ws2_32,dbghelp}` + `um\x64\{same}` + `um\x64\softintrin.lib`
- `/LIBPATH:` for MSVC arm64+x64 and SDK ucrt/um arm64+x64 dirs (resolves `.drectve /defaultlib:` entries without vcvars)
- `/force:multiple` resolves duplicate `DllMain` (arm64 + arm64ec each define it, plus msvcrt's stub); msvcrt's stub wins but correctly chains through `_pRawDllMain` → user DllMain
- No SDK version constraint (both 26100 and 28000 work)

The script can be exercised on a Windows ARM64 host: build both staticlibs+DLLs (`cargo +nightly build --release --target {aarch64,arm64ec}-pc-windows-msvc`), run the script, then validate the merged DLL with `RD_PIPE_DLL_PATH=<dll> cargo +nightly nextest run --target {aarch64,arm64ec}-pc-windows-msvc -E 'binary(dll_smoke) or binary(dvc_emulation)'`.

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
