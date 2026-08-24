# Recover Issue 35 SecretRef core

## Goal

Integrate the minimal SecretRef and local SecretBackend core from external PR
#132 onto the current canonical mainline, harden the native OS-store boundaries,
and make the slice independently merge-ready without absorbing Provider/UCP UI
work that belongs to later verticals.

## Requirements

- Generate `SecretRef` as `sec_` plus lowercase UUIDv4 simple hex. It must not encode an owner, backend, account, provider, or value-derived digest.
- Keep secret material native-only. Public metadata and error DTOs expose reference, backend kind, presence, availability, revision, and source-free recovery guidance; they never expose the value, a value-derived digest, a backend locator, or a raw OS error.
- Provide a small private `SecretBackend` boundary with create-only, replace, read-with-callback, probe, and delete operations. A missing/locked/denied/unavailable result must fail closed with no fallback backend.
- Implement the current-device software backend on macOS Keychain and Windows Credential Manager. Records are device-local and non-synchronizing.
- On macOS, all item operations must explicitly select the Data Protection Keychain while keeping synchronizable false; `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` must never be used against the legacy file-based keychain by accident.
- On Windows, document the real platform contract: `CredWriteW` is create-or-replace. FyAgent's `create_new` performs an in-process read-before-write guard and uses an internally generated random unexposed `SecretRef`, but it must not claim OS-atomic create-only semantics against arbitrary external writers.
- A failed post-write verification must never delete a credential whose current value cannot be proven to be the value just written by FyAgent.
- Zeroize material buffers on drop and prevent `Clone`, `Serialize`, `Deserialize`, `Display`, and value-bearing `Debug` implementations.
- Add a deterministic in-memory backend only for contract/failure tests. It must not be selected by production construction.
- Keep the standalone core unregistered until the first production consumer. Integration tests and native HIL compile it now; do not add broad dead-code/unused-import allowances merely to pre-register a dormant module. Do not add public Tauri commands, AppState wiring, SQLite schema/version, Provider writer paths, Change Plan mutations, network behavior, V2 UI, hardware backends, or migration in this slice.
- Because fail-closed create verification no longer performs an unproven compensating delete, the first production consumer must retain enough durable admission/recovery authority to reconcile a create that reached the native store but whose readback outcome is unknown. This dormant core slice does not claim that lifecycle is complete.
- Treat the frozen #35 D2 documents at `a338ee18` as behavioral input only. Do not rebase or cherry-pick PR #112.
- Preserve the contributor ancestry from PR #132 in the replacement integration branch; the external PR itself will be closed unmerged only after the replacement PR exists.
- Add a maintained SecretBackend code-spec and native Windows CI evidence so the security contract is executable and durable rather than living only in the recovery task.
- `SecretVersion` is an opaque caller-side generation token returned by core create/replace; it is not an OS-store revision and does not itself provide CAS. Any later persistent binding owner must compare its authoritative stored generation before calling replace/delete.

## Acceptance Criteria

- [x] Reference parsing rejects malformed, uppercase, derived-looking, and caller-invented identities at authority creation boundaries; generated refs match `^sec_[0-9a-f]{32}$` and are unique in a large sample.
- [x] Serialized public DTO fixtures contain no material or forbidden semantic fields and use camelCase.
- [x] Secret material is observable only inside a sealed callback and is zeroized after callback/drop; redacted debug output contains no canary.
- [x] In-memory backend tests cover create/read/replace/probe/delete plus duplicate create, missing, locked, denied, unavailable, and zero-fallback behavior.
- [x] Historical pre-DPK macOS CRUD evidence is retained only as legacy file-based Keychain evidence and is explicitly not treated as acceptance for the corrected DPK contract.
- [x] The former plain-`cargo test` macOS Keychain HIL is invalidated after DPK correction: it now fails closed with `errSecMissingEntitlement`, as expected for an unsigned/non-provisioned test host. This result is recorded and is not misreported as DPK acceptance.
- [x] Signed-app macOS DPK CRUD/readback/cleanup is explicitly deferred to the first production consumer, and the maintained contract blocks activation until FyAgent's host identity/provisioning profile authorizes the access group.
- [x] Windows matching-host Credential Manager CRUD/readback/cleanup runs as an explicit native CI step and proves exactly one ignored integration test executed successfully; final replacement-head Required CI must rerun this after the CI-topology update.
- [x] Production Rust lint remains warnings-denied without dormant SecretRef lint suppressions; `services/mod.rs` registration is deferred to the first real consumer.
- [x] `cargo check --locked --all-targets`, Clippy, focused Rust tests, architecture/CI contract tests, and full repository gates pass on the integrated current-main branch.
- [x] A secret canary scan finds zero occurrences in serialized DTOs, Debug/Display output, errors, logs produced by focused tests, and repository fixtures added by this PR.
- [x] Maintained `.trellis/spec/backend/secretref-backend.md` records signatures, platform semantics, validation/error behavior, native evidence, and the Windows non-atomic-create limitation.
- [ ] The old PR #132 provenance/salvage intent is retained, but its active Trellis task is updated, validated, and archived on the replacement branch before merge-ready handoff.
- [x] Issue #35 remains OPEN until create/edit Provider integration, signed-app macOS DPK HIL, Windows native HIL, full lifecycle AC, exact merged SHA, and Required CI are complete.

## Notes

- This task is the consumable core slice required by #63 create/edit. It is not the whole historical #112 implementation and must not be described as closing #35 by itself.
- Replacement integration branch: `dev/secretref-core-integration`; original PR #132 head is preserved as a merge parent rather than copied as anonymous source.
