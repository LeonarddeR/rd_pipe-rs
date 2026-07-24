// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Dynamic Virtual Channel Plugin structs
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

use core::slice;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Weak};
use std::thread;
use tracing::{debug, error, info, instrument, trace, warn};
use windows::Win32::Foundation::{
	E_POINTER, ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_OPERATION_ABORTED, ERROR_PIPE_CONNECTED,
	ERROR_PIPE_NOT_CONNECTED, HANDLE, HLOCAL,
};
use windows::Win32::Storage::FileSystem::{
	FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows::Win32::System::Com::{CO_MTA_USAGE_COOKIE, CoDecrementMTAUsage, CoIncrementMTAUsage};
use windows::Win32::System::IO::OVERLAPPED;
use windows::Win32::System::Pipes::{
	ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
	PIPE_WAIT,
};
use windows::{
	Win32::{
		Foundation::E_UNEXPECTED,
		System::RemoteDesktop::{
			IWTSListener, IWTSListenerCallback, IWTSListenerCallback_Impl, IWTSPlugin,
			IWTSPlugin_Impl, IWTSVirtualChannel, IWTSVirtualChannelCallback,
			IWTSVirtualChannelCallback_Impl, IWTSVirtualChannelManager,
		},
	},
	core::{AgileReference, BSTR, Error, HSTRING, Interface, PCSTR, Result, implement},
};
use windows_core::{BOOL, OutRef, Owned};
use windows_registry::{CURRENT_USER, Key, LOCAL_MACHINE};

use crate::overlapped::{OverlappedWait, OwnedHandle, Shutdown, create_event, run_overlapped};
use crate::security_descriptor::{get_logon_sid, security_attributes_from_sddl};

pub const REG_PATH: &str = r#"Software\Classes\CLSID\{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}"#;
const REG_VALUE_CHANNEL_NAMES: &str = "ChannelNames";

#[derive(Debug, Default)]
#[implement(IWTSPlugin)]
pub struct RdPipePlugin;

impl RdPipePlugin {
	pub fn new() -> Self {
		Self
	}

	#[instrument]
	fn create_listener(
		&self,
		channel_mgr: &IWTSVirtualChannelManager,
		channel_name: String,
	) -> Result<IWTSListener> {
		debug!("Creating listener with name {}", channel_name);
		let callback: IWTSListenerCallback =
			RdPipeListenerCallback::new(channel_name.clone()).into();
		unsafe {
			channel_mgr.CreateListener(
				PCSTR::from_raw(format!("{}\0", channel_name).as_ptr()),
				0,
				&callback,
			)
		}
	}

	#[instrument]
	fn get_channel_names_from_registry(parent_key: &Key) -> windows_core::Result<Vec<String>> {
		let sub_key = parent_key.open(REG_PATH)?;
		sub_key.get_multi_string(REG_VALUE_CHANNEL_NAMES)
	}
}

impl fmt::Debug for RdPipePlugin_Impl {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("RdPipePlugin_Impl").finish()
	}
}

impl IWTSPlugin_Impl for RdPipePlugin_Impl {
	#[instrument(skip(pchannelmgr))]
	fn Initialize(
		&self,
		pchannelmgr: windows_core::Ref<'_, IWTSVirtualChannelManager>,
	) -> Result<()> {
		let channel_mgr = match pchannelmgr.as_ref() {
			Some(m) => m,
			None => {
				error!("No pchannelmgr given when initializing");
				return Err(Error::from(E_UNEXPECTED));
			}
		};
		let mut channels: Vec<String> = Vec::new();
		channels.extend(
			RdPipePlugin::get_channel_names_from_registry(CURRENT_USER).unwrap_or_default(),
		);
		channels.extend(
			RdPipePlugin::get_channel_names_from_registry(LOCAL_MACHINE).unwrap_or_default(),
		);
		let channels: HashSet<String> = channels.into_iter().filter(|s| !s.is_empty()).collect();
		if channels.is_empty() {
			error!("No channels in registry");
			return Err(Error::from(E_UNEXPECTED));
		}
		for channel_name in channels {
			self.create_listener(channel_mgr, channel_name)?;
		}
		Ok(())
	}

	fn Connected(&self) -> Result<()> {
		info!("Client connected");
		Ok(())
	}

	fn Disconnected(&self, dwdisconnectcode: u32) -> Result<()> {
		info!("Client disconnected with {}", dwdisconnectcode);
		Ok(())
	}

	fn Terminated(&self) -> Result<()> {
		info!("Client terminated");
		Ok(())
	}
}

#[derive(Debug)]
#[implement(IWTSListenerCallback)]
pub struct RdPipeListenerCallback {
	name: String,
}

impl RdPipeListenerCallback {
	pub fn new(name: String) -> Self {
		Self { name }
	}
}

impl fmt::Debug for RdPipeListenerCallback_Impl {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("RdPipeListenerCallback_Impl").field("name", &self.name).finish()
	}
}

impl IWTSListenerCallback_Impl for RdPipeListenerCallback_Impl {
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	#[instrument(skip(pchannel, ppcallback))]
	fn OnNewChannelConnection(
		&self,
		pchannel: windows_core::Ref<'_, IWTSVirtualChannel>,
		data: &BSTR,
		pbaccept: *mut BOOL,
		ppcallback: OutRef<'_, IWTSVirtualChannelCallback>,
	) -> Result<()> {
		debug!("Creating new callback for channel with name {}", &self.name);
		let channel = match pchannel.as_ref() {
			Some(c) => c,
			None => return Err(Error::from(E_UNEXPECTED)),
		};
		if pbaccept.is_null() {
			return Err(Error::from(E_POINTER));
		}
		let pbaccept = unsafe { &mut *pbaccept };
		*pbaccept = BOOL::from(true);
		debug!("Creating callback");
		let callback: IWTSVirtualChannelCallback =
			RdPipeChannelCallback::new(channel, &self.name)?.into();
		trace!("Callback {:?} created", callback);
		ppcallback.write(callback.into())?;
		Ok(())
	}
}

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\RDPipe";
const PIPE_BUFFER_SIZE: u32 = 64 * 1024;
/// Queued channel→pipe backlog (bytes) at which the client is considered
/// stalled. Checked before admission, so a single chunk of any size still
/// passes while the backlog is under budget; the in-flight chunk is not
/// counted.
const WRITE_QUEUE_MAX_BYTES: usize = 4 * 1024 * 1024;
/// Backstop on queued chunk count: bounds the per-chunk allocation overhead
/// the byte counter can't see.
const WRITE_QUEUE_MAX_CHUNKS: usize = 65536;
/// How long a graceful close waits for the writer to drain to a connected
/// client before the in-flight write is cancelled.
const DRAIN_TIMEOUT_MS: u32 = 2000;

/// Per-connection writer queue handle held in the callback's slot. The
/// counters track queued (not in-flight) payload; they die with the
/// connection. `sender` must remain the only sender: dropping the slot is
/// what ends the writer's `recv` loop.
#[derive(Debug)]
struct WriterQueue {
	sender: Sender<Vec<u8>>,
	queued_bytes: Arc<AtomicUsize>,
	queued_chunks: Arc<AtomicUsize>,
}

fn channel_write(channel: &IWTSVirtualChannel, data: &[u8]) -> Result<()> {
	unsafe { channel.Write(data, None) }
}

const MSG_XON: u8 = 0x11;
const MSG_XOFF: u8 = 0x13;

fn write_flow_control(channel: &IWTSVirtualChannel, byte: u8) -> Result<()> {
	match channel_write(channel, &[byte]) {
		Ok(()) => {
			trace!("Wrote flow control byte {:#04x} to channel", byte);
			Ok(())
		}
		Err(e) => {
			error!("Error writing flow control byte {:#04x} to channel: {}", byte, e);
			Err(e)
		}
	}
}

fn build_pipe_sddl(logon_sid: &str) -> String {
	format!("D:(A;;GRGW;;;{logon_sid})S:(ML;;NRNW;;;ME)")
}

/// Result of a single overlapped pipe operation.
#[derive(Debug)]
enum PipeIo {
	Done(u32),
	Shutdown,
	Failed(Error),
}

/// Whether a failed pipe op means the client side is gone rather than a
/// genuine IO error.
fn is_disconnect(e: &Error) -> bool {
	e.code() == ERROR_BROKEN_PIPE.into()
		|| e.code() == ERROR_PIPE_NOT_CONNECTED.into()
		|| e.code() == ERROR_NO_DATA.into()
		|| e.code() == ERROR_OPERATION_ABORTED.into()
}

/// Runs one overlapped pipe op via [`run_overlapped`], folding errors into
/// the pipe disconnect taxonomy.
fn run_pipe_op<F>(
	handle: &OwnedHandle,
	shutdown: &Shutdown,
	op_event: &OwnedHandle,
	start: F,
) -> PipeIo
where
	F: FnOnce(HANDLE, &mut OVERLAPPED) -> Result<u32>,
{
	match run_overlapped(handle, shutdown, op_event, start) {
		Ok(OverlappedWait::Completed(n)) => PipeIo::Done(n),
		Ok(OverlappedWait::Shutdown) => PipeIo::Shutdown,
		Err(e) => PipeIo::Failed(e),
	}
}

/// Waits for a pipe client on the instance. `ERROR_PIPE_CONNECTED` means the
/// client beat us to it and never pends the OVERLAPPED, so it maps to a
/// synchronous completion.
fn connect_pipe(handle: &OwnedHandle, shutdown: &Shutdown, op_event: &OwnedHandle) -> PipeIo {
	run_pipe_op(handle, shutdown, op_event, |h, ov| {
		match unsafe { ConnectNamedPipe(h, Some(ov)) } {
			Ok(()) => Ok(0),
			Err(e) if e.code() == ERROR_PIPE_CONNECTED.into() => {
				trace!("Pipe client connected before ConnectNamedPipe");
				Ok(0)
			}
			Err(e) => Err(e),
		}
	})
}

fn read_pipe(
	handle: &OwnedHandle,
	shutdown: &Shutdown,
	op_event: &OwnedHandle,
	buf: &mut [u8],
) -> PipeIo {
	run_pipe_op(handle, shutdown, op_event, |h, ov| unsafe {
		let mut n = 0u32;
		ReadFile(h, Some(buf), Some(&mut n), Some(ov as *mut _))?;
		Ok(n)
	})
}

fn write_pipe(
	handle: &OwnedHandle,
	shutdown: &Shutdown,
	op_event: &OwnedHandle,
	data: &[u8],
) -> PipeIo {
	run_pipe_op(handle, shutdown, op_event, |h, ov| unsafe {
		let mut n = 0u32;
		WriteFile(h, Some(data), Some(&mut n), Some(ov as *mut _))?;
		Ok(n)
	})
}

/// Best-effort disconnect of the pipe instance; also the way a pending
/// overlapped op on the same handle is kicked awake from another thread.
fn disconnect_pipe(handle: &OwnedHandle, context: &str) {
	if let Err(e) = unsafe { DisconnectNamedPipe(handle.raw()) } {
		trace!("Error disconnecting pipe instance ({}): {}", context, e);
	}
}

/// COM channel callback. The pipe instance is owned by the pump and writer
/// threads (via `Arc<OwnedHandle>`), not held here, so the instance closes as
/// soon as those threads exit — giving a connected client a graceful
/// end-of-pipe. The callback keeps only the writer queue slot (for
/// `OnDataReceived`), a `Weak` handle (to disconnect a stalled client without
/// extending the instance's lifetime), and the shutdown signal (`OnClose`).
#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
	writer_slot: Arc<Mutex<Option<WriterQueue>>>,
	pipe_handle: Weak<OwnedHandle>,
	shutdown: Arc<Shutdown>,
}

impl RdPipeChannelCallback {
	#[instrument]
	pub fn new(channel: &IWTSVirtualChannel, channel_name: &str) -> Result<Self> {
		let addr = format!("{}_{}_{}", PIPE_NAME_PREFIX, channel_name, channel.as_raw() as usize);
		let channel_agile = AgileReference::new(channel)?;
		debug!("Constructing the callback");

		let login_sid = get_logon_sid().map_err(|e| {
			error!("Can't get login sid, {}", e);
			e
		})?;
		let sddl = build_pipe_sddl(&login_sid);
		let handle = Arc::new(create_pipe_instance(&addr, &sddl)?);
		let pipe_handle = Arc::downgrade(&handle);
		let writer_slot = Arc::new(Mutex::new(None));
		let shutdown = Arc::new(Shutdown::new()?);

		{
			let writer_slot = writer_slot.clone();
			let shutdown = shutdown.clone();
			thread::Builder::new()
				.name(format!("rd_pipe pump {addr}"))
				.spawn(move || run_pipe_pump(handle, writer_slot, channel_agile, shutdown, addr))
				.map_err(|e| {
					error!("Failed to spawn pipe pump thread: {}", e);
					Error::from(E_UNEXPECTED)
				})?;
		}

		Ok(Self { writer_slot, pipe_handle, shutdown })
	}
}

impl Drop for RdPipeChannelCallback {
	fn drop(&mut self) {
		self.shutdown.signal();
	}
}

/// Creates the single named-pipe instance for a channel; the handle stays
/// open for the channel's lifetime.
fn create_pipe_instance(addr: &str, sddl: &str) -> Result<OwnedHandle> {
	trace!("Creating pipe server with address {}", addr);
	let handle = unsafe {
		let attributes = security_attributes_from_sddl(sddl).map_err(|e| {
			error!("Can't create security attributes, {}", e);
			e
		})?;
		let _sd = Owned::new(HLOCAL(attributes.lpSecurityDescriptor));
		CreateNamedPipeW(
			&HSTRING::from(addr),
			FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE | PIPE_ACCESS_DUPLEX,
			PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
			1,
			PIPE_BUFFER_SIZE,
			PIPE_BUFFER_SIZE,
			0,
			Some(&attributes),
		)
	};
	if handle.is_invalid() {
		let e = Error::from_thread();
		error!("Error while creating named pipe server: {}", e);
		return Err(e);
	}
	Ok(unsafe { OwnedHandle::new(handle) })
}

/// Keeps the process-wide implicit MTA alive while held, so the pump's
/// threads stay valid COM callers for their whole lifetime.
struct MtaGuard(CO_MTA_USAGE_COOKIE);

impl MtaGuard {
	fn new() -> Option<Self> {
		match unsafe { CoIncrementMTAUsage() } {
			Ok(cookie) => Some(Self(cookie)),
			Err(e) => {
				error!("CoIncrementMTAUsage failed: {}", e);
				None
			}
		}
	}
}

impl Drop for MtaGuard {
	fn drop(&mut self) {
		unsafe {
			let _ = CoDecrementMTAUsage(self.0);
		}
	}
}

/// Accept/read loop for one channel. Runs on a dedicated thread; owns the
/// pipe instance (shared with the per-connection writer thread) and keeps it
/// claimed across client reconnects. XOFF is written to the channel at each
/// disconnect while the channel is still live. On channel close (shutdown)
/// the pump returns without disconnecting: the last handle references drop,
/// the instance closes, and a still-connected client observes a graceful
/// end-of-pipe rather than a forced disconnect.
#[instrument(skip(handle, writer_slot, channel_agile, shutdown))]
fn run_pipe_pump(
	handle: Arc<OwnedHandle>,
	writer_slot: Arc<Mutex<Option<WriterQueue>>>,
	channel_agile: AgileReference<IWTSVirtualChannel>,
	shutdown: Arc<Shutdown>,
	addr: String,
) {
	let _mta = MtaGuard::new();
	let op_event = match create_event() {
		Ok(e) => e,
		Err(e) => {
			error!("Can't create pump op event: {}", e);
			return;
		}
	};
	let channel = match channel_agile.resolve() {
		Ok(c) => c,
		Err(e) => {
			error!("Can't resolve channel reference: {}", e);
			return;
		}
	};
	let mut buf = vec![0u8; PIPE_BUFFER_SIZE as usize];
	while !shutdown.is_signalled() {
		trace!("Initiate connection to pipe client");
		match connect_pipe(&handle, &shutdown, &op_event) {
			PipeIo::Done(_) => {}
			PipeIo::Shutdown => {
				trace!("Shutdown signalled while awaiting pipe client connection");
				break;
			}
			PipeIo::Failed(e) => {
				error!("Error connecting to pipe client: {}", e);
				disconnect_pipe(&handle, "connect retry");
				if shutdown.wait(100) {
					trace!("Shutdown signalled during pipe reconnect retry");
					break;
				}
				continue;
			}
		}
		trace!("Pipe client connected");

		let (writer_kill, writer_done) = match (Shutdown::new(), Shutdown::new()) {
			(Ok(k), Ok(d)) => (Arc::new(k), Arc::new(d)),
			(Err(e), _) | (_, Err(e)) => {
				error!("Can't create writer control events: {}", e);
				shutdown.signal();
				let _ = write_flow_control(&channel, MSG_XOFF);
				break;
			}
		};
		let (sender, receiver) = mpsc::channel();
		let queued_bytes = Arc::new(AtomicUsize::new(0));
		let queued_chunks = Arc::new(AtomicUsize::new(0));
		{
			let mut slot = writer_slot.lock();
			if shutdown.is_signalled() {
				trace!("Shutdown signalled before writer registration");
				break;
			}
			*slot = Some(WriterQueue {
				sender,
				queued_bytes: queued_bytes.clone(),
				queued_chunks: queued_chunks.clone(),
			});
		}
		let writer_thread = {
			let writer_handle = handle.clone();
			let kill = writer_kill.clone();
			let done = writer_done.clone();
			match thread::Builder::new().name(format!("rd_pipe writer {addr}")).spawn(move || {
				run_pipe_writer(writer_handle, receiver, queued_bytes, queued_chunks, kill, done)
			}) {
				Ok(t) => t,
				Err(e) => {
					error!("Failed to spawn pipe writer thread: {}", e);
					writer_slot.lock().take();
					shutdown.signal();
					let _ = write_flow_control(&channel, MSG_XOFF);
					break;
				}
			}
		};

		if write_flow_control(&channel, MSG_XON).is_err() {
			shutdown.signal();
		}

		trace!("Initiating pipe_reader loop");
		while !shutdown.is_signalled() {
			match read_pipe(&handle, &shutdown, &op_event, &mut buf) {
				PipeIo::Done(0) => {
					trace!("Zero-byte read from pipe");
				}
				PipeIo::Done(n) => {
					trace!("read {} bytes", n);
					match channel_write(&channel, &buf[..n as usize]) {
						Ok(()) => trace!("Wrote {} bytes to channel", n),
						Err(e) => {
							error!("Error during write to channel: {}", e);
							shutdown.signal();
							break;
						}
					}
				}
				PipeIo::Shutdown => {
					trace!("Shutdown signalled inside reader loop");
					let _ = write_flow_control(&channel, MSG_XOFF);
					break;
				}
				PipeIo::Failed(e) => {
					if is_disconnect(&e) {
						info!("Pipe closed by client: {}", e);
					} else {
						error!("Error reading from pipe client: {}", e);
					}
					let _ = write_flow_control(&channel, MSG_XOFF);
					break;
				}
			}
		}

		let graceful = shutdown.is_signalled();
		// The slot holds the only sender; taking it here, before any wait on
		// the writer, is what lets the writer's recv loop end.
		writer_slot.lock().take();
		if graceful {
			trace!("End of pipe_reader loop, draining writer");
			if !writer_done.wait(DRAIN_TIMEOUT_MS) {
				trace!("Drain timeout; cancelling in-flight write");
				writer_kill.signal();
			}
			if let Err(e) = writer_thread.join() {
				error!("Pipe writer thread panicked: {:?}", e);
			}
			break;
		}
		// Client disconnected: reset the instance and accept the next client.
		trace!("End of pipe_reader loop, reclaiming pipe instance");
		disconnect_pipe(&handle, "teardown");
		if let Err(e) = writer_thread.join() {
			error!("Pipe writer thread panicked: {:?}", e);
			break;
		}
	}
	trace!("Pipe pump for {} exiting", addr);
}

/// Signals a `Shutdown` when dropped; covers panic and early-return exits.
struct SignalOnDrop(Arc<Shutdown>);

impl Drop for SignalOnDrop {
	fn drop(&mut self) {
		self.0.signal();
	}
}

/// Drains queued channel data into the pipe until the sender is dropped or
/// a write fails; `kill` cancels an in-flight write, `done` is signalled on
/// every exit.
fn run_pipe_writer(
	handle: Arc<OwnedHandle>,
	receiver: Receiver<Vec<u8>>,
	queued_bytes: Arc<AtomicUsize>,
	queued_chunks: Arc<AtomicUsize>,
	kill: Arc<Shutdown>,
	done: Arc<Shutdown>,
) {
	let _done = SignalOnDrop(done);
	let op_event = match create_event() {
		Ok(e) => e,
		Err(e) => {
			error!("Can't create writer op event: {}", e);
			disconnect_pipe(&handle, "writer event failure");
			return;
		}
	};
	while let Ok(chunk) = receiver.recv() {
		queued_bytes.fetch_sub(chunk.len(), Ordering::Relaxed);
		queued_chunks.fetch_sub(1, Ordering::Relaxed);
		match write_pipe(&handle, &kill, &op_event, &chunk) {
			PipeIo::Done(n) => {
				trace!("Wrote {} bytes to pipe", n);
				if (n as usize) != chunk.len() {
					error!("Partial pipe write: {} of {} bytes", n, chunk.len());
					disconnect_pipe(&handle, "partial write");
					break;
				}
			}
			PipeIo::Shutdown => {
				trace!("Writer kill signalled inside writer loop");
				break;
			}
			PipeIo::Failed(e) => {
				if is_disconnect(&e) {
					info!("Pipe write target gone: {}", e);
				} else {
					error!("Error writing to pipe: {}", e);
					disconnect_pipe(&handle, "write failure");
				}
				break;
			}
		}
	}
	trace!("Pipe writer exiting");
}

impl fmt::Debug for RdPipeChannelCallback_Impl {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("RdPipeChannelCallback_Impl")
			.field("has_pipe_writer", &self.writer_slot.try_lock().map(|g| g.is_some()))
			.field("shutdown_signalled", &self.shutdown.is_signalled())
			.finish()
	}
}

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback_Impl {
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	#[instrument(skip(self))]
	fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
		debug!("Data received, buffer has size {}", cbsize);
		if cbsize == 0 {
			return Ok(());
		}
		if pbuffer.is_null() {
			return Err(Error::from(E_POINTER));
		}
		let chunk = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) }.to_vec();
		trace!("Queueing {} received bytes for pipe", cbsize);
		let mut writer_lock = self.writer_slot.lock();
		let Some(queue) = writer_lock.as_ref() else {
			debug!("Data received without an open named pipe");
			return Err(Error::from(ERROR_PIPE_NOT_CONNECTED));
		};
		let backlog_bytes = queue.queued_bytes.load(Ordering::Relaxed);
		let backlog_chunks = queue.queued_chunks.load(Ordering::Relaxed);
		if backlog_bytes >= WRITE_QUEUE_MAX_BYTES || backlog_chunks >= WRITE_QUEUE_MAX_CHUNKS {
			warn!(
				"Pipe write backlog over budget ({} bytes, {} chunks); disconnecting stalled client",
				backlog_bytes, backlog_chunks
			);
			writer_lock.take();
			// The disconnect must stay under the lock: the pump's teardown
			// take orders the next accept after it.
			if let Some(handle) = self.pipe_handle.upgrade() {
				disconnect_pipe(&handle, "stalled client");
			}
			return Err(Error::from(ERROR_PIPE_NOT_CONNECTED));
		}
		// Increments must precede the send: the writer decrements after recv.
		queue.queued_bytes.fetch_add(chunk.len(), Ordering::Relaxed);
		queue.queued_chunks.fetch_add(1, Ordering::Relaxed);
		match queue.sender.send(chunk) {
			Ok(()) => Ok(()),
			Err(_) => {
				debug!("Pipe writer gone while queueing data");
				writer_lock.take();
				Err(Error::from(ERROR_PIPE_NOT_CONNECTED))
			}
		}
	}

	#[instrument]
	fn OnClose(&self) -> Result<()> {
		// Signal first: the pump's registration recheck observes the signal
		// under the same lock this take contends on.
		self.shutdown.signal();
		self.writer_slot.lock().take();
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pipe_name_prefix() {
		// Verify the pipe name prefix format is correct
		assert!(PIPE_NAME_PREFIX.starts_with(r"\\.\pipe\"));
		assert!(PIPE_NAME_PREFIX.contains("RDPipe"));
	}

	#[test]
	fn test_reg_path_format() {
		// Verify registry path format
		assert!(REG_PATH.contains("Software\\Classes\\CLSID"));
		assert!(REG_PATH.contains(&format!("{:?}", crate::registry::CLSID_RD_PIPE_PLUGIN)));
	}

	#[test]
	fn test_pipe_name_generation() {
		// Test pipe name generation logic
		let channel_name = "testchannel";
		let channel_addr = 12345_usize;
		let addr = format!("{}_{}_{}", PIPE_NAME_PREFIX, channel_name, channel_addr);

		assert!(addr.starts_with(PIPE_NAME_PREFIX));
		assert!(addr.contains(channel_name));
		assert!(addr.contains(&channel_addr.to_string()));
	}

	#[test]
	fn test_build_pipe_sddl_grants_read_write_only() {
		let sddl = build_pipe_sddl("S-1-5-5-0-12345");
		assert_eq!(sddl, "D:(A;;GRGW;;;S-1-5-5-0-12345)S:(ML;;NRNW;;;ME)");
		assert!(!sddl.contains("GA"));
	}

	#[test]
	fn test_build_pipe_sddl_has_medium_integrity_label() {
		let sddl = build_pipe_sddl("S-1-5-5-0-12345");
		assert!(sddl.contains("S:(ML;;NRNW;;;ME)"));
	}

	#[test]
	fn test_listener_callback_new() {
		// Test listener callback construction
		let name = String::from("test_channel");
		let callback = RdPipeListenerCallback::new(name.clone());

		// Verify the name is stored
		assert_eq!(callback.name, name);
	}

	#[test]
	fn run_overlapped_consumes_sync_completion_without_waiting() {
		use crate::overlapped::test_util::{make_server, open_client};
		use std::io::{Read, Write};

		let addr = r"\\.\pipe\rd_pipe_sync_completion_test";
		let server = make_server(addr);
		let mut client = open_client(addr);
		let shutdown = Shutdown::new().expect("shutdown");
		let op_event = create_event().expect("op event");
		assert!(matches!(connect_pipe(&server, &shutdown, &op_event), PipeIo::Done(_)));

		// A signalled shutdown makes the wait path return `Shutdown`, so a
		// `Done` result below proves the synchronous completion was consumed
		// directly from the issuing call.
		shutdown.signal();

		client.write_all(b"ping").expect("client write");
		let mut buf = [0u8; 8];
		match read_pipe(&server, &shutdown, &op_event, &mut buf) {
			PipeIo::Done(n) => assert_eq!(&buf[..n as usize], b"ping"),
			other => panic!("expected sync Done, got {other:?}"),
		}

		let chunk = b"pong";
		match write_pipe(&server, &shutdown, &op_event, chunk) {
			PipeIo::Done(n) => assert_eq!(n as usize, chunk.len()),
			other => panic!("expected sync Done, got {other:?}"),
		}
		let mut got = [0u8; 4];
		client.read_exact(&mut got).expect("client read");
		assert_eq!(&got, b"pong");
	}

	#[test]
	fn operation_aborted_is_a_disconnect() {
		// A pending read/write aborted by a cross-thread `DisconnectNamedPipe`
		// (e.g. the stalled-client path) completes with ERROR_OPERATION_ABORTED.
		assert!(is_disconnect(&Error::from(ERROR_OPERATION_ABORTED)));
	}
}
