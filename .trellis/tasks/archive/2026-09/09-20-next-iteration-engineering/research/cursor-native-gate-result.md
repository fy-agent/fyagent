# Cursor native-gate result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned).
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- HEAD unchanged: `35848befda2f3690270612a999ff06ec600b442c` plus this package's dirty edits. No reset/stash/branch/worktree/commit/push.
- Writer stopped.

## Changed files

- `~/.../fyagent/src-tauri/src/services/provider/credentials.rs` — `UsageTestInput` + `admit_usage_test` now takes that typed request.
- `~/.../fyagent/src-tauri/src/services/provider/credentials/tests.rs` — same admission assertions, new input shape.
- `~/.../fyagent/src-tauri/src/services/provider/usage.rs` — production Codex admit call and `run_test_with` use `UsageTestInput`.
- `~/.../fyagent/artifacts/cursor-native-gate.log` — full canonical `check:backend` transcript (4076 lines).
- `~/.../fyagent/.trellis/tasks/09-20-next-iteration-engineering/research/cursor-native-gate-result.md`

No lint `allow`, no public `test_usage_script` signature change, no proxy/ACL/change-plan/manifest writes. Prior main-integration test edits were left in place. Temporary Debug blocks were not added for this gate.

## Clippy repair

`artifacts/integration-backend-main.log` stopped on `clippy::too_many_arguments` for `admit_usage_test` (9/7) and test helper `run_test_with` (8/7). Related usage-test fields are now `UsageTestInput { script_code, api_key, base_url, access_token, user_id, template_type }`. Fresh/mask/target-binding behavior is unchanged: caller still goes through `admit_usage_test` then `admit_test`.

## Canonical check

```
CARGO_TARGET_DIR=~/.../fyagent/src-tauri/target
rtk proxy mise run check:backend
```

Log: `~/.../fyagent/artifacts/cursor-native-gate.log`

| Step             | Result                                                                                                                                                         |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rust:fmt:check` | passed                                                                                                                                                         |
| `rust:check`     | passed (after fixing test-module path `super::super::credentials::UsageTestInput`)                                                                             |
| `rust:clippy`    | passed; no `too_many_arguments` in this log                                                                                                                    |
| `rust:test`      | failed, cargo exit 101. `-p fyagent --lib` 3538 passed / 7 failed / 5 ignored; `-p fyagent --test provider_service` 35 passed / 1 failed. Other crates passed. |

The shell pipeline to `tee` returned 0; mise/`rust:test` itself failed.

## Remaining failures (not plaintext public-row compares)

None of these are “public Codex DTO == plaintext input”. Per the grant, they were not rewritten here.

### Owned credentials tests (not caused by the typed input)

| Test                                                                           | Evidence                                                                                                                            |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `blank_legacy_edit_does_not_erase_the_only_copy_when_migration_is_locked`      | `credentials/tests.rs:1188` `save_provider` after unlock → `provider_secret_invalid`. Does not call `admit_usage_test`.             |
| `actual_usage_query_materializes_auxiliary_fields_before_script_validation`    | `futures::executor::block_on(query_usage)` → `there is no reactor running, must be called from the context of a Tokio 1.x runtime`. |
| `common_config_keeps_legal_settings_but_redirect_requires_fresh_authorization` | live `config.toml` after write missing `notifications = false`.                                                                     |

### Outside usage/credentials — reported, not expanded

| Test                                                                              | Evidence                                                                                                                                                                                                                     | Owner hint                    |
| --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| `application_acl_covers_every_registered_command_without_remote_access`           | registered command set missing ACL config-pack names (`apply_config_pack_import`, `list_config_pack_candidates`, `pick_config_pack_file`, `preview_config_pack_*`, `save_config_pack_export`, `cancel_config_pack_preview`). | root frontend/manifest        |
| `credential_capability_accepts_only_extractable_unmanaged_target_material`        | `change_plan/service.rs:2419` `save_provider(inactive_token)` → `provider_secret_invalid` (inactive `experimental_bearer_token` extra secret).                                                                               | change-plan + SecretRef save  |
| `upsert_plan_is_side_effect_free_and_apply_writes_once`                           | apply job `Some(Failed)` vs `Some(Succeeded)` at `change_plan/service.rs:3565`.                                                                                                                                              | change-plan apply/quick-setup |
| `codex_set_takeover_rebuilds_stale_enabled_state_without_overwriting_backup`      | `proxy.rs:5285` rebuild: 无法确认代理配置仍可安全恢复…                                                                                                                                                                       | #64 proxy/DAO                 |
| `recover_from_crash_without_backup_cleans_placeholder_instead_of_writing_it_back` | `tests/provider_service.rs:3150` same restore-safety message.                                                                                                                                                                | #64 proxy/recovery            |

## Residuals

- Fixture/Memory checks do not prove OS keychain HIL, Windows UAT, or release.
- Root still owns acceptance of the remaining 8 failures and any commit.
- No frontend/browser run.
