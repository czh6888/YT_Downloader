use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用配置，通过 TOML 持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    pub video_quality: String,
    pub audio_format: String,
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
        }
    }
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            video_quality: "best".to_string(),
            audio_format: "m4a".to_string(),
            subtitles_enabled: false,
            subtitle_langs: "zh-Hans".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
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
