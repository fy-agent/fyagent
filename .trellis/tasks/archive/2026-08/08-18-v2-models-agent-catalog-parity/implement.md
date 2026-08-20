# Implementation plan

Active task: `.trellis/tasks/08-18-v2-models-agent-catalog-parity`

## Ordered checklist

### 0. Shared contract (must land before parallel UI, or be owned by one agent with no overlap)

1. Rust catalog: TRAE displayName/URL/descriptions；OpenCode+TRAE `models.write` → direct；Qoder 描述去掉“去完成模型设置”。
2. V2 types: Skill/MCP 目标集合；新 Trae/OpenCode 模型 DTO；可选 ProviderSummary.modelId。
3. Ports + tauri/browser adapters + command registration + ACL compatibility set。
4. 从本机 QoderWork CN.app 导出 PNG 替换 agent 图标。
5. 拷贝 leftover vendor SVG 到 `src/v2/shared/assets/models/` 并实现 `resolveModelVendorIcon`。

### 1. Agent directory UX — files

- `src/v2/pages/agents/Page.tsx` + CSS
- catalog tests: `src-tauri/src/commands/agent_catalog.rs` tests, `tests/v2-browser/agents-models.spec.ts`, `tests/v2-browser/support/features.ts`

Do: 短标签、支持项跳转、折叠不适用、TRAE Work CN 文案。Don’t: 改 CatalogMasterDetail 几何。

### 2. Models page — files

- `src/v2/pages/models/Page.tsx`, `quickSetup.ts`, `Page.css`
- `src-tauri/src/services/traework.rs` (+ new persist module if needed)
- OpenCode snapshot/fetch/save commands next to `opencode_config.rs`
- `tests/v2/pages/models/**`

Do: 三个 WorkBuddy 级面板 + Qoder 不支持说明 + 所有模型名带图标。Don’t: 改 WorkBuddy 已有安全合同。

### 3. Skills / MCP — files

- `src/v2/shared/features/types.ts` 目标表
- `src/v2/shared/assets/apps/index.ts`
- `src/v2/pages/skills/Page.tsx`, `src/v2/pages/mcp/**`
- `src-tauri/src/app_config.rs` SkillTargetId/McpApps
- `src-tauri/src/database/schema.rs` v18
- `src-tauri/src/services/skill.rs` WorkBuddy copy dest
- `src-tauri/src/mcp/` WorkBuddy adapter
- assignment tests

Do: V2 目标对齐目录；WorkBuddy 复制/写入。Don’t: AppType::WorkBuddy；Qoder/TRAE MCP 直接分配。

## Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

Focused：catalog URL/name matrix；TRAE save fixture sqlite（含 secret 不出现在 DTO）；OpenCode snapshot/save；Skill schema 18 与 WorkBuddy 复制；MCP `.mcp.json` 适配；模型图标无远程 URL；Qoder PNG 可解码。

## Risky files / rollback

- `state.vscdb` 写入：只动自定义行，先 backup。
- Skill DAO/schema：迁移必须保留旧八列语义。
- `agent_catalog.rs` 与大量精确断言测试。
- ACL：新 command 必须加入 capability 并保持并集完整。

## Parallel dispatch

三个 `trellis-implement` 按上面 1/2/3 分文件。若 0 尚未合并，负责 0 的实现者先改 types/catalog，其余等待或只改自己的页面并依赖已合并的类型。

禁止子代理再派 trellis-implement/check。
