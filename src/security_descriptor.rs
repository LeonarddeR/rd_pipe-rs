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

use tracing::{debug, error, instrument, trace};
use windows::Win32::{
    Foundation::{ERROR_NOT_FOUND, HANDLE, HLOCAL, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_GROUPS, TOKEN_QUERY,
        TokenGroups,
    },
    System::{
        SystemServices::SE_GROUP_LOGON_ID,
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};
use windows_core::{Error, HSTRING, PWSTR, Result};

#[instrument]
pub fn security_attributes_from_sddl(sddl: &str) -> Result<SECURITY_ATTRIBUTES> {
    trace!("Converting SDDL to security descriptor: {}", sddl);
    let mut security_descriptor: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(sddl),
            SDDL_REVISION_1,
            &mut security_descriptor,
            None,
        )
    }?;
    Ok(SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: security_descriptor.0,
        bInheritHandle: false.into(),
    })
}

#[instrument]
pub fn get_logon_sid() -> windows::core::Result<String> {
    unsafe {
        // Open current process token
        let mut token: HANDLE = HANDLE::default();
        trace!("Opening process token");
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;

        // First call to get buffer size
        let mut len: u32 = 0;
        trace!("Getting token information size");
        GetTokenInformation(token, TokenGroups, None, 0, &mut len).unwrap_or_default();

        let mut buffer = vec![0u8; len as usize];
        // Second call to get actual data
        trace!("Getting token information, expecting size {}", len);
        GetTokenInformation(
            token,
            TokenGroups,
            Some(buffer.as_mut_ptr() as *mut _),
            len,
            &mut len,
        )?;

        let groups = &*(buffer.as_ptr() as *const TOKEN_GROUPS);
        let group_slice =
            std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize);
        debug!("Token group count: {}", groups.GroupCount);

        for group in group_slice {
            debug!("Group SID: {:?}", group.Sid);
            if group.Attributes & SE_GROUP_LOGON_ID as u32 != 0 {
                debug!("Found logon SID");
                let mut sid_str: PWSTR = PWSTR::default();
                ConvertSidToStringSidW(group.Sid, &mut sid_str)?;
                debug!("Converted SID to string: {:?}", sid_str);
                let ssid = sid_str.to_string().unwrap();
                LocalFree(Some(HLOCAL(sid_str.0.cast())));
                return Ok(ssid);
            }
        }
    }
    error!("Logon SID not found");
    Err(Error::from(ERROR_NOT_FOUND))
}
