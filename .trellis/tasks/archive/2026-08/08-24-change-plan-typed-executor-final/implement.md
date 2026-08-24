# Implementation plan

1. Extend canonical domain DTOs with adapter descriptor, five new phases,
   idempotent replay, cancel outcome, partial projection, and closed fault/error
   classifications while retaining legacy phase decode variants.
2. Add private typed adapter registry for Codex Provider switch and bind the
   descriptor into new plan approval digest/contract version.
3. Extend DAO with get-job-by-plan, v20-compatible cancellation storage mapping,
   and legacy job/event normalization. Do not alter schema.rs.
4. Refactor executor sequencing around one persist-then-observe helper; add
   double-checked idempotent replay and process-local cancellation gate.
5. Add test-only fault injection and deterministic partial-result projection;
   prove recovery never calls the writer.
6. Add cancel command/permission and update the V2 port/parser contract without
   changing presentation layout.
7. Update backend Codex Change Plan SPEC and frontend V2 Change Plan SPEC,
   including v1 compatibility and schema-v20 cancellation storage semantics.
8. Run focused Rust/renderer tests, supported-platform manifest check, full
   `mise run check`, Trellis prearchive, archive, and post-archive contracts.
9. Create replacement PR, close #134 as superseded, enable exact-head
   Merge-when-ready, and let Merge Queue validate the latest-main candidate.

## Stop conditions

- Any implementation requires redefining schema v20 or silently rebuilding the
  local ledger table.
- Any cancellation path can run after managed-write claim yet report no effects.
- Any recovery/fault path can invoke the Provider writer a second time.
- Any DTO/event exposes a path, raw Provider config, OS error, or secret-bearing
  material.
