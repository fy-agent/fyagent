# Canonical Change Plan typed executor

## Goal

Move the still-valid typed executor capabilities from Draft PR #134 onto the
canonical Change Plan implementation already merged through PR #135. The final
implementation must preserve the current schema-v20/local-only ledger,
Provider single-writer path, targeted Codex projection, and readback-only
recovery while adding the reusable execution contract needed by Issues #58,
#59, and #60.

## Requirements

- Add one closed, versioned typed adapter descriptor for the existing Codex
  Provider switch. The descriptor declares operation, five execution phases,
  read/write resource sets, plan-scoped idempotency, pre-write cancel boundary,
  compensation capability, and two deterministic test-only fault boundaries.
  No shell/script/dynamic write target enters plan or apply input.
- Preserve canonical schema v20. Do not redefine its tables or restore the old
  #130 process-epoch/HMAC/private-proof model. Do not introduce schema v21 for
  this slice.
- Expose five new execution phases for new jobs:
  `precheck -> snapshot -> managed_write -> readback -> finalize`. Existing v1
  persisted `apply/reconcile` steps/events must remain readable and normalize
  safely to the new public projection.
- Make repeat apply with the same `planId + planDigest` idempotent: return the
  existing execution snapshot and invoke the writer zero additional times.
  A changed digest is rejected. Concurrent duplicates must converge to one
  `jobId`/execution.
- Add cancellation only before the managed-write commit point. Cancellation
  that wins persists a terminal `cancelled_before_write` result with writer
  count zero; cancellation after the commit point is rejected and never hides
  a write in progress.
- Keep v20 DB status values unchanged. A cancelled execution is stored using a
  v20-compatible coarse terminal status plus `result_code=cancelled_before_write`;
  the public snapshot derives `status=cancelled` from that authoritative result.
- Add structured partial truth derived from durable steps/resources: succeeded,
  compensated, unverified, remaining effects, and manual-action codes. Never
  include paths, raw errors, Provider definitions, or secrets.
- Persist every observable phase transition before invoking its observer/event
  hint. Events carry only `jobId + eventSeq`; `get_change_job` remains the
  authoritative full snapshot and must not block behind the Provider writer.
- Persist managed-write running before the existing Provider writer. Recovery
  and crash reconciliation inspect actual authorities and never replay the
  writer. Explicit test-only faults cover immediately before managed write and
  immediately after writer return but before its result is durably recorded.
- Keep SecretRef integration, WorkBuddy, general undo, arbitrary workflows,
  network actions during apply, and V2 presentation redesign out of this task.

## Acceptance Criteria

- [ ] New plans/jobs expose one closed typed adapter descriptor and five phases;
      renderer parsers reject unknown descriptor/resource/cancel values.
- [ ] Sequential and concurrent duplicate apply with the same digest return the
      same execution and call the Provider writer no additional times; changed
      digest remains rejected.
- [ ] Pre-write cancellation produces a durable public `cancelled` snapshot
      without violating the schema-v20 status CHECK; post-commit cancellation
      is rejected.
- [ ] Existing persisted v1 `apply/reconcile` job/event rows decode and
      normalize without migration or data loss.
- [ ] Observer tests prove every emitted `{jobId,eventSeq}` resolves to an
      already-committed SQLite snapshot with at least that sequence.
- [ ] Both fault boundaries leave durable recoverable truth; reconciliation
      performs readback only and writer call count remains unchanged.
- [ ] Partial-result projection is deterministic, secret/path free, and
      accurately identifies succeeded/unverified/remaining/manual work.
- [ ] Existing #135 stale/TTL/secret/projection/readback/single-writer tests stay
      green; no schema v20 or WebDAV local-only regression is introduced.
- [ ] Maintained backend/frontend SPECs describe the final executor contract,
      compatibility mapping, and cancellation persistence rule.
- [ ] Full local gate, Trellis prearchive/archive/post-archive contracts, exact
      head PR CI, and Merge Queue `CI / Required` pass before merge.

## Notes

- Draft PR #134 is an implementation/reference source only and must be closed as
  superseded after the replacement PR exists. Its old #130 ancestry and
  incompatible schema-v20 definition do not enter main.
- Issue #58 can be partially closed by the first typed Codex adapter, but its
  second-adapter proof remains WorkBuddy. Issues #59/#60 retain any acceptance
  that depends on later V2 runtime/crash evidence outside this backend slice.
