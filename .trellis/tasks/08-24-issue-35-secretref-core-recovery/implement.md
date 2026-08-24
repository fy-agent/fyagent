# Issue #35 SecretRef core recovery implementation plan

## Closure checklist

1. Add the minimal contract types, closed source-free errors, zeroizing material container, sealed backend port, and one-backend service facade.
2. Add an in-memory test backend and lock failure/no-fallback behavior with focused tests.
3. Harden macOS Keychain with explicit Data Protection Keychain selection on
   every operation and rerun the native CRUD test.
4. Keep Windows Credential Manager behind target cfg, document its real upsert
   semantics, remove unsafe verify-failure cleanup, and add a matching-host
   Windows Backend CI step that proves exactly one native CRUD test ran.
5. Keep the core out of `services/mod.rs` until its first production consumer;
   do not weaken warnings-denied lint with dormant-module allowances.
6. Add/update the durable SecretBackend and CI code-specs, refresh supported
   platform digests from final source, and retain DTO/canary/reference tests.
7. Run formatting and focused tests. Re-run the plain current-host macOS DPK
   probe to confirm its authorization boundary; record `errSecMissingEntitlement`
   as an invalid-harness result rather than product acceptance. Signed-app DPK
   HIL moves to the first production consumer. Then run
   `cargo check --locked --all-targets`, architecture/CI contracts, and full
   repository checks; record evidence truthfully.
8. Push the replacement branch to obtain Windows native Credential Manager CI
   evidence. Only after that evidence is green, finish prearchive/archive and
   exact-head Merge Queue handoff.
9. Create the replacement PR and close original PR #132 unmerged with a
   migration comment only after the replacement URL exists. Issue #35 remains
   open for Provider integration and remaining lifecycle acceptance.

## File ownership

Owned in this task:

- `src-tauri/src/services/secret/**`
- `src-tauri/tests/secret_service_contract.rs`
- target-specific Cargo feature additions required for Windows Credential Manager
- this Trellis task directory

Shared integration in this task:

- `src-tauri/src/services/mod.rs` — intentionally unchanged until first production consumer
- `.github/workflows/ci.yml` + `tests/ciWorkflow.test.ts` — Windows native CRUD evidence
- `.trellis/spec/backend/secretref-backend.md` + CI/index cross-links

Not owned:

- `src-tauri/src/change_plan.rs`
- `src-tauri/src/commands/change_plan.rs`
- `src-tauri/src/database/**`
- `src-tauri/src/lib.rs`
- Provider writers and V2 pages

## Verification commands

- `mise run rust:fmt:check`
- `cd src-tauri && cargo test --locked --test secret_service_contract`
- `cd src-tauri && cargo check --locked --all-targets`
- `mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/ciWorkflow.test.ts`
- `mise run check`

Matching-host HIL uses `FYAGENT_NATIVE_SECRET_TEST=1`. Windows is exercised by
native hosted CI in this slice. macOS DPK HIL additionally requires an
authorized app-like signed host; a plain Cargo test binary is not acceptance.
