# 全面审查并刷新 Trellis Spec

## Goal

基于 FYAgent 当前源码、测试、配置、构建任务和已有 Spec，完成一次全量、可追溯的 Spec 审查与重构，使后续 AI 能够按真实职责快速发现、理解并注入所需合同，而不是加载跨越多个独立领域的巨型文档。

## Confirmed Facts

- 当前仓库是单仓库模式，Trellis Spec 分为 `backend`、`frontend` 与 `guides` 三层。
- 审查基线包含 43 份 Markdown，合计约 15,500 行；Markdown 相对链接和现有索引覆盖总体有效。
- 主要问题不是模板占位或大量失效路径，而是若干文档跨越多个独立源码所有者：
  - `backend/external-agent-p0.md` 同时覆盖 catalog/runtime、install inventory/jobs、Auth、Skills、MCP、Qoder Hooks、TRAE/OpenCode Models。
  - `frontend/v2-agent-models.md` 同时覆盖 Agent directory、Auth、Models 和 Change Plan UI。
  - `frontend/v2-skills-mcp.md` 同时覆盖 Skills、MCP 与共享 assignment UI。
- 当前源码已经存在更清晰的职责边界，例如 `agent_install/auth_*`、`agent_install/inventory.rs`、`commands/agent_*`、V2 `pages/agents|models|skills|mcp` 和 `shared/features/*`。
- 已确认事实漂移：`backend/modular-boundaries.md` 仍使用旧模块名 `auth`、`source`，且未反映当前多个 Agent transport command owner。
- 数据库迁移、Proxy runtime 与本地化分别拥有重要且重复出现的代码/测试所有者，但目前没有独立的高信号 Spec。

## Requirements

### R1. 全量审查可追溯

- 对审查前的每一份 Spec 记录“保留、修正、拆分、合并或仅更新索引”的处置结论与证据。
- 一次性扫描结果、行数和处置矩阵留在本任务 `research.md`；长期 Spec 只保留稳定合同。

### R2. 以事实和单一所有者为准

- 关键规则必须能够追溯到当前源码、测试、配置、构建任务或项目文档。
- 修正过时模块名、命令所有者、DTO、路径、验证顺序和测试入口。
- 不把偶然实现细节、研究时版本 URL、运行 ID 或一次性 commit 证据提升为长期权威。

### R3. 按独立检索域拆分巨型 Spec

- 将外部 Agent 后端合同按 catalog/runtime、lifecycle/inventory/jobs、Auth、vendor configuration、Skills、MCP 拆分。
- 将 V2 Agent/Models 前端合同按 Agent Directory、Auth、Models 拆分。
- 将 V2 Skills/MCP 前端合同按 shared assignment、Skills、MCP 拆分。
- 原有三条历史路径保留为短兼容路由，不再承担行为权威，也不再由层级索引直接注入。

### R4. 补齐高价值缺失 Spec

- 新增 SQLite persistence/schema/migration 合同。
- 新增 local proxy runtime/lifecycle/auth/failover/live-restore 合同。
- 新增 renderer localization/language/key-parity 合同。

### R5. 保守保留高风险单一事务

- 不因行数机械拆分 CI、Release、Windows 安全、Installer、Task Runner、Codex provider transaction 或 V2 Shell。
- 只有当源码和测试已经证明独立所有者时才拆分；高风险顺序、回滚和 fail-closed 语义不得因精简而丢失。

### R6. 更新导航与边界

- 更新 `backend/index.md`、`frontend/index.md` 及受影响的交叉引用。
- Foundation Spec 只描述可复用边界，并链接到 feature owner；不复制完整 feature matrix。
- `guides` 继续保持短 checklist，不承载 DTO、路径或行为权威。

### R7. 不改变产品行为

- 本任务只修改 `.trellis/spec/**` 与本任务/工作日志工件。
- 不修改产品源码、持久化 schema、公开命令、UI 行为、CI/Release 逻辑或依赖。

## Acceptance Criteria

- [x] `research.md` 覆盖审查前全部 43 份 Spec，并给出可核对的处置结论。
- [x] 三份跨域巨型 Spec 已替换为短兼容路由，真实合同由按所有者拆分的新文档承担。
- [x] 新增 persistence、proxy、localization 三份缺失合同；跨层/基础设施文档包含 Trellis 要求的七段式结构。
- [x] `backend/modular-boundaries.md` 与当前 Agent/Proxy 模块可见性和 transport owner 对齐。
- [x] 所有新旧索引、相对链接、引用路径、命令名、Spec 导航和兼容路由一致。
- [x] 不存在 `TBD`、模板占位、空标题或互相矛盾的双重权威。
- [x] `mise run check:contracts`、受影响的 focused contract tests 和 `git diff --check` 通过。
- [x] 归档前使用本任务精确路径完成 `check:contracts:prearchive`，提交后任务与 journal 状态一致。

## Out of Scope

- 产品功能重构、模块可见性调整或 API/DTO 变更。
- Windows/macOS 原生 HIL、签名、安装器或发布验证；本任务只核对已有合同和自动化证据。
- 为所有源码目录各建一份 Spec；仅为重复变更、高风险边界或独立检索域建立长期合同。

## Open Questions

无。用户已明确要求全面审查、按事实更新、必要时拆分/精简/补齐，并以提高 AI 检索和理解质量为验收方向。
