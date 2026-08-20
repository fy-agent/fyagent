# v2 新增 OpenCode 全维度支持 — 技术设计

## 目标

让 OpenCode 成为 v2 前端五个维度（agent 目录、模型、skill、MCP、提示词）的完整受支持应用。

## 现状与改动点

### 后端（`src-tauri/src`）

1. `services/external_agents/mod.rs`：
   - `AgentCatalogId` 增加 `#[serde(rename = "opencode")] OpenCode`。
   - `AgentVariantId` 增加 `#[serde(rename = "opencode")] OpenCode`。
   - `AgentEvidenceId` 增加 opencode 相关证据 id（如 `OpencodeProduct`、`OpencodeRuntime`、`OpencodeModels` 等，按实际能力命名）。
2. `commands/agent_catalog.rs`：
   - `AGENT_CATALOG` 数组从 5 项扩展为 6 项，新增 opencode 条目（`id`、`variant_id`、`display_name`、`description`、`official_links`、`capabilities`）。
   - 新增 opencode 的 `official_links`（OpenCode 官方站点/CLI/文档）与 `capabilities`（复用既有能力矩阵模式，按 opencode 实际能力标注 `direct/assisted/unsupported/unverified`）。
   - 更新测试断言（`agent_catalog_freezes_v3_order_variants_links_and_capability_matrix` 等，从 5 项 → 6 项）。
   - 注意 `agent_catalog_wire_is_exact_v3` 测试对 `AGENT_CATALOG` 结构有精确断言，需同步。

### 前端（`src/v2`）

1. `shared/features/types.ts`：
   - `AGENT_CATALOG_IDS` 增加 `"opencode"`。
   - `AGENT_VARIANT_IDS` 增加 `"opencode"`。
   - 若需 opencode 专属 evidence/能力，同步相关常量。
2. `shared/assets/agents/index.ts`：
   - `agentIconIds` 增加 `"opencode"`，`agentBrandById` 增加 opencode 的 `{ iconUrl, list, detail }`。
   - 图标资源：复用 `src/icons/extracted/index.ts` 中已有的 `opencode` svg，或引入 `opencode-logo-light.svg`。需生成/引入对应的 `v2/shared/assets/agents/opencode.svg`。
3. `pages/agents/Page.tsx`：验证 agent 目录能渲染 opencode 条目（通常基于 catalog 数据自动渲染，若存在硬编码的 id 白名单需放开）。
4. `pages/models/quickSetup.ts` + `pages/models/Page.tsx`：
   - `MODEL_TARGETS` 增加 `"opencode"`。
   - `TARGET_PRESENTATION` / `TARGET_ICON_IDS` 增加 opencode 文案与图标。
   - 为 opencode 提供模型配置面板。opencode 的模型配置不同于 codex/claude（opencode 用 `opencode.json` 而非 config.toml + provider 快速设置）。需决定：
     - 方案 A（推荐）：opencode 复用 provider 快速设置能力（若后端 opencode 有 provider 写入链路）；或
     - 方案 B：opencode 提供引导式面板（类似 QoderGuidancePanel），指向 opencode 自身配置，FyAgent 仅提供 skill/mcp/提示词同步。
   - 需在实施前确认后端 opencode provider 模型写入能力（`opencode_config.rs` 已有 `set_provider` / `get_typed_providers`），据此选 A 或 B。
5. skill / MCP / 提示词页：
   - types 已含 opencode，重点验证页面渲染与选择链路。若页面有硬编码的 app 列表需放开 opencode。

## 待确认点

- opencode 模型配置采用「直接 provider 快速设置」还是「引导式面板」（依赖后端 opencode provider 写入能力与 v2 是否已有对应 port）。
- opencode 官方链接 URL（product/cli/desktop）。

## 兼容性 / 回滚

- 枚举新增是向后兼容的（serde 反序列化旧值不受影响）。
- `agent_catalog` 的 contract_version 是否需从 3 升到 4，取决于本次是否改变 wire 结构（新增条目不改结构，建议保持 3，仅更新 reviewedAt）。
