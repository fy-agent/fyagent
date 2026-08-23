# Issues 58-60 executor recovery implementation plan

1. Add the private registered adapter descriptor/trait and bind its safe fields
   into the plan DTO and approval digest.
2. Replace the four legacy job steps with the five frozen phases and add safe
   execution identity, adapter error, partial-result, and cancellation DTOs.
3. Add DAO lookup by plan ID and normalize derived DTO projections on every
   load without changing schema v20.
4. Add the in-memory atomic execution gate and idempotent existing-job fast
   paths before and after the Provider lock.
5. Refactor apply into commit-then-notify phase snapshots; keep the existing
   Provider writer exactly once and split readback from terminal finalization.
6. Make polling return live active snapshots and make crash reconciliation
   read-only. Add the cancel Tauri command and observer-backed event emission
   for apply/get/list recovery paths.
7. Update fixture, spec, and focused tests for replay, cancellation, event
   ordering, partial classification, and both crash fault points.
8. Run focused fmt/clippy/tests, contracts/architecture checks, full
   `mise run check`, then commit and open a stacked PR against the #130 branch.

Rollback point: if the current v20 rows cannot reconstruct every new public
field deterministically, stop before schema edits and narrow the DTO rather
than silently creating an incompatible unversioned table shape.
