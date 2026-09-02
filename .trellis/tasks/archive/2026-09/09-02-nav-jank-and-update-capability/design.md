# Design: 主路由 keep-alive、导航透镜隔离、生命周期能力表

## 1. Architecture and boundaries

```text
Hash router (leaf paths unchanged)
  -> AppShell (prefetch primary page modules after first paint)
  -> ContentViewport Outlet
       -> PersistentPrimaryOutlet
            PersistentSurface x6 (active = location.pathname)
              -> page trees
                   queries.ts ANDs usePersistentVisibility()
                   Agents scan dispatcher pauses when hidden

lifecycle_policy.rs          native admit (install/update/launch)
agent-lifecycle-capabilities.ts   renderer display projection (must match)
AgentLifecycleActionSlot     directory button/busy UI
allowedActions               runtime gate (never invent update)
```

Owners:

| Concern | Owner |
| --- | --- |
| Legal product/surface/action | `src-tauri/src/agent_install/lifecycle_policy.rs` |
| Directory update chrome | `src/v2/shared/features/agent-lifecycle-capabilities.ts` + slot |
| Route mount lifetime | `PersistentPrimaryOutlet` + `PersistentSurface` |
| Query pause when hidden | `queries.ts` via `usePersistentVisibility` |
| Nav pill geometry / overflow | `SelectionLens` + `shell.css` / `selection-lens.css` |

Pages still must not import Tauri. Capability projection is closed `Record<AgentCatalogId, …>` with exhaustive IDs, not per-card `if (id === "qoderwork")`.

## 2. Route keep-alive

`PersistentPrimaryOutlet` stops using `<Outlet />` for the six primary leaves. Child route records remain for URL matching (path only / null element). The outlet:

1. Reads `useLocation().pathname`.
2. Holds a `useRef<Set<string>>` of mounted primary paths. Initialize with the current path. If the current path is a primary leaf and missing from the set, **add it during that render** (ref mutation, no `setState`).
3. Renders each mounted path inside `PersistentSurface active={pathname === path}` + existing page-level `Suspense`.
4. Non-primary (`__dev/ui-lab`, splat) still goes through `<Outlet />`.

Prefetch: `AppShell` `useEffect` on mount calls the same six `import("../pages/*/Page")` factories (or a `prefetchPrimaryRoutes()` helper next to the lazy map). Chunks exist; first navigation must not show `正在加载页面` after prefetch settles.

Hidden Agents:

- `useAgentDirectoryScan` takes `active` from `usePersistentVisibility()`.
- `autoStart` only when `active` becomes true the first time; do not restart a completed/in-flight scan when returning.
- When `active` is false, ignore `settled`/`finish` dispatches (or skip `dispatch`). In-flight `refetch()` may complete into Query cache.
- On becoming visible, hydrate `results` from `queryClient.getQueryData(featureKeys.agentInstallReadiness(id))` without a new seven-way probe if cache is warm.

Do not warm-mount all six page trees on first paint (Models/Skills are heavy). Prefetch JS only; mount on first visit; thereafter keep-alive.

## 3. Query visibility

In `src/v2/shared/features/queries.ts`, every hook with `enabled` becomes `enabled && usePersistentVisibility()`. Default context is `true`, so existing page tests keep fetching.

Imperative `refetch()` from a hidden scan must not be started; already-in-flight refetches do not call `setState` on a hidden scan reducer.

`useAgentInstallationInventory` in directory cards already has an `enabled` flag; it will also pause when the Agents surface is hidden.

## 4. Selection lens glow

Causes to close together:

1. `backdrop-filter` on `.fy-selection-lens` sampling the content plane across the 12px shell gap during scan (blue progress).
2. Subpixel width making the pill 1px wider than the host, leaking `--fy-highlight` at the top-right corner.
3. Unbounded `useLayoutEffect(syncBox)` on every Group render.

Fixes:

- Primary nav group (`geometry="position"` / `.fy-side-navigation-track`) CSS: lens `backdrop-filter: none`; `isolation: isolate`; `overflow: hidden` on the track (clip pill to nav, not content).
- Round `getBoundingClientRect` to device pixels before `setBox`.
- Remove the no-deps `syncBox` layout effect; keep host/track ResizeObserver, `layoutKey`, and `scheduleSync`.
- Browser assertion: during auto-scan, lens `right <= host.right + 0.5` and computed `backdropFilter` is `none`.

Do not put a second `--fy-selected-*` frame on the Agents `NavLink` (already text-only).

## 5. Lifecycle capability

### Backend

```text
QoderWork / TraeWork / WorkBuddy: install=true, update=false, launch=true
Grok CLI:                         install=true, update=true,  launch=false
Codex / Claude / OpenCode:        install=true, update=true,  launch=true
```

`should_resolve_desktop_source(Installed*)` follows `policy.update`, so the three domestic products skip remote latest while installed. `admit_action(..., Update)` returns `ActionNotSupported`. Inventory `discovered_update_eligible` already ANDs `policy.update`.

### Frontend projection

```ts
type AgentDirectoryUpdateUi = "none" | "generic" | "codex_desktop";

export const AGENT_DIRECTORY_UPDATE_UI: Record<AgentCatalogId, AgentDirectoryUpdateUi>
```

- `none`: never offer 一键更新; `deriveAgentLifecyclePrimaryAction` cannot return `"update"`.
- `generic`: offer only if `allowedActions` includes `update` and install/updateState match.
- `codex_desktop`: existing installer VM.

`AgentLifecycleActionSlot` props are display-only: scanning label, busy label, primary action + handler, retry, or null. GenericDirectoryCard and CodexDirectoryCard both render it. Pages must not switch on `agentId` for button markup.

Defense in depth: even if a test fixture sets `allowedActions: ["update"]` on qoderwork, `none` hides the button. Native still rejects.

Adding a future product: add catalog ID + `lifecycle_policy` row + `AGENT_DIRECTORY_UPDATE_UI` entry. Slot is reused.

### Other reuse in this change

| Extract | Why |
| --- | --- |
| `prefetchPrimaryRoutes()` | router and AppShell share the lazy table |
| `useVisibleQueryEnabled(enabled)` | one AND for all feature queries |
| `AgentLifecycleActionSlot` | one directory action chrome |
| `AGENT_DIRECTORY_UPDATE_UI` | one place to turn update chrome off |

Do not merge Codex installer into generic `start_agent_action`. Do not invent a second backend matrix outside `lifecycle_policy.rs`.

## 6. Data flow

### Navigation

```text
click NavLink
  -> hash path changes
  -> outlet marks path mounted (ref) + PersistentSurface active flags
  -> hidden Agents: queries enabled=false, scan dispatch paused
  -> shown page: queries enabled=true, cache hit or first fetch
  -> SelectionLens position spring only (size assigned)
```

### Update action

```text
scan readiness
  -> allowedActions from native (no update for the three)
  -> deriveAgentLifecyclePrimaryAction + AGENT_DIRECTORY_UPDATE_UI
  -> slot renders 一键安装 | 一键更新 | busy | null
  -> start_agent_action still fail-closed if UI and native ever disagree
```

## 7. Compatibility

- URL, leaf IDs, catalog v5, readiness contract v4 unchanged.
- Browser preview: keep-alive still mounts React trees; queries stay non-authoritative.
- Returning to Agents does not require a full remount; local scan reducer survives if the page stayed mounted.

## 8. Trade-offs

- Keep-alive increases retained heap after visiting several pages. Accepted: six pages, queries paused, UX requirement is zero hitch.
- Display projection duplicates the three `update: false` facts. Mitigated by a unit test listing the same IDs as the Rust expected_policy for those agents.
- Prefetch increases idle network after startup. Accepted vs click-jank.

## 9. Rollback

Revert outlet to `<Outlet />`, restore architecture test, set the three `update: true` only if product re-enables FyAgent update. Capability module + slot can remain as the display owner.
