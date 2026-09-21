# Cursor #64 recovery repair result

Writer: Cursor desktop, existing local account/model.
Directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
Branch: `codex/next-iteration-engineering-20260920` (unchanged)
HEAD: `07519b377af57e6613050941995918804d0f2af5` (unchanged)
No commit, branch switch, or push.

Read Grok `research/grok-recovery-review.md` and Jev `atomic_db_finish`. Implemented the four accepted repairs only.

## Change boundary

- Gap: already-restored legacy preview was renderer-illegal; disable deleted the backup before clearing `enabled`; v0.4.5 upgrade replaced the logical backup before `verify_proof`; conflict previews blanked closed paths.
- Owners: proxy recovery plan/preview, managed unwrap, and the DAO finish transaction.
- Not done: provider/usage/live_summary, frontend, installer, Page wiring, enable-path backup deletes, full suite, Windows/native UAT.

## Changes

### 1. Unchanged preview discloses closed native targets
`src-tauri/src/services/proxy/legacy_recovery.rs`
- `LegacyRestorePlan::Unchanged(Vec<PathBuf>)` returns the same closed list as Exact/Source.
- Restore remains a no-op write.

### 2. Atomic disable + backup delete
`src-tauri/src/database/dao/proxy.rs`
- `complete_app_restore(app_type)`: one SQL transaction updates only `proxy_config.enabled = 0` then deletes that app's `proxy_live_backup`. Other parameters stay. SQL-only; native I/O is outside the mutex.

`src-tauri/src/services/proxy.rs`
- `set_takeover_for_app(false)` and `disable_takeover_for_app_sync` restore files first, then call `complete_app_restore`. Per-app lock unchanged.

### 3. Verify before replacing a v0.4.5 logical backup
`src-tauri/src/services/proxy/managed_recovery.rs`
- Admission/`verify_and_unwrap*` compute and `verify_proof` in memory; they no longer `save_live_backup_sync`.
- `restore_verified_managed_config` uses the verified in-memory proof, not a stale re-decode of the original raw value.
- After `verify_proof` succeeds and before any restore write, persist that complete proof (original preimages + owned hashes). File restore is then idempotent. `mark_managed_restore_proof` updates hashes only after the owned restore finishes. A failed last ownership check leaves the logical backup.

### 4. Conflict preview keeps closed path metadata
`src-tauri/src/services/proxy/restore_preview.rs`
- Successful plans still set `canRestore=true` with plan paths.
- If a live backup exists but ownership fails, disclose closed `path`/`exists` only. `canRestore` stays false.
- Missing backup still has empty targets. No file contents, auth, backup paths, or caller-supplied paths.

### Spec
`.trellis/spec/backend/proxy-runtime.md` — preview disclosure, atomic finish, and verify-before-replace.

## Tests

Canonical commands (official `src-tauri/target`; `CARGO_TARGET_DIR` unset because the sandbox cache is rejected by the native runner):

```text
rtk proxy env -u CARGO_TARGET_DIR mise run rust:test recovery_tests
exit 0
lib: 21 passed, 0 failed, 3480 filtered; 1.67s

rtk proxy env -u CARGO_TARGET_DIR mise run rust:test complete_app_restore
exit 0
lib: 2 passed, 0 failed, 3498 filtered; 0.06s
```

New / updated names:

- `legacy_unchanged_preview_discloses_closed_targets_then_disable_succeeds`
- `complete_app_restore_sql_failure_keeps_retryable_preview`
- `complete_app_restore_rolls_back_when_second_statement_fails`
- `legacy_managed_preview_then_exit_restores_logical_backup`
- `legacy_managed_external_edit_preserves_logical_backup_until_owned_exit`
- `legacy_managed_first_file_interrupt_keeps_retryable_preview`
- `legacy_managed_restore_preview_does_not_upgrade_database_proof` now asserts verify does not persist
- conflict/stale-selection previews now assert closed targets with `canRestore=false`

SQLite `TEMP TRIGGER` on `BEFORE DELETE proxy_live_backup` is the second-statement failure seam. After rollback: `enabled=true`, backup present, other parameters unchanged, restored bytes still preview-admissible. Retry completes disabled + backup removed.

These fixtures are not native user acceptance or Windows evidence.

## Residuals

- Enable/rebuild `delete_live_backup` in `set_takeover_for_app(true)` compensation is outside this finish contract.
- Renderer empty-`canRestore` rejection and Page `recoveryDisabled` / `onBeginRecovery` wiring stay with their owners.
- Codex two-file interrupt after an already-MARKER proof remains `managed_exit_retries_partial_files_but_rebinding_stays_strict`. The new logical-backup first-file window is covered on Claude; persist-before-write is the shared restore owner.
- No full `mise` suite, live accounts, or Windows runtime.

## Writer status

writer stopped
