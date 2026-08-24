# Apply and Grok UI consolidation

## Goal

Migrate #108 Grok copy/tests and rebuild #113 Apply UI as a pure consumer of the real Change Plan/Job contract, without any fake runtime or native shared wiring.

## Requirements

- Restore Grok Official/xAI wording and all four legacy locale resources/tests from #108, adjusted to current main.
- Build V2 ApplyWorkspace using only real ChangePlan/ChangeJobSnapshot props and a route-local view model.
- Preview never applies; Confirm sends `planId + planDigest` once. StrictMode/double click cannot duplicate apply.
- Success and warning explicitly state no real usage evidence; mixed/unknown/recovery states are never green.
- Secret blocked disables Confirm. Expired/stale/consumed/invalid digest offers regenerate only.
- No scenario fixture, fake coordinator/runtime, cancel, backup, restore or second state machine.
- Do not modify shared FeaturePorts/Tauri/browser composition files; integration owns wiring.

## Acceptance Criteria

- [x] Four-locale parity and Grok selector/footer tests pass.
- [x] Apply view-model exhaustively maps every real domain state.
- [x] Confirm deduplicates StrictMode/double clicks and sends only ID/digest.
- [x] All failure/unknown states are non-green and usage evidence copy is honest.
- [x] Static/tests prove fake/scenario/cancel/backup/restore absent from product surface.

## Notes

- Source PRs: #108 and #113. Product V2 boundaries are authoritative.
