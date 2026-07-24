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

## [0.8.0]

- Released via the GitHub Actions CI pipeline (multi-target build: x86, x64,
  arm64, arm64ec, and the merged ARM64X DLL).
