---
name: 自动化测试要求
description: 所有功能必须通过自动化测试验证，不接受手动测试。测试结果用独立页面展示。
type: feedback
---

**规则：** 每次完成功能开发后，必须使用自动化测试脚本自行验证所有功能，不能把用户当成测试人员。

**Why:** 用户明确表示"请不要拿我当成测试/代码审查人员"。

**How to apply:**
- 在 `frontend/src/pages/TestRunner.tsx` 中维护自动化测试用例
- 测试结果在独立的 Test Runner 页面（侧边栏 Flask 图标）展示
- 每次修复 bug 或新增功能后，先运行自动化测试确认通过
