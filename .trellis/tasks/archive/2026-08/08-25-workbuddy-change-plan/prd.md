# WorkBuddy Change Plan adapter salvage

## Goal

On current `main` (after PR #148), recover the still-valid WorkBuddy save /
overwrite flow from Draft PR #139 and Issue #66. A WorkBuddy `models.json`
save must go through the existing plural `change-plans` port: zero-write
preview, one confirm with `{ planId, planDigest }`, typed executor job,
real file readback, and writer-owned backup restore. Do not merge Draft
#139 as-is.

User value: WorkBuddy 保存先看到无副作用计划（规范地址、模型 ID 摘要、
覆盖影响、备份），再单次确认；结果来自真实 Change Job 和文件回读。

## Confirmed facts

- `origin/main` = `dev/laiyongjie` = `8ebfd57d` (PR #148 merged).
- Draft PR #139 remains OPEN on obsolete stacked ancestry (`#137` base)
  and must not merge as-is.
- Current main already has canonical Change Plan ledger (#135), SecretRef
  (#145), typed executor (#146), Models Apply UI (#147), and Codex
  Provider upsert-and-switch (#148).
- WorkBuddy save still calls `save_workbuddy_models` directly, including
  the renderer overwrite-token dialog.
- Chip-remove still writes immediately via `removedModelIds`. That
  current-main contract stays.
- Models Apply still has no cancel button.
- Issue #66 remains OPEN. Close it only after current-main acceptance is
  fully satisfied.

## Requirements

- R1. Keep current-main `change-plans` parsers, Apply workspace, and
  Codex operations. Do not restore Draft #139's old UCP stack.
- R2. WorkBuddy 「保存并应用」 generates a zero-write plan first, then
  confirms once with `{ planId, planDigest }` only. API Key never enters
  the public plan, events, logs, or query cache.
- R3. Revision / file baseline drift after plan creation invalidates the
  plan; regenerate and preview again. No silent retry of a stale digest.
  No renderer-facing overwrite token or WorkBuddy-specific second
  confirmation. Overwrite impact is a plan risk; the adapter consumes any
  internal overwrite capability itself.
- R4. Apply uses the typed executor and shows real job steps / readback /
  `partialResult` / `eventSeq`. Writer keeps backup + atomic replace;
  failure restores the pre-apply backup when that restore succeeds.
- R5. Do not return secrets, full document bytes, full digest, or
  renderer-supplied paths. Usage evidence stays `not_observed`.
- R6. Apply does not fetch remote models, probe connectivity, or treat
  HTTP 200 as Ready.
- R7. Browser ports stay native-required. No production-looking fake job.
- R8. Do not add a Models cancel button. Chip-remove immediate write
  stays on `save_workbuddy_models`.
- R9. Update SPEC only for the landed WorkBuddy vertical. Do not regress
  #140 / #135 / #145 / #146 / #147 / #148.
- R10. Close old Draft #139 as superseded only after the replacement PR
  exists. Do not auto-close #66 from the PR title.

## Acceptance Criteria

- [ ] AC1. WorkBuddy save generates a closed `ChangePlan` with no
      `models.json` write until confirm.
- [ ] AC2. Confirm still sends only `{ planId, planDigest }` and keeps
      the per-plan click lock.
- [ ] AC3. Stale revision / consumed / expired plans offer regenerate,
      not a silent retry of the old digest.
- [ ] AC4. Running jobs refresh through bounded `getChangeJob` polling
      already owned by Apply workspace.
- [ ] AC5. Failed native writes restore the pre-apply backup when
      possible and surface `partialResult`; no generic success.
- [ ] AC6. Models page still has no cancel control. Chip-remove still
      writes immediately without a Change Plan.
- [ ] AC7. Browser fallback remains native-required / non-authoritative.
- [ ] AC8. Focused tests cover preview (no API Key), confirm payload,
      drift/regenerate, overwrite-as-plan-risk, and cancel-button
      absence.
- [ ] AC9. SPEC documents the WorkBuddy vertical on the current Change
      Plan stack, with operation-scoped resource sets.
- [ ] AC10. Replacement PR is based on current `main`; old #139 is
      closed as superseded after the new PR exists; #66 stays open
      unless its exact current-main acceptance is fully satisfied.

## Out of scope

- Merging or restacking Draft PR #139.
- Routing chip-remove through Change Plan.
- SecretRef as a WorkBuddy production consumer.
- WorkBuddy login, first task, usage evidence, or generic Undo.
- Additive AppTypes or a generic job engine.
- Windows matching-host HIL (state remaining evidence; do not close #66).

Worktree: `/Users/<username>/.devspace/worktrees/fyagent-workbuddy-change-plan`.
