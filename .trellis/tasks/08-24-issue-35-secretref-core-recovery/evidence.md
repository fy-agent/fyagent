# Issue #35 SecretRef core recovery evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Source baseline and boundary

- Branch: `codex/issue-35-secretref-core-recovery`
- Base: `origin/main@e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`
- Implementation commit: `6d1636f76abc392dcaeef87bcaa0b4330a8ad75b`
- Replacement Draft PR: <https://github.com/fy-agent/fyagent/pull/132>
- Old PR #112 was not rebased or cherry-picked.
- No ChangePlan, database/schema, Provider writer, V2, command registration, or `lib.rs` file is modified by this branch.
- `services/mod.rs` registration remains a serial integration handoff while the UCP owner holds the shared file.

## Fresh local evidence

| Gate | Result | Evidence level |
| --- | --- | --- |
| `cargo test --locked --test secret_service_contract` | PASS: 6 passed, 1 ignored | focused contract/runtime mock |
| `FYAGENT_NATIVE_SECRET_TEST=1 cargo test --locked --test secret_service_contract native_os_backend_crud_readback -- --ignored --nocapture` | PASS: 1 passed, 6 filtered; create/readback/replace/readback/delete/missing-readback | matching-host macOS Keychain HIL |
| `cargo check --locked --all-targets` | PASS | current-host compile |
| `cargo clippy --locked --test secret_service_contract -- -D warnings` | PASS: no issues | focused lint |
| `mise run rust:fmt:check` | PASS | formatting |
| `mise run rust:clippy` | PASS | canonical workspace lint |
| `mise run rust:test` | PASS: full Rust unit/integration suite, including the focused contract test | canonical workspace test |
| `mise run supported-platform:check` | PASS: 2021 current files | platform governance |
| `mise run check:contracts` | PASS after creating the worktree-local locked `.venv` | task/docs/lock/version/release contracts |
| `mise run check` | PASS after replacing the concrete worktree path with the contract-safe `null` task field | complete current-host repository gate |

An additional `cargo check --locked --target x86_64-pc-windows-msvc --all-targets`
attempt reached the Windows dependency graph, then stopped in unrelated native
dependencies (`aws-lc-sys` and `zstd-sys`) because this macOS host has no MSVC
C SDK headers (`stdlib.h` / `string.h`). This is an environment blocker, not
Windows compile evidence for this slice; Windows Required CI remains mandatory.

The first PR Windows Backend run then supplied authoritative compiler evidence:
it found a Windows-only local binding named `target` shadowing the target-name
helper in create-failure cleanup (`E0618`). The binding is renamed to
`target_name`; a new Required CI run is required at the fix SHA.

Current canonical toolchain is Rust 1.97.1 as frozen by `rust-toolchain.toml` and `mise.lock`.

## Not yet established

- Windows Credential Manager matching-host CRUD/readback/cleanup HIL has not run on this Mac.
- Windows Backend Required CI has not run until the branch is pushed and a replacement PR exists.
- PR CI evidence is recorded only after the replacement PR checks finish.
- Provider create/edit, device-local binding authority, lifecycle/journals, commands, V2 UI, and #63 integration are outside this narrow core slice.
- This evidence does not close Issue #35.
