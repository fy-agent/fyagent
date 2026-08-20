# v2 新增 OpenCode 全维度支持（agent目录/模型/skill/mcp/提示词）

## Goal

让 OpenCode 成为 v2 前端页面体系中的一个完整受支持应用，覆盖 agent 目录、模型配置、skill、MCP、提示词五个维度。

## Requirements

### 现状

- `src/v2/shared/features/types.ts` 中：
  - `McpTargetId` / `MCP_TARGET_IDS` 已包含 `"opencode"`（MCP 层面已有）。
  - `SkillTargetId` / `SKILL_TARGET_IDS` 已包含 `"opencode"`（skill 层面已有）。
  - `PromptAppId` / `PROMPT_APP_IDS` 已包含 `"opencode"`（提示词层面已有）。
  - `AgentCatalogId` / `AGENT_CATALOG_IDS` **不包含** opencode（agent 目录缺失）。
  - `AgentVariantId` / `AGENT_VARIANT_IDS` **不包含** opencode。
- `src/v2/pages/models/quickSetup.ts` 中 `MODEL_TARGETS` **不包含** opencode（模型配置页缺失）。
- `src/v2/shared/assets/agents/index.ts` 中 `agentIconIds` / `agentBrandById` **不包含** opencode（无图标资源）。

### 需要补齐

1. **Agent 目录页**（`src/v2/pages/agents/Page.tsx`）：
   - `AGENT_CATALOG_IDS` 增加 `"opencode"`，`AGENT_VARIANT_IDS` 增加对应 variant id。
   - agent 目录数据源（catalog contract）需提供 opencode 的条目（displayName、description、officialLinks、capabilities）。
   - `agentIconIds` / `agentBrandById` 增加 opencode 图标与品牌视觉配置（需准备 opencode 图标资源）。
2. **模型配置页**（`src/v2/pages/models/Page.tsx` + `quickSetup.ts`）：
   - `MODEL_TARGETS` 增加 `"opencode"`，在 `TARGET_PRESENTATION` / `TARGET_ICON_IDS` 增加对应文案与图标。
   - 为 opencode 提供模型配置面板（参考既有 Codex/Claude 的快速配置，或按 opencode 实际能力提供合理引导）。
3. **Skill / MCP / 提示词页**：检查 `opencode` 在这些页面中的展示与交互是否完整可用（这三个维度 types 已含 opencode，重点验证页面渲染、选择、同步链路是否已打通，如有缺口补齐）。

## Acceptance Criteria

- [ ] OpenCode 出现在 agent 目录页，展示名称、图标、官方链接、能力声明正确。
- [ ] OpenCode 出现在模型配置页，可进入其配置面板。
- [ ] OpenCode 在 skill 页可被选中并同步。
- [ ] OpenCode 在 MCP 页可被选中并配置。
- [ ] OpenCode 在提示词页可被选中并管理。
- [ ] 无类型错误、无 lint 错误，相关测试通过。

## Notes

- 优先复用已有 opencode 后端能力（`opencode_config.rs`、`mcp/opencode.rs`、`session_usage_opencode.rs` 等），本任务聚焦前端 v2 页面接入。
- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
