# 2026-08-26｜A-to-A 审计完成与并行实现启动（M1 前置）

> SUPERSEDED_DO_NOT_EXECUTE（2026-08-26）：V3.1 任务已接管执行。本文仅保留历史证据，旧候选、Windows 等待与对外发送流程已停止。

## 历史背景

本轮不是重做 FyAgent 的主题颜色，而是依据产品经理线框图重新安排按钮、导航和配置入口。11 张已确认高保真图继续作为唯一视觉基线；它们对应 6 个现有页面和 11 个交互状态，不会被拆成 11 个路由。

## 本轮已确认事实

1. 分支：`codex/frontend-interaction-v3-20260825`，基于 `origin/main` 独立推进；当前没有修改 main、没有合并、没有发布。
2. A-to-A 双审计已经完成：
   - Gemini：`gemini-3.7-flash-high / high`，完成视觉、交互、窄屏和键盘可用性审计。
   - Grok：用户期望的 Grok 4.7 当前本机不可用；本次透明使用最新已验证的 `vibekey/grok-4.6 / high` 完成能力矩阵和过度设计挑战，绝不冒充 4.7。
   - Codex：最终方案、实现整合、缺陷修复和验收总负责。
3. 四条 Codex 实现任务全部固定为 `gpt-5.6-sol / max`：
   - 左侧壳层与导航；
   - AI 软件扫描与 Agent 四段配置；
   - Models / Skills / MCP 管理页；
   - Prompts / Memory 管理页。
4. 主整合工作树已经完成依赖 bootstrap 和 macOS 系统预检；Memory 的“复制”行为已改为复制当前正文草稿，长期记忆与每日记忆均覆盖，聚焦测试 18/18 通过、ESLint 通过。

## 最终实现方案

- 保留 `/agents`、`/models`、`/skills`、`/mcp`、`/prompts`、`/memory` 六个一级路径和页面 keep-alive。
- 左侧导航分成「AI软件配置」「配置管理」「记忆模块」三组；配置管理内含模型、Skills、MCP、提示词。
- `/agents` 聚合 7 个现有 Install Readiness 查询；进度按 settled 数量计算，`unknown` 不能显示为“未安装”。
- 单 Agent 配置使用 `?target=<agentId>&section=models|skills|mcp|prompts`，保留返回目录与进入全局管理页的路径。
- Skills/MCP 沿用现有写入、失效、权威回读；模型和提示词按真实能力显示 direct / assisted / unsupported，不允许伪保存。
- 不新增扫描取消协议、后台守护进程、全局 selected-agent store、通用模型分配接口、第二套 assignment store、嵌套路由或新 UI 主题。

## 必须守住的映射

- TRAE：扫描和 Skills/MCP 使用 `trae-work`，模型管理目标使用 `trae`。
- Claude Code：扫描使用 `claude-code`，Skills/MCP、模型和提示词使用 `claude`。
- Qoder、TRAE、WorkBuddy 当前没有 PromptAppId，提示词区域必须诚实显示不支持，不能出现假开关。

## 当前证据与边界

- 已完成：高保真 11 图、产品合同冻结、A-to-A code audit、四条实现任务启动、macOS 开发环境预检、Memory 聚焦实现与测试。
- 进行中：壳层、Agent 扫描/四段配置、五个管理页面的并行实现。
- 尚未完成：整合后的 V2 全门禁、浏览器运行态截图、macOS 新包 UAT、Windows 原生安装与 UAT。
- 群内此前发送的 11 张图片仍是本轮唯一视觉基线；文档上方“11 张高保真页面与逐页确认点”章节包含完整图片和逐页说明。

## 人类协作入口

当前没有需要人类阻断式决策的事项。实现中如发现后端真实能力与已确认原型发生不可兼容冲突，会先在本文档补齐背景、选项、推荐和影响，再在群里发送可直接决策的摘要；在所有本地方案耗尽前不会频繁打断。
