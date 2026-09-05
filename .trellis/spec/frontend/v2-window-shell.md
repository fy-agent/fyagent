# V2 Window Shell and Interaction Contract

## 1. Scope / Trigger

Read this contract before changing V2 shell composition, native-overlay chrome,
top-bar drag regions, frontend-readiness signalling, shared selection material,
collapse animation, motion ownership, external-link interaction, or V2 import
boundaries.

Primary owners are:

- `src/v2/widgets/app-shell/AppShell.tsx`, `TopBar.tsx`, and
  `ContentViewport.tsx` for shell composition and Renderer chrome;
- `src/v2/shared/platform/runtime.ts` and the platform Port surface for runtime
  detection, frontend readiness, window/native effects, and external opening;
- `src/v2/shared/ui/SelectionLens.tsx`, `selection-lens.css`,
  `LiquidGlassLens.tsx`, `motion.ts`, and `Collapsible.tsx` for shared
  interaction material and motion;
- `src/v2/shared/features/controls/ExternalLinkButton.tsx` and
  `src/v2/shared/features/provider.tsx` for HTTP(S) opening and shared feedback;
- `tests/v2/app/architecture.test.ts` for enforceable V2 dependency and
  ownership boundaries.

Route registration, persistent page lifetime, sidebar navigation and return
queries are owned by
[V2 Navigation and Persistent Route](./v2-navigation.md). Native window
geometry and maximize/work-area policy are owned by the backend
[Main Window Layout](../backend/main-window-layout.md) contract.
Shared typography, dialog surfaces and sidebar radius are specified in
[Desktop Visual Hierarchy](./visual-language.md).

## 2. Signatures

Runtime/chrome admission is closed through:

```ts
detectNativePlatform(navigatorIdentity?): "windows" | "macos" | "unknown"
detectRuntime(scope?): RuntimeEnvironment
shouldShowMacOverlayDragStrip(runtime?): boolean
signalFrontendReady(): Promise<void>
useFrontendReady(ready?: boolean): void
```

`AppShell` composes one `TooltipProvider`, one `TopBar`, one
`PrimaryBlockerProvider`, one `SideNavigation`, and one `ContentViewport`.
Only `TopBar.tsx` may declare `data-tauri-drag-region` in V2 production code.

Shared selection/motion APIs are:

```ts
type SelectionLensGeometry = "size-and-position" | "position";

SelectionLensGroup({ id, inset?, geometry?, layoutKey?, children })
SelectionLensTrack({ id, geometry?, layoutKey?, children })
SelectionLens({ active })
selectionLensCollapsedOrigin({ x, y })
selectionLensTransition // alias of fySpringTransition

fySpringTransition
fyMotionTransition(reduceMotion: boolean)

Collapsible({ open, onOpenChange, children, ... })
CollapsibleTrigger
CollapsibleContent({ open, children, ... })
CollapsibleCaret({ open, children, ... })
```

External opening is exposed as:

```ts
ExternalLinkButton({ url?, errorTitle?, busyLabel?, ...buttonProps })
useOpenExternal(): {
  openExternal(url, options?): Promise<void>;
  openingUrl: string | null;
}
SettingsPort.openExternal(url: string): Promise<void>
```

No page receives a native window handle, shell command, unrestricted URL
opener, or direct `@tauri-apps/*` capability through these components.

## 3. Contracts

### Shell and native-window boundary

- `AppShell` is the single production shell root, not the readiness source.
  `useFrontendReady` signals the existing payload-free event after an active
  usable/error route commits; the lifecycle facade deduplicates StrictMode.
  Agents waits for its first local catalog snapshot, Auth for its local
  overview. Other primary routes signal from inside Suspense after content
  commits, not from its fallback. Later background refresh and directory scans
  never delay initial display. RootError and the development lab also signal
  from their actual committed content.
- Shared Brand/BrandIconFrame marks bundled startup artwork; readiness uses
  native image decode for that current local snapshot, not remote/lazy images.
  Failed decoration does not prevent display and stale decode completion after
  hiding/unmount cannot signal. The host recovery handles a truly stalled load.
- A hidden native WebView may not advance animation frames. Do not wait for
  `requestAnimationFrame` or document visibility to authorize initial display.
  A failed signal logs a fixed diagnostic; it does not mark native readiness.
  The host still owns geometry preparation, silent startup, actual show/focus
  and bounded failure recovery. See
  [Window Presentation](../backend/window-presentation.md).
- `shouldShowMacOverlayDragStrip` is true only for a native macOS runtime. The
  macOS overlay adds traffic-light spacing plus one native drag surface. Browser,
  Windows, and unknown-native runtimes do not receive that overlay markup.
- V2 does not draw custom minimize/maximize/close controls. Host/Tauri window
  configuration owns system caption controls, window geometry, monitor/work-
  area decisions, maximize state, persistence, and native resize behavior.
- `data-tauri-drag-region` is limited to the reviewed `TopBar` drag surface. It
  is not placed on feature controls, editable content, navigation links, modal
  overlays, or an entire window where it would intercept normal interaction.
- Renderer layout may reserve native chrome space but cannot use user-agent
  checks as permission for native effects. Runtime detection chooses the visual
  branch; actual effects still go through typed platform Ports.

### Selection material and geometry

- A `SelectionLensGroup`/`Track` owns at most one visual lens overlay. Active
  items register their parent host through the marker component; semantic
  selected/current state remains on that host. The lens and marker are
  `aria-hidden` and never become the interactive target.
- The lens measures relative host geometry, snaps to device pixels, copies the
  host border radius, watches resize/scroll/hidden ancestors, and recomputes
  after a `layoutKey` change. Feature code does not copy this measurement loop.
- `geometry="size-and-position"` animates all box dimensions. A track whose
  selected controls keep a stable size may use `geometry="position"`; the
  primary sidebar uses that mode so movement does not introduce scale
  distortion during collapse/reflow.
- Lens movement uses Motion `x`/`y` transforms rather than updating CSS
  `left`/`top` each frame. Width/height remain fixed in position-only mode.
  The registration context is memoized, so a measured box update does not
  unregister and register every active marker again. Geometry/hidden/reduced-
  motion behavior remains covered by the existing browser and unit tests.
- First positioning and reveal after a hidden ancestor collapse begin at the
  selected host's own origin via `selectionLensCollapsedOrigin`; they do not
  collapse to the track's top-left corner.
- `SelectionLens` must not use Framer Motion `layoutId` scale projection. One
  shared measured overlay and a stable `layoutKey` are the reviewed ownership
  model.
- Reduced-motion preference applies immediately: position/size and collapse
  state settle without spring travel while semantic state remains unchanged.

### Motion, collapse, and material adapters

- `src/v2/shared/ui/motion.ts` is the only direct `framer-motion` import owner.
  Shared components consume its exports and transition helpers; pages do not
  invent unrelated spring literals or import Framer Motion directly.
- `Collapsible` wraps the Radix primitive. Its content remains mounted for
  measured animation, but a closed panel is inert and `aria-hidden`; hidden
  controls must not remain reachable by pointer, focus, or assistive technology.
- Height transitions preserve the last measured open height, settle to `auto`
  after opening, cancel superseded animation generations, and honor reduced
  motion. Consumers own open state and semantic labels; they do not directly
  manipulate the motion value.
- `@samasante/liquid-glass` is imported only by
  `shared/ui/LiquidGlassLens.tsx`. Production callers use the adapter so optics,
  live/filter behavior, accessibility and future dependency replacement remain
  reviewable in one owner.
- Visual glass/lens material never carries selected meaning by itself. Host
  attributes, labels, focus, controls and route state remain the semantic
  source of truth when effects are unavailable.

### External links and V2 architecture

- HTTP(S) jumps use `ExternalLinkButton` or the shared `useOpenExternal`
  context. Pages do not render navigation anchors for external effects, call
  `window.open`, or call `SettingsPort.openExternal` directly.
- The shared opener permits one in-flight external open at a time, exposes the
  active URL for busy UI, disables concurrent buttons, and reports native
  failure through the shared toast path. A click must not launch duplicate
  native requests.
- V2 imports stay inside `src/v2` except for the exact neutral
  `@/shared/codex-desktop` core allowed by architecture tests. This exception
  does not generalize to legacy components, hooks, API wrappers, i18n, or other
  root/shared modules.
- Direct `@tauri-apps/*` imports are limited to
  `src/v2/shared/platform/tauri/`. Dependency direction remains root → app;
  app → pages/widgets/shared/dev; pages → pages/shared; widgets →
  widgets/shared; shared → shared; dev → dev/shared.
- Browser adapters return explicit native-only failures. A browser preview may
  exercise composition and fallbacks, but it is not evidence for native drag,
  window state, shell opening, or readiness signalling.

## 4. Validation & Error Matrix

| Condition                                                                                          | Required result                                                                                                 |
| -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| frontend readiness signalling rejects                                                              | Log the bounded failure; do not claim native lifecycle readiness or crash the shell solely to hide it.          |
| runtime is browser, Windows, or unknown native                                                     | Do not render the macOS overlay drag strip.                                                                     |
| runtime is native macOS                                                                            | Render the reviewed traffic-light reserve and one TopBar drag surface; do not add custom caption buttons.       |
| a feature tries to declare `data-tauri-drag-region`                                                | Architecture check fails; move the drag region to the shell owner.                                              |
| active selection host changes                                                                      | Register one host and move one lens; retain selected semantics on the host.                                     |
| selected host or ancestor is hidden then revealed                                                  | Re-measure after reveal and animate/snap from the host origin without stale track geometry.                     |
| `layoutKey` changes during collapse/reflow                                                         | Re-sample through the bounded settle loop; cancel the superseded loop on cleanup.                               |
| reduced motion is enabled                                                                          | Settle lens/collapse geometry without spring travel while preserving state and accessibility.                   |
| collapsible content is closed                                                                      | Keep measured content mounted only behind inert and aria-hidden state; remove hidden controls from interaction. |
| liquid-glass or Framer Motion is imported outside its shared owner                                 | Architecture check fails; use the reviewed adapter/export.                                                      |
| an external open is already in flight                                                              | Ignore/disable another request until the shared lock releases.                                                  |
| native external opening rejects                                                                    | Clear busy state and emit the shared bounded error toast; do not report success.                                |
| V2 imports legacy/root UI, direct Tauri outside platform/tauri, or a prohibited cross-layer module | Architecture check fails.                                                                                       |

## 5. Good / Base / Bad Cases

- Good: collapsing the configuration group changes its `layoutKey`; the
  sidebar's position-only lens follows the remaining active control without
  scale distortion, while hidden child controls are inert.
- Good: a Skill homepage button disables all external-open buttons during the
  one native request, then clears busy state and reports a native failure
  through the shared toast if opening is denied.
- Base: browser preview renders the standard TopBar without a native drag
  surface. Selection and collapsible semantics still work without proving any
  host effect.
- Base: reduced motion is active; geometry reaches the same final selected and
  collapsed states without animated travel.
- Bad: use `layoutId` on every selected item, import Framer Motion in a page,
  place a transparent drag region over feature controls, draw fake system
  buttons, call `window.open`, or infer native success from a rendered button.

## 6. Tests Required

- `tests/v2/widgets/app-shell/TopBar.test.tsx` covers platform-gated overlay
  markup, traffic-light reserve, and the sole drag region.
- `tests/v2/app/router-shell.test.tsx` covers shell composition and the absence
  of removed page-local/tool chrome; route lifetime itself is owned by
  [V2 Navigation and Persistent Route](./v2-navigation.md).
- `tests/v2/shared/SelectionLens.test.tsx` covers host registration, one overlay,
  size-and-position versus position-only geometry, host-origin reveal,
  layout-key settling, hidden ancestors, cleanup, pixel rounding, and reduced
  motion.
- Shared collapsible tests cover force-mounted height animation, rapid
  generation changes, final `auto`, closed inert/aria-hidden state, caret
  rotation, and reduced motion.
- `tests/v2/shared/ExternalLinkButton.test.tsx` covers undefined URLs, the one
  in-flight lock, busy labels/aria state, duplicate-click suppression, native
  success/failure, toast reporting, and lock cleanup.
- `tests/v2/app/architecture.test.ts` enforces layer direction, the exact
  neutral Codex Desktop exception, direct-Tauri ownership, static imports,
  Framer Motion/liquid-glass adapters, no selection `layoutId`, TopBar-only drag
  regions, and shared external-link usage.
- Browser shell/UI-Lab coverage verifies visual composition, focus, collapse,
  and reduced-motion outcomes. Native macOS/Windows HIL separately verifies
  drag regions and system chrome; portable tests cannot prove those effects.

## 7. Wrong vs Correct

Wrong:

```tsx
<motion.div layoutId="selected" className="selected-glass" />
<a href={url} target="_blank">Open</a>
<div className="page" data-tauri-drag-region>{controls}</div>
```

Correct:

```tsx
<SelectionLensTrack id="primary-nav" geometry="position" layoutKey={open}>
  <NavLink aria-current={active ? "page" : undefined}>
    <SelectionLens active={active} />
  </NavLink>
</SelectionLensTrack>

<ExternalLinkButton url={url}>Open</ExternalLinkButton>
// The reviewed TopBar owns the only native drag surface.
```

Wrong:

```ts
import { invoke } from "@tauri-apps/api/core"; // page/widget
import { motion } from "framer-motion"; // feature page
```

Correct:

```ts
page -> typed feature Port -> shared/platform/tauri adapter -> native command
page/shared component -> shared/ui/motion export -> reviewed transition owner
```
