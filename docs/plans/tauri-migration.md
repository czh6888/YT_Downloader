# Tauri + React + Mantine 迁移计划

## Context

当前项目使用 Iced 0.13 作为 GUI 框架，UI 表现力严重受限（原生控件拼接、无法做现代设计）。目标是迁移到 Tauri + React + Mantine，保留所有 Rust 后端逻辑（yt-dlp 子进程、Cookie 提取、配置管理、SQLite 历史），用 React + Mantine 重写整个前端 UI 层，使 UI 效果对齐甚至超越 Stacher7。

## 架构总览

```
┌─────────────────────────────────────────────────────┐
│                    前端 (React SPA)                   │
│  React 18 + TypeScript + Mantine 7 + Recharts        │
│  - 下载页 / 队列页 / 历史页 / 设置页 / 格式弹窗       │
├─────────────────────────────────────────────────────┤
│                 Tauri IPC Bridge                      │
│  Tauri Commands (Rust) ←→ JavaScript invoke()        │
├─────────────────────────────────────────────────────┤
│                  后端 (Rust lib)                      │
│  src-tauri/src/main.rs  → Tauri setup + commands     │
│  downloader/            → yt-dlp 子进程/格式/进度     │
│  cookies/               → Chrome/Edge/Firefox 提取    │
│  config.rs              → TOML 配置                   │
│  history.rs             → SQLite 历史                 │
└─────────────────────────────────────────────────────┘
```

## 目录结构

```
yt-downloader/
├── Cargo.toml              # 修改：移除 iced，增加 tauri 依赖
├── src/
│   ├── lib.rs              # 修改：移除 app/ui 模块导出
│   ├── config.rs           # 保留不变
│   ├── history.rs          # 保留不变，扩展字段
│   ├── downloader/         # 全部保留，新增 streaming 接口
│   │   ├── mod.rs
│   │   ├── yt_dlp.rs       # 修改：增加 stream_download 方法
│   │   ├── formats.rs      # 保留不变
│   │   ├── progress.rs     # 保留不变
│   │   ├── post_process.rs # 保留不变
│   │   └── cookies/        # 全部保留
│   └── bin/                # 旧 binary 保留
├── src-tauri/              # 新建：Tauri 后端
│   ├── Cargo.toml          # Tauri + tauri-plugin dependencies
│   ├── tauri.conf.json     # Tauri 配置（窗口、权限、Updater）
│   ├── build.rs
│   ├── capabilities/       # Tauri v2 安全能力配置
│   └── src/
│       ├── main.rs         # Tauri entry + plugin setup
│       ├── commands.rs     # 所有 #[tauri::command] 函数
│       ├── state.rs        # Tauri State 管理（Config, History, Downloads）
│       └── download_mgr.rs # 并发下载管理器（暂停/恢复/取消）
├── frontend/               # 新建：React 前端
│   ├── package.json        # React + Vite + Mantine + Recharts
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx        # React entry
│       ├── App.tsx         # Router + Layout + Theme
│       ├── api/            # Tauri invoke 封装
│       │   └── tauri.ts
│       ├── components/     # 通用组件
│       │   ├── Layout.tsx
│       │   ├── Sidebar.tsx
│       │   ├── FormatDialog.tsx
│       │   ├── ProgressBar.tsx
│       │   ├── SpeedChart.tsx
│       │   └── TaskCard.tsx
│       ├── pages/          # 页面
│       │   ├── Download.tsx
│       │   ├── Queue.tsx
│       │   ├── History.tsx
│       │   └── Settings.tsx
│       ├── hooks/          # React hooks
│       │   ├── useConfig.ts
│       │   ├── useDownloads.ts
│       │   └── useClipboard.ts
│       └── store/          # 状态管理（Zustand）
│           └── appStore.ts
```

---

## Phase 1: 项目骨架搭建（1-2 小时）

### 1.1 修改根 Cargo.toml
- 移除 `iced` 依赖
- 添加 `tauri = "2"` 依赖
- 添加 `tauri-plugin-dialog`, `tauri-plugin-notification`, `tauri-plugin-shell` 依赖
- `src/main.rs` 改为空入口，所有逻辑移到 `src-tauri/`

### 1.2 创建 src-tauri/ 目录
- `src-tauri/Cargo.toml`：依赖 `tauri 2`, `serde`, `serde_json`, 以及父 crate 的 `yt-downloader` lib
- `src-tauri/tauri.conf.json`：
  ```json
  {
    "productName": "yt-downloader",
    "version": "0.1.0",
    "build": { "frontendDist": "../frontend/dist" },
    "app": {
      "withGlobalTauri": true,
      "windows": [{
        "title": "YT Downloader",
        "width": 1200, "height": 800,
        "decorations": false,
        "transparent": true,
        "resizable": true
      }]
    },
    "bundle": {
      "targets": ["msi", "nsis"],
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "plugins": {
      "updater": {
        "active": true,
        "endpoints": ["https://github.com/czh6888/YT_Downloader_Rust_Plus/releases/download/updater.json"],
        "dialog": true
      }
    }
  }
  ```
- `src-tauri/build.rs`：标准 `tauri-build`
- `src-tauri/capabilities/default.json`：允许 shell、dialog、notification

### 1.3 创建 frontend/ 目录
- `package.json`：
  ```json
  {
    "dependencies": {
      "@mantine/core": "^7.17",
      "@mantine/hooks": "^7.17",
      "@mantine/notifications": "^7.17",
      "@mantine/dropzone": "^7.17",
      "@tabler/icons-react": "^3.5",
      "react": "^18.3",
      "react-dom": "^18.3",
      "recharts": "^2.13",
      "zustand": "^4.5"
    },
    "devDependencies": {
      "@types/react": "^18.3",
      "@types/react-dom": "^18.3",
      "@vitejs/plugin-react": "^4.3",
      "typescript": "^5.5",
      "vite": "^5.4",
      "postcss": "^8.4",
      "postcss-preset-mantine": "^1.15"
    }
  }
  ```
- `vite.config.ts`、`tsconfig.json`、`index.html`、`postcss.config.cjs`

### 1.4 安装 Node.js 依赖
- `cd frontend && npm install`

---

## Phase 2: Tauri Commands 后端（4-6 小时）

### 2.1 `src-tauri/src/state.rs` — 全局状态

```rust
pub struct AppState {
    pub config: RwLock<Config>,
    pub history: RwLock<HistoryManager>,
    pub download_mgr: RwLock<DownloadManager>,
}
```

### 2.2 `src-tauri/src/download_mgr.rs` — 下载管理器

核心职责：
- 管理活跃下载任务（`HashMap<u64, DownloadTask>`)
- 每个任务持有 `Arc<AtomicBool>` cancel flag
- 支持暂停/恢复（yt-dlp 暂停 = 发送 SIGSTOP，恢复 = SIGCONT；Windows 用 SuspendThread/ResumeThread）
- 并发限制（从 config 读取 max_concurrent）
- 通过 `app_handle.emit()` 推送进度事件到前端

```rust
pub struct DownloadManager {
    tasks: HashMap<u64, DownloadTask>,
    max_concurrent: usize,
    next_id: u64,
}
```

### 2.3 `src-tauri/src/commands.rs` — 所有 IPC 命令

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `check_yt_dlp` | 无 | `Option<Vec<String>>` | 检测 yt-dlp |
| `fetch_info` | url, cookie_args | `Result<FormatList>` | 获取格式列表 |
| `start_download` | url, format_ids, save_dir, audio_only, ... | `TaskId` | 开始下载 |
| `pause_download` | task_id | `Result<()>` | 暂停 |
| `resume_download` | task_id | `Result<()>` | 恢复 |
| `cancel_download` | task_id | `Result<()>` | 取消 |
| `get_download_state` | 无 | `Vec<TaskInfo>` | 获取所有任务状态 |
| `load_config` | 无 | `Config` | 加载配置 |
| `save_config` | Config | `Result<()>` | 保存配置 |
| `get_history` | query | `Vec<HistoryEntry>` | 获取历史 |
| `delete_history` | id | `Result<()>` | 删除历史 |
| `clear_history` | 无 | `Result<()>` | 清空历史 |
| `extract_cookies` | browser | `CookieResult` | 提取 Cookie |
| `detect_ffmpeg` | 无 | `bool` | 检测 ffmpeg |
| `open_folder` | path | `Result<()>` | 打开文件夹 |
| `show_in_explorer` | file_path | `Result<()>` | 定位文件 |
| `delete_file` | file_path | `Result<()>` | 删除本地文件 |
| `get_clipboard` | 无 | `String` | 读取剪贴板 |
| `toggle_theme` | 无 | `String` | 切换主题 |

### 2.4 `src-tauri/src/main.rs` — Tauri 入口

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            check_yt_dlp, fetch_info, start_download,
            pause_download, resume_download, cancel_download,
            get_download_state, load_config, save_config,
            get_history, delete_history, clear_history,
            extract_cookies, detect_ffmpeg, open_folder,
            show_in_explorer, delete_file, get_clipboard,
            toggle_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri");
}
```

### 2.5 修改 `src/downloader/yt_dlp.rs` — 增加流式下载

新增 `stream_download` 方法，替代当前的 `download()` 方法：

```rust
/// 流式下载，通过 channel 推送进度事件。
pub async fn stream_download(
    url: &str,
    cookie_args: &[String],
    format: &str,
    save_dir: &str,
    audio_only: bool,
    audio_format: &str,
    subtitle_langs: &str,
    config: &Config,
    cancel_flag: Arc<AtomicBool>,
    progress_tx: tokio::sync::mpsc::Sender<ProgressEvent>,
) -> DownloadResult { ... }
```

`ProgressEvent` 枚举：
```rust
pub enum ProgressEvent {
    Progress { pct: f64, speed: Option<f64>, eta: Option<u64>, downloaded: u64, total: Option<u64> },
    Log(String),
    Title(String),
    FilePath(String),
}
```

同时支持 `output_template` 和所有 config 中的 yt-dlp 参数。

### 2.6 修改 `src/history.rs` — 扩展字段

新增字段以支持媒体库 UI：
- `thumbnail: Option<String>` — 缩略图 URL/base64
- `duration: Option<f64>` — 视频时长（秒）
- `uploader: String` — 上传者
- `file_size: Option<u64>` — 文件大小

---

## Phase 3: React 前端（8-12 小时）

### 3.1 基础架构
- `App.tsx`：`<MantineProvider>` + `<NotificationProvider>` + `<AppShell>` 布局
- 自定义标题栏（无边框窗口 + 拖拽区域 + 最小化/关闭按钮）
- 侧边栏导航：下载 / 队列 / 历史 / 设置（对应 `Page` enum）
- 明暗主题切换（Mantine 内置）

### 3.2 下载页 (`pages/Download.tsx`)
- URL 输入框（`TextInput`，支持粘贴检测）
- 浏览器 Cookie 选择（`Select`：Chrome / Edge / Firefox / 无）
- "获取信息" 按钮 → 调用 `fetch_info`
- 视频信息卡片（缩略图 + 标题 + 频道）
- 格式摘要 + "选择格式" 按钮
- 格式弹窗（`components/FormatDialog.tsx`）
  - Mantine `Modal` 组件
  - 筛选标签（`Tabs`）：视频 / 音频 / 音视频 / 全部
  - 搜索框（`TextInput`）
  - 格式列表（`ScrollArea` + 自定义行组件）
  - 多选模式（checkbox）vs 单选模式（radio）
- 立即下载 / 加入队列按钮

### 3.3 队列页 (`pages/Queue.tsx`)
- 任务卡片列表（`TaskCard` 组件）
- 每个卡片显示：缩略图、标题、进度条、速度曲线（Recharts）、ETA、状态
- 操作按钮：暂停/恢复/取消/移除
- 空状态提示

### 3.4 历史页 (`pages/History.tsx`)
- 搜索框（标题/URL）
- 历史卡片网格（缩略图 + 标题 + 日期 + 状态）
- 单条删除 / 全部清空
- "Show in Explorer" 按钮
- 空状态

### 3.5 设置页 (`pages/Settings.tsx`)
- Tab 分组：General / Download / Extractor / Post-Processing / Advanced
- 每个 Tab 用 Mantine 表单组件渲染对应配置字段
- 保存按钮 → 调用 `save_config`
- 主题切换 / 语言切换

### 3.6 Tauri API 封装 (`api/tauri.ts`)

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export const api = {
  checkYtDlp: () => invoke('check_yt_dlp'),
  fetchInfo: (url: string, cookieArgs: string[]) => invoke('fetch_info', { url, cookieArgs }),
  startDownload: (params: DownloadParams) => invoke('start_download', { params }),
  // ... all commands
};

// Event listener for real-time progress
export const onDownloadProgress = (
  callback: (event: ProgressEvent) => void
) => listen('download-progress', ({ payload }) => callback(payload));
```

### 3.7 状态管理 (`store/appStore.ts` — Zustand)

```typescript
interface AppState {
  config: Config;
  tasks: Task[];
  history: HistoryEntry[];
  formats: FormatInfo[];
  selectedFormats: string[];
  currentTheme: 'light' | 'dark';
  language: 'en' | 'zh';
  // actions
  setConfig: (c: Config) => void;
  addTask: (t: Task) => void;
  updateTask: (id: number, updates: Partial<Task>) => void;
  // ...
}
```

### 3.8 速度曲线 (`components/SpeedChart.tsx` — Recharts)

```tsx
<AreaChart data={speedHistory}>
  <Area type="monotone" dataKey="speed" stroke="#3b82f6" fill="#3b82f620" />
  <XAxis dataKey="time" hide />
  <YAxis hide />
  <Tooltip />
</AreaChart>
```

---

## Phase 4: 系统集成与打磨（4-6 小时）

### 4.1 自定义窗口标题栏
- CSS `-webkit-app-region: drag` 实现拖拽
- 最小化/关闭按钮调用 `@tauri-apps/api/window` API

### 4.2 剪贴板监听
- Tauri sidecar 或 Rust 后台线程定期读取剪贴板
- 通过 `app_handle.emit()` 推送新 URL 到前端
- 前端自动填充 URL 输入框

### 4.3 桌面通知
- 下载完成时调用 `tauri-plugin-notification`
- 显示标题 + "下载完成" 消息

### 4.4 系统托盘（可选，后续版本）
- `tauri-plugin-tray`
- 托盘图标显示下载状态

### 4.5 自动更新
- Tauri Updater 插件
- 检查 GitHub Releases 的 `updater.json`

---

## Phase 5: 验证与发布（2-3 小时）

### 5.1 开发模式验证
```bash
# 安装 Node 依赖
cd frontend && npm install
# 开发模式（热重载）
cargo tauri dev
```

### 5.2 生产构建
```bash
cargo tauri build
```
输出：`src-tauri/target/release/bundle/msi/yt-downloader.msi` 或 `nsis/yt-downloader-setup.exe`

### 5.3 功能验证清单
1. 启动 → 自定义标题栏 + 侧边栏 + 亮色主题渲染
2. 粘贴 URL → 获取信息 → 格式弹窗 → 选择格式 → 下载
3. 进度条实时更新 + 速度曲线正常绘制
4. 暂停/恢复/取消 → 功能正确
5. 队列并发 → 同时下载 N 个任务
6. 下载完成 → 历史记录出现 + 桌面通知
7. 设置页 → 修改配置 → 保存 → 重启后保留
8. 历史页 → 搜索/删除 → 功能正确
9. 主题切换 → 明暗切换流畅
10. 打包 → exe 正常运行

---

## 关键设计决策

### 为什么不保留 Iced
- Iced 的 Canvas API 不适合做复杂的 UI（速度曲线 flickering、样式受限）
- 没有成熟的组件生态（弹窗、通知、表单验证都要手写）
- CSS 的排版能力是原生 GUI 无法比拟的

### 为什么用 Tauri 而非 Electron
- 我们本身就是 Rust 项目，Tauri 的 backend 是 Rust native
- 包体积 ~15MB vs Electron ~150MB
- 内存 ~100MB vs Electron ~300MB
- 现有 Rust 代码可以直接复用为 lib

### 状态管理为什么用 Zustand
- 比 Redux 简单，比 Context 强大
- 适合中小型应用（我们这个规模正好）
- TypeScript 友好

### 为什么前端不直接嵌入 src/ 而是独立 frontend/
- 保持前后端分离，前端可以用 Vite HMR 独立开发
- Tauri 的 `frontendDist` 指向 `frontend/dist`，构建产物独立
- 符合 Stacher7 的 `.vite/renderer/` 分离思路

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Tauri v2 文档不完善 | 开发效率 | 参考官方示例 + Discord 社区 |
| Windows WebView2 版本过低 | 兼容性 | Tauri 自动检测，提示用户更新 |
| 前端重写工作量大 | 周期长 | 分 Phase 逐步交付，每 Phase 可独立验证 |
| 管理员权限 + 自定义窗口 | 冲突 | 管理员权限在 Tauri 的 `requireAdministrator` 配置中处理 |
