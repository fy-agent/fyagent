# Current Auth gap review

## Required full-contract reads

Before implementation, read the complete `.trellis/spec/backend/external-agent-p0.md` and `.trellis/spec/frontend/v2-agent-models.md`. They are intentionally omitted from automatic JSONL injection because each exceeds the configured context-file size limit.

## Current backend

`src-tauri/src/agent_install/auth_actions.rs` currently:

- launches `claude auth login`;
- runs `claude auth logout`;
- launches `grok login` / `grok logout`;
- launches `opencode /connect`;
- maps desktop Auth login to application launch.

`src-tauri/src/agent_install/mod.rs` then returns an immediate `AgentActionJobStage::Succeeded` after the launch/run function returns. For interactive login this proves only that FyAgent initiated a terminal/application handoff.

Only Claude has a current status observer. OpenCode is projected as `provider_connection_required`; Grok and desktop agents remain unknown. The current `AgentAuthState` is a single enum and cannot represent OpenCode's multiple provider connections.

## Current frontend

`AgentInstallReadinessSection` renders generic actions named `登录`, `退出登录` and `连接 Provider`. It consumes the same lifecycle action hook used for install/update, so an immediate backend success can produce a misleading success message.

## Existing owners to reuse

- Tooling owns closed CLI discovery/execution and terminal launch.
- Windows runtime owns the frozen interactive user.
- Auth Center and existing OAuth command/services own Codex/FyAgent-managed login.
- FeaturePorts/strict parsers own V2 backend access.

Conclusion: add one Auth-specific coordinator and adapter layer; do not extend the install job until it becomes a generic and semantically correct interactive-action engine.
