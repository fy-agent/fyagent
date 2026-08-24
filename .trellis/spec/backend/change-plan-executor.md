# Change Plan Typed Executor

## 1. Scope / Trigger

This contract owns the reusable execution layer above the canonical local-only
Change Plan schema-v20 ledger. Use it when adding an operation adapter,
execution phase, idempotency/cancellation behavior, job event, partial result,
or crash-recovery rule.

The current registered operations are `codex_provider_switch`,
`codex_provider_upsert_and_switch`, and `workbuddy_models_save`. The executor
must not become an arbitrary workflow engine: no shell command, script, caller
path, raw Provider definition, network action, or dynamic write target may be
supplied by the renderer or stored in the ledger. Upsert holds the intended
Provider in a process-private draft keyed by `planId`; WorkBuddy save holds the
intended `SaveWorkBuddyModelsRequest` (including API Key) the same way. The
public plan and SQLite row stay credential-free. API keys and overwrite tokens
never appear in plan, job, event, or log payloads.

Schema v20 remains canonical. `change_plans`, `change_jobs`, and
`change_job_events` stay local-only and are not redefined by executor changes.

## 2. Signatures

### Tauri commands

```text
create_codex_provider_switch_plan(targetProviderId) -> ChangePlan
create_codex_provider_upsert_plan(request) -> ChangePlan
create_workbuddy_save_plan(request) -> ChangePlan
apply_change_plan(planId, planDigest) -> ApplyChangePlanOutcome
cancel_change_job(jobId) -> CancelChangeJobOutcome
get_change_job(jobId) -> ChangeJobSnapshot
list_recoverable_change_jobs() -> ChangeJobSnapshot[]
```

Apply accepts exactly `planId + planDigest`. Cancel accepts exactly `jobId`.
Neither command accepts an operation body, path, command, secret, or target
configuration. `create_workbuddy_save_plan` is zero-write; it inspects
`models.json` without taking the WorkBuddy write lock.

### Event hint

```text
change-job://updated -> { jobId, eventSeq }
```

The event is only a cursor hint. Consumers must call `get_change_job`; the
event never carries the full snapshot.

### Adapter contract

The private registered adapter owns typed `inspect`, `plan`, `precheck`,
`snapshot`, `managed_write`, `verify`, and `compensation_capability` methods.
Codex switch and upsert share `CodexExecutionAdapter` and this closed
execution contract; only `adapterId` / `operationType` differ. WorkBuddy save
is a separate adapter and apply path that reuses the same phase machine,
admission, and `persist_transition` helpers. It is not a third
`CodexExecutionAdapter` variant.

Resource sets are operation-scoped. Codex keeps the four-kind read set and
three-kind write set below. WorkBuddy `readSet` and `writeSet` are
`work_buddy_models_config`, `work_buddy_backup`.

```text
adapterId          = codex_provider_switch
                     | codex_provider_upsert_and_switch
                     | workbuddy_models_save
adapterVersion     = 1
operationType      = same as adapterId
phases             = precheck -> snapshot -> managed_write -> readback -> finalize
Codex readSet      = provider_db_current, device_current,
                     target_definition, codex_live_projection
Codex writeSet     = provider_db_current, device_current, codex_live_projection
WorkBuddy readSet  = work_buddy_models_config, work_buddy_backup
WorkBuddy writeSet = work_buddy_models_config, work_buddy_backup
idempotencyScope   = plan
cancelMode         = before_managed_write
compensationMode   = writer_owned_rollback
faultPoints        = before_managed_write,
                     after_managed_write_before_record
```

## 3. Contracts

### Wire version and phase model

- `CHANGE_PLAN_CONTRACT_VERSION = fyagent-change-plan/v2`.
- Wire-contract and adapter versions are independent axes. The current Codex
  adapter is the first registered implementation (`adapterVersion=1`); a wire
  contract revision alone does not imply an adapter-version bump.
- New jobs expose exactly five phases: `precheck`, `snapshot`,
  `managed_write`, `readback`, `finalize`.
- Step status is closed to `pending`, `running`, `succeeded`, `failed`,
  `compensating`, `compensated`, `skipped`.
- Existing v1 persisted `apply`/`reconcile` values remain decode-only legacy
  variants. DAO readback normalizes them to `managed_write`/`finalize` and adds
  a skipped `snapshot=legacy_not_recorded`; it does not rewrite the stored row.

### Idempotency

- `jobId` is the execution ID.
- `planId` is the idempotency key; `change_jobs.plan_id` remains UNIQUE.
- A repeated apply with the same exact digest and current v2 contract returns
  `kind=idempotent_replay` with the existing job and invokes the Provider
  writer zero additional times.
- A different digest is `invalid_digest`; an old contract/adapter is `stale`.
- Concurrent duplicates converge to one admitted job and one or more
  idempotent replays. They never create a second side effect.

### Cancellation and schema-v20 persistence

- An in-memory execution gate has one commit-point race: cancellation may win
  only before `managed_write` is claimed.
- `accepted` cancellation persists `resultCode=cancelled_before_write`, marks
  managed write/readback skipped, and invokes the writer zero times.
- Schema v20's SQLite status CHECK is immutable and does not contain
  `cancelled`. The durable row therefore stores coarse `status=failed` plus
  `result_code=cancelled_before_write`; DAO projection derives public
  `status=cancelled`.
- After managed-write claim, cancel returns `commit_point_passed` and cannot
  hide or roll back an in-flight write.
- The cancellation gate is process-local. If cancellation wins but the process
  is lost before the executor commits the terminal cancellation snapshot, the
  durable ledger still proves only a pre-write interruption. Recovery therefore
  reports `interrupted_before_write`, never fabricates a persisted
  `cancelled_before_write` record, and still invokes the writer zero times.

### Durable transitions and partial truth

- Every observable transition updates the snapshot and appends its event in
  one SQLite transaction before the event hint is emitted.
- `get_change_job` must return an active snapshot without waiting on the
  Provider mutation guard when the execution is known active.
- `partialResult` is derived from durable step/resource state, never stored as
  a second truth. It may contain only phase/resource enums and closed manual
  action codes (`retry_readback`, `review_configuration`).
- A confirmed writer-owned rollback marks `managed_write=compensated`.
  Current Codex has no generic undo executor.
- Adapter error classes must describe facts the executor can actually prove.
  The current Provider writer does not expose retryability, so writer-returned
  failures use `writer_failed`; do not infer `transient` or `permanent` from a
  collapsed writer error. `unknown_outcome` is reserved for execution/readback
  uncertainty, while executor-level cancellation/interruption is represented
  by its result code rather than misclassified as an adapter error.

### Crash recovery

- `managed_write=running` is committed before invoking the existing Provider
  writer.
- Recovery never calls `managed_write` and never replays the Provider writer.
- If process loss occurs before managed write and readback confirms the
  baseline, result is `interrupted_before_write`.
- If process loss occurs after the writer and readback confirms the target,
  result is warning `recovered_target_reached`.
- Mixed or unavailable authority remains `recovery_required`; confirmed
  writer-owned baseline restoration remains a failed execution with
  `writer_failed_baseline_restored`.

### Security and ownership

- Provider mutation remains owned by the existing lock-held Provider writer;
  the executor does not implement a second writer. WorkBuddy apply never takes
  the Codex Provider mutation lock. It takes the existing WorkBuddy tokio
  Mutex via `blocking_lock()` on the `spawn_blocking` apply path, then calls
  `save_workbuddy_models_at_locked`. If the first call returns
  `overwrite_confirmation_required`, apply retries once with that token while
  still holding the lock. The overwrite token never leaves the adapter. A
  successful WorkBuddy write returns `WriterReceipt { live_config_changed: false }`
  and `restartExpectation=not_required`. Reserved `targetProviderId` is
  `fyagent-v2-workbuddy-models`. Plan inspect is read-only. Risks always include
  `local_configuration_write`; when existing model IDs would be updated, add
  `existing_model_ids_will_be_updated`. WorkBuddy classify compares models.json
  revision/content digest and backup digest against the stored baseline. Writer
  failure plus restored baseline is `writer_failed_baseline_restored` with
  recovery succeeded. Revision drift before admit is `stale`.
- Plan/job/event/partial/error DTOs remain credential- and path-free.
- SecretRef integration is separate. A secret-blocked target fails closed
  before admission/writer invocation.
- WebDAV continues to skip and locally preserve all three Change Plan tables.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| unknown adapter/operation/resource/cancel enum | Reject at Rust registry or V2 strict parser; do not execute |
| same plan + same digest after admission | Return existing execution as `idempotent_replay`; writer +0 |
| same plan + changed digest | `invalid_digest`; writer +0 |
| old contract/adapter after executor version change | `stale`; writer +0 |
| cancel wins before managed-write claim | `cancelled_before_write`; public `cancelled`; stored v20 status stays legal; writer 0 |
| cancel arrives after managed-write claim | `commit_point_passed`; execution continues authoritatively |
| crash before managed write; baseline confirmed | `interrupted_before_write`; no replay |
| crash after write; target confirmed | warning `recovered_target_reached`; no replay |
| writer fails and writer-owned rollback is confirmed | failed + `writer_failed_baseline_restored`; `managed_write=compensated` |
| target/readback mixed or unavailable | `recovery_required`; partial result lists unverified/remaining work |
| observer receives `{jobId,eventSeq}` | the matching SQLite snapshot/event sequence is already committed |

## 5. Good / Base / Bad Cases

- Good: two concurrent applies race; one writes once, the other receives the
  same execution as an idempotent replay.
- Good: cancel is accepted after snapshot but before managed-write claim; the
  persisted raw row uses v20 `failed` plus `cancelled_before_write`, while the
  public DTO says `cancelled`.
- Good: a crash after target write is recovered only by real DB/device/live
  readback and becomes `recovered_target_reached` without a second write.
- Base: an old v1 running job is read as five phases with a synthetic skipped
  snapshot marker; its stored JSON remains untouched.
- Bad: add `cancelled` to the existing schema-v20 CHECK without a migration,
  retry the writer during recovery, or let renderer input select a path/script.

## 6. Tests Required

- Rust Change Plan tests assert typed descriptor exactness, plan/digest
  idempotency, concurrent duplicate writer count, stale/TTL/secret rejection,
  and existing Quick Setup/backup/takeover projection parity.
- Cancellation tests must race through the real execution gate and verify both
  pre-write acceptance/writer-zero and post-commit rejection.
- Observer tests must read SQLite inside the callback and prove every emitted
  event sequence is already committed.
- Fault-injection tests cover both descriptor fault points and assert recovery
  changes no writer call count.
- DAO tests insert legacy v1 `apply/reconcile` JSON/events directly and prove
  public normalization without rewriting the raw row.
- Shared `tests/fixtures/changePlanDtoContract.v2.json` must match Rust serde
  and pass the V2 strict parser for plan/job/cancel/event-hint fields.
- V2 tests cover idempotent replay, cancel DTO validation, five-phase labels,
  compensated state, cancelled/interrupted/recovered result copy, and browser
  native-only behavior.
- Run `mise run rust:fmt:check`, `mise run rust:clippy`, `mise run rust:test`,
  `mise run typecheck:v2`, `mise run test:v2`, browser tests, repository
  contracts, then the full prearchive gate.

## 7. Wrong vs Correct

Wrong:

```text
retry apply after timeout -> call Provider writer again
cancelled -> write status='cancelled' into schema v20
event -> emit full mutable job before SQLite commit
```

Correct:

```text
same plan/digest -> return the existing execution, writer +0
cancelled -> persist v20-legal failed + cancelled_before_write -> project cancelled
transition -> commit snapshot + event -> emit only {jobId,eventSeq}
recovery -> inspect real authorities -> classify -> never replay writer
```
