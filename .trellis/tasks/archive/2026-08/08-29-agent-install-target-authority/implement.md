# Implement — Stage 1

## 1. Preflight

- [ ] Re-read current Agent action DTOs, `codex_desktop` candidate types and #31/#47/#101.
- [ ] Record whether the shared candidate core is extracted from Codex Desktop or implemented as a narrow adapter, including the rejected alternative.
- [ ] Update backend/frontend specs before or with the contract change; do not leave the old “request has no target” contract authoritative.

## 2. Backend domain

- [ ] Add closed scope/owner/package/evidence/eligibility enums.
- [ ] Add private candidate target capability and opaque snapshot/revision types.
- [ ] Add one inventory service/facade with bounded snapshot lifetime and exact Agent ID selector.
- [ ] Adapt current macOS/Windows observation into evidence producers without changing deployment behavior.
- [ ] Implement identity-based dedup, trusted count, conflict and ambiguity projection.
- [ ] Add fresh-install destination projection from closed product/platform policy.
- [ ] Add safe location-label normalization and controlled candidate reveal contract if included.

## 3. IPC and compatibility

- [ ] Add thin Tauri command(s) and ACL entries; commands only validate/translate/delegate.
- [ ] Add exact Rust/TypeScript DTO parsers and contract-version tests.
- [ ] Evolve `StartAgentActionRequest` with opaque target binding.
- [ ] Require target for update and ambiguous launch; preserve unique-candidate legacy launch behavior.
- [ ] Re-enumerate and compare revisions immediately before an action is admitted.

## 4. Frontend shared owner

- [ ] Add inventory FeaturePort/query key.
- [ ] Add a shared target-selection view model; no page-local winner logic.
- [ ] Add/extend a shared radio-style target picker with loading/error/refresh/disabled states.
- [ ] Integrate it into the Agent lifecycle surface and expose a stable API for Stage 2/3.
- [ ] Ensure target IDs/revisions do not enter URL, localStorage or sessionStorage.

## 5. Tests

- [ ] Exact wire keys/version/enum and forbidden-field scans.
- [ ] Zero, one, multiple, duplicate-evidence, stale, conflict and scan-failure fixtures.
- [ ] User/system/custom location-label redaction fixtures.
- [ ] Inventory expiry, revision drift, candidate disappearance and replay rejection.
- [ ] Unique legacy launch compatibility and ambiguous launch/update rejection.
- [ ] Picker keyboard/radio/disabled-reason tests and browser geometry at supported viewports.
- [ ] Codex Desktop candidate/restart/install regression suite.

## 6. Review gates

- [ ] No product adapter owns dedup/default selection.
- [ ] No renderer-controlled path or command crosses IPC.
- [ ] No candidate label is used as an identity.
- [ ] No shared type is made broadly public only to simplify tests.
- [ ] Same-domain defects found during testing are fixed with regression coverage or explicitly split.

## Validation

```bash
mise run typecheck:v2
mise run lint:v2
mise run test:v2
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:v2:browser
```

Also run focused architecture/ACL tests for Rust module ownership and V2 FeaturePort boundaries.

## Rollback point

Land the read-only inventory and parsers before admitting destructive target-bound actions. If target validation cannot prove the same candidate after reread, keep install/update disabled and retain the old read-only behavior.
