# Session identity hardening

## Delivered and released for integration

Only `src-tauri/src/session_manager/migrate/identity.rs` and the new private sibling `identity_platform.rs` were changed by this work package. File ownership is released after this report; no other production/test files, Cargo manifest, dependency or lockfile changed. The staging copy and integration notes remain under `work/identity-staging` outside the repository.

- `install_identity()` combines a persisted random v4 seed with OS machine/user identity, canonical state-directory location and filesystem instance identity. Copied configuration at the same pathname on another machine/user cannot adopt prior receipt bindings.
- `source_store_namespace()` serializes install/source identity read-modify-write under a persistent OS lock and keys random namespaces by the machine-bound installation identity. Invalid JSON, duplicate namespace keys, unknown formats, invalid UUIDs or missing installation seed beside existing namespace state fail closed.
- `store_instance_id(provider_id, path)` is available to the native provider owners. It binds canonical location plus device/inode/creation time on Unix or volume/file-index/creation time on Windows. Normal appends preserve identity; replacement at the same pathname changes it.
- `stable_random_origin_id()` keeps the extractor's existing `store|path` input contract but includes the actual source-file instance. Ambiguous/unavailable paths fail closed; Unix paths containing a pipe are covered.
- RAII locks use existing macOS libc flock and Windows LockFileEx/UnlockFileEx; Linux uses the same narrow libc ABI without adding a crate. This preserves the declared Rust 1.85 MSRV. The lock file is never replaced/deleted; JSON commits reuse config::atomic_write.
- Thread-local test overrides inject both state location and synthetic machine identity. Nested panics restore both through Drop, with no test fallback to actual HOME or OS identity.

## Actual validation

1. Only the two owned Rust files were formatted using the locked mise rustfmt; no complete-crate formatting ran.
2. `rtk proxy mise run rust:test identity`: PASS, 84 matching tests total, no failures. The 19 identity module tests execute in both the library and existing integration harness. Meaningful additions include identical copied seed on a different simulated machine/user, copied/moved installation directories, corruption and duplicate-key rejection, replacement versus append, 12 simultaneous threads and six actual independent child processes preserving all namespaces.
3. Local read-only macOS probe of the exact fixed ioreg command: success, exactly one nonzero UUID and bounded output, euid readable. Only booleans were recorded; the actual machine identifier was never printed.
4. Test compile produced one integration-harness warning outside this work package: the former `config::get_app_config_dir()` stub at `src-tauri/tests/session_migration_model.rs:53` is now unused. The coordinator was notified to have its owner remove the dead stub before final Clippy `-D warnings`. No warnings were emitted from these two product modules.

## Remaining integration and limits

Native OpenCode/Hermes/Codex writer owners must consume `store_instance_id`; this package does not edit their files. The whole integrated tree still needs the final standard checks.

Windows API signatures and the existing frozen Explorer-user context were checked against current source; Windows/Linux runtime and Windows UAT were not run. OS identity or file creation-time unavailability gives an explicit failure. A complete VM clone preserving OS machine/user identity and filesystem instance metadata cannot be physically distinguished; no such guarantee is claimed.

The previous unreleased unversioned identity-state JSON is rejected, not silently reset or reassigned. Recovering such development metadata requires an explicit migration-metadata reset. Content digest, snapshot and slot formats are unchanged.
