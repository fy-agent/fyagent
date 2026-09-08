# Implement — Windows vendor installer handoff

## Checklist

1. [x] `user-helper` `run_verified_exe_installer`: return after successful `ShellExecute`; drop the wait/exit-code loop; treat a missing handle as launched.
2. [x] Shrink `WindowsHelperDeadlines::AGENT_EXE_INSTALL.operation` to a UAC/launch bound.
3. [x] `run_windows_desktop_install_job`: on helper `Ok`, `Succeeded`; never call `wait_for_windows_deployment`. Extract settle mapping + unit tests.
4. [x] Remove unused `wait_for_windows_deployment` if nothing else calls it.
5. [x] Update `.trellis/spec/backend/external-agent-p0.md`, `codex-desktop-installer.md`, and `windows-runtime-security.md` Windows job/error matrix and Tests Required.
6. [x] Update `.trellis/spec/frontend/v2-agent-models.md` so Windows success is vendor-wizard handoff, not installed proof.
7. [x] Frontend copy only if a stage string would now lie; keep `launching_installer` / `awaiting_user`.
8. [x] Run focused tests: user-helper, `agent_install` jobs/mod, lifecycle copy tests.

## Validation

```bash
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml -p fyagent-user-helper --offline
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --offline agent_install
mise exec -- pnpm exec vitest run tests/v2/pages/agents/useAgentLifecycleAction.test.tsx tests/v2/pages/agents/Page.test.tsx
```

If the crate names differ, use the repository's existing `mise run` targets that cover user-helper and agent_install.

## Rollback

Revert the helper wait change first if launched installers disappear when the bridge is deleted; then restore parent verification only if handoff settlement is also wrong.
