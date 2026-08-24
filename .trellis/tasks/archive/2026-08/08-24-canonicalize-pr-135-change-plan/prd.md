# Canonicalize PR 135 Change Plan

## Goal

将 PR #135 从旧 `main@e94307cd` 上的 consolidation 候选，收敛为当前
`main@67f50b8f` 上唯一可信的 Change Plan 主线实现，并在不回退 #140 已建立
的 Models / Codex / WorkBuddy 配置安全合同的前提下，完成工程审查、规范化
SPEC、Trellis 收尾和本地完整门禁，形成可推送并进入最终 Required CI 的
merge-ready PR head。任务归档后，最终仅经 GitHub PR 合入 `main`。

## Confirmed Facts

- 当前远端 `main` 为 `67f50b8ffdf4105b1e478f87fe60eca0af7dc9c2`；#135 head 为
  `d826bbf50a51efeb18629c474d44c74f31c0512d`。
- #135 相对当前 main 为 `main-only 7 / PR-only 1`，实际 Git merge 冲突集中在
  `.trellis/spec/frontend/v2-agent-models.md` 与
  `scripts/tasks/supported-platform-structure-assets.json`；Codex/Provider/Models 等产品
  文件虽可自动合并，仍必须做语义复审。
- #135 已定义 Schema v20 的 `change_plans`、`change_jobs`、
  `change_job_events`，并声明三张表为 local-only sync state。
- #135 以 `ProviderService` 为唯一配置 writer；Change Plan 负责 orchestration，
  recovery 只允许 readback，不允许 writer replay。
- 已关闭未合并的 #130 明确把有效成果迁移到 #135；#134 仍继承旧底座且定义另一套
  不兼容 Schema v20。本任务不得把旧 schema/DAO/第二 owner 重新引入。
- #140 已把 Quick Setup 改为 targeted live patch，并强化 Codex/WorkBuddy/OpenCode
  backup、write-target disclosure、Models revision dirty state 与 probe contract。

## Requirements

1. #135 必须更新到执行时最新 `origin/main`；旧 SHA 的 CI 不作为最终 merge 证据。
2. Schema v20 只保留一套 canonical Change Plan 表结构；fresh DB、v19→v20、reopen、
   future-version rejection、memory DB 都必须有可执行验证。
3. Change Plan 三张 ledger 表必须在 WebDAV export 中跳过、import 时保留本机数据；
   设备 A 的执行状态不得传播或覆盖设备 B。
4. Apply 只能调用现有 Provider lock-held writer 最多一次；invalid/expired/consumed/
   stale/secret-blocked 请求 writer=0；reconcile/get/list 永不重放 writer。
5. 必须保留 #140 的 targeted Codex patch、未知配置保留、backup/write-target disclosure、
   WorkBuddy/OpenCode backup 语义；不得恢复 whole-file snapshot overwrite。
6. Preview 除 credential-free ledger 外零业务写、零网络；Apply 只接收
   `planId + planDigest`，重新校验 15 分钟 TTL、DB/device/live/target baseline，drift
   时 fail closed。
7. writer success 不能直接成为 UI success；DB/device/target/live authoritative readback
   决定终态，mixed/unavailable authority 进入 `recovery_required`。
8. 本任务不实现 #132 SecretBackend；无法证明无需新凭据时必须
   `secret_dependency_unavailable` 且 writer=0。secret/raw config/path 不得进入 DTO、
   ledger、error 或 log。
9. Tauri IPC 保持封闭窄合同；renderer 不得提交任意 path、command、raw config 或第二份
   operation payload。Browser adapter 保持 native-only。
10. V2 Apply UI 只呈现真实 plan/job snapshot，一次确认、无 fake progress、失败不绿；
    无被动真实使用证据时保持 `usageEvidence=not_observed`。
11. 保留 #135 已验证的 Grok 文案和 read-only Agent install readiness；不在本任务扩展
    #132、#134 新能力、#137 vertical 或 #139 adapter。
12. **SPEC 硬门禁**：冲突解决后必须按最终代码和测试重新规范化 backend/frontend
    SPEC；不得机械选择 ours/theirs，不得让旧 #135 文档覆盖 #140 当前安全合同；最终
    contract/spec drift 检查必须通过。
13. **Trellis 硬门禁**：#135 分支携带的 completed/review 任务在 merge 前必须正确收口；
    completed 子任务归档，review parent 与最终事实一致后归档；本 canonicalization task
    也必须在 merge 前完成验证、记录并归档。
14. `main` 禁止直接 push；最终只能在最新 head SHA required CI 全绿后经 GitHub PR merge。

## Acceptance Criteria

- [x] #135 已基于执行时最新 main 完成语义整合，`git diff --check` 通过。
- [x] Schema 0→20、19→20、reopen、future rejection、memory DB、sync skip/local preserve
      全部通过，仓库不存在第二套有效 v20 Change Plan 定义。
- [x] focused tests 证明 preview 零副作用、TTL/digest/baseline stale、并发单次 admission、
      writer exactly-once、所有 rejection writer=0、recovery no-replay。
- [x] #140 Codex targeted patch、backup/write-target、WorkBuddy/OpenCode 回归全部继续通过。
- [x] readback/recovery 故障注入覆盖 writer failure、target mismatch、baseline restored、
      interrupted/recovery-required，UI 不产生假绿色状态。
- [x] Tauri DTO/ACL、V2 typecheck/lint/unit/browser、Rust fmt/check/clippy/test、Repository
      Contracts、supported-platform 与 `mise run check` 全绿。
- [x] 两轮最终 review 无未解决 P0/P1/blocker；非阻断项明确记录为 follow-up。
- [x] backend/frontend SPEC 与最终实现/测试一致，SPEC/contract 漂移检查通过。
- [x] #135 原 consolidation Trellis task tree 已归档，本任务 `check:prearchive` 通过；随后立即
      归档本任务，确保 merge 前无本次工作遗留的 completed/review/in-progress active task。

## Post-Archive Merge Gate

- 最终 archive commit 推送到 PR #135 后，以该精确 head SHA 的 GitHub Required CI 为准；
  旧 SHA 的绿灯不替代最终 head。
- Required CI 全绿且 `origin/main` 未发生未整合漂移后，只通过 GitHub PR merge；禁止直接
  push `main`。
- merge 后继续核对 `origin/main` merge SHA，并等待该 main SHA 的 `CI / Required` success。

## Out of Scope

- #132 SecretRef/Keychain/Credential Manager 实现与 HIL。
- #134 typed adapter/cancel/compensation 新能力迁移。
- #136 后续 UI 能力、#137 Codex Provider create/edit vertical、#139 WorkBuddy adapter。
- Windows Authenticode / release UAT。
- 无明确边界收益的大规模模块重构。
