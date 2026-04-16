use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use iced::widget::{
    button, canvas, checkbox, column, container, horizontal_space, pick_list, progress_bar, row,
    scrollable, text_input, vertical_space, Column,
};
use iced::widget::canvas::{Frame, Geometry, Path};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use tokio::io::AsyncBufReadExt;

use crate::downloader::{self, CookieResult, FormatInfo};
use crate::config::Config;
use crate::history::{HistoryEntry, HistoryManager};
use crate::ui::format_dialog::{FormatDialog, FormatFilter};

// ---------------------------------------------------------------------------
// CJK Font
// ---------------------------------------------------------------------------

/// Use the default font (sans-serif). cosmic-text will automatically fall back
/// to Microsoft YaHei for CJK and Segoe UI Emoji for emoji, since both are
/// loaded in main.rs via .font() calls.
fn cjk_text<'a>(content: impl std::fmt::Display) -> iced::widget::Text<'a, iced::Theme> {
    iced::widget::text(content.to_string())
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
    Downloads,
    History,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Download,
    Extractor,
    PostProcessing,
    Subtitle,
    Advanced,
    SponsorBlock,
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
    NoCookies,
}
impl std::fmt::Display for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Browser::Chrome => write!(f, "Chrome"),
            Browser::Edge => write!(f, "Edge"),
            Browser::Firefox => write!(f, "Firefox"),
            Browser::NoCookies => write!(f, "No cookies"),
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
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: u64,
    pub speed_history: std::collections::VecDeque<(f64, f64)>, // (elapsed_seconds, speed_bytes_per_sec)
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
    view_mode: ViewMode,
    settings_tab: SettingsTab,

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
    cookie_file_path: String, // Manual cookie file path override

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
    ask_each_time: bool, // "每次询问" mode
    pending_download: bool, // true when user clicked download while formats were empty

    config: Config,
    config_saved: bool,

    format_dialog: FormatDialog,
}

#[derive(Debug, Clone, PartialEq)]
enum FetchStatus {
    Idle,
    ExtractingCookies,
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
    CookieFilePathChanged(String),
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
    TaskProgress { id: u64, progress: f64, speed: Option<f64>, eta: Option<u64>, downloaded: u64, total: Option<u64> },

    SaveDirChanged(String),
    MaxConcurrentChanged(String),
    ClipboardToggled(bool),
    ClipboardCheck(String),

    SubtitlesToggled(bool),
    SubtitleLangsChanged(String),

    SelectFormat,
    FormatSelected(String),
    DownloadWithFormats(Vec<String>), // Multi-format download
    ToggleFormatSelection(String, bool),
    CloseFormatDialog,
    FormatDialogFilterChanged(FormatFilter),
    FormatDialogSearchChanged(String),
    TaskTitle { id: u64, title: String },
    AskEachTimeToggled(bool),

    // General settings
    OutputTemplateChanged(String),
    MergeFormatChanged(String),

    // Download settings
    ConcurrentFragmentsChanged(String),
    LimitRateChanged(String),
    ThrottledRateChanged(String),
    RetriesChanged(String),
    FileAccessRetriesChanged(String),
    DownloadArchiveChanged(String),
    AbortOnErrorToggled(bool),
    IgnoreErrorsToggled(bool),
    ContinueDownloadsToggled(bool),
    NoOverwritesToggled(bool),

    // Extractor settings
    ExtractorRetriesChanged(String),
    ExtractorArgsChanged(String),
    ForceGenericExtractorToggled(bool),
    AllowUnsafeUrlToggled(bool),
    ExtractFlatToggled(bool),
    ExternalDownloaderChanged(String),
    ExternalDownloaderArgsChanged(String),

    // Post-processing settings
    EmbedThumbnailToggled(bool),
    EmbedMetadataToggled(bool),
    EmbedSubsToggled(bool),
    KeepVideoToggled(bool),
    NoPostOverwritesToggled(bool),
    ConvertThumbnailsChanged(String),
    PostprocessorArgsChanged(String),

    // Advanced settings
    VerboseToggled(bool),
    UserAgentChanged(String),
    RefererChanged(String),
    ProxyChanged(String),
    GeoBypassToggled(bool),
    GeoBypassCountryChanged(String),
    GeoVerificationProxyChanged(String),
    CustomHeadersChanged(String),
    SleepIntervalChanged(String),
    MaxSleepIntervalChanged(String),
    PreferFreeFormatsToggled(bool),
    CheckFormatsToggled(bool),
    SimulateToggled(bool),

    // SponsorBlock settings
    SponsorblockRemoveChanged(String),
    SponsorblockApiChanged(String),

    HistorySearchChanged(String),
    HistoryClear,

    ToggleViewMode,
    Tick,
    SettingsTabChanged(SettingsTab),

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
            view_mode: ViewMode::default(),
            settings_tab: SettingsTab::default(),
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
            cookie_file_path: String::new(),
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
            ask_each_time: config.defaults.ask_each_time,
            config,
            config_saved: false,
            format_dialog: FormatDialog::default(),
            pending_download: false,
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
            Message::CookieFilePathChanged(path) => {
                self.cookie_file_path = path;
            }
            Message::Fetch => {
                if self.url.trim().is_empty() {
                    return Task::none();
                }
                self.video_info_log.clear();
                self.formats.clear();

                let url = self.url.clone();
                let browser_name = self.browser.to_string();
                let cookie_file = PathBuf::from(&self.cookie_file);
                let no_cookies = matches!(self.browser, Browser::NoCookies);
                let manual_cookie_path = self.cookie_file_path.clone();

                if !no_cookies && manual_cookie_path.is_empty() {
                    self.status = FetchStatus::ExtractingCookies;
                }

                return Task::perform(
                    async move {
                        if no_cookies {
                            // Skip cookie extraction entirely
                            let cookie_result = downloader::CookieResult {
                                cookie_file: String::new(),
                                use_cookie_file: false,
                                browser_native: None,
                                message: "No cookies - downloading without authentication".to_string(),
                            };
                            let info = downloader::fetch_info(&url, &[]).await;
                            match info {
                                Ok(v) => Ok((v, cookie_result)),
                                Err(e) => Err((e.to_string(), cookie_result)),
                            }
                        } else if !manual_cookie_path.is_empty()
                            && std::path::Path::new(&manual_cookie_path).exists()
                        {
                            // Use manual cookie file path
                            let cookie_result = downloader::CookieResult {
                                cookie_file: manual_cookie_path.clone(),
                                use_cookie_file: true,
                                browser_native: None,
                                message: format!("Using cookies from: {manual_cookie_path}"),
                            };
                            let cookie_args = downloader::cookie_args(
                                Some(&manual_cookie_path),
                                None,
                            );
                            let info = downloader::fetch_info(&url, &cookie_args).await;
                            match info {
                                Ok(v) => Ok((v, cookie_result)),
                                Err(e) => Err((e.to_string(), cookie_result)),
                            }
                        } else {
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

                // If download was pending, open format dialog now
                if self.pending_download {
                    self.pending_download = false;
                    self.format_dialog.open(&self.selected_format_id, true);
                }
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

                // "Ask each time" mode: open format dialog instead of direct download
                if self.ask_each_time && matches!(message, Message::Download) {
                    if self.formats.is_empty() {
                        // Need to fetch formats first
                        self.pending_download = true;
                        return self.update(Message::Fetch);
                    } else {
                        self.format_dialog.open(&self.selected_format_id, true);
                        return Task::none();
                    }
                }

                // Queue button in "ask each time" mode → skip
                if self.ask_each_time && matches!(message, Message::AddToQueue) {
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

                // Try to get title from video_info_log, fall back to URL
                // (will be updated by title extraction in start_download)
                let initial_title = self.video_info_log.iter()
                    .find(|l| l.starts_with("Title: "))
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| format!("Title: {}", self.url));

                let task = DownloadTask {
                    id,
                    url: self.url.clone(),
                    title: initial_title,
                    format: format_label,
                    audio_only: self.audio_only,
                    status,
                    progress: 0.0,
                    speed: String::new(),
                    eta: String::new(),
                    log: Vec::new(),
                    downloaded_bytes: 0,
                    total_bytes: None,
                    speed_bytes: None,
                    eta_seconds: None,
                    elapsed_seconds: 0,
                    speed_history: std::collections::VecDeque::new(),
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
                        if let Some(speed) = prog.speed {
                            task.speed_bytes = Some(speed);
                        }
                        task.eta_seconds = prog.eta;
                        if let Some(dl) = prog.downloaded {
                            task.downloaded_bytes = dl;
                        }
                        if let Some(total) = prog.total {
                            task.total_bytes = Some(total);
                        }
                    }
                }
            }
            Message::TaskProgress { id, progress, speed, eta, downloaded, total } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.progress = progress;
                    task.downloaded_bytes = downloaded;
                    if let Some(t) = total {
                        task.total_bytes = Some(t);
                    }
                    if let Some(s) = speed {
                        task.speed = format_speed(Some(s));
                        task.speed_bytes = Some(s);
                        // Record speed sample (keep last 60 samples = ~12 seconds at 200ms poll)
                        task.speed_history.push_back((task.elapsed_seconds as f64, s));
                        if task.speed_history.len() > 60 {
                            task.speed_history.pop_front();
                        }
                    }
                    if let Some(e) = eta {
                        task.eta = format_eta(Some(e));
                        task.eta_seconds = Some(e);
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
                self.format_dialog.open(&current, self.ask_each_time);
            }
            Message::FormatSelected(format_id) => {
                self.selected_format_id = format_id;
                self.format_dialog.close();
            }
            Message::DownloadWithFormats(format_ids) => {
                if self.url.trim().is_empty() || format_ids.is_empty() {
                    self.format_dialog.close();
                    return Task::none();
                }
                self.format_dialog.close();

                let title = self.video_info_log.iter()
                    .find(|l| l.starts_with("Title: "))
                    .map(|l| l.strip_prefix("Title: ").unwrap_or(l).to_string())
                    .unwrap_or_else(|| self.url.clone());

                let mut task_ids = Vec::new();
                for format_id in &format_ids {
                    let id = self.next_task_id;
                    self.next_task_id += 1;

                    let format_label = if *format_id == "best" {
                        "best".to_string()
                    } else {
                        self.formats
                            .iter()
                            .find(|f| f.format_id == *format_id)
                            .map(|f| f.resolution.clone())
                            .unwrap_or_else(|| format_id.clone())
                    };

                    let task = DownloadTask {
                        id,
                        url: self.url.clone(),
                        title: format!("Title: {title}"),
                        format: format_label,
                        audio_only: self.audio_only,
                        status: TaskStatus::Downloading,
                        progress: 0.0,
                        speed: String::new(),
                        eta: String::new(),
                        log: Vec::new(),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        speed_bytes: None,
                        eta_seconds: None,
                        elapsed_seconds: 0,
                        speed_history: std::collections::VecDeque::new(),
                    };
                    self.tasks.push(task);
                    task_ids.push(id);
                }

                // Start all downloads
                let download_tasks: Vec<Task<Message>> = task_ids
                    .into_iter()
                    .map(|id| self.start_download(id))
                    .collect();
                return Task::batch(download_tasks);
            }
            Message::ToggleFormatSelection(format_id, selected) => {
                if selected {
                    self.format_dialog.selected_formats.insert(format_id);
                } else {
                    self.format_dialog.selected_formats.remove(&format_id);
                }
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
            Message::TaskTitle { id, title } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.title = format!("Title: {title}");
                }
            }
            Message::AskEachTimeToggled(val) => {
                self.ask_each_time = val;
                self.config.defaults.ask_each_time = val;
                self.config.defaults.video_quality = if val { "ask".to_string() } else { "best".to_string() };
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
            Message::ToggleViewMode => {
                self.view_mode = match self.view_mode {
                    ViewMode::List => ViewMode::Log,
                    ViewMode::Log => ViewMode::List,
                };
            }
            Message::Tick => {
                for task in &mut self.tasks {
                    if matches!(task.status, TaskStatus::Downloading) {
                        task.elapsed_seconds += 1;
                    }
                }
            }
            Message::SettingsTabChanged(tab) => {
                self.settings_tab = tab;
            }

            // General settings handlers
            Message::OutputTemplateChanged(t) => {
                self.config.general.output_template = t;
            }
            Message::MergeFormatChanged(fmt) => {
                self.config.general.merge_output_format = fmt;
            }

            // Download settings handlers
            Message::ConcurrentFragmentsChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n > 0 && n <= 32 {
                    self.config.download.concurrent_fragments = n;
                }
            }
            Message::LimitRateChanged(v) => { self.config.download.limit_rate = v; }
            Message::ThrottledRateChanged(v) => { self.config.download.throttled_rate = v; }
            Message::RetriesChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n <= 100 {
                    self.config.download.retries = n;
                }
            }
            Message::FileAccessRetriesChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n <= 20 {
                    self.config.download.file_access_retries = n;
                }
            }
            Message::DownloadArchiveChanged(v) => { self.config.download.download_archive = v; }
            Message::AbortOnErrorToggled(v) => { self.config.download.abort_on_error = v; }
            Message::IgnoreErrorsToggled(v) => { self.config.download.ignore_errors = v; }
            Message::ContinueDownloadsToggled(v) => { self.config.download.continue_downloads = v; }
            Message::NoOverwritesToggled(v) => { self.config.download.no_overwrites = v; }

            // Extractor settings handlers
            Message::ExtractorRetriesChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n <= 20 {
                    self.config.extractor.extractor_retries = n;
                }
            }
            Message::ExtractorArgsChanged(v) => {
                self.config.extractor.extractor_args = v.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            Message::ForceGenericExtractorToggled(v) => { self.config.extractor.force_generic_extractor = v; }
            Message::AllowUnsafeUrlToggled(v) => { self.config.extractor.allow_unsafe_url = v; }
            Message::ExtractFlatToggled(v) => { self.config.extractor.extract_flat = v; }
            Message::ExternalDownloaderChanged(v) => { self.config.extractor.external_downloader = v; }
            Message::ExternalDownloaderArgsChanged(v) => { self.config.extractor.external_downloader_args = v; }

            // Post-processing settings handlers
            Message::EmbedThumbnailToggled(v) => { self.config.post_processing.embed_thumbnail = v; }
            Message::EmbedMetadataToggled(v) => { self.config.post_processing.embed_metadata = v; }
            Message::EmbedSubsToggled(v) => { self.config.post_processing.embed_subs = v; }
            Message::KeepVideoToggled(v) => { self.config.post_processing.keep_video = v; }
            Message::NoPostOverwritesToggled(v) => { self.config.post_processing.no_post_overwrites = v; }
            Message::ConvertThumbnailsChanged(v) => { self.config.post_processing.convert_thumbnails = v; }
            Message::PostprocessorArgsChanged(v) => {
                self.config.post_processing.postprocessor_args = v.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }

            // Advanced settings handlers
            Message::VerboseToggled(v) => { self.config.advanced.verbose = v; }
            Message::UserAgentChanged(v) => { self.config.advanced.user_agent = v; }
            Message::RefererChanged(v) => { self.config.advanced.referer = v; }
            Message::ProxyChanged(v) => { self.config.advanced.proxy = v; }
            Message::GeoBypassToggled(v) => { self.config.advanced.geo_bypass = v; }
            Message::GeoBypassCountryChanged(v) => { self.config.advanced.geo_bypass_country = v; }
            Message::GeoVerificationProxyChanged(v) => { self.config.advanced.geo_verification_proxy = v; }
            Message::CustomHeadersChanged(v) => {
                self.config.advanced.custom_headers = v.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            Message::SleepIntervalChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n <= 3600 {
                    self.config.advanced.sleep_interval = n;
                }
            }
            Message::MaxSleepIntervalChanged(v) => {
                if let Ok(n) = v.parse::<usize>() && n <= 3600 {
                    self.config.advanced.max_sleep_interval = n;
                }
            }
            Message::PreferFreeFormatsToggled(v) => { self.config.advanced.prefer_free_formats = v; }
            Message::CheckFormatsToggled(v) => { self.config.advanced.check_formats = v; }
            Message::SimulateToggled(v) => { self.config.advanced.simulate = v; }

            // SponsorBlock settings handlers
            Message::SponsorblockRemoveChanged(v) => { self.config.post_processing.sponsorblock_remove = v; }
            Message::SponsorblockApiChanged(v) => { self.config.post_processing.sponsorblock_api = v; }

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

    // -----------------------------------------------------------------------
    // Helper: parse yt-dlp --progress-template output
    // Format: [YTDL_PROGRESS] <pct>\t<speed>\t<eta>\t<dl_bytes>\t<total_bytes>
    // Values can be "N/A" or numeric (pct may have % suffix like "50.0%")
    // -----------------------------------------------------------------------
    fn parse_ytdl_progress_line(line: &str) -> Option<(f64, Option<f64>, Option<u64>, Option<u64>, Option<u64>)> {
        let rest = line.strip_prefix("[YTDL_PROGRESS] ")?;
        let parts: Vec<&str> = rest.split('\t').collect();
        if parts.len() < 5 {
            return None;
        }

        // Parse percentage: can be "N/A" or "50.0%" or "50.0"
        let pct_str = parts[0].trim();
        let pct = if pct_str == "N/A" {
            None
        } else {
            let numeric = pct_str.trim_end_matches('%');
            numeric.parse::<f64>().ok().map(|v| v / 100.0)
        }?;

        // Parse speed: "N/A" or numeric bytes/sec
        let speed = if parts[1].trim() == "N/A" {
            None
        } else {
            parts[1].trim().parse::<f64>().ok()
        };

        // Parse ETA: "N/A" or numeric seconds
        let eta = if parts[2].trim() == "N/A" {
            None
        } else {
            parts[2].trim().parse::<u64>().ok()
        };

        // Parse downloaded bytes
        let downloaded = if parts[3].trim() == "N/A" {
            0
        } else {
            parts[3].trim().parse::<u64>().unwrap_or(0)
        };

        // Parse total bytes
        let total = if parts[4].trim() == "N/A" {
            None
        } else {
            parts[4].trim().parse::<u64>().ok()
        };

        Some((pct, speed, eta, Some(downloaded), total))
    }

    fn start_download(&self, id: u64) -> Task<Message> {
        let url = self.url.clone();
        let format_id = self.selected_format_id.clone();
        let save_dir = self.save_dir.clone();
        let audio_only = self.audio_only;
        let audio_format = self.audio_format.to_string();
        let no_cookies = matches!(self.browser, Browser::NoCookies);
        let cookie_file = self.cookie_file.clone();
        let cookie_result = self.cookie_result.clone();
        let subtitles_enabled = self.subtitles_enabled;
        let subtitle_langs = self.subtitle_langs.clone();
        let config = self.config.clone();
        let manual_cookie_path = self.cookie_file_path.clone();

        // Get total file size from selected format for progress calculation.
        // Use approx_total_size (includes filesize_approx fallback) instead of exact filesize.
        let total_bytes: Option<u64> = if self.selected_format_id == "best" {
            // "best" is a yt-dlp keyword, not an actual format_id. Estimate using
            // the largest available video format's approx_total_size.
            self.formats
                .iter()
                .filter(|f| f.is_video)
                .filter_map(|f| f.approx_total_size)
                .max()
        } else {
            self.formats
                .iter()
                .find(|f| f.format_id == self.selected_format_id)
                .and_then(|f| f.approx_total_size)
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut flags = self.cancel_flags.lock().unwrap();
            flags.insert(id, cancel_flag.clone());
        }

        Task::stream(iced::stream::channel(256, move |mut output| {
            let url = url.clone();
            let format_id = format_id.clone();
            let save_dir = save_dir.clone();
            let cookie_file = cookie_file.clone();
            let cookie_result = cookie_result.clone();
            let no_cookies = no_cookies;
            let manual_cookie_path = manual_cookie_path.clone();
            let subtitle_langs = subtitle_langs.clone();
            let cancel_flag = cancel_flag.clone();
            let config = config.clone();

            async move {
                let cookie_args: Vec<String> = if no_cookies {
                    Vec::new()
                } else if !manual_cookie_path.is_empty()
                    && std::path::Path::new(&manual_cookie_path).exists()
                {
                    downloader::cookie_args(Some(&manual_cookie_path), None)
                } else {
                    let (cookie_file_opt, browser_native_opt) = cookie_result
                        .as_ref()
                        .map(|cr| cookie_args_from_result(cr))
                        .unwrap_or_else(|| {
                            let path = PathBuf::from(&cookie_file);
                            let cr = downloader::extract_browser_cookies("Chrome", &path);
                            cookie_args_from_result(&cr)
                        });
                    downloader::cookie_args(cookie_file_opt.as_deref(), browser_native_opt.as_deref())
                };

                let format_str = if audio_only {
                    "bestaudio/best".to_string()
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

                // Title was already extracted from --dump-json (UTF-8 JSON).
                // Don't use --print title subprocess as it outputs GBK on Windows,
                // causing garbled Chinese characters when decoded as UTF-8.

                let output_template = if config.general.output_template.is_empty() {
                    format!("{save_dir}/%(title)s [%(id)s].%(ext)s")
                } else {
                    format!("{save_dir}/{}", config.general.output_template)
                };

                let mut cmd = tokio::process::Command::new(&yt_dlp[0]);

                // When yt-dlp runs via Python, stdout is block-buffered when piped.
                // This causes all progress lines to arrive at once when the download finishes.
                // Setting PYTHONUNBUFFERED=1 forces line-buffered output for real-time progress.
                // This also helps PyInstaller-bundled yt-dlp.exe which embeds Python.
                cmd.env("PYTHONUNBUFFERED", "1");

                cmd.args(&yt_dlp[1..])
                    .arg("--newline")
                    .arg("--progress")
                    .arg("-o")
                    .arg(&output_template);

                if !config.general.merge_output_format.is_empty() {
                    cmd.arg("--merge-output-format").arg(&config.general.merge_output_format);
                }

                // Download config options
                if config.download.concurrent_fragments > 1 {
                    cmd.arg("--concurrent-fragments").arg(config.download.concurrent_fragments.to_string());
                }
                if !config.download.limit_rate.is_empty() {
                    cmd.arg("--limit-rate").arg(&config.download.limit_rate);
                }
                if !config.download.throttled_rate.is_empty() {
                    cmd.arg("--throttled-rate").arg(&config.download.throttled_rate);
                }
                if config.download.retries > 0 {
                    cmd.arg("--retries").arg(config.download.retries.to_string());
                }
                if config.download.file_access_retries > 0 {
                    cmd.arg("--file-access-retries").arg(config.download.file_access_retries.to_string());
                }
                if !config.download.download_archive.is_empty() {
                    cmd.arg("--download-archive").arg(&config.download.download_archive);
                }
                if config.download.abort_on_error {
                    cmd.arg("--abort-on-error");
                }
                if config.download.ignore_errors {
                    cmd.arg("--ignore-errors");
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

                // Extractor config options
                if config.extractor.extractor_retries > 0 {
                    cmd.arg("--extractor-retries").arg(config.extractor.extractor_retries.to_string());
                }
                for arg in &config.extractor.extractor_args {
                    cmd.arg("--extractor-args").arg(arg);
                }
                if config.extractor.force_generic_extractor {
                    cmd.arg("--force-generic-extractor");
                }
                if config.extractor.allow_unsafe_url {
                    cmd.arg("--allow-unsafe-url");
                }
                if config.extractor.extract_flat {
                    cmd.arg("--extract-flat");
                }
                if !config.extractor.external_downloader.is_empty() {
                    cmd.arg("--external-downloader").arg(&config.extractor.external_downloader);
                }
                if !config.extractor.external_downloader_args.is_empty() {
                    cmd.arg("--external-downloader-args").arg(&config.extractor.external_downloader_args);
                }

                // Advanced config options
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
                    cmd.arg("--geo-bypass-country").arg(&config.advanced.geo_bypass_country);
                }
                if !config.advanced.geo_verification_proxy.is_empty() {
                    cmd.arg("--geo-verification-proxy").arg(&config.advanced.geo_verification_proxy);
                }
                for header in &config.advanced.custom_headers {
                    cmd.arg("--add-header").arg(header);
                }
                if config.advanced.sleep_interval > 0 {
                    cmd.arg("--sleep-interval").arg(config.advanced.sleep_interval.to_string());
                }
                if config.advanced.max_sleep_interval > 0 {
                    cmd.arg("--max-sleep-interval").arg(config.advanced.max_sleep_interval.to_string());
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

                // Post-processing options
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
                    cmd.arg("--convert-thumbnails").arg(&config.post_processing.convert_thumbnails);
                }
                for arg in &config.post_processing.postprocessor_args {
                    cmd.arg("--postprocessor-args").arg(arg);
                }

                // SponsorBlock
                if !config.post_processing.sponsorblock_remove.is_empty() {
                    cmd.arg("--sponsorblock-remove").arg(&config.post_processing.sponsorblock_remove);
                }
                if !config.post_processing.sponsorblock_api.is_empty() {
                    cmd.arg("--sponsorblock-api").arg(&config.post_processing.sponsorblock_api);
                }

                // Format and audio
                if audio_only {
                    cmd.arg("-x").arg("--audio-format").arg(&audio_format);
                } else {
                    cmd.arg("-f").arg(&format_str);
                }

                // Subtitles
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

                match cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn() {
                    Ok(mut child) => {
                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();
                        let stdout_reader = tokio::io::BufReader::new(stdout);
                        let stderr_reader = tokio::io::BufReader::new(stderr);
                        let mut stdout_lines = stdout_reader.lines();
                        let mut stderr_lines = stderr_reader.lines();

                        let mut stdout_done = false;
                        let mut stderr_done = false;

                        // File size monitor: yt-dlp's stdout is block-buffered on Windows,
                        // so progress lines arrive all at once. We monitor the .part file
                        // size on disk to get real-time progress updates.
                        let monitor_save_dir = save_dir.clone();
                        let mut monitor_output = output.clone();
                        let monitor_cancel = cancel_flag.clone();
                        let monitor_id = id;
                        let monitor_total = total_bytes;
                        let monitor_handle = tokio::spawn(async move {
                            let mut prev_downloaded: u64 = 0;
                            let mut last_speed: Option<f64> = None;
                            let mut last_check: Option<std::time::Instant> = None;
                            let poll_interval = tokio::time::Duration::from_millis(200);
                            let mut send_count: u64 = 0;

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
                                        if let Some(ext) = path.extension()
                                            && ext == "part" {
                                                if let Ok(meta) = path.metadata() {
                                                    total_size += meta.len();
                                                    if let Ok(m) = meta.modified() {
                                                        if m > newest_mtime {
                                                            newest_mtime = m;
                                                        }
                                                    }
                                                }
                                            }
                                    }

                                    if total_size > 0 {
                                        let now = std::time::Instant::now();
                                        let elapsed = last_check.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);

                                        // Always calculate speed
                                        let speed = if elapsed > 0.05 && total_size > prev_downloaded {
                                            let delta = total_size - prev_downloaded;
                                            let s = delta as f64 / elapsed;
                                            Some(s)
                                        } else {
                                            last_speed
                                        };

                                        if speed.is_some() {
                                            last_speed = speed;
                                        }

                                        // Calculate ETA
                                        let eta = speed.and_then(|s| if s > 0.0 { Some((total_size as f64 / s) as u64) } else { None });

                                        // Calculate progress
                                        let (progress, total) = if let Some(t) = monitor_total {
                                            if t > 0 {
                                                ((total_size as f64 / t as f64).min(1.0), Some(t))
                                            } else {
                                                (0.0, Some(total_size))
                                            }
                                        } else {
                                            // Unknown total: use downloaded as total for display purposes
                                            // Progress will show 100% but that's OK since we don't know
                                            (if total_size > 0 { 0.01 * (send_count % 100) as f64 / 100.0 } else { 0.0 }, None)
                                        };

                                        // ALWAYS send update when we have data (not just on size change)
                                        let _ = monitor_output.try_send(Message::TaskProgress {
                                            id: monitor_id,
                                            progress,
                                            speed,
                                            eta,
                                            downloaded: total_size,
                                            total,
                                        });

                                        send_count += 1;
                                        prev_downloaded = total_size;
                                        last_check = Some(now);
                                    }
                                }

                                tokio::time::sleep(poll_interval).await;
                            }
                        });

                        loop {
                            if cancel_flag.load(Ordering::SeqCst) {
                                let _ = child.kill().await;
                                monitor_handle.abort();
                                let _ = monitor_handle.await;
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

                            if stdout_done && stderr_done {
                                break;
                            }

                            tokio::select! {
                                line = stdout_lines.next_line(), if !stdout_done => {
                                    match line {
                                        Ok(Some(line)) => {
                                            let trimmed = line.trim_end().to_owned();
                                            if !trimmed.is_empty() {
                                                // Try to parse as yt-dlp progress template first
                                                if trimmed.starts_with("[YTDL_PROGRESS]") {
                                                    if let Some((pct, speed, eta, downloaded, total)) =
                                                        App::parse_ytdl_progress_line(&trimmed)
                                                    {
                                                        let _ = output.try_send(Message::TaskProgress {
                                                            id,
                                                            progress: pct,
                                                            speed,
                                                            eta,
                                                            downloaded: downloaded.unwrap_or(0),
                                                            total,
                                                        });
                                                    }
                                                }
                                                // Fall back to raw log line
                                                log_lines.push(trimmed.clone());
                                                let _ = output.try_send(Message::TaskLog { id, line: trimmed });
                                            }
                                        }
                                        Ok(None) | Err(_) => { stdout_done = true; }
                                    }
                                }
                                line = stderr_lines.next_line(), if !stderr_done => {
                                    match line {
                                        Ok(Some(line)) => {
                                            let trimmed = line.trim_end().to_owned();
                                            if !trimmed.is_empty() {
                                                log_lines.push(trimmed.clone());
                                                let _ = output.try_send(Message::TaskLog { id, line: trimmed });
                                            }
                                        }
                                        Ok(None) | Err(_) => { stderr_done = true; }
                                    }
                                }
                                // If one stream is done, the other may still have data
                                else => {
                                    // Both conditions are false, meaning both are done
                                    break;
                                }
                            }
                        }

                        // Drain remaining stderr
                        while let Ok(Some(line)) = stderr_lines.next_line().await {
                            let trimmed = line.trim_end().to_owned();
                            if !trimmed.is_empty() {
                                log_lines.push(trimmed.clone());
                            }
                        }

                        // Stop the file size monitor
                        monitor_handle.abort();
                        let _ = monitor_handle.await;

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
        let mut subs = Vec::new();

        // Tick every second for elapsed time updates
        subs.push(
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick),
        );

        // Clipboard monitoring
        if self.clipboard_monitor {
            subs.push(
                iced::time::every(std::time::Duration::from_secs(2)).map(|_| {
                    Message::ClipboardCheck(
                        arboard::Clipboard::new()
                            .ok()
                            .and_then(|mut c| c.get_text().ok())
                            .unwrap_or_default(),
                    )
                }),
            );
        }

        Subscription::batch(subs)
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    fn page_label(&self, page: Page) -> &'static str {
        let l = self.lang();
        match page {
            Page::Downloads => l.sidebar_download,
            Page::History => l.sidebar_history,
            Page::Settings => l.sidebar_settings,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let is_dark = matches!(self.theme, ThemeMode::Dark);

        let content = match self.page {
            Page::Downloads => self.downloads_view(),
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
            let dialog = if self.format_dialog.download_mode {
                // Download mode: multi-select with download callback
                self.format_dialog.view(
                    &self.formats,
                    Message::FormatSelected,
                    || Message::CloseFormatDialog,
                    Message::FormatDialogFilterChanged,
                    Message::FormatDialogSearchChanged,
                    Some(Message::DownloadWithFormats),
                    Some(Message::ToggleFormatSelection),
                )
            } else {
                // Normal mode: single select, no download callback
                self.format_dialog.view(
                    &self.formats,
                    Message::FormatSelected,
                    || Message::CloseFormatDialog,
                    Message::FormatDialogFilterChanged,
                    Message::FormatDialogSearchChanged,
                    Option::<fn(Vec<String>) -> Message>::None,
                    Option::<fn(String, bool) -> Message>::None,
                )
            };

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

        let pages = [Page::Downloads, Page::History, Page::Settings];
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

    // --- Stacher7-style combined downloads view ---

    fn downloads_view(&self) -> Element<'_, Message> {
        let is_dark = matches!(self.theme, ThemeMode::Dark);
        let card_bg = if is_dark {
            iced::Color { r: 0.18, g: 0.18, b: 0.18, a: 1.0 }
        } else {
            iced::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
        };

        fn card<'a>(content: Element<'a, Message>, bg: iced::Color) -> Element<'a, Message> {
            container(content)
                .padding(14)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .into()
        }

        // --- Top toolbar ---
        let l = self.lang();

        // Browser selector row
        let browser_row = row![
            cjk_text(l.browser_label).size(13),
            pick_list(
                [Browser::Chrome, Browser::Edge, Browser::Firefox, Browser::NoCookies].as_slice(),
                Some(self.browser.clone()),
                Message::BrowserChanged,
            )
            .padding(6),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // Manual cookie file path input (for when auto-extraction fails)
        let cookie_input = row![
            cjk_text("Cookies 文件:").size(11)
                .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
            text_input("可选：手动指定 Cookies 文件路径", &self.cookie_file_path)
                .on_input(Message::CookieFilePathChanged)
                .padding(6)
                .size(12)
                .width(Length::Fixed(300.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let url_input = text_input(l.video_url_hint, &self.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Fetch)
            .padding(10)
            .size(14);

        let is_fetching = !matches!(self.status, FetchStatus::Idle | FetchStatus::Ready);

        // Format button and top-row content
        let format_btn_content: Element<'_, Message> = if self.selected_format_id == "best" {
            cjk_text(format!("{}: {}", l.best_quality, l.format_count_label)).size(13).into()
        } else {
            cjk_text(&self.selected_format_id).size(13).into()
        };
        let format_btn = button(format_btn_content)
            .padding(8)
            .on_press(Message::SelectFormat);

        let dl_btn: Element<'_, Message> = if is_fetching {
            button(cjk_text(l.download_now_btn).size(14)).padding(10).into()
        } else {
            button(cjk_text(l.download_now_btn).size(14))
                .padding(10)
                .on_press(Message::Download)
                .into()
        };

        let top_toolbar = if self.ask_each_time {
            // "Ask each time" mode: just URL input + download button
            // Format dialog is triggered by clicking download
            let ask_dl_btn: Element<'_, Message> = if is_fetching {
                button(cjk_text(l.download_now_btn).size(14)).padding(10).into()
            } else {
                button(cjk_text(l.download_now_btn).size(14))
                    .padding(10)
                    .on_press(Message::Download)
                    .into()
            };
            column![
                browser_row,
                cookie_input,
                row![url_input, ask_dl_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
            ]
            .spacing(6)
        } else {
            // Normal mode: URL + format button + download + queue
            let queue_btn: Element<'_, Message> = if is_fetching {
                button(cjk_text(l.add_to_queue_btn).size(14)).padding(10).into()
            } else {
                button(cjk_text(l.add_to_queue_btn).size(14))
                    .padding(10)
                    .on_press(Message::AddToQueue)
                    .into()
            };
            column![
                browser_row,
                cookie_input,
                row![url_input, format_btn, dl_btn, queue_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
            ]
            .spacing(6)
        };

        // --- Second row: view toggle + download path ---
        let view_toggle_label = match self.view_mode {
            ViewMode::List => "\u{E00B}", // list icon unicode
            ViewMode::Log => "\u{2318}", // terminal icon unicode
        };
        let view_toggle_btn = button(cjk_text(view_toggle_label).size(14))
            .padding(8)
            .on_press(Message::ToggleViewMode);
        let path_display = cjk_text(format!("{}: {}", l.download_dir_label, self.save_dir))
            .size(11)
            .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 });

        let second_row = row![view_toggle_btn, path_display].spacing(12).align_y(Alignment::Center);

        // --- Main content ---
        let main_content = match self.view_mode {
            ViewMode::List => self.downloads_list_view(is_dark),
            ViewMode::Log => self.downloads_log_view(is_dark),
        };

        let mut col = column![card(top_toolbar.into(), card_bg), second_row, main_content].spacing(10);

        // --- Bottom global progress bar ---
        if !self.tasks.is_empty() {
            let active: Vec<&DownloadTask> = self.tasks.iter()
                .filter(|t| matches!(t.status, TaskStatus::Downloading)).collect();
            if !active.is_empty() {
                let avg = active.iter().map(|t| t.progress).sum::<f64>() / active.len() as f64;
                col = col.push(progress_bar(0.0..=1.0, avg as f32).height(6));
            }
        }

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn downloads_list_view(&self, is_dark: bool) -> Element<'_, Message> {
        let bg = if is_dark {
            iced::Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 }
        } else {
            iced::Color { r: 0.98, g: 0.98, b: 1.0, a: 1.0 }
        };
        let header_fg = if is_dark {
            iced::Color { r: 0.7, g: 0.7, b: 0.7, a: 1.0 }
        } else {
            iced::Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 }
        };

        let l = self.lang();

        // Column headers
        let headers = row![
            cjk_text("\u{26A1}").size(11).color(header_fg).width(Length::Fixed(32.0)),
            cjk_text("标题").size(11).color(header_fg).width(Length::Fill),
            cjk_text("进度").size(11).color(header_fg).width(Length::Fixed(140.0)),
            cjk_text("总计").size(11).color(header_fg).width(Length::Fixed(80.0)),
            cjk_text("速度").size(11).color(header_fg).width(Length::Fixed(90.0)),
            cjk_text("ETA").size(11).color(header_fg).width(Length::Fixed(60.0)),
            cjk_text("已用").size(11).color(header_fg).width(Length::Fixed(60.0)),
            cjk_text("操作").size(11).color(header_fg).width(Length::Fixed(60.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([4, 12]);

        let header_bg = if is_dark {
            iced::Color { r: 0.18, g: 0.18, b: 0.18, a: 1.0 }
        } else {
            iced::Color { r: 0.92, g: 0.92, b: 0.95, a: 1.0 }
        };

        let mut list = column![container(headers).width(Length::Fill).style(move |_| container::Style {
            background: Some(iced::Background::Color(header_bg)),
            border: iced::border::rounded(4),
            ..Default::default()
        })];

        if self.tasks.is_empty() {
            let empty = cjk_text(l.queue_empty).size(13)
                .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 });
            list = list.push(container(empty).padding(40).width(Length::Fill));
        } else {
            for task in &self.tasks {
                list = list.push(self.download_row(task, is_dark));
            }
        }

        container(list)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            })
            .into()
    }

    fn download_row(&self, task: &DownloadTask, is_dark: bool) -> Element<'_, Message> {
        let l = self.lang();
        let row_bg = if is_dark {
            iced::Color { r: 0.20, g: 0.20, b: 0.20, a: 1.0 }
        } else {
            iced::Color { r: 0.96, g: 0.96, b: 0.98, a: 1.0 }
        };

        let status_icon = match &task.status {
            TaskStatus::Queued => "\u{23F3}",  // hourglass
            TaskStatus::Fetching => "\u{27A4}",  // arrow
            TaskStatus::Downloading => "\u{2BC0}", // download arrow
            TaskStatus::Done => "\u{2714}",   // check
            TaskStatus::Cancelled => "\u{2718}", // x
            TaskStatus::Failed(_) => "\u{2718}",
        };
        let status_color = match &task.status {
            TaskStatus::Queued => iced::Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
            TaskStatus::Fetching | TaskStatus::Downloading => iced::Color { r: 0.0, g: 0.55, b: 1.0, a: 1.0 },
            TaskStatus::Done => iced::Color { r: 0.0, g: 0.65, b: 0.0, a: 1.0 },
            TaskStatus::Cancelled => iced::Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
            TaskStatus::Failed(_) => iced::Color { r: 0.85, g: 0.0, b: 0.0, a: 1.0 },
        };

        let raw_title = task.title.strip_prefix("Title: ")
            .unwrap_or(if task.title.is_empty() { "(unknown)" } else { &task.title });
        // Truncate long titles with ellipsis (use char-based slicing for UTF-8)
        let title_text = if raw_title.chars().count() > 50 {
            let truncated: String = raw_title.chars().take(47).collect();
            format!("{}...", truncated)
        } else {
            raw_title.to_string()
        };

        // Progress bar: determinate or unknown
        let (progress_pct, progress_val) = if matches!(task.status, TaskStatus::Done) {
            (format!("{:.0}%", task.progress * 100.0), 1.0)
        } else if matches!(task.status, TaskStatus::Downloading) {
            if task.total_bytes.is_none() {
                // Unknown total: show downloaded bytes, no percentage
                (format!("{} / ???", format_bytes(task.downloaded_bytes)), 0.0)
            } else {
                (format!("{:.0}%", task.progress * 100.0), task.progress as f32)
            }
        } else {
            (format!("{:.0}%", task.progress * 100.0), 0.0)
        };
        let progress_bar_w = progress_bar(0.0..=1.0, progress_val).height(6);

        let total_text = if let Some(total) = task.total_bytes {
            format_bytes(total)
        } else if matches!(task.status, TaskStatus::Downloading) {
            "...".to_string()
        } else {
            String::new()
        };

        let action_btn: Element<'_, Message> = match &task.status {
            TaskStatus::Downloading | TaskStatus::Fetching => {
                button(cjk_text(l.cancel_btn).size(10)).padding(4)
                    .on_press(Message::CancelTask(task.id)).into()
            }
            TaskStatus::Done | TaskStatus::Cancelled | TaskStatus::Failed(_) | TaskStatus::Queued => {
                button(cjk_text(l.remove_btn).size(10)).padding(4)
                    .on_press(Message::RemoveTask(task.id)).into()
            }
        };

        let elapsed_text = format_elapsed(task.elapsed_seconds);

        let row_content = row![
            cjk_text(status_icon).size(13).color(status_color).width(Length::Fixed(32.0)),
            column![
                cjk_text(title_text).size(12),
                row![
                    progress_bar_w,
                    cjk_text(&progress_pct).size(10).width(Length::Fixed(36.0)),
                ].spacing(4).align_y(Alignment::Center),
            ].width(Length::Fill),
            cjk_text(total_text).size(10).width(Length::Fixed(80.0)),
            // Speed: always show text + sparkline
            {
                let spark_part: Element<'_, Message> = if task.speed_history.len() >= 1 {
                    let history: Vec<(f64, f64)> = task.speed_history.iter().copied().collect();
                    let max_spd = history.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max);
                    let sparkline = Sparkline::new(&history, max_spd, is_dark);
                    canvas(sparkline)
                        .width(Length::Fixed(100.0))
                        .height(Length::Fixed(24.0))
                        .into()
                } else {
                    cjk_text("").height(Length::Fixed(24.0)).into()
                };
                let speed_col: Element<'_, Message> = column![
                    cjk_text(&task.speed).size(9).color(if is_dark {
                        iced::Color { r: 0.6, g: 0.8, b: 1.0, a: 1.0 }
                    } else {
                        iced::Color { r: 0.0, g: 0.4, b: 0.8, a: 1.0 }
                    }),
                    spark_part,
                ].spacing(2).into();
                container(speed_col).width(Length::Fixed(110.0))
            },
            cjk_text(&task.eta).size(10).width(Length::Fixed(60.0)),
            cjk_text(elapsed_text).size(10).width(Length::Fixed(60.0)),
            container(action_btn).width(Length::Fixed(60.0)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([6, 12]);

        container(row_content)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(row_bg)),
                border: iced::border::rounded(4),
                ..Default::default()
            })
            .into()
    }

    fn downloads_log_view(&self, is_dark: bool) -> Element<'_, Message> {
        // Left panel: task list (200px wide), Right panel: terminal log
        let sidebar_bg = if is_dark {
            iced::Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 }
        } else {
            iced::Color { r: 0.94, g: 0.94, b: 0.96, a: 1.0 }
        };
        let terminal_bg = if is_dark {
            iced::Color { r: 0.08, g: 0.08, b: 0.08, a: 1.0 }
        } else {
            iced::Color { r: 0.11, g: 0.11, b: 0.12, a: 1.0 }
        };
        let terminal_fg = iced::Color { r: 0.85, g: 0.85, b: 0.85, a: 1.0 };

        // Build left sidebar
        let mut sidebar = column![];
        if self.tasks.is_empty() {
            sidebar = sidebar.push(
                container(cjk_text("无任务").size(11).color(iced::Color {
                    r: 0.56, g: 0.56, b: 0.58, a: 1.0,
                }))
                .padding(12)
                .width(Length::Fill),
            );
        } else {
            for task in &self.tasks {
                let raw = task.title.strip_prefix("Title: ")
                    .unwrap_or(if task.title.is_empty() { "(unknown)" } else { &task.title });
                // Use char-based slicing for UTF-8 safety
                let title_text = if raw.chars().count() > 30 {
                    let truncated: String = raw.chars().take(27).collect();
                    format!("{}...", truncated)
                } else {
                    raw.to_string()
                };
                let progress_val = if matches!(task.status, TaskStatus::Done) {
                    1.0
                } else if matches!(task.status, TaskStatus::Downloading) {
                    task.progress as f32
                } else {
                    0.0
                };
                let progress = progress_bar(0.0..=1.0, progress_val).height(3);
                sidebar = sidebar.push(
                    container(column![
                        cjk_text(title_text).size(10),
                        progress,
                    ].spacing(3))
                    .padding(8)
                    .width(Length::Fill),
                );
            }
        }

        let left_panel = container(scrollable(sidebar))
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(sidebar_bg)),
                border: iced::border::rounded(4),
                ..Default::default()
            });

        // Build terminal log
        let all_logs: Vec<String> = self.tasks.iter()
            .filter(|t| !t.log.is_empty())
            .flat_map(|t| t.log.iter().cloned())
            .collect();

        let log_text = if all_logs.is_empty() {
            "等待 yt-dlp 输出...".to_string()
        } else {
            all_logs.join("\n")
        };

        let right_panel = container(
            scrollable(
                cjk_text(&log_text)
                    .size(11)
                    .font(Font::MONOSPACE)
                    .color(terminal_fg),
            )
            .height(Length::Fill),
        )
        .padding(10)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(terminal_bg)),
            border: iced::border::rounded(4),
            ..Default::default()
        });

        row![left_panel, right_panel].spacing(8).height(Length::Fill).into()
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
        let card_bg = if is_dark {
            iced::Color { r: 0.18, g: 0.18, b: 0.18, a: 1.0 }
        } else {
            iced::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
        };
        let tab_active = iced::Color { r: 0.0, g: 0.47, b: 1.0, a: 1.0 };

        // Build tab bar
        let tab_labels: [(SettingsTab, &str); 7] = [
            (SettingsTab::General, "常规"),
            (SettingsTab::Download, "下载"),
            (SettingsTab::Extractor, "提取器"),
            (SettingsTab::PostProcessing, "后处理"),
            (SettingsTab::Subtitle, "字幕"),
            (SettingsTab::Advanced, "高级"),
            (SettingsTab::SponsorBlock, "SponsorBlock"),
        ];

        let mut tab_row = row![cjk_text(l.settings_title).size(18).width(Length::Fill)];
        for (tab, label) in tab_labels {
            let is_active = tab == self.settings_tab;
            let btn = button(cjk_text(label).size(12)).padding([6, 14]).on_press(Message::SettingsTabChanged(tab));
            let btn = if is_active {
                btn.style(move |_theme, _status| button::Style {
                    background: Some(iced::Background::Color(tab_active)),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else {
                btn
            };
            tab_row = tab_row.push(btn);
        }

        let mut col = Column::new().spacing(16).padding(12);
        col = col.push(tab_row);

        // Helper for labeled text input in a card
        fn labeled_input<'a>(
            label: &str,
            value: &str,
            hint: Option<&str>,
            on_change: fn(String) -> Message,
            card_bg: iced::Color,
        ) -> Element<'a, Message> {
            let mut c = column![cjk_text(label).size(14)];
            if let Some(h) = hint {
                c = c.push(cjk_text(h).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }));
            }
            c = c.push(text_input("", value).on_input(on_change).padding(8));
            c = c.spacing(4);
            container(c).padding(16).width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(card_bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                }).into()
        }

        // Helper for numeric input in a card
        fn numeric_input<'a>(
            label: &str,
            hint: Option<&str>,
            value: usize,
            _min: usize,
            max: usize,
            on_change: fn(String) -> Message,
            card_bg: iced::Color,
        ) -> Element<'a, Message> {
            let mut c = column![cjk_text(label).size(14)];
            if let Some(h) = hint {
                c = c.push(cjk_text(h).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }));
            }
            c = c.push(
                row![
                    button(cjk_text("-").size(14))
                        .padding(8)
                        .on_press(on_change((value.saturating_sub(1)).to_string())),
                    text_input("", &value.to_string())
                        .on_input(on_change)
                        .padding(8)
                        .width(Length::Fixed(80.0)),
                    button(cjk_text("+").size(14))
                        .padding(8)
                        .on_press(on_change(((value + 1).min(max)).to_string())),
                ].spacing(4).align_y(Alignment::Center)
            );
            c = c.spacing(4);
            container(c).padding(16).width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(card_bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                }).into()
        }

        // Helper for checkbox in a card
        fn check_card<'a>(
            label: &str,
            hint: Option<&str>,
            value: bool,
            on_toggle: fn(bool) -> Message,
            card_bg: iced::Color,
        ) -> Element<'a, Message> {
            let mut c = column![checkbox(label, value).on_toggle(on_toggle).size(16)];
            if let Some(h) = hint {
                c = c.push(cjk_text(h).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }));
            }
            c = c.spacing(4);
            container(c).padding(16).width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(card_bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                }).into()
        }

        // Helper for multi-line text input
        fn multiline_input<'a>(
            label: &str,
            hint: &str,
            value: &str,
            _height: f32,
            on_change: fn(String) -> Message,
            card_bg: iced::Color,
        ) -> Element<'a, Message> {
            container(column![
                cjk_text(label).size(14),
                cjk_text(hint).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
                text_input("", value).on_input(on_change).padding(8),
            ].spacing(4)).padding(16).width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(card_bg)),
                    border: iced::border::rounded(8),
                    ..Default::default()
                }).into()
        }

        // Tab content
        match self.settings_tab {
            SettingsTab::General => {
                col = col.push(self.settings_card(
                    row![
                        cjk_text(l.language_label).size(14),
                        button(cjk_text(self.language.label()).size(13))
                            .padding(8)
                            .on_press(Message::ToggleLanguage),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    card_bg,
                ));
                col = col.push(self.settings_card(
                    row![
                        cjk_text(l.theme_label).size(14),
                        button(cjk_text(self.theme.label()).size(13))
                            .padding(8)
                            .on_press(Message::ToggleTheme),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    card_bg,
                ));
                col = col.push(labeled_input(
                    l.download_dir_label,
                    &self.save_dir,
                    Some(l.download_dir_hint),
                    Message::SaveDirChanged,
                    card_bg,
                ));
                col = col.push(numeric_input(
                    l.max_concurrent_label,
                    None,
                    self.max_concurrent,
                    1, 10,
                    Message::MaxConcurrentChanged,
                    card_bg,
                ));
                col = col.push(check_card(
                    l.clipboard_cb,
                    None,
                    self.clipboard_monitor,
                    Message::ClipboardToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "输出文件名模板",
                    &self.config.general.output_template,
                    Some("默认: %(title)s [%(id)s].%(ext)s"),
                    Message::OutputTemplateChanged,
                    card_bg,
                ));
                col = col.push(self.settings_card(
                    column![
                        cjk_text("合并输出格式").size(14),
                        {
                            let current = self.config.general.merge_output_format.as_str();
                            let selected = match current {
                                "mkv" => Some("mkv"),
                                "webm" => Some("webm"),
                                _ => Some("mp4"),
                            };
                            pick_list(
                                ["mp4", "mkv", "webm"].as_slice(),
                                selected,
                                |s: &str| Message::MergeFormatChanged(s.to_string()),
                            ).padding(6)
                        },
                    ].spacing(8),
                    card_bg,
                ));
            }
            SettingsTab::Download => {
                col = col.push(check_card(
                    "每次询问（下载前选择格式）",
                    Some("启用后，点击下载将弹出格式选择弹窗，支持多选同时下载"),
                    self.ask_each_time,
                    Message::AskEachTimeToggled,
                    card_bg,
                ));
                // Only show resolution picker when NOT in "ask each time" mode
                if !self.ask_each_time {
                    let current_label = if self.selected_format_id == "best" {
                        l.best_quality.to_string()
                    } else {
                        self.formats
                            .iter()
                            .find(|f| f.format_id == self.selected_format_id)
                            .map(|f| f.resolution.clone())
                            .unwrap_or_else(|| self.selected_format_id.clone())
                    };
                    col = col.push(self.settings_card(
                        column![
                            cjk_text(l.resolution_label).size(14),
                            button(cjk_text(format!("{current_label} ▼")).size(13))
                                .padding(10)
                                .width(Length::Fixed(200.0))
                                .on_press(Message::SelectFormat),
                        ]
                        .spacing(8),
                        card_bg,
                    ));
                }
                col = col.push(check_card(
                    l.audio_only_cb,
                    Some("仅提取音频轨道，不下载视频"),
                    self.audio_only,
                    Message::AudioOnlyToggled,
                    card_bg,
                ));
                if self.audio_only {
                    col = col.push(self.settings_card(
                        row![
                            cjk_text(l.audio_format_label).size(14),
                            pick_list(
                                [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Flac, AudioFormat::Opus].as_slice(),
                                Some(self.audio_format),
                                Message::AudioFormatChanged,
                            )
                            .padding(6),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                        card_bg,
                    ));
                }
                let ffmpeg_status = if downloader::detect_ffmpeg() {
                    l.ffmpeg_available
                } else {
                    l.ffmpeg_missing
                };
                col = col.push(self.settings_card(
                    row![
                        cjk_text(l.ffmpeg_label).size(14),
                        cjk_text(ffmpeg_status).size(12),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    card_bg,
                ));
                // yt-dlp download options
                col = col.push(numeric_input(
                    "并发分段数量",
                    Some("并行下载的分段数量 (--concurrent-fragments)"),
                    self.config.download.concurrent_fragments,
                    1, 32,
                    Message::ConcurrentFragmentsChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "下载速率限制",
                    &self.config.download.limit_rate,
                    Some("每秒最大下载速率，示例: 50K 或 4.6M (--limit-rate)"),
                    Message::LimitRateChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "限制下载速率",
                    &self.config.download.throttled_rate,
                    Some("启用限速的最小下载速率，示例: 50K 或 4.6M (--throttled-rate)"),
                    Message::ThrottledRateChanged,
                    card_bg,
                ));
                col = col.push(numeric_input(
                    "重试次数",
                    Some("下载失败的重试次数 (--retries)"),
                    self.config.download.retries,
                    0, 100,
                    Message::RetriesChanged,
                    card_bg,
                ));
                col = col.push(numeric_input(
                    "文件访问重试次数",
                    Some("文件访问错误的重试次数 (--file-access-retries)"),
                    self.config.download.file_access_retries,
                    0, 20,
                    Message::FileAccessRetriesChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "下载档案",
                    &self.config.download.download_archive,
                    Some("记录已下载视频的文件路径，跳过重复下载 (--download-archive)"),
                    Message::DownloadArchiveChanged,
                    card_bg,
                ));
                col = col.push(check_card(
                    "出错时中止",
                    Some("下载出错时立即停止 (--abort-on-error)"),
                    self.config.download.abort_on_error,
                    Message::AbortOnErrorToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "忽略错误",
                    Some("忽略下载错误继续下一个 (--ignore-errors)"),
                    self.config.download.ignore_errors,
                    Message::IgnoreErrorsToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "断点续传",
                    Some("支持中断后继续下载 (--continue)"),
                    self.config.download.continue_downloads,
                    Message::ContinueDownloadsToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "不覆盖已有文件",
                    Some("不覆盖已存在的文件 (--no-overwrites)"),
                    self.config.download.no_overwrites,
                    Message::NoOverwritesToggled,
                    card_bg,
                ));
            }
            SettingsTab::Extractor => {
                col = col.push(numeric_input(
                    "提取器重试次数",
                    Some("已知提取器错误的重试次数，默认为 3 (--extractor-retries)"),
                    self.config.extractor.extractor_retries,
                    0, 20,
                    Message::ExtractorRetriesChanged,
                    card_bg,
                ));
                col = col.push(multiline_input(
                    "提取器参数",
                    "每行一个参数，示例: client=value (--extractor-args)",
                    &self.config.extractor.extractor_args.join("\n"),
                    120.0,
                    Message::ExtractorArgsChanged,
                    card_bg,
                ));
                col = col.push(check_card(
                    "强制通用提取器",
                    Some("使用通用提取器而非站点特定提取器 (--force-generic-extractor)"),
                    self.config.extractor.force_generic_extractor,
                    Message::ForceGenericExtractorToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "允许不安全 URL",
                    Some("允许不安全的 URL (--allow-unsafe-url)"),
                    self.config.extractor.allow_unsafe_url,
                    Message::AllowUnsafeUrlToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "扁平化播放列表",
                    Some("不解析播放列表内容 (--extract-flat)"),
                    self.config.extractor.extract_flat,
                    Message::ExtractFlatToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "外部下载器",
                    &self.config.extractor.external_downloader,
                    Some("使用外部下载器如 aria2c, ffmpeg, curl (--external-downloader)"),
                    Message::ExternalDownloaderChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "外部下载器参数",
                    &self.config.extractor.external_downloader_args,
                    Some("传递给外部下载器的参数 (--external-downloader-args)"),
                    Message::ExternalDownloaderArgsChanged,
                    card_bg,
                ));
            }
            SettingsTab::PostProcessing => {
                col = col.push(check_card(
                    "嵌入缩略图",
                    Some("下载缩略图并嵌入为视频封面 (--embed-thumbnail)"),
                    self.config.post_processing.embed_thumbnail,
                    Message::EmbedThumbnailToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "嵌入元数据",
                    Some("自动写入视频标题、描述、上传日期等信息 (--embed-metadata)"),
                    self.config.post_processing.embed_metadata,
                    Message::EmbedMetadataToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "嵌入字幕",
                    Some("将字幕嵌入到视频中 (--embed-subs)"),
                    self.config.post_processing.embed_subs,
                    Message::EmbedSubsToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "保留原始视频",
                    Some("提取音频后不删除原始视频文件 (--keep-video)"),
                    self.config.post_processing.keep_video,
                    Message::KeepVideoToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "不覆盖后处理文件",
                    Some("不覆盖已存在的后处理文件 (--no-post-overwrites)"),
                    self.config.post_processing.no_post_overwrites,
                    Message::NoPostOverwritesToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "转换缩略图格式",
                    &self.config.post_processing.convert_thumbnails,
                    Some("将缩略图转换为目标格式，如 jpg, png (--convert-thumbnails)"),
                    Message::ConvertThumbnailsChanged,
                    card_bg,
                ));
                col = col.push(multiline_input(
                    "后处理参数",
                    "每行一个 yt-dlp 后处理参数 (--postprocessor-args)",
                    &self.config.post_processing.postprocessor_args.join("\n"),
                    120.0,
                    Message::PostprocessorArgsChanged,
                    card_bg,
                ));
            }
            SettingsTab::Subtitle => {
                col = col.push(check_card(
                    l.subtitles_cb,
                    Some("下载时自动获取字幕"),
                    self.subtitles_enabled,
                    Message::SubtitlesToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    l.subtitle_langs_label,
                    &self.subtitle_langs,
                    Some(l.subtitle_langs_hint),
                    Message::SubtitleLangsChanged,
                    card_bg,
                ));
            }
            SettingsTab::Advanced => {
                col = col.push(check_card(
                    "详细模式",
                    Some("输出详细的调试信息 (--verbose)"),
                    self.config.advanced.verbose,
                    Message::VerboseToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "User-Agent",
                    &self.config.advanced.user_agent,
                    Some("自定义浏览器标识 (--user-agent)"),
                    Message::UserAgentChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "Referer",
                    &self.config.advanced.referer,
                    Some("自定义 Referer 头 (--referer)"),
                    Message::RefererChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "代理服务器",
                    &self.config.advanced.proxy,
                    Some("HTTP/SOCKS 代理 (--proxy)"),
                    Message::ProxyChanged,
                    card_bg,
                ));
                col = col.push(check_card(
                    "地理位置绕过",
                    Some("尝试绕过地理限制 (--geo-bypass)"),
                    self.config.advanced.geo_bypass,
                    Message::GeoBypassToggled,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "地理绕过国家代码",
                    &self.config.advanced.geo_bypass_country,
                    Some("两字母国家代码，如 US, JP (--geo-bypass-country)"),
                    Message::GeoBypassCountryChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "地理验证代理",
                    &self.config.advanced.geo_verification_proxy,
                    Some("用于地理验证的代理服务器 (--geo-verification-proxy)"),
                    Message::GeoVerificationProxyChanged,
                    card_bg,
                ));
                col = col.push(multiline_input(
                    "自定义请求头",
                    "每行一个，格式: Header: Value (--add-header)",
                    &self.config.advanced.custom_headers.join("\n"),
                    120.0,
                    Message::CustomHeadersChanged,
                    card_bg,
                ));
                col = col.push(numeric_input(
                    "请求间隔（秒）",
                    Some("两次请求之间的最小等待时间 (--sleep-interval)"),
                    self.config.advanced.sleep_interval,
                    0, 3600,
                    Message::SleepIntervalChanged,
                    card_bg,
                ));
                col = col.push(numeric_input(
                    "最大请求间隔（秒）",
                    Some("两次请求之间的最大等待时间 (--max-sleep-interval)"),
                    self.config.advanced.max_sleep_interval,
                    0, 3600,
                    Message::MaxSleepIntervalChanged,
                    card_bg,
                ));
                col = col.push(check_card(
                    "优先免费格式",
                    Some("优先选择免费/开源格式 (--prefer-free-formats)"),
                    self.config.advanced.prefer_free_formats,
                    Message::PreferFreeFormatsToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "检查格式可用性",
                    Some("下载前检查所有格式是否可用 (--check-formats)"),
                    self.config.advanced.check_formats,
                    Message::CheckFormatsToggled,
                    card_bg,
                ));
                col = col.push(check_card(
                    "模拟模式",
                    Some("仅模拟下载，不实际下载文件 (--simulate)"),
                    self.config.advanced.simulate,
                    Message::SimulateToggled,
                    card_bg,
                ));
            }
            SettingsTab::SponsorBlock => {
                col = col.push(labeled_input(
                    "移除的类别",
                    &self.config.post_processing.sponsorblock_remove,
                    Some("逗号分隔: sponsor,intro,outro,selfpromo,interaction,filler,poi_highlight,music_offtopic。留空则不启用"),
                    Message::SponsorblockRemoveChanged,
                    card_bg,
                ));
                col = col.push(labeled_input(
                    "SponsorBlock API 地址",
                    &self.config.post_processing.sponsorblock_api,
                    Some("自定义 API 端点 (--sponsorblock-api)"),
                    Message::SponsorblockApiChanged,
                    card_bg,
                ));
            }
        }

        // Save button
        let mut save_row = row![].spacing(12);
        save_row = save_row.push(
            button(cjk_text(l.save_config_btn).size(13))
                .padding(10)
                .on_press(Message::SaveConfig),
        );
        if self.config_saved {
            save_row = save_row.push(
                cjk_text(l.config_saved).size(11).color(iced::Color {
                    r: 0.0, g: 0.6, b: 0.0, a: 1.0,
                }),
            );
        }
        col = col.push(save_row);

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn settings_card<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        bg: iced::Color,
    ) -> Element<'a, Message> {
        container(content.into())
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            })
            .into()
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

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn format_elapsed(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Sparkline widget for speed history.
struct Sparkline {
    history: Vec<(f64, f64)>,
    max_speed: f64,
    is_dark: bool,
}

impl Sparkline {
    fn new(history: &[(f64, f64)], max_speed: f64, is_dark: bool) -> Self {
        Sparkline {
            history: history.to_vec(),
            max_speed: if max_speed > 0.0 { max_speed } else { 1.0 },
            is_dark,
        }
    }
}

impl canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        if self.history.len() < 2 {
            return vec![frame.into_geometry()];
        }

        let max_y = self.max_speed;

        // Build points
        let points: Vec<iced::Point> = self.history
            .iter()
            .enumerate()
            .map(|(i, (_, speed))| {
                let x = (i as f32 / (self.history.len() - 1) as f32) * w;
                let y = h - ((speed / max_y).min(1.0) as f32 * (h - 4.0) + 2.0);
                iced::Point::new(x, y)
            })
            .collect();

        let line_color = if self.is_dark {
            iced::Color { r: 0.3, g: 0.8, b: 1.0, a: 1.0 }
        } else {
            iced::Color { r: 0.0, g: 0.5, b: 1.0, a: 1.0 }
        };

        // Draw line segments
        for i in 0..points.len() - 1 {
            frame.stroke(
                &Path::line(points[i], points[i + 1]),
                canvas::Stroke::default()
                    .with_color(line_color)
                    .with_width(1.5),
            );
        }

        // Filled area under the line
        let fill_alpha = if self.is_dark { 0.15 } else { 0.1 };
        let area = Path::new(|b| {
            b.move_to(iced::Point::new(points[0].x, h));
            b.line_to(points[0]);
            for i in 1..points.len() {
                b.line_to(points[i]);
            }
            b.line_to(iced::Point::new(points[points.len() - 1].x, h));
            b.close();
        });
        frame.fill(&area, iced::Color::from_rgba8(0, 200, 83, fill_alpha));

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        iced::mouse::Interaction::default()
    }
}
