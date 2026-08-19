# Codex Provider Change Plan Switch Vertical Slice

## Goal

Deliver the first real Unified Change Plan vertical slice for switching from the
current Codex Provider to an existing Codex Provider. The user must review one
immutable, baseline-bound plan, approve it once, and receive a durable job result
whose success is derived from local readback rather than a mutation return value
or toast.

This slice proves the reusable plan/apply/readback contract without expanding
into Provider creation/editing, WorkBuddy, Prompt/Memory writes, network probes,
or the complete SecretBackend product.

## User value

- The user sees exactly which existing Provider will become current before any
  local state changes.
- A plan becomes stale when the current Provider, target Provider definition, or
  Codex live configuration changes; stale plans perform zero writes.
- The result distinguishes applied, warning, failed-and-recovered, and partial or
  recovery-required outcomes.
- “Configuration applied” remains separate from “real Agent usage observed”.

## Confirmed baseline and facts

- Implementation branch: `codex/unified-change-plan-codex-switch`.
- Immutable product baseline: `origin/main@4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287`.
- PR #104 (`codex/prompt-memory-v2-main-pr`) is closed without merge. The slice
  therefore integrates with the current Provider UI on `main` and creates a
  reusable dialog/component boundary; it does not depend on unmerged V2 files.
- Existing Provider switching is owned by `ProviderService::switch`; the new
  orchestrator must reuse it rather than copy the writer.
- `ProviderMutationResult.liveConfigChanged` only compares final Codex live-file
  bytes and is a restart hint. It is not the readback predicate and is not real
  usage evidence.
- The current database schema version is 16. Prompt/Memory Native Integration
  separately plans the next versioned migration, so this slice must not claim or
  consume schema version 17.

## In scope

### Plan

- Generate a side-effect-free switch plan for `AppType::Codex` and an existing
  target Provider.
- Bind the plan to the effective current Provider, target Provider definition,
  relevant Codex live configuration, application/schema version, and a 15-minute
  expiry.
- Persist only identifiers, domain-separated digests, stable presentation codes,
  timestamps, and non-sensitive projections. Do not persist Provider settings,
  secret values, raw config, absolute paths, or full file digests in renderer
  payloads.
- Equivalent intent on the same baseline produces the same semantic plan digest;
  each request still receives a distinct plan ID.

### Approval and apply

- The UI exposes one “Confirm and apply” action.
- `apply_change_plan(planId, planDigest)` is the approval boundary. The backend
  validates and consumes the plan atomically; no approval token is exposed to the
  renderer and replay is rejected.
- Apply re-reads the full baseline before the first write. Expired, consumed,
  digest-mismatched, or drifted plans create no write job and perform zero writes.
- A valid apply creates a durable job, calls the existing Provider switch path,
  and records bounded step transitions for precheck, apply, readback, and any
  reconciliation.

### Readback and recovery truth

- Read back the database current Provider, device-local current Provider setting,
  target Provider record, and Codex live projection after the writer returns.
- A successful result requires the target Provider to remain unchanged and both
  current-provider authorities plus the live projection to agree with the plan.
- If the writer returns an error or post-write readback fails, re-inspect actual
  state. Classify baseline restored, target reached, or third/unknown state; never
  blindly replay the switch.
- Persist enough non-sensitive journal state for an interrupted job to be
  reconciled after process restart by readback, without automatically repeating
  an unknown write.

### Frontend

- Add a shared Change Plan dialog used by the existing Codex Provider switch
  entry.
- Render preview, expiry/stale state, single confirmation, bounded step progress,
  local readback, restart recommendation, warning/partial/recovery outcome, and
  the fixed evidence statement `usageEvidence=not_observed`.
- A backend event may invalidate the job query, but the query snapshot is
  authoritative and polling must recover from missed or duplicate events.
- Remove the direct switch-success toast from this Codex path. Other AppTypes and
  Provider create/edit retain existing behavior in this slice.

## Out of scope

- Provider create, edit, delete, import, sort, proxy, failover, or additive-app
  flows.
- Claude, Gemini, OpenCode, OpenClaw, Grok Build, Hermes, or other AppType switch
  flows.
- WorkBuddy integration or removal of its overwrite confirmation.
- Full `#35` SecretBackend implementation, credential rotation, or hardware
  backend work.
- Prompt/Memory Native Integration, Profile v2, Workspace Pack, or V2 Shell merge.
- Network reachability checks, provider/model probes, model-list requests, real
  model calls, or passive usage observation.
- A generic transaction DSL, arbitrary resource registry, cross-adapter atomicity,
  or force-apply path.
- Creating or merging a pull request; a verified branch may be pushed, while main
  publication remains a separate decision.

## Acceptance criteria

### AC-01 Side-effect-free plan

- Creating a plan does not change the database current Provider, device-local
  current setting, Provider records, Codex live file, jobs, or external services.
- Tests inject write counters or exact before/after snapshots and prove zero
  mutation and zero network access.

### AC-02 Stable identity and redaction

- Plan ID is unique; semantic plan digest is stable for the same intent/baseline.
- Plan/job/event/IPC serialization contains no secret value, Provider settings,
  raw live configuration, absolute path, or unrestricted backend error text.

### AC-03 One approval and replay resistance

- One explicit UI action applies the reviewed `planId + planDigest`.
- Expired, mismatched, stale, already-consumed, or replayed plans make zero writes
  and return a stable typed outcome.

### AC-04 Baseline drift

- Changing current Provider, target Provider settings, or Codex live projection
  after plan creation makes the plan stale.
- Stale handling keeps the current real state intact and offers replan, never
  force overwrite.

### AC-05 Durable job and bounded steps

- A valid apply creates one job with monotonic event sequence and observable
  `precheck -> apply -> readback` state.
- Reloading the dialog or missing events recovers the same snapshot from query.
- Repeated apply requests do not create a second switch attempt.

### AC-06 Readback-derived result

- `succeeded` requires database current, device-local current, target Provider
  revision, and live projection to match the plan.
- `liveConfigChanged` affects only restart guidance.
- Required readback failure or disagreement never renders green success.

### AC-07 Failure and interruption truth

- Writer error, post-write mismatch, interrupted job, baseline-restored state,
  target-reached state, and third/unknown state have distinct stable outcomes.
- Reconciliation inspects before acting and never blindly repeats an unknown
  switch.

### AC-08 Frontend behavior

- The existing Codex Provider switch action opens the shared preview dialog and
  no longer directly mutates.
- Preview, stale, running, succeeded, warning, failed-and-recovered, and
  recovery-required fixtures are keyboard accessible and exhaustively rendered.
- UI always separates local configuration readback from real usage evidence.

### AC-09 Compatibility and scope

- Non-Codex and non-switch Provider paths retain their prior behavior and tests.
- No file under `src/v2/`, Prompt/Memory tasks, WorkBuddy, or the source checkout’s
  untracked image/reference directories is modified.
- The additive Change Plan tables work on a schema-v16 database without claiming
  schema version 17; future migration integration remains explicit.

### AC-10 Verification and delivery

- Focused Rust contract/store/provider tests pass before frontend integration.
- Focused TypeScript API/query/component/hook tests pass before full integration.
- Fresh repository quality gates, Trellis validation, `git diff --check`, scope
  audit, and branch-diff review pass on the final source.
- Each stage is an independently reviewable commit; final verified changes are
  pushed to `codex/unified-change-plan-codex-switch` without merging main.

## Key decisions

- Start with switching an existing Codex Provider because it proves the full
  contract without introducing new credential storage.
- Use one sequential owner: backend DTO/store/IPC, Provider writer integration,
  and the first frontend entry share too many registration and contract files for
  safe parallel editing.
- Integrate with the current Provider UI on `main`; the closed, unmerged V2 PR is
  a future consumer, not a baseline dependency.
- Treat apply as the single approval boundary rather than exposing a separate
  renderer approval token.
- Use additive, idempotent tables under schema v16 for this slice and reserve the
  next versioned migration number for coordinated integration.
- Reconcile unknown outcomes by readback, never automatic replay.

## Deferred items and risks

- The later general Change Plan program must coordinate schema numbering with
  Prompt/Memory Native Integration before either branch lands.
- Provider create/edit will need the stable #35 SecretBackend contract.
- A later V2 integration must map the same DTOs and result dialog without cloning
  the state machine.
- Process termination during an external file replace cannot be made SQLite-
  transactional; the ledger records uncertainty and readback decides the outcome.

## Blocking open questions

None. The user explicitly approved implementation and the current-host build
closure gate on 2026-08-14.
