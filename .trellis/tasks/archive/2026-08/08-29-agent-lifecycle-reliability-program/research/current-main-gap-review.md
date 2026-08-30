# Current-main gap review

## Baseline

- Planning branch: `dev/laiyongjie`.
- Synced baseline: `4e47aab51b272f819da31773b348a2ea0ed8dee2`.
- The branch had no unique commits before sync and was fast-forwarded from `94c294e0`.

## Required full-contract reads

The following authoritative SPEC files exceed Trellis automatic context injection limits and are therefore intentionally not listed as injected JSONL context. The executor/reviewer must read each complete file directly before implementation or final review:

- `.trellis/spec/backend/external-agent-p0.md`
- `.trellis/spec/frontend/v2-agent-models.md`
- `.trellis/spec/frontend/v2-shell.md`

## Backend findings

- `src-tauri/src/agent_install/types.rs` exposes only `agentId + action + expectedReleaseId`; no installed candidate can be selected or revision-bound.
- `src-tauri/src/agent_install/desktop.rs` combines source resolution, product identity, macOS deployment, Windows discovery, launch and version parsing. It is approximately 1,168 lines; `agent_install/mod.rs` is approximately 736 lines and owns readiness/action/job orchestration.
- macOS `install_from_mount` always uses `get_home_dir()/Applications`, so an application observed in `/Applications` can be copied into a different user-scope location during update.
- Windows EXE is explicitly rejected by `windows_exe_unavailable`; current tests assert that Windows EXE installation is not started.
- Windows discovery scans a few fixed roots and closed relative EXE paths, then reads product/version strings from bounded PE byte windows. It does not aggregate Uninstall registry, App Paths, MSIX/PackageManager or custom installer locations.
- The desktop job re-observes TRAE after copy, but QoderWork and WorkBuddy can reach `succeeded` without the same authoritative target/version verification.
- Auth launch commands return an immediate successful action result even when the user has not completed the interactive login.

## Existing reuse candidates

`src-tauri/src/codex_desktop/platform.rs` already defines crate-private trusted installation candidates, scope, candidate inspection, prepared package capability and platform install planning. The new design should first determine whether those concepts can become a product-neutral private owner without weakening Codex-specific invariants.

The Codex Desktop macOS adapter already has staging/rollback/permission tests. Generic Agent installation should reuse the transaction principles; update semantics must be stricter than fresh-install fallback because an update may not silently change scope.

## Frontend findings

- `PersistentPrimaryOutlet` statically imports all six routes and permanently mounts every visited page.
- `ModelsPage` repeats a visited-target keep-alive and updates session/visited state during render.
- `SelectionLens` recursively observes a subtree with ResizeObserver and MutationObserver; selected navigation/tab hosts are otherwise transparent, so delayed measurement can remove the only visible selected surface.
- `FeatureTabs` hand-writes tab roles/click behavior despite `@radix-ui/react-tabs` already being installed.
- `ToolCluster` exposes Search, Settings and Account buttons whose handlers are `noop`.
- Agent Skills/MCP assignment uses one global pending ID; other switches remain visually enabled but their clicks are discarded.
- Production build reported an approximately 855.76 KB main JS chunk and a Vite large-chunk warning.
- V2 unit/browser tests pass, but current unit output contains repeated React `act(...)` warnings, which reduces confidence in interaction timing assertions.

## Planning conclusion

The five stages require contract and ownership changes, not isolated path/CSS patches. Backend work should separate inventory, execution and verification. Frontend work should make semantic state independent from animation, use adopted primitives, and replace blanket keep-alive with explicit state ownership.
