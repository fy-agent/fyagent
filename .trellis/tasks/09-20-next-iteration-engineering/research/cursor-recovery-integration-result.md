# Proxy recovery compatibility after native gate

Writer: Cursor desktop, existing local account/model.
Directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
Branch: `codex/next-iteration-engineering-20260920` (unchanged)
HEAD: `35848befda2f3690270612a999ff06ec600b442c` (unchanged)
No commit, branch switch, or push. Recovery implementation and specs were not changed.

## Diagnosis

Both full-gate failures are stale fixtures that still expected retired unsafe fallback, not a broken owned-proof path.

Reproduced:

```text
rtk proxy env -u CARGO_TARGET_DIR mise run rust:test \
  codex_set_takeover_rebuilds_stale_enabled_state_without_overwriting_backup
exit 101
rebuild Codex takeover: "无法确认代理配置仍可安全恢复…"

rtk proxy env -u CARGO_TARGET_DIR mise run rust:test \
  recover_from_crash_without_backup_cleans_placeholder_instead_of_writing_it_back
exit 101
recover from crash: same conflict
```

1. Codex stale rebuild seeded live with `write_codex_live_atomic` at an official DeepSeek URL plus `PROXY_MANAGED`, then asked `set_takeover(true)` to restore the DB backup and rewrite takeover. That live is not an owned loopback projection and has no restore receipt. #64 correctly refuses the overwrite.
2. Crash recovery seeded Claude placeholder bytes with `std::fs::write`, no backup, and a polluted SSOT that is itself the placeholder. Old assertion wanted manufactured cleanup by deleting the token/URL. Spec forbids inventing an original from an ownership failure.

Foreign live bytes and the Codex recovery backup stay. Owned success remains the existing receipt-backed `recovery_tests` (exact restore, verified original receipt, managed retry).

## Changes

- `src-tauri/src/services/proxy.rs`
  - `codex_stale_enabled_rebuild_preserves_unproven_live_and_backup`
  - Asserts conflict, byte-for-byte live auth/config, unchanged backup, `enabled=true`
  - Removed now-unused `running_codex_base_url`
- `src-tauri/tests/provider_service.rs`
  - `recover_from_crash_without_owned_proof_preserves_placeholder`
  - Asserts conflict and preserved placeholder URL/token

Did not edit credentials/ChangePlan, catalog ACL, frontend, installer, or proxy recovery implementation.

## Tests

```text
rtk proxy env -u CARGO_TARGET_DIR mise run rust:test \
  codex_stale_enabled_rebuild_preserves_unproven_live_and_backup
exit 0
lib: 1 passed

rtk proxy env -u CARGO_TARGET_DIR mise run rust:test \
  recover_from_crash_without_owned_proof_preserves_placeholder
exit 0
provider_service: 1 passed

rtk proxy env -u CARGO_TARGET_DIR mise run rust:test recovery_tests
exit 0
lib: 21 passed, 0 failed, 3529 filtered
```

Fixtures are not native UAT or Windows evidence. No full backend gate.

## Writer status

writer stopped
