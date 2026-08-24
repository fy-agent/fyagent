# Issue #35 SecretRef core recovery evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Source baseline and boundary

- Original external branch: `codex/issue-35-secretref-core-recovery`
- Replacement integration branch: `dev/secretref-core-integration`
- Original base: `origin/main@e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`
- Integration base includes canonical Change Plan/mainline governance through
  `6e2a065be713a7add8ddbf5fe1ee128972f533db`; the final branch will refresh
  against latest `main`/`dev/laiyongjie` before merge-ready handoff.
- Implementation commit: `6d1636f76abc392dcaeef87bcaa0b4330a8ad75b`
- Replacement Draft PR: <https://github.com/fy-agent/fyagent/pull/132>
- Old PR #112 was not rebased or cherry-picked.
- Original PR #132 is preserved as a merge parent in the replacement branch;
  its contributor commits are not flattened or anonymously copied.
- No Change Plan, database/schema, Provider writer, V2, public command, or
  `lib.rs` behavior is owned by this slice. Trial `services/mod.rs`
  registration exposed a broad dormant dead-code surface under warnings-denied
  Rust checks, so production registration remains deferred to the first real
  consumer instead of being hidden with lint allowances.

## Fresh local evidence

| Gate | Result | Evidence level |
| --- | --- | --- |
| `cargo test --locked --test secret_service_contract` | PASS: 6 passed, 1 ignored | focused contract/runtime mock |
| historical pre-DPK `FYAGENT_NATIVE_SECRET_TEST=1 ... native_os_backend_crud_readback` | PASS under the legacy/default file-based Keychain path | historical evidence only; invalid for the corrected DPK contract |
| corrected DPK plain-Cargo native probe | FAIL CLOSED: `SECRET_PERMISSION_DENIED`, raw OS status mapped from `errSecMissingEntitlement (-34018)` | proves the Cargo test host lacks authorized DPK entitlement; not product HIL |
| `cargo check --locked --all-targets` | PASS | current-host compile |
| `cargo clippy --locked --test secret_service_contract -- -D warnings` | PASS: no issues | focused lint |
| `mise run rust:fmt:check` | PASS | formatting |
| `mise run rust:clippy` | PASS | canonical workspace lint |
| `mise run rust:test` | PASS: full Rust unit/integration suite, including the focused contract test | canonical workspace test |
| `mise run supported-platform:check` | PASS: 2021 current files | platform governance |
| `mise run check:contracts` | PASS after creating the worktree-local locked `.venv` | task/docs/lock/version/release contracts |
| `mise run check` | PASS after replacing the concrete worktree path with the contract-safe `null` task field | complete current-host repository gate |

### Current replacement integration evidence

- `mise run rust:fmt:check` — PASS.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract` — PASS: 6 passed, 1 ignored native HIL.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract -- -D warnings` — PASS.
- `cargo check --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml` — PASS without registering a dormant production module or adding lint suppressions.
- `mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/ciWorkflow.test.ts` — PASS: 15/15.
- `mise run supported-platform:check` — PASS: 2114 current files after refreshing the reviewed structure manifest.
- `mise run check:contracts` — PASS: task/docs/lock/version/release contracts, 510 release contract tests with 1 skip, and 4/4 native-Fetch tests.
- `mise run check` — PASS on macOS current host: frontend 171 files / 1489 passed / 1 skipped; Rust main library 2847 passed / 5 ignored plus all integration/helper suites; release/contracts/native-Fetch all green.
- Corrected DPK plain-Cargo probe — FAIL CLOSED with `SECRET_PERMISSION_DENIED` mapped from `errSecMissingEntitlement (-34018)`; this is recorded as invalid-harness evidence, not a failed product HIL.

An additional `cargo check --locked --target x86_64-pc-windows-msvc --all-targets`
attempt reached the Windows dependency graph, then stopped in unrelated native
dependencies (`aws-lc-sys` and `zstd-sys`) because this macOS host has no MSVC
C SDK headers (`stdlib.h` / `string.h`). This is an environment blocker, not
Windows compile evidence for this slice; Windows Required CI remains mandatory.

The first PR Windows Backend run supplied authoritative compiler evidence: it
found a Windows-only local binding named `target` shadowing the target-name
helper in create-failure cleanup (`E0618`). Commit
`ea7af1f1367e37a7279629940af8a060cee05de5` renamed the binding to
`target_name`.

The second Windows Backend run compiled the Windows path and completed the test
suite, then Clippy rejected `CredWriteW(&mut credential, 0)` as
`unnecessary_mut_passed`. The follow-up uses an immutable `CREDENTIALW` value
and passes `&credential`; a final Required CI run at that fix SHA is still
required.

Current canonical toolchain is Rust 1.97.1 as frozen by `rust-toolchain.toml` and `mise.lock`.

## Not yet established

- Windows Credential Manager matching-host CRUD/readback/cleanup evidence has
  not yet run for the replacement branch. A new explicit Windows Backend CI
  step will own that proof.
- The final Windows Backend and aggregate Required CI run for the immutable
  `CredWriteW` fix is pending; earlier runs are failure evidence, not acceptance.
- PR CI evidence is recorded only after the final replacement-PR head finishes.
- Signed-app macOS Data Protection Keychain CRUD has not yet been established.
  Apple TN3137 requires DPK access groups to come from code-signing entitlements
  authorized by a provisioning profile; the current plain Cargo harness has no
  such app identity. The first production consumer must close this gap before
  activating SecretRef on macOS.
- Provider create/edit, device-local binding authority, lifecycle/journals, commands, V2 UI, and #63 integration are outside this narrow core slice.
- This evidence does not close Issue #35.

## Integration review findings to resolve

1. macOS source currently uses
   `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` without
   `kSecUseDataProtectionKeychain=true`; official Apple guidance requires the
   Data Protection Keychain (or synchronizable items) when using accessibility
   attributes on macOS. This slice chooses DPK + non-sync.
2. Windows `CredWriteW` is create-or-replace. The original read-before-write
   comment overclaimed external-race detection: a racing external writer can
   be overwritten before readback. The implementation and SPEC must state the
   real limitation and must not delete an unverified raced value.
3. `SecretVersion` rotates at the service boundary but is not persisted in the
   OS store. It is therefore a caller-side generation token, not an OS revision
   or standalone CAS mechanism; later binding integration owns authoritative
   stale-handle checks.
