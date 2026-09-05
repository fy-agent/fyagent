# Surfaces and Container Response

## 1. Scope / Trigger

Read before changing V2 palette, translucent surfaces, material dependencies,
radius/spacing, narrow forms or overflow behavior. `tokens.css` owns visual
roles; `controls.css`, feature CSS and catalog/split CSS compose them. This
does not redesign the seven primary routes or native window geometry.

## 2. Signatures and Owners

```ts
// shared/ui/GlassMaterial.tsx — sole @samasante/liquid-glass import owner
FrostedSurface(): JSX.Element
LiquidGlassLens({ children, className? }): JSX.Element // UI Lab specimen
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
The material adapter reads the shared blur token on mount; its static optical
parameters remain in that adapter, not in feature pages.

## 3. Contracts

- Modal content is transparent and isolated. `FrostedSurface` is an absolute,
  pointer-inert, aria-hidden backing; text and controls remain outside it.
  Do not apply opacity or refraction to the entire form or clone credential DOM.
- Bare library wrapping selects material mode. Do not pass `refract`, video,
  animated geometry or `filterResolution` just to render a modal background;
  those select different, more expensive paths in the adopted version.
- The normal surface is translucent, with a separate blurred/dimmed overlay.
  Do not hide the effect behind an almost opaque tint. Readability has priority
  over maximizing transparency; retain solid primary and readable secondary
  text roles instead of repeated low-alpha white layers.
- Missing canvas/ResizeObserver uses the CSS backing. Missing backdrop-filter,
  reduced transparency and forced colors keep a readable solid fallback.
  The fallback must override library inline filters when necessary; it never
  removes dialog semantics, labels, focus or actions.
- `.fy-catalog-detail` is the named `fy-detail` inline-size container.
  Models and account detail react to their own available width, not just the
  window. Forms use `auto-fit/minmax(min(100%, ...), 1fr)` as a useful baseline;
  container queries enhance stacked fields and headers at constrained widths.
- `.fy-control-dialog-content` is the `fy-dialog` container. A constrained
  account picker stacks options; actions stay in the nonshrinking footer.
- Flexible text/actions need `min-width:0`, bounded width, wrapping and
  `overflow-wrap:anywhere` where URLs/identities can be long. Editable code and
  path/list previews may have explicit local scrolling/ellipsis; do not add
  whole-page overflow clipping to conceal missing fields or buttons.
- Critical control boundaries target 3:1; ordinary and supporting readable
  text target 4.5:1. Disabled controls and decorative brand artwork are reported
  separately, not misclassified as completed contrast checks.

## 4. Validation & Error Matrix

| Condition                                            | Required result                                                                    |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Pane is narrow in a wide window                      | Form stacks based on container; long text remains within its pane.                 |
| URL/identity has no natural breaks                   | Wrap in detail; no horizontal escape or lost action.                               |
| Standard/comfortable dialog at small viewport        | Body scrolls as needed; footer actions remain reachable.                           |
| Canvas/ResizeObserver is absent                      | CSS material fallback, not startup failure.                                        |
| Filter absent, reduced transparency or forced colors | Readable backing, native semantic colors where appropriate, no invisible controls. |
| Transparent/gradient background                      | Review composited result, not just isolated token color values.                    |
| New raw radius declaration outside tokens            | `designTokens.test.ts` fails; assign the appropriate role.                         |

## 5. Good / Base / Bad Cases

Good: moving the split handle narrows a Models detail while the window remains
wide; the form stacks without changing navigation or save semantics.
Base: Firefox/Safari retain ordinary frost even when the library's Chromium
backdrop refraction is unavailable. Bad: a GPU screenshot of the full account
page becomes a background texture, or modal text is made translucent to fake
glass.

## 6. Tests Required

- `tests/v2/shared/designTokens.test.ts` uses PostCSS to reject scattered radius
  literals; do not invent a CSS parser or skip component files.
- `tests/v2-browser/materials-responsive.spec.ts`: seven page surfaces, actual
  composited text samples, axe contrast/label checks, critical input boundaries,
  a 320px detail independent of viewport width, 760px boundary sides, a 616px
  viewport as horizontal 200%-zoom pressure, and forced-color/transparency
  fallbacks. Existing four-desktop-size tests remain mandatory.
- `support/visual.ts` samples finite raster backgrounds with glyph paint hidden
  but unchanged layout. It supplements axe's incomplete gradient/filter cases;
  it is not a replacement accessibility engine or blanket WCAG certification.
- Production bundle smoke/chunk budgets and final motion+material performance
  must still pass. A dev-only screenshot does not prove native WebView/GPU
  behavior, every long document or every contrast pair.

## 7. Wrong vs Correct

Wrong: `border-radius:13px` in one account card and `15px` in the same-role
card elsewhere; `opacity:.7` on the entire modal; `overflow:hidden` on a form
to silence overflow; window-only breakpoints after adding resizable panes.

Correct: choose the shared role, render material as a decorative sibling of
crisp content, wrap real text, and respond to the detail container's width.
