# Canonical V2 Change Plan UI salvage

## Goal

On current `main` (after PR #146), recover the still-missing V2 Models Change
Plan product surface from Draft PR #136. A user switching an already-saved
Codex Provider must see a side-effect-free four-section preview, confirm once,
then watch backend-owned five-phase progress, readback, partial truth, and
recovery copy. The UI must not invent a second coordinator or restore the old
singular `change-plan` backend.

User value: before confirm, the user can judge what will change, the risk,
preconditions, and how recovery works; after confirm, the page shows the real
job instead of a one-shot snapshot.

## Confirmed facts

- `origin/main` = `dev/lai-yongjie` = `a2902df6` (PR #146 merged).
- Draft PR #136 remains OPEN on the obsolete stacked ancestry and must not
  merge as-is.
- Current main already has the plural `change-plans` port, closed parsers,
  five-phase job DTO, `partialResult`, `eventSeq`, `cancelChangeJob`,
  recoverable-job notice, and Apply workspace.
- Current Apply preview only shows target Provider, restart expectation, and
  plan status. It does not present Issue #56's four blocks.
- `ChangePlanWorkspace` applies once, then calls `getChangeJob` once. It does
  not poll while `status=running`.
- Apply workspace renders steps and resources, but does not surface
  `partialResult` counts, remaining effects, manual actions, or `eventSeq`.
- Frontend SPEC currently forbids a Models cancel button and a client-owned
  cancellation state machine. `cancelChangeJob` stays a bounded port for later
  product use.
- `ChangePlan` already carries enough facts for a four-section preview:
  current/target Provider codes, `risks`, `restartExpectation`, adapter
  read/write sets, `secretCapability`, baseline ids/digest, `evidenceNote`,
  and writer-owned compensation.
- Issues #41 and #56 remain OPEN. Close them only if this replacement fully
  meets current Issue acceptance on current main.

## Requirements

- R1. Keep the current-main `change-plans` port and fail-closed parsers.
  Do not restore Draft #136's singular `change-plan` module, old backend, or
  fake coordinator.
- R2. Preview remains zero-write. Confirm sends only `planId + planDigest` and
  retains the per-plan repeat-click lock.
- R3. Replace the compact plan `dl` with a four-section preview derived only
  from the existing closed `ChangePlan` DTO:
  语义变化 / 风险与重启 / 前置条件与范围 / 恢复方式.
  Do not invent a second preview schema or unsanitized diff/path/secret.
- R4. While a job is non-terminal, poll authoritative `getChangeJob` with a
  bounded interval and cancel in-flight work on unmount, target change, or a
  newer request revision. Same-plan/same-digest replay remains
  `idempotent_replay`.
- R5. Render durable partial truth when present: succeeded / compensated /
  unverified steps, remaining effects, and closed manual-action codes. Render
  `eventSeq` as backend sequence, not a local counter.
- R6. Keep existing terminal copy contracts: no generic success from mixed or
  unavailable readback; `cancelled_before_write` / `interrupted_before_write` /
  `recovered_target_reached` stay confirmed specialized copy; usage evidence
  remains `not_observed`.
- R7. Browser ports stay native-required. No production-looking fake job.
- R8. Do not add a Models cancel button or client-owned cancel state machine
  in this slice.
- R9. Update frontend SPEC to match the landed UI. Do not regress
  #140 / #135 / #146 contracts.
- R10. Close old Draft #136 as superseded only after the replacement PR exists.
  Do not auto-close #41 / #56 from the PR title.

## Acceptance Criteria

- [ ] AC1. Ready-plan preview shows four labelled sections sourced only from
      the closed `ChangePlan` DTO; generating the preview performs no Provider
      write.
- [ ] AC2. Confirm still sends only `{ planId, planDigest }` and ignores a
      second click for the same plan key.
- [ ] AC3. A `running` job is refreshed through repeated `getChangeJob` calls
      until a terminal snapshot arrives or the workspace is closed/retargeted.
- [ ] AC4. When `partialResult` is present, the UI shows succeeded /
      compensated / unverified counts, remaining effects, and any
      `manualActions`.
- [ ] AC5. Visible event identity comes from `eventSeq` / job events, not from
      `Date.now()` or a React-owned sequencer.
- [ ] AC6. Models page still has no cancel control; existing cancel-copy for a
      backend `cancelled` job remains.
- [ ] AC7. Browser fallback remains native-required / non-authoritative.
- [ ] AC8. Focused V2 Apply/Change Plan tests cover the four-section preview,
      polling refresh, partial-result rendering, and cancel-button absence.
- [ ] AC9. Frontend SPEC documents the four-section preview, polling, partial
      truth, and the still-intentional absence of a Models cancel button.
- [ ] AC10. Replacement PR is based on current `main`, old #136 is closed as
      superseded after the new PR exists, and #41/#56 stay open unless their
      exact current-main acceptance is fully satisfied.

## Out of scope

- Merging or restacking Draft PRs #136 / #137 / #139.
- Codex Provider create/edit/switch vertical (Issue #63 / old #137).
- WorkBuddy second adapter (Issue #66 / old #139).
- Adding a cancel button or changing schema v20 / SecretRef / Quick Setup
  targeted-patch contracts.
- Expanding the wire DTO with new preview-only fields in this slice.
- Closing #41 / #56 / #58 / #59 / #60 by PR title alone.
- Native macOS isolated-home UAT as a merge blocker; keep it as optional
  follow-up evidence.

## Notes

Branch: `dev/v2-change-plan-ui`.
Worktree: `/Users/pythonrust/.devspace/worktrees/fyagent-v2-cp-ui`.
Sources: Draft PR #136 for product UX only; current `main` for contracts.
