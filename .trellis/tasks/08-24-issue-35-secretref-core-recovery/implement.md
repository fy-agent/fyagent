# Issue #35 SecretRef core recovery implementation plan

## Closure checklist

1. Add the minimal contract types, closed source-free errors, zeroizing material container, sealed backend port, and one-backend service facade.
2. Add an in-memory test backend and lock failure/no-fallback behavior with focused tests.
3. Add macOS Keychain and Windows Credential Manager leaves behind target cfgs, plus explicit opt-in native CRUD tests with cleanup guards.
4. Add DTO/canary/debug/reference property tests and a small scanner for new public fixtures.
5. After the integration owner releases the shared registration files, register only the private service module; do not add public Tauri commands in this slice.
6. Run formatting, focused tests, `cargo check --locked --all-targets`, project architecture gates, and `mise run check`; record host-specific evidence truthfully.
7. Commit and push the isolated branch, open a replacement PR with the salvage/deferred map, then mark PR #112 superseded only after the replacement PR URL exists.

## File ownership

Owned in this task:

- `src-tauri/src/services/secret/**`
- `src-tauri/tests/secret_service_contract.rs`
- target-specific Cargo feature additions required for Windows Credential Manager
- this Trellis task directory

Serial integration only:

- `src-tauri/src/services/mod.rs`

Not owned:

- `src-tauri/src/change_plan.rs`
- `src-tauri/src/commands/change_plan.rs`
- `src-tauri/src/database/**`
- `src-tauri/src/lib.rs`
- Provider writers and V2 pages

## Verification commands

- `mise run rust-fmt-check`
- `cd src-tauri && cargo test --locked --test secret_service_contract`
- `cd src-tauri && cargo check --locked --all-targets`
- `mise run test:architecture:rust`
- `mise run check`

Matching-host HIL uses `FYAGENT_NATIVE_SECRET_TEST=1` and is recorded separately for macOS and Windows; an ignored test or cross-check is not native CRUD evidence.
