use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{
    button, checkbox, column, container, horizontal_space, pick_list, progress_bar, row,
    scrollable, text_input, vertical_space, Column,
};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use tokio::io::AsyncBufReadExt;

use crate::downloader::{self, CookieResult, FormatInfo};
use crate::config::Config;
use crate::history::{HistoryEntry, HistoryManager};
use crate::ui::format_dialog::{FormatDialog, FormatFilter};

// ---------------------------------------------------------------------------
// CJK Font
// ---------------------------------------------------------------------------

fn cjk_text<'a>(content: impl std::fmt::Display) -> iced::widget::Text<'a, iced::Theme> {
    iced::widget::text(content.to_string()).font(Font::with_name("Microsoft YaHei"))
}

// ---------------------------------------------------------------------------
// Internationalization
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Chinese,
}
impl Language {
    pub fn toggle(self) -> Self {
        match self {
            Language::English => Language::Chinese,
            Language::Chinese => Language::English,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }
}

pub struct Lang {
    pub page_title: &'static str,
    pub sidebar_download: &'static str,
    pub sidebar_queue: &'static str,
    pub sidebar_history: &'static str,
    pub sidebar_settings: &'static str,
    pub video_url_label: &'static str,
    pub video_url_hint: &'static str,
    pub browser_label: &'static str,
    pub fetch_btn: &'static str,
    pub audio_only_cb: &'static str,
    pub audio_format_label: &'static str,
    pub download_now_btn: &'static str,
    pub add_to_queue_btn: &'static str,
    pub info_label: &'static str,
    pub open_folder_btn: &'static str,
    pub resolution_label: &'static str,
    pub best_quality: &'static str,
    pub select_format_btn: &'static str,
    pub format_count_label: &'static str,
    pub status_idle: &'static str,
    pub status_extracting: &'static str,
    pub status_fetching: &'static str,
    pub status_ready: &'static str,
    pub queue_title: &'static str,
    pub queue_empty: &'static str,
    pub status_queued: &'static str,
    pub status_fetching_info: &'static str,
    pub status_downloading: &'static str,
    pub status_done: &'static str,
    pub status_cancelled: &'static str,
    pub status_failed: &'static str,
    pub cancel_btn: &'static str,
    pub remove_btn: &'static str,
    pub history_title: &'static str,
    pub history_empty: &'static str,
    pub history_search_hint: &'static str,
    pub history_clear_btn: &'static str,
    pub settings_title: &'static str,
    pub language_label: &'static str,
    pub theme_label: &'static str,
    pub download_dir_label: &'static str,
    pub download_dir_hint: &'static str,
    pub max_concurrent_label: &'static str,
    pub clipboard_cb: &'static str,
    pub subtitles_label: &'static str,
    pub subtitles_cb: &'static str,
    pub subtitle_langs_label: &'static str,
    pub subtitle_langs_hint: &'static str,
    pub ffmpeg_label: &'static str,
    pub ffmpeg_available: &'static str,
    pub ffmpeg_missing: &'static str,
    pub save_config_btn: &'static str,
    pub config_saved: &'static str,
}

impl Lang {
    pub fn for_lang(lang: Language) -> &'static Self {
        static EN: Lang = Lang {
            page_title: "YouTube Downloader",
            sidebar_download: "Download",
            sidebar_queue: "Queue",
            sidebar_history: "History",
            sidebar_settings: "Settings",
            video_url_label: "Video URL",
            video_url_hint: "https://www.youtube.com/watch?v=... or any yt-dlp supported site",
            browser_label: "Browser:",
            fetch_btn: "Fetch Info",
            audio_only_cb: "Audio only (extract audio track)",
            audio_format_label: "Audio format:",
            download_now_btn: "Download Now",
            add_to_queue_btn: "Add to Queue",
            info_label: "Info",
            open_folder_btn: "Open Download Folder",
            resolution_label: "Resolution:",
            best_quality: "Best quality",
            select_format_btn: "Select Format",
            format_count_label: "formats available",
            status_idle: "Ready",
            status_extracting: "Extracting browser cookies...",
            status_fetching: "Fetching video info...",
            status_ready: "Ready to download",
            queue_title: "Download Queue",
            queue_empty: "No downloads in queue.\nFetch a video and click 'Add to Queue' to get started.",
            status_queued: "Queued",
            status_fetching_info: "Fetching info...",
            status_downloading: "Downloading",
            status_done: "Done",
            status_cancelled: "Cancelled",
            status_failed: "Failed",
            cancel_btn: "Cancel",
            remove_btn: "Remove",
            history_title: "Download History",
            history_empty: "No downloads yet.",
            history_search_hint: "Search history...",
            history_clear_btn: "Clear History",
            settings_title: "Settings",
            language_label: "Language:",
            theme_label: "Theme:",
            download_dir_label: "Download Directory",
            download_dir_hint: "Download path",
            max_concurrent_label: "Max Concurrent Downloads",
            clipboard_cb: "Monitor clipboard for video URLs",
            subtitles_label: "Subtitles",
            subtitles_cb: "Download subtitles when available",
            subtitle_langs_label: "Subtitle Languages",
            subtitle_langs_hint: "Comma-separated: zh-Hans,en",
            ffmpeg_label: "FFmpeg",
            ffmpeg_available: "Installed",
            ffmpeg_missing: "Not found in PATH",
            save_config_btn: "Save Settings",
            config_saved: "Settings saved!",
        };
        static ZH: Lang = Lang {
            page_title: "YouTube 下载器",
            sidebar_download: "下载",
            sidebar_queue: "队列",
            sidebar_history: "历史",
            sidebar_settings: "设置",
            video_url_label: "视频链接",
            video_url_hint: "https://www.youtube.com/watch?v=... 或任意 yt-dlp 支持的站点",
            browser_label: "浏览器：",
            fetch_btn: "获取信息",
            audio_only_cb: "仅音频（提取音轨）",
            audio_format_label: "音频格式：",
            download_now_btn: "立即下载",
            add_to_queue_btn: "加入队列",
            info_label: "信息",
            open_folder_btn: "打开下载文件夹",
            resolution_label: "分辨率：",
            best_quality: "最佳质量",
            select_format_btn: "选择格式",
            format_count_label: "个可用格式",
            status_idle: "就绪",
            status_extracting: "正在提取浏览器Cookie...",
            status_fetching: "正在获取视频信息...",
            status_ready: "可以下载",
            queue_title: "下载队列",
            queue_empty: "队列中暂无任务。\n获取视频信息后点击\"加入队列\"开始。",
            status_queued: "排队中",
            status_fetching_info: "获取信息中...",
            status_downloading: "正在下载",
            status_done: "已完成",
            status_cancelled: "已取消",
            status_failed: "失败",
            cancel_btn: "取消",
            remove_btn: "移除",
            history_title: "下载历史",
            history_empty: "暂无下载记录。",
            history_search_hint: "搜索历史记录...",
            history_clear_btn: "清空历史",
            settings_title: "设置",
            language_label: "语言：",
            theme_label: "主题：",
            download_dir_label: "下载目录",
            download_dir_hint: "下载路径",
            max_concurrent_label: "最大并发下载数",
            clipboard_cb: "自动检测剪贴板中的视频链接",
            subtitles_label: "字幕",
            subtitles_cb: "下载时自动获取字幕",
            subtitle_langs_label: "字幕语言",
            subtitle_langs_hint: "逗号分隔：zh-Hans,en",
            ffmpeg_label: "FFmpeg",
            ffmpeg_available: "已安装",
            ffmpeg_missing: "PATH 中未找到",
            save_config_btn: "保存设置",
            config_saved: "设置已保存！",
        };
        match lang {
            Language::English => &EN,
            Language::Chinese => &ZH,
        }
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Download,
    Queue,
    History,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
    pub fn to_iced(&self) -> iced::Theme {
        match self {
            ThemeMode::Light => iced::theme::Theme::Light,
            ThemeMode::Dark => iced::theme::Theme::Dark,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
}
impl std::fmt::Display for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Browser::Chrome => write!(f, "Chrome"),
            Browser::Edge => write!(f, "Edge"),
            Browser::Firefox => write!(f, "Firefox"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub format: String,
    pub audio_only: bool,
    pub status: TaskStatus,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Fetching,
    Downloading,
    Done,
    Cancelled,
    Failed(String),
}

type CancelFlags = Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>;

pub struct App {
    page: Page,
    pub theme: ThemeMode,
    pub language: Language,

    url: String,
    browser: Browser,
    status: FetchStatus,
    formats: Vec<FormatInfo>,
    selected_format_id: String,
    audio_only: bool,
    audio_format: AudioFormat,
    video_info_log: Vec<String>,
    save_dir: String,

    cookie_file: String,
    cookie_result: Option<CookieResult>,

    tasks: Vec<DownloadTask>,
    next_task_id: u64,
    max_concurrent: usize,
    cancel_flags: CancelFlags,

    history: Vec<HistoryEntry>,
    history_mgr: Option<HistoryManager>,
    history_search: String,

    clipboard_monitor: bool,
    last_clipboard: String,

    subtitles_enabled: bool,
    subtitle_langs: String,

    config: Config,
    config_saved: bool,

    format_dialog: FormatDialog,
}

#[derive(Debug, Clone, PartialEq)]
enum FetchStatus {
    Idle,
    ExtractingCookies,
    Fetching,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioFormat {
    Mp3,
    #[default]
    M4a,
    Flac,
    Opus,
}
impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Mp3 => write!(f, "mp3"),
            AudioFormat::M4a => write!(f, "m4a"),
            AudioFormat::Flac => write!(f, "flac"),
            AudioFormat::Opus => write!(f, "opus"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    NavigateTo(Page),
    ToggleTheme,
    ToggleLanguage,

    UrlChanged(String),
    BrowserChanged(Browser),
    Fetch,
    FetchResult(Result<serde_json::Value, String>, CookieResult),
    AudioOnlyToggled(bool),
    AudioFormatChanged(AudioFormat),

    Download,
    AddToQueue,
    CancelTask(u64),
    RemoveTask(u64),
    TaskResult { id: u64, result: downloader::DownloadResult },
    TaskLog { id: u64, line: String },

    SaveDirChanged(String),
    MaxConcurrentChanged(String),
    ClipboardToggled(bool),
    ClipboardCheck(String),

    SubtitlesToggled(bool),
    SubtitleLangsChanged(String),

    SelectFormat,
    FormatSelected(String),
    CloseFormatDialog,
    FormatDialogFilterChanged(FormatFilter),
    FormatDialogSearchChanged(String),

    HistorySearchChanged(String),
    HistoryClear,

    SaveConfig,
    ConfigSaved(Result<(), String>),
}

// ---------------------------------------------------------------------------
// Impl
// ---------------------------------------------------------------------------

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = Config::load();

        let history_mgr = HistoryManager::new();
        let history = history_mgr
            .as_ref()
            .map(|m| m.load_entries())
            .unwrap_or_default();

        let theme = match config.general.theme.as_str() {
            "Dark" => ThemeMode::Dark,
            _ => ThemeMode::Light,
        };
        let language = match config.general.language.as_str() {
            "中文" => Language::Chinese,
            _ => Language::English,
        };

        let save_dir = if !config.general.download_dir.is_empty() {
            config.general.download_dir.clone()
        } else {
            dirs::video_dir()
                .unwrap_or_else(|| PathBuf::from("C:\\Users\\CZH\\Videos"))
                .to_string_lossy()
                .to_string()
        };

        let cookie_file = std::env::var("TEMP")
            .map(|t| format!("{t}\\yt_cookies.txt"))
            .unwrap_or_else(|_| "yt_cookies.txt".to_string());

        let app = App {
            page: Page::default(),
            theme,
            language,
            url: String::new(),
            browser: Browser::Chrome,
            status: FetchStatus::Idle,
            formats: Vec::new(),
            selected_format_id: "best".to_string(),
            audio_only: false,
            audio_format: match config.defaults.audio_format.as_str() {
                "mp3" => AudioFormat::Mp3,
                "flac" => AudioFormat::Flac,
                "opus" => AudioFormat::Opus,
                _ => AudioFormat::M4a,
            },
            video_info_log: Vec::new(),
            save_dir,
            cookie_file,
            cookie_result: None,
            tasks: Vec::new(),
            next_task_id: 1,
            max_concurrent: config.general.max_concurrent,
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            history,
            history_mgr,
            history_search: String::new(),
            clipboard_monitor: config.general.clipboard_monitor,
            last_clipboard: String::new(),
            subtitles_enabled: config.defaults.subtitles_enabled,
            subtitle_langs: config.defaults.subtitle_langs.clone(),
            config,
            config_saved: false,
            format_dialog: FormatDialog::default(),
        };

        (app, Task::none())
    }

    fn lang(&self) -> &'static Lang {
        Lang::for_lang(self.language)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavigateTo(page) => {
                self.page = page;
            }
            Message::ToggleTheme => {
                self.theme = self.theme.toggle();
                self.config.general.theme = self.theme.label().to_string();
            }
            Message::ToggleLanguage => {
                self.language = self.language.toggle();
                self.config.general.language = self.language.label().to_string();
            }
            Message::UrlChanged(url) => {
                self.url = url;
            }
            Message::BrowserChanged(browser) => {
                self.browser = browser;
            }
            Message::Fetch => {
                if self.url.trim().is_empty() {
                    return Task::none();
                }
                self.status = FetchStatus::ExtractingCookies;
                self.video_info_log.clear();
                self.formats.clear();

                let url = self.url.clone();
                let browser_name = self.browser.to_string();
                let cookie_file = PathBuf::from(&self.cookie_file);

                return Task::perform(
                    async move {
                        let cookie_result =
                            downloader::extract_browser_cookies(&browser_name, &cookie_file);
                        let (cookie_file_opt, browser_native_opt) =
                            cookie_args_from_result(&cookie_result);
                        let cookie_args = downloader::cookie_args(cookie_file_opt.as_deref(), browser_native_opt.as_deref());
                        let info = downloader::fetch_info(&url, &cookie_args).await;
                        match info {
                            Ok(v) => Ok((v, cookie_result)),
                            Err(e) => Err((e.to_string(), cookie_result)),
                        }
                    },
                    |result| match result {
                        Ok((info, cookie_result)) => Message::FetchResult(Ok(info), cookie_result),
                        Err((e, cookie_result)) => Message::FetchResult(Err(e), cookie_result),
                    },
                );
            }
            Message::FetchResult(Ok(info), cookie_result) => {
                self.formats = downloader::parse_formats(&info);
                let title = info
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                self.selected_format_id = "best".to_string();
                self.status = FetchStatus::Ready;
                self.video_info_log.push(format!("Title: {title}"));
                self.video_info_log.push(format!("Found {} formats", self.formats.len()));
                self.video_info_log.push(format!("Cookie: {}", cookie_result.message));
                self.cookie_result = Some(cookie_result);
            }
            Message::FetchResult(Err(e), cookie_result) => {
                self.status = FetchStatus::Idle;
                self.video_info_log.push(format!("Error: {e}"));
                self.video_info_log.push(format!("Cookie: {}", cookie_result.message));
                self.cookie_result = Some(cookie_result);
            }
            Message::AudioOnlyToggled(val) => {
                self.audio_only = val;
            }
            Message::AudioFormatChanged(fmt) => {
                self.audio_format = fmt;
                self.config.defaults.audio_format = fmt.to_string();
            }
            Message::Download | Message::AddToQueue => {
                if self.url.trim().is_empty() {
                    return Task::none();
                }
                let id = self.next_task_id;
                self.next_task_id += 1;

                let status = if matches!(message, Message::Download) {
                    TaskStatus::Downloading
                } else {
                    TaskStatus::Queued
                };

                let format_label = if self.selected_format_id == "best" {
                    "best".to_string()
                } else {
                    self.formats
                        .iter()
                        .find(|f| f.format_id == self.selected_format_id)
                        .map(|f| f.resolution.clone())
                        .unwrap_or_else(|| self.selected_format_id.clone())
                };

                let task = DownloadTask {
                    id,
                    url: self.url.clone(),
                    title: self.video_info_log.first().cloned().unwrap_or_default(),
                    format: format_label,
                    audio_only: self.audio_only,
                    status,
                    progress: 0.0,
                    speed: String::new(),
                    eta: String::new(),
                    log: Vec::new(),
                };
                self.tasks.push(task);

                if matches!(message, Message::Download) {
                    return self.start_download(id);
                }
            }
            Message::CancelTask(id) => {
                // Set cancel flag
                {
                    let flags = self.cancel_flags.lock().unwrap();
                    if let Some(flag) = flags.get(&id) {
                        flag.store(true, Ordering::SeqCst);
                    }
                }
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.status = TaskStatus::Cancelled;
                }
                return self.process_queue();
            }
            Message::RemoveTask(id) => {
                self.tasks.retain(|t| t.id != id);
            }
            Message::TaskResult { id, result } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    let was_cancelled = {
                        let flags = self.cancel_flags.lock().unwrap();
                        flags.get(&id).map(|f| f.load(Ordering::SeqCst)).unwrap_or(false)
                    };

                    if was_cancelled {
                        task.status = TaskStatus::Cancelled;
                    } else if result.success {
                        task.status = TaskStatus::Done;
                        task.progress = 1.0;
                        let title = task.title.strip_prefix("Title: ")
                            .unwrap_or(&task.title).to_string();
                        let file_path = result.file_path.clone().unwrap_or_default();
                        let entry_id = if let Some(mgr) = &self.history_mgr {
                            mgr.add_entry(&title, &task.url, &task.format, "Completed", &file_path).ok()
                        } else {
                            None
                        };
                        self.history.insert(0, HistoryEntry {
                            id: entry_id.unwrap_or(0),
                            title: title.clone(),
                            url: task.url.clone(),
                            format: task.format.clone(),
                            status: "Completed".to_string(),
                            date: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                            file_path: file_path.clone(),
                        });
                    } else {
                        task.status = TaskStatus::Failed("Download failed".to_string());
                    }
                    {
                        let mut flags = self.cancel_flags.lock().unwrap();
                        flags.remove(&id);
                    }
                    // Start next queued task
                    return self.process_queue();
                }
            }
            Message::TaskLog { id, line } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.log.push(line.clone());
                    if let Some(prog) = downloader::parse_progress(&line) {
                        task.progress = prog.percentage / 100.0;
                        task.speed = format_speed(prog.speed);
                        task.eta = format_eta(prog.eta);
                    }
                }
            }
            Message::SaveDirChanged(dir) => {
                self.save_dir = dir.clone();
                self.config.general.download_dir = dir;
            }
            Message::MaxConcurrentChanged(val) => {
                if let Ok(n) = val.parse::<usize>()
                    && n > 0 && n <= 10 {
                        self.max_concurrent = n;
                        self.config.general.max_concurrent = n;
                    }
            }
            Message::ClipboardToggled(val) => {
                self.clipboard_monitor = val;
                self.config.general.clipboard_monitor = val;
            }
            Message::ClipboardCheck(content) => {
                if self.clipboard_monitor && !content.is_empty() && content != self.last_clipboard {
                    if is_video_url(&content) {
                        self.last_clipboard = content.clone();
                        self.url = content;
                    }
                }
            }
            Message::SubtitlesToggled(val) => {
                self.subtitles_enabled = val;
                self.config.defaults.subtitles_enabled = val;
            }
            Message::SubtitleLangsChanged(langs) => {
                self.subtitle_langs = langs.clone();
                self.config.defaults.subtitle_langs = langs;
            }
            Message::SelectFormat => {
                let current = self.selected_format_id.clone();
                self.format_dialog.open(&current);
            }
            Message::FormatSelected(format_id) => {
                self.selected_format_id = format_id;
                self.format_dialog.close();
            }
            Message::CloseFormatDialog => {
                self.format_dialog.close();
            }
            Message::FormatDialogFilterChanged(filter) => {
                self.format_dialog.filter = filter;
            }
            Message::FormatDialogSearchChanged(text) => {
                self.format_dialog.search = text;
            }
            Message::HistorySearchChanged(text) => {
                self.history_search = text;
            }
            Message::HistoryClear => {
                self.history.clear();
                self.history_search.clear();
                if let Some(mgr) = &self.history_mgr {
                    let _ = mgr.clear_all();
                }
            }
            Message::SaveConfig => {
                let config = self.config.clone();
                return Task::perform(
                    async move { config.save() },
                    Message::ConfigSaved,
                );
            }
            Message::ConfigSaved(Ok(_)) => {
                self.config_saved = true;
            }
            Message::ConfigSaved(Err(_)) => {}
        }
        Task::none()
    }

    fn start_download(&self, id: u64) -> Task<Message> {
        let url = self.url.clone();
        let format_id = self.selected_format_id.clone();
        let save_dir = self.save_dir.clone();
        let audio_only = self.audio_only;
        let audio_format = self.audio_format.to_string();
        let browser = self.browser.to_string();
        let cookie_file = self.cookie_file.clone();
        let cookie_result = self.cookie_result.clone();
        let subtitles_enabled = self.subtitles_enabled;
        let subtitle_langs = self.subtitle_langs.clone();

        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut flags = self.cancel_flags.lock().unwrap();
            flags.insert(id, cancel_flag.clone());
        }

        Task::stream(iced::stream::channel(16, move |mut output| {
            let url = url.clone();
            let format_id = format_id.clone();
            let save_dir = save_dir.clone();
            let cookie_file = cookie_file.clone();
            let cookie_result = cookie_result.clone();
            let browser = browser.clone();
            let subtitle_langs = subtitle_langs.clone();
            let cancel_flag = cancel_flag.clone();

            async move {
                let (cookie_file_opt, browser_native_opt) = cookie_result
                    .as_ref()
                    .map(|cr| cookie_args_from_result(cr))
                    .unwrap_or_else(|| {
                        let path = PathBuf::from(&cookie_file);
                        let cr = downloader::extract_browser_cookies(&browser, &path);
                        cookie_args_from_result(&cr)
                    });

                let cookie_args = downloader::cookie_args(cookie_file_opt.as_deref(), browser_native_opt.as_deref());

                let format_str = if audio_only {
                    format!("-x --audio-format {audio_format}")
                } else {
                    downloader::build_format_string_from_id(&format_id)
                };

                let yt_dlp = match downloader::find_yt_dlp() {
                    Some(cmd) => cmd,
                    None => {
                        let _ = output.try_send(Message::TaskResult {
                            id,
                            result: downloader::DownloadResult {
                                success: false,
                                log_lines: vec!["Error: yt-dlp not found".to_string()],
                                file_path: None,
                            },
                        });
                        return;
                    }
                };

                let output_template = format!("{save_dir}/%(title)s [%(id)s].%(ext)s");

                let mut cmd = tokio::process::Command::new(&yt_dlp[0]);
                cmd.args(&yt_dlp[1..])
                    .arg("--newline")
                    .arg("--progress")
                    .arg("-o")
                    .arg(&output_template)
                    .arg("--merge-output-format")
                    .arg("mp4");

                if audio_only {
                    cmd.arg("-x").arg("--audio-format").arg(&audio_format);
                } else {
                    cmd.arg("-f").arg(&format_str);
                }

                if subtitles_enabled && !subtitle_langs.is_empty() {
                    cmd.arg("--write-subs")
                       .arg("--write-auto-subs")
                       .arg("--sub-langs")
                       .arg(&subtitle_langs);
                }

                cmd.args(&cookie_args).arg(&url);

                let mut log_lines = Vec::new();
                log_lines.push(format!("Command: {} ...", yt_dlp[0]));
                log_lines.push("-".repeat(50));

                match cmd.stdout(std::process::Stdio::piped()).spawn() {
                    Ok(mut child) => {
                        let stdout = child.stdout.take().unwrap();
                        let reader = tokio::io::BufReader::new(stdout);
                        let mut lines = reader.lines();

                        loop {
                            if cancel_flag.load(Ordering::SeqCst) {
                                let _ = child.kill().await;
                                log_lines.push("-".repeat(50));
                                log_lines.push("Download cancelled".to_string());
                                let _ = output.try_send(Message::TaskResult {
                                    id,
                                    result: downloader::DownloadResult {
                                        success: false,
                                        log_lines,
                                        file_path: None,
                                    },
                                });
                                return;
                            }

                            match lines.next_line().await {
                                Ok(Some(line)) => {
                                    let trimmed = line.trim_end().to_owned();
                                    if !trimmed.is_empty() {
                                        log_lines.push(trimmed.clone());
                                        let _ = output.try_send(Message::TaskLog { id, line: trimmed });
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }

                        if cancel_flag.load(Ordering::SeqCst) {
                            log_lines.push("Download cancelled".to_string());
                            let _ = output.try_send(Message::TaskResult {
                                id,
                                result: downloader::DownloadResult {
                                    success: false,
                                    log_lines,
                                    file_path: None,
                                },
                            });
                            return;
                        }

                        let status = child.wait().await;
                        let success = status.as_ref().is_ok_and(|s| s.success());

                        if success {
                            log_lines.push("-".repeat(50));
                            log_lines.push("Download complete!".to_string());
                        } else {
                            let code = status.as_ref().ok().and_then(|s| s.code());
                            log_lines.push(format!("Download failed (exit code {code:?})"));
                        }

                        let _ = output.try_send(Message::TaskResult {
                            id,
                            result: downloader::DownloadResult {
                                success,
                                log_lines,
                                file_path: Some(save_dir.clone()),
                            },
                        });
                    }
                    Err(e) => {
                        let _ = output.try_send(Message::TaskResult {
                            id,
                            result: downloader::DownloadResult {
                                success: false,
                                log_lines: vec![format!("Failed to start: {e}")],
                                file_path: None,
                            },
                        });
                    }
                }
            }
        }))
    }

    fn process_queue(&mut self) -> Task<Message> {
        let active_count = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Downloading | TaskStatus::Fetching))
            .count();

        if active_count < self.max_concurrent
            && let Some(pos) = self
                .tasks
                .iter()
                .position(|t| matches!(t.status, TaskStatus::Queued))
            {
                let id = self.tasks[pos].id;
                self.tasks[pos].status = TaskStatus::Downloading;
                return self.start_download(id);
            }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.clipboard_monitor {
            iced::time::every(std::time::Duration::from_secs(2)).map(|_| {
                Message::ClipboardCheck(
                    arboard::Clipboard::new()
                        .ok()
                        .and_then(|mut c| c.get_text().ok())
                        .unwrap_or_default(),
                )
            })
        } else {
            Subscription::none()
        }
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    fn page_label(&self, page: Page) -> &'static str {
        let l = self.lang();
        match page {
            Page::Download => l.sidebar_download,
            Page::Queue => l.sidebar_queue,
            Page::History => l.sidebar_history,
            Page::Settings => l.sidebar_settings,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let is_dark = matches!(self.theme, ThemeMode::Dark);

        let content = match self.page {
            Page::Download => self.download_view(is_dark),
            Page::Queue => self.queue_view(is_dark),
            Page::History => self.history_view(is_dark),
            Page::Settings => self.settings_view(is_dark),
        };

        let sidebar = self.sidebar_view(is_dark);

        let main = row![
            sidebar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(16),
        ]
        .height(Length::Fill);

        if self.format_dialog.open && !self.formats.is_empty() {
            let dialog = self.format_dialog.view(
                &self.formats,
                Message::FormatSelected,
                || Message::CloseFormatDialog,
                Message::FormatDialogFilterChanged,
                Message::FormatDialogSearchChanged,
            );

            iced::widget::stack![
                main,
                dialog,
            ]
            .into()
        } else {
            main.into()
        }
    }

    fn sidebar_view(&self, is_dark: bool) -> Element<'_, Message> {
        let bg = if is_dark {
            iced::Color { r: 0.12, g: 0.12, b: 0.14, a: 1.0 }
        } else {
            iced::Color { r: 0.95, g: 0.95, b: 0.97, a: 1.0 }
        };

        let pages = [Page::Download, Page::Queue, Page::History, Page::Settings];
        let mut col = Column::new().spacing(4).padding(12);

        for page in pages {
            let label = self.page_label(page);
            let is_active = page == self.page;
            let btn = button(cjk_text(label).size(14))
                .padding(10)
                .width(Length::Fill)
                .on_press(Message::NavigateTo(page));

            let btn = if is_active {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                    })),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else {
                btn
            };
            col = col.push(btn);
        }

        col = col.push(vertical_space());

        container(col)
            .width(Length::Fixed(160.0))
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                ..Default::default()
            })
            .into()
    }

    fn download_view(&self, is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let mut col = Column::new().spacing(16).padding(8);

        col = col.push(cjk_text(l.page_title).size(22));

        col = col.push(
            column![
                cjk_text(l.video_url_label).size(14),
                text_input(l.video_url_hint, &self.url)
                    .on_input(Message::UrlChanged)
                    .on_submit(Message::Fetch)
                    .padding(10)
                    .size(13),
            ]
            .spacing(4),
        );

        col = col.push(self.browser_row());

        let is_fetching = !matches!(self.status, FetchStatus::Idle | FetchStatus::Ready);
        let fetch_btn: Element<'_, Message> = if is_fetching {
            button(cjk_text(l.fetch_btn).size(14))
                .padding(10)
                .width(Length::Fixed(150.0))
                .into()
        } else {
            button(cjk_text(l.fetch_btn).size(14))
                .padding(10)
                .width(Length::Fixed(150.0))
                .on_press(Message::Fetch)
                .into()
        };
        col = col.push(fetch_btn);

        let status_text = match &self.status {
            FetchStatus::Idle => l.status_idle,
            FetchStatus::ExtractingCookies => l.status_extracting,
            FetchStatus::Fetching => l.status_fetching,
            FetchStatus::Ready => l.status_ready,
        };
        col = col.push(cjk_text(status_text).size(12).color(iced::Color {
            r: 0.56, g: 0.56, b: 0.58, a: 1.0,
        }));

        col = col.push(
            checkbox(l.audio_only_cb, self.audio_only)
                .on_toggle(Message::AudioOnlyToggled)
                .size(16),
        );

        if self.audio_only {
            let fmts = [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Flac, AudioFormat::Opus];
            col = col.push(
                row![
                    cjk_text(l.audio_format_label).size(13),
                    pick_list(fmts, Some(self.audio_format), Message::AudioFormatChanged)
                        .padding(6),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            if !self.formats.is_empty() || matches!(self.status, FetchStatus::Ready) {
                let video_count = self.formats.iter().filter(|f| f.is_video).count();
                let audio_count = self.formats.iter().filter(|f| f.is_audio).count();

                let current_label = if self.selected_format_id == "best" {
                    l.best_quality.to_string()
                } else {
                    self.formats
                        .iter()
                        .find(|f| f.format_id == self.selected_format_id)
                        .map(|f| f.resolution.clone())
                        .unwrap_or_else(|| self.selected_format_id.clone())
                };

                col = col.push(
                    column![
                        row![
                            cjk_text(l.resolution_label).size(14),
                            cjk_text(format!("({} {} | {} audio)", video_count, l.format_count_label, audio_count))
                                .size(11)
                                .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        row![
                            button(cjk_text(format!("{current_label} ▼")).size(13))
                                .padding(10)
                                .width(Length::Fixed(200.0))
                                .on_press(Message::SelectFormat),
                        ]
                        .spacing(8),
                    ]
                    .spacing(6),
                );
            }
        }

        let can_download = matches!(self.status, FetchStatus::Ready) || !self.url.is_empty();
        let mut action_row = row![].spacing(12);
        action_row = action_row.push(
            button(cjk_text(l.download_now_btn).size(14))
                .padding(10)
                .on_press(Message::Download),
        );
        action_row = action_row.push(
            button(cjk_text(l.add_to_queue_btn).size(14))
                .padding(10)
                .on_press(Message::AddToQueue),
        );
        if can_download {
            col = col.push(action_row);
        }

        if !self.video_info_log.is_empty() {
            col = col.push(self.info_log_view(is_dark));
        }

        col = col.push(
            button(cjk_text(l.open_folder_btn).size(13))
                .padding(8)
                .on_press(Message::SaveDirChanged(self.save_dir.clone())),
        );

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn browser_row(&self) -> Column<'_, Message> {
        let l = self.lang();
        let browsers = [Browser::Chrome, Browser::Edge, Browser::Firefox];
        let mut r = row![].spacing(16).align_y(Alignment::Center);
        r = r.push(cjk_text(l.browser_label).size(13));
        for browser in browsers {
            let is_selected = browser == self.browser;
            let btn = button(cjk_text(browser.to_string()).size(13))
                .padding(6)
                .on_press(Message::BrowserChanged(browser));
            let btn = if is_selected {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                    })),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else {
                btn
            };
            r = r.push(btn);
        }
        column![].push(r)
    }

    fn info_log_view(&self, is_dark: bool) -> Column<'_, Message> {
        let l = self.lang();
        let bg = if is_dark {
            iced::Color { r: 0.13, g: 0.13, b: 0.13, a: 1.0 }
        } else {
            iced::Color { r: 0.95, g: 0.95, b: 0.97, a: 1.0 }
        };
        let fg = if is_dark {
            iced::Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 }
        } else {
            iced::Color { r: 0.11, g: 0.11, b: 0.12, a: 1.0 }
        };

        let content = self.video_info_log.join("\n");
        column![
            cjk_text(l.info_label).size(14),
            container(cjk_text(content).size(11).font(Font::MONOSPACE).color(fg))
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fixed(80.0))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                }),
        ]
        .spacing(4)
    }

    fn queue_view(&self, is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let mut col = Column::new().spacing(12).padding(8);
        col = col.push(cjk_text(l.queue_title).size(20));

        if self.tasks.is_empty() {
            col = col.push(
                cjk_text(l.queue_empty)
                    .size(13)
                    .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
            );
        } else {
            for task in &self.tasks {
                col = col.push(self.task_row(task, is_dark));
            }
        }

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn task_row(&self, task: &DownloadTask, is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let bg = if is_dark {
            iced::Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 }
        } else {
            iced::Color { r: 0.95, g: 0.95, b: 0.97, a: 1.0 }
        };

        let status_icon = match &task.status {
            TaskStatus::Queued => "[Q]",
            TaskStatus::Fetching | TaskStatus::Downloading => "[D]",
            TaskStatus::Done => "[OK]",
            TaskStatus::Cancelled => "[C]",
            TaskStatus::Failed(_) => "[X]",
        };

        let status_text = match &task.status {
            TaskStatus::Queued => l.status_queued.to_string(),
            TaskStatus::Fetching => l.status_fetching_info.to_string(),
            TaskStatus::Downloading => {
                format!("{} {:.0}% | {} | ETA: {}", l.status_downloading, task.progress * 100.0, task.speed, task.eta)
            }
            TaskStatus::Done => l.status_done.to_string(),
            TaskStatus::Cancelled => l.status_cancelled.to_string(),
            TaskStatus::Failed(e) => format!("{}: {e}", l.status_failed),
        };

        let title_text = if task.title.is_empty() || task.title.starts_with("Title: ") {
            task.title.strip_prefix("Title: ").unwrap_or(&task.title).to_string()
        } else {
            task.title.clone()
        };

        let action_btn: Element<'_, Message> = match &task.status {
            TaskStatus::Downloading | TaskStatus::Fetching => {
                button(cjk_text(l.cancel_btn).size(11))
                    .padding(4)
                    .on_press(Message::CancelTask(task.id))
                    .into()
            }
            TaskStatus::Done | TaskStatus::Cancelled | TaskStatus::Failed(_) | TaskStatus::Queued => {
                button(cjk_text(l.remove_btn).size(11))
                    .padding(4)
                    .on_press(Message::RemoveTask(task.id))
                    .into()
            }
        };

        let mut content = column![
            row![
                cjk_text(format!("{status_icon} {title_text}")).size(13),
                horizontal_space(),
                action_btn,
            ]
            .spacing(8),
            cjk_text(status_text).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
        ];

        if matches!(task.status, TaskStatus::Downloading) {
            content = content.push(progress_bar(0.0..=1.0, task.progress as f32).height(4));
        }

        container(content)
            .padding(12)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            })
            .into()
    }

    fn history_view(&self, _is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let mut col = Column::new().spacing(12).padding(8);
        col = col.push(cjk_text(l.history_title).size(20));

        col = col.push(
            text_input(l.history_search_hint, &self.history_search)
                .on_input(Message::HistorySearchChanged)
                .padding(8)
                .size(13),
        );

        let filtered = if self.history_search.is_empty() {
            self.history.clone()
        } else if let Some(mgr) = &self.history_mgr {
            mgr.search_entries(&self.history_search)
        } else {
            self.history.iter()
                .filter(|e| e.title.to_lowercase().contains(&self.history_search.to_lowercase())
                    || e.url.to_lowercase().contains(&self.history_search.to_lowercase()))
                .cloned()
                .collect()
        };

        if filtered.is_empty() {
            col = col.push(
                cjk_text(l.history_empty).size(13).color(iced::Color {
                    r: 0.56, g: 0.56, b: 0.58, a: 1.0,
                }),
            );
        } else {
            for entry in &filtered {
                col = col.push(
                    row![
                        column![
                            cjk_text(&entry.title).size(13),
                            cjk_text(format!("{} | {} | {}", entry.date, entry.format, entry.status))
                                .size(11)
                                .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
                        ],
                        horizontal_space(),
                    ]
                    .spacing(8),
                );
            }
        }

        if !filtered.is_empty() {
            col = col.push(
                button(cjk_text(l.history_clear_btn).size(13))
                    .padding(8)
                    .on_press(Message::HistoryClear),
            );
        }

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn settings_view(&self, is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let bg = if is_dark {
            iced::Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 }
        } else {
            iced::Color { r: 0.95, g: 0.95, b: 0.97, a: 1.0 }
        };

        fn card<'a>(content: Element<'a, Message>, bg: iced::Color) -> Element<'a, Message> {
            container(content)
                .padding(16)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .into()
        }

        let mut col = Column::new().spacing(16).padding(8);
        col = col.push(cjk_text(l.settings_title).size(20));

        col = col.push(card(
                row![
                    cjk_text(l.language_label).size(14),
                    button(cjk_text(self.language.label()).size(13))
                        .padding(8)
                        .on_press(Message::ToggleLanguage),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
                bg,
            ),
        );

        col = col.push(card(
                row![
                    cjk_text(l.theme_label).size(14),
                    button(cjk_text(self.theme.label()).size(13))
                        .padding(8)
                        .on_press(Message::ToggleTheme),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
                bg,
            ),
        );

        col = col.push(card(
                column![
                    cjk_text(l.download_dir_label).size(14),
                    text_input(l.download_dir_hint, &self.save_dir)
                        .on_input(Message::SaveDirChanged)
                        .padding(8),
                ]
                .spacing(8)
                .into(),
                bg,
            ),
        );

        col = col.push(card(
                column![
                    cjk_text(l.max_concurrent_label).size(14),
                    text_input("3", &self.max_concurrent.to_string())
                        .on_input(Message::MaxConcurrentChanged)
                        .padding(8),
                ]
                .spacing(8)
                .into(),
                bg,
            ),
        );

        col = col.push(card(
                checkbox(l.clipboard_cb, self.clipboard_monitor)
                    .on_toggle(Message::ClipboardToggled)
                    .size(16)
                    .into(),
                bg,
            ),
        );

        col = col.push(card(
                column![
                    checkbox(l.subtitles_cb, self.subtitles_enabled)
                        .on_toggle(Message::SubtitlesToggled)
                        .size(16),
                    row![
                        cjk_text(l.subtitle_langs_label).size(12),
                        text_input(l.subtitle_langs_hint, &self.subtitle_langs)
                            .on_input(Message::SubtitleLangsChanged)
                            .padding(6)
                            .size(12),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(8)
                .into(),
                bg,
            ),
        );

        let ffmpeg_status = if downloader::detect_ffmpeg() {
            l.ffmpeg_available
        } else {
            l.ffmpeg_missing
        };
        col = col.push(card(
                row![
                    cjk_text(l.ffmpeg_label).size(14),
                    cjk_text(ffmpeg_status).size(12),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
                bg,
            ),
        );

        col = col.push(card(
                {
                    let mut cfg_col = column![
                        button(cjk_text(l.save_config_btn).size(14))
                            .padding(10)
                            .on_press(Message::SaveConfig),
                    ]
                    .spacing(8);
                    if self.config_saved {
                        cfg_col = cfg_col.push(
                            cjk_text(l.config_saved).size(11).color(iced::Color {
                                r: 0.0, g: 0.6, b: 0.0, a: 1.0,
                            }),
                        );
                    }
                    cfg_col.into()
                },
                bg,
            ),
        );

        container(scrollable(col)).height(Length::Fill).into()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cookie_args_from_result(result: &CookieResult) -> (Option<String>, Option<String>) {
    if result.use_cookie_file {
        (Some(result.cookie_file.clone()), None)
    } else {
        (None, result.browser_native.clone())
    }
}

fn is_video_url(text: &str) -> bool {
    let patterns = [
        "youtube.com/watch",
        "youtu.be/",
        "youtube.com/shorts",
        "youtube.com/playlist",
        "youtube.com/live",
        "vimeo.com/",
        "bilibili.com/video",
        "bilibili.com/list",
        "dailymotion.com/video",
        "twitch.tv/videos",
        "twitter.com/",
        "x.com/",
        "tiktok.com/",
        "instagram.com/",
        "reddit.com/r/",
        "douyin.com/",
        "weibo.com/",
        "acfun.cn/v/",
    ];
    let lower = text.to_lowercase();
    patterns.iter().any(|p| lower.contains(*p))
}

fn format_speed(speed: Option<f64>) -> String {
    match speed {
        Some(s) if s > 1024.0 * 1024.0 => {
            format!("{:.1} MB/s", s / (1024.0 * 1024.0))
        }
        Some(s) => {
            format!("{:.1} KB/s", s / 1024.0)
        }
        None => String::new(),
    }
}

fn format_eta(eta: Option<u64>) -> String {
    match eta {
        Some(secs) => {
            let mins = secs / 60;
            let s = secs % 60;
            format!("{mins:02}:{s:02}")
        }
        None => String::new(),
    }
}
