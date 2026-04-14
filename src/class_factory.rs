// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Class Factory code
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

use core::{ffi::c_void, fmt, mem::transmute, ptr::null_mut};
use tracing::{debug, instrument, trace};
use windows::{
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE},
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            RemoteDesktop::IWTSPlugin,
        },
    },
    core::{Error, GUID, IUnknown, Interface as _, Result, implement},
};
use windows_core::BOOL;

use crate::rd_pipe_plugin::RdPipePlugin;

#[derive(Debug)]
#[implement(IClassFactory)]
pub struct ClassFactory;

impl fmt::Debug for ClassFactory_Impl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ClassFactory_Impl").finish()
    }
}

impl IClassFactory_Impl for ClassFactory_Impl {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[instrument(skip(outer))]
    fn CreateInstance(
        &self,
        outer: windows_core::Ref<'_, IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        let riid = unsafe { *iid };
        let robject = unsafe { &mut *object };
        *robject = null_mut();
        trace!("Object with type {:?} requested", riid);
        if outer.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        debug!("Creating plugin instance");
        match riid {
            IUnknown::IID => {
                trace!("Requested IUnknown");
                let plugin: IUnknown = RdPipePlugin::new().into();
                *robject = unsafe { transmute::<IUnknown, *mut c_void>(plugin) };
            }
            IWTSPlugin::IID => {
                trace!("Requested IWTSPlugin");
                let plugin: IWTSPlugin = RdPipePlugin::new().into();
                *robject = unsafe { transmute::<IWTSPlugin, *mut c_void>(plugin) };
            }
            _ => return Err(Error::from(E_NOINTERFACE)),
        }
        Ok(())
    }

    #[instrument]
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_factory_construction() {
        // Test that ClassFactory can be constructed
        let factory = ClassFactory;
        // Verify size is zero (empty struct)
        assert_eq!(std::mem::size_of_val(&factory), 0);
    }

    #[test]
    fn test_class_factory_into_iclassfactory() {
        // Test conversion to IClassFactory interface
        let factory = ClassFactory;
        let _interface: IClassFactory = factory.into();
        // If we get here without panicking, the conversion succeeded
    }

    #[test]
    fn test_lock_server_always_succeeds() {
        // LockServer should always return Ok
        let factory = ClassFactory;
        let factory_impl = factory.into_outer();

        // Test both lock and unlock
        assert!(factory_impl.LockServer(true.into()).is_ok());
        assert!(factory_impl.LockServer(false.into()).is_ok());
    }

    #[test]
    fn test_supported_interface_iids() {
        // Verify that we handle the expected interface IIDs
        // IUnknown::IID and IWTSPlugin::IID should be supported

        // These are the GUIDs we expect to handle
        let iunknown_iid = IUnknown::IID;
        let iwtsplugin_iid = IWTSPlugin::IID;

        // Verify they are different
        assert_ne!(iunknown_iid, iwtsplugin_iid);

        // Verify they are not null GUIDs
        assert_ne!(iunknown_iid, GUID::zeroed());
        assert_ne!(iwtsplugin_iid, GUID::zeroed());
    }

    #[test]
    fn test_class_factory_debug_impl() {
        // Test that Debug is properly implemented
        let factory = ClassFactory;
        let debug_str = format!("{:?}", factory);
        assert!(debug_str.contains("ClassFactory"));
    }

    #[test]
    fn test_class_factory_impl_debug() {
        // Test that ClassFactory_Impl Debug is properly implemented
        let factory_impl = ClassFactory.into_outer();
        let debug_str = format!("{:?}", factory_impl);
        assert!(debug_str.contains("ClassFactory_Impl"));
    }
}
