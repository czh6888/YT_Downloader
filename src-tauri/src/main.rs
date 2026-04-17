// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod download_mgr;
mod state;

use state::AppState;
use commands::*;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            check_yt_dlp,
            fetch_info,
            start_download,
            pause_download,
            resume_download,
            cancel_download,
            get_download_state,
            load_config,
            save_config,
            get_history,
            delete_history,
            clear_history,
            extract_cookies,
            detect_ffmpeg,
            open_folder,
            show_in_explorer,
            delete_file,
            get_clipboard,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri");
}
