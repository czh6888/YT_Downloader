use std::path::PathBuf;

use iced::widget::{
    button, checkbox, column, container, horizontal_space, pick_list, progress_bar, row,
    scrollable, text_input, vertical_space, Column,
};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use tokio::io::AsyncBufReadExt;

use crate::downloader::{self, CookieResult, FormatInfo};

// ---------------------------------------------------------------------------
// CJK Font
// ---------------------------------------------------------------------------

/// Create text with CJK font support (Microsoft YaHei).
fn t<'a>(content: impl std::fmt::Display) -> iced::widget::Text<'a, iced::Theme> {
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

/// All localized strings in one place.
pub struct Lang {
    // Page titles
    pub page_title: &'static str,
    // Sidebar
    pub sidebar_download: &'static str,
    pub sidebar_queue: &'static str,
    pub sidebar_history: &'static str,
    pub sidebar_settings: &'static str,
    // Download page
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
    // Status messages
    pub status_idle: &'static str,
    pub status_extracting: &'static str,
    pub status_fetching: &'static str,
    pub status_ready: &'static str,
    // Queue page
    pub queue_title: &'static str,
    pub queue_empty: &'static str,
    pub status_queued: &'static str,
    pub status_fetching_info: &'static str,
    pub status_downloading: &'static str,
    pub status_done: &'static str,
    pub status_cancelled: &'static str,
    pub status_failed: &'static str,
    pub cancel_btn: &'static str,
    // History page
    pub history_title: &'static str,
    pub history_empty: &'static str,
    // Settings page
    pub settings_title: &'static str,
    pub language_label: &'static str,
    pub theme_label: &'static str,
    pub download_dir_label: &'static str,
    pub download_dir_hint: &'static str,
    pub max_concurrent_label: &'static str,
    pub clipboard_cb: &'static str,
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
            video_url_hint: "https://www.youtube.com/watch?v=...",
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
            history_title: "Download History",
            history_empty: "No downloads yet.",
            settings_title: "Settings",
            language_label: "Language:",
            theme_label: "Theme:",
            download_dir_label: "Download Directory",
            download_dir_hint: "Download path",
            max_concurrent_label: "Max Concurrent Downloads",
            clipboard_cb: "Monitor clipboard for video URLs",
        };
        static ZH: Lang = Lang {
            page_title: "YouTube 下载器",
            sidebar_download: "下载",
            sidebar_queue: "队列",
            sidebar_history: "历史",
            sidebar_settings: "设置",
            video_url_label: "视频链接",
            video_url_hint: "https://www.youtube.com/watch?v=...",
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
            history_title: "下载历史",
            history_empty: "暂无下载记录。",
            settings_title: "设置",
            language_label: "语言：",
            theme_label: "主题：",
            download_dir_label: "下载目录",
            download_dir_hint: "下载路径",
            max_concurrent_label: "最大并发下载数",
            clipboard_cb: "自动检测剪贴板中的视频链接",
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

/// Downloadable page enum.
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

/// A single download task in the queue.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DownloadTask {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub format: String,
    pub audio_only: bool,
    pub status: TaskStatus,
    pub progress: f64,      // 0.0 - 1.0
    pub speed: String,
    pub eta: String,
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TaskStatus {
    Queued,
    Fetching,
    Downloading,
    Done,
    Failed(String),
}

/// History entry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HistoryEntry {
    pub title: String,
    pub url: String,
    pub format: String,
    pub status: String,
    pub date: String,
    pub file_path: String,
}

/// Main application state.
pub struct App {
    // Navigation
    page: Page,

    // Theme & Language
    pub theme: ThemeMode,
    pub language: Language,

    // Download page
    url: String,
    browser: Browser,
    status: FetchStatus,
    formats: Vec<FormatInfo>,
    selected_resolution: String,
    audio_only: bool,
    audio_format: AudioFormat,
    video_info_log: Vec<String>,
    save_dir: String,

    // Cookie state
    cookie_file: String,
    cookie_result: Option<CookieResult>,

    // Queue
    tasks: Vec<DownloadTask>,
    next_task_id: u64,
    max_concurrent: usize,

    // History
    history: Vec<HistoryEntry>,

    // Settings
    clipboard_monitor: bool,
    last_clipboard: String,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub enum Message {
    // Navigation
    NavigateTo(Page),
    ToggleTheme,
    ToggleLanguage,

    // Download page
    UrlChanged(String),
    BrowserChanged(Browser),
    Fetch,
    FetchResult(Result<serde_json::Value, String>, CookieResult),
    ResolutionSelected(String),
    AudioOnlyToggled(bool),
    AudioFormatChanged(AudioFormat),

    // Queue actions
    Download,
    AddToQueue,
    CancelTask(u64),
    TaskResult { id: u64, result: downloader::DownloadResult },
    TaskLog { id: u64, line: String },

    // Settings
    SaveDirChanged(String),
    MaxConcurrentChanged(String),
    ClipboardToggled(bool),
    ClipboardCheck(String),
}

// ---------------------------------------------------------------------------
// Impl
// ---------------------------------------------------------------------------

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let save_dir = dirs::video_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\Users\\CZH\\Videos"))
            .to_string_lossy()
            .to_string();
        let cookie_file = std::env::var("TEMP")
            .map(|t| format!("{t}\\yt_cookies.txt"))
            .unwrap_or_else(|_| "yt_cookies.txt".to_string());

        (
            App {
                page: Page::default(),
                theme: ThemeMode::default(),
                language: Language::default(),
                url: String::new(),
                browser: Browser::Chrome,
                status: FetchStatus::Idle,
                formats: Vec::new(),
                selected_resolution: "best".to_string(),
                audio_only: false,
                audio_format: AudioFormat::default(),
                video_info_log: Vec::new(),
                save_dir,
                cookie_file,
                cookie_result: None,
                tasks: Vec::new(),
                next_task_id: 1,
                max_concurrent: 3,
                history: Vec::new(),
                clipboard_monitor: true,
                last_clipboard: String::new(),
            },
            Task::none(),
        )
    }

    /// Get the current language's string bundle.
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
            }

            Message::ToggleLanguage => {
                self.language = self.language.toggle();
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
                        // Step 1: Extract cookies
                        let cookie_result =
                            downloader::extract_browser_cookies(&browser_name, &cookie_file);
                        let (cookie_file_opt, browser_native_opt) =
                            cookie_args_from_result(&cookie_result);

                        // Step 2: Fetch video info
                        let cookie_args = downloader::cookie_args(cookie_file_opt.as_deref(), browser_native_opt.as_deref());
                        let info = downloader::fetch_info(&url, &cookie_args).await;

                        match info {
                            Ok(v) => Ok((v, cookie_result)),
                            Err(e) => Err((e.to_string(), cookie_result)),
                        }
                    },
                    |result| match result {
                        Ok((_info, cookie_result)) => {
                            Message::FetchResult(Ok(_info), cookie_result)
                        }
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
                self.selected_resolution = "best".to_string();
                self.status = FetchStatus::Ready;
                self.video_info_log.push(format!("Title: {title}"));
                self.video_info_log.push(format!("Found {} formats", self.formats.len()));
                self.video_info_log.push(format!("Cookie: {}", cookie_result.message));

                // Store cookie result for download
                self.cookie_result = Some(cookie_result);
            }

            Message::FetchResult(Err(e), cookie_result) => {
                self.status = FetchStatus::Idle;
                self.video_info_log.push(format!("Error: {e}"));
                self.video_info_log.push(format!("Cookie: {}", cookie_result.message));
                self.cookie_result = Some(cookie_result);
            }

            Message::ResolutionSelected(res) => {
                self.selected_resolution = res;
            }

            Message::AudioOnlyToggled(val) => {
                self.audio_only = val;
            }

            Message::AudioFormatChanged(fmt) => {
                self.audio_format = fmt;
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

                let task = DownloadTask {
                    id,
                    url: self.url.clone(),
                    title: self.video_info_log.first().cloned().unwrap_or_default(),
                    format: self.selected_resolution.clone(),
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
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.status = TaskStatus::Failed("Cancelled".to_string());
                }
            }

            Message::TaskResult { id, result } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.log.extend(result.log_lines.clone());
                    if result.success {
                        task.status = TaskStatus::Done;
                        task.progress = 1.0;
                        // Add to history
                        self.history.insert(0, HistoryEntry {
                            title: task.title.clone(),
                            url: task.url.clone(),
                            format: task.format.clone(),
                            status: "Completed".to_string(),
                            date: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                            file_path: result.file_path.unwrap_or_default(),
                        });
                    } else {
                        task.status = TaskStatus::Failed("Download failed".to_string());
                    }
                    // Start next queued task
                    self.process_queue();
                }
            }

            Message::TaskLog { id, line } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.log.push(line.clone());
                    // Try to parse progress
                    if let Some(prog) = downloader::parse_progress(&line) {
                        task.progress = prog.percentage / 100.0;
                        task.speed = format_speed(prog.speed);
                        task.eta = format_eta(prog.eta);
                    }
                }
            }

            Message::SaveDirChanged(dir) => {
                self.save_dir = dir;
            }

            Message::MaxConcurrentChanged(val) => {
                if let Ok(n) = val.parse::<usize>()
                    && n > 0 && n <= 10 {
                        self.max_concurrent = n;
                    }
            }

            Message::ClipboardToggled(val) => {
                self.clipboard_monitor = val;
            }

            Message::ClipboardCheck(content) => {
                if self.clipboard_monitor && !content.is_empty() && content != self.last_clipboard {
                    // Check if it looks like a video URL
                    if is_video_url(&content) {
                        self.last_clipboard = content.clone();
                        self.url = content;
                    }
                }
            }
        }
        Task::none()
    }

    /// Start a download and stream progress updates live.
    fn start_download(&self, id: u64) -> Task<Message> {
        let url = self.url.clone();
        let format = self.selected_resolution.clone();
        let save_dir = self.save_dir.clone();
        let audio_only = self.audio_only;
        let audio_format = self.audio_format.to_string();
        let browser = self.browser.to_string();
        let cookie_file = self.cookie_file.clone();
        let cookie_result = self.cookie_result.clone();

        Task::stream(iced::stream::channel(16, move |mut output| {
            let url = url.clone();
            let format = format.clone();
            let save_dir = save_dir.clone();
            let cookie_file = cookie_file.clone();
            let cookie_result = cookie_result.clone();
            let browser = browser.clone();

            async move {
                // Get cookie args
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
                    downloader::build_format_string(&format, None)
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

                cmd.args(&cookie_args).arg(&url);

                let mut log_lines = Vec::new();
                log_lines.push(format!("Command: {} ...", yt_dlp[0]));
                log_lines.push("-".repeat(50));

                match cmd.stdout(std::process::Stdio::piped()).spawn() {
                    Ok(mut child) => {
                        let stdout = child.stdout.take().unwrap();
                        let reader = tokio::io::BufReader::new(stdout);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            let trimmed = line.trim_end().to_owned();
                            if !trimmed.is_empty() {
                                log_lines.push(trimmed.clone());
                                // Stream each line to UI for live progress
                                let _ = output.try_send(Message::TaskLog { id, line: trimmed });
                            }
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

    fn process_queue(&mut self) {
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
                self.tasks[pos].status = TaskStatus::Fetching;
            }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.clipboard_monitor {
            iced::time::every(std::time::Duration::from_secs(2)).map(|_| {
                // Clipboard check
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

    /// Localized page label.
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

        let sidebar = self.sidebar_view(is_dark);
        let content = match self.page {
            Page::Download => self.download_view(is_dark),
            Page::Queue => self.queue_view(is_dark),
            Page::History => self.history_view(is_dark),
            Page::Settings => self.settings_view(is_dark),
        };

        row![
            sidebar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(16),
        ]
        .height(Length::Fill)
        .into()
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
            let btn = button(t(label).size(14))
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

        // Title
        col = col.push(
            t(l.page_title).size(22),
        );

        // URL Input
        col = col.push(
            column![
                t(l.video_url_label).size(14),
                text_input(l.video_url_hint, &self.url)
                    .on_input(Message::UrlChanged)
                    .on_submit(Message::Fetch)
                    .padding(10)
                    .size(13),
            ]
            .spacing(4),
        );

        // Browser selection
        col = col.push(self.browser_row());

        // Fetch button
        let is_fetching = !matches!(self.status, FetchStatus::Idle | FetchStatus::Ready);
        let fetch_btn: Element<'_, Message> = if is_fetching {
            button(t(l.fetch_btn).size(14))
                .padding(10)
                .width(Length::Fixed(150.0))
                .into()
        } else {
            button(t(l.fetch_btn).size(14))
                .padding(10)
                .width(Length::Fixed(150.0))
                .on_press(Message::Fetch)
                .into()
        };
        col = col.push(fetch_btn);

        // Status
        let status_text = match &self.status {
            FetchStatus::Idle => l.status_idle,
            FetchStatus::ExtractingCookies => l.status_extracting,
            FetchStatus::Fetching => l.status_fetching,
            FetchStatus::Ready => l.status_ready,
        };
        col = col.push(t(status_text).size(12).color(iced::Color {
            r: 0.56, g: 0.56, b: 0.58, a: 1.0,
        }));

        // Audio-only toggle
        col = col.push(
            checkbox(l.audio_only_cb, self.audio_only)
                .on_toggle(Message::AudioOnlyToggled)
                .size(16),
        );

        if self.audio_only {
            // Audio format selection
            let fmts = [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Flac, AudioFormat::Opus];
            col = col.push(
                row![
                    t(l.audio_format_label).size(13),
                    pick_list(fmts, Some(self.audio_format), Message::AudioFormatChanged)
                        .padding(6),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            // Resolution selection
            if !self.formats.is_empty() || matches!(self.status, FetchStatus::Ready) {
                col = col.push(self.resolution_section());
            }
        }

        // Action buttons
        let can_download = matches!(self.status, FetchStatus::Ready) || !self.url.is_empty();
        let mut action_row = row![].spacing(12);
        action_row = action_row.push(
            button(t(l.download_now_btn).size(14))
                .padding(10)
                .on_press(Message::Download),
        );
        action_row = action_row.push(
            button(t(l.add_to_queue_btn).size(14))
                .padding(10)
                .on_press(Message::AddToQueue),
        );
        if can_download {
            col = col.push(action_row);
        }

        // Video info log
        if !self.video_info_log.is_empty() {
            col = col.push(self.info_log_view(is_dark));
        }

        // Download folder
        col = col.push(
            button(t(l.open_folder_btn).size(13))
                .padding(8)
                .on_press(Message::SaveDirChanged(self.save_dir.clone())),
        );

        container(scrollable(col)).height(Length::Fill).into()
    }

    fn browser_row(&self) -> Column<'_, Message> {
        let l = self.lang();
        let browsers = [Browser::Chrome, Browser::Edge, Browser::Firefox];
        let mut r = row![].spacing(16).align_y(Alignment::Center);
        r = r.push(t(l.browser_label).size(13));
        for browser in browsers {
            let is_selected = browser == self.browser;
            let btn = button(t(browser.to_string()).size(13))
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

    fn resolution_section(&self) -> Column<'_, Message> {
        let l = self.lang();
        let mut col = Column::new().spacing(4);
        col = col.push(t(l.resolution_label).size(14));

        // Best option
        let best_btn = button(t(l.best_quality).size(12))
            .padding(8)
            .width(Length::Fill)
            .on_press(Message::ResolutionSelected("best".to_string()));
        let best_btn = if self.selected_resolution == "best" {
            best_btn.style(|_, _| button::Style {
                background: Some(iced::Background::Color(iced::Color {
                    r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                })),
                text_color: iced::Color::WHITE,
                border: iced::border::rounded(6),
                ..Default::default()
            })
        } else {
            best_btn
        };
        col = col.push(best_btn);

        // Video resolutions
        let mut seen = std::collections::HashSet::new();
        for fmt in &self.formats {
            if !fmt.is_video { continue; }
            let Some(h) = fmt.height else { continue; };
            if !seen.insert(h) { continue; }

            let mut label = fmt.resolution.clone();
            if let Some(fps) = fmt.fps
                && fps > 30.0 { label.push_str(&format!(" {fps}fps")); }
            if fmt.note.to_lowercase().contains("hdr") { label.push_str(" HDR"); }

            let h_str = h.to_string();
            let is_sel = self.selected_resolution == h_str;
            let btn = button(t(label).size(12))
                .padding(8)
                .width(Length::Fill)
                .on_press(Message::ResolutionSelected(h_str));
            let btn = if is_sel {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                    })),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else { btn };
            col = col.push(btn);
        }

        col
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
            t(l.info_label).size(14),
            container(t(content).size(11).font(Font::MONOSPACE).color(fg))
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
        col = col.push(t(l.queue_title).size(20));

        if self.tasks.is_empty() {
            col = col.push(
                t(l.queue_empty)
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
            TaskStatus::Failed(_) => "[X]",
        };

        let status_text = match &task.status {
            TaskStatus::Queued => l.status_queued.to_string(),
            TaskStatus::Fetching => l.status_fetching_info.to_string(),
            TaskStatus::Downloading => {
                format!("{} {:.0}% | {} | ETA: {}", l.status_downloading, task.progress * 100.0, task.speed, task.eta)
            }
            TaskStatus::Done => l.status_done.to_string(),
            TaskStatus::Failed(e) => format!("{}: {e}", l.status_failed),
        };

        let title_text = if task.title.is_empty() || task.title.starts_with("Title: ") {
            task.title.strip_prefix("Title: ").unwrap_or(&task.title).to_string()
        } else {
            task.title.clone()
        };

        let mut content = column![
            row![
                t(format!("{status_icon} {title_text}")).size(13),
                horizontal_space(),
                {
                    let cancel_btn: Element<'_, Message> = if matches!(task.status, TaskStatus::Downloading | TaskStatus::Fetching) {
                        button(t(l.cancel_btn).size(11))
                            .padding(4)
                            .on_press(Message::CancelTask(task.id))
                            .into()
                    } else {
                        button(t("X").size(11)).padding(4).into()
                    };
                    cancel_btn
                },
            ]
            .spacing(8),
            t(status_text).size(11).color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
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
        col = col.push(t(l.history_title).size(20));

        if self.history.is_empty() {
            col = col.push(
                t(l.history_empty).size(13).color(iced::Color {
                    r: 0.56, g: 0.56, b: 0.58, a: 1.0,
                }),
            );
        } else {
            for entry in &self.history {
                col = col.push(
                    row![
                        column![
                            t(&entry.title).size(13),
                            t(format!("{} | {} | {}", entry.date, entry.format, entry.status))
                                .size(11)
                                .color(iced::Color { r: 0.56, g: 0.56, b: 0.58, a: 1.0 }),
                        ],
                        horizontal_space(),
                    ]
                    .spacing(8),
                );
            }
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

        let mut col = Column::new().spacing(16).padding(8);
        col = col.push(t(l.settings_title).size(20));

        // Language switch
        col = col.push(
            container(
                row![
                    t(l.language_label).size(14),
                    button(t(self.language.label()).size(13))
                        .padding(8)
                        .on_press(Message::ToggleLanguage),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            }),
        );

        // Theme
        col = col.push(
            container(
                row![
                    t(l.theme_label).size(14),
                    button(t(self.theme.label()).size(13))
                        .padding(8)
                        .on_press(Message::ToggleTheme),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            }),
        );

        // Download directory
        col = col.push(
            container(
                column![
                    t(l.download_dir_label).size(14),
                    text_input(l.download_dir_hint, &self.save_dir)
                        .on_input(Message::SaveDirChanged)
                        .padding(8),
                ]
                .spacing(8),
            )
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            }),
        );

        // Max concurrent downloads
        col = col.push(
            container(
                column![
                    t(l.max_concurrent_label).size(14),
                    text_input("3", &self.max_concurrent.to_string())
                        .on_input(Message::MaxConcurrentChanged)
                        .padding(8),
                ]
                .spacing(8),
            )
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            }),
        );

        // Clipboard monitor
        col = col.push(
            container(
                checkbox(l.clipboard_cb, self.clipboard_monitor)
                    .on_toggle(Message::ClipboardToggled)
                    .size(16),
            )
            .padding(16)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(8),
                ..Default::default()
            }),
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
        "vimeo.com/",
        "bilibili.com/video",
        "dailymotion.com/video",
        "twitch.tv/videos",
        "twitter.com/",
        "x.com/",
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
