# Design

## Boundaries

- Renderer: `pages/models`, `pages/mcp`, shared `FeatureList` / `AssignmentPanel` / feature types / app icons. PR #111 还触及 `pages/prompts`, `pages/memory`, `pages/skills`。
- Native: `app_config::McpTargetId` / `McpApps`, `mcp/` adapters, `services/mcp.rs`, `database` schema v19, Agent catalog `mcp.write` 声明。
- 不新增通用 Provider 域，不把 Qoder/TRAE 变成 `AppType`。

## MCP live files (vendor-evidenced)

| Target | Canonical file | Format | Skip |
| --- | --- | --- | --- |
| QoderWork CN | `{home}/.qoderworkcn/mcp.json` | `{ mcpServers: { id: spec } }` | 家目录与文件都不存在 |
| TRAE Work CN | `{User}/mcp.json`（macOS `Library/Application Support/TRAE SOLO CN/User`，Windows Roaming 同名） | 同上 | User 目录与文件都不存在 |

证据：

- QoderWork CN `app.asar` `constants.m` = `join(homedir(), ".qoderworkcn", ...)`；`CUSTOM_MCP_CONFIG_PATH = constants.m("mcp.json")`。`userData/mcp.json` 是内置表，禁止写。
- TRAE 社区与 VS Code 风格工作台：全局 MCP 为 `User/mcp.json`；本仓库模型写入已定位同一 User 树下的 `globalStorage/state.vscdb`。

写入契约对齐 WorkBuddy：backup、atomic write、去掉 UI 辅助字段、import 时 `validate_server_spec`。Qoder 条目可带 `enabled`；同步时保留厂商未知字段，但写入我方权威 spec 时去掉 FyAgent UI 字段。

## Direct assignment

- `McpTargetId` 增加 `QoderWork` / `TraeWork`（serde `qoderwork` / `trae-work`）。
- `mcp_servers` schema 18 → 19：`enabled_qoderwork`、`enabled_trae_work` default false。
- V2 `MCP_TARGET_IDS` / `MCP_TARGETS` 扩到六项。`AssignmentPanel` 已走 `getSkillTargetIcon`，图标复用现有 PNG。
- Catalog：Qoder/TRAE `mcp.write` 从 `assisted` + `vendor_ui_required` 改为 `direct` + `dedicated_native_contract`。`validate_external_mcp_config` 仍保留给 Agents 准备流。

## List overlap

`.fy-feature-list` 当前是 CSS Grid，`SelectionLens` 是同容器里 `position: absolute` 的最后一个子节点，仍参与 grid 自动放置，把列表行挤到同一轨道。改为纵向 flex，lens 不占 flex 槽。Skills 共用同一 chrome，一并修好。

## PR #111 merge

- `git fetch` + merge `cursor/prompt-memory-frontend-align-06e7` into `dev/laiyongjie`。
- 冲突优先保留本分支 `FeatureTabs` / `FeatureSearch` / `FeatureList` / `SelectionLens` / `SplitPanes`。
- PR 私有 tabs/search/list 用共享件替换。PR 里发现页「将安装到 {应用}」等可复用 chrome 抽到 `shared/ui`。
- 完成后 `gh pr close 111`，归档 `08-18-prompt-memory-frontend-replan`。
