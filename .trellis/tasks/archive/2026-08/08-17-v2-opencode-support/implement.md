# 实施计划 — v2 新增 OpenCode 全维度支持

## 分批提交计划

### 批次 1：后端枚举与 agent catalog（可独立构建 + 单测）

1. `src-tauri/src/services/external_agents/mod.rs`：
   - `AgentCatalogId` 增加 `OpenCode`（rename `"opencode"`）。
   - `AgentVariantId` 增加 `OpenCode`（rename `"opencode"`）。
   - `AgentEvidenceId` 增加 opencode 相关证据 id。
2. `src-tauri/src/commands/agent_catalog.rs`：
   - 新增 `OPENCODE_OFFICIAL_LINKS`、`OPENCODE_CAPABILITIES`。
   - `AGENT_CATALOG` 扩展为 6 项。
   - 更新相关测试断言。
3. 验证：`cargo test` 通过（至少 agent_catalog 相关测试）。

### 批次 2：前端类型与图标资源（可独立构建）

1. `src/v2/shared/features/types.ts`：`AGENT_CATALOG_IDS` / `AGENT_VARIANT_IDS` 增加 opencode。
2. `src/v2/shared/assets/agents/`：新增 `opencode.svg`（复用既有 opencode 图标）。
3. `src/v2/shared/assets/agents/index.ts`：`agentIconIds` / `agentBrandById` 增加 opencode。
4. 验证：`pnpm build` / tsc 通过。

### 批次 3：模型配置页接入 opencode（可独立验证）

1. `src/v2/pages/models/quickSetup.ts`：`MODEL_TARGETS` 增加 `"opencode"`；扩展 `ProviderQuickSetupTarget` 或新增 opencode 分支。
2. `src/v2/pages/models/Page.tsx`：`TARGET_PRESENTATION` / `TARGET_ICON_IDS` 增加 opencode；`TargetPanel` 增加 opencode 分支（面板实现取决于设计确认的 A/B 方案）。
3. 验证：页面可进入 opencode 配置面板。

### 批次 4：skill / MCP / 提示词页验证与补齐（可独立验证）

1. 检查 `pages/skills/Page.tsx`、`pages/mcp/Page.tsx`、`pages/prompts/Page.tsx` 是否有 opencode 硬编码缺口，补齐。
2. 验证 opencode 在三页可被选中、配置、同步。
3. 运行相关前端测试。

## 验证命令

- 后端：`cargo test`（或 `cargo test agent_catalog`）
- 前端：`pnpm lint`、`pnpm tsc`、相关 jest/vitest 测试
- 端到端：启动 dev 后手动核对五页 opencode 条目与交互。

## 回滚点

- 每个批次独立提交；某批次失败可单独 revert，不影响其他批次。
