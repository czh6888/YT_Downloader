use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use yt_downloader::config::Config;
use yt_downloader::downloader::{self, FormatInfo};
use crate::state::{AppState, DownloadTask};

/// Resolve cookie flags for yt-dlp from a browser name.
/// Uses the full extraction pipeline (DPAPI → Python → yt-dlp native).
fn resolve_cookie_flags(browser: &str) -> Vec<String> {
    if browser == "NoCookies" || browser.is_empty() {
        return Vec::new();
    }

    let config_dir = match dirs::config_dir() {
        Some(p) => p.to_string_lossy().to_string(),
        None => return Vec::new(),
    };
    let cookie_dir_path = format!("{config_dir}/yt-downloader/cookies");
    let cookie_file = std::path::PathBuf::from(format!(
        "{cookie_dir_path}/{}_cookies.txt",
        browser.to_lowercase()
    ));

    // Create cookie directory if it doesn't exist
    let _ = std::fs::create_dir_all(&cookie_dir_path);

    let result = downloader::extract_browser_cookies(browser, &cookie_file);
    if result.use_cookie_file {
        vec!["--cookies".to_string(), result.cookie_file]
    } else if let Some(browser_native) = result.browser_native {
        vec!["--cookies-from-browser".to_string(), browser_native]
    } else {
        // Extraction failed, proceed without cookies
        log::warn!("Cookie extraction failed for {browser}: {}", result.message);
        Vec::new()
    }
}

/// Progress event to send to frontend
#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub id: u64,
    pub progress: f64,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_history: Vec<(f64, f64)>,
}

/// Start a download with real-time progress streaming.
pub async fn run_download(
    app: AppHandle,
    task_id: u64,
    url: String,
    format_ids: Vec<String>,
    save_dir: String,
    audio_only: bool,
    audio_format: String,
    cookie_browser: String,
    subtitles_enabled: bool,
    subtitle_langs: String,
    config: Config,
    cancel_flag: Arc<AtomicBool>,
) -> (bool, Option<String>, Vec<String>) {
    let yt_dlp = match downloader::find_yt_dlp() {
        Some(cmd) => cmd,
        None => {
            let log = vec!["Error: yt-dlp not found".to_string()];
            return (false, None, log);
        }
    };

    let cookie_args = resolve_cookie_flags(&cookie_browser);

    // Build format string
    let format_str = if audio_only {
        format!("bestaudio[ext={}]/bestaudio", audio_format)
    } else if format_ids.len() == 1 && format_ids[0] == "best" {
        "bestvideo+bestaudio/best".to_string()
    } else if format_ids.len() == 1 {
        downloader::build_format_string_from_id(&format_ids[0])
    } else {
        // Multi-format: join with + so yt-dlp downloads and merges (e.g. "137+251")
        format_ids.join("+")
    };

    let output_template = if config.general.output_template.is_empty() {
        format!("{save_dir}/%(title)s [%(id)s].%(ext)s")
    } else {
        format!("{save_dir}/{}", config.general.output_template)
    };

    let mut cmd = Command::new(&yt_dlp[0]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.args(&yt_dlp[1..])
        .arg("--newline")
        .arg("--progress")
        .arg("--progress-template")
        .arg("{\"id\":\"%(id)s\",\"progress\":\"%(progress.percent)s\",\"total_bytes\":\"%(progress.total_bytes)s\",\"downloaded_bytes\":\"%(progress.downloaded_bytes)s\",\"speed\":\"%(progress.speed)s\",\"eta\":\"%(progress.eta)s\"}")
        .arg("-o")
        .arg(&output_template)
        .arg("--merge-output-format")
        .arg(&config.general.merge_output_format)
        .args(&cookie_args);

    if !audio_only {
        cmd.arg("-f").arg(&format_str);
    } else {
        cmd.arg("-x").arg("--audio-format").arg(&audio_format);
    }

    if subtitles_enabled && !subtitle_langs.is_empty() {
        cmd.arg("--write-subs")
            .arg("--write-auto-subs")
            .arg("--sub-langs")
            .arg(&subtitle_langs);
    }

    // Apply download config
    if config.download.concurrent_fragments > 1 {
        cmd.arg("-N").arg(config.download.concurrent_fragments.to_string());
    }
    if !config.download.limit_rate.is_empty() {
        cmd.arg("-r").arg(&config.download.limit_rate);
    }
    if !config.download.throttled_rate.is_empty() {
        cmd.arg("--throttled-rate").arg(&config.download.throttled_rate);
    }
    if config.download.retries > 0 {
        cmd.arg("--retries").arg(config.download.retries.to_string());
    }
    if config.download.ignore_errors {
        cmd.arg("-i");
    }
    if !config.download.continue_downloads {
        cmd.arg("--no-continue");
    }
    if config.download.no_overwrites {
        cmd.arg("--no-overwrites");
    }
    if config.download.no_part {
        cmd.arg("--no-part");
    }
    if config.download.no_mtime {
        cmd.arg("--no-mtime");
    }

    // Apply extractor config
    if config.extractor.extractor_retries > 0 {
        cmd.arg("--extractor-retries")
            .arg(config.extractor.extractor_retries.to_string());
    }
    if config.extractor.force_generic_extractor {
        cmd.arg("--force-generic-extractor");
    }
    if config.extractor.extract_flat {
        cmd.arg("--flat-playlist");
    }
    if !config.extractor.external_downloader.is_empty() {
        cmd.arg("--external-downloader")
            .arg(&config.extractor.external_downloader);
    }
    if !config.extractor.external_downloader_args.is_empty() {
        cmd.arg("--external-downloader-args")
            .arg(&config.extractor.external_downloader_args);
    }

    // Apply post-processing
    if config.post_processing.embed_thumbnail {
        cmd.arg("--embed-thumbnail");
    }
    if config.post_processing.embed_metadata {
        cmd.arg("--embed-metadata");
    }
    if config.post_processing.embed_subs {
        cmd.arg("--embed-subs");
    }
    if config.post_processing.keep_video {
        cmd.arg("--keep-video");
    }
    if config.post_processing.no_post_overwrites {
        cmd.arg("--no-post-overwrites");
    }
    if !config.post_processing.convert_thumbnails.is_empty() {
        cmd.arg("--convert-thumbnails")
            .arg(&config.post_processing.convert_thumbnails);
    }
    if !config.post_processing.sponsorblock_remove.is_empty() {
        cmd.arg("--sponsorblock-remove")
            .arg(&config.post_processing.sponsorblock_remove);
    }
    if !config.post_processing.sponsorblock_api.is_empty() {
        cmd.arg("--sponsorblock-api")
            .arg(&config.post_processing.sponsorblock_api);
    }

    // Apply advanced settings
    if config.advanced.verbose {
        cmd.arg("--verbose");
    }
    if !config.advanced.user_agent.is_empty() {
        cmd.arg("--user-agent").arg(&config.advanced.user_agent);
    }
    if !config.advanced.referer.is_empty() {
        cmd.arg("--referer").arg(&config.advanced.referer);
    }
    if !config.advanced.proxy.is_empty() {
        cmd.arg("--proxy").arg(&config.advanced.proxy);
    }
    if config.advanced.geo_bypass {
        cmd.arg("--geo-bypass");
    }
    if !config.advanced.geo_bypass_country.is_empty() {
        cmd.arg("--geo-bypass-country")
            .arg(&config.advanced.geo_bypass_country);
    }
    if !config.advanced.geo_verification_proxy.is_empty() {
        cmd.arg("--geo-verification-proxy")
            .arg(&config.advanced.geo_verification_proxy);
    }
    if config.advanced.prefer_free_formats {
        cmd.arg("--prefer-free-formats");
    }
    if config.advanced.check_formats {
        cmd.arg("--check-formats");
    }
    if config.advanced.simulate {
        cmd.arg("--simulate");
    }
    if config.advanced.sleep_interval > 0 {
        cmd.arg("--sleep-interval")
            .arg(config.advanced.sleep_interval.to_string());
    }
    if config.advanced.max_sleep_interval > 0 {
        cmd.arg("--max-sleep-interval")
            .arg(config.advanced.max_sleep_interval.to_string());
    }

    cmd.arg(&url);

    let mut log_lines = Vec::new();
    log_lines.push(format!("Command: {} ...", yt_dlp[0]));

    let mut file_path = None;

    match cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            let stderr = child.stderr.take().unwrap();
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();

            // Shared state between file monitor and stderr reader
            let shared_total = Arc::new(std::sync::Mutex::new(None::<u64>));
            let shared_downloaded = Arc::new(std::sync::Mutex::new(0u64));
            let shared_speed_history = Arc::new(std::sync::Mutex::new(Vec::<(f64, f64)>::new()));
            let download_start = Instant::now();

            // File size monitor for real-time progress
            let monitor_save_dir = save_dir.clone();
            let monitor_cancel = cancel_flag.clone();
            let monitor_app = app.clone();
            let monitor_id = task_id;
            let monitor_total = shared_total.clone();
            let monitor_downloaded = shared_downloaded.clone();
            let monitor_speed_history = shared_speed_history.clone();
            let monitor_download_start = download_start;

            let monitor_handle = tokio::spawn(async move {
                let mut prev_downloaded: u64 = 0;
                let mut last_speed: Option<f64> = None;
                let mut last_check: Option<Instant> = None;
                let poll_interval = tokio::time::Duration::from_millis(200);
                let speed_history = monitor_speed_history;
                let download_start = monitor_download_start;

                loop {
                    if monitor_cancel.load(Ordering::SeqCst) {
                        break;
                    }

                    // Find .part files in the output directory
                    if let Ok(entries) = std::fs::read_dir(&monitor_save_dir) {
                        let mut total_size: u64 = 0;
                        let mut newest_mtime = std::time::SystemTime::UNIX_EPOCH;

                        for entry in entries.flatten() {
                            let path = entry.path();
                            let is_part = path.extension().map_or(false, |e| e == "part");
                            let is_temp = path.extension().map_or(false, |e| e == "ytdl");

                            if is_part || is_temp {
                                if let Ok(meta) = path.metadata() {
                                    total_size += meta.len();
                                    if let Ok(m) = meta.modified() {
                                        if m > newest_mtime {
                                            newest_mtime = m;
                                        }
                                    }
                                    // Try to detect total from .ytdl file
                                    if is_temp {
                                        if let Some(t) = monitor_total.lock().unwrap().as_ref() {
                                            // Already have total, skip
                                            let _ = t;
                                        } else {
                                            if let Some(parsed_total) = try_parse_ytdl_file(&path) {
                                                if parsed_total > 0 {
                                                    *monitor_total.lock().unwrap() = Some(parsed_total);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Also check for completed files (no .part extension)
                        if total_size == 0 {
                            if let Ok(entries2) = std::fs::read_dir(&monitor_save_dir) {
                                for entry in entries2.flatten() {
                                    let path = entry.path();
                                    if let Some(ext) = path.extension() {
                                        let ext_str = ext.to_string_lossy().to_lowercase();
                                        if matches!(ext_str.as_str(), "mp4" | "mkv" | "webm" | "m4a" | "mp3" | "flac" | "opus") {
                                            if let Ok(meta) = path.metadata() {
                                                let size = meta.len();
                                                if size > 10000 {
                                                    total_size += size;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if total_size > 0 {
                            let now = Instant::now();
                            let elapsed = last_check.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);

                            let speed = if elapsed > 0.05 && total_size > prev_downloaded {
                                let delta = total_size - prev_downloaded;
                                Some(delta as f64 / elapsed)
                            } else {
                                last_speed
                            };

                            if speed.is_some() {
                                last_speed = speed;
                            }

                            let total_for_progress = *monitor_total.lock().unwrap();

                            let eta = speed.and_then(|s| {
                                if s > 0.0 {
                                    total_for_progress.map(|t| ((t - total_size) as f64 / s) as u64)
                                } else {
                                    None
                                }
                            });

                            // Progress: if we know total, use percentage;
                            // otherwise, still emit events so UI shows activity
                            let progress = if let Some(t) = total_for_progress {
                                if t > 0 {
                                    (total_size as f64 / t as f64).min(1.0)
                                } else {
                                    0.0
                                }
                            } else {
                                // No total known — use a small fixed total as proxy
                                // This lets the progress bar show *some* movement
                                // based on downloaded amount relative to 100MB
                                (total_size as f64 / 100_000_000f64).min(0.95)
                            };

                            // Track downloaded amount for stdout reader
                            *monitor_downloaded.lock().unwrap() = total_size;

                            // Track speed history
                            {
                                let elapsed = download_start.elapsed().as_secs_f64();
                                if let Some(s) = speed {
                                    speed_history.lock().unwrap().push((elapsed, s));
                                }
                            }
                            let speed_history_snapshot = {
                                let lock = speed_history.lock().unwrap();
                                lock.clone()
                            };

                            let _ = monitor_app.emit("download-progress", ProgressPayload {
                                id: monitor_id,
                                progress,
                                speed,
                                eta,
                                downloaded: total_size,
                                total: total_for_progress,
                                speed_history: speed_history_snapshot,
                            });

                            prev_downloaded = total_size;
                            last_check = Some(now);
                        }
                    }

                    tokio::time::sleep(poll_interval).await;
                }
            });

            // Read stderr line by line — yt-dlp sends progress/log to stderr
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim_end().to_owned();
                if trimmed.is_empty() {
                    continue;
                }

                // Try to parse as JSON progress line from --progress-template
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                    if json.get("progress").is_some() {
                        // This is a progress line — values are strings in yt-dlp template output
                        let parse_u64 = |v: &serde_json::Value| -> Option<u64> {
                            v.as_u64().or_else(|| v.as_str().and_then(|s| {
                                if s.is_empty() || s == "null" || s == "nan" { return None; }
                                s.parse::<u64>().ok()
                            }))
                        };
                        let parse_f64 = |v: &serde_json::Value| -> Option<f64> {
                            v.as_f64().or_else(|| v.as_str().and_then(|s| {
                                if s.is_empty() || s == "null" || s == "nan" || s == "inf" { return None; }
                                s.parse::<f64>().ok()
                            }))
                        };

                        let total = json.get("total_bytes")
                            .and_then(parse_u64)
                            .filter(|&t| t > 0);
                        let downloaded = json.get("downloaded_bytes")
                            .and_then(parse_u64)
                            .unwrap_or(0);
                        let speed = json.get("speed")
                            .and_then(parse_f64)
                            .filter(|&s| s > 0.0);
                        let eta = json.get("eta")
                            .and_then(parse_u64);

                        // Update shared total (only if yt-dlp provided it directly)
                        if let Some(t) = total {
                            *shared_total.lock().unwrap() = Some(t);
                        }

                        let progress_pct = json.get("progress")
                            .and_then(parse_f64)
                            .map(|p| (p / 100.0).min(1.0));

                        // Use yt-dlp percent if available, fallback to calculated
                        let progress = progress_pct.unwrap_or_else(|| {
                            let total_for_progress = *shared_total.lock().unwrap();
                            if let Some(t) = total_for_progress {
                                if t > 0 { (downloaded as f64 / t as f64).min(1.0) } else { 0.0 }
                            } else if downloaded > 0 {
                                (downloaded as f64 / 100_000_000f64).min(0.95)
                            } else {
                                0.0
                            }
                        });

                        // Derive total from progress% when yt-dlp didn't provide it
                        let total_derived: Option<u64> = total.or_else(|| {
                            if progress > 0.01 && downloaded > 0 {
                                Some((downloaded as f64 / progress) as u64)
                            } else {
                                None
                            }
                        });

                        // Update shared total with derived value if needed
                        if let (Some(derived), None) = (total_derived, *shared_total.lock().unwrap()) {
                            *shared_total.lock().unwrap() = Some(derived);
                        }

                        let eta = speed.and_then(|s| {
                            total_derived.map(|t| if s > 0.0 { ((t - downloaded) as f64 / s) as u64 } else { 0 })
                        });

                        // Track speed history
                        {
                            let elapsed = download_start.elapsed().as_secs_f64();
                            if let Some(s) = speed {
                                shared_speed_history.lock().unwrap().push((elapsed, s));
                            }
                        }
                        let speed_history_snapshot = {
                            let lock = shared_speed_history.lock().unwrap();
                            lock.clone()
                        };

                        let _ = app.emit("download-progress", ProgressPayload {
                            id: task_id,
                            progress,
                            speed,
                            eta,
                            downloaded,
                            total: total_derived,
                            speed_history: speed_history_snapshot,
                        });

                        continue;
                    }
                }

                // Non-JSON line — log important events only (filter out verbose yt-dlp chatter)
                if trimmed.starts_with("[download]")
                    || trimmed.starts_with("[info]")
                    || trimmed.starts_with("[MergeFormats]")
                    || trimmed.starts_with("[Merger]")
                    || trimmed.starts_with("[ExtractAudio]")
                    || trimmed.starts_with("[ffmpeg]")
                    || trimmed.starts_with("Error")
                    || trimmed.starts_with("WARNING")
                {
                    log_lines.push(trimmed.clone());
                    let _ = app.emit("download-log", serde_json::json!({
                        "id": task_id,
                        "line": trimmed,
                    }));
                }

                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = child.kill().await;
                    break;
                }
            }

            monitor_handle.abort();

            let status = child.wait().await;
            let success = status.as_ref().is_ok_and(|s| s.success());

            if success {
                log_lines.push("-".repeat(50));
                log_lines.push("Download complete!".to_string());
                file_path = extract_file_path(&log_lines, &save_dir);
            } else {
                let code = status.as_ref().ok().and_then(|s| s.code());
                log_lines.push(format!("Download failed (exit code {code:?})"));
            }

            (success, file_path, log_lines)
        }
        Err(e) => (
            false,
            None,
            vec![format!("Failed to start download: {e}")],
        ),
    }
}

/// Parse a size string like "123.45MiB" or "1.23GiB" into bytes.
/// Handles MiB, GiB, KiB suffixes.
fn parse_size_string(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s.split_whitespace().next().unwrap_or(s); // Take first token

    if let Some(idx) = s.find("GiB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1024.0 * 1024.0 * 1024.0) as u64);
        }
    }
    if let Some(idx) = s.find("MiB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1024.0 * 1024.0) as u64);
        }
    }
    if let Some(idx) = s.find("KiB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1024.0) as u64);
        }
    }
    if let Some(idx) = s.find("GB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1_000_000_000.0) as u64);
        }
    }
    if let Some(idx) = s.find("MB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1_000_000.0) as u64);
        }
    }
    if let Some(idx) = s.find("KB") {
        if let Ok(val) = s[..idx].trim().parse::<f64>() {
            return Some((val * 1_000.0) as u64);
        }
    }
    None
}

fn try_parse_ytdl_file(path: &std::path::Path) -> Option<u64> {
    // yt-dlp .ytdl files are JSON with total_bytes info.
    // Check all known fields across yt-dlp versions.
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            for key in [
                "total_bytes",
                "total_bytes_estimate",
                "filesize",
                "filesize_approx",
                "estimated_video_size",
                "estimated_file_size",
                "content_length",
            ] {
                if let Some(total) = json.get(key).and_then(|v| v.as_u64()) {
                    if total > 0 {
                        return Some(total);
                    }
                }
            }
            // Also check nested "info" or "requested_downloads" objects
            for nested_key in ["info", "requested_downloads"] {
                if let Some(arr) = json.get(nested_key) {
                    if let Some(first) = arr.get(0) {
                        if let Some(total) = first.get("total_bytes").and_then(|v| v.as_u64()) {
                            if total > 0 {
                                return Some(total);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_file_path(log_lines: &[String], save_dir: &str) -> Option<String> {
    for line in log_lines.iter().rev() {
        if line.contains("[download]") && line.contains("has already been downloaded") {
            if let Some(start) = line.find('\'') {
                if let Some(end) = line[start + 1..].find('\'') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
        if line.contains("[Merger] Merging formats into") || line.contains("[ExtractAudio]") {
            // Try to extract the output file path
            if let Some(idx) = line.find("into \"") {
                let rest = &line[idx + 6..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
            if let Some(idx) = line.find("to \"") {
                let rest = &line[idx + 4..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    Some(save_dir.to_string())
}
