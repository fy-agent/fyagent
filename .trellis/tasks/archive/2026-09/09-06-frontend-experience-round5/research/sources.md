# Primary research and reuse decision

Research checked2026-09-06. Public current documentation is not permission to
use APIs absent from the repository's installed versions. Product: React18.3.1,
framer-motion12.23.25, Radix Dialog1.1.15, liquid-glass0.1.1; verify lock again
before implementation. No runtime dependency was installed during planning.

## Motion and lifecycle

- https://motion.dev/docs/react-transitions
  Cubic-bezier tween curves differ from physical springs. Keep press physics
  separate; do not supply ignored/contradictory spring and tween parameters.
- https://motion.dev/docs/animate
  Existing animation/sequence controls can coordinate tracks without a new
  interpolator. Verify local12.x exports before using current13.x examples.
- https://motion.dev/docs/performance
  Transform/opacity are the safest compositing path. Isolated geometry changes
  can be reasonable for a small absolute subtree, but require actual measurement.
- https://www.radix-ui.com/primitives/docs/guides/animation
  Radix supports CSS and JS animation composition; preserve its presence,
  semantics and focus ownership rather than writing a second dialog manager.
- https://developer.mozilla.org/en-US/docs/Web/API/Animation/finished
- https://developer.mozilla.org/en-US/docs/Web/API/Animation/cancel
  Native completion/cancellation can drive lifecycle. Cancelling can reject
  `finished`; catch cancellation and cancel prior generations, not setTimeout
  with guessed completion. Avoid indefinite frame waits in hidden windows.

## Material alternatives

| Candidate                                     | Verified boundary                                                                                                                         | Decision                                                                                                                       |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Existing `@samasante/liquid-glass@0.1.1`      | MIT; installed README distinguishes ordinary material from copying/in-place/media modes; arbitrary live-backdrop bending is Chromium-only | Keep single adapter and compare a bounded rim enhancement against CSS baseline; no whole-window copy/media mode.               |
| `rdev/liquid-glass-react`                     | MIT; author explicitly notes missing displacement inSafari/Firefox; includes its own interaction/elasticity                               | Not preferred: replacing the adapter does not fixWebKit capability and introduces overlapping motion ownership.                |
| React Bits Glass Surface                      | Official component advertises distortion/lighting; same-family SVG effects need capability and source review                              | Comparison candidate, not assumed faster or more mature; no adoption without license/local compatibility and measured benefit. |
| React Bits Fluid Glass / general WebGL scenes | Separate scene/refraction path rather than a drop-in semantic dialog backing                                                              | Reject for this round: no need for another live canvas/scene to style business forms.                                          |
| Reusable CSS glass layers                     | Stable tint, modest blur, static rim/sheen and shadow, crisp children                                                                     | Base treatment for large surfaces; coordinate it with existing adapter, do not build a new optical engine.                     |

Sources:

- https://github.com/samasante/liquid-glass
- https://github.com/samasante/liquid-glass/blob/main/BROWSERS.md
- https://github.com/rdev/liquid-glass-react
- https://reactbits.dev/components/glass-surface
- https://reactbits.dev/components/fluid-glass
- https://aave.com/design/building-glass-for-the-web

Important distinction: the library browser matrix describes filtering the DOM
you supply. It does not mean arbitrary background refraction works inWebKit.
Copying a track or decoration is different from screenshotting a live account
form. Its authors also warn that large/manySVG surfaces areGPU-bound. No source
provides a benchmark proving any replacement faster forFyAgent.

## Material/readability principles

- https://developer.apple.com/design/human-interface-guidelines/materials
- https://developer.apple.com/videos/play/wwdc2025/219/
  Material supports separation/hierarchy; use it deliberately rather than
  applying optical effects to every content layer. Native guidance is design
  input, not a claim that WebView implements Apple's native material renderer.
- https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/backdrop-filter
  Blur depends on compositing and the backdrop root. Review ancestors and
  opacity/filter topology rather than assuming a larger blur creates glass.
- https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html
  Preserve4.5:1 ordinary readable text; paired foreground/background and actual
  blended states matter when moving to a lighter palette.

## Chosen direction

No new runtime engine. Brighter paired semantic palette + stable thin frost /
rim/sheens, with restricted existing-library optical enhancement only where it
has measured benefit. Reuse the verified desktop spatial curve and material/
content handoff through currentMotion/WAAPI andRadix. Do not infer quality from
star counts, install a heavy component because of its name, or copy a sensitive
DOM snapshot to imitate a demonstration.
