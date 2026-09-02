# V2 Shell Contract

## 1. Scope / Trigger

Read this contract before changing `src/v2/**`, the V2-only test/configuration
files, or the renderer entry that selects `src/v2/main.tsx`. Production HTML
(`src/index.html`) loads that entry; leftover `src/main.tsx` is not the
desktop/browser shell. This page is a narrow V2 exception to the leftover
renderer conventions: the existing frontend specs remain authoritative for
every path outside the V2 boundary.

Production V2 code uses this structure:

```text
src/v2/
|- main.tsx              # production composition root
|- app/                  # router, PersistentPrimaryOutlet, RootError, and styles
|- pages/<route>/        # one folder for each first-level route
|- widgets/app-shell/    # visible web-shell composition
|- shared/               # config, assets, UI, feature ports, platform adapters
`- dev/                  # development-only UI Lab
```

Do not create empty `entities`, store, or service layers speculatively.
Existing `shared/features` is the approved port/query module, not a
speculative FSD `features` layer. Reuse is the default V2 preference: search
[Frontend Reuse](./reuse.md) before adding page-local chrome, and put a new
component in `shared/` on the first commit when another module will use it.
Tauri `titleBarStyle: Overlay` is the host native title bar; it does not
hand window geometry to V2. Caption buttons stay system chrome. The inert
macOS drag strip is V2-owned Overlay React chrome in
`src/v2/widgets/app-shell/TopBar.tsx` (app-shell widget, not a feature
route). React must not reimplement caption buttons. The native host still
owns maximize, min-size, and work-area geometry (`set_min_size` is skipped
while maximized).
The approved Skills and MCP exception follows the dedicated
[V2 Skills and MCP Feature Contract](./v2-skills-mcp.md).
Agent directory and model quick setup follow the dedicated
[V2 Agent and Models Contract](./v2-agent-models.md).
Prompt and Memory native business integration follows the dedicated
[V2 Prompts and Memory Native Business Contract](./v2-prompts-memory.md).
The Codex detail may additionally consume the narrow renderer-neutral
`src/shared/codex-desktop/**` contract described by
[Codex Desktop Installer](../backend/codex-desktop-installer.md); this is not a
general legacy-import exception.

## 2. Signatures

Navigation uses this exact internal contract:

```ts
export type NavigationItem = {
  id: "agents" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path: "/agents" | "/models" | "/skills" | "/mcp" | "/prompts" | "/memory";
  label: string;
};
```

For V3, `NavigationItem` remains the authoritative leaf route type and is
wrapped by a typed presentation tree rather than replaced by a second route
registry:

```ts
export type NavigationGroup = {
  id: "agent-configuration" | "configuration-management" | "memory";
  label: string;
  collapsible: boolean;
  items: readonly NavigationItem[];
};
```

The configuration group owns expandable UI state. Leaf route selection stays
Router-owned. The flattened leaf list derived from the tree is the sole input
to the six primary route definitions; `PersistentPrimaryOutlet` mounts each
visited primary page behind `PersistentSurface` and does not keep a React
`useEffect` visited setter. Unvisited primary routes stay lazy.

The selected-lens adapter is V2-internal and does not expose the dependency's
props or types. It is a best-effort decorative enhancement only: the active
host remains visibly selected through its own CSS plus `aria-current`,
`aria-selected`, or Radix `data-state` even when the Lens, observers, motion,
or backdrop filtering are unavailable:

```ts
interface LiquidGlassLensProps {
  children: ReactNode;
  className?: string;
}

export function SelectionLensGroup({
  id,
  inset = 0,
  geometry = "size-and-position",
  layoutKey,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & {
  id: string;
  inset?: number;
  geometry?: "size-and-position" | "position";
  layoutKey?: string | number | boolean;
}): JSX.Element;

export function SelectionLens({
  active,
}: {
  active: boolean;
}): JSX.Element | null;

export function SelectionLensTrack({
  id,
  geometry,
  layoutKey,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & {
  id: string;
  geometry?: "size-and-position" | "position";
  layoutKey?: string | number | boolean;
}): JSX.Element;

export const selectionLensTransition = {
  type: "spring",
  stiffness: 520,
  damping: 42,
  mass: 0.62,
} as const;

export function selectionLensCollapsedOrigin(box: { x: number; y: number }): {
  x: number;
  y: number;
  width: number;
  height: number;
};
```

Feature-page exclusive tracks, management search, and feature lists must
reuse `FeatureTabs`, `FeatureSearch`, and `FeatureList` from
[Frontend Reuse](./reuse.md). Do not hand-roll those recipes in a page, and
do not add a page-local variant "just for this screen".

`LiquidGlassLens` wraps `@samasante/liquid-glass@0.1.1` with balanced optics
plus `dispersion: 0`, `live={false}`, and `filterResolution={1}`. The sliding
selection pill is a separate V2 adapter, `SelectionLens`. `SelectionLensGroup`
owns at most one `pointer-events: none` overlay pill. The default
`size-and-position` mode springs `left` / `top` / `width` / `height` with
`selectionLensTransition`. The narrow `position` mode springs only `left` /
`top` and synchronizes `width` / `height` directly to the active host; it is
used by the fixed-size primary navigation so scan/layout repaint cannot stretch
the pill's right edge. The primary-nav lens (`.fy-side-navigation-track >
.fy-selection-lens`) must set `backdrop-filter: none`: an extra blur samples
the content-plane scan bar across the shell gap and paints a 1px highlight on
the selected capsule's top-right corner. Round overlay geometry to device
pixels; clip the track with `isolation: isolate` and `overflow: hidden`. Do
not run `getBoundingClientRect` on every unrelated Group render. Drive animated values with Motion values so a later
click retargets from the live geometry. Do not unmount or `key=` the overlay
when the active host changes: that restarts the appear transition instead of
interrupting. In the default `size-and-position` mode, appear and
show-after-`hidden` collapse through `selectionLensCollapsedOrigin` to the
active host's top-left with size 0, then spring open there. In `position` mode,
the same origin owns `left` / `top`, while `width` / `height` are synchronized
to the active host before position animation so the primary-nav frame never
grows from zero or stretches during scan repaint. Do not collapse to the track
origin (`inset`, `inset`): that flies the pill from the parent top-left on every
page mount. Callers must not reimplement this origin. Do not use Motion
`layoutId` or `LayoutGroup` scale
projection for this pill: non-uniform `scaleX` plus `backdrop-filter` smears
the capsule and the label. `SelectionLens` only registers the active host; it
is not the semantic state. Geometry observation is bounded to
the active host and its track/container; do not recursively observe the layout
subtree or attach a child-list MutationObserver. Do not import `framer-motion` outside
`shared/ui/motion.ts`. That file owns `fySpringTransition`
(`stiffness: 520`, `damping: 42`, `mass: 0.62`) and re-exports `animate` /
`motion` / `useMotionValue` / `useReducedMotion`. `SelectionLens` and the V2
`Collapsible` adapter both consume it; pages and widgets import the adapter,
not Motion. The lifecycle-ready
operation returns `Promise<void>` and owns a module-level promise guard. Its
native side effect remains the existing payload-free `frontend-deeplink-ready`
event.

There is deliberately no `WindowFramePort` or React/native caption-action
signature in V2. Adding one requires a new reviewed task and native-window
contract, not an ad hoc shell prop.

The Overlay drag strip is gated only by this helper. Pass an injected runtime
in tests; production calls `detectRuntime()`:

```ts
export function shouldShowMacOverlayDragStrip(
  runtime: RuntimeEnvironment = detectRuntime(),
): boolean {
  return runtime.isNative && runtime.platform === "macos";
}
```

Do not derive this from `navigator.userAgent` alone. Playwright on a Mac host
would otherwise render the strip in browser tests.

HTTP(S) jumps share one FeatureProvider outlet. Pages must not wrap
`ports.settings.openExternal` locally, render `<a href>`, or call
`window.open`. Opening a directory is a different port and stays out of this
control:

```ts
export interface OpenExternalOptions {
  errorTitle?: string;
}

export function useOpenExternal(): {
  openExternal: (url: string, options?: OpenExternalOptions) => Promise<void>;
  openingUrl: string | null;
};

export function ExternalLinkButton({
  url,
  children,
  errorTitle,
  busyLabel,
  ...props
}: {
  url?: string;
  children: ReactNode;
  errorTitle?: string;
  busyLabel?: string;
} & Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children" | "onClick" | "type"
>): JSX.Element;
```

`useOpenExternal` keeps one in-flight URL. A second click is ignored until
the first `settings.openExternal` settles. Failures toast
`errorTitle` (default `无法打开链接`) plus `errorMessage(error)` and never
echo the URL. The matching button shows `busyLabel` (default `正在打开…`);
every other `ExternalLinkButton` disables while `openingUrl` is set.

## 3. Contracts

### Navigation and content

The navigation source remains the six leaf routes, presented as three
left-navigation groups from `navigationGroups`. Leaf IDs and paths do not
change; visible labels and grouping do:

| ID        | Path       | Group                    | Visible label |
| --------- | ---------- | ------------------------ | ------------- |
| `agents`  | `/agents`  | `AI软件配置`             | `AI软件配置`  |
| `models`  | `/models`  | `配置管理` (collapsible) | `模型管理`    |
| `skills`  | `/skills`  | `配置管理` (collapsible) | `Skills 管理` |
| `mcp`     | `/mcp`     | `配置管理` (collapsible) | `MCP 管理`    |
| `prompts` | `/prompts` | `配置管理` (collapsible) | `提示词管理`  |
| `memory`  | `/memory`  | `记忆模块`               | `记忆模块`    |

`configuration-management` owns expand/collapse UI state only. Leaf selection
stays Router-owned. The flattened `navigationItems` list remains the sole
input to the six primary route definitions; `PersistentPrimaryOutlet` keeps
each visited primary route mounted behind `PersistentSurface`
(`hidden` / `inert` / `aria-hidden` when inactive).

- Use a hash data router. The index route and every unknown route redirect to
  `/agents`; the stable default URL is `#/agents`.
- Derive selected state only from router location. The active link has
  `aria-current="page"`; do not maintain a second `currentView` state.
- Primary route modules use literal dynamic `import("../pages/<route>/Page")`
  from `app/primaryPages.tsx`. `main.tsx` calls `prefetchPrimaryRoutes()` so
  the first visit to a configuration page does not flash 「正在加载页面」.
  `PersistentPrimaryOutlet` registers a newly visited primary path with the
  React-allowed during-render state adjustment (no `useEffect` visited setter).
  Unvisited routes stay unmounted. Hidden visited trees pause page queries
  through `usePersistentVisibility` in `queries.ts`, freeze route-owned search
  with `usePersistentSearchParams`, and must not rewrite the active route's
  query. Native jobs stay backend/query-owned. Secrets still must not enter the
  hash, URL query, localStorage, sessionStorage, or query cache.
- Leaving an Agent configuration for Models, Skills, MCP or Prompts appends
  only the validated non-secret `agentReturn` / `agentSection` tuple. The
  management navigation propagates that tuple and derives the return Agent URL
  from its closed Agent/section enums; it never accepts an arbitrary return
  path. Clicking `AI软件配置` while already on the Agent route still returns the
  directory. The hidden Agents tree may keep its last visible search snapshot;
  it must not own or write the return path.
- Put each production page element below its matching `pages/<route>/` folder.
  All six routes render their approved business surfaces. Prompts and Memory
  use bounded native feature ports and must not widen the existing command,
  filesystem, or synchronization scope. Browser preview reports these features
  as native-only and never seeds business data.
- Register the UI Lab only when `import.meta.env.DEV` is true. Production must
  not expose `#/__dev/ui-lab`.
- Hidden visited primary routes must not create new page queries, polling,
  subscriptions, or scan UI dispatch. Unvisited routes stay lazy. Cross-route
  install/Auth/change-plan jobs live in backend/query owners; a hidden React
  tree is not a job daemon. `NavLink` pending opacity applies only when the
  destination is not already the active page.
- Brand is non-interactive. React tab order starts with the visible
  left-navigation controls; `TopBar` contributes no keyboard stop until a real
  shell action is implemented. `SideNavigation` owns `ArrowUp` / `ArrowDown` /
  `Home` / `End` among currently visible controls. The configuration toggle
  uses `ArrowRight` to expand or enter the leaf list, and `ArrowLeft` /
  `Escape` to collapse and return focus to the toggle. 「配置管理」
  open/close uses the V2 `Collapsible` adapter (`@radix-ui/react-collapsible`
  plus `fySpringTransition`); do not import leftover
  `src/components/ui/collapsible.tsx` and do not invent a second cubic-bezier.
  Closed or closing leaves are `hidden` / `inert` / `aria-hidden` and must
  not enter Tab or arrow-key cycling, including during the close animation.
  Native caption controls are outside the renderer and outside this tab-order
  contract.

### Window chrome

- The React top bar has exactly one web region in its chrome row: Brand.
  Primary navigation lives in `SideNavigation`, not `TopBar`. Search,
  Settings, and Account are absent rather than focusable placeholders. The
  top bar contains no minimize, maximize, close, or traffic-light controls.
- Overlay React chrome is V2-owned window chrome in `TopBar.tsx` under
  `src/v2/widgets/app-shell/`. It is not a feature route and is not outside
  the V2 React tree. On native macOS `TopBar` renders one inert 28px
  `data-tauri-drag-region` strip above the chrome row. Browser preview,
  Windows, and tests without a native macOS runtime must not render that
  strip. `titleBarStyle: Overlay` remains host chrome; V2 does not own
  maximize, min-size, or work-area geometry.
- Gate that strip with `shouldShowMacOverlayDragStrip()`. The left
  `--fy-titlebar-traffic-light-width` (78px) spacer uses `pointer-events:
none` so traffic lights stay clickable; only the remaining surface is the
  drag region. Brand sits in the 68px chrome row below the
  strip (`--fy-titlebar-drag-height` + `--fy-top-bar-height` = 96px).
- Windows Visible chrome keeps the 68px row and no drag strip. Reports that
  maximize sends UI off-screen are host geometry; follow
  [Main Window Layout](../backend/main-window-layout.md) instead of shrinking
  React layout.
- V2 must not call `setDecorations(false)` or otherwise disable system
  decorations at runtime. Browser preview correctly renders no native controls.
- Do not fake system controls for browser screenshots or geometry tests.
- Direct Tauri imports still live only below
  `src/v2/shared/platform/tauri/**`. The outer shell's only native bridge is the
  ready lifecycle event, not a window-frame facade; feature pages use dedicated
  ports below the same platform boundary.
- The ready lifecycle emits at most once per renderer lifetime, including
  React StrictMode or repeated calls, and is a browser no-op.

### Material and dependency boundary

The V2 shell owns one Blue Ambient / Clear Glass appearance:

```text
L0 ambient background      blue-gray gradients and controlled light fields
L1 content plane           route-owned, translucent, and low-boundary
L2 structural glass        primary navigation track
L3 interactive glass       selected lens, tooltip, and popover
```

- Every semantic token starts with `--fy-`. Material-fill opacity increases
  from ambient to structural to interactive. A base edge may match the
  interactive fill's alpha; the emphasized edge and highlight remain stronger.
- Keep near-white foregrounds, restrained blue/cyan highlights, a visible
  glass edge, an inset highlight, and a depth shadow. Do not use an opaque
  white dashboard, selected underline, rainbow/chromatic effects, or fake
  native chrome.
- Use `@samasante/liquid-glass` only behind `LiquidGlassLens`. Do not wrap
  production navigation labels in `LiquidGlassLens`: SVG refraction smears the
  selected text. Production selected state is host-owned CSS plus an optional
  `SelectionLens` overlay. The UI Lab may mount one isolated specimen. Do not
  stretch a lens across the navigation track, popovers, content plane, or
  background.
- Use `SelectionLens` for interruptible exclusive option tracks: primary nav,
  catalog lists, feature tabs, feature lists, and UI Lab tabs. Feature pages
  must go through `FeatureTabs` / `FeatureList` / `FeaturePagination` rather
  than hand-rolling `fy-feature-tab`, `fy-feature-list-item`, or a page-local
  page-number window. Do not add a pagination npm package; extend
  `FeaturePagination`. One
  `SelectionLensGroup` per track; at most one active pill per group. A page
  may host several groups. Do not use it for Switch, Checkbox, `<select>`,
  pagination, or independent tool buttons. Management-list search uses
  `FeatureSearch`. See [Frontend Reuse](./reuse.md).
- The pill is CSS interactive glass (`--fy-glass-interactive`,
  `--fy-shadow-control`, inset highlight, backdrop fallback). Every selected
  host must independently retain semantic state, focus, and readable selected
  text. Tracks other than primary navigation keep the shared
  `--fy-selected-*` background/border/shadow fallback. Primary navigation is a
  deliberate single-frame owner: the shared Lens alone paints the glass frame,
  while the active `NavLink` paints text/weight/focus only. The expanded
  configuration toggle may keep a non-overlapping context frame; once
  collapsed onto the active leaf it clears that frame and uses the lighter
  secondary/tertiary text tokens. This prevents two coincident glass borders
  and shadows from producing a dragged edge during Agent scanning.
  Motion uses `selectionLensTransition` on one overlay's position and, in the
  default mode, size. A new click retargets that
  spring from the overlay's current geometry, not from the previous host's
  rest box, and must not remount the overlay. When a group first appears, or
  is shown again after an ancestor `hidden`, the same overlay uses
  `selectionLensCollapsedOrigin` of the active
  host (that host's top-left, size 0) and springs open in place. Do not
  collapse to the track origin (`inset`, `inset`). Do not give catalog rails a
  second slider. Do not interpolate size with `transform: scale`.
  `SelectionLensGroup` must retarget that overlay when in-scope layout moves
  the host. Observation is bounded to the active host and track/container.
  Known sibling-layout changes that do not resize either observed box, such as
  expanding 「配置管理」 while 「记忆模块」 is selected, must change the
  group's explicit `layoutKey` (or schedule an equivalent owner-triggered
  remeasurement). Do not recursively observe descendants, attach a child-list
  `MutationObserver`, or use `layoutId` to chase that translation.
- The `NavLink` owns hit area, focus, accessible name, and `aria-current`.
  Selected labels stay ordinary CSS text. Do not wrap them in `LiquidGlassLens`.
  Project CSS must independently express tint, selected border/color/shadow,
  edge/highlight, and backdrop fallback.
- Keep broad structural glass in CSS. SVG filters are not a substitute for
  accessible state and must not be animated across layout or multiplied across
  controls. Do not put the SVG `Glass` node on the sliding pill, and do not
  put the label inside a scaled `layoutId` projection.

### Styling and responsive behavior

- V2 owns its globals, motion, primitives, and semantic tokens. Do not import
  legacy `src/index.css`, dark-theme tokens, UI wrappers, or `src/i18n/**`.
- Namespace V2 selectors. Do not use `transition: all`, animate the
  `backdrop-filter` property, globally hide scrollbars, or ignore
  `prefers-reduced-motion`. Approved layout motion lives in
  `shared/ui/motion.ts`: `SelectionLens` springs overlay geometry and V2
  `Collapsible` springs 「配置管理」 height/caret from the same
  `fySpringTransition`. Both collapse to an instant or near-instant swap when
  the user prefers reduced motion. Do not add another spring token or CSS
  `ease` for that disclosure. Never combine `layoutId` scale projection
  with `backdrop-filter` on the same node. Fill-height master/detail columns
  use shared `SplitPanes` (`shared/ui/split`); do not copy catalog rail
  classes onto Skills, MCP, Prompts, or Memory. Split-pane children fill the
  pane and scroll inside it (`overflow: auto`). `height: 100%` without
  overflow lets cards and assignment controls paint past the pane chrome.
  Skills discovery is the exception: `.fy-skills-page-discovery` scrolls the
  whole feature page (`overflow: auto`); its inner
  `.fy-feature-discovery-scroll` stays in-flow (`overflow: visible`). MCP
  discovery keeps the shared independent scroller.
- HTTP(S) product/docs/repo jumps use `ExternalLinkButton`. The native
  command remains `settings.openExternal`; the Tauri adapter still admits
  only `http:` / `https:`. Do not add page-local open wrappers, custom
  underline links, or a second lock. Memory directory open stays on
  `openOpenClawDirectory`.
- Keep the chrome row near 68px, brand mark 28px, brand text 19px, navigation
  track 46px, and interactive navigation targets 38px. V2 Overlay chrome adds a
  28px inert drag strip above that chrome row so the window can be dragged and
  double-clicked. At 900px, reduce CSS gaps and
  padding without hiding any label or tool or using JavaScript viewport state.
- Preserve Radix Tooltip/Popover/Tabs behavior and portals, Phosphor icons,
  React 18, Tailwind 3, and the existing logo.

### Layer boundaries

Dependencies point downward only. Same-layer imports are allowed
(`pages` → `pages`, `widgets` → `widgets`).

```text
main.tsx (root) -> app
app -> pages, widgets, shared, dev (DEV-only)
pages -> shared
widgets -> shared
shared -> third-party packages or other shared modules
dev -> shared
```

No V2 module may import legacy `src/App.tsx`, `src/main.tsx`,
`src/components/**`, `src/hooks/**`, `src/lib/**`, `src/i18n/**`, or
`src/index.css`. `pages`, `widgets`, and `app` must not import
`@tauri-apps/**` directly.

The sole cross-root shared exception is `@/shared/codex-desktop`. It contains
only installer DTOs, unknown-input parsers, version/state/snapshot/progress
derivations, and safe error projection. It may not import React, Tauri, legacy
renderer modules, i18n, toast, clipboard, or platform adapters. V2 side effects
still flow through `FeaturePorts.codexDesktop`, with Tauri imports confined to
`src/v2/shared/platform/tauri/**`. Architecture tests allow this exact prefix
only and continue rejecting every other `@/shared/**` or legacy import.

The V2 renderer preserves only the minimum host activation handshake. It does
not restore legacy deep-link consumption, database recovery UI, generalized
model synchronization, or the complete startup contract. The bounded
Agent/Models, Skills, and MCP ports do not by themselves make it Release-ready.

## 4. Validation & Error Matrix

| Condition                                                                          | Required result                                                                                                                                                                      |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Empty hash, root route, or unknown route                                           | Redirect to `#/agents`; Agent directory alone has `aria-current="page"`                                                                                                              |
| V2 imports `framer-motion` outside `shared/ui/motion.ts`                           | Architecture test fails; SelectionLens and Collapsible consume the motion owner                                                                                                      |
| Changing the active option remounts the overlay or restarts from `{width:0}`       | Unit test fails; the same overlay node must keep identity and retarget from current geometry                                                                                         |
| First show or show-after-`hidden` collapses to the track origin (`inset`, `inset`) | Unit and architecture tests fail; appear must use `selectionLensCollapsedOrigin` of the active host                                                                                  |
| 「记忆模块」 is selected, then 「配置管理」 expands                                | Overlay follows the memory host; the disclosure changes the group's explicit `layoutKey`, bounded host/track measurement retargets, and Playwright keeps the pill on the memory link |
| Agent scan repaints while primary navigation is selected                           | One `position`-geometry Lens remains; width/height/right stay stable, `backdrop-filter` is `none`, lens right does not exceed the selected host, and the selected host has no duplicate background/border/shadow |
| From `#/agents` open 模型管理 / Skills / MCP / 提示词                              | Agents tree stays mounted and hidden; destination is visible without 「正在加载页面」; hidden Agents must not rewrite the destination query |
| Active configuration leaf is collapsed into its group toggle                       | Toggle keeps `aria-expanded=false` and selected semantics, clears its context frame, uses lighter text/caret, and the one Lens owns the frame                                        |
| Any normal production route                                                        | Exactly one active primary link and one nav `SelectionLens` overlay; no production `LiquidGlassLens`; other tracks may each have their own pill                                      |
| UI Lab development route                                                           | No primary link active; the lab may render one isolated lens specimen                                                                                                                |
| SVG/backdrop filter unavailable                                                    | CSS tint, edge, shadow, focus, and selected state remain readable                                                                                                                    |
| React StrictMode or repeated ready calls                                           | One native `frontend-deeplink-ready` emission per renderer lifetime                                                                                                                  |
| Production requests the UI Lab path                                                | Route is absent and wildcard fallback selects `#/agents`                                                                                                                             |
| Custom caption buttons or `setDecorations(false)` appear                           | Unit, architecture, or browser negative assertion fails                                                                                                                              |
| A drag region appears outside the V2 `TopBar` Overlay chrome                       | Architecture test fails; browser preview still has no drag strip                                                                                                                     |
| Drag strip is gated on userAgent instead of `detectRuntime()`                      | Mac-host Playwright/jsdom can show a false strip; runtime tests must fail                                                                                                            |
| Windows maximize overflow is “fixed” by shrinking V2 chrome                        | Wrong layer; host must skip `set_min_size` while maximized                                                                                                                           |
| V2 calls `setDecorations(false)`                                                   | Static contract search and V2 tests fail                                                                                                                                             |
| V2 imports legacy/upward code, or Tauri outside the platform boundary              | ESLint and executable architecture test fail                                                                                                                                         |
| V2 imports neutral code outside `@/shared/codex-desktop`                           | Architecture test fails; no broader shared-root allowlist                                                                                                                            |
| Neutral Codex shared code imports React, Tauri, platform, or legacy UI             | Architecture test fails; move the side effect behind the V2 port                                                                                                                     |
| A route's rendered state disagrees with its dedicated feature contract             | Shell/content test fails                                                                                                                                                             |
| Prompts or Memory becomes empty after integration                                  | Final task acceptance fails; validate the resolved tree rather than merge messages                                                                                                   |
| Browser Prompts/Memory exposes seeded or private records                           | Native-only/preview contract test fails                                                                                                                                              |
| A supported viewport overflows or overlaps                                         | Playwright geometry gate fails                                                                                                                                                       |
| A page opens HTTP(S) with `<a>`, `window.open`, or a local wrapper                 | Unit/architecture test fails; use `ExternalLinkButton`                                                                                                                               |
| A second HTTP(S) jump starts while one is in flight                                | Ignored; only the in-flight button shows `正在打开…`                                                                                                                                 |
| `settings.openExternal` rejects                                                    | Toast fixed title plus `errorMessage`; the URL is not echoed                                                                                                                         |

## 5. Good / Base / Bad Cases

- **Good:** Clicking `AI软件配置` changes the hash to `#/agents`; that
  `NavLink` alone owns `aria-current="page"`, keeps a sharp CSS label, remains
  keyboard-focusable, and the left nav track keeps one overlay `SelectionLens`
  aligned to the active leaf or the collapsed configuration toggle. Production
  routes mount no `LiquidGlassLens`. A second click before the spring
  settles keeps the same overlay node and continues from its current box.
  Opening a page with a catalog or feature rail expands that page's pill
  from the selected row's top-left, not from the rail's parent origin.
  `/agents` renders the scan directory or four-section configuration shell.
  Models, Skills, MCP, Prompts, and Memory render only their approved
  bounded feature surfaces.
- **Base:** Opening without a route lands on `#/agents`, with the three left
  groups and Brand visible. Browser preview has no system, simulated, or
  focusable placeholder controls.
- **Fallback:** If refraction cannot render, every selected item remains
  semantically exposed and readable through selected text/weight/focus. Tracks
  other than primary navigation also keep their CSS material, border and
  shadow fallback; primary navigation deliberately avoids a second frame.
- **Bad:** React disables decorations, stores `currentView`, renders caption
  buttons/traffic lights, spreads drag regions across interactive chrome,
  stretches one SVG lens across a wide bar, mounts a
  lens per tool, or uses an underline/filter as the only selected indicator.

## 6. Tests Required

Run the V2-specific project tasks:

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

- Unit tests assert default/wildcard redirects, six-route order, Router-owned
  selection, `aria-current`, no production `LiquidGlassLens` on nav labels, one
  nav `SelectionLens` overlay on the track, the L1 control spring, stable overlay
  node identity when the active option changes, appear origin at the active
  host top-left via `selectionLensCollapsedOrigin` (not the track origin), no
  lens outside a
  group, no `layoutId` / `LayoutGroup` on the pill adapter, left-navigation
  group / leaf / expanded / collapsed / keyboard behavior, `aria-expanded` on
  「配置管理」, closed/closing leaves excluded from Tab/arrow cycling,
  reduced-motion instant collapse, stable accessible
  names, absence of Search/Settings/Account keyboard stops, absence of custom
  caption buttons, six non-empty product pages, and idempotent ready behavior.
  Browser/jsdom shells have no drag strip; native macOS may show one inert
  V2 `TopBar` Overlay strip above the chrome row.
  Side-navigation tests cover collapsing 「配置管理」 while a configuration
  leaf is active, and expanding it while 「记忆模块」 is active so the overlay
  stays on the memory host instead of the pre-expand coordinate. They also
  assert one frame material owner and the position-only geometry mode.
  Browser coverage samples the nav Lens across Agent auto-scan frames and
  requires stable width/height/right, `backdrop-filter: none`, lens right not
  exceeding the selected host, plus a transparent selected-host
  background/border/shadow. Route-lifecycle tests keep visited primary pages
  mounted and hidden, forbid a hidden Agents tree from rewriting Models search,
  and require opening 模型管理 from `#/agents` without 「正在加载页面」.
- Architecture/static tests reject legacy dependencies, upward layer imports,
  direct Tauri imports outside `shared/platform/tauri`, and the retired
  window-frame contract. They keep visited primary routes behind
  `PersistentSurface` in `PersistentPrimaryOutlet`, require
  `prefetchPrimaryRoutes()` from `main.tsx`, keep `framer-motion` behind
  `shared/ui/motion.ts`, reject `layoutId` / `LayoutGroup` on the
  SelectionLens adapter, reject collapsing the pill to `left.set(inset)` /
  `top.set(inset)`, and keep `@samasante/liquid-glass` behind
  `LiquidGlassLens`. They keep HTTP(S) jumps on `ExternalLinkButton` /
  `useOpenExternal`. They positively allow only the exact neutral Codex
  shared boundary and negatively prove that a neighboring shared path remains
  forbidden.
- Vitest may mock the third-party filter surface to isolate router and semantic
  behavior. Playwright must load the real production dependency.
- Playwright runs at `900x600`, `1152x640`, `1232x700`, and `1440x900`. At each
  viewport assert no document/top-bar overflow; no Brand overlap with the left
  navigation; the three navigation groups are keyboard reachable and Brand is
  visible; all six product pages are non-empty;
  hash/selected/ARIA/lens agreement;
  left-navigation keyboard order on the default shell route; memory selected
  then 「配置管理」 expanded keeps the nav overlay on the memory link;
  absence of fake
  chrome; and no
  console, page, or framework-overlay error.
- UI Lab browser tests cover translucent surfaces, backdrop or meaningful CSS
  fallback, selected styling without underline, edge/highlight/shadow,
  Tooltip/Popover portal visibility, focus ring, long multilingual stress
  labels, and reduced-motion state independence.
- The production renderer build must omit the UI Lab route and succeed.
- The final post-merge gate asserts all six routes are non-empty and reruns the
  shell, architecture, and four-viewport browser matrix from the resolved tree.
  Pre-merge results remain diagnostic only.
- The root `FyAgent-前端交互预览.html` is a deterministic local standalone
  bundle written by `mise run build:renderer`. It is gitignored and must not
  enter the Git index. The supported-platform scanner may text-exclude only
  that exact root body's local copy; `src/v2/**` sources,
  `scripts/build-v2-preview.mjs`, and every nested same-named file remain in
  scope. Builder tests freeze generator behavior. Do not treat a committed
  copy or its SHA-256 as Required CI evidence.

The full local project gate remains `mise run check`. Real Windows
Tauri/WebView2 chrome, SVG/backdrop performance, current-host 125%/150% display
scaling, and subjective visual similarity remain separate unverified manual
acceptance evidence unless a task explicitly requires them.

## 7. Wrong vs Correct

Wrong: morph the pill with `layoutId` scale projection (or a CSS transition
queue). Non-uniform `scaleX` plus `backdrop-filter` deforms the capsule and
smears the label. Collapsing appear to the track origin flies the pill from
the parent top-left on every page mount.

```tsx
<motion.div layoutId="nav" className="fy-selection-lens" />
<motion.div key={activeId} animate={{ left, width }} transition={{ duration: 0.25 }} />
setHost(null); // on every option change, then mount at the new rest box
left.set(inset);
top.set(inset);
width.set(0);
height.set(0);
```

Correct: one overlay pill per exclusive track. Catalog rails, feature lists,
and tabs keep the default `size-and-position` mode and spring `left` / `top` /
`width` / `height` from the current overlay values. Primary navigation is the
narrow exception: `geometry="position"` springs only `left` / `top` and
assigns `width` / `height` directly so Agent scan repaint cannot stretch the
glass edge. First show and show-after-`hidden` replay
`selectionLensCollapsedOrigin(activeHostBox)` for position; in `position`
mode, size is synchronized to the host before that animation.

```tsx
<SelectionLensGroup
  id="side-navigation"
  geometry="position"
  layoutKey={configurationExpanded}
  inset={1}
>
  <NavLink to={item.path}>
    {({ isActive }) => (
      <>
        <SelectionLens active={isActive} />
        <span className="fy-side-navigation-item-label">{item.label}</span>
      </>
    )}
  </NavLink>
</SelectionLensGroup>
```

Wrong: keep `--fy-selected-*` background/border/shadow on a collapsed
active configuration toggle, or let SideNavigation use the default size
spring. Two coincident frames plus width interpolation produce a dragged
glass edge while Agent scan repaints.

```tsx
<SelectionLensGroup id="side-navigation" inset={1}>
  <button className="fy-side-navigation-item is-active" aria-expanded={false} />
</SelectionLensGroup>
```

Correct: the expanded configuration toggle may keep a non-overlapping
context frame. Once collapsed onto the active leaf, set
`data-collapsed-active` so CSS clears the host frame and weakens text/caret;
the one Lens remains the only frame owner.

```tsx
<button
  data-collapsed-active={visuallyActive ? "true" : undefined}
  aria-expanded={false}
/>
```

Wrong: recursively observe the complete track subtree to discover every
possible sibling-layout change. That makes observer count and layout reads
scale with unrelated descendants.

```ts
observeLayoutSubtree(scope, observer);
```

Correct: observe only the active host and track/container. The owner of a
known layout transition changes `layoutKey`, which schedules one bounded
remeasurement and springs the same overlay to the host's new box.

```ts
observer.observe(scope);
observer.observe(host);
<SelectionLensGroup layoutKey={configurationExpanded}>…</SelectionLensGroup>
```

Wrong: import `framer-motion` in a widget, or hide 「配置管理」 with only
instant `hidden` and a separate CSS `ease` on the caret.

```tsx
import { motion } from "framer-motion";
<ul hidden={!expanded}>{items}</ul>;
```

Correct: pages/widgets import V2 `Collapsible`; only `shared/ui/motion.ts`
imports Motion. Height and caret share `fySpringTransition`. Closed leaves
stay out of keyboard cycling.

```tsx
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
```

Wrong: give split-pane children `height: 100%` with visible overflow, so
assignment controls paint past the panel.

```css
.fy-split-pane > * {
  height: 100%;
}
```

Correct: fill the pane and scroll inside it, the same way catalog rails do.

```css
.fy-split-pane > * {
  min-height: 100%;
  height: 100%;
  overflow: auto;
}
```

Wrong: make React own native chrome and selected state while depending directly
on an optical effect.

```tsx
const [currentView, setCurrentView] = useState("models");
await getCurrentWindow().setDecorations(false);
return <button aria-label="Close" onClick={closeWindow} />;
```

Correct: let Router own the semantic link, keep the selected label as CSS
text, and keep caption buttons outside React. Overlay drag
chrome stays in `TopBar.tsx`; `titleBarStyle: Overlay` and window geometry
stay with the host.

```tsx
<NavLink to={item.path}>
  {({ isActive }) => (
    <>
      <SelectionLens active={isActive} />
      <span className="fy-side-navigation-item-label">{item.label}</span>
    </>
  )}
</NavLink>
```

Wrong: show the Overlay drag strip because the user agent looks like macOS.

```ts
if (/Mac/i.test(navigator.userAgent)) {
  return <div data-tauri-drag-region />;
}
```

Correct: require a native macOS Tauri runtime.

```ts
if (shouldShowMacOverlayDragStrip()) {
  return <div data-testid="titlebar-drag-region">…</div>;
}
```

Wrong: use the neutral-core exception as a route into a legacy Hook.

```ts
import { useCodexDesktopInstaller } from "@/hooks/useCodexDesktopInstaller";
```

Correct: import only pure Codex contracts and place native effects behind the
V2 feature port.

```ts
import { deriveInstallerViewState } from "@/shared/codex-desktop";
const local = await ports.codexDesktop.getLocalStatus();
```

Wrong: give MCP Discover a custom underline control, or wrap
`ports.settings.openExternal` again on a feature page.

```tsx
<button className="fy-mcp-card-link" onClick={() => onOpen(item.docs)}>
  文档
</button>
```

Correct: reuse `ExternalLinkButton`. The FeatureProvider lock is the only
in-flight gate.

```tsx
<ExternalLinkButton url={item.docs}>文档</ExternalLinkButton>
```
