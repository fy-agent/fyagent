# Implement

1. Add `services/model_probe.rs` recovered from `a5903d86^` (Claude/Codex/OpenAI-chat only). Make `StreamCheckService::validate_probe_url` `pub(crate)`.
2. Add IPC `stream_check_model`; register in `lib.rs` and `legacy-application-commands.toml`; bump the handler-count assertion.
3. Add V2 `checkModel` on providers / workbuddy / opencodeModels ports; parse at the Tauri adapter; browser preview stays native-only.
4. Add `ModelConnectivityTest` and mount it on WorkBuddy, ProviderPanel, OpenCode. Hide the button when `modelIds` is empty. Enable 「拉取模型」 for Codex and Grok Build.
5. Tests: Rust URL/protocol/error-body; component picker/search/group/error display; page tests that the button appears after models exist and is absent on Qoder/TRAE; port invoke shape.
6. Update `.trellis/spec/frontend/v2-agent-models.md` and the cross-layer probe bullet.

## Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

## Rollback

Revert the commit. `stream_check_url` remains the URL probe.
