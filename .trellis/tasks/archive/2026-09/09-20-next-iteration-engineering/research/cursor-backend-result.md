# Cursor backend repair result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned). Not independently re-verified against a different product/account.
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- HEAD unchanged: `07519b377af57e6613050941995918804d0f2af5` plus shared dirty tree. No reset/stash/branch/worktree/commit/push.
- Writer stopped after the directed rework. Did not touch `#64` proxy/DAO paths.

## Changed files (this package only)

- `~/.../fyagent/src-tauri/src/services/provider/usage.rs`
- `~/.../fyagent/src-tauri/src/services/provider/credentials.rs`
- `~/.../fyagent/src-tauri/src/services/provider/credentials/material.rs`
- `~/.../fyagent/src-tauri/src/services/provider/credentials/tests.rs`
- `~/.../fyagent/src-tauri/src/commands/provider/live_summary.rs`
- `~/.../fyagent/src-tauri/src/commands/provider/live_summary/tests.rs`
- `~/.../fyagent/.trellis/spec/backend/provider-credentials.md`
- `~/.../fyagent/.trellis/tasks/09-20-next-iteration-engineering/research/cursor-backend-result.md`

All `#region agent log` blocks, hardcoded debug paths, and temporary reproduce writers were removed after the passing fixture run. No production log records caller URLs, templates, or secrets.

## Rework

### 1. Codex route precedence

`codex_connection` no longer does `extract_codex_base_url(route_text).or_else(profile base_url)`, which let a root `base_url` beat the selected profile. Selected profile `model` / `model_provider` / `base_url` / `wire_api` are folded into a temporary root view; the formal extractor then sees active provider, then that view. Precedence is **active provider > selected profile > file root**. Inactive profiles stay unread.

### 2. Strict route field types

`require_string_field` checks `base_url` and `wire_api` at the active provider, selected profile, and file root **before** extraction. Wrong types (`42`, arrays) are `unreadable`, not silently skipped by `as_str()`.

### 3. NewAPI token-only test admission

`CredentialMaterial::admit_test` no longer takes unused `fresh_*` parameters. After a UsageTarget match it admits a non-empty usage `api_key` **or** `access_token`; inference-key fallback remains only when neither is present. Same-target token-only tests retain the saved token; a changed target still rejects; a fresh token does not inherit a saved API key. Caller-side fresh credentials stay in `admit_usage_test` only.

## Focused checks

Canonical runner: `mise run rust:test` with `CARGO_TARGET_DIR` set to `~/.../fyagent/src-tauri/target` so the native runner stays inside the verified host target directory. Mid-run `#64` syntax / move errors in `services/proxy/managed_recovery.rs` and `recovery_tests.rs` briefly blocked compile; those files were not edited here. Retried after the other writer restored a compiling tree.

| Command | Exit |
| --- | --- |
| `rtk mise exec -- rustfmt --edition 2021 --config skip_children=true -- <owned rust files>` | 0 |
| `rtk git diff --check -- <owned files>` | 0 |
| `rtk mise run rust:test -- usage_script_test` | 0 (5 passed, including token-only same-target / mask / changed-target reject / fresh token) |
| `rtk mise run rust:test -- usage_test_admission` | 0 (2 passed: original target-binding + token-only retain/mask/reject) |
| `rtk mise run rust:test -- fresh_usage_token` | 0 (1 passed: fresh token does not inherit saved API key) |
| `rtk mise run rust:test -- top_level_and_selected` | 0 (1 passed: temp files for top-level route, base_url-only, provider>root, profile>root, provider>profile>root) |
| `rtk mise run rust:test -- wrong_route_field_types` | 0 (1 passed: root / selected profile / active provider `base_url=42` or array → unreadable) |
| `rtk mise run rust:test -- explicit_profile_and_unspecified` | 0 (1 passed: in-memory profile>root and provider>profile>root) |

No global `check:backend`, no real credentials, no user-network calls, no live app instance.

## Residuals

- Fixture/Memory backend checks do not prove native OS keychain HIL, Windows UAT, release, or production Codex process adoption.
- Root still owns serial integration, `check:backend`, and any commit.
- Non-Codex `test_usage_script` still uses the existing resolve-then-fallback path; SecretRef admission is the Codex owner.
- `#64` proxy/DAO remains another writer's scope.
