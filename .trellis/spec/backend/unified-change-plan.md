# Unified Change Plan: Registered Native Adapter Contract

## 1. Scope / Trigger

Read this contract before changing the Unified Change Plan verticals for Codex
Provider switch/upsert or WorkBuddy model configuration. It covers plan
identity, schema-v20 persistence, registered adapter identity, idempotent
admission, the five-phase executor, domain mutation ownership,
readback/reconciliation, and the Rust/Tauri wire DTOs. It does not authorize
arbitrary adapters, dynamic writes, generic Undo, or a generic execution
engine.

Plan creation may insert one immutable UCP control-plane row. It must perform
zero Provider/business-state writes, zero live/external-target writes, zero
job/event writes, and zero network requests.

## 2. Signatures

```text
create_codex_provider_switch_plan(targetProviderId: String)
  -> Result<ChangePlan, ChangePlanErrorCode>

create_codex_provider_upsert_plan({ name, baseUrl, apiKey, modelId, codexFeatures? })
  -> Result<ChangePlan, ChangePlanErrorCode>

create_workbuddy_models_plan({
  baseUrl, apiKey, allowNoApiKey, selectedModelIds, manualModelIds,
  removedModelIds, clearExistingApiKeys, expectedRevision
}) -> Result<ChangePlan, ChangePlanErrorCode>

apply_change_plan(planId: String, planDigest: String)
  -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>

get_change_job(jobId: String)
  -> Result<ChangeJobSnapshot, ChangePlanErrorCode>

list_recoverable_change_jobs()
  -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode>

cancel_change_job(jobId: String)
  -> Result<CancelChangeJobOutcome, ChangePlanErrorCode>

event change-job://updated
  -> { jobId: String, eventSeq: i64 }
```

The existing Provider and WorkBuddy services remain the only writers. Codex
UCP holds `ProviderService::lock_provider_mutation(..., Codex)` across baseline
check, SQLite admission, one guarded writer call, and fresh readback. WorkBuddy
UCP holds its existing mutation lock across the equivalent lifecycle and calls
the existing atomic fixed-path writer through a lock-held facade. No second
domain writer is allowed.

Schema v20 adds:

```text
change_plans(plan_id, operation, target_provider_id, target_provider_name,
  plan_digest, baseline_digest, actor_code, source_version, plan_revision,
  proof_id, process_epoch_id, current_provider_id, current_provider_code,
  target_provider_code, contract_digest, created_at, expires_at, status,
  consumed_at)
change_jobs(... one UNIQUE plan_id ...)
change_job_events(PRIMARY KEY(job_id, event_seq), ...)
```

## 3. Contracts

- `ChangePlan` uses camelCase and includes `planId`, `operation`, target ID and
  bounded display name, `planDigest`, `baselineDigest`,
  `actor.type=direct_user`, `sourceVersion`, `revision=1`, timestamps, closed
  status/codes, restart expectation, risks, and evidence note.
- Each plan embeds one closed adapter descriptor: adapter/version/operation,
  five fixed phases, read/write sets, plan-scoped idempotency, pre-write-only
  cancellation, writer-owned rollback, and the two test-only fault boundaries.
  The registry is an exhaustive `ChangeOperation` match and accepts no shell,
  script, argv, dynamic command, or undeclared write target.
- `planDigest` and `baselineDigest` are per-plan opaque approval bindings over
  non-secret fields. They start with `mac1:` and are not stable content hashes.
- The apply gate recomputes `planDigest` from every immutable public plan field,
  the random proof/epoch IDs, the baseline binding, and the contract identity.
  Mutating a stored expiry, source version, target display field, risk, or
  contract must produce `stale` before admission; comparing only the caller
  digest to the stored digest is insufficient.
- Full current/target Provider definitions and Codex live/target projections
  are bound only by process-private HMAC proofs keyed by random `proofId`.
  Private proof bytes never enter SQLite, IPC, logs, exports, events, or Debug.
- A WorkBuddy plan keeps the full request, exact credential-bearing preimage,
  and complete-byte revision only in the process-private proof map. SQLite and
  IPC receive only bounded model counts/codes, random proof/epoch IDs, and the
  per-plan non-secret approval binding. Plan creation performs no WorkBuddy
  write, backup, overwrite-token issuance, network request, job, or event.
- SQLite stores only the random `proofId`, random `processEpochId`, bounded
  non-sensitive metadata, and non-secret approval bindings. A process restart
  loses the private proof by design.
- `apply` accepts only the exact stored `planId + planDigest`, rechecks the
  non-secret binding and private proof under the Provider mutation lock, then
  atomically consumes the plan and creates one job before invoking the writer.
- `jobId` is the execution ID and the random `planId` is its idempotency key.
  The unique job-per-plan row is the durable authority. Repeating the exact
  apply returns the existing full job snapshot as `idempotent_replay`; it does
  not precheck or call the writer again. Reusing the plan with another digest
  remains `invalid_digest`.
- Public execution phases are exactly `precheck -> snapshot -> managed_write
  -> readback -> finalize`. Every transition is committed with an increasing
  `eventSeq` before the host emits `change-job://updated {jobId,eventSeq}`.
  Events are hints; `get_change_job` is the full safe snapshot and returns an
  active execution without blocking behind its writer.
- An in-memory atomic gate chooses exactly one transition from `cancel_safe` to
  either `cancelled` or `write_claimed`. Cancellation that wins persists
  `cancelled_before_write` and writer zero. After `write_claimed`, cancellation
  is rejected as `commit_point_passed` and cannot hide existing effects.
- The durable phase/resource journal projects a non-sensitive partial result:
  succeeded/compensated/unverified phases, remaining effect codes, and bounded
  manual action codes. It never projects raw errors, paths, definitions, or
  secret material.
- `managed_write=running` is committed before invoking the Provider writer. A
  process loss after that boundary is an unknown outcome, so recovery performs
  readback only. Complete target state becomes `recovered_target_reached`;
  complete baseline becomes compensated; mixed/unavailable state requires
  recovery. A journal proving managed write never started becomes
  `interrupted_before_write` without readback or replay.
- Writer return is not success evidence. DB current, device current, target
  definition, and Codex live projection must pass fresh readback.
- WorkBuddy apply performs no network request and never enters Provider/AppType.
  Its one UCP confirmation authorizes the admitted plan. If the existing writer
  requires an overwrite capability, the adapter issues and consumes that
  request/revision-bound token internally; it never crosses IPC and there is no
  WorkBuddy-specific second confirmation.
- WorkBuddy readback rereads the real `models.json`. Exact writer revision or a
  same-process semantic target proves target state. A post-write mixed state is
  restored from the exact private baseline through the same fixed-path atomic
  storage service when possible; reconciliation never invokes the writer.
- A known-missing Codex live file is a bindable baseline distinct from an
  unreadable/malformed file. Unreadable/malformed state cannot create a plan
  or reach the writer. The first switch slice also rejects proxy-takeover mode
  before persistence until a proxy-aware target projection is implemented.
- Reconciliation reads and classifies only. It never calls the writer. If the
  private proof is unavailable after restart, the job is
  `recovery_required`; current IDs may still be displayed, but secret equality
  is never guessed.
- `usageEvidence=not_observed` is the only claim in this slice.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Target missing | `target_not_found`; no plan row |
| Target already effective current | `target_already_current`; no plan row |
| Malformed/unreadable live baseline | `baseline_unavailable`; no plan row |
| Proxy takeover active | `unsupported_operation`; no plan row |
| Unknown plan | rejected `plan_not_found`; writer zero |
| Wrong plan digest | rejected `invalid_digest`; writer zero |
| `now >= expiresAt` | rejected `expired`; writer zero |
| Exact duplicate apply after admission | existing job + `idempotent_replay`; writer zero |
| Consumed plan whose unique job is missing | fail closed as internal corruption; writer zero |
| Same plan with changed digest | `invalid_digest`; writer zero |
| Cancel wins before managed write | terminal `cancelled_before_write`; writer zero |
| Cancel after managed-write claim | `commit_point_passed`; execution continues |
| IDs, target definition, common config, live projection, or API key drift | rejected `stale`; writer zero |
| Stored immutable plan field or contract identity mutated | rejected `stale`; writer zero |
| Process epoch/private proof missing | unapplied plan `stale`; nonterminal job `recovery_required`; no replay |
| Writer error and baseline restored | failed `writer_failed_baseline_restored`, recovery succeeded |
| Writer error but complete target readback | warning `writer_error_target_reached` |
| Mixed state or definition drift after write | failed `post_write_mismatch`, recovery required |
| Readback unavailable | failed `readback_unavailable`, recovery required |
| Crash before `managed_write` journal | failed `interrupted_before_write`; writer zero |
| Crash after writer side effect, before receipt journal | readback target -> `recovered_target_reached`; never replay |
| WorkBuddy API-key-only or complete-byte revision drift | rejected `stale`; no job, backup, or writer |
| WorkBuddy overwrite is required after UCP confirmation | token issued/consumed only inside the adapter; one writer commit |
| WorkBuddy post-write mismatch with restorable baseline | failed `writer_failed_baseline_restored`, recovery succeeded |
| WorkBuddy interrupted job loses private proof on restart | failed `recovery_required`; never replay or guess secret equality |

## 5. Good / Base / Bad Cases

- Good: two previews over unchanged secret-bearing state produce distinct
  plan IDs, proof IDs, and approval bindings; only two immutable plan rows are
  added and no target state changes.
- Good: API-Key-only drift between preview and apply is detected by the
  memory-only proof before admission, so writer calls remain zero.
- Base: a valid apply calls the existing Provider writer once, reports
  `not_observed`, and uses readback to decide success/restart truth.
- Good: WorkBuddy preview inserts one safe plan row and leaves the primary,
  backup, jobs, events, and network untouched; one confirmed apply consumes any
  legacy overwrite capability internally and performs one atomic commit.
- Good: WorkBuddy crash reconciliation either proves the target, restores the
  exact baseline, or requires recovery. It never replays the writer.
- Good: concurrent exact duplicate applies all return one execution ID and the
  Provider writer count remains exactly one.
- Bad: hash or HMAC full Provider/live projections and persist the result.
  Even keyed output is secret-derived durable state and conflicts with #35.
- Bad: release the Provider mutation guard between baseline validation and
  writer/readback, or call public `ProviderService::switch` while already
  holding that guard.

## 6. Tests Required

Focused Rust assertions must cover:

- closed camelCase DTOs and the shared fixture;
- frozen `cancelled`, `warning`, and `not_started` wire spellings;
- v19-to-v20 migration plus identical fresh-database shape;
- exactly one plan-ledger insert and zero Provider/live/job/event changes on
  preview;
- no raw secret or private HMAC in persisted UCP fields;
- digest mismatch, expiry, stale ID/config/live/API-key state, proof loss, and
  replay all calling the writer zero times;
- stored-plan mutation, missing-to-malformed live drift, unreadable baseline,
  and proxy takeover rejection before the writer;
- existing writer exactly once, independent four-authority readback, writer
  failure classifications, terminal-race reload, and no-replay reconcile;
- process-proof loss yielding `stale` or `recovery_required` as appropriate;
- both Unix and Windows path separators in display-name sanitization.
- closed registered adapter metadata with no dynamic command surface;
- the exact five phase order and committed-snapshot event sequencing;
- concurrent/sequential idempotent replay with one execution and one writer;
- cancellation winning before write and rejection after the atomic claim;
- structured partial projection and compensated rollback classification;
- before-write and after-write-before-record fault injection, followed by
  read-only reconciliation that never replays the writer.
- WorkBuddy preview zero-write and canary non-persistence; internal overwrite
  capability, exact real-file readback, duplicate apply idempotency,
  API-key-only stale detection, post-write compensation, and proof-loss crash
  recovery without writer replay.

Final gates:

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run test:unit -- tests/remainingPlatformSurface.test.ts
```

GitHub Windows/Linux/macOS required jobs must pass before calling the PR
reviewable. Local macOS evidence is not Windows runtime evidence.

## 7. Wrong vs Correct

Wrong:

```text
SHA256(full Provider/live JSON) -> change_plans.*_digest
check baseline -> release lock -> ProviderService::switch -> readback
restart + missing secret proof -> infer success from current provider ID
```

Correct:

```text
full secret-bearing projections -> memory-only per-plan HMAC proof
non-secret metadata + random proofId -> persisted approval binding
hold existing Provider mutation lock:
  baseline + private proof -> atomic admission
  -> precheck -> snapshot -> managed_write journal
  -> existing writer once -> readback -> finalize
restart + missing private proof -> stale/recovery_required, never replay
```
