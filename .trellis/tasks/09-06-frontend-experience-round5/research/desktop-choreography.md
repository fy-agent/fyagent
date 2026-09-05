# Desktop source-to-surface choreography

This records the required behavior and inspected timing, not a local filesystem
dependency. Only desktop presentation behavior is in scope.

## Spatial profile

Common presentation curve: `cubic-bezier(.32,.72,0,1)`.
Use this for shared spatial movement, while keeping role-specific durations.
It is not the only curve used for every reference control: press recovery,
opacity and indefinite progress have distinct purposes.

| Track                | Open                                                  | Close                                                                |
| -------------------- | ----------------------------------------------------- | -------------------------------------------------------------------- |
| Overall shell        | 420ms, fluid curve                                    | 360ms total; geometry starts16ms later and runs344ms                 |
| Source visual        | Fades over first100ms                                 | Returns at270ms over90ms                                             |
| Source material      | Holds to14%, fades out by42%                          | Starts return at18%, reaches full at88%                              |
| Destination material | Starts at10%, reaches full at52%                      | Holds to12%, fades out by78%                                         |
| Real content handoff | Begins252ms, lasts168ms, overlaps last40% of geometry | Gives way over80ms while material return starts                      |
| Final ownership      | Real final-layout window, proxy removed               | Real source visible before proxy removal; focus returned after close |
| Backdrop             | Separate180ms enter opacity                           | Separate160ms exit opacity                                           |

The observed mechanism keeps the real dialog at final layout, uses an inert
fixed material proxy for position/size/corners and crossfades source material,
destination material and final content. Width/height are isolated to a small
subtree, not propagated through the form. A single transform owner prevents
gesture and layout projection fighting.

## Adaptation boundaries

- Reuse browser interpolation and existing Motion/Radix. Do not import another
  application's presentation store or upgrade this repository toReact19/Motion13.
- The inspected implementation includes a detached visual clone and frozen
  source geometry. Do NOT copy those contracts wholesale: FyAgent forbids
  arbitrary DOM/credential snapshots and returns neutrally when a source is
  disconnected, hidden, clipped or offscreen.
- Retain only explicit source element references and allowlisted material/
  geometry numbers; original source feedback must be coordinated, not replaced
  by a guessed clone or duplicate actionable control.
- Preserve immediate business close/cancel and sensitive input clearing.
  Non-sensitive original presentation can fade for at most80ms only after it
  becomes inert, hidden from accessibility and action-blocked. No copied form
  pixels, stored draft snapshots or deferred native cancellation. Update the
  prior all-body-immediate-removal contract explicitly with equivalent safety
  assertions plus the required visible content exit.
- If source navigation is blocked, use existing one-shot destination-matched
  origin. Keyboard/history/automatic dialogs without a valid origin stay neutral.
- The actual desktop material implementation is CSS tint, rim, shadow and
  modest backdrop blur, with moving large presentations disabling expensive
  backdrop sampling. It is NOT proof that a full-window live refraction shader
  is necessary or inexpensive.

## Why the previous transition can feel too abrupt

Both old and desired shell timing are420ms. Ideal normalized displacement at
10% elapsed is49.4% for`.16,1,.3,1` versus27.0% for`.32,.72,0,1`.
Old content starts near92ms; the desired handoff starts252ms. Therefore merely
lengthening total duration cannot reproduce the button→shell→content sequence.
These values are calculated from cubic-bezier polynomials, not visual/FPS tests.

## Assertions required during implementation

Measure first-frame source geometry/corners and press visibility, intermediate
material/foreground opacity, monotonic uninterrupted trajectory and final
ownership. Verify the forward and reverse sequence, rapid reversal from current
visual values, destruction of old editor sessions and cancellation cleanup.
Use paused keyframe assertions PLUS real-time browser frame sampling; tests must
not disable motion globally or mistake a final-position assertion for continuity.
