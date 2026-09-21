# Latest-main SecretRef regression integration

Work only in `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`, branch
`codex/next-iteration-engineering-20260920`. Root just merged verified
`origin/main` `da91427e` after checkpoint `9fd852cd`. Read actual HEAD/status.
The earlier provider and recovery writers are stopped. You are not alone: root
owns final integration and Antigravity owns a separate CLI worktree.

Own only `src-tauri/src/commands/provider.rs` test
`kimi_quick_setup_uses_native_responses_and_preserves_saved_providers` and
`src-tauri/src/services/provider/universal.rs` tests plus the narrow production
sync path only if an actual regression proves it necessary. Read the relevant
provider-credentials and Codex configuration specs. Preserve all earlier edits.

The new upstream tests compare plaintext input providers directly with DB
readback. The new Codex save facade stores SecretRef and redacts public readback.
Reproduce the exact tests (`kimi_quick_setup`, `compat_config_universal`) using
canonical `mise run rust:test`; use the verified native target directory and
`rtk` terminal prefix. Integrate the tests with the native-only credential
resolver and explicit redaction assertions, preserving their original checks:
Kimi Responses route and old user overrides; universal child metadata,
created_at/sort_index, unknown config fields and current selection.

Do not weaken production SecretRef ownership or copy plaintext back into public
DTOs just to satisfy the old fixtures. If universal sync exposes a real bug,
implement the smallest owner-preserving repair and a meaningful regression.
No network, real keys, global configuration changes, commits, branch changes,
push, or Issue updates. Remove temporary Debug instrumentation before final
checks. Do not run the full backend suite; root will do that once writers stop.

Write `research/cursor-main-integration-result.md` with actual changes, exact
focused results and any remaining failure. Stop writer explicitly when done.
