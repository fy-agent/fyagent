# Codex Provider Change Plan Switch — Technical Design

## 1. Architecture

The slice adds a narrow `change_plan` domain beside the existing Provider domain.
The orchestrator owns immutable plans, one-time admission, job snapshots, bounded
events, and reconciliation. `ProviderService` remains the only Provider writer.

```text
Existing Provider switch control
  -> ChangePlanFlow dialog
  -> create_codex_provider_switch_plan(targetProviderId)
  -> ChangePlanService::plan_codex_switch (read-only)
  -> apply_change_plan(planId, planDigest)
  -> ChangePlanService::admit + create job
  -> CodexProviderSwitchAdapter
       -> ProviderService::switch (existing writer)
       -> DB/settings/live readback
  -> durable ChangeJobSnapshot
  -> query snapshot + event invalidation
```

There is no dynamic adapter registry in the first slice. A closed Rust enum and
exhaustive TypeScript union contain only `codex_provider_switch`.

## 2. Baseline strategy

The implementation is based on `origin/main@4b4e1754`. PR #104 is closed without
merge, so no `src/v2` code is used or modified. The shared dialog lives under the
current component architecture and exposes DTOs that a later V2 consumer can
reuse.

Change Plan persistence is additive under schema v16:

- `change_plans`
- `change_jobs`
- `change_job_events`

`Database::create_tables` creates them idempotently for new and existing v16
databases. The slice does not bump `SCHEMA_VERSION`, so it cannot collide by
claiming v17 while Prompt/Memory Native Integration is still unresolved. Schema
tests must prove that initialization adds the tables and leaves `user_version=16`.

## 3. Data contracts

### Plan

`ChangePlan` contains:

- opaque `planId`
- `operation=codex_provider_switch`
- `targetProviderId` and presentation-safe target name
- `planDigest`, `baselineDigest`
- `createdAt`, `expiresAt`, `status`
- current/target presentation codes, restart expectation, risks, evidence note

The persisted private plan payload additionally records domain-separated digests
for:

- effective current Provider identity
- target Provider semantic definition
- Codex live projection
- app/schema contract version

It does not persist target settings or secret values. Apply re-reads the target
Provider by ID and rejects definition drift before calling the writer.

### Job

`ChangeJobSnapshot` contains:

- `jobId`, `planId`, monotonic `revision`/`eventSeq`
- `status=planned|running|succeeded|warning|failed`
- `resultCode`
- bounded steps: `precheck|apply|readback|reconcile`
- resource results for `provider_db_current`, `device_current`,
  `target_definition`, and `codex_live_projection`
- `restartRequirement`
- `usageEvidence=not_observed`
- recovery state and stable diagnostic code

No raw backend error is serialized. Internal errors are mapped to stable codes and
logged through existing redaction rules.

## 4. Digest and redaction

Digest input uses a versioned, domain-separated struct with fixed field ordering.
Provider semantic input is hashed in the backend and never serialized to the
renderer or Change Plan tables. Plan digest covers operation, target ID, baseline
digest, contract version, and stable presentation codes; transient timestamps and
plan ID do not affect semantic equivalence.

Tests seed obvious sentinel secrets and absolute paths, then scan plan/job/event
rows, IPC JSON, logs captured by the test harness, and rendered fixtures.

## 5. Plan and admission flow

1. Reject non-Codex or missing target.
2. Read effective current Provider from DB + device settings.
3. Read target Provider and safe display name.
4. Read the Codex live projection without writing.
5. Build baseline and semantic plan digests.
6. Persist immutable, unconsumed plan with 15-minute expiry.
7. Return safe projection.

`apply_change_plan` runs one SQLite admission transaction that verifies plan ID,
digest, expiry, status, and unconsumed state; re-inspects the baseline before
marking the plan consumed and creating exactly one job. Baseline drift returns a
typed stale result without a job or write event.

## 6. Apply, readback, and reconciliation

The adapter executes the existing Codex switch path under its existing locking and
rollback behavior. The job ledger records `apply started` before the call and the
bounded outcome afterward.

Readback independently checks:

- database current Provider equals target
- device-local current Provider equals target
- target Provider definition still matches the admitted digest
- live Provider projection matches the target’s expected safe projection

Classification:

- all target predicates true: `succeeded`
- target reached but an auxiliary/restart observation is unavailable: `warning`
- baseline predicates restored: `failed`, `recovery=succeeded`
- mixed/third state: `failed`, `recovery=recovery_required`
- readback unavailable: `failed`, `result=readback_unavailable`

On process restart or query of a nonterminal job, reconciliation reads the same
resources and applies this classifier. It never calls `ProviderService::switch`
again automatically.

## 7. Commands and frontend boundary

New Tauri commands:

- `create_codex_provider_switch_plan(target_provider_id)`
- `apply_change_plan(plan_id, plan_digest)`
- `get_change_job(job_id)`
- `list_recoverable_change_jobs()`

`change-job://updated` carries only `jobId + eventSeq`; the renderer invalidates
and refetches the snapshot. Polling remains the fallback.

Frontend additions:

- `src/lib/api/change-plan.ts`
- `src/lib/query/change-plan.ts`
- `src/components/change-plan/ChangePlanFlow.tsx`
- focused child components and tests under the same directory

The existing Codex switch entry stores the selected target and opens the flow. It
does not call `useSwitchProviderMutation` directly. Non-Codex paths retain the
existing hook.

## 8. Compatibility and rollback

- Old hosts without the new commands show a typed unsupported message; they do
  not silently fall back to direct mutation for Codex switch.
- Reverting the frontend entry restores no hidden automatic writer path inside
  the new component; rollback is a deliberate commit-level operation.
- Additive tables are ignored by older binaries. No existing Provider table or
  row format changes.
- Each implementation stage is independently revertible until the final entry
  routing commit.

## 9. File ownership and protected scope

Single owner may modify only the narrow Change Plan domain, DB schema/tests,
command registration, Provider switch adapter seam, current Provider switch UI
entry, i18n, and focused tests.

Protected:

- `src/v2/**`
- Prompt/Memory task artifacts and preview generator
- WorkBuddy code
- Provider create/edit/delete behavior
- current source checkout and its untracked reference/image directories
- Native Integration/Profile v2/Workspace Pack worktrees

## 10. Trade-offs

- Additive v16 tables avoid a known migration-number collision, at the cost of a
  later coordinated migration that may formalize them without changing shape.
- A single adapter enum is less extensible than a registry, but prevents premature
  generic infrastructure.
- Readback-based reconciliation cannot make file I/O transactional, but it is more
  honest and safer than replaying an uncertain side effect.
- Existing UI integration avoids coupling the feature to a closed V2 PR; later V2
  adoption remains a separate integration task.
