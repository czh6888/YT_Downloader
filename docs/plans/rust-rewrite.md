# YouTube Downloader - Rust Rewrite Plan

## Context

当前项目是一个 Python + CustomTkinter 的 YouTube 下载工具，功能包括：通过 yt-dlp 下载视频、从 Chrome/Edge/Firefox 提取 Cookie（涉及复杂的 v20 DPAPI/CNG 解密）、简单的分辨率选择 GUI。项目存在以下问题：
- 只有单视频下载，无音频模式/播放列表/字幕/进度条
- Cookie 解密逻辑脆弱（硬编码密钥、lsass 注入）
- GUI 简单，功能有限
- 仅限 Windows

目标：用 **Rust + Iced** 重写，保留现有功能的同时大幅增加新特性。

---

## 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| 语言 | Rust | 用户指定，内存安全、高性能 |
| GUI | Iced (0.13+) | 用户指定，Elm 架构、纯 Rust、跨平台 |
| 下载引擎 | yt-dlp (subprocess) | 成熟的下载引擎，支持 1000+ 站点，无需重造轮子 |
| Cookie 解密 | windows-rs crate | 在 Rust 中重新实现当前 DPAPI+CNG 逻辑 |
| Cookie 备用 | chromelevator.exe | 保留现有 tools/ 中的二进制 |
| 剪贴板 | arboard crate | 跨平台剪贴板访问 |
| 异步运行时 | tokio | Rust 生态标准异步运行时 |
| 数据持久化 | sqlite (rusqlite) | 存储下载历史 |
| 配置 | serde + toml | 用户配置保存 |
| 后处理 | ffmpeg | 自动转码（用户安装或自动下载） |

---

## 项目结构

```
yt-downloader/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口点
│   ├── app.rs               # Iced Application 实现（状态管理）
│   ├── config.rs            # 配置加载/保存 (TOML)
│   ├── downloader/
│   │   ├── mod.rs           # 下载器接口
│   │   ├── yt_dlp.rs        # yt-dlp subprocess 封装
│   │   ├── formats.rs       # 格式解析（分辨率、音频、字幕）
│   │   └── progress.rs      # 下载进度解析
│   ├── cookies/
│   │   ├── mod.rs           # Cookie 提取统一入口
│   │   ├── firefox.rs       # Firefox sqlite 读取
│   │   ├── chromium.rs      # Chrome/Edge DPAPI+CNG 解密
│   │   ├── chromelevator.rs # chromelevator 备用方案
│   │   └── netscape.rs      # Netscape 格式序列化
│   ├── history.rs           # 下载历史 (SQLite)
│   ├── clipboard.rs         # 剪贴板监控
│   └── ui/
│       ├── mod.rs           # UI 组件集合
│       ├── layout.rs        # 主布局
│       ├── url_input.rs     # URL 输入组件
│       ├── browser_select.rs# 浏览器选择
│       ├── format_select.rs # 格式/分辨率选择
│       ├── download_queue.rs# 下载队列管理
│       ├── download_history.rs # 下载历史视图
│       ├── progress_bar.rs  # 进度条组件
│       ├── log_panel.rs     # 日志面板
│       └── settings.rs      # 设置面板
├── tools/
│   ├── chromelevator_x64.exe
│   └── chromelevator_arm64.exe
└── assets/                  # 图标、样式
```

---

## 实现阶段

### Phase 1: 项目骨架 + 基础下载

**目标**: 建立 Rust 项目结构，实现基本 yt-dlp 封装，Iced 窗口显示 URL 输入和下载按钮。

**关键文件**:
- `Cargo.toml` - 依赖声明
- `src/main.rs` - 入口
- `src/app.rs` - Iced Application (state + update + view)
- `src/downloader/yt_dlp.rs` - yt-dlp subprocess 封装
  - `find_yt_dlp()` - 查找 yt-dlp 可执行文件
  - `fetch_info(url)` - `yt-dlp --dump-json` 获取视频信息
  - `parse_formats(info)` - 解析可用格式（视频分辨率、音频轨道、字幕）
  - `download(url, format_opts)` - 启动下载，stdout 流式读取

**复用**: 对应现有 `yt_downloader/downloader.py` 中的 `find_yt_dlp`, `fetch_formats`, `download_video`

---

### Phase 2: Cookie 提取 (Rust 重实现)

**目标**: 在 Rust 中重新实现 v20 Cookie 解密。

**关键文件**:
- `src/cookies/mod.rs` - `extract_cookies(browser) -> Option<CookieFile>`
- `src/cookies/firefox.rs` - Firefox cookie.sqlite 读取（最简单，无加密）
- `src/cookies/chromium.rs` - Chrome/Edge v20 解密:
  - 读取 `Local State` 中的 `app_bound_encrypted_key`
  - DPAPI 双阶段解密 (SYSTEM → User)
  - 使用 `windows-rs` crate 的 `CryptUnprotectData`
  - 解析 key blob (flag 1/2/3)
  - flag=3: CNG `NCryptDecrypt` 解密 AES key
  - AES-GCM / ChaCha20Poly1305 解密 cookie 值
  - 序列化到 Netscape 格式
- `src/cookies/chromelevator.rs` - chromelevator.exe 备用调用
- `src/cookies/netscape.rs` - Netscape 格式序列化

**复用**: 对应现有 `yt_downloader/browser_cookies.py`、`decrypt_chrome_v20.py`、`decrypt_edge_v20.py` 的全部逻辑
**外部工具**: 保留 `tools/chromelevator_*.exe` 作为 DPAPI 失败后的备用

---

### Phase 3: Iced GUI 完整实现

**目标**: 实现完整的 UI 界面，对标并超越当前 CustomTkinter GUI。

**UI 布局** (单窗口，左侧导航 + 右侧内容):

1. **主下载页面**:
   - URL 输入框（支持粘贴多行 URL）
   - 浏览器选择（Chrome/Edge/Firefox 单选）
   - "获取信息" 按钮 → 解析视频信息
   - 格式选择区：视频分辨率、音频轨道、字幕
   - "添加到队列" / "立即下载" 按钮

2. **下载队列页面** (Phase 4):
   - 当前下载列表 + 进度条
   - 排队等待列表
   - 暂停/恢复/取消按钮
   - 并发数设置

3. **下载历史页面** (Phase 5):
   - 历史记录列表（缩略图、标题、日期、状态）
   - 搜索/过滤
   - 重新下载 / 打开文件位置

4. **设置页面** (Phase 5):
   - 下载目录选择
   - 默认格式偏好
   - 剪贴板监控开关
   - ffmpeg 路径配置

**关键文件**: `src/ui/*.rs` 各组件

---

### Phase 4: 下载队列 & 并发 & 进度

**目标**: 支持多任务并发下载，实时进度显示。

**关键文件**:
- `src/app.rs` - 新增 `DownloadQueue` 状态管理
- `src/downloader/progress.rs` - 解析 yt-dlp `--newline --progress` 输出
  - 正则匹配进度行: `[download]  45.2% of ~  12.34MiB at  2.15MiB/s ETA 00:05`
  - 提取: 百分比、速度、ETA、已下载大小
- `src/app.rs` - `Message::DownloadProgress(id, ProgressInfo)` 更新 UI

**yt-dlp 命令**: `yt-dlp --newline --progress -o "..." -f "..." URL`

---

### Phase 5: 历史记录 & 配置 & 剪贴板

**目标**: 数据持久化、用户偏好、自动检测。

**关键文件**:
- `src/history.rs` - SQLite 下载历史:
  ```sql
  CREATE TABLE downloads (
      id INTEGER PRIMARY KEY, url TEXT, title TEXT, format TEXT,
      status TEXT, file_path TEXT, created_at TEXT, completed_at TEXT
  );
  ```
- `src/config.rs` - TOML 配置:
  ```toml
  [general]
  download_dir = "~/Videos"
  concurrent_downloads = 3

  [defaults]
  video_quality = "best"
  audio_format = "m4a"
  download_subtitles = false

  [clipboard]
  enabled = true
  ```
- `src/clipboard.rs` - `arboard` 轮询剪贴板，匹配 YouTube/多站点 URL 正则

---

### Phase 6: 音频下载 & 字幕 & 后处理

**目标**: 音频单独下载、字幕下载、ffmpeg 后处理。

**音频模式**:
- yt-dlp `-x --audio-format mp3` 或 `--audio-format m4a`
- UI 增加 "仅音频" 切换开关
- 格式选择: MP3 / M4A / FLAC / OPUS

**字幕**:
- yt-dlp `--write-subs --write-auto-subs --sub-langs "zh-Hans,en"`
- UI 字幕语言多选

**后处理**:
- 下载完成后检测 ffmpeg
- 可选: 自动转码为 MKV / MP3 / 其他格式
- `src/downloader/post_process.rs` - ffmpeg subprocess

---

### Phase 7: 多站点支持 & 浏览器扩展集成

**目标**: 不局限于 YouTube，支持任意 yt-dlp 支持的站点。

- URL 输入通用化（不再硬编码 YouTube）
- 可选: 浏览器扩展（独立小项目）向本地 HTTP server 发送 URL
- `src/downloader/yt_dlp.rs` 中 `fetch_info` 支持任意 URL

---

## 验证方案

1. **编译**: `cargo build` / `cargo build --release` 无警告
2. **类型检查**: `cargo clippy` 通过
3. **Cookie 提取**: 在 Chrome/Edge/Firefox 上测试，验证生成的 Netscape cookie 文件可被 yt-dlp 使用
4. **视频下载**: 测试最佳质量、指定分辨率、仅音频、播放列表
5. **并发**: 同时添加 3 个下载任务，验证进度更新正确
6. **历史记录**: 下载完成后检查 SQLite 记录
7. **剪贴板**: 复制 YouTube URL，验证自动检测
8. **UI 响应**: 长时间下载时 UI 不卡顿

---

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| windows-rs 的 DPAPI/CNG API 使用复杂 | 先做 Firefox 路径验证整体架构，再做 Chromium |
| Chrome v20 加密密钥可能随版本变化 | 保留 chromelevator 作为备用方案 |
| Iced 的异步 + subprocess 组合可能复杂 | 使用 `tokio::process::Command` 在 spawn 中处理 |
| 项目范围大，第一版可能过于庞大 | Phase 1-3 作为 MVP，其余按优先级迭代 |
