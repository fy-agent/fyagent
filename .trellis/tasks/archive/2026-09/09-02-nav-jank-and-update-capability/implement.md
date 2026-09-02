# Implement

## Checklist

1. **Backend policy** — `lifecycle_policy.rs`: Qoder/TRAE/WorkBuddy `update: false`; fix `expected_policy` and `managed_desktop_products_admit_*` (assert update unsupported). Confirm `desktop_source_resolve` skips installed remote lookup when `update` is false.
2. **Frontend capability module** — `src/v2/shared/features/agent-lifecycle-capabilities.ts`: exhaustive `AGENT_DIRECTORY_UPDATE_UI`; helpers `directoryUpdateUi(id)`, `canOfferDirectoryUpdate(id, readiness)`. Unit test: three domestic IDs are `none`; others not `none` except as specified; `canOfferDirectoryUpdate` is false for `none` even if `allowedActions` contains `update`.
3. **Shared slot** — extract `AgentLifecycleActionSlot` from `AgentDirectory.tsx`; Generic + Codex consume it. `deriveAgentLifecyclePrimaryAction` takes agentId (or callers AND `canOfferDirectoryUpdate`).
4. **Directory tests** — move 「一键更新」positive case off WorkBuddy onto Grok/Claude/OpenCode; add negative cases for the three domestic products.
5. **Keep-alive outlet** — rewrite `PersistentPrimaryOutlet` per design; router child leaves have no second page element; AppShell prefetches; architecture test: allow PersistentSurface, forbid render-phase useState/visited setter, still require dynamic `import("../pages/...")`.
6. **Visibility gating** — `queries.ts` AND `usePersistentVisibility`; scan hook `active` flag; AgentsPage passes it. Tests: hidden surface does not call readiness get again.
7. **Lens** — device-pixel box; remove no-deps syncBox effect; CSS clip + no backdrop-filter on primary-nav lens; unit + browser assertions.
8. **Validation** — `mise run lint:v2 typecheck:v2 test:v2 test:v2:browser` then `mise run check` / prearchive exclude.

## Validation commands

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
cargo test --manifest-path src-tauri/Cargo.toml lifecycle_policy -- --nocapture
mise run check
```

Prearchive: `mise run check -- --exclude-active-task .trellis/tasks/09-02-nav-jank-and-update-capability` (confirm exact flag from task-runner spec if different).

## Risky files / rollback

- `src/v2/app/PersistentPrimaryOutlet.tsx` + `router.tsx` — double-mount if child routes still render pages.
- `src/v2/shared/ui/SelectionLens.tsx` — do not break catalog/feature-tab size-and-position mode.
- `src-tauri/src/agent_install/lifecycle_policy.rs` — Windows/macOS inventory tests that assumed domestic update.

If keep-alive regresses hidden queries, set `enabled: false` is the rollback for data; outlet can return to `<Outlet />` without touching policy.

## Follow-up before start

- JSONL curated with shell, reuse, agent-models, external-agent-p0, state-management, this design.
- User authorized plan-through-archive without further questions.
