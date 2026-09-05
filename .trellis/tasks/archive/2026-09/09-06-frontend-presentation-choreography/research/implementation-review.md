# Implementation and regression review

## Closed behavior gaps

The source/target timeline is a scoped composition of native animation tracks,
not a new animation engine. The isolated material box interpolates corners and
geometry; the actual form stays unscaled. The first80ms leaves the real press
visible, content begins252ms into420ms, and360ms return overlaps source material
with content retirement. Existing Motion owns press/small controls and Radix
still owns modality/focus/scroll. No runtime dependency was added.

Production CSS unit normalization was the most significant factual defect:
the minifier emitted `.42s`, while the old reader produced0.00042seconds.
Unit tests now prove source/optimized equivalence, invalid-value rejection and
CSS/JS curve alignment. The production smoke checks real elapsed/native timing.

The browser source audit found a36px trigger whose top edge was0.5px outside
its overflow ancestor after native focus/scroll. The previous strict boundary
incorrectly chose neutral despite an intact visible control. One physical-pixel
tolerance is now explicit, with a negative3px-clipping test. Truly hidden,
deleted or scrolled-away sources remain neutral. The empty-state MCP Add button
also now forwards the same explicit origin as the header Add button.

## Preserved safety

The default body clears on close. Only pre-session login chooser opts into an
inert/aria-hidden original-content fade, capped80ms even if shell tokens change.
All action footers clear immediately. Session/device codes and MCP inputs do not
opt in. Native cancellation and business effects never wait for an animation.
Mid-entry Escape and exit-time reopen preserve current geometry/opacity; the
existing fresh conditional session and old-focus guard remain in force.

Native-track unit tests deliberately fail a later layer after earlier tracks
started; all earlier handles are cancelled and finished-promise rejections are
handled. Cancellation is not reported as completion. Only allowlisted CSS
material is sampled; no source text, form values, cloned DOM or URL resource.

## Observed test issues and repairs

- Empty-layout unit mounting scheduled a needless settle-state microtask after
  the test. The production owner now avoids setting visual completion for a
  zero-sized surface; targeted tests then passed without act warnings.
- A keyboard test asserted Router-selected state immediately after Radix focus
  changed. It now awaits the exact selected-state and visible-region assertions,
  retaining focus checks, not a fixed sleep or looser requirement.
- Browser origin geometry and interruption tests pass with actual box/corner
  samples. Old scale-matrix assertions were migrated to actual geometry while
  retaining the separate foreground's identity transform requirement.
- The first valid-duration frame p95 was33.400000000001455ms. Six-decimal
  normalization removes sub-nanosecond subtraction error while keeping the
  original33.4ms budget, not adding a performance allowance.
- Concurrent unrelated compilation/browser activity caused a5000ms Models test
  timeout. No timeout or test was removed. A separate full V2 pass recorded
  86files/578tests passed; verify final source fingerprints before using it.

## Evidence limits

The original broken-duration modal samples are not a valid full-animation FPS
baseline. Report them as the defect evidence, then measure the corrected native
timeline. Headless frame intervals and CPU throttling do not certify minimum
macOS/Windows WebView or real GPU behavior. Full combined performance and gate
results are recorded at completion, not inferred from these targeted checks.
