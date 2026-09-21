# Cursor main-integration result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned).
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- HEAD unchanged: `35848befda2f3690270612a999ff06ec600b442c` plus this package's dirty edits. No reset/stash/branch/worktree/commit/push.
- Writer stopped.

## Changed files

- `~/.../fyagent/src-tauri/src/commands/provider.rs` — only `kimi_quick_setup_uses_native_responses_and_preserves_saved_providers` plus a local public-readback helper.
- `~/.../fyagent/src-tauri/src/services/provider/universal.rs` — only `compat_config_universal_*` tests plus a local settings helper.
- `~/.../fyagent/.trellis/tasks/09-20-next-iteration-engineering/research/cursor-main-integration-result.md`

No production sync/save/resolve path was changed. Temporary `#region agent log` blocks were removed after the passing run.

## Reproduction

Canonical `mise run rust:test` with `CARGO_TARGET_DIR=~/.../fyagent/src-tauri/target`.

| Filter                    | Before         | Evidence                                                                                                              |
| ------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------- |
| `kimi_quick_setup`        | fail, exit 101 | public `auth: {}` vs input `OPENAI_API_KEY=new-kimi-key`                                                              |
| `compat_config_universal` | fail, exit 101 | init: Codex child JSON vs plaintext projection; preserve: `codex/auth` `{}` vs `{OPENAI_API_KEY: shared-fixture-key}` |

Claude/Gemini children and Kimi route construction were not the failing surface. Save/sync themselves succeeded; the old fixtures compared public SecretRef rows to plaintext input.

## Integration

Public Codex readback now asserts an opaque `pc_*` `credentialRef`, empty `auth`, and no fixture keys in serialized DTO. Native material is checked only through `ProviderCredentials::resolve`. Original product checks remain:

- Kimi / Kimi For Coding `wire_api=responses`, custom provider route, `requires_openai_auth`, no Chat meta on the new setup.
- Existing saved Kimi config, model catalog, and Chat routing/user overrides stay on the old card.
- Universal children keep `created_at` / `sort_index` / app-owned meta / `childOnly` / current selection; Claude/Gemini still compare projected settings; Codex config and non-secret fields still match.

Did not copy plaintext back into public DTOs or weaken SecretRef ownership to satisfy the old equality.

## Focused checks (after integration)

| Command                                             | Exit         |
| --------------------------------------------------- | ------------ |
| `rtk mise exec -- rustfmt -- <owned rust files>`    | 0            |
| `rtk git diff --check -- <owned files>`             | 0            |
| `rtk mise run rust:test -- kimi_quick_setup`        | 0 (1 passed) |
| `rtk mise run rust:test -- compat_config_universal` | 0 (2 passed) |

Post-fix logs (then removed): Kimi and Kimi For Coding `publicHasRef=true`, `publicHasPlainKey=false`; Codex universal children the same.

No full backend/native suite. No real keys or network.

## Residuals

- Fixture/Memory backend checks do not prove OS keychain HIL, Windows UAT, or release.
- Root still owns frontend/manifest integration and any commit.
- Universal production merge still layers projected settings onto the existing public row before the SecretRef save facade; that path did not fail closed in these fixtures, so it was left unchanged.
