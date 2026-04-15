use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::{Context, Result};
use chacha20poly1305::ChaCha20Poly1305;
use rusqlite::Connection;
use std::io::Cursor;
use std::path::PathBuf;

use super::netscape;

/// Browser-specific paths.
pub struct BrowserPaths {
    pub cookie_db: PathBuf,
    pub local_state: PathBuf,
    pub key_name: &'static str,
}

impl BrowserPaths {
    pub fn chrome() -> Self {
        let up = std::env::var("USERPROFILE").unwrap_or_default();
        Self {
            cookie_db: PathBuf::from(&up)
                .join(r"AppData\Local\Google\Chrome\User Data\Default\Network\Cookies"),
            local_state: PathBuf::from(&up)
                .join(r"AppData\Local\Google\Chrome\User Data\Local State"),
            key_name: "Google Chromekey1",
        }
    }

    pub fn edge() -> Self {
        let up = std::env::var("USERPROFILE").unwrap_or_default();
        Self {
            cookie_db: PathBuf::from(&up)
                .join(r"AppData\Local\Microsoft\Edge\User Data\Default\Network\Cookies"),
            local_state: PathBuf::from(&up)
                .join(r"AppData\Local\Microsoft\Edge\User Data\Local State"),
            key_name: "Microsoft EdgeKey1",
        }
    }
}

/// Known hardcoded keys for Chromium v20 decryption.
const CHROME_FIXED_AES_KEY: [u8; 32] = [
    0xB3, 0x1C, 0x6E, 0x24, 0x1A, 0xC8, 0x46, 0x72, 0x8D, 0xA9, 0xC1, 0xFA, 0xC4, 0x93, 0x66, 0x51,
    0xCF, 0xFB, 0x94, 0x4D, 0x14, 0x3A, 0xB8, 0x16, 0x27, 0x6B, 0xCC, 0x6D, 0xA0, 0x28, 0x47, 0x87,
];
const CHROME_FIXED_CHACHA_KEY: [u8; 32] = [
    0xE9, 0x8F, 0x37, 0xD7, 0xF4, 0xE1, 0xFA, 0x43, 0x3D, 0x19, 0x30, 0x4D, 0xC2, 0x25, 0x80, 0x42,
    0x09, 0x0E, 0x2D, 0x1D, 0x7E, 0xEA, 0x76, 0x70, 0xD4, 0x1F, 0x73, 0x8D, 0x08, 0x72, 0x96, 0x60,
];

/// XOR key used for flag=3 CNG decryption.
const ABE_XOR_KEY: [u8; 32] = [
    0xCC, 0xF8, 0xA1, 0xCE, 0xC5, 0x66, 0x05, 0xB8, 0x51, 0x75, 0x52, 0xBA, 0x1A, 0x2D, 0x06, 0x1C,
    0x03, 0xA2, 0x9E, 0x90, 0x27, 0x4F, 0xB2, 0xFC, 0xF5, 0x9B, 0xA4, 0xB7, 0x5C, 0x39, 0x23, 0x90,
];

/// Extract cookies from Chromium-based browsers (Chrome/Edge).
///
/// Uses pure Rust with lsass impersonation via standard Windows APIs:
/// 1. Enable SeDebugPrivilege
/// 2. Open lsass with PROCESS_QUERY_LIMITED_INFORMATION (PPL-compatible)
/// 3. Duplicate lsass token and impersonate (SYSTEM context)
/// 4. Double DPAPI decrypt
/// 5. For Chrome flag=3: CNG NCryptDecrypt (still in SYSTEM context)
/// 6. For Edge: extract raw master key from decrypted blob
/// Falls back to Python subprocess if any step fails.
pub fn extract_cookies(paths: &BrowserPaths, cookie_file: &PathBuf) -> Result<()> {
    // Step 1: Read Local State
    let local_state_text = std::fs::read_to_string(&paths.local_state)
        .context("Failed to read browser Local State file")?;
    let local_state: serde_json::Value =
        serde_json::from_str(&local_state_text).context("Failed to parse Local State JSON")?;

    // Step 2: Get app_bound_encrypted_key
    let abek_b64 = local_state
        .get("os_crypt")
        .and_then(|v| v.get("app_bound_encrypted_key"))
        .and_then(|v| v.as_str())
        .context("app_bound_encrypted_key not found in Local State")?;
    let abek = data_encoding::BASE64.decode(abek_b64.as_bytes())?;

    if &abek[..4] != b"APPB" {
        anyhow::bail!("Unexpected APPB header: {:?}", &abek[..4]);
    }
    let enc_key = &abek[4..];

    // Step 3: Double DPAPI decrypt (SYSTEM -> User)
    // Returns (user_decrypted_blob, lsass_token_handle) for potential CNG re-impersonation.
    let (user_dec, h_lsass_token) = match double_dpapi_decrypt(enc_key) {
        Ok(v) => v,
        Err(e) => {
            log::debug!("Rust DPAPI failed: {e}, trying Python fallback...");
            return extract_via_python(paths, cookie_file);
        }
    };

    // Step 4: Extract master key
    // Edge uses a simple structure: header_len + header + content_len + master_key
    // Chrome uses a flag-based structure with CNG for flag=3
    let master_key = if paths.key_name.contains("Edge") {
        extract_edge_master_key(&user_dec).map_err(|e| {
            log::debug!("Edge key extraction failed: {e}");
            e
        })?
    } else {
        // Chrome: parse key blob with flag-based structure
        let parsed = match parse_key_blob(&user_dec) {
            Ok(blob) => blob,
            Err(_) => {
                parse_key_blob_edge(&user_dec)
                    .context("Failed to parse key blob (tried both Chrome and Edge formats)")?
            }
        };

        // For flag=3, re-impersonate lsass for CNCryptDecrypt
        if parsed.flag == 3 {
            let impersonated = unsafe {
                windows_sys::Win32::Security::ImpersonateLoggedOnUser(h_lsass_token)
            };
            if impersonated == 0 {
                unsafe { windows_sys::Win32::Foundation::CloseHandle(h_lsass_token) };
                anyhow::bail!("Re-ImpersonateLoggedOnUser failed");
            }

            let result = derive_master_key(&parsed, paths.key_name);

            unsafe { windows_sys::Win32::Security::RevertToSelf() };
            unsafe { windows_sys::Win32::Foundation::CloseHandle(h_lsass_token) };

            result?
        } else {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(h_lsass_token) };
            derive_master_key(&parsed, paths.key_name)?
        }
    };

    // Step 5: Read cookie database
    let cookies = read_cookies(&paths.cookie_db)?;

    // Step 6: Decrypt cookies and write
    let decrypted = decrypt_all_cookies(&master_key, &cookies)?;
    let content = netscape::to_netscape(&decrypted);
    std::fs::write(cookie_file, content)
        .with_context(|| format!("Failed to write cookie file to {}", cookie_file.display()))?;

    Ok(())
}

/// Extract cookies using the Python decrypt script as fallback.
pub fn extract_via_python(paths: &BrowserPaths, cookie_file: &PathBuf) -> Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir).parent()
        .ok_or_else(|| anyhow::anyhow!("Could not find project root"))?;

    let script_path = if paths.key_name.contains("Chrome") {
        project_root.join("yt_downloader").join("decrypt_chrome_v20.py")
    } else {
        project_root.join("yt_downloader").join("decrypt_edge_v20.py")
    };

    if !script_path.exists() {
        anyhow::bail!("Python decrypt script not found at {:?}", script_path);
    }

    log::info!("Using Python fallback: {:?}", script_path);

    let python_cmds = ["python", "python3"];
    let mut last_err = None;

    for cmd in &python_cmds {
        let mut child = match std::process::Command::new(cmd)
            .arg(&script_path)
            .arg(cookie_file.to_str().unwrap())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                last_err = Some(format!("{cmd} not found: {e}"));
                continue;
            }
        };

        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() && cookie_file.exists() {
                        log::info!("Python extraction succeeded via {cmd}");
                        return Ok(());
                    }
                    let stdout = child.stdout.take().map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    last_err = Some(format!(
                        "{cmd} failed (exit={status}): stderr={stderr}, stdout={stdout}"
                    ));
                    break;
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        last_err = Some(format!("{cmd} timed out after {timeout:?}"));
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Err(e) => {
                    last_err = Some(format!("{cmd} wait error: {e}"));
                    break;
                }
            }
        }
    }

    anyhow::bail!(
        "Python extraction failed: {}",
        last_err.unwrap_or_else(|| "Unknown error".to_string())
    )
}

// ---------------------------------------------------------------------------
// Double DPAPI: SYSTEM (lsass impersonation) -> User
// ---------------------------------------------------------------------------

/// Double DPAPI decrypt: SYSTEM -> User.
/// Returns (user_decrypted_key_blob, lsass_token_handle).
/// The lsass_token is kept open so we can re-impersonate for CNCryptDecrypt.
fn double_dpapi_decrypt(enc_key: &[u8]) -> Result<(Vec<u8>, windows_sys::Win32::Foundation::HANDLE)> {
    // Step 1: Enable SeDebugPrivilege (needed for OpenProcess on lsass)
    enable_debug_privilege().context("Failed to enable SeDebugPrivilege")?;

    // Step 2: Find lsass.exe PID
    let lsass_pid = find_lsass_pid().context("Could not find lsass.exe process")?;

    // Step 3: Open lsass process and duplicate its token
    let h_token = duplicate_lsass_token(lsass_pid)
        .context("Failed to duplicate lsass token")?;

    // Step 4: Impersonate lsass token (switch to SYSTEM context)
    let impersonated = unsafe {
        windows_sys::Win32::Security::ImpersonateLoggedOnUser(h_token)
    };
    if impersonated == 0 {
        let err = std::io::Error::last_os_error();
        unsafe { windows_sys::Win32::Foundation::CloseHandle(h_token) };
        anyhow::bail!("ImpersonateLoggedOnUser failed: {err}");
    }

    // Step 5: First DPAPI decrypt in SYSTEM context
    let sys_dec = match dpapi_unprotect(enc_key) {
        Ok(v) => v,
        Err(e) => {
            unsafe { windows_sys::Win32::Security::RevertToSelf() };
            unsafe { windows_sys::Win32::Foundation::CloseHandle(h_token) };
            return Err(e);
        }
    };

    // Step 6: Revert to user context BEFORE second DPAPI
    // (the second layer is encrypted with the USER's DPAPI key)
    unsafe { windows_sys::Win32::Security::RevertToSelf() };

    // Step 7: Second DPAPI decrypt in user context
    let user_dec = match dpapi_unprotect(&sys_dec) {
        Ok(v) => v,
        Err(e) => {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(h_token) };
            return Err(e);
        }
    };

    // Keep h_token open for re-impersonation (CNG key access)
    Ok((user_dec, h_token))
}

/// Enable SeDebugPrivilege on the current process token.
fn enable_debug_privilege() -> Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW, TOKEN_ADJUST_PRIVILEGES,
        TOKEN_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut h_token: HANDLE = std::ptr::null_mut();
        let rc = OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut h_token,
        );
        if rc == 0 {
            anyhow::bail!("OpenProcessToken failed");
        }

        let mut luid: LUID = std::mem::zeroed();
        let se_debug_name: Vec<u16> = "SeDebugPrivilege\0".encode_utf16().collect();
        let rc = LookupPrivilegeValueW(std::ptr::null(), se_debug_name.as_ptr(), &mut luid);
        if rc == 0 {
            CloseHandle(h_token);
            anyhow::bail!("LookupPrivilegeValueW(SeDebugPrivilege) failed");
        }

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [windows_sys::Win32::Security::LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        let rc = AdjustTokenPrivileges(
            h_token,
            0,
            &tp as *const TOKEN_PRIVILEGES as *mut TOKEN_PRIVILEGES,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if rc == 0 {
            CloseHandle(h_token);
            anyhow::bail!("AdjustTokenPrivileges failed");
        }

        // AdjustTokenPrivileges can return non-zero even if the privilege
        // wasn't actually granted. Check GetLastError.
        const ERROR_NOT_ALL_ASSIGNED: u32 = 1300;
        let last_err = GetLastError();
        if last_err == ERROR_NOT_ALL_ASSIGNED {
            CloseHandle(h_token);
            anyhow::bail!("SeDebugPrivilege not available in current token");
        }

        CloseHandle(h_token);
    }

    Ok(())
}

/// Find lsass.exe PID using ToolHelp snapshot.
fn find_lsass_pid() -> Result<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
        PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            anyhow::bail!("CreateToolhelp32Snapshot failed");
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            anyhow::bail!("Process32FirstW failed");
        }

        loop {
            let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
            if name.to_lowercase() == "lsass.exe" {
                let pid = entry.th32ProcessID;
                CloseHandle(snapshot);
                return Ok(pid);
            }

            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
    }

    anyhow::bail!("lsass.exe not found in process list")
}

/// Open lsass process, duplicate its token for impersonation.
/// Uses PROCESS_QUERY_LIMITED_INFORMATION which is allowed by PPL.
fn duplicate_lsass_token(pid: u32) -> Result<windows_sys::Win32::Foundation::HANDLE> {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::{
        DuplicateTokenEx, SecurityImpersonation, TokenImpersonation,
        TOKEN_ALL_ACCESS, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        // PPL-protected processes (like lsass) only allow PROCESS_QUERY_LIMITED_INFORMATION.
        let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h_process.is_null() {
            let err = std::io::Error::last_os_error();
            anyhow::bail!("OpenProcess(lsass pid={pid}) failed: {err}");
        }

        let mut h_lsass_token: HANDLE = std::ptr::null_mut();
        let rc = OpenProcessToken(h_process, TOKEN_DUPLICATE | TOKEN_QUERY, &mut h_lsass_token);
        if rc == 0 {
            let err = std::io::Error::last_os_error();
            windows_sys::Win32::Foundation::CloseHandle(h_process);
            anyhow::bail!("OpenProcessToken(lsass) failed: {err}");
        }

        windows_sys::Win32::Foundation::CloseHandle(h_process);

        let mut h_impersonation_token: HANDLE = std::ptr::null_mut();
        let rc = DuplicateTokenEx(
            h_lsass_token,
            TOKEN_ALL_ACCESS,
            std::ptr::null_mut(),
            SecurityImpersonation,
            TokenImpersonation,
            &mut h_impersonation_token,
        );
        windows_sys::Win32::Foundation::CloseHandle(h_lsass_token);

        if rc == 0 {
            let err = std::io::Error::last_os_error();
            anyhow::bail!("DuplicateTokenEx failed: {err}");
        }

        Ok(h_impersonation_token)
    }
}

/// Call CryptUnprotectData on the given data.
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    unsafe {
        let data_in = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };

        let mut data_out: CRYPT_INTEGER_BLOB = std::mem::zeroed();
        let mut data_desc: *mut u16 = std::ptr::null_mut();
        let result = CryptUnprotectData(
            &data_in as *const CRYPT_INTEGER_BLOB,
            &mut data_desc,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        );

        if result == 0 {
            anyhow::bail!("CryptUnprotectData failed");
        }

        let decrypted = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
        Ok(decrypted)
    }
}

/// Extract AES-GCM master key from Edge user_dec after double DPAPI.
///
/// Edge uses a simple structure:
///   4 bytes: header_len (usually 32)
///   header_len bytes: header (ASCII text like Edge install path)
///   4 bytes: content_len (usually 32)
///   content_len bytes: raw AES-GCM master key
fn extract_edge_master_key(user_dec: &[u8]) -> Result<Vec<u8>> {
    if user_dec.len() < 8 {
        anyhow::bail!("Edge key blob too short ({} bytes)", user_dec.len());
    }
    let header_len = u32::from_le_bytes(user_dec[..4].try_into()?) as usize;
    let content_len_offset = 4 + header_len;
    if content_len_offset + 4 > user_dec.len() {
        anyhow::bail!("Edge key blob: invalid header_len {}", header_len);
    }
    let content_len = u32::from_le_bytes(
        user_dec[content_len_offset..content_len_offset + 4].try_into()?
    ) as usize;
    let master_key_offset = content_len_offset + 4;
    if master_key_offset + content_len > user_dec.len() {
        anyhow::bail!("Edge key blob: content extends beyond data");
    }
    let master_key = user_dec[master_key_offset..master_key_offset + content_len].to_vec();
    Ok(master_key)
}

/// Decrypt data using CNG NCrypt API (for flag=3 key blobs).
fn decrypt_with_cng(input_data: &[u8], key_name: &str) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::HANDLE;

    let ncrypt_name: Vec<u16> = "ncrypt.dll\0".encode_utf16().collect();
    let ncrypt =
        unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(ncrypt_name.as_ptr()) };
    if ncrypt.is_null() {
        anyhow::bail!("Failed to load ncrypt.dll");
    }

    type NtCryptOpenStorageProvider = unsafe extern "system" fn(
        provider: *mut HANDLE,
        name: *const u16,
        flags: u32,
    ) -> u32;
    type NtCryptOpenKey = unsafe extern "system" fn(
        provider: HANDLE,
        key: *mut HANDLE,
        name: *const u16,
        legacy_spec: u32,
        flags: u32,
    ) -> u32;
    type NtCryptDecrypt = unsafe extern "system" fn(
        key: HANDLE,
        input: *const u8,
        input_len: u32,
        padding_info: *const u8,
        output: *mut u8,
        output_len: u32,
        result_len: *mut u32,
        flags: u32,
    ) -> u32;
    type NtCryptFreeObject = unsafe extern "system" fn(object: HANDLE) -> u32;

    unsafe {
        let open_provider: NtCryptOpenStorageProvider = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(ncrypt, b"NCryptOpenStorageProvider\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("NCryptOpenStorageProvider not found"))?,
        );
        let open_key: NtCryptOpenKey = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(ncrypt, b"NCryptOpenKey\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("NCryptOpenKey not found"))?,
        );
        let decrypt: NtCryptDecrypt = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(ncrypt, b"NCryptDecrypt\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("NCryptDecrypt not found"))?,
        );
        let free_object: NtCryptFreeObject = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(ncrypt, b"NCryptFreeObject\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("NCryptFreeObject not found"))?,
        );

        let mut h_provider: HANDLE = std::ptr::null_mut();
        let provider_name: Vec<u16> = "Microsoft Software Key Storage Provider\0".encode_utf16().collect();
        let status = open_provider(&mut h_provider, provider_name.as_ptr(), 0);
        if status != 0 {
            anyhow::bail!("NCryptOpenStorageProvider failed: 0x{:08X}", status);
        }

        let key_name_wide: Vec<u16> = key_name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut h_key: HANDLE = std::ptr::null_mut();
        let status = open_key(h_provider, &mut h_key, key_name_wide.as_ptr(), 0, 0);
        if status != 0 {
            free_object(h_provider);
            anyhow::bail!("NCryptOpenKey('{}') failed: 0x{:08X}", key_name, status);
        }

        let mut output_len: u32 = 0;
        const NCRYPT_SILENT_FLAG: u32 = 0x00000040;
        let status = decrypt(
            h_key,
            input_data.as_ptr(),
            input_data.len() as u32,
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
            &mut output_len,
            NCRYPT_SILENT_FLAG,
        );
        if status != 0 {
            free_object(h_key);
            free_object(h_provider);
            anyhow::bail!("NCryptDecrypt (size query) failed: 0x{:08X}", status);
        }

        let mut output = vec![0u8; output_len as usize];
        let status = decrypt(
            h_key,
            input_data.as_ptr(),
            input_data.len() as u32,
            std::ptr::null(),
            output.as_mut_ptr(),
            output_len,
            &mut output_len,
            NCRYPT_SILENT_FLAG,
        );
        free_object(h_key);
        free_object(h_provider);

        if status != 0 {
            anyhow::bail!("NCryptDecrypt failed: 0x{:08X}", status);
        }

        output.truncate(output_len as usize);
        Ok(output)
    }
}

/// XOR two byte slices.
fn byte_xor(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| x ^ y).collect()
}

// ---------------------------------------------------------------------------
// Key blob parsing
// ---------------------------------------------------------------------------

/// Parse Chrome-style app-bound key blob.
fn parse_key_blob(user_dec: &[u8]) -> Result<KeyBlob> {
    let mut cursor = Cursor::new(user_dec);
    use std::io::Read;

    let mut hl_buf = [0u8; 4];
    cursor.read_exact(&mut hl_buf)?;
    let header_len = u32::from_le_bytes(hl_buf) as usize;

    cursor.set_position(cursor.position() + header_len as u64);

    let mut cl_buf = [0u8; 4];
    cursor.read_exact(&mut cl_buf)?;
    let _content_len = u32::from_le_bytes(cl_buf);

    let mut flag_byte = [0u8; 1];
    cursor.read_exact(&mut flag_byte)?;
    let flag = flag_byte[0];

    let mut blob = KeyBlob {
        flag,
        iv: None,
        ciphertext: None,
        tag: None,
        encrypted_aes_key: None,
    };

    match flag {
        1 | 2 => {
            let mut iv = [0u8; 12];
            cursor.read_exact(&mut iv)?;
            let mut ct = [0u8; 32];
            cursor.read_exact(&mut ct)?;
            let mut tag = [0u8; 16];
            cursor.read_exact(&mut tag)?;
            blob.iv = Some(iv.to_vec());
            blob.ciphertext = Some(ct.to_vec());
            blob.tag = Some(tag.to_vec());
        }
        3 => {
            let mut eak = [0u8; 32];
            cursor.read_exact(&mut eak)?;
            let mut iv = [0u8; 12];
            cursor.read_exact(&mut iv)?;
            let mut ct = [0u8; 32];
            cursor.read_exact(&mut ct)?;
            let mut tag = [0u8; 16];
            cursor.read_exact(&mut tag)?;
            blob.encrypted_aes_key = Some(eak.to_vec());
            blob.iv = Some(iv.to_vec());
            blob.ciphertext = Some(ct.to_vec());
            blob.tag = Some(tag.to_vec());
        }
        _ => anyhow::bail!("Unsupported key blob flag: {}", flag),
    }

    Ok(blob)
}

/// Parse Edge-style app-bound key blob (has "ImportPvt1" prefix).
fn parse_key_blob_edge(user_dec: &[u8]) -> Result<KeyBlob> {
    let mut cursor = Cursor::new(user_dec);
    use std::io::Read;

    let mut hl_buf = [0u8; 4];
    cursor.read_exact(&mut hl_buf)?;
    let header_len = u32::from_le_bytes(hl_buf) as usize;

    cursor.set_position(cursor.position() + header_len as u64);

    let mut cl_buf = [0u8; 4];
    cursor.read_exact(&mut cl_buf)?;
    let _content_len = u32::from_le_bytes(cl_buf);

    let mut prefix = [0u8; 10];
    cursor.read_exact(&mut prefix)?;
    if &prefix != b"ImportPvt1" {
        anyhow::bail!(
            "Expected 'ImportPvt1' prefix in Edge key blob, got {:?}",
            prefix
        );
    }

    let mut flag_byte = [0u8; 1];
    cursor.read_exact(&mut flag_byte)?;
    let flag = flag_byte[0];

    if flag != 3 {
        anyhow::bail!("Unsupported Edge key blob flag: {}", flag);
    }

    let mut eak = [0u8; 32];
    cursor.read_exact(&mut eak)?;
    let mut iv = [0u8; 12];
    cursor.read_exact(&mut iv)?;
    let mut ct = [0u8; 32];
    cursor.read_exact(&mut ct)?;
    let mut tag = [0u8; 16];
    cursor.read_exact(&mut tag)?;

    Ok(KeyBlob {
        flag,
        iv: Some(iv.to_vec()),
        ciphertext: Some(ct.to_vec()),
        tag: Some(tag.to_vec()),
        encrypted_aes_key: Some(eak.to_vec()),
    })
}

pub struct KeyBlob {
    pub flag: u8,
    pub iv: Option<Vec<u8>>,
    pub ciphertext: Option<Vec<u8>>,
    pub tag: Option<Vec<u8>>,
    pub encrypted_aes_key: Option<Vec<u8>>,
}

/// Derive AES-GCM master key from parsed key blob.
fn derive_master_key(blob: &KeyBlob, key_name: &str) -> Result<Vec<u8>> {
    match blob.flag {
        1 => {
            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&CHROME_FIXED_AES_KEY));
            let iv = Nonce::from_slice(blob.iv.as_ref().unwrap());
            let mut ct_tag = blob.ciphertext.as_ref().unwrap().clone();
            ct_tag.extend_from_slice(blob.tag.as_ref().unwrap());
            let plaintext = cipher
                .decrypt(iv, ct_tag.as_slice())
                .map_err(|e| anyhow::anyhow!("AES-GCM decryption failed for flag=1: {:?}", e))?;
            Ok(plaintext)
        }
        2 => {
            let cipher = ChaCha20Poly1305::new(Key::<ChaCha20Poly1305>::from_slice(
                &CHROME_FIXED_CHACHA_KEY,
            ));
            let nonce = chacha20poly1305::Nonce::from_slice(blob.iv.as_ref().unwrap());
            let mut ct_tag = blob.ciphertext.as_ref().unwrap().clone();
            ct_tag.extend_from_slice(blob.tag.as_ref().unwrap());
            let plaintext = cipher
                .decrypt(nonce, ct_tag.as_slice())
                .map_err(|e| anyhow::anyhow!("ChaCha20Poly1305 decryption failed for flag=2: {:?}", e))?;
            Ok(plaintext)
        }
        3 => {
            let encrypted_aes_key = blob.encrypted_aes_key.as_ref()
                .ok_or_else(|| anyhow::anyhow!("flag=3 requires encrypted_aes_key"))?;
            let dec_key = decrypt_with_cng(encrypted_aes_key, key_name)?;
            let aes_key = byte_xor(&dec_key, &ABE_XOR_KEY);

            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
            let iv = Nonce::from_slice(blob.iv.as_ref().unwrap());
            let mut ct_tag = blob.ciphertext.as_ref().unwrap().clone();
            ct_tag.extend_from_slice(blob.tag.as_ref().unwrap());
            let plaintext = cipher
                .decrypt(iv, ct_tag.as_slice())
                .map_err(|e| anyhow::anyhow!("AES-GCM decryption failed for flag=3: {:?}", e))?;
            Ok(plaintext)
        }
        _ => anyhow::bail!("Unsupported flag: {}", blob.flag),
    }
}

// ---------------------------------------------------------------------------
// SQLite cookie reading
// ---------------------------------------------------------------------------

/// Copy a file, using Windows Restart Manager to release file locks if needed.
fn copy_unlocked(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    if std::fs::copy(src, dst).is_ok() {
        return Ok(());
    }

    log::debug!("Cookie DB is locked, unlocking via Rstrtmgr...");

    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[allow(non_snake_case)]
    type RmStartSessionFn = unsafe extern "system" fn(pSession: *mut u32, dwSessionFlags: u32, strSessionKey: *mut u16) -> u32;
    #[allow(non_snake_case)]
    type RmEndSessionFn = unsafe extern "system" fn(dwSessionHandle: u32) -> u32;
    #[allow(non_snake_case)]
    type RmRegisterResourcesFn = unsafe extern "system" fn(
        dwSessionHandle: u32,
        nFiles: u32,
        rgsFilenames: *mut *mut u16,
        nApplications: u32,
        rgApplications: *mut u8,
        nServices: u32,
        rgsServiceNames: *mut u16,
    ) -> u32;
    #[allow(non_snake_case)]
    type RmShutdownFn = unsafe extern "system" fn(dwSessionHandle: u32, lActionFlags: u32, fnStatus: *mut u8) -> u32;

    let rstrtmgr_name: Vec<u16> = "Rstrtmgr.dll\0".encode_utf16().collect();
    let rstrtmgr =
        unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(rstrtmgr_name.as_ptr()) };
    if rstrtmgr.is_null() {
        anyhow::bail!("Failed to load Rstrtmgr.dll");
    }

    unsafe {
        let rm_start: RmStartSessionFn = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(rstrtmgr, b"RmStartSession\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("RmStartSession not found"))?,
        );
        let rm_end: RmEndSessionFn = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(rstrtmgr, b"RmEndSession\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("RmEndSession not found"))?,
        );
        let rm_register: RmRegisterResourcesFn = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(rstrtmgr, b"RmRegisterResources\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("RmRegisterResources not found"))?,
        );
        let rm_shutdown: RmShutdownFn = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(rstrtmgr, b"RmShutdown\0".as_ptr())
                .ok_or_else(|| anyhow::anyhow!("RmShutdown not found"))?,
        );

        let mut session: u32 = 0;
        let mut session_key: [u16; 256] = [0; 256];
        let rc = rm_start(&mut session, 0, session_key.as_mut_ptr());
        if rc != 0 {
            anyhow::bail!("RmStartSession failed: {rc}");
        }

        let src_wide: Vec<u16> = OsStr::new(src).encode_wide().chain(std::iter::once(0)).collect();
        let mut src_ptr = src_wide.as_ptr() as *mut u16;

        let rc = rm_register(session, 1, &mut src_ptr, 0, std::ptr::null_mut(), 0, std::ptr::null_mut());
        if rc != 0 {
            rm_end(session);
            anyhow::bail!("RmRegisterResources failed: {rc}");
        }

        let rc = rm_shutdown(session, 1, std::ptr::null_mut());
        if rc != 0 {
            rm_end(session);
            anyhow::bail!("RmShutdown failed: {rc}");
        }

        rm_end(session);
    }

    std::fs::copy(src, dst)
        .with_context(|| "Failed to copy cookie database even after Rstrtmgr unlock")?;

    Ok(())
}

fn read_cookies(db_path: &PathBuf) -> Result<Vec<CookieRow>> {
    if !db_path.exists() {
        anyhow::bail!("Cookie database not found: {}", db_path.display());
    }

    let tmp_dir = tempfile::tempdir()?;
    let tmp_db = tmp_dir.path().join("Cookies");

    copy_unlocked(db_path, &tmp_db)?;

    let conn = Connection::open_with_flags(
        &tmp_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;

    let mut stmt = conn.prepare(
        "SELECT host_key, name, CAST(encrypted_value AS BLOB), is_secure, is_httponly, expires_utc FROM cookies",
    )?;

    let cookies: Vec<CookieRow> = stmt
        .query_map([], |row| {
            Ok(CookieRow {
                host_key: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                encrypted_value: row.get::<_, Vec<u8>>(2)?,
                is_secure: row.get::<_, bool>(3)?,
                is_httponly: row.get::<_, bool>(4)?,
                expires_utc: row.get::<_, Option<i64>>(5)?.unwrap_or(0).to_string(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(cookies)
}

struct CookieRow {
    host_key: String,
    name: String,
    encrypted_value: Vec<u8>,
    is_secure: bool,
    is_httponly: bool,
    expires_utc: String,
}

// ---------------------------------------------------------------------------
// Cookie decryption
// ---------------------------------------------------------------------------

/// Decrypt all cookies using the master key.
fn decrypt_all_cookies(
    master_key: &[u8],
    cookies: &[CookieRow],
) -> Result<Vec<(String, String, String, bool, bool, String)>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));

    let mut result = Vec::new();
    let mut failed = 0;

    for row in cookies {
        let ev = &row.encrypted_value;

        if ev.len() > 3 && &ev[..3] == b"v20" {
            if ev.len() < 3 + 12 + 16 {
                failed += 1;
                continue;
            }
            let iv = &ev[3..15];
            let ct = &ev[15..ev.len() - 16];
            let tag = &ev[ev.len() - 16..];

            let mut ct_tag = ct.to_vec();
            ct_tag.extend_from_slice(tag);

            match cipher.decrypt(Nonce::from_slice(iv), ct_tag.as_slice()) {
                Ok(plaintext) => {
                    if plaintext.len() > 32 {
                        if let Ok(value) = String::from_utf8(plaintext[32..].to_vec()) {
                            result.push((
                                row.host_key.clone(),
                                row.name.clone(),
                                value,
                                row.is_secure,
                                row.is_httponly,
                                row.expires_utc.clone(),
                            ));
                        } else {
                            failed += 1;
                        }
                    } else {
                        failed += 1;
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        } else if !ev.is_empty() && ev[0] != 1
            && let Ok(value) = String::from_utf8(ev.clone()) {
                result.push((
                    row.host_key.clone(),
                    row.name.clone(),
                    value,
                    row.is_secure,
                    row.is_httponly,
                    row.expires_utc.clone(),
                ));
            }
    }

    log::info!(
        "Decrypted {} cookies ({} failed)",
        result.len(),
        failed
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Public test helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Test DPAPI decryption step.
pub fn test_dpapi(enc_key: &[u8]) -> Result<(Vec<u8>, &'static str)> {
    let (user_dec, h_token) = double_dpapi_decrypt(enc_key)?;
    unsafe { windows_sys::Win32::Foundation::CloseHandle(h_token) };
    Ok((user_dec, "lsass impersonation"))
}

#[allow(dead_code)]
/// Test key blob parsing.
pub fn test_parse_key_blob(user_dec: &[u8]) -> Result<(KeyBlob, u8)> {
    let blob = parse_key_blob(user_dec)
        .or_else(|_| parse_key_blob_edge(user_dec))?;
    let flag = blob.flag;
    Ok((blob, flag))
}

#[allow(dead_code)]
/// Test master key derivation.
pub fn test_derive_master_key(blob: &KeyBlob, key_name: &str) -> Result<Vec<u8>> {
    derive_master_key(blob, key_name)
}

#[allow(dead_code)]
/// Test cookie reading.
pub fn test_read_cookies(db_path: &PathBuf) -> Result<Vec<PublicCookieRow>> {
    let cookies = read_cookies(db_path)?;
    Ok(cookies.into_iter().map(|c| PublicCookieRow {
        encrypted_value: c.encrypted_value,
    }).collect())
}

#[allow(dead_code)]
/// Test cookie decryption.
pub fn test_decrypt_cookies(master_key: &[u8], cookies: &[PublicCookieRow]) -> (usize, usize, Vec<(String, String, String)>) {
    let mut result = Vec::new();
    let mut failed = 0;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));

    for row in cookies {
        let ev = &row.encrypted_value;
        if ev.len() > 3 && &ev[..3] == b"v20" {
            if ev.len() < 3 + 12 + 16 {
                failed += 1;
                continue;
            }
            let iv = &ev[3..15];
            let ct = &ev[15..ev.len() - 16];
            let tag = &ev[ev.len() - 16..];
            let mut ct_tag = ct.to_vec();
            ct_tag.extend_from_slice(tag);
            match cipher.decrypt(Nonce::from_slice(iv), ct_tag.as_slice()) {
                Ok(plaintext) => {
                    if plaintext.len() > 32 {
                        if let Ok(value) = String::from_utf8(plaintext[32..].to_vec()) {
                            result.push(("test".to_string(), "test".to_string(), value));
                        } else { failed += 1; }
                    } else { failed += 1; }
                }
                Err(_) => { failed += 1; }
            }
        }
    }
    (result.len(), failed, result)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PublicCookieRow {
    pub encrypted_value: Vec<u8>,
}
