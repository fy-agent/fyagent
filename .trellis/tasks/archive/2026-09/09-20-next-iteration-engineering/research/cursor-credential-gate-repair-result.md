# Cursor credential/ChangePlan gate-repair result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned).
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- HEAD at delivery: `a0a170b2` plus this package's dirty edits. No reset/stash/branch/worktree/commit/push.
- Writer stopped.

## Changed files

- `~/.../fyagent/src-tauri/src/services/provider/credentials/tests.rs` — blank-legacy model-only edit; usage query uses a Tokio current-thread runtime; HTTPS-reject fixture keeps `token_plan`.
- `~/.../fyagent/src-tauri/src/services/provider/live.rs` — after source-switch write, merge unowned common-config tables (including `[tui]`) into live.
- `~/.../fyagent/src-tauri/src/services/change_plan/service.rs` — inactive bearer uses raw `save_provider_record`; classify accepts first SecretRef persist by comparing the unbound definition.
- `~/.../fyagent/src-tauri/src/services/change_plan/adapter.rs` — upsert `verify` reads the persisted row, same path as switch.
- `~/.../fyagent/.trellis/tasks/09-20-next-iteration-engineering/research/cursor-credential-gate-repair-result.md`

Prior typed `UsageTestInput` edits in `credentials.rs` / `usage.rs` were left in place. No installer/helper/frontend/proxy/recovery/ACL writes. Temporary `#region agent log` blocks were removed after the passing post-fix run.

## Failures and fixes

| #   | Test                                                                           | Cause                                                                                                                                                                                                                                                                                                                     | Fix                                                                                                                                                                                                                                                |
| --- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `blank_legacy_edit_does_not_erase_the_only_copy_when_migration_is_locked`      | Fixture replaced the whole config with `model = 'changed-model'`, changing InferenceTarget. Production correctly rejected blank retain across a route change.                                                                                                                                                             | Edit only `model = 'fixture-model'` → `model = 'changed-model'`.                                                                                                                                                                                   |
| 2   | `actual_usage_query_materializes_auxiliary_fields_before_script_validation`    | `query_usage` needs Tokio; `futures::executor::block_on` has no reactor. A `custom` template also skips HTTPS and would send HTTP.                                                                                                                                                                                        | Current-thread Tokio runtime; keep `token_plan` so URL validation rejects HTTP before I/O.                                                                                                                                                         |
| 3   | `common_config_keeps_legal_settings_but_redirect_requires_fresh_authorization` | `patch_source` copies a closed key list and drops `[tui]`. Real legal-config gap.                                                                                                                                                                                                                                         | `persist_unowned_codex_common_tables` after the ordinary live snapshot. Redirect still requires fresh authorization.                                                                                                                               |
| 4   | `credential_capability_accepts_only_extractable_unmanaged_target_material`     | Inactive bearer is correctly rejected by the save facade. The test needs legacy raw data so fail-closed happens at plan extraction.                                                                                                                                                                                       | `db.save_provider_record` for the inactive fixture only. Active/managed/unknown still go through `save_provider`.                                                                                                                                  |
| 5   | `upsert_plan_is_side_effect_free_and_apply_writes_once`                        | Runtime: verify re-inspected the pre-write draft. After save, `native_credential_draft.source_ref` no longer matches the ready `pc_*` binding → `provider_secret_revoked` → classify `readback_unavailable`. Then the persisted row's definition digest includes `credentialRef` while the plan hashed the unbound draft. | Upsert `verify` uses `inspect_codex_switch` on the saved id. `definition_matches_after_first_secretref` strips readback `credentialRef` and compares the unbound digest. Rotation still mismatches because the stored digest includes the old ref. |

### Failure 5 evidence (session `f6f5be`)

- Before persist-readback: `H5f` `target live projection failed` `error=provider_secret_revoked` `hasAuthKey=true`; then `H5c` `readback unavailable` for `fyagent-v2-quick-setup-codex`.
- After persist-readback: `H5h` `found=true` `hasRef=true`; `H5` `exact=false` `stripped=true` `hadRef=true` `matched=true`; `H5b` `dbTarget/deviceTarget/definitionTarget/liveTarget=true`. Job Succeeded. `credential_rotation_after_plan_is_stale_before_writer` stayed green.

Security/legal semantics kept: active-target-only extraction, secret-free public DTOs, native resolver, target-bound reauthorization, no plaintext fallback, legal `[tui]` preserved, redirect still needs fresh capture.

## Focused tests (after removing Debug instrumentation)

```
CARGO_TARGET_DIR=~/.../fyagent/src-tauri/target
rtk mise run rust:test -- <filter>
```

| Filter                                   | Result    | cargo/mise exit |
| ---------------------------------------- | --------- | --------------- |
| `blank_legacy_edit`                      | 1 passed  | 0               |
| `actual_usage_query_materializes`        | 1 passed  | 0               |
| `common_config_keeps_legal`              | 1 passed  | 0               |
| `credential_capability_accepts_only`     | 1 passed  | 0               |
| `upsert_plan_is_side_effect_free`        | 1 passed  | 0               |
| `credential_rotation_after_plan`         | 1 passed  | 0               |
| `services::provider::credentials::tests` | 35 passed | 0               |

No concurrent full `check:backend`. Root runs the integrated gate after the recovery package stops.

## Residuals

- Fixture/Memory checks do not prove OS keychain HIL, Windows UAT, or release.
- Two proxy/`provider_service` recoveries remain with the other conversation.
- Catalog ACL was already fixed by root inventory only; not touched here.
- No commit/push/Issue closure.
