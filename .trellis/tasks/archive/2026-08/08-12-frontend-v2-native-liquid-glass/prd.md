# FyAgent frontend V2 native liquid glass

## Goal

Restore system-owned native window chrome and turn the existing V2 renderer
shell into a Blue Ambient / Clear Glass desktop surface. Keep routing,
lifecycle, accessibility, and the empty Phase 1 business-page boundary stable.

## Requirements

### Native window ownership

- The operating system and Tauri own native title bars, caption buttons, and
  dragging. React renders only Brand, Primary Navigation, Search, Settings,
  Avatar, and the content viewport.
- Remove the V2 custom window-control DOM, drag region, `WindowFramePort`, its
  browser/Tauri factories, and their dedicated tests.
- V2 must not call `setDecorations(false)`. Preserve the existing
  system-decoration configuration and do not change Rust or Tauri config.
- Keep the idempotent payload-free `frontend-deeplink-ready` lifecycle bridge.

### Navigation and content

- Keep exactly six routes in this order: `agents`, `models`, `skills`, `mcp`,
  `prompts`, `memory`; retain `#/models` as index and wildcard fallback.
- Router location remains the only selected-state source. The selected link
  has `aria-current="page"`; do not add component state for selection.
- Keep all six production page elements empty. Preserve the development-only
  `#/__dev/ui-lab` route and the V2/legacy import boundary.
- Keyboard order is exactly the six navigation links followed by Search,
  Settings, and Avatar. Native title-bar controls are outside the React tab
  order.

### Visual system

- Use one Blue Ambient / Clear Glass appearance with four material levels:
  ambient background, low-boundary content plane, structural navigation glass,
  and interactive glass for the selected lens/tools/popovers.
- Use near-white text, restrained blue/cyan highlights, translucent surfaces,
  glass edges, inner highlights, and depth shadows. Do not use an opaque white
  dashboard, selected underline, rainbow/chromatic gimmicks, fake traffic
  lights, or fake Windows controls.
- Keep the existing shell geometry near a 68px top bar, 28px mark, 19px brand,
  46px navigation track, and 38px navigation/tool targets. At narrower widths,
  reduce gaps and padding through CSS media queries; do not use JavaScript
  viewport state.
- Preserve Radix behavior primitives, Phosphor icons, the current logo, React
  18, Tailwind 3, and the current six empty pages.

### Selected navigation lens

- Pin `@samasante/liquid-glass` to exact version `0.1.1` as a runtime
  dependency. Do not add another UI kit, icon library, store, or theme system.
- Hide the third-party API behind one V2-internal `LiquidGlassLens` adapter.
  Use balanced optics with only `dispersion: 0`, `live={false}`, and
  `filterResolution={1}`.
- Render the lens only inside the active `NavLink`, with at most one production
  instance. The link retains the hit target, focus, accessible name, and
  `aria-current` semantics.
- The adapter must always carry a CSS tint/border/highlight/shadow and backdrop
  fallback. SVG refraction is enhancement only and cannot be the sole selected
  or focus indicator.

### Scope boundaries

- Do not modify `src-tauri/**`, native config, backend behavior, IPC,
  persistence, business-page content, legacy renderer files, release scripts,
  installer behavior, or dependency versions beyond the selected lens.
- A pre-existing local-gate contract may be repaired only when the ordered
  full-project gate directly exposes it, repository history proves it predates
  this task, and the repair is limited to the checker/test/spec surface needed
  to restore the existing fail-closed intent. Such repairs must not weaken,
  skip, or suppress the gate.
- Do not start Tauri, run desktop automation, capture visual baselines, or
  claim native/manual visual evidence.
- Repository task/spec/code/journal content must be self-contained and must
  not depend on or identify out-of-repository implementation materials.

## Acceptance Criteria

- [x] Top bar DOM contains only Brand, Primary Navigation, and Tools; no custom
      window controls or title-bar drag region exist.
- [x] V2 has no `WindowFramePort`, window-frame factory, or
      `setDecorations(false)` reference; lifecycle ready still emits at most
      once per renderer lifetime.
- [x] Six routes, default/wildcard redirects, empty content pages, and the
      DEV-only UI Lab keep their current behavior.
- [x] Exactly one selected route renders a `LiquidGlassLens`; inactive routes
      do not, and `aria-current` follows the hash.
- [x] Tab order is six navigation links, Search, Settings, Avatar.
- [x] At 900x600, 1152x640, 1232x700, and 1440x900, Brand/Nav/Tools do not
      overlap, all nine controls remain visible, and the document has no
      horizontal overflow.
- [x] Structural/interactive surfaces are translucent and retain a CSS
      fallback, selected navigation has no underline, portals are not clipped,
      focus is visible, and reduced motion does not remove state meaning.
- [x] The complete local gate passes in order: `mise run env:check`,
      `mise run lint:v2`, `mise run typecheck:v2`, `mise run test:v2`,
      `mise run test:v2:browser`, `mise run build:renderer`,
      `mise run format:check`, `git diff --check`, and `mise run check`.
- [x] The V2 shell code-spec records the system-owned chrome and bounded lens
      contracts.
- [x] Any pre-existing acceptance blocker discovered by the full-project gate
      is root-caused, minimally repaired with regression coverage, and remains
      fail-closed.
- [ ] After local gates pass, the task is committed, archived, journaled, and
      the final worktree is clean without pushing.

## Evidence Boundary

Local automation is the archive gate. Windows title-bar rendering, WebView2
SVG/backdrop performance, 125%/150% DPI behavior, and subjective visual
similarity remain explicitly unverified native/manual evidence.
