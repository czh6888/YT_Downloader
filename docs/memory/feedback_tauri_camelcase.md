---
name: Tauri v2 invoke 参数用 camelCase + Config 处理
description: 前端 invoke 参数用 camelCase；Config 从 AppState 加载而非前端传入
type: feedback
---

**规则 1：** 前端调用 `invoke()` 时参数名用 camelCase，Rust 侧 struct 加 `#[serde(rename_all = "camelCase")]`。

**规则 2：** 前端传浏览器名（如 `['Chrome']`），Rust 侧转为 `--cookies-from-browser Chrome`。

**规则 3（关键）：** Config 用于 TOML 持久化（snake_case），不能加 camelCase rename。Config 从 AppState/磁盘加载，不通过 invoke 参数传入。
