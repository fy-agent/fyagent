# Canonical typed executor design

## Ownership

`services/change_plan` remains the only orchestration owner. The existing
Provider lock-held switch remains the only side-effect writer. A private
`adapter` module provides a closed compile-time registry for supported
operations; commands never dispatch arbitrary strings or paths.

## Compatibility and persistence

Schema v20 is immutable. New executor facts are carried by existing
`steps_json`, `resources_json`, `result_code`, and append-only events.

New jobs use five phases: precheck, snapshot, managed_write, readback, finalize.
The Rust enum keeps legacy `apply` and `reconcile` variants only for decoding
existing rows. DAO read normalization maps legacy phases to the closest new
phase and inserts a skipped `snapshot` phase when legacy evidence cannot prove
one occurred.

The v20 status CHECK has no `cancelled` value. A cancelled-before-write job is
stored with coarse `failed` status plus `cancelled_before_write` result code.
DAO/public normalization derives `ChangeJobStatus::Cancelled` for callers.
This avoids a table rebuild and does not redefine v20.

## Idempotency

`plan_id` is already UNIQUE in `change_jobs`. Before taking the Provider lock,
apply checks a consumed plan with an exact digest and returns the existing job
by `plan_id`. After taking the lock it repeats the same check to close the race.
The admission transaction still creates the only job and consumes the plan.

## Cancellation

Each active execution registers one process-local atomic gate:

`cancel_safe -> cancelled` or `cancel_safe -> write_claimed`.

Only one CAS wins. Cancel does not take the Provider lock. If cancellation wins,
apply persists terminal `cancelled_before_write` without entering the writer.
After `write_claimed`, cancel returns `commit_point_passed`. Process loss drops
the in-memory gate; durable nonterminal jobs are recovered by readback only.

## Observation

All phase transitions go through one helper:

1. mutate in-memory snapshot;
2. save snapshot + event in one SQLite transaction;
3. invoke observer with only `{job_id,event_seq}`.

The observer never owns state. A missed hint is repaired by `get_change_job`.

## Fault and recovery model

Test-only fault points exist immediately before managed write and immediately
after the writer returns but before its result is recorded. They intentionally
leave a durable nonterminal job. Reconcile re-inspects DB/device/target/live
authorities and never invokes managed write.

## Partial truth

Partial result is a pure projection from durable step/resource state. It
contains closed enum/code lists only: succeeded, compensated, unverified,
remaining effects, and manual actions. It stores no raw paths, errors, configs,
or credentials and therefore needs no extra persistence column.

## Explicit non-goals

- no schema migration;
- no old process epoch, plan HMAC, or second Change Plan mutex;
- no second Provider writer;
- no WorkBuddy adapter;
- no general undo engine;
- no V2 layout/presentation redesign.
