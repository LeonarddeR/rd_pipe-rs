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

- **Toolchain must be at least `beta` (was `nightly`).** `arm64ec` staticlibs built on a toolchain *without* the TLS-destructors→FLS fix (rust-lang/rust#148799) crash at `0xc0000096` when an ARM64X DLL is loaded from an x64 process on ARM64 Windows (rust-lang/rust#145154). The fix first landed in nightly `1.98.0` (2026-06-03) and, as of 2026-07-17, is in `beta` (`1.98.0-beta.4`, verified: beta contains merge `9c963eec`; arm64ec EC-view tests pass 15/15 on a `windows-11-arm` host). CI therefore uses `beta`. Once `1.98.0` reaches stable the arm64 path can move to `stable`. The `Test (arm64x-on-arm64ec)` job is the gate; it only runs on the `windows-11-arm` runner.

Link recipe (dynamic CRT, MSVC link.exe):
- Both per-arch staticlibs as explicit inputs (`rd_pipe.lib` arm64 + arm64ec)
- `arm64\vcruntime.lib` + `arm64\msvcrt.lib` (arm64 MSVC CRT libs serve both views; arm64\msvcrt.lib provides `__icall_helper_arm64ec` for the EC view's raw_dylib import stubs)
- `um\arm64\{kernel32,ntdll,userenv,ws2_32,dbghelp}` + `um\x64\{same}` + `um\x64\softintrin.lib`
- `/LIBPATH:` for MSVC arm64+x64 and SDK ucrt/um arm64+x64 dirs (resolves `.drectve /defaultlib:` entries without vcvars)
- `/force:multiple` resolves duplicate `DllMain` (arm64 + arm64ec each define it, plus msvcrt's stub); msvcrt's stub wins but correctly chains through `_pRawDllMain` → user DllMain
- No SDK version constraint (both 26100 and 28000 work)

The script can be exercised on a Windows ARM64 host: build both staticlibs+DLLs (`cargo +beta build --release --target {aarch64,arm64ec}-pc-windows-msvc`), run the script, then validate the merged DLL with `RD_PIPE_DLL_PATH=<dll> cargo +beta nextest run --target {aarch64,arm64ec}-pc-windows-msvc -E 'binary(dll_smoke) or binary(dvc_emulation)'`.

## Registration (DllInstall)

`regsvr32 /i:"<flags> <ChannelName1> <ChannelName2> ..." rd_pipe.dll` drives registration via `DllInstall`. Flag chars parsed from arg[0]:

- `c` — COM in-proc server (requires channel names as remaining args)
- `r` — RDP/MSTS Add-In registration
- `x` — Citrix (x86 only)
- `m` — write to `HKLM` instead of `HKCU`

Uninstall: `regsvr32 /u /i:"<flags>"`. See `CMD_*` constants in `src/lib.rs`.

Log level read from `HKCU\...\CLSID\{...}\LogLevel` (fallback HKLM), values 1–5 → tracing `Level`. Logs written to `%TEMP%\RdPipe.log`.

## Architecture

Entry points live in `src/lib.rs`: `DllMain` (init tracing), `DllGetClassObject` (hand out `ClassFactory`), `DllInstall` (registry setup). No async runtime — all pipe IO is overlapped Win32 on dedicated `std::thread`s.

Call flow inside an RDS client:

1. RDS loads DLL, calls `DllGetClassObject` for `CLSID_RD_PIPE_PLUGIN`.
2. `class_factory::ClassFactory::CreateInstance` returns an `IWTSPlugin` (`RdPipePlugin`).
3. On `Initialize`, `RdPipePlugin` reads channel names from registry (`ChannelNames` multi-string) and calls `CreateListener` per channel, attaching an `IWTSListenerCallback`.
4. When the server opens a channel, `OnNewChannelConnection` creates the single named-pipe instance (`CreateNamedPipeW`, `FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE`, `max_instances 1` — the name stays claimed for the channel lifetime, incl. across client reconnects) and spawns the **pump thread** (`run_pipe_pump` in `rd_pipe_plugin.rs`). The pipe instance (`Arc<OwnedHandle>`) is owned by the pump and per-connection writer threads, **not** the callback — so it closes as soon as those threads exit.
5. Pump thread: overlapped `ConnectNamedPipe` → XON → per-connection **writer thread** fed by a bounded `std::sync::mpsc::sync_channel` → overlapped `ReadFile` loop forwarding to the COM `IWTSVirtualChannel`. `OnDataReceived` only does `try_send` into the queue (never blocks the RDS thread); a full queue means a stalled client, disconnected via a `Weak<OwnedHandle>` held by the callback. XOFF is written to the channel at each disconnect while the channel is still live. `OnClose` takes the sender and signals a manual-reset shutdown event that wakes every pending overlapped wait (`wait_overlapped` in `overlapped.rs`); the pump then exits **without** disconnecting, so the instance closes as the threads drop it and a connected client observes a graceful end-of-pipe (`ERROR_BROKEN_PIPE`), not a forced `DisconnectNamedPipe` (`ERROR_PIPE_NOT_CONNECTED`). A client-initiated disconnect instead resets the instance (`DisconnectNamedPipe`) and loops to accept the next client. This close/reconnect split mirrors the pre-`dropTokio` behavior.
6. XON/XOFF byte constants gate flow control.

Named-pipe ACL built from caller's **logon SID** via SDDL in `security_descriptor.rs` (`get_logon_sid` + `security_attributes_from_sddl`), so only the interactive user can connect.

Module map:

- `lib.rs` — DLL exports, logging, install dispatcher.
- `class_factory.rs` — `IClassFactory` impl producing `IWTSPlugin`.
- `overlapped.rs` — `OwnedHandle` RAII wrapper, event creation, `Shutdown` signal, `wait_overlapped` (op event + shutdown event pair, `CancelIoEx` on shutdown), `run_overlapped` (issue op, consume sync completion or await pending).
- `rd_pipe_plugin.rs` — plugin, listener callback, channel callback, pump/writer threads. **Core of the crate.**
- `registry.rs` — CLSID constant, registry path constants, add/delete helpers for InprocServer, MSTS AddIns, Citrix modules.
- `security_descriptor.rs` — logon SID lookup and SDDL → `SECURITY_ATTRIBUTES` conversion (caller must `LocalFree` the descriptor).

Concurrency: two `std` threads per connected channel (pump + writer), no runtime. COM interfaces crossed between threads are wrapped in `AgileReference`. The pipe handle is an `Arc<OwnedHandle>` shared only by those two threads; the callback keeps a `Weak` clone (to disconnect a stalled client without extending the instance's lifetime), so the instance closes promptly when the threads exit. The writer queue sender sits behind an `Arc<parking_lot::Mutex<Option<SyncSender>>>`; taking it is the synchronous "pipe gone" signal for `OnDataReceived`.

## Testing Notes

Unit tests across modules. Tests never mutate the live registry, never open real RDS channels. Full plugin lifecycle (`DllMain`, `DllInstall`, `IWTSPlugin::Initialize`) requires a live RDS session and is not covered by unit tests. `security_descriptor` tests may fail in restricted/CI contexts lacking a logon session — they're written to degrade gracefully. See `TESTING.md` for per-module breakdown before adding tests.
