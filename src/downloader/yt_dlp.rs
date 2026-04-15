use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Result of a download operation.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub success: bool,
    pub log_lines: Vec<String>,
    pub file_path: Option<String>,
}

/// Find the yt-dlp executable.
/// Tries: `yt-dlp` / `yt-dlp.exe` on PATH, then `python -m yt_dlp`.
pub fn find_yt_dlp() -> Option<Vec<String>> {
    if which("yt-dlp").is_some() {
        return Some(vec!["yt-dlp".to_string()]);
    }
    if which("yt-dlp.exe").is_some() {
        return Some(vec!["yt-dlp.exe".to_string()]);
    }
    if std::process::Command::new("python")
        .args(["-m", "yt_dlp", "--version"])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(vec![
            "python".to_string(),
            "-m".to_string(),
            "yt_dlp".to_string(),
        ]);
    }
    if std::process::Command::new("python3")
        .args(["-m", "yt_dlp", "--version"])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(vec![
            "python3".to_string(),
            "-m".to_string(),
            "yt_dlp".to_string(),
        ]);
    }
    None
}

/// Fetch video info from yt-dlp as JSON.
pub async fn fetch_info(url: &str, cookie_args: &[String]) -> Result<Value> {
    let yt_dlp = find_yt_dlp()
        .ok_or_else(|| anyhow!("yt-dlp not found. Install with: pip install yt-dlp"))?;

    let mut cmd = Command::new(&yt_dlp[0]);
    cmd.args(&yt_dlp[1..])
        .arg("--no-warnings")
        .arg("--dump-json")
        .arg("--no-download")
        .args(cookie_args)
        .arg(url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = cmd.output().await.context("Failed to run yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("yt-dlp failed:\n{}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            return Ok(value);
        }
    }

    Err(anyhow!("No valid JSON output from yt-dlp"))
}

/// Download a video using yt-dlp. Returns log lines and final status.
pub async fn download(
    url: &str,
    cookie_args: &[String],
    format: &str,
    save_dir: &str,
    audio_only: bool,
) -> DownloadResult {
    let yt_dlp = match find_yt_dlp() {
        Some(cmd) => cmd,
        None => {
            return DownloadResult {
                success: false,
                log_lines: vec!["Error: yt-dlp not found".to_string()],
                file_path: None,
            };
        }
    };

    let output_template = format!("{save_dir}/%(title)s [%(id)s].%(ext)s");

    let mut cmd = Command::new(&yt_dlp[0]);
    cmd.args(&yt_dlp[1..])
        .arg("--newline")
        .arg("--progress")
        .arg("-o")
        .arg(&output_template)
        .arg("--merge-output-format")
        .arg("mp4");

    if audio_only {
        cmd.arg("-x").arg("--audio-format").arg(format);
    } else {
        cmd.arg("-f").arg(format);
    }

    cmd.args(cookie_args).arg(url);

    let mut log_lines = Vec::new();
    log_lines.push(format!("Command: {} ...", yt_dlp[0]));
    log_lines.push("-".repeat(50));

    let mut file_path = None;

    match cmd.stdout(std::process::Stdio::piped()).spawn() {
        Ok(mut child) => {
            let stdout = child.stdout.take().unwrap();
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim_end().to_owned();
                if !trimmed.is_empty() {
                    log_lines.push(trimmed);
                }
            }

            let status = child.wait().await;
            let success = status.as_ref().is_ok_and(|s| s.success());

            if success {
                log_lines.push("-".repeat(50));
                log_lines.push("Download complete!".to_string());
                file_path = extract_file_path(&log_lines, save_dir);
            } else {
                let code = status.as_ref().ok().and_then(|s| s.code());
                log_lines.push(format!("Download failed (exit code {code:?})"));
            }

            DownloadResult {
                success,
                log_lines,
                file_path,
            }
        }
        Err(e) => DownloadResult {
            success: false,
            log_lines: vec![format!("Failed to start download: {e}")],
            file_path: None,
        },
    }
}

/// Build cookie arguments for yt-dlp from a cookie file or browser name.
pub fn cookie_args(cookie_file: Option<&str>, browser_native: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(browser) = browser_native {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    } else if let Some(file) = cookie_file
        && std::path::Path::new(file).exists() {
            args.push("--cookies".to_string());
            args.push(file.to_string());
        }
    args
}

/// Build format string for yt-dlp -f flag.
pub fn build_format_string(resolution: &str, best_format_id: Option<&str>) -> String {
    if resolution == "best" {
        "bestvideo+bestaudio/best".to_string()
    } else if let Some(fmt_id) = best_format_id {
        format!("{fmt_id}+bestaudio/best[height<={resolution}]")
    } else {
        format!(
            "bestvideo[height<={resolution}]+bestaudio/best[height<={resolution}]"
        )
    }
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .filter_map(|dir| {
                let full = dir.join(name);
                if full.is_file() {
                    Some(full)
                } else {
                    let with_ext = full.with_extension("exe");
                    if with_ext.is_file() {
                        Some(with_ext)
                    } else {
                        None
                    }
                }
            })
            .next()
    })
}

/// Try to extract the downloaded file path from yt-dlp log output.
fn extract_file_path(log_lines: &[String], save_dir: &str) -> Option<String> {
    for line in log_lines.iter().rev() {
        if line.contains("[download]") && line.contains("has already been downloaded")
            && let Some(start) = line.find('\'')
                && let Some(end) = line[start + 1..].find('\'') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
    }
    Some(save_dir.to_string())
}
