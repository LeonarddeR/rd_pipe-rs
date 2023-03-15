// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Class Factory code
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

use std::mem::transmute;
use tracing::{debug, instrument, trace};
use windows::{core::Error, Win32::System::RemoteDesktop::IWTSPlugin};
use windows::{
    core::{implement, ComInterface, IUnknown, Result, GUID},
    Win32::{
        Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_NOINTERFACE},
        System::Com::{IClassFactory, IClassFactory_Impl},
    },
};

use crate::rd_pipe_plugin::RdPipePlugin;

#[implement(IClassFactory)]
#[derive(Debug)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    #[instrument]
    fn CreateInstance(
        &self,
        outer: Option<&IUnknown>,
        iid: *const GUID,
        object: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        let iid = unsafe { *iid };
        let object = unsafe { &mut *object };
        *object = std::ptr::null_mut();
        trace!("Object with type {:?} requested", iid);
        if outer.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        debug!("Creating plugin instance");
        match iid {
            IUnknown::IID => {
                trace!("Requested IUnknown");
                let plugin: IUnknown = RdPipePlugin::new().into();
                *object = unsafe { transmute(plugin) };
            }
            IWTSPlugin::IID => {
                trace!("Requested IWTSPlugin");
                let plugin: IWTSPlugin = RdPipePlugin::new().into();
                *object = unsafe { transmute(plugin) };
            }
            _ => return Err(Error::from(E_NOINTERFACE)),
        }
        Ok(())
    }

    #[instrument]
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        assert!(lock.as_bool());
        Ok(())
    }
}
