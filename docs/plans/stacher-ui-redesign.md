# 实现计划：Stacher7 风格界面重写

## 关键发现

### 进度条不工作的根因
yt-dlp 的进度信息（`[download] 45.2% of ...`）输出到 **stderr** 不是 stdout。当前 `start_download()` 只捕获了 stdout，需要同时捕获 stderr。

### Stacher7 UI 分析（从截图+changelog）

**下载页面（默认列表视图）：**
- 顶部：URL 输入框 + 图标工具栏 + 格式选择 + 下载按钮
- 第二行：视图切换按钮（网格 | 列表 | RSS | 终端）+ 过滤器 + 下载路径
- 列表列：缩略图 | 标题 | 进度条 | 总计 | 速度(含sparkline) | 预计剩余 | 已用时间 | 展开按钮
- 底部：全局进度条

**日志视图：**
- 左侧面板：缩略图 + 迷你进度条 + 标题
- 右侧主区域：终端样式黑色背景，显示 yt-dlp 原始日志
- 日志中显示：Download ID、Configuration、Arguments、yt-dlp 输出行

**设置页面（多个 tab）：**
- General：语言、主题、下载目录、最大并发、剪贴板监控
- Download/Format：格式选择、音频格式、合并格式、输出模板
- Subtitles：下载字幕、嵌入字幕、字幕语言
- Metadata：嵌入元数据、嵌入缩略图
- Post Processing：预处理/后处理脚本
- Advanced：Verbose 模式、SponsorBlock、速率限制、重试次数、下载档案、Abort on Error、Ignore Errors、Custom Headers、Extractor Args、Post Processor Args

---

## 实施步骤

### 1. 修复进度条 — `src/downloader/yt_dlp.rs`
**最小改动**：在 `start_download()` 中同时捕获 stderr 和 stdout
- `.stderr(std::process::Stdio::piped())` 合并到 stdout 或使用 `Stdio::piped()` 分别处理
- 将 stderr 行也发送到 `TaskLog`，让 `parse_progress` 能解析到进度信息

### 2. 增强 DownloadTask — `src/app.rs`
新增字段：
- `downloaded_bytes: u64` — 已下载字节
- `total_bytes: Option<u64>` — 总字节
- `speed_bytes: Option<f64>` — 实时速度 bytes/s
- `eta_seconds: Option<u64>` — 预计剩余秒数
- `elapsed_seconds: u64` — 已用秒数
- `started_at: Option<Instant>` — 开始时间

### 3. ViewMode 枚举 — `src/app.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    Log,
}
```
新增 `Message::ToggleViewMode`

### 4. 合并下载+队列页 — `src/app.rs`
- 移除 `Page::Queue`，合并到 `Page::Downloads`
- 重写 `downloads_view()` 为 Stacher7 风格表格
- 每行显示：缩略图 | 标题 | 进度条 | 总计 | 速度 | ETA | 已用时间 | 操作按钮

### 5. 列表视图 — `src/app.rs`
- 列标题行：缩略图 | 标题 | 进度 | 总计 | 速度 | 预计剩余 | 已用时间 | 操作
- 进度条显示 `task.progress`，同时显示文字 `45%`
- 速度显示格式化：`2.5 MB/s`
- ETA 显示：`00:02`
- 已用时间：`00:00:04`（每秒 tick 更新）

### 6. 日志视图 — `src/app.rs`
- 左右分栏：左侧缩略图+迷你进度，右侧终端窗口
- 终端区域：深色背景 + 等宽字体 + scrollable
- 显示 `task.log` 内容

### 7. 顶部工具栏
- URL 输入（大，带 placeholder）
- 格式选择下拉
- 下载按钮
- 第二行：视图切换按钮（列表/终端图标）+ 下载路径选择

### 8. 底部全局进度条
- 所有活跃下载的平均进度
- 底部固定条

### 9. 设置页重写 — `src/app.rs`
改为 tab 式多页设置：

**General tab：**
- 语言、主题、下载目录、最大并发、剪贴板监控

**Download tab：**
- 偏好格式：下拉 (最佳质量, 4K, 1080p, 720p, 480p, 360p, 仅音频)
- 仅音频格式：下拉 (MP3, M4A, FLAC, OPUS)
- 合并输出格式：下拉 (MP4, MKV, WEBM)
- 输出模板：文本输入 `%(title)s [%(id)s].%(ext)s`

**Subtitle tab：**
- 下载字幕：复选框
- 嵌入字幕：复选框
- 字幕语言：文本输入

**Metadata tab：**
- 嵌入元数据：复选框
- 嵌入缩略图：复选框

**Advanced tab：**
- Verbose 模式：复选框
- Abort on Error：复选框
- Ignore Errors：复选框
- 速率限制：文本输入
- 重试次数：数字输入
- 下载档案（跳过已下载）：复选框
- 自定义参数：文本输入（每行一个参数）

### 10. 计时器
- `Subscription` 中增加 `iced::time::every(Duration::from_secs(1))` → `Message::Tick`
- `Message::Tick` 更新所有下载中任务的 `elapsed_seconds`

### 11. 清理
- 移除旧 `Page::Queue` 相关代码
- 移除旧的 `queue_view()` 方法
- 移除不再使用的 `audio_only`/`audio_format` 从主下载页

---

## 验证方法

1. `cargo build --release` 编译无错误
2. 下载视频 → 进度条实时更新，速度/ETA 正确
3. 列表视图正常显示所有列
4. 切换到日志视图 → 终端样式显示
5. 设置页可配置所有选项并保存
6. 重启 app → 配置保留
