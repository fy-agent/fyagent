# 模型页入口、MCP Qoder/TRAE 适配、合并提示词 PR

## Goal

在 `dev/laiyongjie` 上完成三件用户可见工作，并以该分支已有的共享前端模块为主线：去掉模型页无用的官方设置入口、让 MCP 已安装列表把 QoderWork CN / TRAE Work CN 当作可直接分配目标、把 PR #111 的提示词/记忆/Skills 发现页改动合入本地后关闭该 PR。

## Requirements

- 模型页：移除 QoderWork CN「打开官方设置」和 TRAE Work CN「打开 TRAE 官方模型设置」两个按钮。QoderWork 仍展示「不支持第三方模型配置」，并保留「管理 Hooks 和 MCP」。TRAE 仍走现有原生模型保存。
- MCP 页：QoderWork CN 与 TRAE Work CN 进入直接分配（开关、全开/全关、安装对话框、导入），写入厂商真实 live 文件，而不是只做校验或跳转官方 UI。
- Skills 与 MCP 的应用分配、发现页安装目标、全开/全关列表必须与 Agent 目录同一顺序：QoderWork CN、TRAE Work CN、WorkBuddy、Codex、Claude Code、OpenCode。不要按字母序或旧的 Claude 优先序。
- MCP 已安装列表：修好黑色字块（列表行）互相重叠。修复落在共享 `FeatureList` / `SelectionLens`，不要再做一页一份的列表。
- 合并 https://github.com/fy-agent/fyagent/pull/111 到本地 `dev/laiyongjie`。冲突以本分支模块化/共享化为准：PR 私有组件能被现有共享件替换则替换；PR 里值得给其他页用的抽到 `src/v2/shared`。合并无问题后关闭 PR。
- 若 PR #111 对应 Trellis 任务未完成，按 Trellis 要求归档。
- 完成后按 `trellis-update-spec` 更新相关 code-spec。

## Out of scope

- 不把 QoderWork / TRAE Work 做成 `AppType`。
- 不改 QoderWork Skills 目录（现有 `~/.qoderwork/skills` 与 CN 实际 `~/.qoderworkcn/skills` 的差异另开任务）。
- 不写 QoderWork Hooks 的 `settings.json`，不写 TRAE `state.vscdb`，不写 QoderWork `userData/mcp.json` 内置表。
- 不扩大 MCP 发现页目录，不碰 xxk 的中国精选扩容任务。
- 不证明厂商进程一定加载了新 MCP 行（HIL 仍为 unverified）。

## Acceptance Criteria

- [x] 模型页 QoderWork / TRAE 详情不再出现「打开官方设置」「打开 TRAE 官方模型设置」。
- [x] MCP 已安装分配目标为六个，且顺序与 Agent 目录一致：qoderwork、trae-work、workbuddy、codex、claude、opencode。
- [x] Skills 应用分配与发现页安装目标使用同一顺序。
- [x] 启用 QoderWork 时写入 `{trusted-home}/.qoderworkcn/mcp.json` 的 `mcpServers`；启用 TRAE 时写入 TRAE SOLO CN `User/mcp.json` 的 `mcpServers`。家目录与文件都不存在时跳过写入（WorkBuddy 同款）。
- [x] 已安装列表行不再因 SelectionLens 占用 grid 轨道而互相重叠。
- [x] PR #111 的提示词/记忆/Skills 发现页行为合入本地；冲突处使用本分支共享 chrome。
- [x] `08-18-prompt-memory-frontend-replan` 已归档或已记录为何不能归档。
- [x] GitHub PR #111 在本地合并验证后关闭。
- [x] 相关 spec 已按 7 段 code-spec 深度更新。
