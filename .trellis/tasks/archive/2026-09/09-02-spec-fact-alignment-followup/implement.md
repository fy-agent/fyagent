# Implementation Plan

## Phase A — Reopen evidence, not the archived task

- [x] 读取本地 `trellis-update-spec`、`trellis-check` skill、SPEC 索引和上一轮
  归档审计。
- [x] 创建独立 follow-up 任务，不修改已归档主体任务的历史结论。
- [x] 以当前源码和可执行测试为权威，公开 Trellis 资料只用于校验文档组织原则。
- [x] 把文件级反证结果写入 `research/spec-fact-audit.md`，把跨域结论写入
  `research.md`。

## Phase B — Correct executable contracts

- [x] 校准 Skill 九目标、统一命令、归档安全、正常写入顺序与历史脏目录行的
  数据库-only 卸载恢复例外。
- [x] 校准 MCP `serde_json::Value`、适配器后置校验、无启用目标时的校验缺口，
  以及 upsert/enable/disable/delete 各自真实的非原子顺序。
- [x] 校准 Agent Directory Port 路径与 lifecycle revision 字段。
- [x] 校准 `AgentAuthPort`、严格 DTO、恢复/轮询、终态回调去重，以及
  `startSession` 尚未实现的响应 Agent 二次绑定限制。
- [x] 校准七目标 Assignment、`AssignmentPanel`、单实例串行与精确读回；明确
  simple adapter 只有编译期类型，运行时未知目标由 Rust 拒绝。
- [x] 校准 Skills 管理页的 SkillHub/ZIP/路径预览/可选备份、query invalidation
  与当前忽略 resolved `false` 的真实行为。
- [x] 校准 MCP 管理页三种 transport、开放高级 JSON、编辑态敏感值、默认七目标、
  post-save 投影失败与 query invalidation 行为。
- [x] 按真实的 Provider、WorkBuddy、OpenCode、TRAE 和 Change Plan Port/协议重写
  Models；删除不存在的聚合 `ModelPorts` 叙述。
- [x] 修正全库扫描发现的错误码映射、Cargo 权威路径，以及“文件 + 子命令/配置段”
  的歧义记法。

## Phase C — Full-library review

- [x] 复核 root/backend/frontend/guides 入口、所有相对链接与可达性。
- [x] 复核显式源码/测试路径、跨文件重复段落、旧拆分目标名和兼容路由体量。
- [x] 复核新增/改写跨层合同的七段式结构与唯一语义 owner。
- [x] 逐份复核六个 600+ 行文档；仅在拆分会割裂有序失败/证据域时保留。

## Phase D — Validate

- [x] 以最终文本重跑结构扫描与 `git diff --check`。
- [x] 以最终文本重跑 `mise run check:contracts`。
- [x] 以最终文本重跑完整 `mise run test:v2`。
- [x] 以明确 `TRELLIS_CONTEXT_ID` 重跑精确 prearchive gate。
- [x] 确认最终差异仅为 SPEC、任务记录和主体归档任务既有清单勾选。

## Phase E — Finish

- [x] 提交事实校准的 SPEC/task 变更（`f0479ac1`）。
- [x] 将 follow-up 任务转为 completed 并归档（`89f027ec`）。
- [x] 提交归档生命周期记录并复核最终工作树状态。

## Rollback points

- 某条结论不能由当前源码或测试证明时，删除结论，不用推测补齐。
- 某次修订会形成第二权威时，保留聚焦 owner，其他文档只写边界与链接。
- 发现产品加固机会时，只写“当前限制 / 协调变更要求”，不在本任务修改产品代码。
