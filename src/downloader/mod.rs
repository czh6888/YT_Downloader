pub mod cookies;
mod formats;
mod post_process;
mod progress;
mod yt_dlp;

pub use cookies::CookieResult;
pub use formats::{FormatInfo, parse_formats};
pub use post_process::detect_ffmpeg;
pub use progress::parse_progress;
pub use yt_dlp::{
    DownloadResult, build_format_string_from_id, cookie_args, fetch_info, fetch_info_with_cookies, find_yt_dlp,
};

/// Extract cookies from the specified browser.
/// Returns a CookieResult with the cookie file path or fallback browser name.
pub fn extract_browser_cookies(browser: &str, cookie_file: &std::path::PathBuf) -> CookieResult {
    cookies::extract_cookies(browser, cookie_file)
}
