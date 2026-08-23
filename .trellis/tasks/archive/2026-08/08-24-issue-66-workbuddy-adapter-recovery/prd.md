# Issue 66 WorkBuddy Change Plan adapter recovery

## Goal

Prove that the shared Unified Change Plan contract can execute a second real
native adapter by routing WorkBuddy `models.json` changes through one preview,
one confirmation, five persisted phases, exact readback and no-replay recovery.

## Requirements

- WorkBuddy remains outside Provider/AppType and keeps the existing fixed-path,
  revision, backup and atomic-replacement writer as the only mutation owner.
- Plan creation may insert one UCP ledger row. It must not write WorkBuddy,
  create a job/event, issue an overwrite capability, or perform network I/O.
- Base URL, API key, exact baseline bytes, request-bound revision and private
  target evidence remain process-private. SQLite, IPC responses, event hints,
  logs and exports contain only safe counts/codes and per-plan approval binding.
- Apply rechecks the exact private revision before admission, then runs
  `precheck -> snapshot -> managed_write -> readback -> finalize`.
- The adapter may acquire and consume the existing request/revision-bound
  overwrite token internally after the one UCP confirmation. No second
  WorkBuddy confirmation or renderer-visible token is allowed.
- Readback rereads the real `models.json`, verifies the admitted semantic target
  including supplied credential material, and never performs a model request.
- Unknown or mismatched outcomes restore the captured baseline when safely
  possible. Reconciliation may read and restore but must never replay the
  WorkBuddy writer.

## Acceptance Criteria

- [ ] Preview is side-effect-free outside one UCP ledger insert and public
      surfaces contain no API-key canary, raw path, private revision or bytes.
- [ ] API-key-only/external drift, expiration and process-proof loss stop before
      WorkBuddy writes; duplicate apply returns the same job.
- [ ] Add/update/delete use one preview and one confirmation, with no special
      overwrite/delete confirmation and no renderer-visible capability.
- [ ] Successful execution emits persisted snapshots for all five phases and
      reports real WorkBuddy config/backup readback with `usage=not_observed`.
- [ ] Writer error, semantic mismatch and interrupted execution distinguish
      target reached, baseline restored and recovery required without replay.
- [ ] Existing unknown JSON fields, backup semantics, atomic replacement and
      Windows handle-pinned storage remain owned by the WorkBuddy service.
- [ ] Focused Rust and V2 tests, V2 gates, full `mise run check`, macOS native
      UAT and final-head Required CI pass. Issue #66 remains open until required
      Windows native evidence and merge evidence exist.

## Closure checklist

1. Freeze WorkBuddy operation/resource/plan request DTOs and private proof.
2. Add pure preview, locked execution/readback and baseline restoration facade.
3. Register the typed adapter and route add/update/delete through shared V2 UI.
4. Run canary, drift, idempotency, readback, crash and native UAT gates.
5. Push a stacked Draft PR with exact evidence and leave #66 open.

## Notes

The output is a stacked recovery PR based on the exact #63 head. It does not
merge `main`, close #66, or claim Windows HIL.
