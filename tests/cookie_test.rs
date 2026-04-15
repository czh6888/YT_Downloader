use std::path::PathBuf;

#[test]
fn test_firefox_cookie_extraction() {
    println!("--- Firefox ---");
    let cookie_file = PathBuf::from("test_firefox_cookies.txt");
    let result = yt_downloader::downloader::extract_browser_cookies("Firefox", &cookie_file);
    println!("Cookie file: {}", result.cookie_file);
    println!("Use cookie file: {}", result.use_cookie_file);
    println!("Message: {}", result.message);

    if result.use_cookie_file {
        assert!(!result.cookie_file.is_empty());
        if let Ok(content) = std::fs::read_to_string(&result.cookie_file) {
            let lines: Vec<&str> = content.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
            println!("Cookies written: {}", lines.len());
            if lines.len() > 0 {
                println!("Sample: {}", lines[0]);
            }
        }
    } else {
        println!("Firefox cookie extraction did not produce a file (may not have Firefox installed or logged in)");
    }
}

#[test]
fn test_chrome_cookie_extraction() {
    println!("--- Chrome ---");
    let cookie_file = PathBuf::from("test_chrome_cookies.txt");
    let result = yt_downloader::downloader::extract_browser_cookies("Chrome", &cookie_file);
    println!("Cookie file: {}", result.cookie_file);
    println!("Use cookie file: {}", result.use_cookie_file);
    println!("Message: {}", result.message);

    if result.use_cookie_file {
        assert!(!result.cookie_file.is_empty());
        if let Ok(content) = std::fs::read_to_string(&result.cookie_file) {
            let lines: Vec<&str> = content.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
            println!("Cookies written: {}", lines.len());
            if lines.len() > 0 {
                println!("Sample: {}", lines[0]);
            }
        }
    } else {
        println!("Chrome cookie extraction fell back to --cookies-from-browser: {:?}", result.browser_native);
    }
}

#[test]
fn test_edge_cookie_extraction() {
    println!("--- Edge ---");
    let cookie_file = PathBuf::from("test_edge_cookies.txt");
    let result = yt_downloader::downloader::extract_browser_cookies("Edge", &cookie_file);
    println!("Cookie file: {}", result.cookie_file);
    println!("Use cookie file: {}", result.use_cookie_file);
    println!("Message: {}", result.message);

    if result.use_cookie_file {
        assert!(!result.cookie_file.is_empty());
        if let Ok(content) = std::fs::read_to_string(&result.cookie_file) {
            let lines: Vec<&str> = content.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
            println!("Cookies written: {}", lines.len());
            if lines.len() > 0 {
                println!("Sample: {}", lines[0]);
            }
        }
    } else {
        println!("Edge cookie extraction fell back to --cookies-from-browser: {:?}", result.browser_native);
    }
}

#[test]
fn test_yt_dlp_find() {
    println!("--- yt-dlp ---");
    let path = yt_downloader::downloader::find_yt_dlp();
    match path {
        Some(ref p) => println!("Found yt-dlp at: {:?}", p),
        None => println!("yt-dlp not found!"),
    }
    assert!(path.is_some(), "yt-dlp should be installed");
}

#[test]
fn test_cookie_args() {
    println!("--- cookie_args ---");
    let args = yt_downloader::downloader::cookie_args(None, None);
    println!("Empty cookie args: {:?}", args);
    assert!(args.is_empty());

    // With browser_native (no file check)
    let args = yt_downloader::downloader::cookie_args(None, Some("firefox"));
    assert!(args.contains(&"--cookies-from-browser".to_string()));
    assert!(args.contains(&"firefox".to_string()));

    // With cookie_file (must exist)
    std::fs::write("test_cookie_file.txt", "# test").unwrap();
    let args = yt_downloader::downloader::cookie_args(Some("test_cookie_file.txt"), None);
    assert!(args.contains(&"--cookies".to_string()));
    assert!(args.contains(&"test_cookie_file.txt".to_string()));
    let _ = std::fs::remove_file("test_cookie_file.txt");
}
