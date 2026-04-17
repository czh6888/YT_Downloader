use std::sync::Arc;
use tauri::{State, AppHandle, Manager, Emitter};

use yt_downloader::config::Config;
use yt_downloader::history::HistoryManager;
use yt_downloader::downloader::{self, FormatInfo};
use crate::state::{AppState, DownloadTask};
use crate::download_mgr;

// === Helper ===

/// Resolve cookie arguments: extract cookies via DPAPI/Python pipeline,
/// falling back to yt-dlp --cookies-from-browser if extraction fails.
/// Returns yt-dlp command arguments.
fn resolve_cookie_args(browser_names: &[String]) -> Result<Vec<String>, String> {
    if browser_names.is_empty() {
        return Ok(Vec::new());
    }
    let browser = &browser_names[0];
    if browser == "NoCookies" {
        return Ok(Vec::new());
    }

    let config_dir = dirs::config_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let cookie_dir_path = format!("{config_dir}/yt-downloader/cookies");
    let cookie_file = std::path::PathBuf::from(format!(
        "{cookie_dir_path}/{}_cookies.txt",
        browser.to_lowercase()
    ));

    // Create cookie directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&cookie_dir_path) {
        return Err(format!("Failed to create cookie directory: {e}"));
    }

    let result = downloader::extract_browser_cookies(browser, &cookie_file);
    if result.use_cookie_file {
        Ok(vec!["--cookies".to_string(), result.cookie_file])
    } else if let Some(browser_native) = result.browser_native {
        Ok(vec![
            "--cookies-from-browser".to_string(),
            browser_native,
        ])
    } else {
        Err(format!("Failed to extract cookies from {browser}: {}", result.message))
    }
}

// === yt-dlp Commands ===

#[tauri::command]
pub async fn check_yt_dlp() -> Result<Option<Vec<String>>, String> {
    Ok(downloader::find_yt_dlp())
}

#[derive(serde::Serialize)]
pub struct FetchInfoResult {
    pub info: VideoInfo,
    pub formats: Vec<FormatInfo>,
}

#[derive(serde::Serialize)]
pub struct VideoInfo {
    pub title: String,
    pub thumbnail: String,
    pub uploader: String,
    pub duration: Option<f64>,
    pub description: String,
}

#[tauri::command]
pub async fn fetch_info(
    url: String,
    cookie_args: Vec<String>,
) -> Result<FetchInfoResult, String> {
    // Extract cookies first using the full pipeline (DPAPI → Python → yt-dlp native)
    let cookie_flags = resolve_cookie_args(&cookie_args).unwrap_or_else(|e| {
        log::warn!("Cookie extraction failed: {e}, proceeding without cookies");
        Vec::new()
    });

    let result = downloader::fetch_info_with_cookies(&url, &cookie_flags)
        .await
        .map_err(|e| format!("yt-dlp failed: {e}"))?;

    let title = result.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
    let thumbnail = result.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let uploader = result.get("uploader").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
    let duration = result.get("duration").and_then(|v| v.as_f64());
    let description = result.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let formats = downloader::parse_formats(&result);

    Ok(FetchInfoResult {
        info: VideoInfo {
            title,
            thumbnail,
            uploader,
            duration,
            description,
        },
        formats,
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadParams {
    pub url: String,
    pub title: String,
    pub format_ids: Vec<String>,
    pub save_dir: String,
    pub audio_only: bool,
    pub audio_format: String,
    pub cookie_args: Vec<String>,
    pub subtitles_enabled: bool,
    pub subtitle_langs: String,
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    params: DownloadParams,
) -> Result<u64, String> {
    // Load config from AppState (snake_case TOML source of truth)
    let config = state.config.read().await.clone();

    // Create task and insert into manager — lock scope limited
    let task_id;
    let cancel_flag;
    let cookie_browser;
    {
        let mut mgr = state.download_mgr.write().await;
        task_id = mgr.next_task_id();
        cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Extract browser name from cookie_args
        cookie_browser = if params.cookie_args.is_empty() {
            "NoCookies".to_string()
        } else {
            params.cookie_args[0].clone()
        };

        let task = DownloadTask {
            id: task_id,
            url: params.url.clone(),
            title: params.title.clone(),
            format: params.format_ids.join(", "),
            audio_only: params.audio_only,
            status: "Queued".to_string(),
            progress: 0.0,
            speed: "---".to_string(),
            eta: "--:--".to_string(),
            log: Vec::new(),
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bytes: None,
            eta_seconds: None,
            elapsed_seconds: 0,
            speed_history: Vec::new(),
            file_path: None,
            cancel_flag: cancel_flag.clone(),
        };

        mgr.tasks.insert(task_id, task);
        mgr.mark_started();
    }
    // ← mgr lock released here

    let url = params.url.clone();
    let format_ids = params.format_ids.clone();
    let save_dir = params.save_dir.clone();
    let audio_only = params.audio_only;
    let audio_format = params.audio_format.clone();
    let subtitles_enabled = params.subtitles_enabled;
    let subtitle_langs = params.subtitle_langs.clone();
    let url_for_history = params.url.clone();
    let format_ids_for_history = params.format_ids.clone();
    let title_for_history = params.title.clone();

    // Spawn the download
    let app_clone = app.clone();

    // Immediately set status to Download so the UI shows it
    {
        let state_ref = app.state::<AppState>();
        let mut mgr = state_ref.inner().download_mgr.write().await;
        if let Some(task) = mgr.tasks.get_mut(&task_id) {
            task.status = "Downloading".to_string();
        }
    }
    // ← mgr lock released here
    let _ = app.emit("download-progress", serde_json::json!({
        "id": task_id,
        "progress": 0.0,
        "speed": null,
        "eta": null,
        "downloaded": 0,
        "total": null,
    }));

    tokio::spawn(async move {
        let (success, file_path, log_lines) = download_mgr::run_download(
            app_clone.clone(),
            task_id,
            url,
            format_ids,
            save_dir.clone(),
            audio_only,
            audio_format,
            cookie_browser,
            subtitles_enabled,
            subtitle_langs,
            config.clone(),
            cancel_flag,
        ).await;

        // Update task state
        let state_clone = app_clone.state::<AppState>();
        {
            let mut mgr = state_clone.inner().download_mgr.write().await;
            mgr.mark_finished();
            if let Some(task) = mgr.tasks.get_mut(&task_id) {
                task.status = if success { "Done".to_string() } else { "Failed".to_string() };
                task.log = log_lines.clone();
                task.file_path = file_path.clone();
                if success {
                    task.progress = 1.0;
                }
            }
        }

        // Record in history
        if let Some(ref history_mgr) = *state_clone.inner().history.lock().unwrap() {
            let title = if title_for_history.is_empty() {
                url_for_history.clone()
            } else {
                title_for_history.clone()
            };
            let _ = history_mgr.add_entry(
                &title,
                &url_for_history,
                &format_ids_for_history.join(", "),
                if success { "completed" } else { "failed" },
                file_path.as_deref().unwrap_or(""),
            );
        }

        // Emit completion event
        let _ = app_clone.emit(
            if success { "download-complete" } else { "download-error" },
            task_id,
        );
    });

    Ok(task_id)
}

#[tauri::command]
pub async fn pause_download(
    state: State<'_, AppState>,
    task_id: u64,
) -> Result<(), String> {
    let mut mgr = state.download_mgr.write().await;
    if let Some(task) = mgr.tasks.get_mut(&task_id) {
        task.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        task.status = "Cancelled".to_string();
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_download(
    state: State<'_, AppState>,
    task_id: u64,
) -> Result<(), String> {
    let mut mgr = state.download_mgr.write().await;
    if let Some(task) = mgr.tasks.get_mut(&task_id) {
        task.cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
        task.status = "Downloading".to_string();
    }
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    task_id: u64,
) -> Result<(), String> {
    let mut mgr = state.download_mgr.write().await;
    if let Some(task) = mgr.tasks.get_mut(&task_id) {
        task.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        task.status = "Cancelled".to_string();
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct TaskInfo {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub format: String,
    pub audio_only: bool,
    pub status: String,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: u64,
    pub speed_history: Vec<(f64, f64)>,
    pub file_path: Option<String>,
}

#[tauri::command]
pub async fn get_download_state(
    state: State<'_, AppState>,
) -> Result<Vec<TaskInfo>, String> {
    let mgr = state.download_mgr.read().await;
    Ok(mgr.tasks.values().map(|t| TaskInfo {
        id: t.id,
        url: t.url.clone(),
        title: t.title.clone(),
        format: t.format.clone(),
        audio_only: t.audio_only,
        status: t.status.clone(),
        progress: t.progress,
        speed: t.speed.clone(),
        eta: t.eta.clone(),
        downloaded_bytes: t.downloaded_bytes,
        total_bytes: t.total_bytes,
        speed_bytes: t.speed_bytes,
        eta_seconds: t.eta_seconds,
        elapsed_seconds: t.elapsed_seconds,
        speed_history: t.speed_history.clone(),
        file_path: t.file_path.clone(),
    }).collect())
}

// === Config Commands ===

#[tauri::command]
pub async fn load_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    config.save()?;
    let mut cfg = state.config.write().await;
    *cfg = config;
    Ok(())
}

// === History Commands ===

#[derive(serde::Serialize)]
pub struct HistoryEntryDto {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub format: String,
    pub status: String,
    pub date: String,
    pub file_path: String,
}

#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<HistoryEntryDto>, String> {
    let history = state.history.lock().unwrap();
    let Some(mgr) = history.as_ref() else {
        return Ok(Vec::new());
    };

    let entries = if query.is_empty() {
        mgr.load_entries()
    } else {
        mgr.search_entries(&query)
    };

    Ok(entries.into_iter().map(|e| HistoryEntryDto {
        id: e.id,
        title: e.title,
        url: e.url,
        format: e.format,
        status: e.status,
        date: e.date,
        file_path: e.file_path,
    }).collect())
}

#[tauri::command]
pub async fn delete_history(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let history = state.history.lock().unwrap();
    if let Some(mgr) = history.as_ref() {
        mgr.delete_entry(id).map_err(|e| format!("{e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_history(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let history = state.history.lock().unwrap();
    if let Some(mgr) = history.as_ref() {
        mgr.clear_all().map_err(|e| format!("{e}"))?;
    }
    Ok(())
}

// === Utility Commands ===

#[tauri::command]
pub async fn extract_cookies(
    browser: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config_dir = dirs::config_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let cookie_dir_path = format!("{config_dir}/yt-downloader/cookies");
    let cookie_file = format!("{cookie_dir_path}/{}_cookies.txt", browser.to_lowercase());

    // Create cookie directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&cookie_dir_path) {
        return Err(format!("Failed to create cookie directory: {e}"));
    }

    let result = downloader::extract_browser_cookies(
        &browser,
        &std::path::PathBuf::from(&cookie_file),
    );

    let file_path = if result.use_cookie_file {
        Some(result.cookie_file)
    } else {
        None
    };

    Ok(serde_json::json!({
        "file": file_path,
        "fallback": result.browser_native,
        "success": file_path.is_some() || result.browser_native.is_some(),
        "message": result.message,
    }))
}

#[tauri::command]
pub async fn detect_ffmpeg() -> bool {
    downloader::detect_ffmpeg()
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open: {e}"))
}

#[tauri::command]
pub async fn show_in_explorer(file_path: String) -> Result<(), String> {
    // Windows: explorer /select,"path"
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        open::that(&file_path).map_err(|e| format!("Failed: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_file(file_path: String) -> Result<(), String> {
    std::fs::remove_file(&file_path).map_err(|e| format!("Failed to delete: {e}"))
}

#[tauri::command]
pub async fn get_clipboard(
    app: AppHandle,
) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| format!("Clipboard error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_params_deserializes_camelcase() {
        let json = r#"{
            "url": "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "formatIds": ["best", "137"],
            "saveDir": "C:\\Users\\CZH\\Videos",
            "audioOnly": false,
            "audioFormat": "mp3",
            "cookieArgs": ["Firefox"],
            "subtitlesEnabled": false,
            "subtitleLangs": ""
        }"#;
        let params: DownloadParams = serde_json::from_str(json).expect("should deserialize camelCase");
        assert_eq!(params.url, "https://www.youtube.com/watch?v=jNQXAC9IVRw");
        assert_eq!(params.format_ids, vec!["best", "137"]);
        assert_eq!(params.save_dir, "C:\\Users\\CZH\\Videos");
        assert!(!params.audio_only);
        assert_eq!(params.audio_format, "mp3");
        assert_eq!(params.cookie_args, vec!["Firefox"]);
        assert!(!params.subtitles_enabled);
    }
}
