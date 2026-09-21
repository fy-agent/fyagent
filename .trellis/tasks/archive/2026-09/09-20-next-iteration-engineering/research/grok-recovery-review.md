# Independent terminal Grok review

Executor: Grok CLI 1.0.34, grok.com existing login, grok-4.6, read-only plan mode.
Root must assess and verify findings; this review does not execute or prove fixes.

Issue #64’s preview → confirm → restore → readback path does not survive the crash/retry states the feature is supposed to handle. Native can produce a preview the renderer treats as invalid, and disable deletes recovery evidence before the enabled flag is cleared.

## Findings

### 1. High — already-restored legacy preview is renderer-illegal, so exit cannot be confirmed

**Where**
- `src-tauri/src/services/proxy/legacy_recovery.rs:139-144` (`Unchanged` → empty path list)
- `src-tauri/src/services/proxy/legacy_recovery.rs:171-175` (live already equals backup → `Unchanged`)
- `src-tauri/src/services/proxy/restore_preview.rs:47-50` (any successful plan sets `canRestore: true` and uses that path list)
- `src/shared/platform/tauri/feature-ports/managedSubscriptions.ts:40-42` (rejects `canRestore` with zero targets)
- `src/pages/models/ProviderSubscriptionRestore.tsx:137-139` (recheck parse/throw → `onUnconfirmed`)

**Trigger**
Proxy remains `enabled=true` while the live file already matches the DB backup. That is the documented retry/no-op case (`legacy_exit_restores_exact_bytes_and_retry_is_a_noop`) and the crash window after files are restored but before `enabled` is cleared.

Native then returns `{ enabled: true, canRestore: true, targets: [] }`.

**Consequence**
The port throws `"Proxy restore preview is unavailable"`. The exit button never becomes a valid confirmation. Recheck takes the `catch` path, shows “未能确认配置已完整恢复”, and blocks further writes for that target. Native `set_takeover_for_app(false)` would have treated this as a no-op restore and cleared the flag.

Managed exit does not hit this: it always discloses the closed path list. Legacy API-key Claude/Codex/Grok does.

**Fix**
Keep `Unchanged` as a no-op write plan, but preview the same native paths used for `Exact`/`Source` (`path` + `exists`). Never emit `canRestore: true` with an empty `targets` array. Add a native fixture that restores bytes, leaves `enabled=true`, and asserts a renderer-admissible preview; the port test that rejects empty `canRestore` should stay.

### 2. High — backup is deleted before `enabled=false`, then the UI cannot finish disable

**Where**
- `src-tauri/src/services/proxy.rs:1336-1355` (`restore` → `delete_live_backup` → then `enabled=false`)
- `src-tauri/src/services/proxy/restore_preview.rs:42-50` (enabled but missing/unverified backup → `canRestore: false`, empty targets)
- `src/pages/models/ProviderSubscriptionRestore.tsx:130-132`, `202`, `215-220` (retry/exit only if `canRestore`)

**Trigger**
`set_proxy_takeover_for_app({ enabled: false })` restores files, deletes `proxy_live_backup`, then crashes or fails the following `update_proxy_config_for_app`. `DELETE` of a missing row is success (`database/dao/proxy.rs:875-885`), so a later native retry would still be able to clear the flag.

**Consequence**
Preview is `{ enabled: true, canRestore: false, targets: [] }`. Recheck sets `retryReady=false` and never calls restore. The user is stuck with takeover still enabled, original backup gone, and no confirmation path. This is exactly the interrupted-exit case Issue #64 asked to make retryable.

**Fix**
Persist `enabled=false` before deleting the backup, or do both in one DB transaction. Only drop the backup after the disabled row exists. Until then, preview must remain retryable (`canRestore: true` with the real paths, including already-restored files). Do not require the renderer to call restore when `canRestore` is false; that would violate the confirmation contract.

### 3. Medium — v0.4.5 upgrade writes a new recovery record before ownership recheck

**Where**
- `src-tauri/src/services/proxy/managed_recovery.rs:271-278` (`save_live_backup_sync` of `fyagentManagedRestore`)
- `src-tauri/src/services/proxy/managed_recovery.rs:232-237` (`verify_proof` runs after that write)

Preview itself is clean: `managed_restore_preview_paths` uses `compute_legacy_managed_restore_proof` and does not write (`241-258`; covered by `legacy_managed_restore_preview_does_not_upgrade_database_proof`).

**Trigger**
First exit/bind verify of a pre-MARKER logical backup. `legacy_managed_restore_proof` computes a proof, replaces the DB backup, then `verify_proof` rereads the live files. Claude/Codex/Grok can rewrite the live file in that window; FyAgent’s switch lock does not cover those writers.

**Consequence**
The original logical backup is gone. The new proof’s hashes no longer match live bytes, so later preview is `canRestore: false` with no upgrade path (`decode` now succeeds and never recomputes). That contradicts proxy-runtime: reject unexplained changes without replacing recovery records.

**Fix**
Call `verify_proof` before `save_live_backup_sync`. Persist the upgraded proof only after the live hashes still match, preferably after the owned restore commits. If the recheck fails, leave the logical backup untouched.

## Acceptance gaps

The “16 native recovery tests + 137 frontend tests” claims do not cover the contracts above.

- `recovery_tests.rs` has 14 `#[tokio::test]` functions. None assert the Unchanged/already-restored preview shape, and none run `get_restore_preview` after a disable that restored files but left `enabled=true`.
- `providerProxyRestorePort.test.ts` only rejects `canRestore: true` with `targets: []`. There is no fixture where native produces that value and the UI still completes disable.
- No test drives a v0.4.5 logical backup through `get_restore_preview` then `set_takeover_for_app(false)` (the upgrade test stops at `verify_and_unwrap_managed_restore_for_exit`).
- No test covers backup-deleted / `enabled` still true, or disable crash ordering.
- Conflict previews blank `targets` (`restore_preview.rs:47-50` only fills them on success), so `ProviderSubscriptionRestore.tsx:239-246` cannot show the files the copy tells the user to protect.
- `tests/renderer/pages/models/Page.test.tsx` never exercises `recoveryDisabled` / `onBeginRecovery` / `onRecoveryConfirmed`. The omit-`writesBlocked` wiring is untested at the page that owns it.

Checked and not reported: preview does not upgrade or mutate files; renderer never submits paths; Codex `auth.json` is not a preview/restore target; secret fields are stripped from the preview DTO; `onBeginRecovery` is independent of the ordinary write block; late unmount still calls `onUnconfirmed`; #35 and #47 were not re-audited. Not verified: full `mise` suite, Windows runtime, live accounts.
