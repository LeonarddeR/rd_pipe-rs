// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Main library entrypoint
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

pub mod class_factory;
pub mod rd_pipe_plugin;
pub mod registry;
pub mod security_descriptor;

use crate::{class_factory::ClassFactory, registry::CLSID_RD_PIPE_PLUGIN};
use core::{ffi::c_void, str::FromStr};
use rd_pipe_plugin::REG_PATH;
use registry::{
    COM_CLS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME, TS_ADD_INS_FOLDER, delete_from_registry,
    inproc_server_add_to_registry, msts_add_to_registry,
};
#[cfg(target_arch = "x86")]
use registry::{ctx_add_to_registry, ctx_delete_from_registry};
use std::{panic, sync::LazyLock};
use tokio::runtime::Runtime;
use tracing::{debug, error, instrument, trace};
use windows::{
    Win32::{
        Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_UNEXPECTED, S_OK},
        System::{
            Com::IClassFactory,
            LibraryLoader::DisableThreadLibraryCalls,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
    core::{GUID, HRESULT},
};
use windows::{
    Win32::{
        Foundation::{ERROR_INVALID_PARAMETER, HMODULE},
        System::LibraryLoader::GetModuleFileNameW,
    },
    core::{Interface, PCWSTR},
};
use windows_core::{BOOL, OutRef, Ref};
use windows_registry::{self, CURRENT_USER, LOCAL_MACHINE};

static ASYNC_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    trace!("Constructing runtime");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

const REG_VALUE_LOG_LEVEL: &str = "LogLevel";

fn get_log_level_from_registry(parent_key: &windows_registry::Key) -> windows_core::Result<u32> {
    let sub_key = parent_key.open(REG_PATH)?;
    sub_key.get_u32(REG_VALUE_LOG_LEVEL)
}

static mut INSTANCE: Option<HMODULE> = None;

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            unsafe {
                INSTANCE = Some(hinst);
            }
            // Set up logging
            let file_appender =
                tracing_appender::rolling::never(std::env::temp_dir(), "RdPipe.log");
            let log_level = tracing::Level::from_str(
                &(match get_log_level_from_registry(CURRENT_USER) {
                    Ok(l @ 1..=5) => l,
                    _ => get_log_level_from_registry(LOCAL_MACHINE).unwrap_or_default(),
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
            panic::set_hook(Box::new(|info| {
                error!("{:?}", info);
            }));
            trace!(
                "DllMain: DLL_PROCESS_ATTACH, logging at level {}",
                log_level
            );
            unsafe { DisableThreadLibraryCalls(hinst) }.unwrap();
            trace!("Disabled thread library calls");
        }
        DLL_PROCESS_DETACH => {
            debug!("DllMain: DLL_PROCESS_DETACH");
        }
        _ => {}
    }
    true.into()
}

#[unsafe(no_mangle)]
#[instrument(skip_all)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: Ref<GUID>,
    riid: Ref<GUID>,
    ppv: OutRef<IClassFactory>,
) -> HRESULT {
    debug!("DllGetClassObject called");
    let clsid = match rclsid.ok() {
        Ok(c) => *c,
        Err(e) => {
            return e.into();
        }
    };
    let iid = match riid.ok() {
        Ok(i) => *i,
        Err(e) => {
            return e.into();
        }
    };

    if clsid != CLSID_RD_PIPE_PLUGIN {
        error!("DllGetClassObject called for unknown class: {:?}", clsid);
        _ = ppv.write(None);
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    if iid != IClassFactory::IID {
        error!("DllGetClassObject called for unknown interface: {:?}", iid);
        _ = ppv.write(None);
        return E_UNEXPECTED;
    }
    trace!("Constructing class factory");
    let factory = ClassFactory;
    trace!("Setting result pointer to class factory");
    ppv.write(Some(factory.into())).into()
}

const CMD_COM_SERVER: char = 'c'; // Registers/unregisters the COM server
const CMD_MSTS: char = 'r'; // Registers/unregisters RDP/MSTS support
const CMD_CITRIX: char = 'x'; // Registers/unregisters Citrix support
const CMD_LOCAL_MACHINE: char = 'm'; // If omitted, registers to HKEY_CURRENT_USER

#[unsafe(no_mangle)]
#[instrument]
pub extern "system" fn DllInstall(install: bool, cmd_line: PCWSTR) -> HRESULT {
    let path_string: String;
    debug!("DllInstall called");
    if cmd_line.is_null() {
        error!("No command line provided");
        return ERROR_INVALID_PARAMETER.into();
    }
    let arguments: String = match unsafe { cmd_line.to_string() } {
        Ok(s) => {
            trace!("Command line has: {}", &s);
            s
        }
        Err(e) => {
            error!("Couldn't convert arguments from PCWSTR: {}", e);
            return ERROR_INVALID_PARAMETER.into();
        }
    };
    if arguments.is_empty() {
        error!("No arguments provided");
        return ERROR_INVALID_PARAMETER.into();
    }
    let arguments: Vec<&str> = arguments.split(" ").collect();
    let commands = arguments[0].to_lowercase();
    #[cfg(not(target_arch = "x86"))]
    if commands.contains(CMD_CITRIX) {
        error!("Citrix registration not supported for non-X86 builds");
        return ERROR_INVALID_PARAMETER.into();
    }
    let scope_hkey = match commands.contains(CMD_LOCAL_MACHINE) {
        true => LOCAL_MACHINE,
        false => CURRENT_USER,
    };
    match install {
        true => {
            if commands.contains(CMD_COM_SERVER) {
                if arguments.len() == 1 {
                    error!("No channel names provided");
                    return ERROR_INVALID_PARAMETER.into();
                }
                let mut file_name = [0u16; 256];
                match unsafe { GetModuleFileNameW(INSTANCE, file_name.as_mut()) } > 0 {
                    true => {
                        path_string = String::from_utf16_lossy(&file_name);
                    }
                    false => {
                        let e = windows::core::Error::from_win32();
                        error!("Error calling GetModuleFileNameW: {}", e);
                        return e.into();
                    }
                }
                if let Err(e) = inproc_server_add_to_registry(
                    scope_hkey,
                    COM_CLS_FOLDER,
                    &path_string,
                    &arguments[1..],
                ) {
                    error!("Error calling inproc_server_add_to_registry: {}", e);
                    return e.into();
                }
            }
            if commands.contains(CMD_MSTS) {
                if let Err(e) = msts_add_to_registry(scope_hkey) {
                    error!("Error calling msts_add_to_registry: {}", e);
                    return e.into();
                }
            }
            #[cfg(target_arch = "x86")]
            if commands.contains(CMD_CITRIX) {
                if let Err(e) = ctx_add_to_registry(scope_hkey) {
                    error!("Error calling ctx_add_to_registry: {}", e);
                    return e.into();
                }
            }
        }
        false => {
            #[cfg(target_arch = "x86")]
            if commands.contains(CMD_CITRIX) {
                if let Err(e) = ctx_delete_from_registry(scope_hkey) {
                    error!("Error calling ctx_delete_from_registry: {}", e);
                    return e.into();
                }
            }
            if commands.contains(CMD_MSTS) {
                if let Err(e) = delete_from_registry(
                    scope_hkey,
                    TS_ADD_INS_FOLDER,
                    TS_ADD_IN_RD_PIPE_FOLDER_NAME,
                ) {
                    error!("Error calling delete_from_registry: {}", e);
                    return e.into();
                }
            }
            if commands.contains(CMD_COM_SERVER) {
                if let Err(e) = delete_from_registry(
                    scope_hkey,
                    COM_CLS_FOLDER,
                    &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN),
                ) {
                    error!("Error calling delete_from_registry: {}", e);
                    return e.into();
                }
            }
        }
    }
    S_OK
}
