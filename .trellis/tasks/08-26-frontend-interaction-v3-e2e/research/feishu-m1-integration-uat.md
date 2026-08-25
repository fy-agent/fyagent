# 2026-08-26｜前端交互重构 v3 M1 集成、全门禁与 macOS 只读 UAT

## 一句话结论

v3 已在独立分支完成 6 路由 / 11 状态的前端交互重排，最新候选提交为 `0ad9a7e1`。全仓门禁退出码 0，浏览器交互验收 132/132，最终 macOS 调试包可精确启动。当前可进入 Windows 原生验证，但仍不是 main、Release 或生产版本。

## 历史背景与范围

本轮不重做 FyAgent 既有蓝色 liquid-glass 主题，而是依据产品经理线框图调整按钮位置、导航层级、配置入口和状态反馈。11 张已确认高保真图对应 6 个真实路由和 11 个代表状态，不新增 11 个独立页面，也不新造另一套业务状态。

- 六个路由：`/agents`、`/models`、`/skills`、`/mcp`、`/prompts`、`/memory`。
- 三个一级导航：AI软件配置、配置管理、记忆模块；仅配置管理可展开。
- Agent 配置深链：`/agents?target=<agentId>&section=models|skills|mcp|prompts`。
- 七路扫描复用既有 Install Readiness 查询；`unknown` 与“未安装”严格分开，不提供虚假的取消扫描协议。
- Skills/MCP 继续执行真实写入、失效、权威回读；模型只做能力投影；无 PromptAppId 的 Agent 显示真实不支持态。
- Memory 的“复制”复制当前编辑正文，不复制文件路径。

## A-to-A 协作事实

- Gemini：`gemini-3.7-flash-high / high`，完成视觉、交互、窄屏与键盘可用性审计。
- Grok：本机没有可用的 Grok 4.7；透明改用已验证的 `vibekey/grok-4.6 / high` 完成能力矩阵与过度设计挑战，未冒充 4.7。
- Codex：所有实现与复验任务均使用 `gpt-5.6-sol / max`；Codex 负责最终整合、修复、门禁和证据分层。

## 已落地方案

1. 左侧壳层改为三一级入口、单可展开组，同时保留六页面 keep-alive。
2. `/agents` 提供 7 Agent 目录、按需扫描、扫描中/完成/空/失败/unknown 状态。
3. 单 Agent 内提供模型、Skills、MCP、提示词四段配置；URL 可恢复 target 与 section。
4. WorkBuddy MCP 只在新启用并完成权威回读后提示“手动信任”；已经全开时再次全开不会重复提示。
5. Models、Skills、MCP、Prompts、Memory 五个管理页按高保真图重排操作位置，不重写真实业务 owner。
6. 资产合同已登记全部 11 张原型图及摘要；本机具体用户路径已替换为语义占位符。

## 当前候选与提交

- 分支：`codex/frontend-interaction-v3-20260825`
- 基线：`origin/main` 的 `91a66254…`
- 最新候选：`0ad9a7e1`（`fix(mcp): suppress redundant WorkBuddy trust prompt`）
- 分支相对 `origin/main`：ahead 17；尚未 push、未合并 main、未创建 Release、未部署生产。

## Fresh 验证结果

| 证据层 | 结果 | 说明 |
|---|---|---|
| 聚焦回归 | PASS | MCP/Skills 功能页 33/33；新增 WorkBuddy 零变更不重复提示回归 |
| 全仓门禁 | PASS | `mise run check` 退出码 0；含前端、Rust、任务/文档/平台/锁文件/版本/Release 合同 |
| browser_runtime | PASS | Playwright 132/132；900×600、1152×640、1232×700、1440×900 四视口各 33 条 |
| renderer build | PASS | 737 modules；主 chunk 904.82 kB，有非阻塞体积警告 |
| macOS native_UAT_read_only | PASS | 7 Agent 扫描、unknown 边界、WorkBuddy 模型投影、Prompt 不支持态、Memory 正文复制入口均在精确候选应用中读取验证；未改真实配置 |
| final package smoke | PASS | `0ad9a7e1` 重建后精确 `.app` 启动到 `/agents`，三一级导航与 7 Agent 目录可读，随后精确退出 |
| Windows native | PENDING | 必须在 Windows 内形成 fresh receipt，不能由 macOS/browser 替代 |
| pixel_diff | NOT RUN | 本轮已有运行态截图，但未执行严格 1:1 pixel diff |

## macOS 调试候选

- 版本：FyAgent `0.4.2`，arm64。
- DMG：`FyAgent_0.4.2_aarch64.dmg`，34,132,454 bytes。
- SHA-256：`b4e8c5688fb0cf5f64a112073235bd9b0822df9ab26542691a649c48fb6dcfe9`。
- 签名边界：ad-hoc / linker-signed，`TeamIdentifier` 未设置；这不是正式 Developer ID 签名，也不是 Release 候选。

## macOS 只读 UAT 观察

1. AI 软件目录：新三一级导航、7 Agent 目录、初始“尚未扫描”。
2. 扫描完成：Qoder 与 WorkBuddy 为 unknown；TRAE 未安装；Grok、Codex、Claude、OpenCode 已安装；界面明确“未确认不等于未安装”。
3. WorkBuddy 模型：展示现有模型能力投影，并明确开关 owner 在模型管理页。
4. WorkBuddy 提示词：由于没有 PromptAppId，展示真实不支持态，不出现假保存成功。
5. Memory：展示当前正文编辑器和复制入口；语义由单元/浏览器测试确认复制 draft 正文。

## 已知边界与下一步

- 原生 UAT 本轮采用只读路径，未真实切换 Skills/MCP、未保存 Prompt/Memory，避免污染用户现有配置；这些写入与权威回读已由单元及浏览器 fixture 覆盖。
- 仍存在既有 React `act(...)`、MSW 与 `NO_COLOR/FORCE_COLOR` 警告；它们未导致门禁失败。
- Vite 主 chunk 904.82 kB，作为后续性能治理项，不阻断本轮交互重构。
- 下一步只做 Windows 原生构建/启动/六路由/11 状态 fresh validation，并单独保存 receipt、截图/日志与失败路径证据。
- Windows 关闭后再形成最终候选结论；push、PR、main、Release、部署均需独立授权。

## 人类协作入口

当前不需要阻断式产品决策。若希望在 Windows 验证前做一次人工视觉复核，请重点看：左侧导航层级、Agent 扫描完成态、WorkBuddy 模型投影、提示词不支持态和 Memory 内容区。任何新意见都应以本节为上下文，明确是“视觉微调”“交互合同变化”还是“新增业务能力”，避免混入同一验收层。
