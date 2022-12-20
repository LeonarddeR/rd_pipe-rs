// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Dynamic Virtual Channel Plugin structs
// Copyright (C) 2022 Leonard de Ruijter <alderuijter@gmail.com>
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
use std::ffi::c_void;
use std::{
    io::{self, ErrorKind::WouldBlock},
    sync::Arc,
};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, WriteHalf},
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace, warn};
use windows::{
    core::{implement, AgileReference, Error, Result, Vtable, BSTR, PCSTR},
    s,
    Win32::{
        Foundation::{BOOL, ERROR_SUCCESS, E_UNEXPECTED, S_FALSE},
        System::{
            Registry::{
                RegGetValueA, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_MULTI_SZ,
            },
            RemoteDesktop::{
                IWTSListener, IWTSListenerCallback, IWTSListenerCallback_Impl, IWTSPlugin,
                IWTSPlugin_Impl, IWTSVirtualChannel, IWTSVirtualChannelCallback,
                IWTSVirtualChannelCallback_Impl, IWTSVirtualChannelManager,
            },
        },
    },
};

use crate::ASYNC_RUNTIME;

const REG_PATH: PCSTR = s!(r#"Software\Microsoft\Terminal Server Client\Default\AddIns\RdPipe"#);
const REG_VALUE: PCSTR = s!("ChannelNames");

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
        channel_name: PCSTR,
    ) -> Result<IWTSListener> {
        debug!("Creating listener with name {}", unsafe {
            channel_name.to_string().unwrap_or_default()
        });
        let callback: IWTSListenerCallback =
            RdPipeListenerCallback::new(unsafe { channel_name.to_string() }.unwrap()).into();
        unsafe { channel_mgr.CreateListener(channel_name, 0, &callback) }
    }

    #[instrument]
    fn get_channel_names_from_registry(parent_key: HKEY) -> Result<Vec<Vec<u8>>> {
        let mut size: u32 = 0;
        let size_ptr: *mut u32 = &mut size;
        let res = unsafe {
            RegGetValueA(
                parent_key,
                REG_PATH,
                REG_VALUE,
                RRF_RT_REG_MULTI_SZ,
                None,
                None,
                Some(size_ptr),
            )
        };
        if res != ERROR_SUCCESS {
            return Err(Error::from(res));
        }
        let mut value: Vec<u8> = vec![0; size as _];
        let res = unsafe {
            RegGetValueA(
                parent_key,
                REG_PATH,
                REG_VALUE,
                RRF_RT_REG_MULTI_SZ,
                None,
                Some(value.as_mut_ptr() as *mut c_void),
                Some(size_ptr),
            )
        };
        if res != ERROR_SUCCESS {
            return Err(Error::from(res));
        }
        let v: Vec<Vec<u8>> = value
            .split_inclusive(|c| *c == 0)
            .filter(|s| s[0] != 0)
            .map(|s| s.to_owned())
            .collect();
        Ok(v)
    }
}

impl IWTSPlugin_Impl for RdPipePlugin {
    #[instrument]
    fn Initialize(&self, pchannelmgr: &Option<IWTSVirtualChannelManager>) -> Result<()> {
        let channel_mgr = match pchannelmgr {
            Some(m) => m,
            None => {
                error!("No pchannelmgr given when initializing");
                return Err(Error::from(E_UNEXPECTED));
            }
        };
        let mut channels: Vec<Vec<u8>> = Vec::new();
        channels.extend(
            RdPipePlugin::get_channel_names_from_registry(HKEY_CURRENT_USER).unwrap_or_default(),
        );
        channels.extend(
            RdPipePlugin::get_channel_names_from_registry(HKEY_LOCAL_MACHINE).unwrap_or_default(),
        );
        if channels.len() == 0 {
            error!("No channels in registry");
            return Err(Error::from(E_UNEXPECTED));
        }
        for channel_name in channels.iter().unique() {
            self.create_listener(channel_mgr, PCSTR::from_raw(channel_name.as_ptr()))?;
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
        Self { name: name }
    }
}

impl IWTSListenerCallback_Impl for RdPipeListenerCallback {
    #[instrument]
    fn OnNewChannelConnection(
        &self,
        pchannel: &Option<IWTSVirtualChannel>,
        data: &BSTR,
        pbaccept: *mut BOOL,
        ppcallback: *mut Option<IWTSVirtualChannelCallback>,
    ) -> Result<()> {
        debug!(
            "Creating new callback for channel {:?} with name {}",
            pchannel, &self.name
        );
        let channel = match pchannel {
            Some(c) => c.to_owned(),
            None => return Err(Error::from(E_UNEXPECTED)),
        };
        let pbaccept = unsafe { &mut *pbaccept };
        let ppcallback = unsafe { &mut *ppcallback };
        *pbaccept = BOOL::from(true);
        debug!("Creating callback");
        let callback: IWTSVirtualChannelCallback =
            RdPipeChannelCallback::new(channel, &self.name).into();
        trace!("Callback {:?} created", callback);
        *ppcallback = Some(callback);
        Ok(())
    }
}

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\RDPipe";

#[derive(Debug)]
#[implement(IWTSVirtualChannelCallback)]
pub struct RdPipeChannelCallback {
    pipe_writer: Arc<Mutex<Option<WriteHalf<NamedPipeServer>>>>,
}

impl RdPipeChannelCallback {
    #[instrument]
    pub fn new(channel: IWTSVirtualChannel, channel_name: &str) -> Self {
        let addr = format!(
            "{}_{}_{}",
            PIPE_NAME_PREFIX,
            channel_name,
            channel.as_raw() as usize
        );
        debug!("Creating agile reference to channel");
        let channel_agile = AgileReference::new(&channel).unwrap();
        debug!("Constructing the callback");
        let callback = Self {
            pipe_writer: Arc::new(Mutex::new(None)),
        };
        debug!("Spawning process_messages task");
        callback.process_pipe(channel_agile, addr);
        callback
    }

    #[instrument]
    fn process_pipe(
        &self,
        channel_agile: AgileReference<IWTSVirtualChannel>,
        pipe_addr: String,
    ) -> JoinHandle<io::Result<()>> {
        let writer = self.pipe_writer.clone();
        ASYNC_RUNTIME.spawn(async move {
            let mut first_pipe_instance = true;
            loop {
                trace!("Creating pipe server with address {}", pipe_addr);
                let server = ServerOptions::new()
                    .first_pipe_instance(first_pipe_instance)
                    .max_instances(1)
                    .pipe_mode(PipeMode::Message)
                    .create(&pipe_addr)
                    .unwrap();
                first_pipe_instance = false;
                trace!("Initiate connection to pipe client");
                server.connect().await.unwrap();
                let (mut server_reader, server_writer) = split(server);
                {
                    let mut writer_guard = writer.lock();
                    *writer_guard = Some(server_writer);
                }
                trace!("Pipe client connected. Initiating pipe_reader loop");
                loop {
                    let mut buf = Vec::with_capacity(64 *1024);
                    match server_reader.read_buf(&mut buf).await {
                        Ok(0) => {
                            info!("Received 0 bytes, pipe closed by client");
                            break;
                        }
                        Ok(n) => {
                            trace!("read {} bytes", n);
                            let channel = channel_agile.resolve().unwrap();
                            unsafe { channel.Write(&buf, None) }.unwrap();
                        }
                        Err(e) if e.kind() == WouldBlock => {
                            warn!("Reading pipe would block: {}", e);
                            continue;
                        }
                        Err(e) => {
                            error!("Error reading from pipe server: {}", e);
                            break;
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

impl Drop for RdPipeChannelCallback {
    #[instrument]
    fn drop(&mut self) {
        self.OnClose().unwrap_or_default();
    }
}

impl IWTSVirtualChannelCallback_Impl for RdPipeChannelCallback {
    #[instrument]
    fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> Result<()> {
        debug!("Data received, buffer has size {}", cbsize);
        let mut writer_lock = self.pipe_writer.lock();
        match *writer_lock {
            Some(ref mut writer) => {
                let slice = unsafe { slice::from_raw_parts(pbuffer, cbsize as usize) };
                trace!("Writing received data to pipe: {:?}", slice);
                ASYNC_RUNTIME.block_on(writer.write(slice)).unwrap();
                trace!("Received data written to pipe");
                Ok(())
            }
            None => {
                debug!("Data received without an open named pipe");
                Err(Error::from(S_FALSE))
            }
        }
    }

    #[instrument]
    fn OnClose(&self) -> Result<()> {
        let mut writer_lock = self.pipe_writer.lock();
        if let Some(ref mut writer) = *writer_lock {
            ASYNC_RUNTIME.block_on(writer.shutdown()).unwrap();
        }
        Ok(())
    }
}
