# Stage 5 implementation results — 2026-08-30

## Identity and evidence boundary

- Specification/baseline commit: `9e7eef985316c23df99cc4738ca1cb127ccf5595`.
- Main implementation commit: `3110a5cc24d3d1b65ace734954a5dc05203ff806`.
- Agent return URL correction: `0c87c12e473172c9516bb2233e6a9d7ee95cdd07`.
- React warning gate: `32f81e9682941ef5c646020b776e6c916cc48203`.
- Comparison base: `origin/main` at
  `1e52e416900426cdc86539ee5c359f486ed08bb3`.
- Local host: macOS 26.6.2 arm64; Node 24.19.0; pnpm 10.12.3;
  Rust 1.97.1.

Browser and mock evidence below does not prove installed Tauri WebView or
Windows WebView2 behavior. Native installed-app acceptance remains explicitly
unexecuted.

## Production build before and after

The Stage 5 baseline emitted one app-owned main JavaScript chunk of 881.21 kB
(278.51 kB gzip) and the default Vite `>500 kB` warning. Primary routes were
statically imported and no route chunk existed.

The final production graph contains one active-shell entry and six separately
reachable route chunks. `scripts/verify-v2-route-chunks.mjs` reported:

| Boundary | Bytes |
| --- | ---: |
| Initial JavaScript, including reviewed vendor chunks | 595,690 |
| Initial CSS | 32,554 |
| Largest app-owned initial chunk (`main`) | 96,886 |
| Largest route chunk (Models) | 111,639 |

| Route module | Bytes |
| --- | ---: |
| Agents | 52,111 |
| Models | 111,639 |
| Skills | 21,567 |
| MCP | 48,620 |
| Prompts | 12,574 |
| Memory | 15,281 |

Reviewed initial vendor chunks are React/router (262,039), Motion (79,276),
shared dependencies (72,439), Radix (45,704), TanStack Query (35,843), and
Tauri (1,127) bytes. The contract fails if a primary route is no longer
separately reachable or an app/route/initial/CSS budget is exceeded. No warning
limit was raised.

The self-contained HTML preview uses a separate source-level Vite build with
`inlineDynamicImports`. Its intentional single JavaScript file still receives
Vite's size advisory, but it is not the production route graph and no limit is
raised to hide it. The builder then inlines CSS, JavaScript, and assets. A real
Chromium `file://` test opened Prompt and Memory without page errors or network
chunk requests.

## Route, query, DOM and observer lifecycle

```text
Hash data router
  -> lazy primary route module
    -> active route tree only
      -> domain-owned queries/effects
```

- `PersistentPrimaryOutlet` renders only `<Outlet />`; it owns no visited-page
  registry and performs no render-phase state update.
- Router tests prove inactive Models, Skills, MCP, Prompts and Memory trees
  unmount. Returning reconstructs backend resources through FeaturePort/query
  owners and non-secret selection through URL authority.
- Browser fixtures prove an unvisited Agent route does not invoke its catalog
  or readiness commands. Route-owned effects, polling and observers disappear
  with their route. Backend install/Auth/change-plan jobs remain backend/query
  resources and are reread on remount.
- Agent configuration return state uses only validated `agentReturn` and
  `agentSection` query parameters. The shell propagates the closed tuple across
  management routes and derives the Agent URL from it; arbitrary return paths,
  secrets, commands and filesystem values are not accepted.

`SelectionLens` is decorative. Selected hosts independently paint semantic
CSS and ARIA state. Each lens group creates at most one `ResizeObserver`,
observing only the group scope and active host. The optional MutationObserver
observes only ancestor `hidden` attributes. Explicit sibling layout changes use
a bounded 48-frame remeasurement window; there is no subtree resize walk,
child-list observation or permanent animation-frame loop.

## State ownership decisions

| State class | Final owner |
| --- | --- |
| Backend resources and durable jobs | FeaturePort + TanStack Query/backend job owner |
| Current route/Agent/section/return selection | Validated hash query parameters |
| Prompt/Memory dirty draft | Route/domain component + shared blocker dialog |
| Transient tab/list/search state | Active route/component; discarded on unmount unless explicitly URL-owned |
| Secret draft | Active domain panel memory only; existing security cleanup contract |

No general global store or second design system was introduced.

## Responsibility review

### Models

```text
Models route -> URL target orchestration -> target panel
             -> domain FeaturePort / Change Plan workspace
             -> authoritative reread
```

The route no longer retains visited target trees or updates selection during
render. Existing panel/workspace boundaries are independently tested; further
line-count-only splitting was rejected.

### Skills

```text
Skills route -> Installed / Discover FeatureTabs
             -> domain controllers -> Skills FeaturePort
```

The route adopts the shared Radix-backed tab and authoritative assignment
owners. Search/list/clear chrome remains shared. It does not share the
Prompt/Memory draft lifecycle, so no generic draft provider was added.

### MCP and MCP catalog

```text
MCP route -> Installed / Discover FeatureTabs
          -> server editor or catalog presentation -> MCP FeaturePort
MCP catalog -> existing recipe/data validation owner -> presentation consumer
```

The catalog recipe/data file is already separated from route orchestration.
Moving it again solely to reduce line count would add no props, test, lazy-load
or reuse boundary and was rejected.

### Prompts

```text
Prompts route -> application rail/query selection -> route dirty draft
              -> PrimaryBlocker/ConfirmDialog -> Prompts FeaturePort reread
```

Dirty navigation is explicit and tested; no blanket keep-alive is used.

### Memory

```text
Memory route -> Long-term / Daily FeatureTabs -> editor/search controller
             -> route dirty draft blocker -> Memory FeaturePort
```

Dialog tests await both opening and Presence cleanup instead of suppressing
React warnings.

### Agent Skills/MCP assignment

```text
Agent section -> shared authoritative-assignment controller
              -> serialized visible mutation -> authoritative refetch
              -> typed verification and feedback
```

Skills and MCP are the two real consumers. While a write is pending the
relevant controls expose busy/disabled state; clicks are not silently dropped.
The helper is not a generic async mutation engine.

## Same-domain defects found and fixed

| Finding | Root owner | Resolution |
| --- | --- | --- |
| Rebundling minified lazy chunks broke React circular initialization in the standalone file | Preview build owner | Dedicated source-level single-chunk Vite mode |
| Agent return selection was lost after active-route unmount | Shell/Agent route contract | Strict URL tuple propagated across management routes |
| Memory Lens lagged behind Collapsible sibling translation | SelectionLens geometry owner | Explicit layout key plus bounded tracking |
| UI Lab Popover occasionally ignored Escape after layered overlays | UI Lab Radix specimen | Controlled open state and explicit Escape close |
| Lazy Agent command assertion raced route settlement | Browser test boundary | Poll authoritative fixture calls |
| Codex/Prompt/Memory tests emitted `act(...)` warnings | Test lifecycle boundary | Await reads/Dialog presence cleanup; exact global fail-fast guard |

## Validation evidence

- `mise run typecheck:v2`, `mise run lint:v2`, `mise run test:v2`:
  58 files / 417 tests passed; zero React act warnings.
- `mise run test:v2:browser`: 140/140 passed across 900×600, 1152×640,
  1232×700 and 1440×900.
- `mise exec -- pnpm build:renderer`: production route budget and standalone
  source build passed.
- `mise run format:check`, `mise run supported-platform:check`,
  `mise run rust:fmt:check`, `mise run rust:clippy`, `mise run rust:test`
  passed. The Rust run included 2,945 library tests and integration suites,
  with five platform fixtures explicitly ignored and no failures.
- `mise run test:desktop:mock`: 7/7 passed.
- `mise run test:desktop:visual:preflight`: read-only preflight ready; no
  screenshots or repository writes were produced.
- `mise run check:contracts`: repository/task/version/release/platform
  contracts, 575 repository contract tests, and native-fetch mocks passed.
- `mise run check:prearchive --exclude-active-task
  .trellis/tasks/08-29-frontend-reliability-architecture`: the complete
  current-host environment, frontend, backend, desktop-mock, platform and
  release gate passed with exit code 0 after the new route-chunk tooling was
  assigned to the frontend/contracts CI domains.

## Residual evidence and non-claims

Installed-app UAT was not run on macOS Tauri or Windows WebView2. This task
therefore does not prove native focus integration, real DPI scaling,
minimize/restore, GPU/backdrop implementation, multi-monitor behavior, OS
accessibility integration or native long-content scrolling. Windows 125/150%
DPI and full keyboard acceptance from Issue #141 remain an explicit evidence
gap. Browser/mock results are not reported as native compatibility or release
readiness.
