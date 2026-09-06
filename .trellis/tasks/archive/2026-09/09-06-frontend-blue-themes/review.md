# Blue themes implementation review

## Scope and decisions

Paired light/mist-blue semantic tokens, one top-bar switch and the existing
native theme command. No backend changes, new dependency, cloned DOM, renderer
context rerender or router reconstruction. The desktop reference was inspected
read-only; only the effect/lifecycle is recorded, not a machine dependency.

Native View Transitions and WAAPI supply snapshot/interpolation, while the
adopted Motion owner supplies the shared curve and unit-safe duration. Review:
https://developer.mozilla.org/en-US/docs/Web/API/ViewTransition
https://developer.mozilla.org/en-US/docs/Web/API/Document/startViewTransition
https://motion.dev/docs/react-layout-animations

## Defects found and corrected during review

- A dark selected surface compounded the light-only sheen and weakened text.
  Shared sheen roles and opaque/tinted control backing were paired with dark
  tokens; ordinary and selected text still require 4.5:1.
- Raster tests sampled ellipsized Range tails outside the clipping element.
  Measured example: a 187px text range inside a 118px clip. Intersect clipping
  ancestors before sampling; an independent adversarial fixture proves the fix.
- Filled primary buttons are identified by silhouette versus surrounding
  material, not a same-colored border versus their own fill. Outlined controls
  retain both-side checks; the 3:1 requirement is unchanged.
- The theme marker precedes native capture readiness. Tests now await the
  actual pseudo-element animation before inspecting keyframes rather than
  assuming the marker proves an animation exists.
- Preserve system preferences using the existing media store. External storage
  supersedes pending local capture without committing that obsolete preference.
- Functional browser discovery now excludes all purpose-named performance
  suites; only serial production runs are performance evidence.

## Verification

Unified type/lint passed; 173 unit files, 1,546 passed and 1 existing skipped.
Full browser run passed 304 cases; that first run also discovered eight theme
profiling cases on the development server. Those eight are not used as performance
evidence. The corrected functional configuration and final gate are rerun below.

Three independent serial production runs, one cold and twenty warm toggles per
CPU condition, 1232x700:

| Run | CPU | preparation p95 ms | early / middle / late frame p95 ms |
| --- | --- | ------------------ | ---------------------------------- |
| 1   | 1x  | 24.1               | 16.7 / 16.8 / 16.7                 |
| 1   | 4x  | 34.9               | 16.7 / 16.8 / 16.7                 |
| 2   | 1x  | 23.7               | 16.8 / 16.7 / 16.8                 |
| 2   | 4x  | 35.1               | 16.7 / 16.8 / 16.8                 |
| 3   | 1x  | 34.1               | 16.8 / 16.7 / 16.7                 |
| 3   | 4x  | 34.9               | 16.7 / 16.7 / 16.7                 |

Each third has 220 warm samples. Logs: `/tmp/fyagent-round6-theme-perf-{1,2,3}.log`.
The 33.4ms budget was not increased. These are controlled browser measurements,
not proof that the reference application's reported bug or every native GPU is
fixed. Exact native IPC was mocked; real credentials, signing and minimum-version
WebViews are not exercised. Palette-level screenshot inspection does not replace
the focused full-resolution geometry/contrast assertions.

Final isolated prearchive passed, exit 0: unified 1,546 unit tests (1 existing
skip), native 3,495 (6 existing ignored), exact task contexts and release gates.
The preceding concurrent browser/full-check run timed out one unchanged
five-second process-injection rejection test. That failure remains recorded;
no timeout/assertion was relaxed, and the entire gate was rerun alone.
Corrected functional discovery passed 301 browser cases. The subsequent paired
light/dark painted-glyph and live-preference/geometry focus run passed all 20
cases across four Chromium sizes and WebKit. Root entry and task/SPEC changes
are committed before archive; final integrated performance is owned by the parent.
