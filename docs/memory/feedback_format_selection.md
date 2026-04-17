---
name: Fetch 后自动弹窗 + 单选最佳格式
description: Fetch 成功后自动打开格式选择弹窗，默认只选单个最佳格式，不要选中所有 video formats
type: feedback
---

**规则 1：** Fetch 成功后自动调用 `setFormatDialogOpen(true)` 打开格式选择弹窗。

**规则 2：** 只选 1 个最佳格式，不是全选。按 height 降序排序选第一个。没有 video 就选 combined，再没有选 'best'。

**Rule 3：** 质量下拉框切换时也只选 1 个匹配格式。
