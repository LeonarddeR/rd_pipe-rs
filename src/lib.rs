// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Main library entrypoint
// Copyright (C) 2022-2023 Leonard de Ruijter <alderuijter@gmail.com>
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

pub mod class_factory;
pub mod rd_pipe_plugin;
pub mod registry;

use crate::{
    class_factory::ClassFactory, rd_pipe_plugin::RdPipePlugin, registry::CLSID_RD_PIPE_PLUGIN,
};
use clap::Parser;
use rd_pipe_plugin::REG_PATH;
use registry::{
    ctx_add_to_registry, ctx_delete_from_registry, delete_from_registry,
    inproc_server_add_to_registry, msts_add_to_registry, Cli, Scope, COM_CLS_FOLDER,
    TS_ADD_INS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME,
};
use std::{ffi::c_void, io, mem::transmute, panic, str::FromStr};
use tokio::runtime::Runtime;
use tracing::{debug, error, instrument, trace};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{Foundation::S_FALSE, System::LibraryLoader::GetModuleFileNameW},
};
use windows::{
    core::{GUID, HRESULT},
    Win32::{
        Foundation::{BOOL, CLASS_E_CLASSNOTAVAILABLE, E_UNEXPECTED, HINSTANCE, S_OK},
        System::{
            Com::IClassFactory,
            LibraryLoader::DisableThreadLibraryCalls,
            RemoteDesktop::IWTSPlugin,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey, HKEY,
};

lazy_static::lazy_static! {
    static ref ASYNC_RUNTIME: Runtime = {
        trace!("Constructing runtime");
        tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap()
    };
}

const REG_VALUE_LOG_LEVEL: &str = "LogLevel";

fn get_log_level_from_registry(parent_key: HKEY) -> io::Result<u32> {
    let key = RegKey::predef(parent_key);
    let sub_key = key.open_subkey(REG_PATH)?;
    sub_key.get_value(REG_VALUE_LOG_LEVEL)
}

static mut DLL_PATH: Option<String> = None;

#[no_mangle]
pub extern "stdcall" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            let mut file_name: Vec<u16> = Vec::with_capacity(256);
            if unsafe { GetModuleFileNameW(hinst, file_name.as_mut_slice()) } > 0 {
                unsafe {
                    DLL_PATH = Some(String::from_utf16_lossy(&file_name));
                }
            }
            panic::set_hook(Box::new(|info| {
                error!("{:?}", info);
            }));
            // Set up logging
            let file_appender =
                tracing_appender::rolling::never(std::env::temp_dir(), "RdPipe.log");
            let log_level = tracing::Level::from_str(
                &(match get_log_level_from_registry(HKEY_CURRENT_USER) {
                    Ok(l @ 1..=5) => l,
                    _ => get_log_level_from_registry(HKEY_LOCAL_MACHINE).unwrap_or_default(),
                }
                .to_string()),
            )
            .unwrap_or(tracing::Level::WARN);
            tracing_subscriber::fmt()
                .compact()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_max_level(log_level)
                .init();
            trace!(
                "DllMain: DLL_PROCESS_ATTACH, logging at level {}",
                log_level
            );
            unsafe { DisableThreadLibraryCalls(hinst) };
            trace!("Disabled thread library calls");
        }
        DLL_PROCESS_DETACH => {
            debug!("DllMain: DLL_PROCESS_DETACH");
        }
        _ => {}
    }
    BOOL::from(true)
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    debug!("DllGetClassObject called");
    let clsid = unsafe { *rclsid };
    let iid = unsafe { *riid };
    let ppv = unsafe { &mut *ppv };
    // ppv must be null if we fail so set it here for safety
    *ppv = std::ptr::null_mut();

    if clsid != CLSID_RD_PIPE_PLUGIN {
        debug!("DllGetClassObject called for unknown class: {:?}", clsid);
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    if iid != IClassFactory::IID {
        debug!("DllGetClassObject called for unknown interface: {:?}", iid);
        return E_UNEXPECTED;
    }
    debug!("Constructing class factory");
    let factory = ClassFactory;
    let factory: IClassFactory = factory.into();
    debug!("Setting result pointer to class factory");
    *ppv = unsafe { transmute(factory) };

    S_OK
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn VirtualChannelGetInstance(
    riid: *const GUID,
    pnumobjs: *mut u32,
    ppo: *mut *mut c_void,
) -> HRESULT {
    debug!("VirtualChannelGetInstance called");
    let riid = unsafe { *riid };
    if riid != IWTSPlugin::IID {
        debug!(
            "VirtualChannelGetInstance called for unknown interface: {:?}",
            riid
        );
        return E_UNEXPECTED;
    }
    let pnumobjs = unsafe { &mut *pnumobjs };
    debug!("Checking whether result pointer is null (i.e. whether this call is a query for number of plugins or a query for the plugins itself)");
    if ppo.is_null() {
        debug!("Result pointer is null, client is querying for number of objects. Setting pnumobjs to 1, since we only support one plugin");
        *pnumobjs = 1;
    } else {
        debug!("{} plugins requested", *pnumobjs);
        if *pnumobjs != 1 {
            error!("Invalid number of plugins requested: {}", *pnumobjs);
            return E_UNEXPECTED;
        }
        let ppo = unsafe { &mut *ppo };
        debug!("Constructing the plugin");
        let plugin: IWTSPlugin = RdPipePlugin::new().into();
        debug!("Setting result pointer to plugin");
        *ppo = unsafe { transmute(plugin) };
    }
    S_OK
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllInstall(install: bool, cmd_line: *const u16) -> HRESULT {
    if cmd_line.is_null() {
        return S_FALSE;
    }
    let arguments: String = unsafe { PCWSTR::from_raw(cmd_line).to_string() }.unwrap();
    let cli = Cli::try_parse_from(arguments.split(" ")).unwrap();
    let scope_hkey = match cli.scope {
        Scope::CurrentUser => HKEY_CURRENT_USER,
        Scope::LocalMachine => HKEY_LOCAL_MACHINE,
    };
    match install {
        true => {
            if cli.com_server {
                if let Err(_) = inproc_server_add_to_registry(scope_hkey, unsafe {
                    DLL_PATH.as_deref().unwrap()
                }) {
                    return S_FALSE;
                }
            }
            if cli.rdp {
                if let Err(_) = msts_add_to_registry(scope_hkey) {
                    return S_FALSE;
                }
            }
            if cli.citrix {
                if let Err(_) = ctx_add_to_registry(scope_hkey) {
                    return S_FALSE;
                }
            }
        }
        false => {
            if cli.com_server {
                if let Err(_) = delete_from_registry(
                    scope_hkey,
                    COM_CLS_FOLDER,
                    &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN),
                ) {
                    return S_FALSE;
                }
            }
            if cli.rdp {
                if let Err(_) = delete_from_registry(
                    scope_hkey,
                    TS_ADD_INS_FOLDER,
                    TS_ADD_IN_RD_PIPE_FOLDER_NAME,
                ) {
                    return S_FALSE;
                }
            }
            if cli.citrix {
                if let Err(_) = ctx_delete_from_registry(scope_hkey) {
                    return S_FALSE;
                }
            }
        }
    }
    S_OK
}
