RD Pipe is a library that offers a [named pipe](https://docs.microsoft.com/en-us/windows/win32/ipc/named-pipes) layer for Windows Remote Desktop Services [Dynamic Virtual Channels](https://docs.microsoft.com/en-us/windows/win32/termserv/dynamic-virtual-channels).
In short, RD Pipe allows you to transmit data over a RDS virtual channel by connecting to a named pipe.
Data written to the named pipe is send over the virtual channel to the server, and data received from the server can be read from the named pipe.

## Why this library

Microsoft has two sets of APIs for Dynamic Virtual Channels. De [server APIs](https://docs.microsoft.com/en-us/windows/win32/termserv/dvc-server-apis) are relatively easi to implement, as they are based on basic file I/O.
On the other hand, implementing the [client APIs](https://docs.microsoft.com/en-us/windows/win32/termserv/dvc-client-apis) is much less trivial as it involves implementing a COM server.
This implies a lot of overhead when done in languages that don't compile to native code.
Therefore, RD Pipe implements the COM server part, exposing a named pipe instead that can be easily consumed in languages like C#, Python, etc.

## Building from source

Building RD Pipe is straight forward when you are acustomed to development in the Rust language.
If not, it is yet pretty simple, as you mainly have to follow the [Rust installation instructions for Windows](https://www.rust-lang.org/tools/install).
After that, building RD Pipe is as easy as executing `cargo build` from the command line.
