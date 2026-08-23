# Recover Issue 35 SecretRef core

## Goal

Rebuild the minimal SecretRef and pluggable local SecretBackend contract from current main without importing PR 112 history or touching UCP v20 ownership.

## Requirements

- Generate `SecretRef` as `sec_` plus lowercase UUIDv4 simple hex. It must not encode an owner, backend, account, provider, or value-derived digest.
- Keep secret material native-only. Public metadata and error DTOs expose reference, backend kind, presence, availability, revision, and source-free recovery guidance; they never expose the value, a value-derived digest, a backend locator, or a raw OS error.
- Provide a small private `SecretBackend` boundary with create-only, replace, read-with-callback, probe, and delete operations. A missing/locked/denied/unavailable result must fail closed with no fallback backend.
- Implement the current-device software backend on macOS Keychain and Windows Credential Manager. Records are device-local and non-synchronizing; create must never silently upsert.
- Zeroize material buffers on drop and prevent `Clone`, `Serialize`, `Deserialize`, `Display`, and value-bearing `Debug` implementations.
- Add a deterministic in-memory backend only for contract/failure tests. It must not be selected by production construction.
- Do not add or change SQLite schema/version, Provider writer paths, ChangePlan files, network behavior, V2 UI, hardware backends, journals, migration, or `lib.rs` registration while the UCP integration owner holds those files.
- Treat the frozen #35 D2 documents at `a338ee18` as behavioral input only. Do not rebase or cherry-pick PR #112.

## Acceptance Criteria

- [x] Reference parsing rejects malformed, uppercase, derived-looking, and caller-invented identities at authority creation boundaries; generated refs match `^sec_[0-9a-f]{32}$` and are unique in a large sample.
- [x] Serialized public DTO fixtures contain no material or forbidden semantic fields and use camelCase.
- [x] Secret material is observable only inside a sealed callback and is zeroized after callback/drop; redacted debug output contains no canary.
- [x] In-memory backend tests cover create/read/replace/probe/delete plus duplicate create, missing, locked, denied, unavailable, and zero-fallback behavior.
- [x] macOS native CRUD/readback test passes when explicitly enabled on a matching host and leaves no keychain item behind.
- [ ] Windows native CRUD/readback test passes when explicitly enabled on a matching host and leaves no Credential Manager record behind.
- [x] `cargo check --locked --all-targets` and focused Rust tests pass on the local host; Windows Backend Required CI must pass before merge.
- [x] A secret canary scan finds zero occurrences in serialized DTOs, Debug/Display output, errors, logs produced by focused tests, and repository fixtures added by this PR.
- [x] PR description records a salvage map from #112 behavior/test intent to the new narrow files and explicitly lists deferred lifecycle/integration/HIL evidence.
- [ ] Issue #35 remains OPEN until create/edit Provider integration, full lifecycle AC, both native-host HIL records, exact merged SHA, and Required CI are complete.

## Notes

- This task is the consumable core slice required by #63 create/edit. It is not the whole historical #112 implementation and must not be described as closing #35 by itself.
