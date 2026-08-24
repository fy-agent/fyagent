# Issue #35 SecretRef core recovery evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Source baseline and boundary

- Original external branch: `codex/issue-35-secretref-core-recovery`
- Final integration branch: `dev/secretref-core-final`
- Original base: `origin/main@e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`
- CI-topology PR #144 entered `main` through Merge Queue as
  `b296ed9e8a851c871805a69d0dfc50ee8964cd95`. Its `merge_group` run
  `32713452472` completed successfully, while final-main and `dev/laiyongjie`
  pushes emitted only the lightweight `Commit Convention / Push` workflow.
- Final SecretRef branch merged that exact main SHA with the conventional merge
  commit `chore(secrets): sync final main baseline` before final local gates.
- Implementation commit: `6d1636f76abc392dcaeef87bcaa0b4330a8ad75b`
- Historical source Draft PR #132 is closed unmerged. Earlier replacement Draft
  #143 remains historical integration evidence and is superseded by final
  replacement PR #145: <https://github.com/fy-agent/fyagent/pull/145>.
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

### Earlier replacement integration evidence (#143)

- `mise run rust:fmt:check` — PASS.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract` — PASS: 7 passed, 1 ignored native HIL after adding the cross-platform native-leaf source guard.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract -- -D warnings` — PASS.
- `cargo check --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml` — PASS without registering a dormant production module or adding lint suppressions.
- `mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/ciWorkflow.test.ts` — PASS: 15/15.
- `mise run supported-platform:check` — PASS: 2114 current files after refreshing the reviewed structure manifest.
- `mise run check:contracts` — PASS: task/docs/lock/version/release contracts, 510 release contract tests with 1 skip, and 4/4 native-Fetch tests.
- `mise run check` — PASS on macOS current host: frontend 171 files / 1489 passed / 1 skipped; Rust main library 2847 passed / 5 ignored plus all integration/helper suites; release/contracts/native-Fetch all green.
- Corrected DPK plain-Cargo probe — FAIL CLOSED with `SECRET_PERMISSION_DENIED` mapped from `errSecMissingEntitlement (-34018)`; this is recorded as invalid-harness evidence, not a failed product HIL.

### Final clean integration branch evidence

- `chore(secrets): preserve PR 132 ancestry` is an explicit two-parent merge
  retaining all four original #132 contributor commits while using a valid,
  <=72-character Conventional Commit subject.
- Full range from final `main@b296ed9e...` through the SecretRef integration
  head passed `verify-commit-messages`; the complete ancestry path also retains
  all four original PR #132 contributor commits through a two-parent merge.
- `mise run rust:fmt:check` — PASS.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract` — PASS: 7 passed, 1 ignored native HIL.
- focused SecretRef Clippy with `-D warnings` — PASS.
- `mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/ciWorkflow.test.ts tests/githubWorkflowTriggers.test.ts` — PASS: 22/22.
- `mise run supported-platform:check` — PASS: 2125 current files.
- Final-main `mise run check` — PASS: frontend 171 files / 1491 passed / 1
  skipped; Rust main library 2847 passed / 5 ignored plus integration/helper
  suites; SecretRef focused contract 7 passed / 1 ignored native HIL;
  task/docs/platform/release/native-fetch contracts all green.
- Final-main focused gate — PASS: rustfmt, SecretRef contract, focused Clippy
  with `-D warnings`, CI/module-boundary tests 22/22, and supported-platform
  surface 2125 current files.
- Final-main direct-session `check:prearchive --exclude-active-task
  .trellis/tasks/08-24-issue-35-secretref-core-recovery` — PASS with exit 0.
  The composite reran frontend 1491 passed / 1 skipped, Rust 2847 passed / 5
  ignored plus integration/helper suites, SecretRef 7 passed / 1 ignored native
  HIL, task/docs/platform/release contracts, and native-fetch 4/4.
- Canonical post-archive `mise run check:contracts` — PASS with exit 0 after
  moving the completed task into `archive/2026-08/`; task validation, archive
  scanner, supported-platform surface (2133 current files), release contracts,
  512 contract tests with 1 skip, and native-fetch 4/4 all remained green.
- Final replacement PR created as #145 from `dev/secretref-core-final`; the
  archived task metadata is bound to that PR before exact-head auto-merge
  handoff.

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

- Windows Credential Manager matching-host CRUD/readback/cleanup is established
  on hosted Windows run `32709228999`, job `Backend Checks (Windows)`:
  `Exercise Credential Manager SecretRef CRUD` ran with
  `FYAGENT_NATIVE_SECRET_TEST=1` and reported exactly `1 passed; 0 failed; 6 filtered`.
  The final replacement head must rerun this after rebasing/merging the corrected
  CI topology; the old run proves native behavior, not final Required closeout.
- The final aggregate Required CI run for the replacement head remains pending.
- PR CI evidence is recorded only after the final replacement-PR head finishes.
- Signed-app macOS Data Protection Keychain CRUD has not yet been established.
  Apple TN3137 requires DPK access groups to come from code-signing entitlements
  authorized by a provisioning profile; the current plain Cargo harness has no
  such app identity. The first production consumer must close this gap before
  activating SecretRef on macOS.
- Provider create/edit, device-local binding authority, lifecycle/journals, commands, V2 UI, and #63 integration are outside this narrow core slice.
- This evidence does not close Issue #35.

## Integration review findings resolved in this slice

1. macOS now sets `kSecUseDataProtectionKeychain=true` together with
   `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` and non-synchronizing items.
   The unsigned/plain-Cargo harness fails closed with `errSecMissingEntitlement`;
   signed-app DPK activation remains a later production-consumer gate.
2. Windows `CredWriteW` is documented and implemented as create-or-replace at
   the OS boundary. The original external-race overclaim was removed, and a
   failed verification no longer blindly deletes an unverified raced value.
3. macOS create verification now follows the same ownership rule: `SecItemAdd`
   remains atomic create-only, but a later readback failure/mismatch does not
   authorize deleting a value that may have been updated by another entitled
   process between add and verification.
4. `SecretVersion` is explicitly documented as a caller-side generation token,
   not an OS revision or standalone CAS mechanism; later binding integration
   owns authoritative stale-handle checks.
5. The original native leaves stored their mutex inside each backend instance
   while comments/SPEC described process-local serialization. Both macOS and
   Windows now use one process-global static mutex, so multiple backend/service
   instances cannot silently bypass FyAgent's own native-store serialization.
