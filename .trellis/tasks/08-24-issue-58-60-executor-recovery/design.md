# Issues 58-60 executor recovery design

## Boundary

The implementation stays inside the current schema-v20 ChangePlan slice. A
small private `change_plan::adapter` module defines one registered operation
and its typed contract; `ChangePlanService` remains the orchestration facade.
The first implementation delegates its only side effect to the existing Codex
Provider writer.

## Data flow

```text
stored plan + private proof
  -> registered adapter lookup
  -> precheck and atomic admission
  -> durable planned snapshot
  -> durable precheck snapshot -> event
  -> durable snapshot snapshot -> event
  -> atomic cancel/write claim
  -> durable managed_write started -> event
  -> existing Provider writer exactly once
  -> durable readback started -> event
  -> fresh adapter readback and classification
  -> durable finalize started -> event
  -> durable terminal snapshot -> event
```

Events are hints, never state. The callback runs only after the SQLite
transaction commits. A listener that misses any event calls `get_change_job`
and receives the complete safe snapshot.

## Typed adapter contract

The descriptor is a closed serializable value:

- `adapterId`, `adapterVersion`, `operationType`;
- fixed five `phases`;
- closed `readSet` and `writeSet` resource kinds;
- `idempotencyScope=plan`;
- `cancelMode=before_managed_write`;
- `compensationMode=writer_owned_rollback`;
- fault points `before_managed_write` and
  `after_managed_write_before_record`.

The private trait exposes typed inspect/plan/precheck/snapshot/managed-write/
readback/compensation-capability methods. The registry is an exhaustive match
on `ChangeOperation`; there is no string command dispatcher.

## Idempotency

`planId` is already a random one-time approval identity and `change_jobs` has a
unique `plan_id`. It therefore becomes the idempotency key without a new DB
column; `jobId` is the execution ID. Before and after acquiring the Provider
mutation lock, apply checks for an existing job with the same exact persisted
plan digest. It returns that snapshot as an idempotent replay and never reaches
precheck or the writer.

## Cancellation race

An in-memory execution gate uses one atomic state:

```text
cancel_safe --CAS--> cancelled
cancel_safe --CAS--> write_claimed
```

Only one transition can win. The cancel command does not take the Provider or
ChangePlan lock, so it can win while apply is between durable snapshot and the
commit point. Once `write_claimed` wins, the command returns
`commit_point_passed`. Process loss discards the gate; durable running jobs are
then reconciled by readback.

## Durable recovery and partial truth

The existing job row plus phase/resource JSON is the minimal non-sensitive
journal; no new schema column is required. `managed_write=running` is persisted
before the writer. Any job left nonterminal is read back from real authorities.
The partial object is a deterministic projection of durable phase/resource
state, so it survives restart without duplicated storage.

Recovery never calls `managed_write`. With the process-private proof it can
recognize complete target state or restored baseline; without proof it records
`recovery_required` rather than guessing secret equality.

## Compatibility

The wire contract version is bumped while schema remains v20. Old unexecuted
plans become stale across the contract change. Existing persisted terminal job
rows can still be decoded because added DTO fields are derived from job/plan
identity and durable status rather than requiring a migration.
