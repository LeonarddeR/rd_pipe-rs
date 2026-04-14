// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Dynamic Virtual Channel Plugin structs
// Copyright (C) 2022-2025 Leonard de Ruijter <alderuijter@gmail.com>
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
use itertools::Itertools;
use parking_lot::Mutex;
use std::fmt;
use std::{io::ErrorKind::WouldBlock, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, WriteHalf, split},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    task::JoinHandle,
    time::{Duration, sleep},
};
use tracing::{debug, error, info, instrument, trace, warn};
use windows::Win32::Foundation::{ERROR_PIPE_NOT_CONNECTED, HLOCAL};
use windows::{
    Win32::{
        Foundation::E_UNEXPECTED,
        System::RemoteDesktop::{
            IWTSListener, IWTSListenerCallback, IWTSListenerCallback_Impl, IWTSPlugin,
            IWTSPlugin_Impl, IWTSVirtualChannel, IWTSVirtualChannelCallback,
            IWTSVirtualChannelCallback_Impl, IWTSVirtualChannelManager,
        },
    },
    core::{AgileReference, BSTR, Error, Interface, PCSTR, Result, implement},
};
use windows_core::{BOOL, OutRef, Owned};
use windows_registry::{CURRENT_USER, Key, LOCAL_MACHINE};

use crate::{
    ASYNC_RUNTIME,
    security_descriptor::{get_logon_sid, security_attributes_from_sddl},
};

pub const REG_PATH: &str = r#"Software\Classes\CLSID\{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}"#;
const REG_VALUE_CHANNEL_NAMES: &str = "ChannelNames";

#[derive(Debug)]
#[implement(IWTSPlugin)]
pub struct RdPipePlugin;

impl RdPipePlugin {
    #[instrument]
    pub fn new() -> Self {
        trace!("Constructing plugin");
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

impl Default for RdPipePlugin {
    fn default() -> Self {
        Self::new()
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
        if channels.is_empty() {
            error!("No channels in registry");
            return Err(Error::from(E_UNEXPECTED));
        }
        for channel_name in channels.into_iter().unique() {
            self.create_listener(channel_mgr, channel_name)?;
        }
        Ok(())
    }

    #[instrument]
    fn Connected(&self) -> Result<()> {
        info!("Client connected");
        Ok(())
    }

    #[instrument]
    fn Disconnected(&self, dwdisconnectcode: u32) -> Result<()> {
        info!("Client disconnected with {}", dwdisconnectcode);
        Ok(())
    }

    #[instrument]
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
    #[instrument]
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl fmt::Debug for RdPipeListenerCallback_Impl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RdPipeListenerCallback_Impl")
            .field("name", &self.name)
            .finish()
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

const MSG_XON: u8 = 0x11;
const MSG_XOFF: u8 = 0x13;

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
    pipe_writer: Arc<Mutex<Option<WriteHalf<NamedPipeServer>>>>,
    join_handle: JoinHandle<()>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(channel: &IWTSVirtualChannel, channel_name: &str) -> Result<Self> {
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        let channel_agile = AgileReference::new(channel)?;
        let pipe_writer = Arc::new(Mutex::new(None));
        debug!("Constructing the callback");

        Ok(Self {
            pipe_writer: pipe_writer.clone(),
            join_handle: Self::process_pipe(pipe_writer, channel_agile, addr),
        })
    }

    #[instrument]
    pub fn process_pipe(
        writer: Arc<Mutex<Option<WriteHalf<NamedPipeServer>>>>,
        channel_agile: AgileReference<IWTSVirtualChannel>,
        pipe_addr: String,
    ) -> JoinHandle<()> {
        ASYNC_RUNTIME.spawn(async move {
            let mut first_pipe_instance = true;
            let login_sid = match get_logon_sid() {
                Ok(s) => s,
                Err(e) => {
                    error!("Can't get login sid,  {}", e);
                    return;
                }
            };
            let sddl = format!("D:(A;;GA;;;{login_sid})", login_sid = login_sid);

            loop {
                trace!(
                    "Creating pipe server with address {}, first instance {}",
                    pipe_addr, first_pipe_instance
                );
                let server = match unsafe {
                    let mut attributes = match security_attributes_from_sddl(&sddl) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Can't create security attributes, {}", e);
                            break;
                        }
                    };
                    let _ = Owned::new(HLOCAL(attributes.lpSecurityDescriptor));

                    ServerOptions::new()
                        .first_pipe_instance(first_pipe_instance)
                        .max_instances(1)
                        .create_with_security_attributes_raw(
                            &pipe_addr,
                            &raw mut attributes as *mut _,
                        )
                } {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Error while creating named pipe server: {}", e);
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                first_pipe_instance = false;
                trace!("Initiate connection to pipe client");
                match server.connect().await {
                    Ok(_) => {
                        let channel = match channel_agile.resolve() {
                            Ok(channel) => channel,
                            Err(e) => {
                                error!("Failed to resolve agile reference for channel: {}", e);
                                break;
                            }
                        };
                        match unsafe { channel.Write(&[MSG_XON], None) } {
                            Ok(_) => trace!("Wrote XON to channel"),
                            Err(e) => {
                                error!("Error writing XON to channel: {}", e);
                            }
                        }
                    }
                    Err(e) => error!("Error connecting to pipe client: {}", e),
                }
                let (mut server_reader, server_writer) = split(server);
                {
                    let mut writer_guard = writer.lock();
                    *writer_guard = Some(server_writer);
                }
                trace!("Pipe client connected. Initiating pipe_reader loop");
                'reader: loop {
                    let mut buf = Vec::with_capacity(64 * 1024);
                    match server_reader.read_buf(&mut buf).await {
                        Ok(0) => {
                            info!("Received 0 bytes, pipe closed by client");
                            let channel = match channel_agile.resolve() {
                                Ok(channel) => channel,
                                Err(e) => {
                                    error!("Failed to resolve agile reference for channel: {}", e);
                                    break 'reader;
                                }
                            };
                            match unsafe { channel.Write(&[MSG_XOFF], None) } {
                                Ok(_) => trace!("Wrote XOFF to channel"),
                                Err(e) => {
                                    error!("Error writing XOFF to channel: {}", e);
                                }
                            }
                            break 'reader;
                        }
                        Ok(n) => {
                            trace!("read {} bytes", n);
                            let channel = match channel_agile.resolve() {
                                Ok(channel) => channel,
                                Err(e) => {
                                    error!("Failed to resolve agile reference for channel: {}", e);
                                    break 'reader;
                                }
                            };
                            match unsafe { channel.Write(&buf, None) } {
                                Ok(_) => trace!("Wrote {} bytes to channel", n),
                                Err(e) => {
                                    error!("Error during write to channel: {}", e);
                                }
                            }
                        }
                        Err(e) if e.kind() == WouldBlock => {
                            warn!("Reading pipe would block: {}", e);
                            continue;
                        }
                        Err(e) => {
                            error!("Error reading from pipe client: {}", e);
                            let channel = match channel_agile.resolve() {
                                Ok(channel) => channel,
                                Err(e) => {
                                    error!("Failed to resolve agile reference for channel: {}", e);
                                    break 'reader;
                                }
                            };
                            match unsafe { channel.Write(&[MSG_XOFF], None) } {
                                Ok(_) => trace!("Wrote XOFF to channel"),
                                Err(e) => {
                                    error!("Error writing XOFF to channel: {}", e);
                                }
                            }
                            break 'reader;
                        }
                    }
                }
                trace!("End of pipe_reader loop, releasing writer");
                {
                    let mut writer_guard = writer.lock();
                    *writer_guard = None;
                }
                trace!("Writer released");
            }
        })
    }
}

impl fmt::Debug for RdPipeChannelCallback_Impl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RdPipeChannelCallback_Impl")
            .field("pipe_writer", &self.pipe_writer)
            .finish()
    }
}
impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback_Impl {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        debug!("Data received, buffer has size {}", cbsize);
        let mut writer_lock = self.pipe_writer.lock();
        writer_lock.as_mut().map_or_else(
            || {
                debug!("Data received without an open named pipe");
                Err(Error::from(ERROR_PIPE_NOT_CONNECTED))
            },
            |writer| {
                let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
                trace!("Writing received data to pipe: {:?}", slice);
                if let Err(e) = ASYNC_RUNTIME.block_on(writer.write_all(slice)) {
                    error!("Error writing received data to pipe: {}", e);
                    return Err(Error::from(ERROR_PIPE_NOT_CONNECTED));
                }
                trace!("Received data written to pipe");
                Ok(())
            },
        )
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        let mut writer_guard = self.pipe_writer.lock();
        if let Some(ref mut writer) = *writer_guard {
            ASYNC_RUNTIME.block_on(writer.shutdown())?;
            *writer_guard = None;
        }
        if !self.join_handle.is_finished() {
            self.join_handle.abort();
        }
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
    fn test_msg_constants() {
        // Verify XON and XOFF message constants
        assert_eq!(MSG_XON, 0x11);
        assert_eq!(MSG_XOFF, 0x13);
        // Ensure they are different
        assert_ne!(MSG_XON, MSG_XOFF);
    }

    #[test]
    fn test_reg_path_format() {
        // Verify registry path format
        assert!(REG_PATH.contains("Software\\Classes\\CLSID"));
        assert!(REG_PATH.contains(&format!("{:?}", crate::registry::CLSID_RD_PIPE_PLUGIN)));
    }

    #[test]
    fn test_channel_names_value_name() {
        // Verify the channel names registry value name
        assert_eq!(REG_VALUE_CHANNEL_NAMES, "ChannelNames");
    }

    #[test]
    fn test_rd_pipe_plugin_default() {
        // Test that default implementation works
        let plugin = RdPipePlugin;
        // Just verify it can be constructed
        assert_eq!(
            std::mem::size_of_val(&plugin),
            std::mem::size_of::<RdPipePlugin>()
        );
    }

    #[test]
    fn test_rd_pipe_plugin_new() {
        // Test that new implementation works
        let plugin = RdPipePlugin::new();
        // Verify it can be constructed
        assert_eq!(
            std::mem::size_of_val(&plugin),
            std::mem::size_of::<RdPipePlugin>()
        );
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
    fn test_listener_callback_new() {
        // Test listener callback construction
        let name = String::from("test_channel");
        let callback = RdPipeListenerCallback::new(name.clone());

        // Verify the name is stored
        assert_eq!(callback.name, name);
    }
}
