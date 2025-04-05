// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Security descriptor helper module
// Copyright (C) 2025 Leonard de Ruijter <alderuijter@gmail.com>
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

use std::ptr::{null, null_mut};

use tracing::instrument;
use windows::Win32::{
    Foundation::{HLOCAL, LocalFree},
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
};
use windows_core::{HSTRING, Owned, Result};

#[derive(Debug)]
struct SecurityAttributesWrapper(SECURITY_ATTRIBUTES, Owned<HLOCAL>);

impl SecurityAttributesWrapper {
    #[instrument]
    fn from_sddl(sddl: &str) -> Result<Self> {
        let mut security_descriptor: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(sddl),
                SDDL_REVISION_1,
                &mut security_descriptor,
                None,
            )
        }?;
        let attrs = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security_descriptor.0,
            bInheritHandle: false.into(),
        };
        Ok(SecurityAttributesWrapper(attrs, unsafe {
            Owned::new(HLOCAL(security_descriptor.0))
        }))
    }
}
