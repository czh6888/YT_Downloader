mod chromelevator;
#[allow(dead_code)] // kept for future reference
pub mod chromium;
mod firefox;
mod netscape;

use std::path::PathBuf;

/// Result of cookie extraction.
#[derive(Debug, Clone)]
pub struct CookieResult {
    /// Path to the generated Netscape cookie file.
    pub cookie_file: String,
    /// Whether we used a cookie file (true) or --cookies-from-browser (false).
    pub use_cookie_file: bool,
    /// Browser name for --cookies-from-browser (if not using cookie file).
    pub browser_native: Option<String>,
    /// Human-readable status message.
    pub message: String,
}

/// Unified cookie extraction entry point.
///
/// Priority per browser:
/// - Firefox: direct sqlite read (no encryption)
/// - Chrome: Rust direct DPAPI → Python fallback (lsass impersonation, works when admin)
/// - Edge: Rust direct DPAPI → Python fallback (lsass impersonation, works when admin)
pub fn extract_cookies(browser: &str, cookie_file: &PathBuf) -> CookieResult {
    match browser {
        "Firefox" => extract_firefox(cookie_file),
        "Chrome" => extract_chromium(cookie_file, &chromium::BrowserPaths::chrome(), browser),
        "Edge" => extract_chromium(cookie_file, &chromium::BrowserPaths::edge(), browser),
        _ => CookieResult {
            cookie_file: String::new(),
            use_cookie_file: false,
            browser_native: None,
            message: format!("Unsupported browser: {browser}"),
        },
    }
}

fn extract_firefox(cookie_file: &PathBuf) -> CookieResult {
    match firefox::extract_cookies(cookie_file) {
        Ok(()) => CookieResult {
            cookie_file: cookie_file.to_string_lossy().to_string(),
            use_cookie_file: true,
            browser_native: None,
            message: "Extracted cookies from Firefox".to_string(),
        },
        Err(e) => CookieResult {
            cookie_file: String::new(),
            use_cookie_file: false,
            browser_native: None,
            message: format!("Firefox extraction failed: {e}"),
        },
    }
}

fn extract_chromium(
    cookie_file: &PathBuf,
    paths: &chromium::BrowserPaths,
    browser_name: &str,
) -> CookieResult {
    // Step 1: Try Rust direct DPAPI (no lsass, no executable memory allocation)
    log::info!("Trying Rust DPAPI cookie extraction for {browser_name}");
    match chromium::extract_cookies(paths, cookie_file) {
        Ok(()) => {
            return CookieResult {
                cookie_file: cookie_file.to_string_lossy().to_string(),
                use_cookie_file: true,
                browser_native: None,
                message: format!("Extracted cookies from {browser_name} (DPAPI)"),
            };
        }
        Err(e) => {
            log::debug!("{browser_name} Rust DPAPI failed: {e}, trying Python fallback...");
        }
    }

    // Step 2: Python fallback with lsass impersonation (works when running as admin)
    log::info!("Trying Python fallback for {browser_name}");
    match chromium::extract_via_python(paths, cookie_file) {
        Ok(()) => CookieResult {
            cookie_file: cookie_file.to_string_lossy().to_string(),
            use_cookie_file: true,
            browser_native: None,
            message: format!("Extracted cookies from {browser_name} (Python)"),
        },
        Err(e) => {
            log::debug!("{browser_name} Python fallback failed: {e}");
            // Step 3: Final fallback - yt-dlp --cookies-from-browser
            CookieResult {
                cookie_file: String::new(),
                use_cookie_file: false,
                browser_native: Some(browser_name.to_lowercase()),
                message: format!(
                    "Falling back to yt-dlp --cookies-from-browser: {e}"
                ),
            }
        }
    }
}
