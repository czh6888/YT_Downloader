use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenPrivileges, TOKEN_PRIVILEGES,
    LookupPrivilegeValueW, TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

fn main() {
    unsafe {
        let mut h_token: HANDLE = std::ptr::null_mut();
        let rc = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token);
        if rc == 0 {
            eprintln!("OpenProcessToken failed");
            return;
        }

        // Get token info size
        let mut buf_size: u32 = 0;
        GetTokenInformation(h_token, TokenPrivileges, std::ptr::null_mut(), 0, &mut buf_size);

        let mut buf = vec![0u8; buf_size as usize];
        let rc = GetTokenInformation(h_token, TokenPrivileges, buf.as_mut_ptr() as *mut _, buf_size as u32, &mut buf_size);
        CloseHandle(h_token);
        if rc == 0 {
            eprintln!("GetTokenInformation failed");
            return;
        }

        let tp = buf.as_ptr() as *const TOKEN_PRIVILEGES;
        let priv_count = (*tp).PrivilegeCount;
        eprintln!("Token has {} privileges", priv_count);

        // Get SeDebugPrivilege LUID
        let mut debug_luid: LUID = std::mem::zeroed();
        let se_debug_name: Vec<u16> = "SeDebugPrivilege\0".encode_utf16().collect();
        LookupPrivilegeValueW(std::ptr::null(), se_debug_name.as_ptr(), &mut debug_luid);

        // Check if SeDebugPrivilege is in our token
        let luid_attr_base = (buf.as_ptr() as usize + std::mem::size_of::<u32>()) as *const windows_sys::Win32::Security::LUID_AND_ATTRIBUTES;
        for i in 0..priv_count {
            let la = &*luid_attr_base.offset(i as isize);
            if la.Luid.LowPart == debug_luid.LowPart && la.Luid.HighPart == debug_luid.HighPart {
                eprintln!("SeDebugPrivilege FOUND! Attributes: 0x{:x}", la.Attributes);
                if la.Attributes & SE_PRIVILEGE_ENABLED != 0 {
                    eprintln!("  -> ENABLED");
                } else {
                    eprintln!("  -> DISABLED");
                }
                return;
            }
        }
        eprintln!("SeDebugPrivilege NOT in token");
    }
}
