# Surfaces and Container Response

## 1. Scope / Trigger

Read before changing Renderer palette, translucent surfaces, material dependencies,
radius/spacing, narrow forms or overflow behavior. `tokens.css` owns visual
roles; `controls.css`, feature CSS and catalog/split CSS compose them. This
does not redesign the seven primary routes or native window geometry.

## 2. Signatures and Owners

```ts
// shared/ui/GlassMaterial.tsx — sole @samasante/liquid-glass import owner
FrostedSurface({ enhanced?: boolean }): JSX.Element // stable CSS backing, true by default
LiquidGlassLens({ children, className? }): JSX.Element // UI Lab specimen
// shared/ui/split/SplitPanes.tsx — product adapter; split/vendor.ts owns the library import
SplitPanes({ children, className?, minWidths?, maxWidths?, defaultWidths?, flexiblePane?, separatorLabels?, paneCssVars? })
// Size arrays are readonly pixel constraints; flexiblePane defaults to last.
// split/sizing.ts: DETAIL_PANE_SIZING selects middle index 1 for Skills/MCP.
FeatureTabPanel({ tabsId, value, active, layout: "flow" | "workspace", ... })
```

Radius roles are compact/control/item/panel/dialog/pill/circle/brand/separator.
Use `var(--fy-radius-...)` rather than page-local numeric radius declarations.
`inherit` is valid for a backing/progress child; `0` is valid for a square edge.
Small geometry exceptions still have named tokens. `--fy-space-*` defines the
shared spacing scale; do not couple unrelated layout dimensions just because
they happen to be numerically equal.

`--fy-dialog-surface`, `--fy-modal-scrim`, `--fy-modal-blur`,
`--fy-surface-blur`, `--fy-surface-opaque`, `--fy-surface-input` and
`--fy-glass-sheen` separate foreground backing, page dimming and filter roles.
The default palette is light blue-grey with dark ink; the optional mist-blue
dark palette and preference lifecycle are owned by [Appearance](./appearance.md).
`--fy-surface-inset`,
`--fy-surface-raised`, `--fy-surface-hover` and `--fy-surface-popup` pair local
surfaces with readable text; pages must not hardcode a theme's fills or white
foreground assumptions. Selection/control sheen also has paired token roles so
stacked translucent highlights do not wash out dark-mode text.
The content viewport is not a nested backdrop sampler.
CSS consumes the blur/rim/sheen tokens directly, including preference changes.

## 3. Contracts

- Page-level tabs declare `layout="workspace"` to propagate bounded remaining
  height through the semantic panel to its workspace/panes. Form/ordinary content
  tabs explicitly declare `"flow"`. The prop is required so a new tab cannot
  silently become an unconstrained block above an overflow-hidden ancestor.
  Headers remain nonshrinking; workspace panels use min-height:0 and flex-basis:0,
  with native overflow for error/empty fallbacks. Discovery/list panes own their
  own scrolling, not a Skills-only whole-page overflow override.
- Scroll reachability requires native wheel and keyboard evidence using long
  fixtures. Programmatic scrollTop, scrollIntoView and Playwright click's automatic
  positioning cannot prove a user can scroll to an action. Do not intercept wheel
  or build a second scroll implementation to compensate for a broken height chain.
- `SplitPanes` delegates pointer admission, keyboard resizing, ARIA values,
  cursor management and constraints to `react-resizable-panels`. Its own
  responsibility is product minimum/default sizes, semantic labels and the
  actual available container width. Do not add page-local drag handlers or
  override the library group's layout with Grid/Flex declarations.
- Insufficient width stacks the same Panel tree vertically with local scrolling;
  it does not remount editors or assume the viewport is as wide as a nested
  pane. Zero-size/hidden groups cannot admit resize gestures and retain their
  last orientation until measurable. Non-flexible columns preserve pixels;
  the selected flexible column consumes remaining space. The default remains
  the final column for ordinary two-pane consumers. Skills/MCP share
  `DETAIL_PANE_SIZING`: list min/default/max 220/268/420px, middle detail min
  360px and flexible, assignment min/default/max 220/280/360px. These are
  product roles, not universal percentages or a second stored layout. Retain
  library constraints and users' drag choices across ordinary group resizing.
  Double-click resets the adjacent auxiliary rail, not the flexible detail;
  the adapter disables the library's competing default reset for that action.
  Keep the vendor import at the split
  boundary rather than the eagerly imported global primitive facade.
- If ResizeObserver is unavailable, do not mount the library's observer-dependent
  Group. Render readable stacked static panes, no resize handles, retaining the
  same content within that fallback. No custom resize engine/polyfill is added.
  This is a capability fallback, not a viewport breakpoint remount strategy.
- Selection material is a decorative sibling below the real controls. A
  `display:contents` semantic wrapper is not a stacking box; assign content
  stacking to the real buttons/labels. A pointer-inert backdrop can still
  obscure text, so DOM presence/hit testing alone is not paint evidence.
- Modal content is transparent and isolated. `FrostedSurface` is an absolute,
  pointer-inert, aria-hidden backing; text and controls remain outside it.
  Do not lower steady-state form opacity to simulate glass, refract text or
  clone credential DOM. The bounded foreground arrival transition is owned by
  [Motion and Dialog Presence](./motion-system.md), not the material adapter.
- `FrostedSurface` preserves the same backing and static rim nodes during
  travel and at rest. `enhanced` only crossfades the rim emphasis; it must not
  replace the material at the last frame. Large business dialogs use CSS tint,
  thin frost and layered highlights, not SDF/displacement-map generation.
  The installed liquid-glass package remains isolated to the UI Lab specimen.
  No `refract`, video, canvas, copied DOM or live optical RAF belongs in a form.
- The normal surface is translucent, with a separate blurred/dimmed overlay.
  Do not hide the effect behind an almost opaque tint. Readability has priority
  over maximizing transparency; retain solid primary and readable secondary
  text roles instead of repeated low-alpha white layers.
- The business backing does not require canvas/ResizeObserver. Missing backdrop-filter,
  reduced transparency and forced colors keep a readable solid fallback.
  The fallback must override library inline filters when necessary; it never
  removes dialog semantics, labels, focus or actions.
- `.fy-catalog-detail` is the named `fy-detail` inline-size container.
  Models and account detail react to their own available width, not just the
  window. Forms use `auto-fit/minmax(min(100%, ...), 1fr)` as a useful baseline;
  container queries enhance stacked fields and headers at constrained widths.
- `.fy-control-dialog-content` is the `fy-dialog` container. A constrained
  account picker stacks options; actions stay in the nonshrinking footer.
- Assignment grids use explicit `grid-auto-rows:max-content`; never fixed
  heights or leftover-space automatic rows. Continuous narrow/wide re-entry
  must not inflate a 31px row to hundreds of pixels in WebKit. Shared bulk
  presentation uses explicit name/action slots and one content-box breakpoint;
  see [Assignments](./assignments.md). No repaint timer or route remount is a fix.
- Info cards use `--fy-info-card-min:256px` and `--fy-info-card-gap` with
  intrinsic Grid sizing, capped at two columns. Admission depends on the actual
  grid width, not a window breakpoint; a full-span item must not keep a blank
  third column. `align-items:start` preserves natural short-card height.
  Metadata uses `fit-content(var(--fy-definition-label-cap)) minmax(0,1fr)`;
  the shared label cap is `min(30%,8em)`, with `--fy-definition-gap` between
  name and value. The `fy-info-card` content container stacks definitions at
  210px or less. Existing Auth 500px and form-specific floors retain their roles.
  Copy-only paths remain copy-only; do not reveal hidden data to fill space.
- Flexible text/actions need `min-width:0`, bounded width, wrapping and
  `overflow-wrap:anywhere` where URLs/identities can be long. Editable code and
  path/list previews may have explicit local scrolling/ellipsis; do not add
  whole-page overflow clipping to conceal missing fields or buttons.
- Critical control boundaries target 3:1; ordinary and supporting readable
  text target 4.5:1. Disabled controls and decorative brand artwork are reported
  separately, not misclassified as completed contrast checks.

## 4. Validation & Error Matrix

| Condition                                            | Required result                                                                     |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Pane is narrow in a wide window                      | Form stacks based on container; long text remains within its pane.                  |
| Three-pane window grows after shrinking              | Middle detail absorbs growth; auxiliary rails obey pixel bounds and drag choices.   |
| Detail has less than two card floors plus one gap    | One full-width card column, regardless of viewport width.                           |
| WebKit crosses two/three-pane admission repeatedly   | Rows remain intrinsic; no accumulated height or force-remounted content.            |
| URL/identity has no natural breaks                   | Wrap in detail; no horizontal escape or lost action.                                |
| Standard/comfortable dialog at small viewport        | Body scrolls as needed; footer actions remain reachable.                            |
| Canvas/ResizeObserver is absent                      | Same stable CSS material, not startup failure.                                      |
| ResizeObserver is unavailable to split-pane library  | Use static stacked readable panes without pointer/keyboard resizing; retain drafts. |
| Filter absent, reduced transparency or forced colors | Readable backing, native semantic colors where appropriate, no invisible controls.  |
| Transparent/gradient background                      | Review composited result, not just isolated token color values.                     |
| New raw radius declaration outside tokens            | `designTokens.test.ts` fails; assign the appropriate role.                          |

## 5. Good / Base / Bad Cases

Good: moving the split handle narrows a Models detail while the window remains
wide; the form stacks without changing navigation or save semantics.
Base: Firefox/Safari retain ordinary frost even when the library's Chromium
backdrop refraction is unavailable. Bad: a GPU screenshot of the full account
page becomes a background texture, or modal text is made translucent to fake
glass.

## 6. Tests Required

- `responsive-density.spec.ts` exercises Skills/MCP in both themes and engines:
  1564→1232→1180→900→1181→1564, content-relative row heights, uniform atomic
  bulk pairs, width growth, real drag/keyboard/reset, bounded long metadata,
  616px/font-enlargement pressure and draft/hidden-route lifetime. Check both
  initial and post-resize geometry; no-overflow alone misses inflated rows.
- `scroll-ownership.spec.ts` covers long Skills/MCP installed lists with wheel
  and End, discovery scrolling, revisits, both themes and Chromium/WebKit. Static
  type checking requires every FeatureTabPanel to choose its layout role.
  Other route fixtures exercise actual native owners, including the ancestor
  viewport for flow pages and textareas for long documents. Wait for real
  content-size transitions before checking the bottom; do not invent nested
  scrolling merely because a test only searched descendants.
- `tests/renderer/shared/designTokens.test.ts` uses PostCSS to reject scattered radius
  literals; do not invent a CSS parser or skip component files.
- `tests/renderer/shared/GlassMaterial.test.tsx` proves stable node identity across
  enhancement and absence of canvas/SVG/form copies in business backing.
- `tests/browser/materials-responsive.spec.ts`: seven page surfaces, actual
  composited text samples, axe contrast/label checks, critical input boundaries,
  a 320px detail independent of viewport width, 760px boundary sides, a 616px
  viewport as horizontal 200%-zoom pressure, and forced-color/transparency
  fallbacks. Existing four-desktop-size tests remain mandatory.
- `support/visual.ts` samples finite raster backgrounds with glyph paint hidden
  but unchanged layout. It supplements axe's incomplete gradient/filter cases;
  it is not a replacement accessibility engine or blanket WCAG certification.
- `layout-integrity.spec.ts` additionally checks actual selected-label raster
  ink in Chromium/WebKit, meaningful prompt detail/search/editor widths with
  empty and populated data, real pointer/keyboard min/max/reset behavior and
  unsaved draft identity across narrow/wide container changes. No horizontal
  overflow alone is insufficient: an unusable 30px pane can pass that check.
- Static material/contrast evidence waits for `data-motion-settled="true"`
  and the actual expected computed filter/foreground state. A CDP media feature
  override supplies the complete intended set, including reduced motion while
  testing reduced transparency; do not accidentally restart an entrance and
  treat a transient alpha sample as a stable color. Motion itself remains
  covered by real-time and interrupted-animation browser tests.
- Production bundle smoke/chunk budgets and final motion+material performance
  must still pass. A dev-only screenshot does not prove native WebView/GPU
  behavior, every long document or every contrast pair.

## 7. Wrong vs Correct

Wrong: `border-radius:13px` in one account card and `15px` in the same-role
card elsewhere; `opacity:.7` on the entire modal; `overflow:hidden` on a form
to silence overflow; window-only breakpoints after adding resizable panes.

Correct: choose the shared role, render material as a decorative sibling of
crisp content, wrap real text, and respond to the detail container's width.
