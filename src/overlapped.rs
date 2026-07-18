// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Overlapped IO primitives: owned handle/event wrappers and the event-pair wait helper
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

use windows::Win32::{
	Foundation::{CloseHandle, HANDLE, WAIT_EVENT, WAIT_OBJECT_0},
	System::{
		IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
		Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects},
	},
};
use windows::core::Result;

/// Owned Win32 handle, closed on drop. A `HANDLE` is a plain kernel object
/// reference; using it from multiple threads is part of the Win32 contract,
/// hence the `Send`/`Sync` impls.
#[derive(Debug)]
pub struct OwnedHandle(HANDLE);

unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl OwnedHandle {
	/// Takes ownership of `handle`; it is closed when the wrapper drops.
	///
	/// # Safety
	/// `handle` must be a valid handle owned by the caller; nothing else may
	/// close it after this call.
	pub unsafe fn new(handle: HANDLE) -> Self {
		Self(handle)
	}

	pub fn raw(&self) -> HANDLE {
		self.0
	}
}

impl Drop for OwnedHandle {
	fn drop(&mut self) {
		if !self.0.is_invalid() {
			unsafe {
				let _ = CloseHandle(self.0);
			}
		}
	}
}

/// Creates an unnamed event. Manual-reset events stay signalled until reset,
/// auto-reset events release exactly one waiter per signal.
pub fn create_event(manual_reset: bool) -> Result<OwnedHandle> {
	let handle = unsafe { CreateEventW(None, manual_reset, false, None) }?;
	Ok(unsafe { OwnedHandle::new(handle) })
}

pub fn signal_event(event: &OwnedHandle) -> Result<()> {
	unsafe { SetEvent(event.raw()) }
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
/// `overlapped.hEvent`) or the shutdown event, whichever fires first. On
/// shutdown the operation is cancelled via `CancelIoEx` and reaped with a
/// blocking `GetOverlappedResult` so the kernel no longer references
/// `overlapped` or the IO buffer when this returns.
///
/// # Safety
/// `overlapped` must reference a pending operation on `handle` whose event
/// member is a valid auto-reset or manual-reset event, and the IO buffer must
/// stay alive until this function returns.
pub unsafe fn wait_overlapped(
	handle: HANDLE,
	overlapped: &mut OVERLAPPED,
	shutdown_event: &OwnedHandle,
) -> Result<OverlappedWait> {
	let handles = [overlapped.hEvent, shutdown_event.raw()];
	let status = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
	const WAIT_SHUTDOWN: WAIT_EVENT = WAIT_EVENT(WAIT_OBJECT_0.0 + 1);
	match status {
		WAIT_OBJECT_0 => {
			let mut bytes = 0u32;
			unsafe { GetOverlappedResult(handle, overlapped, &mut bytes, false) }?;
			Ok(OverlappedWait::Completed(bytes))
		}
		WAIT_SHUTDOWN => {
			unsafe {
				let _ = CancelIoEx(handle, Some(overlapped));
				let mut bytes = 0u32;
				let _ = GetOverlappedResult(handle, overlapped, &mut bytes, true);
			}
			Ok(OverlappedWait::Shutdown)
		}
		_ => Err(windows::core::Error::from_thread()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use windows::Win32::{
		Foundation::{ERROR_IO_PENDING, ERROR_PIPE_CONNECTED},
		Storage::FileSystem::{
			FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX, ReadFile,
		},
		System::Pipes::{
			ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
		},
	};
	use windows::core::HSTRING;

	fn make_server(addr: &str) -> OwnedHandle {
		let handle = unsafe {
			CreateNamedPipeW(
				&HSTRING::from(addr),
				FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE | PIPE_ACCESS_DUPLEX,
				PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
				1,
				65536,
				65536,
				0,
				None,
			)
		};
		assert!(
			!handle.is_invalid(),
			"CreateNamedPipeW failed: {:?}",
			windows::core::Error::from_thread()
		);
		unsafe { OwnedHandle::new(handle) }
	}

	fn connect_server(server: &OwnedHandle, shutdown: &OwnedHandle) {
		let event = create_event(false).expect("event");
		let mut ov = OVERLAPPED { hEvent: event.raw(), ..Default::default() };
		let pending = match unsafe { ConnectNamedPipe(server.raw(), Some(&mut ov)) } {
			Ok(()) => false,
			Err(e) if e.code() == ERROR_PIPE_CONNECTED.into() => false,
			Err(e) if e.code() == ERROR_IO_PENDING.into() => true,
			Err(e) => panic!("ConnectNamedPipe: {e}"),
		};
		if pending {
			match unsafe { wait_overlapped(server.raw(), &mut ov, shutdown) } {
				Ok(OverlappedWait::Completed(_)) => {}
				other => panic!("connect wait: {other:?}"),
			}
		}
	}

	#[test]
	fn wait_overlapped_completes_read() {
		let addr = r"\\.\pipe\rd_pipe_ov_test_read";
		let server = make_server(addr);
		let shutdown = create_event(true).expect("shutdown event");

		let client = std::thread::spawn(move || {
			let mut f = loop {
				match std::fs::OpenOptions::new().read(true).write(true).open(addr) {
					Ok(f) => break f,
					Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
				}
			};
			f.write_all(b"ping").expect("client write");
			f
		});

		connect_server(&server, &shutdown);

		let event = create_event(false).expect("event");
		let mut ov = OVERLAPPED { hEvent: event.raw(), ..Default::default() };
		let mut buf = [0u8; 16];
		// Synchronous completion still signals the event, so both non-error
		// outcomes are awaited identically.
		match unsafe { ReadFile(server.raw(), Some(&mut buf), None, Some(&mut ov)) } {
			Ok(()) => {}
			Err(e) if e.code() == ERROR_IO_PENDING.into() => {}
			Err(e) => panic!("ReadFile: {e}"),
		}
		match unsafe { wait_overlapped(server.raw(), &mut ov, &shutdown) } {
			Ok(OverlappedWait::Completed(n)) => {
				assert_eq!(&buf[..n as usize], b"ping");
			}
			other => panic!("read wait: {other:?}"),
		}
		drop(client.join().expect("client thread"));
	}

	#[test]
	fn wait_overlapped_returns_shutdown_on_signal() {
		let addr = r"\\.\pipe\rd_pipe_ov_test_shutdown";
		let server = make_server(addr);
		let shutdown = create_event(true).expect("shutdown event");

		let client = std::thread::spawn(move || {
			loop {
				match std::fs::OpenOptions::new().read(true).write(true).open(addr) {
					Ok(f) => break f,
					Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
				}
			}
		});

		connect_server(&server, &shutdown);
		let _client = client.join().expect("client thread");

		// Pending read with no data; signal shutdown from another thread.
		let event = create_event(false).expect("event");
		let mut ov = OVERLAPPED { hEvent: event.raw(), ..Default::default() };
		let mut buf = [0u8; 16];
		match unsafe { ReadFile(server.raw(), Some(&mut buf), None, Some(&mut ov)) } {
			Ok(()) => {}
			Err(e) if e.code() == ERROR_IO_PENDING.into() => {}
			Err(e) => panic!("ReadFile: {e}"),
		}
		let signaller = {
			let shutdown_raw = shutdown.raw().0 as usize;
			std::thread::spawn(move || {
				std::thread::sleep(std::time::Duration::from_millis(50));
				let shutdown = HANDLE(shutdown_raw as _);
				unsafe { SetEvent(shutdown) }.expect("SetEvent");
			})
		};
		match unsafe { wait_overlapped(server.raw(), &mut ov, &shutdown) } {
			Ok(OverlappedWait::Shutdown) => {}
			other => panic!("expected Shutdown, got {other:?}"),
		}
		signaller.join().expect("signaller thread");
	}
}
