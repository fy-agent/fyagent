# 最终集成审查（Grok 4.7 high）

模型：grok-4.7-high。只读，未改源码，未跑构建或测试套件。范围：`migrate/{restore,reconcile,receipt,capability,model}.rs`，`database/dao/session_restore.rs`，`native/mod.rs` 公共 runner，`native/opencode.rs` 暂存导入事务。Codex 碰撞扫描与 Hermes/Gemini 输出上限只作依赖核对。身份文件未复审。先前已处置项无反证，不重开。

## P0

无。

## P1

1. **读回不一致被记成已发布，对账不会再看它。**
   - 位置：`restore.rs` `apply_readback` 585–594；对照 `reconcile.rs` `resolve_known_id` 117–123；阶段定义 `model.rs` 342–361；同请求短路 `restore.rs` 463–464。
   - 触发：写入后或稍后 `verify_native_readback` 得到 `ReadbackVerdict::Mismatch`。当前阶段即使已是 `NativeReadbackVerified`（或更强）也会被改成 `NativeWritten`，并附 `NativeReadbackMismatch`。
   - 后果：`NativeWritten` 不是 `is_restored()`，打开会被拒绝；它也不在 `blocks_native_write` 里，但 463 行对同请求直接返回，所以这次不会再写一遍。对账只拉 pending / needsReconciliation / ambiguous，这行永远进不了 `Ambiguous`。规格要求读回不同保持未决并禁止盲目重试；重试被挡住了，阶段仍然表示发布成功。
   - 最小修法：与对账相同，Mismatch 写成 `Ambiguous`。已 `is_restored()` 的行不要降到 `NativeWritten`。

## P2

1. **空的 snapshotIds 会恢复整个包。**
   - 位置：`restore.rs` `select_sessions` 748–749。
   - 触发：`RestoreRequest.snapshotIds` 为空，包内会话都是同一目标供应商。
   - 后果：未点名的快照会在 `claim_batch` 之前被收进本次选择并写入。混供应商会在认领前失败。
   - 最小修法：空选择直接拒绝。调用方要整包时显式列出每个 snapshotId。

## 本轮核对后不构成缺陷

- 认领：`claim_batch`（`receipt.rs` 71–79）要求非空且与选择集合相等，之后才 `execute_claim`。生产路径没有单行 `claim()`。
- 店铺与版本：`require_verified_target` / `require_same_store` 在认领前。对账店铺或版本不符保持 `NeedsReconciliation`。
- 失败分类：OpenCode 在发布前的失败是 `ProvenNoSideEffect`；发布事务 `rollback`/`commit` 失败是 `Unresolved`。Hermes/Gemini 在已记录 id 之后的调用错误走 `Unresolved`。Runner 只有两端都是完整合法 UTF-8 才算 `Exited`；截断、非法 UTF-8、超时都不是成功。
- 阶段：`ProvenNoSideEffect` → `Failed`（可重试）；`Unresolved` → `NeedsReconciliation`；同请求的 `Restored` / `NativeWritten` 不重写。唯一的阶段回退是上面的 P1。
- 源摘要：回执只存 `content_digest`。`expectation_from_receipt`（687–697）不带正文；四个 writer 用摘要比较，不是正文缓存。回执表仍只有 `session_restore_attempts`。`set_restore_native_id` 更新行数不是 1 时返回错误（`session_restore.rs` 132–134）。
- OpenCode：`opencode.import-staged.create-only-v3` 先写入临时库，导出可见后再对真库做 `BEGIN IMMEDIATE` 的纯 INSERT。会话、消息或 part 冲突则回滚。目标已有同 id 的 project 行时不覆盖，这是 `publication_creates_missing_project_and_preserves_existing_project_metadata` 锁住的行为。
- Codex：`rollout_exists`（`codex.rs` 191–207）仍按文件名后缀扫描 active/archived，不跟随符号链接。与已处置结论一致。
- Windows 正式提权构建：`user_cli_execution_blocked` 在 spawn 前拒绝，版本探测也走同一 runner。不建议绕过。普通用户 helper 仍缺，真机 UAT 已由用户豁免。
- 可写实现仍是 Codex / OpenCode / Hermes / Gemini。Claude、OpenClaw、Grok Build 没有 writer。

## 冗余

没有第二套回执表、第二套请求指纹或可删的并行导入器。`claim()` 仅测试可见。

## 限度

未跑 cargo / mise，未读原始 CLI 流，未复审正在改的身份模块，未用真实模型回复。证据仍是合成 home 与 loopback 400。审查完成不等于校验通过。
