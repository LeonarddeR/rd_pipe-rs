# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

First stable release. The pipe transport and registration interface as
shipped in 0.9.0 have proven stable in production use; from this release
on, changes to them follow the semantic versioning compatibility
guarantees.

### Changed

- The arm64, arm64ec and arm64x CI targets build on `stable` instead of
  `beta`, now that the arm64ec TLS-destructors→FLS fix
  (rust-lang/rust#148799) has reached stable in Rust 1.98.0. Every CI
  target is on the same toolchain again, so the per-target `toolchain`
  matrix key is gone.
- Declared a minimum supported Rust version of `1.98` for the whole
  crate. The requirement originates with arm64ec — an ARM64X image built
  without rust-lang/rust#148799 aborts at `0xc0000096` when its EC view
  is loaded — but is enforced for every target so the failure is a clear
  `requires rustc 1.98` message instead of a crash at DLL load.

## [0.9.0] - 2026-07-24

### Changed

- Replaced the tokio-based pipe transport with overlapped Win32 IO on
  dedicated threads; the `tokio`/`tokio-util` dependencies are gone
  entirely (crate and tests).
- `OnDataReceived` no longer blocks the RDS callback thread: data is
  queued to a per-connection writer thread through an unbounded channel
  gated by a 4 MiB queued-bytes budget plus a 65536-chunk backstop. A
  pipe client whose backlog exceeds a budget is disconnected as stalled
  and the tripping call returns `ERROR_PIPE_NOT_CONNECTED`. A received
  chunk is forwarded whole regardless of size, matching the DVC contract
  that places no upper bound on the buffer delivered to
  `OnDataReceived`.
- Client reconnects reuse the same claimed pipe instance without the
  stale-read workaround the tokio/mio stack required.
- `OnClose` now closes the pipe instance gracefully instead of forcing
  `DisconnectNamedPipe`: the writer first drains all accepted data to a
  connected client (bounded at 2 seconds for a client that has stopped
  reading, after which the in-flight write is cancelled), then the
  instance closes as the pump and writer threads exit (the callback
  holds only a `Weak` handle) and the client observes
  `ERROR_BROKEN_PIPE` rather than `ERROR_PIPE_NOT_CONNECTED`. This
  restores the pre-`dropTokio` teardown behavior the RDAccess client
  relies on, including delivery of data sent immediately before the
  close, and stops a channel-close from being logged as an error on the
  consumer side.
- Pipe-instance creation is a single attempt inside
  `OnNewChannelConnection`; a failure now refuses the channel instead of
  accepting it and retrying creation every 100 ms in the background. The
  retry loop predated once-per-channel instance creation; the remaining
  failure modes are persistent, and a refused channel is observable on
  the server side where an accepted-but-pipe-less channel is not.
- Releasing the channel callback without `OnClose` now shuts the pump
  down (shutdown is signalled from `Drop`), and the pump keeps the
  process-wide implicit MTA alive (`CoIncrementMTAUsage`) for its
  lifetime.
- The named-pipe instance is created once per channel and stays claimed
  for the channel lifetime (`FILE_FLAG_FIRST_PIPE_INSTANCE`,
  `max_instances 1`) instead of being dropped and recreated on every
  client disconnect, so the name cannot be taken over between
  reconnects.
- Release builds abort on panic (`panic = "abort"`), so a panic can no
  longer unwind across the COM boundary.
- `Cargo.lock` is tracked, making the shipped dependency resolution the
  one that gets audited.
- The arm64, arm64ec and arm64x CI targets build on `beta` instead of
  `nightly`.

### Fixed

- The COM entry points (`ClassFactory::CreateInstance`,
  `OnNewChannelConnection`, `OnDataReceived`) null-check their pointer
  arguments and handle `cbsize == 0`. `CreateInstance` also nulls its
  out-pointer before validating the IID, so a caller cannot read an
  uninitialized value on failure.
- The `GetTokenInformation` buffer is aligned for the `TOKEN_GROUPS`
  cast.
- `GetModuleFileNameW` grows its buffer on truncation, capped at
  `UNICODE_STRING_MAX_CHARS`.

### Security

- The named-pipe security descriptor grants the logon SID `GR`/`GW`
  instead of `GENERIC_ALL`, and carries a medium-integrity
  no-read-up/no-write-up mandatory label. A process below medium
  integrity in the same session can no longer read or write
  virtual-channel data; consumers must run at medium integrity or
  above.
- Inbound payloads are logged by length only — channel data is never
  written to the log as raw bytes.
- A scheduled `cargo-deny` workflow (`deny.toml`) gates the dependency
  graph on advisories, licenses and sources.

## [0.8.0]

- Released via the GitHub Actions CI pipeline (multi-target build: x86, x64,
  arm64, arm64ec, and the merged ARM64X DLL).
