use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用配置，通过 TOML 持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub download: DownloadConfig,
    #[serde(default)]
    pub extractor: ExtractorConfig,
    #[serde(default)]
    pub post_processing: PostProcessingConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub defaults: DefaultConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub download_dir: String,
    pub max_concurrent: usize,
    pub theme: String,
    pub language: String,
    pub clipboard_monitor: bool,
    /// yt-dlp output filename template
    pub output_template: String,
    /// Merge output format (mp4, mkv, webm)
    pub merge_output_format: String,
    /// Audio-only mode
    pub audio_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    /// Number of concurrent fragments (--concurrent-fragments)
    pub concurrent_fragments: usize,
    /// Download rate limit (--limit-rate), e.g. "50K", "4.6M"
    pub limit_rate: String,
    /// Throttled rate (--throttled-rate)
    pub throttled_rate: String,
    /// Number of retries (--retries)
    pub retries: usize,
    /// File access retries (--file-access-retries)
    pub file_access_retries: usize,
    /// Download archive file path (--download-archive)
    pub download_archive: String,
    /// Abort on error (--abort-on-error)
    pub abort_on_error: bool,
    /// Ignore errors (--ignore-errors)
    pub ignore_errors: bool,
    /// Continue on error (--continue)
    pub continue_downloads: bool,
    /// No overwrites (--no-overwrites)
    pub no_overwrites: bool,
    /// No part file (--no-part)
    pub no_part: bool,
    /// No mtime (--no-mtime)
    pub no_mtime: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// Extractor arguments (--extractor-args), one per line
    pub extractor_args: Vec<String>,
    /// Extractor retries (--extractor-retries)
    pub extractor_retries: usize,
    /// Force generic extractor (--force-generic-extractor)
    pub force_generic_extractor: bool,
    /// Allow unsafe URL (--allow-unsafe-url)
    pub allow_unsafe_url: bool,
    /// Extract flat playlist (--extract-flat)
    pub extract_flat: bool,
    /// External downloader (e.g., "aria2c", "ffmpeg", "curldownloader")
    pub external_downloader: String,
    /// External downloader arguments (--external-downloader-args)
    pub external_downloader_args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingConfig {
    /// Embed thumbnail (--embed-thumbnail)
    pub embed_thumbnail: bool,
    /// Embed metadata (--embed-metadata)
    pub embed_metadata: bool,
    /// Embed subtitles (--embed-subs)
    pub embed_subs: bool,
    /// Post-processor args (--postprocessor-args), one per line
    pub postprocessor_args: Vec<String>,
    /// Keep video after extraction (--keep-video)
    pub keep_video: bool,
    /// No post-overwrites (--no-post-overwrites)
    pub no_post_overwrites: bool,
    /// Convert thumbnails (--convert-thumbnails)
    pub convert_thumbnails: String,
    /// SponsorBlock categories (--sponsorblock-remove)
    pub sponsorblock_remove: String,
    /// SponsorBlock API URL (--sponsorblock-api)
    pub sponsorblock_api: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Verbose mode (--verbose)
    pub verbose: bool,
    /// Custom headers (--add-header), one per line
    pub custom_headers: Vec<String>,
    /// User agent (--user-agent)
    pub user_agent: String,
    /// Referer (--referer)
    pub referer: String,
    /// Proxy (--proxy)
    pub proxy: String,
    /// Geo-verification proxy (--geo-verification-proxy)
    pub geo_verification_proxy: String,
    /// Geo-bypass (--geo-bypass)
    pub geo_bypass: bool,
    /// Geo-bypass country (--geo-bypass-country)
    pub geo_bypass_country: String,
    /// Sleep interval (--sleep-interval)
    pub sleep_interval: usize,
    /// Max sleep interval (--max-sleep-interval)
    pub max_sleep_interval: usize,
    /// Prefer free formats (--prefer-free-formats)
    pub prefer_free_formats: bool,
    /// Check formats (--check-formats)
    pub check_formats: bool,
    /// Simulate (--simulate)
    pub simulate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    pub video_quality: String,
    pub audio_format: String,
    pub ask_each_time: bool,
    pub subtitles_enabled: bool,
    pub subtitle_langs: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        let download_dir = dirs::video_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\Users\\CZH\\Videos"))
            .to_string_lossy()
            .to_string();
        Self {
            download_dir,
            max_concurrent: 3,
            theme: "Light".to_string(),
            language: "English".to_string(),
            clipboard_monitor: true,
            output_template: "%(title)s [%(id)s].%(ext)s".to_string(),
            merge_output_format: "mp4".to_string(),
            audio_only: false,
        }
    }
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            concurrent_fragments: 1,
            limit_rate: String::new(),
            throttled_rate: String::new(),
            retries: 10,
            file_access_retries: 3,
            download_archive: String::new(),
            abort_on_error: false,
            ignore_errors: false,
            continue_downloads: true,
            no_overwrites: false,
            no_part: false,
            no_mtime: false,
        }
    }
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            extractor_args: Vec::new(),
            extractor_retries: 3,
            force_generic_extractor: false,
            allow_unsafe_url: false,
            extract_flat: false,
            external_downloader: String::new(),
            external_downloader_args: String::new(),
        }
    }
}

impl Default for PostProcessingConfig {
    fn default() -> Self {
        Self {
            embed_thumbnail: false,
            embed_metadata: false,
            embed_subs: false,
            postprocessor_args: Vec::new(),
            keep_video: false,
            no_post_overwrites: false,
            convert_thumbnails: String::new(),
            sponsorblock_remove: String::new(),
            sponsorblock_api: "https://sponsor.ajay.app".to_string(),
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            custom_headers: Vec::new(),
            user_agent: String::new(),
            referer: String::new(),
            proxy: String::new(),
            geo_verification_proxy: String::new(),
            geo_bypass: false,
            geo_bypass_country: String::new(),
            sleep_interval: 0,
            max_sleep_interval: 0,
            prefer_free_formats: false,
            check_formats: false,
            simulate: false,
        }
    }
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            video_quality: "ask".to_string(),
            audio_format: "m4a".to_string(),
            ask_each_time: true,
            subtitles_enabled: false,
            subtitle_langs: "zh-Hans".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            download: DownloadConfig::default(),
            extractor: ExtractorConfig::default(),
            post_processing: PostProcessingConfig::default(),
            advanced: AdvancedConfig::default(),
            defaults: DefaultConfig::default(),
        }
    }
}

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("yt-downloader"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("config.toml"))
}

impl Config {
    /// 从磁盘加载配置，如果不存在则返回默认值。
    pub fn load() -> Self {
        if let Some(path) = config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
        Config::default()
    }

    /// 保存配置到磁盘。
    pub fn save(&self) -> Result<(), String> {
        let dir = config_dir().ok_or("无法获取配置目录")?;
        fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
        let content = toml::to_string_pretty(self).map_err(|e| format!("序列化失败: {e}"))?;
        let path = dir.join("config.toml");
        fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
        Ok(())
    }
}
