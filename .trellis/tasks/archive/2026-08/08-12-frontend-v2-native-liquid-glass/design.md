# Design

## Boundaries

The production change is confined to the existing V2 renderer, its V2 tests,
the root package manifest/lock, and the V2 shell code-spec. Trellis planning
artifacts record the work and archive evidence. Full-project verification may
also repair a pre-existing gate contract only on its classifier/checker,
test-harness, and existing spec surfaces; those repairs preserve fail-closed
semantics and do not change product or release-script behavior. The task does
not introduce an API, IPC, storage, backend, or native configuration change.

```text
AppShell
|- signalFrontendReady()                 lifecycle only
|- TopBar
|  |- Brand
|  |- PrimaryNav -> NavLink -> LiquidGlassLens (active item only)
|  `- ToolCluster
`- ContentViewport                       empty Phase 1 route outlet
```

## System-owned chrome

AppShell no longer accepts a frame port. Its effect invokes only the existing
idempotent ready bridge and reports a lifecycle-specific error if that promise
rejects. TopBar has no props and renders Brand/Nav/Tools in DOM order. Removing
the pure window-frame modules and exports makes it impossible for V2 React to
disable decorations or invoke caption actions.

## Material model

- L0: namespaced blue-gray linear/radial ambient gradients.
- L1: transparent/low-boundary content plane with no opaque white card.
- L2: one structural navigation track using translucent layered backgrounds,
  backdrop blur/saturation, border, inset highlight, and depth shadow.
- L3: selected lens, icon controls, tooltip, popover, and UI Lab surfaces use
  a slightly stronger translucent treatment.

All semantic tokens remain `--fy-*`. Material-fill opacity preserves ambient
glass < structural glass < interactive glass. The base edge may match the
interactive fill's alpha, while the emphasized edge and highlight are
stronger. The focus ring, text contrast, selected CSS layer, and
`aria-current` remain functional without the SVG filter. Motion is limited to
color, border, shadow, opacity, and small press transforms; reduced motion
shortens transitions.

## LiquidGlassLens contract

`LiquidGlassLens` accepts only `children` and optional project-owned
`className`. It renders the package's `Glass` with `dispersion: 0`,
`live={false}`, and `filterResolution={1}`, plus the stable
`fy-liquid-glass-lens` class and a test marker. It exposes no package-specific
types or optics to widgets.

`PrimaryNav` retains `NavLink` as the semantic/focus element. Its render
callback places the label inside `LiquidGlassLens` only when active; inactive
labels use the same stable label span without the filter. CSS gives both
branches identical geometry, while the selected link also owns a non-filter
border/color/shadow state. At most one active lens is mounted.

## Responsive and accessibility behavior

The DOM order is the keyboard order: six links then the three buttons. Native
caption controls are outside the renderer. CSS grid keeps Brand and Tools at
the edges and Nav centered; media queries reduce horizontal shell padding,
nav item padding, gaps, and tool sizes at the 1024/920 pressure points without
hiding labels or controls.

Radix continues to own Tooltip/Popover/Tabs behavior and portals. The content
viewport does not clip portal nodes. `prefers-reduced-motion` removes perceptual
motion without removing selected, focus, hover, or pressed styling.

## Compatibility and failure policy

- React 18, Tailwind 3, Router routes, V2 entry, lifecycle payload, and legacy
  renderer remain unchanged.
- Vitest may mock only the filter implementation so jsdom does not pretend to
  validate SVG/browser rendering. Playwright loads the production dependency.
- If the exact package fails typecheck, Chromium Playwright, renderer build, or
  the full project gate and cannot be fixed inside this boundary, the task is
  not archived and the dependency decision returns for review.
- Acceptance-blocking baseline repairs require a reproduced failure, history
  evidence that the gap predates this task, focused positive and negative
  regression coverage, and a successful full-project rerun. Skipping or
  suppressing a gate is not a valid repair.
