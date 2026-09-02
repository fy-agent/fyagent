# Design — Trellis Spec Information Architecture Refresh

## 1. Evidence hierarchy

按以下顺序判定长期合同：

1. 当前源码的所有权、类型和执行顺序。
2. 自动化测试与 fixture 的断言边界。
3. `mise`、Cargo、package、Tauri permission 等仓库 authority 配置。
4. 当前项目文档与仍有效的既有 Spec。
5. Git 历史仅用于解释兼容路径，不作为当前行为权威。

每一项重要规则应至少有一个真实文件或测试锚点。无法由仓库证明的外部 vendor 行为必须继续标为 `unverified`、`handoff_only` 或 residual HIL risk。

## 2. Target information architecture

### Backend

| Semantic owner | Target Spec | Primary evidence |
| --- | --- | --- |
| Catalog/runtime observation and closed launch | `external-agent-catalog-runtime.md` | `services/external_agents/`, `commands/agent_catalog.rs` |
| Readiness, inventory, lifecycle policy, source resolution and jobs | `external-agent-lifecycle.md` | `agent_install/`, `commands/agent_install_readiness.rs` |
| Auth observation/session coordination | `external-agent-auth.md` | `agent_install/auth_actions.rs`, `auth_sessions.rs`, `commands/agent_auth.rs` |
| Qoder Hooks, TRAE preflight/observation, OpenCode model persistence | `external-agent-configuration.md` | `commands/qoderwork.rs`, `traework.rs`, `opencode_models.rs` and services |
| Skill persistence, discovery, installation and target sync | `skill-management.md` | `services/skill*`, `commands/skill.rs`, `database/dao/skills.rs` |
| MCP persistence, live vendor files and validation | `mcp-management.md` | `mcp/`, `services/mcp.rs`, `commands/mcp.rs`, `database/dao/mcp.rs` |
| SQLite lifecycle, schema and migration | `persistence-and-migrations.md` | `database/mod.rs`, `schema.rs`, `migration.rs`, `backup.rs`, tests |
| Local proxy listener, routing, auth, failover and live restore | `proxy-runtime.md` | `proxy/`, `services/proxy.rs`, `commands/proxy.rs`, proxy tests |

`external-agent-p0.md` becomes a compatibility router. It links to the six owning contracts and explicitly states that it is not a behavioral authority.

### Frontend

| Semantic owner | Target Spec | Primary evidence |
| --- | --- | --- |
| Agent route, catalog projection, directory scan and lifecycle UI | `v2-agent-directory.md` | `pages/agents/`, agent catalog/readiness features |
| Auth observation/session UI | `v2-agent-auth.md` | `AgentAuthStatusPanel.tsx`, `useAgentAuthSession.ts`, `agent-auth.ts` |
| Models catalog, vendor panels, quick setup and change plan UI | `v2-models.md` | `pages/models/`, `features/models.ts`, `change-plans.ts` |
| Shared assignment state and UI | `v2-assignments.md` | `assignments.ts`, `authoritative-assignment.ts`, `AssignmentPanel.tsx` |
| Skills page and writes | `v2-skills.md` | `pages/skills/`, `features/skills.ts` |
| MCP page, secret drafts and validation | `v2-mcp.md` | `pages/mcp/`, `features/mcp*.ts`, `mcpSecurity.ts` |
| Language selection and locale schema | `localization.md` | `i18n/index.ts`, locale JSON, `localeKeyParity.test.ts` |

`v2-agent-models.md` and `v2-skills-mcp.md` become short compatibility routers. Historical tasks and classifier fixtures retain a valid path, while the live layer index injects only the focused owner.

## 3. Ownership and non-duplication rules

- Backend lifecycle owns legal product/surface/action policy and opaque target capabilities; frontend specs own only strict parsing, projection and user interaction.
- Installer primitives remain under `codex-desktop-installer.md`; external lifecycle owns which product may consume them and how post-action evidence is interpreted.
- Windows/macOS security specs own native identity, privilege and helper constraints. Feature specs link to them instead of re-stating the entire native contract.
- `persistence-and-migrations.md` owns schema versioning, migration transaction and DAO placement. Feature specs may define only their table fields and feature-specific migration outcome.
- `proxy-runtime.md` owns listener lifecycle, bearer admission, provider routing, failover and live configuration compensation. Provider-specific config specs own provider document mutation.
- `localization.md` owns locale keys and language selection. `user-facing-copy.md` owns wording quality and evidence strength.
- `v2-assignments.md` owns shared assignment state/interaction. Skills and MCP specs define resource-specific load/write behavior and link to the shared owner.

## 4. Compatibility and migration

- Do not delete the three old giant paths. Replace each with a concise router containing:
  - replacement map;
  - instruction not to add new behavior there;
  - historical compatibility rationale.
- Update layer indexes to route directly to focused files; compatibility routers are intentionally omitted from primary reading order.
- Update existing cross-links to the semantic owner when touched. Archived tasks may keep old links because routers remain valid.
- No `.trellis/config.yaml` registry change is required: `spec_source` is empty and the current local backend/frontend/guides tree is unmanaged local content.

## 5. Compression policy

- Prefer 200–500 line focused documents where the domain permits it, but do not impose a hard line limit.
- Keep high-risk, order-sensitive state machines intact when splitting would create two authorities for one transaction.
- Remove repeated scenario introductions and duplicate matrices while preserving exact wire enums, forbidden fields, rollback rules and assertion points.
- Use tables for closed enums/matrices and bullet lists for ownership; avoid copying long production code blocks.

## 6. Validation strategy

1. Structural audit: index coverage, relative links, placeholders, empty headings, compatibility router targets.
2. Fact audit: referenced paths exist; command/task names resolve; critical symbols appear in current source.
3. Ownership audit: each split domain has one indexed behavioral authority; foundation docs link rather than duplicate.
4. Focused tests: module-boundary, locale parity, change classification and task/spec contracts.
5. Repository gate: `mise run check:contracts` and exact prearchive exclusion.
6. Diff review: only `.trellis/spec/**`, task files and journal/archive lifecycle artifacts may change.

## 7. Rollback

- Before commit, restore any individual document from `HEAD` if a split loses a verified contract.
- Because old paths remain as routers and product code is unchanged, rollback is a documentation-only Git revert.
- If a new owner is not sufficiently source-backed, retain the old detailed content and defer that split rather than inventing a contract.
