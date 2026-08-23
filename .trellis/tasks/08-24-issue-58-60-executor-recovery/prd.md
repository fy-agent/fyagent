# Implement Issues 58, 59, and 60 executor recovery

## Goal

Turn the existing Codex Provider switch slice into the first registered typed
adapter execution without building a general workflow product. The executor
must make every durable transition observable, make duplicate apply requests
idempotent, allow cancellation only before the first target write, and recover
by readback without replaying the writer.

## Requirements

- Register the existing Codex Provider switch under one closed adapter
  descriptor containing adapter/version/operation, read and write sets,
  idempotency scope, cancel mode, compensation mode, and the two supported
  fault-injection boundaries. No arbitrary script, shell, command, or dynamic
  write target may appear in the plan or execution input.
- Bind the safe adapter descriptor into the per-plan approval binding. Preserve
  all #55 memory-only secret-proof and zero-target-write preview guarantees.
- Use exactly five public execution phases in order:
  `precheck -> snapshot -> managed_write -> readback -> finalize`.
- Commit every phase snapshot before notifying observers. Emit only
  `{jobId,eventSeq}`; `get_change_job` remains the authoritative full snapshot
  and must return active progress without blocking behind the writer.
- Treat `jobId` as `executionId` and the random `planId` as the idempotency key.
  A repeated apply with the same plan/digest returns the existing execution and
  never calls the writer again; a changed digest remains rejected.
- Implement an atomic pre-write cancellation gate. Cancellation that wins the
  gate persists `cancelled_before_write` with writer count zero. After the
  managed-write commit point, cancellation must be rejected and may not hide
  effects already in progress.
- Project structured partial truth from durable steps/resources: succeeded,
  compensated, unverified, remaining effects, and manual action codes. Do not
  put raw errors, paths, Provider definitions, or secrets into the projection.
- Persist `managed_write` running before invoking the existing Provider writer.
  Recovery inspects DB/device/target/live state and classifies target reached,
  baseline restored, unavailable, or mixed state; it never invokes the writer.
- Keep schema v20 and the existing Provider writer. Do not add schema v21,
  generic Undo, automatic post-commit replay, a second writer, frontend UI,
  SecretRef integration, WorkBuddy, or network work during apply.

## Acceptance Criteria

- [x] The shared DTO fixture exposes the registered adapter descriptor,
      execution/idempotency identity, five phases, unified adapter error class,
      cancellation outcome, and structured partial projection.
- [x] First apply calls the Provider writer once; duplicate sequential and
      concurrent applies return the same job and call it zero additional times.
- [x] Cancellation before managed write persists a terminal cancelled snapshot
      and writer zero; cancellation after the atomic commit point is rejected.
- [x] Observer tests prove monotonically increasing event sequences are sent
      only after their matching snapshots can be read from SQLite.
- [x] Pre-write and post-write/pre-record fault injection both leave durable
      recoverable state; reconciliation performs readback only and never
      replays the writer.
- [x] Target already reached after an unknown writer outcome is recognized by
      readback; proof loss after restart remains `recovery_required`.
- [x] Existing #55 drift, approval, replay, secret-canary, and readback tests
      remain green.
- [ ] Focused Rust gates, contract/architecture checks, full `mise run check`,
      and final-head Required CI pass before the PR is considered reviewable.

## Governance Boundary

This stacked PR is evidence for #58/#59/#60 but does not by itself close the
issues. #58 still needs the real WorkBuddy second adapter (#66), and #59/#60
still require the stacked V2/native runtime and crash evidence named in the
approved recovery plan.
