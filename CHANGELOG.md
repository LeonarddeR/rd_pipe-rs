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
  chunks) is exceeded, and single chunks over 64 KiB are rejected.
- Client reconnects reuse the same claimed pipe instance without the
  stale-read workaround the tokio/mio stack required.

## [0.8.0]

- Released via the GitHub Actions CI pipeline (multi-target build: x86, x64,
  arm64, arm64ec, and the merged ARM64X DLL).
