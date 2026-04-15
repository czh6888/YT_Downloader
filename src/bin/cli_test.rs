/// CLI download test: yt-dlp with Chrome/Edge v20 cookie extraction
///
/// Usage: cargo run --release --bin cli_test -- chrome "https://youtube.com/watch?v=VIDEO_ID"
///        cargo run --release --bin cli_test -- edge "https://youtube.com/watch?v=VIDEO_ID"

use std::path::PathBuf;
use yt_downloader::downloader::cookies;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cli_test <chrome|edge|firefox> <url>");
        std::process::exit(1);
    }

    let browser = &args[1];
    let url = &args[2];

    let browser = match browser.to_lowercase().as_str() {
        "chrome" => "Chrome",
        "edge" => "Edge",
        "firefox" => "Firefox",
        other => {
            eprintln!("Unsupported browser: {other}");
            std::process::exit(1);
        }
    };

    println!("=== CLI Download Test ===");
    println!("Browser: {browser}");
    println!("URL: {url}");
    println!();

    // Step 1: Extract cookies
    let cookie_file = PathBuf::from("C:\\Users\\CZH\\AppData\\Local\\Temp\\yt_cookies_cli.txt");
    println!("[1/3] Extracting cookies from {browser}...");

    let cookie_result = cookies::extract_cookies(browser, &cookie_file);
    println!("  Message: {}", cookie_result.message);
    println!("  Use cookie file: {}", cookie_result.use_cookie_file);
    println!("  Browser native: {:?}", cookie_result.browser_native);

    if !cookie_result.use_cookie_file && cookie_result.browser_native.is_none() {
        eprintln!("ERROR: Cookie extraction failed: {}", cookie_result.message);
        std::process::exit(1);
    }

    // Step 2: Build yt-dlp command
    println!("\n[2/3] Building yt-dlp command...");

    let mut cmd = std::process::Command::new("yt-dlp");
    cmd.arg(url);

    if cookie_result.use_cookie_file {
        cmd.arg("--cookies").arg(&cookie_file);
        println!("  Using cookie file: {}", cookie_file.display());
    } else if let Some(browser_name) = &cookie_result.browser_native {
        cmd.arg("--cookies-from-browser").arg(browser_name);
        println!("  Using --cookies-from-browser={browser_name}");
    }

    cmd.arg("--no-playlist");
    cmd.arg("--dump-json");

    // Step 3: Run yt-dlp
    println!("\n[3/3] Running yt-dlp...");
    let output = cmd.output().expect("Failed to run yt-dlp");

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
            println!("\nTitle: {title}");
        }
        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
            println!("ID: {id}");
        }
    }

    if output.status.success() {
        println!("\nSUCCESS: yt-dlp fetched video info with {browser} cookies");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("\nyt-dlp failed (exit {}):\n{}", output.status, stderr.lines().take(20).collect::<Vec<_>>().join("\n"));
        std::process::exit(1);
    }
}
