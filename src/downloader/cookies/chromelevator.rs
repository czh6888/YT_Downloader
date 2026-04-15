use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;

use super::netscape;

/// Find chromelevator executable in known locations.
#[allow(dead_code)] // kept for future reference
fn find_chromelevator() -> Option<PathBuf> {
    let exe_name = if cfg!(target_arch = "aarch64") {
        "chromelevator_arm64.exe"
    } else {
        "chromelevator_x64.exe"
    };

    let mut candidates = Vec::new();

    // Relative to binary
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(parent) = current_exe.parent() {
            candidates.push(parent.join("tools").join(exe_name));
            candidates.push(parent.parent().map(|p| p.join("tools").join(exe_name)).unwrap_or_default());
        }

    // Relative to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("tools").join(exe_name));
    }

    for path in &candidates {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Extract Edge/Chrome cookies using chromelevator (process injection + COM hijack).
/// This is the fallback when pure DPAPI decryption fails.
/// Currently unused because chromelevator 0.20 only extracts tokens, not cookies.
#[allow(dead_code)]
pub fn extract_with_chromelevator(
    browser: &str,
    cookie_file: &PathBuf,
) -> Result<()> {
    let exe_path = find_chromelevator()
        .ok_or_else(|| anyhow::anyhow!("chromelevator executable not found"))?;

    // Map GUI browser name to chromelevator argument
    let chromelevator_name = match browser {
        "Chrome" => "chrome",
        "Edge" => "edge",
        _ => &browser.to_lowercase(),
    };

    let output_dir = tempfile::tempdir()?;
    let mut child = std::process::Command::new(&exe_path)
        .args(["-o", output_dir.path().to_str().unwrap(), chromelevator_name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start chromelevator")?;

    // Wait with 30-second timeout
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    anyhow::bail!("chromelevator failed: {}", stderr);
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    anyhow::bail!("chromelevator timed out after {timeout:?}");
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                anyhow::bail!("chromelevator wait error: {e}");
            }
        }
    }

    // Determine profile directory
    let profile_dir = match chromelevator_name {
        "chrome" => "Chrome",
        "edge" => "Edge",
        "brave" => "Brave",
        _ => "Chrome",
    };

    let cookie_json_path = output_dir
        .path()
        .join(profile_dir)
        .join("Default")
        .join("cookies.json");

    if !cookie_json_path.exists() {
        // Try alternate path
        let alt_path = output_dir.path().join("Default").join("cookies.json");
        if !alt_path.exists() {
            anyhow::bail!("Extracted cookies JSON not found in chromelevator output");
        }
        // Copy to expected location
        std::fs::copy(&alt_path, &cookie_json_path)?;
    }

    // Read and parse cookie JSON
    let raw = std::fs::read(&cookie_json_path)?;
    let text = String::from_utf8(raw)
        .unwrap_or_else(|_| {
            std::fs::read_to_string(&cookie_json_path)
                .unwrap_or_default()
        });
    let cookies: Vec<serde_json::Value> = serde_json::from_str(&text)?;

    // Convert to Netscape format
    let mut cookies_vec = Vec::new();
    for c in &cookies {
        let host = c.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if value.is_empty() {
            continue;
        }
        let secure = c.get("is_secure").and_then(|v| v.as_bool()).unwrap_or(false);
        let httponly = c.get("is_httponly").and_then(|v| v.as_bool()).unwrap_or(false);
        let expiry = c.get("expires").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
        cookies_vec.push((host, name, value, secure, httponly, expiry));
    }

    let content = netscape::to_netscape(&cookies_vec);
    std::fs::write(cookie_file, content)
        .with_context(|| format!("Failed to write cookie file to {}", cookie_file.display()))?;

    Ok(())
}
