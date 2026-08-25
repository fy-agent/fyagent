# 前端交互重构 v3 - 高保真原型集

> 视觉方向：`CONFIRMED_BY_USER_2026-08-25`
> 原型集状态：`APPROVED_FOR_IMPLEMENTATION_2026-08-26`
> 说明：本目录记录高保真设计候选，不代表代码已实现、运行页面已验收或产品已发布。

## 设计来源

- 产品讨论：[新录音 2](https://bcnntymbwnto.feishu.cn/docx/WXTZdpDUHo6Q2fxyjXSc3242n2a)
- 线框原型：`未命名原型.zip` 内的 `画布.pdf`，共 10 页
- 代码基线：`origin/main` 的 `91a66254a0f7190fbc500591d188f52cde74fc7e`

## 已锁定方向

- 产品类型：AI / Developer Tool 桌面控制中心。
- 不更换现有主题；继续使用 FyAgent 的蓝色 liquid-glass 体系。
- 核心色板：`#324D69`、`#567495`、`#7B99B8`、`#9DDCFF`、`#F6FBFF`。
- 本轮重点是信息架构、按钮位置、入口层级和状态反馈，而不是重新做品牌视觉。
- 主导航改为左侧三组：`AI软件配置`、`配置管理`、`记忆模块`。
- Agent 内使用 `模型 / Skills / MCP / 提示词` 四段选配入口。
- 线框第 5 页的“进入 Skills 管理”按讨论原文纠正为“进入提示词管理”。

## 页面清单

1. [AI 软件扫描完成](./01-ai-software-scan-complete.png)
2. [AI 软件扫描中](./02-ai-software-scanning.png)
3. [Agent 模型选配](./03-agent-model-selection.png)
4. [Agent Skills 选配](./04-agent-skills-selection.png)
5. [Agent MCP 选配](./05-agent-mcp-selection.png)
6. [Agent 提示词选配](./06-agent-prompts-selection.png)
7. [模型管理](./07-model-management.png)
8. [Skills 管理](./08-skills-management.png)
9. [MCP 管理](./09-mcp-management.png)
10. [提示词管理](./10-prompts-management.png)
11. [记忆模块](./11-memory-module.png)

## 配套材料

- [产品评审包](./REVIEW_PACKET.md)：背景、方案、逐页确认点与反馈格式。
- [提示词与协作工作流](./PROMPTS_AND_WORKFLOW.md)：高保真生成约束、页面变量、A-to-A 分工和质量门禁。

## 当前边界

- 已完成：线框和讨论内容读取、高保真视觉方向确认、11 张原型及其人类批准、协作提示词、评审材料和开发规划。
- 当前：`planning_ready_pending_approval`，等待实施计划与 Grok/Gemini 真实模型路由的一次最终批准。
- 未开始：前端代码重构、运行态截图、交互测试、像素对比、合并、发布。
- 下一门禁：批准后启动 Trellis 任务与 A-to-A 实现；代码仍只允许落在 `codex/frontend-interaction-v3-20260825`。
