# RD Pipe

RD Pipe is a Windows-only library that bridges Remote Desktop Services [Dynamic Virtual Channels](https://docs.microsoft.com/en-us/windows/win32/termserv/dynamic-virtual-channels) (DVCs) to [named pipes](https://docs.microsoft.com/en-us/windows/win32/ipc/named-pipes).

Data written to the named pipe is sent over the virtual channel to the server, and data received from the server can be read from the named pipe.

```
RDP/Citrix client ── COM (IWTSPlugin) ── rd_pipe.dll ── named pipe ── your app (Python/C#/…)
```

## Why this library

Microsoft exposes two sets of APIs for Dynamic Virtual Channels. The [server APIs](https://docs.microsoft.com/en-us/windows/win32/termserv/dvc-server-apis) are easy to implement, as they are based on file I/O. The [client APIs](https://docs.microsoft.com/en-us/windows/win32/termserv/dvc-client-apis) are much less trivial, since they require implementing a COM server — significant overhead in languages that don't compile to native code.

RD Pipe implements the COM server, exposing a named pipe that can be consumed easily from C#, Python, and similar languages.

## Pipe path

For each opened channel, RD Pipe creates a per-session pipe:

```
\\.\pipe\RdPipe\<session-id>\<channel-name>
```

The pipe ACL is built from the caller's logon SID, so only the interactive user owning the session can connect.

## Building from source

Requires the MSVC Rust toolchain on Windows. Cross-platform builds are not supported.

```
cargo build
cargo build --release --target x86_64-pc-windows-msvc
cargo test
```

CI targets: `i686`, `x86_64`, `aarch64`, `arm64ec` (all `*-pc-windows-msvc`). Citrix registration is x86-only.

## Registration

CLSID: `{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}`.

Register via `regsvr32` calling `DllInstall`. The first argument is a flag string; remaining arguments are channel names.

```
regsvr32 /i:"cr ChannelOne ChannelTwo" rd_pipe.dll
```

Flags:

| Flag | Effect |
|------|--------|
| `c` | Register COM in-proc server (requires channel names) |
| `r` | Register as RDP/MSTS add-in |
| `x` | Register as Citrix module (x86 only) |
| `m` | Write to `HKLM` instead of `HKCU` |

Uninstall:

```
regsvr32 /u /i:"crm" rd_pipe.dll
```

## Logging

Log level is read from `LogLevel` (REG_DWORD, 1–5) under the CLSID key in `HKCU` (fallback `HKLM`), mapped to `tracing` levels. Output is written to `%TEMP%\RdPipe.log`.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
