use yt_downloader::app::App;
use yt_downloader::downloader;
use iced::application;

/// Load font bytes into cosmic-text's font database.
fn load_fonts() -> Vec<(&'static [u8], &'static str)> {
    let mut fonts = Vec::new();

    // Primary CJK font: Microsoft YaHei
    let cjk_candidates = [
        concat!(env!("CARGO_MANIFEST_DIR"), "\\assets\\msyh_regular.ttf"),
        "C:\\Windows\\Fonts\\msyh.ttc",
    ];
    for path in &cjk_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let b: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            fonts.push((b, "Microsoft YaHei"));
            break;
        }
    }

    // Emoji/symbol font: Segoe UI Symbol (monochrome, cosmic-text compatible)
    let emoji_candidates = [
        "C:\\Windows\\Fonts\\seguisym.ttf",
    ];
    for path in &emoji_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let b: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            fonts.push((b, "Segoe UI Symbol"));
            break;
        }
    }

    fonts
}

/// Check if running with administrator privileges.
fn is_admin() -> bool {
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION,
    };
    use windows_sys::Win32::Security::TOKEN_QUERY;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut h_token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token) } == 0 {
        return false;
    }

    let mut elevation: TOKEN_ELEVATION = unsafe { std::mem::zeroed() };
    let mut size = 0u32;
    let result = unsafe {
        GetTokenInformation(
            h_token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
    };

    if result == 0 {
        return false;
    }

    elevation.TokenIsElevated != 0
}

/// Re-launch the current executable with administrator privileges via UAC prompt.
fn elevate_to_admin() -> ! {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe_path = std::env::current_exe().unwrap();
    let exe_wide: Vec<u16> = exe_path.to_string_lossy().encode_utf16().chain([0]).collect();
    let verb_wide: Vec<u16> = "runas".encode_utf16().chain([0]).collect();

    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb_wide.as_ptr(),
            exe_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }

    std::process::exit(0);
}

fn main() -> iced::Result {
    // Auto-elevate: if not admin, trigger UAC prompt automatically
    if !is_admin() {
        elevate_to_admin();
    }

    // Check yt-dlp before launching GUI
    if downloader::find_yt_dlp().is_none() {
        eprintln!("Error: yt-dlp not found.");
        eprintln!("Please install it via: pip install yt-dlp");
        eprintln!("Or: winget install yt-dlp");
        std::process::exit(1);
    }

    let mut settings = application("yt-downloader", App::update, App::view)
        .theme(|app: &App| app.theme.to_iced())
        .antialiasing(false)
        .subscription(App::subscription);

    // Load all fonts into cosmic-text's font database
    let loaded = load_fonts();
    for (font_bytes, _name) in &loaded {
        settings = settings.font(*font_bytes);
    }
    // Set Microsoft YaHei as default font for CJK support
    settings = settings.default_font(iced::Font::with_name("Microsoft YaHei"));

    settings.run_with(App::new)
}
