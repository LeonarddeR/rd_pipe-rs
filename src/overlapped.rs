// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Overlapped IO primitives: event creation, the shutdown signal and the overlapped wait/run helpers
// Copyright (C) 2022-2026 Leonard de Ruijter <alderuijter@gmail.com>
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::os::windows::io::{AsRawHandle, HandleOrNull, OwnedHandle};
use tracing::error;
use windows_core::{Error, Result, WIN32_ERROR};

use crate::bindings::Windows::Win32::{
	CancelIoEx, CreateEventW, ERROR_IO_PENDING, GetOverlappedResult, HANDLE, INFINITE, OVERLAPPED,
	SetEvent, WAIT_OBJECT_0, WAIT_TIMEOUT, WaitForMultipleObjects, WaitForSingleObject,
};

/// Creates an unnamed manual-reset event; it stays signalled until reset.
pub fn create_event() -> Result<OwnedHandle> {
	let handle = unsafe { CreateEventW(None, true, false, None) };
	OwnedHandle::try_from(unsafe { HandleOrNull::from_raw_handle(handle.0) })
		.map_err(|_| Error::from_thread())
}

/// Shutdown signal shared across threads: a manual-reset event that stays
/// signalled once set, waking every pending overlapped wait.
#[derive(Debug)]
pub struct Shutdown {
	event: OwnedHandle,
}

impl Shutdown {
	pub fn new() -> Result<Self> {
		Ok(Self { event: create_event()? })
	}

	pub fn signal(&self) {
		if let Err(e) = unsafe { SetEvent(HANDLE(self.event.as_raw_handle())) }.ok() {
			error!("Failed to signal shutdown event: {}", e);
		}
	}

	pub fn is_signalled(&self) -> bool {
		self.wait(0)
	}

	/// Waits up to `ms` for the shutdown event; true if it fired.
	pub fn wait(&self, ms: u32) -> bool {
		match unsafe { WaitForSingleObject(HANDLE(self.event.as_raw_handle()), ms) } as i32 {
			WAIT_OBJECT_0 => true,
			WAIT_TIMEOUT => false,
			_ => {
				error!("Wait on shutdown event failed: {}", Error::from_thread());
				true
			}
		}
	}
}

/// Outcome of waiting for a pending overlapped operation.
#[derive(Debug)]
pub enum OverlappedWait {
	/// Operation finished; carries the transferred byte count.
	Completed(u32),
	/// The shutdown event fired first; the operation has been cancelled and
	/// reaped, the buffer is safe to release.
	Shutdown,
}

/// Waits for a pending overlapped operation (its event is
/// `overlapped.hEvent`) or the shutdown signal, whichever fires first. On
/// shutdown — and on a failed wait — the operation is cancelled via
/// `CancelIoEx` and reaped with a blocking `GetOverlappedResult` so the
/// kernel no longer references `overlapped` or the IO buffer when this
/// returns.
///
/// # Safety
/// `overlapped` must reference a pending operation on `handle` whose event
/// member is a valid manual-reset event, and the IO buffer must stay alive
/// until this function returns.
pub unsafe fn wait_overlapped(
	handle: HANDLE,
	overlapped: &mut OVERLAPPED,
	shutdown: &Shutdown,
) -> Result<OverlappedWait> {
	let handles = [overlapped.hEvent, HANDLE(shutdown.event.as_raw_handle())];
	let status = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
	match status as i32 {
		WAIT_OBJECT_0 => {
			let mut bytes = 0u32;
			unsafe { GetOverlappedResult(handle, overlapped, &mut bytes, false) }.ok()?;
			Ok(OverlappedWait::Completed(bytes))
		}
		n if n == WAIT_OBJECT_0 + 1 => {
			unsafe {
				let _ = CancelIoEx(handle, Some(overlapped));
				let mut bytes = 0u32;
				let _ = GetOverlappedResult(handle, overlapped, &mut bytes, true);
			}
			Ok(OverlappedWait::Shutdown)
		}
		_ => {
			// Must run before CancelIoEx/GetOverlappedResult clobber the thread error.
			let e = Error::from_thread();
			unsafe {
				let _ = CancelIoEx(handle, Some(overlapped));
				let mut bytes = 0u32;
				let _ = GetOverlappedResult(handle, overlapped, &mut bytes, true);
			}
			Err(e)
		}
	}
}

/// Runs one overlapped op to completion: `start` issues the op against the
/// handle with an `OVERLAPPED` carrying `op_event` and returns the byte
/// count of a synchronous completion, which is consumed directly. On
/// `ERROR_IO_PENDING` the op is awaited via [`wait_overlapped`]; any other
/// start error is returned as-is. The IO buffer `start` hands to the kernel
/// must be borrowed from outside the closure so it stays alive until this
/// returns.
pub(crate) fn run_overlapped<F>(
	handle: &OwnedHandle,
	shutdown: &Shutdown,
	op_event: &OwnedHandle,
	start: F,
) -> Result<OverlappedWait>
where
	F: FnOnce(HANDLE, &mut OVERLAPPED) -> Result<u32>,
{
	let raw = HANDLE(handle.as_raw_handle());
	let mut ov = OVERLAPPED { hEvent: HANDLE(op_event.as_raw_handle()), ..Default::default() };
	match start(raw, &mut ov) {
		Ok(n) => return Ok(OverlappedWait::Completed(n)),
		Err(e) if e.code() == WIN32_ERROR(ERROR_IO_PENDING as u32).into() => {}
		Err(e) => return Err(e),
	}
	unsafe { wait_overlapped(raw, &mut ov, shutdown) }
}

#[cfg(test)]
pub(crate) mod test_util {
	use crate::bindings::Windows::Win32::{
		CreateNamedPipeW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
		PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
	};
	use std::os::windows::io::{HandleOrInvalid, OwnedHandle};
	use windows_core::{Error, HSTRING};

	pub(crate) fn make_server(addr: &str) -> OwnedHandle {
		let handle = unsafe {
			CreateNamedPipeW(
				&HSTRING::from(addr),
				(FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE | PIPE_ACCESS_DUPLEX) as u32,
				(PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT) as u32,
				1,
				65536,
				65536,
				0,
				None,
			)
		};
		OwnedHandle::try_from(unsafe { HandleOrInvalid::from_raw_handle(handle.0) })
			.unwrap_or_else(|_| panic!("CreateNamedPipeW failed: {:?}", Error::from_thread()))
	}

	/// The instance exists before this is called, so the open never races
	/// the server; the subsequent connect handles the resulting
	/// `ERROR_PIPE_CONNECTED`.
	pub(crate) fn open_client(addr: &str) -> std::fs::File {
		std::fs::OpenOptions::new().read(true).write(true).open(addr).expect("client open")
	}
}

#[cfg(test)]
mod tests {
	use super::test_util::{make_server, open_client};
	use super::*;
	use crate::bindings::Windows::Win32::{ConnectNamedPipe, ERROR_PIPE_CONNECTED, ReadFile};
	use std::io::Write;

	fn connect_server(server: &OwnedHandle, shutdown: &Shutdown) {
		let event = create_event().expect("event");
		let res = run_overlapped(server, shutdown, &event, |h, ov| {
			match unsafe { ConnectNamedPipe(h, Some(ov)) }.ok() {
				Ok(()) => Ok(0),
				Err(e) if e.code() == WIN32_ERROR(ERROR_PIPE_CONNECTED as u32).into() => Ok(0),
				Err(e) => Err(e),
			}
		});
		match res {
			Ok(OverlappedWait::Completed(_)) => {}
			other => panic!("connect wait: {other:?}"),
		}
	}

	/// Issues an overlapped read; a synchronous completion still signals the
	/// event, so the caller awaits both non-error outcomes identically.
	fn start_read(server: &OwnedHandle, ov: &mut OVERLAPPED, buf: &mut [u8]) {
		let started = unsafe {
			ReadFile(
				HANDLE(server.as_raw_handle()),
				Some(buf.as_mut_ptr().cast()),
				buf.len() as u32,
				None,
				Some(ov as *mut _),
			)
		};
		match started.ok() {
			Ok(()) => {}
			Err(e) if e.code() == WIN32_ERROR(ERROR_IO_PENDING as u32).into() => {}
			Err(e) => panic!("ReadFile: {e}"),
		}
	}

	#[test]
	fn wait_overlapped_completes_read() {
		let addr = r"\\.\pipe\rd_pipe_ov_test_read";
		let server = make_server(addr);
		let shutdown = Shutdown::new().expect("shutdown");

		let mut client = open_client(addr);
		connect_server(&server, &shutdown);
		client.write_all(b"ping").expect("client write");

		let event = create_event().expect("event");
		let mut ov = OVERLAPPED { hEvent: HANDLE(event.as_raw_handle()), ..Default::default() };
		let mut buf = [0u8; 16];
		start_read(&server, &mut ov, &mut buf);
		match unsafe { wait_overlapped(HANDLE(server.as_raw_handle()), &mut ov, &shutdown) } {
			Ok(OverlappedWait::Completed(n)) => {
				assert_eq!(&buf[..n as usize], b"ping");
			}
			other => panic!("read wait: {other:?}"),
		}
	}

	#[test]
	fn wait_overlapped_returns_shutdown_on_signal() {
		let addr = r"\\.\pipe\rd_pipe_ov_test_shutdown";
		let server = make_server(addr);
		let shutdown = Shutdown::new().expect("shutdown");

		// Client stays open without writing so the read below stays pending.
		let _client = open_client(addr);
		connect_server(&server, &shutdown);

		let event = create_event().expect("event");
		let mut ov = OVERLAPPED { hEvent: HANDLE(event.as_raw_handle()), ..Default::default() };
		let mut buf = [0u8; 16];
		start_read(&server, &mut ov, &mut buf);
		std::thread::scope(|s| {
			s.spawn(|| {
				std::thread::sleep(std::time::Duration::from_millis(50));
				shutdown.signal();
			});
			match unsafe { wait_overlapped(HANDLE(server.as_raw_handle()), &mut ov, &shutdown) } {
				Ok(OverlappedWait::Shutdown) => {}
				other => panic!("expected Shutdown, got {other:?}"),
			}
		});
	}

	#[test]
	fn shutdown_signal_sets_event() {
		let shutdown = Shutdown::new().expect("shutdown");
		assert!(!shutdown.is_signalled());
		assert!(!shutdown.wait(0));
		shutdown.signal();
		assert!(shutdown.is_signalled());
		assert!(shutdown.wait(0));
	}
}
