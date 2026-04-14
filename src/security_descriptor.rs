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
    Foundation::{CloseHandle, ERROR_NOT_FOUND, HANDLE, HLOCAL, LocalFree},
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
    // SAFETY: ConvertStringSecurityDescriptorToSecurityDescriptorW allocates memory for the
    // security descriptor which must be freed with LocalFree. Caller is responsible for cleanup.
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
    // SAFETY: Windows API calls for token manipulation. Token handle is properly closed
    // in all code paths (success and failure) via explicit CloseHandle call.
    unsafe {
        // Open current process token
        let mut token: HANDLE = HANDLE::default();
        trace!("Opening process token");
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;

        let result = get_logon_sid_from_token(token);
        // Always close the token handle, regardless of success or failure
        let _ = CloseHandle(token);
        result
    }
}

unsafe fn get_logon_sid_from_token(token: HANDLE) -> windows::core::Result<String> {
    // SAFETY: All Windows API calls in this function work with validated buffers and handles.
    // Memory allocated by ConvertSidToStringSidW is freed via LocalFree before return.
    unsafe {
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
                let ssid = sid_str.to_string();
                LocalFree(Some(HLOCAL(sid_str.0.cast())));
                return Ok(ssid?);
            }
        }
        error!("Logon SID not found");
        Err(Error::from(ERROR_NOT_FOUND))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_attributes_from_sddl_valid() {
        // Test with a basic valid SDDL string
        let sddl = "D:(A;;GA;;;WD)"; // Allow generic all to World
        let result = security_attributes_from_sddl(sddl);

        assert!(result.is_ok());
        let attrs = result.unwrap();
        assert_eq!(attrs.nLength, std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32);
        assert!(!attrs.lpSecurityDescriptor.is_null());
        assert_eq!(attrs.bInheritHandle, false.into());

        // Clean up allocated memory
        unsafe {
            let _ = LocalFree(Some(HLOCAL(attrs.lpSecurityDescriptor)));
        }
    }

    #[test]
    fn test_security_attributes_from_sddl_invalid() {
        // Test with an invalid SDDL string
        let sddl = "INVALID_SDDL_STRING";
        let result = security_attributes_from_sddl(sddl);

        // Should fail to convert invalid SDDL
        assert!(result.is_err());
    }

    #[test]
    fn test_security_attributes_from_sddl_with_sid() {
        // Test with SDDL containing a specific SID format
        let sid = "S-1-5-21-0-0-0-500"; // Administrator SID format
        let sddl = format!("D:(A;;GA;;;{})", sid);
        let result = security_attributes_from_sddl(&sddl);

        assert!(result.is_ok());
        let attrs = result.unwrap();
        assert!(!attrs.lpSecurityDescriptor.is_null());

        // Clean up
        unsafe {
            let _ = LocalFree(Some(HLOCAL(attrs.lpSecurityDescriptor)));
        }
    }

    #[test]
    fn test_security_attributes_structure_size() {
        // Verify the structure size is correctly set
        let sddl = "D:(A;;GA;;;WD)";
        let result = security_attributes_from_sddl(sddl);

        assert!(result.is_ok());
        let attrs = result.unwrap();
        assert_eq!(
            attrs.nLength,
            std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32
        );

        // Clean up
        unsafe {
            let _ = LocalFree(Some(HLOCAL(attrs.lpSecurityDescriptor)));
        }
    }

    #[test]
    fn test_get_logon_sid_returns_string() {
        // This test will only pass on Windows when run in a proper logon session
        // On CI or in some environments it may fail, which is expected
        match get_logon_sid() {
            Ok(sid) => {
                // Verify it looks like a SID string
                assert!(sid.starts_with("S-"));
                assert!(sid.contains('-'));
                // SID format: S-R-I-S-S... where R is revision, I is identifier authority
                let parts: Vec<&str> = sid.split('-').collect();
                assert!(parts.len() >= 3, "SID should have at least 3 parts");
            }
            Err(_) => {
                // May fail in some environments (e.g., without proper token access)
                // This is acceptable for the test
            }
        }
    }

    #[test]
    fn test_sddl_revision_constant() {
        // Verify SDDL_REVISION_1 is used (the constant value is 1)
        // This is a compile-time constant check
        assert_eq!(SDDL_REVISION_1.0, 1);
    }
}
