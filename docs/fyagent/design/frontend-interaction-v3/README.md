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

- 已完成：11 张原型及人类批准、A-to-A 审计、6 路由 / 11 状态实现、全仓门禁、浏览器 132/132、macOS 调试包与只读 UAT、飞书 M1 文档和群消息回读。
- 当前：`local_candidate_pass_windows_delivered_not_executed`；运行代码冻结在 `0ad9a7e122d8877f4ab6d648ac187cdb037ba444`。Windows UAT 交接包已送达 AIMaster，但没有认证执行入口和 Windows 本机返回证据；分支后续提交仅记录任务与交付证据。
- 未完成：Windows-native 执行与 fresh validation、严格 1:1 pixel diff、push、PR、main 合并、正式签名、Release 和生产部署。
- 下一门禁：AIMaster 本机 Codex 在隔离 fixture 中执行后，返回 nonce/hash 绑定的 fresh receipt、截图或日志与应用失败路径证据；macOS、浏览器、Taildrop 送达或远程可达均不能替代。
