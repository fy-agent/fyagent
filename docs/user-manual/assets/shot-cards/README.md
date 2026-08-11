# FyAgent 截图拍摄任务卡

这里的 15 张任务卡描述下一轮真实桌面截图，不是可以提前嵌入手册的占位图片。截图只有在 FyAgent 桌面壳实际运行、画面与当前源码一致后才能交付；浏览器 mock、静态 HTML 和生成式图片都不能冒充运行时证据。

## 统一拍摄规则

- 基准画布为 1600×1000，系统缩放 100%，同一语种保持相同窗口尺寸、主题和数据。
- 先拍中文裸文件名，再按需要拍 `-en`、`-ja`。本轮审计认定所有含界面文字的关键图都需要三语版本。
- 使用虚构的供应商、端点、模型、会话和路径；不能出现真实 API Key、账号、用户名、内网地址、工作目录或业务数据。
- 必须能看到 FyAgent 当前身份；不要裁出能够误导产品归属的碎片图。
- 不在截图里后期改按钮、数字或状态。只允许无损裁切和压缩，修改后必须重新做视觉复核。
- 每张图记录应用提交、系统、语言、主题、缩放、文件 SHA-256 和拍摄日期。验收证据等级至少为 `runtime_screenshot`。

## 任务清单

| # | 章节 | 任务卡 | 目标文件 |
|---:|---|---|---|
| 001 | 1.3 | [主界面全景](./001-main-overview.md) | `main-overview.png` |
| 002 | 1.4 | [添加供应商](./002-quickstart-add-provider.md) | `quickstart-add-provider.png` |
| 003 | 1.5 | [通用设置](./003-settings-general.md) | `settings-general.png` |
| 004 | 2.1 | [工具安装区](./004-about-tool-install.md) | `about-tool-install.png` |
| 005 | 2.2 | [冲突诊断](./005-about-diagnose-conflict.md) | `about-diagnose-conflict.png` |
| 006 | 3.1 | [供应商列表](./006-provider-card-list.md) | `provider-card-list.png` |
| 007 | 3.3 | [编辑供应商](./007-provider-edit-form.md) | `provider-edit-form.png` |
| 008 | 4.1 | [MCP 管理](./008-mcp-panel.md) | `mcp-panel.png` |
| 009 | 4.2 | [Prompts 编辑器](./009-prompts-editor.md) | `prompts-editor.png` |
| 010 | 4.3 | [Skills 管理](./010-skills-panel.md) | `skills-panel.png` |
| 011 | 4.4 | [会话列表](./011-sessions-list.md) | `sessions-list.png` |
| 012 | 4.6 | [WorkBuddy 连接](./012-workbuddy-connection.md) | `workbuddy-connection.png` |
| 013 | 4.6 | [WorkBuddy 模型](./013-workbuddy-models.md) | `workbuddy-models.png` |
| 014 | 5.1 | [代理服务](./014-proxy-service.md) | `proxy-service.png` |
| 015 | 5.3 | [故障转移队列](./015-failover-queue.md) | `failover-queue.png` |

当前主机在 Visual Studio 2022 Developer PowerShell 中已经通过 `cl.exe` 与 WebView2 预检。这批任务卡可以进入独立拍摄轮次；仍应从 001、002 两张 README P0 画面开始，并在三语数据、窗口尺寸和脱敏状态固定后再批量拍摄。
