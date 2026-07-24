# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Replaced the tokio-based pipe transport with overlapped Win32 IO on
  dedicated threads; the `tokio`/`tokio-util` dependencies are gone
  entirely (crate and tests).
- `OnDataReceived` no longer blocks the RDS callback thread: data is
  queued to a per-connection writer thread via a bounded queue. A pipe
  client that stops reading is disconnected once the queue cap (512
  chunks) is exceeded. A received chunk is forwarded whole regardless of
  size, matching the DVC contract that places no upper bound on the
  buffer delivered to `OnDataReceived`.
- Client reconnects reuse the same claimed pipe instance without the
  stale-read workaround the tokio/mio stack required.
- `OnClose` now closes the pipe instance gracefully instead of forcing
  `DisconnectNamedPipe`: the instance is owned by the pump and writer
  threads (the callback holds only a `Weak` handle), so it closes as they
  exit and a connected client observes `ERROR_BROKEN_PIPE` rather than
  `ERROR_PIPE_NOT_CONNECTED`. This restores the pre-`dropTokio` teardown
  behavior the RDAccess client relies on and stops a channel-close from
  being logged as an error on the consumer side.

## [0.8.0]

- Released via the GitHub Actions CI pipeline (multi-target build: x86, x64,
  arm64, arm64ec, and the merged ARM64X DLL).
