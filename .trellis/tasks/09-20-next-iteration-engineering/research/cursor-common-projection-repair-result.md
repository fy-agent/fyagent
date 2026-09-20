# Cursor common-projection repair result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned).
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- HEAD at delivery: `3df9c8df` plus this package's dirty edits. No reset/stash/branch/worktree/commit/push.
- Writer stopped.

## Changed files

- `~/.../fyagent/src-tauri/src/services/provider/live.rs` — removed `persist_unowned_codex_common_tables`. Codex non-quick-setup writes pass only an explicitly enabled common snippet into the existing source writer.
- `~/.../fyagent/src-tauri/src/codex_config.rs` — `write_codex_live_projection` is one read / in-memory `project_source` / one atomic write. Public wrappers delegate; they do not add a second publication.
- `~/.../fyagent/src-tauri/src/codex_config/source_switch.rs` — `project_source` = `patch_source` then sibling-merge only enabled snippet keys. Invalid current reads reject. Missing file stays empty.
- `~/.../fyagent/src-tauri/src/services/provider/mod.rs` — switch preview uses the same `project_codex_source_config` + enabled snippet as the writer.
- `~/.../fyagent/src-tauri/src/services/provider/credentials/tests.rs` — sibling `[tui]` + MCP preservation; disabled/stale tables stay out; invalid live rejects without empty replacement; preview matches write.
- `~/.../fyagent/src-tauri/src/services/change_plan/adapter.rs` — upsert `verify` reads the saved row on success; `TargetNotFound` only falls back to draft inspect. Other inspect errors stay fail-closed. Existing reserved-row precheck rejects a rotated `credentialRef` as `Stale` before the writer.
- `~/.../fyagent/src-tauri/src/services/change_plan/service.rs` — failed-create/no-insert baseline-restored regression; upsert retained-binding rotation-before-mutation check.
- `~/.../fyagent/.trellis/spec/backend/codex-source-selection.md` — enabled snippet may overlay only its keys on the same projected document; preview/writer/readback share `project_source`.
- `~/.../fyagent/.trellis/tasks/09-20-next-iteration-engineering/research/cursor-common-projection-repair-result.md`

No installer/helper/frontend/proxy/recovery/ACL writes. Temporary `#region agent log` blocks were removed after the passing post-fix run.

## Grok findings 1–3 — superseded

`research/grok-final-credentials-review.md` reviewed the old `persist_unowned_codex_common_tables` delta, not this replacement. That helper is gone (no remaining symbol).

| Finding  | Old defect                                                                                 | Replacement                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| 1 High   | post-snapshot `read_to_string(...).unwrap_or_default()` can wipe the just-written snapshot | I/O of an existing file is an error via `read_and_validate_codex_config_text`; there is no second read after commit |
| 2 High   | two atomic writes / partial commit                                                         | one in-memory `project_source`, one `write_codex_live_config_atomic`                                                |
| 3 Medium | whole-table replace of every unowned desired key                                           | `merge_common_table` overlays only enabled snippet keys; siblings and other live tables stay                        |

Focused coverage: live sibling `[tui]` + MCP survive an enabled snippet; disabled/absent snippet does not import stale Provider tables; invalid current config rejects; preview digest matches write.

## Finding 4 — failed upsert-create verify

Confirmed on the previous persisted-row-only `verify`: create preview + writer `Err` with no reserved row → `inspect_codex_switch` → `TargetNotFound` → `classify_job` `ReadbackUnavailable` / `RecoveryRequired`. Compensation needs a successful inspect plus unchanged baseline current/live + matching unbound definition.

Narrow repair: `inspect_codex_switch` when the row exists; on `TargetNotFound` only, `inspect_codex_intended_provider(self.provider)`. `SecretDependencyUnavailable` / `Internal` and other inspect errors are not remapped.

### Evidence (session `f6f5be`)

Failed create, no insert:

- `adapter.rs:verify` `branch=draft_fallback` after `TargetNotFound`
- `classify_job` flags: `writerOk=false`, `dbTarget=false`, `deviceTarget=false`, `definitionTarget=true`, `liveTarget=false`, `baselineDb=true`, `baselineDevice=true`, `baselineLive=true`
- Job: `WriterFailedBaselineRestored`, `recovery_state=Succeeded`, writer calls `== 1`, reserved row absent, current/live unchanged
- No `classify readback error` / `ReadbackUnavailable` log

Successful upsert (existing test):

- `adapter.rs:verify` `branch=saved_row`
- flags: `writerOk=true` and all target matches true

Retained-binding rotation (test gap, not a confirmed prior bug):

- `adapter.rs:precheck` `hasCurrentRef=true` `hasPlannedRef=true` `refsMatch=false` → `Stale`, writer calls `== 0`

Grok Q2 speculative items 1, 3–6 were not expanded.

## Focused tests (after removing Debug instrumentation)

```
CARGO_TARGET_DIR=~/.../fyagent/src-tauri/target
rtk mise run rust:test -- <filter>
```

`mise rust:test` accepts one filter. Sequential runs, all cargo/mise exit 0:

| Filter                                                  | Result                               |
| ------------------------------------------------------- | ------------------------------------ |
| `upsert_failed_create_without_insert`                   | 1 passed                             |
| `upsert_retained_binding_rotation`                      | 1 passed                             |
| `upsert_plan_is_side_effect_free_and_apply_writes_once` | 1 passed                             |
| `credential_rotation_after_plan`                        | 1 passed                             |
| `common_config`                                         | 21 lib + 8 `provider_service` passed |
| `disabled_common_config`                                | 1 passed                             |
| `invalid_live_config`                                   | 1 passed                             |
| `source_switch`                                         | 7 lib + 1 `provider_service` passed  |
| `services::provider::credentials::tests`                | 38 passed                            |

No concurrent full `check:backend`. Root runs the integrated gate after the recovery package stops.

## Residuals

- `write_codex_live_for_provider` is a public wrapper over `write_codex_live_projection` and currently unused inside the lib crate (`dead_code` warning). Not part of this grant.
- Fixture/Memory checks do not prove OS keychain HIL, Windows UAT, or release.
- Two proxy/`provider_service` recoveries remain with the other conversation.
- Catalog ACL was already fixed by root inventory only; not touched here.
- No commit/push/Issue closure.

## Root integration review

GPT-6 reviewed the actual one-read / shared-projection / one-atomic-write path and TargetNotFound-only failed-create fallback. The existing `merge_edit` always snapshots the prior source reference, including fresh-key updates, so the new precheck does not inherently reject those updates. No temporary Debug instrumentation remains in the changed production paths. The now-unused wrapper was removed and its internal documentation updated; the canonical backend gate is running against the resulting source. Other recovery repairs have already been integrated in `2ad28ef8`; the older residual in the worker report is superseded.
