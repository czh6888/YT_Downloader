use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use yt_downloader::config::Config;
use yt_downloader::history::HistoryManager;

pub struct DownloadTask {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub format: String,
    pub audio_only: bool,
    pub status: String,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub log: Vec<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: u64,
    pub speed_history: Vec<(f64, f64)>,
    pub file_path: Option<String>,
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct DownloadManager {
    pub tasks: HashMap<u64, DownloadTask>,
    pub max_concurrent: usize,
    pub next_id: u64,
    pub active_count: usize,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            max_concurrent: 3,
            next_id: 1,
            active_count: 0,
        }
    }

    pub fn next_task_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn can_start(&self) -> bool {
        self.active_count < self.max_concurrent
    }

    pub fn mark_started(&mut self) {
        self.active_count += 1;
    }

    pub fn mark_finished(&mut self) {
        if self.active_count > 0 {
            self.active_count -= 1;
        }
    }

    pub fn update_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = max;
    }
}

pub struct AppState {
    pub config: tokio::sync::RwLock<Config>,
    pub history: Mutex<Option<HistoryManager>>,
    pub download_mgr: tokio::sync::RwLock<DownloadManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: tokio::sync::RwLock::new(Config::load()),
            history: Mutex::new(HistoryManager::new()),
            download_mgr: tokio::sync::RwLock::new(DownloadManager::new()),
        }
    }
}
