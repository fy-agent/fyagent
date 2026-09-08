# Parallel implementation with the helper task

Date: 2026-08-31

Helper task `08-31-macos-privileged-application-commit-helper` is already
`in_progress`. This install-policy task now runs in parallel. File ownership
must not collide.

## Helper wave 1 exclusive files — do not touch

- `src-tauri/macos-privileged-helper/**`
- `src-tauri/src/macos_system_commit/**`
- `src-tauri/src/lib.rs`
- `src-tauri/src/agent_install/types.rs`
- `src-tauri/src/agent_install/macos.rs`
- `src-tauri/src/agent_install/inventory.rs`
- `src-tauri/src/agent_install/mod.rs` (reason mapping / dispatcher)
- `src/v2/shared/features/agent-install-readiness.ts`
- `src/v2/pages/agents/useAgentLifecycleAction.ts`
- `scripts/release/**`
- `.github/workflows/release.yml`
- `tests/releaseWorkflow.test.ts`
- Helper-owned fixtures:
  - `tests/v2/features/agent-install-readiness.test.ts`
  - `tests/v2/pages/agents/useAgentLifecycleAction.test.tsx`
  - `tests/v2/pages/agents/Page.test.tsx`
  - `tests/v2/pages/agents/AgentInstallReadinessSection.test.tsx`
  - `tests/v2/platform/agentInstallReadinessPort.test.ts`
  - `tests/v2-browser/agents-v3.spec.ts`
  - `tests/v2-browser/support/features.ts`
  - `tests/v2/pages/agents/useAgentDirectoryScan.test.ts`

## This task wave 1 may write

- Frontend directory metadata + pure order projection + AgentDirectory order
  commit lifecycle
- New `src-tauri/src/agent_install/lifecycle_policy.rs`
- New Claude source module and desktop product identity row
- Agent Catalog official links (Claude desktop-only, OpenCode product+desktop)
- OpenCode GitHub latest-version reuse/extract without touching helper files

## Still gated

- Production `/Applications` enablement and signed helper HIL
- Wiring `MacSystemCommitPort` into Claude/OpenCode system commits happens
  after the helper port exists; until then keep `authorization_required`
- `action_not_supported` wire enum lives in `types.rs`; add it in wave 2 after
  the helper reason-code bump lands, or extend the same enum once that file is
  free
