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

use crate::{class_factory::ClassFactory, rd_pipe_plugin::RdPipePlugin};
use std::{ffi::c_void, mem::transmute, panic};
use tokio::runtime::Runtime;
use tracing::{debug, error, instrument, trace};
use windows::{
    core::{Interface, GUID, HRESULT},
    Win32::{
        Foundation::{BOOL, E_UNEXPECTED, HINSTANCE, S_OK},
        System::{
            Com::IClassFactory,
            LibraryLoader::DisableThreadLibraryCalls,
            RemoteDesktop::IWTSPlugin,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

lazy_static::lazy_static! {
    static ref ASYNC_RUNTIME: Runtime = {
        trace!("Constructing runtime");
        tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap()
    };
}

#[no_mangle]
#[instrument]
pub extern "stdcall" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            panic::set_hook(Box::new(|info| {
                error!("{:?}", info);
            }));
            // Set up logging
            let file_appender = tracing_appender::rolling::never("d:", "RdPipe.log");
            tracing_subscriber::fmt()
                .compact()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_max_level(tracing::Level::TRACE)
                .init();
            trace!("DllMain: DLL_PROCESS_ATTACH");
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
    _rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    debug!("DllGetClassObject called");
    let riid = unsafe { *riid };
    let ppv = unsafe { &mut *ppv };
    // ppv must be null if we fail so set it here for safety
    *ppv = std::ptr::null_mut();

    if riid != IClassFactory::IID {
        debug!("DllGetClassObject called for unknown interface: {:?}", riid);
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
