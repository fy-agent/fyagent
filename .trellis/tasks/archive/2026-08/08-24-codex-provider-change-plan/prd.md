# Codex Provider Change Plan vertical salvage

## Goal

On current `main` (after PR #147), recover the still-valid Codex Provider
create / edit / switch product flow from Draft PR #137 and Issue #63. A
saved Codex Provider change must go through the existing plural
`change-plans` port: zero-write preview, one confirm with
`planId + planDigest`, typed executor job, readback, and honest usage
evidence. Do not merge Draft #137 as-is.

User value: Provider 新建、编辑、切换先看到无副作用计划，再单次确认；
执行结果来自真实 Change Job，而不是第二套保存路径。

## Confirmed facts

- `origin/main` = `dev/laiyongjie` = `53900da4` (PR #147 merged).
- Draft PR #137 remains OPEN on obsolete stacked ancestry and must not
  merge as-is.
- Current main already has canonical Change Plan ledger (#135), SecretRef
  (#145), typed executor (#146), and Models Apply four-section preview plus
  bounded `getChangeJob` polling (#147).
- Models Apply still has no cancel button. `cancelChangeJob` stays a
  bounded later-product port.
- Issue #63 remains OPEN. Close it only after current-main acceptance is
  fully satisfied.

## Requirements

- R1. Keep current-main `change-plans` parsers, DTO, and Apply workspace.
  Do not restore Draft #137's old stack, singular `change-plan`, or a
  second executor.
- R2. Codex Provider create, edit, and switch each generate a zero-write
  plan first, then confirm once with `{ planId, planDigest }` only.
- R3. Drift after plan creation (Provider or Codex baseline change)
  invalidates the plan; regenerate and preview again. Do not execute a
  stale plan.
- R4. Apply uses the typed executor and shows real job steps / readback /
  `partialResult` / `eventSeq`. No fake progress, scenario coordinator, or
  client-owned success.
- R5. Do not return secrets, full config, full digest, or absolute paths
  in UI copy. `secretCapability=secret_dependency_unavailable` remains
  fail-closed.
- R6. Core apply does not probe connectivity, send model requests, or
  treat HTTP 200 / process exit 0 / restart request as Ready. Usage
  evidence stays `not_observed` unless a closed DTO field says otherwise.
- R7. Browser ports stay native-required. No production-looking fake job.
- R8. Do not add a Models cancel button in this slice.
- R9. Update frontend/backend SPEC only for the landed Provider vertical.
  Do not regress #140 / #135 / #145 / #146 / #147.
- R10. Close old Draft #137 as superseded only after the replacement PR
  exists. Do not auto-close #63 from the PR title.

## Acceptance Criteria

- [x] AC1. Codex Provider create/edit/switch generate a closed `ChangePlan`
      with no Provider write until confirm.
- [x] AC2. Confirm still sends only `{ planId, planDigest }` and keeps the
      per-plan click lock.
- [x] AC3. Stale baseline / consumed / expired plans offer regenerate, not
      a silent retry of the old digest.
- [x] AC4. Running jobs refresh through bounded `getChangeJob` polling
      already owned by Apply workspace.
- [x] AC5. Partial/failed native writes surface `partialResult` and closed
      recovery copy; no generic success.
- [x] AC6. Models page still has no cancel control.
- [x] AC7. Browser fallback remains native-required / non-authoritative.
- [x] AC8. Focused tests cover create/edit/switch preview, confirm payload,
      drift/regenerate, and cancel-button absence.
- [x] AC9. SPEC documents the Provider vertical on the current Change Plan
      stack.
- [ ] AC10. Replacement PR is based on current `main`; old #137 is closed
      as superseded after the new PR exists; #63 stays open unless its
      exact current-main acceptance is fully satisfied.

## Out of scope

- Merging or restacking Draft PRs #137 / #139.
- WorkBuddy second adapter (Issue #66 / old #139).
- Additive providers, other AppTypes, or a generic job engine.
- SecretBackend rewrite, multi-resource undo (#61), or schema v20 / v20
  proof restoration.
- Windows release trust chain (#68) and installed-package UAT (#141).
- Closing #58 / #59 / #60 / #63 by PR title alone.

## Notes

Branch: `dev/codex-provider-change-plan`.
Worktree: `/Users/<username>/.devspace/worktrees/fyagent-codex-provider`.
Sources: Draft PR #137 and Issue #63 for product intent; current `main`
for contracts.
