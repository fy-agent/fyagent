# Codex Provider Change Plan Switch — Implementation Plan

## Phase 0 — Design freeze and baseline

- Validate this task and parent linkage.
- Record `origin/main@4b4e1754`, current branch, clean worktree, protected paths,
  and the closed/unmerged PR #104 boundary.
- Read backend/frontend/guides specs through `trellis-before-dev`.
- Commit the child task planning package before product code.

Validation:

```text
python3 .trellis/scripts/task.py validate 08-14-unified-change-plan-codex-switch
git diff --check
git status --short --branch
```

## Phase 1 — Backend contract and additive persistence

- Add closed DTO/error/state unions and versioned digest inputs.
- Add idempotent plan/job/event tables under schema v16.
- Implement DAO methods for immutable plans, one-time admission, job snapshot,
  event sequence, and recoverable-job queries.
- Add serialization redaction and schema compatibility tests.
- Commit independently.

Focused validation:

```text
mise run rust:test -- change_plan_contract
mise run rust:test -- change_plan_store
```

## Phase 2 — Side-effect-free Codex switch plan

- Implement current/target/live inspection and semantic digests.
- Add `create_codex_provider_switch_plan` command.
- Prove identical intent/baseline digest stability, unique plan IDs, expiry, and
  zero DB/settings/live/network mutation.
- Commit independently.

Focused validation:

```text
mise run rust:test -- codex_provider_switch_plan
mise run rust:test -- change_plan_no_side_effects
```

## Phase 3 — Admission, apply, readback, and reconcile

- Implement atomic admission and replay/stale/expiry rejection.
- Call the existing Provider switch writer exactly once.
- Persist bounded job events and monotonic snapshots.
- Implement independent DB/settings/target/live readback classifier.
- Reconcile nonterminal jobs by readback without replay.
- Add failure-injection coverage for stale, writer error, post-write mismatch,
  baseline restored, target reached, and mixed state.
- Commit independently.

Focused validation:

```text
mise run rust:test -- codex_provider_change_job
mise run rust:test -- change_plan_reconciliation
```

## Phase 4 — Thin API/query layer and shared dialog

- Add exhaustive TypeScript decoders and API/query hooks.
- Build preview, stale, running, result, resource, restart, recovery, and evidence
  states in `ChangePlanFlow`.
- Cover keyboard/focus behavior, safe unknown-code fallback, polling, missed and
  duplicate events, and sentinel secret/path absence.
- Do not route the production switch entry yet.
- Commit independently.

Focused validation:

```text
mise run test:unit -- change-plan
mise run typecheck
```

## Phase 5 — Codex Provider switch entry migration

- Route only Codex switch through Change Plan.
- Preserve non-Codex direct switch behavior.
- Remove Codex direct-success toast and derive results from job snapshot.
- Invalidate only controlled Provider/current/live queries after terminal job.
- Add hook/component regression tests proving no direct Codex mutation bypass.
- Commit independently.

Focused validation:

```text
mise run test:unit -- useProviderActions
mise run test:unit -- ProviderList
mise run typecheck
```

## Phase 6 — Integration, review, and evidence freeze

After all backend and frontend focused tests pass:

- Run `trellis-check` and fix all findings.
- Run the repository-managed full quality gate (`mise run check`).
- Run task validation and `git diff --check`.
- Audit cumulative diff and protected paths.
- Verify no secret/path sentinel appears in tracked output.
- Update task notes with exact fresh results and evidence boundaries.
- Commit final evidence/docs only after source behavior is frozen.
- Push `codex/unified-change-plan-codex-switch`; do not merge main or create a PR
  without a separate request.

## Review gates

- Backend DTO and TypeScript union remain exact and exhaustive.
- Plan is read-only; stale/replay failures have zero writer calls.
- Provider switch writer has one owning implementation.
- Job result is derived from readback, not mutation return or toast.
- `liveConfigChanged` only drives restart guidance.
- No raw error, settings, secret, full path, or raw live config crosses IPC.
- No `src/v2`, WorkBuddy, Profile, Workspace Pack, or Prompt/Memory scope bleed.

## Rollback points

- Phase 1: revert additive tables/DAO before any entry uses them.
- Phase 2: revert plan command without changing Provider behavior.
- Phase 3: revert adapter/job commands while direct switch is still production.
- Phase 4: remove unused UI/query layer.
- Phase 5: revert the single entry-routing commit; preserve job records for any
  already-admitted operation and do not delete uncertain evidence.

## Pre-start checklist

- [x] Goal, scope, non-goals, UX, compatibility, and evidence boundaries frozen.
- [x] Current code and PR #104 state inspected.
- [x] Prior 18-commit feature-wave practice incorporated.
- [x] Parent/child relationship established.
- [x] User approves the final planning summary in a subsequent message.
- [x] `task.py start` executed after approval.
- [x] `trellis-before-dev` spec reading completed in the implementation worktree.

## Completion evidence

All six implementation phases and the final current-host build/smoke gate are
complete. Exact commands, acceptance-criterion mapping, artifact hashes, and
evidence boundaries are recorded in [evidence.md](./evidence.md).
