# Shared Motion, Press Feedback and Preferences

## 1. Scope / Trigger

Read before changing press, selection, disclosure, notification or shared timing.
`shared/ui/motion.ts` is the sole direct Motion import owner; `Button.tsx`,
`usePressFeedback.ts`, `Collapsible.tsx`, `useMediaQuery.ts` and
`ToastViewport.tsx` compose it. CSS roles live in `app/styles/tokens.css`.

Modal source capture, presence, cancellation, focus and content resize now have
one focused owner: [Dialog Lifecycle](./dialog-lifecycle.md). Selection geometry
belongs to [Window Shell](./window-shell.md), theme reveal to
[Appearance](./appearance.md), and readable backing to [Surfaces](./surfaces-responsive.md).
These are code-spec owners, not additional runtime layers.

## 2. Signatures

```ts
useMediaQuery(query: string, fallback?: boolean): boolean
useReducedMotion(): boolean
parseMotionDuration(value: string): number // seconds; invalid -> 0
motionDuration(role: "press" | "dialog-enter" | "dialog-exit" | "dialog-resize" | "content" | "toast" | "theme"): number
fySelectionTransition // 300ms tween, not a spring
fyPressRecovery       // bounded recovery spring
fyPressScale          // target .96; hard visual maximum 1.004
ToastViewport({ messages: readonly ToastMessage[] })
```

`fySpatialEasing` derives a native CSS curve from the shared `.32,.72,0,1`
tuple; the CSS easing token must match. CSS owns duration tokens. The parser
accepts one finite, nonnegative `ms` or `s` value and returns seconds; native
WAAPI converts to milliseconds at its adapter, exactly once. Unitless,
compound, negative, nonfinite or missing values mean no travel, not a guessed
delay. Do not mix stiffness/damping/mass with duration/bounce in one spring.

## 3. Contracts

- Buttons/links own the business action. Motion `press` filters non-primary
  pointers and supplies Enter feedback; native Space adds visual feedback only.
  Never synthesize another click or wait for rebound before executing a click.
- One `usePressFeedback` registration per host uses live disabled/hidden/reduced
  admission refs and cancels its effects on cleanup. A fast click must visibly
  dip/recover, not only a long hold. Scale must not change layout or neighbours.
- Positioned or measured hosts use `pressVisualRef` for the existing inner
  visual. Secret/search controls preserve centering; selection hosts preserve
  lens geometry. Feature buttons reuse PressableButton, not copied gestures.
- Style subscriptions are independent: unmounting one control must not cancel
  another. Keep the upstream cleanup regression; no node_modules patch or local
  interpolation engine substitutes for the adopted dependency.
- Selection/disclosure use the shared spatial tween. Tooltips/popovers retain
  Radix CSS presence and their own transform origins. Persistent page/tab
  arrival changes opacity only; ancestor position tweens must not displace
  measured hosts. Route commit and native readiness do not wait for animation.
- Initial/re-shown lenses adopt real geometry rather than growing from zero.
  Real selection changes travel; layout observations correct geometry without
  restarting motion on every frame of a sibling disclosure. Window resizing
  is not slowed by a decorative transition.
- Collapsible uses Motion `height: open ? "auto" : 0`, `initial={false}` and
  the shared transition. Do not restore a scrollHeight cache, generation loop
  or final Promise write. Radix retains content; closed content immediately
  becomes inert/aria-hidden. Read-only model disclosures share ModelsExistingSection.
- Live reduced-motion preferences settle travel/rebound immediately through
  the shared media subscription, not polling. CSS and JS must agree. The real
  control stays usable even without animation APIs.
- ToastViewport presents only: FeatureProvider owns timers/state/cleanup.
  Exiting messages stop announcements; zero-duration/reduced-motion messages
  appear without an invisible frame. No toast proves native success by itself.

## 4. Validation & Error Matrix

| Condition                                   | Required result                                                 |
| ------------------------------------------- | --------------------------------------------------------------- |
| Optimizer changes `420ms` to `.42s`         | Same physical duration, tested in production assets.            |
| Disabled/hidden/right-click/secondary touch | No new press admission or duplicate action.                     |
| Positioned control is pressed               | Only its visual scales; centering and neighbours remain stable. |
| One control unmounts                        | Other style subscriptions still update.                         |
| Live preference changes mid-motion          | Settle promptly, preserve action semantics and cleanup.         |
| A kept-alive page returns                   | No zero-size lens replay or ancestor position jump.             |
| Motion unavailable or zero duration         | Usable final state; no invisible toast or stranded modal.       |

## 5. Good / Base / Bad Cases

Good: quick pointer/Enter/Space activation produces one action and a bounded
dip/rebound without moving adjacent controls. Base: reduced motion renders
the same semantic control without spatial travel. Bad: lengthen motion to hide
route CPU work, duplicate click handling or scale the whole credential form.

## 6. Tests Required

`motionDuration.test.ts` covers both units, exponents and invalid values;
production timing checks preserve optimized 420ms entry/360ms exit.
`pressIsolation.test.tsx` uses two real style subscriptions. Browser press tests
sample fractional bounding boxes for quick pointer/Enter/Space/touch, clipping,
disabled/hidden state, adjacent geometry and live preferences. Integer
offsetWidth cannot measure a sub-percent rebound budget.

`state-motion.spec.ts` proves declarative disclosure and lens revisit versus
actual selection travel. Dialog tests and 320ms same-session resize belong to
[Dialog Lifecycle](./dialog-lifecycle.md). Complete browser, production boot,
route chunk and serial performance gates remain in [Quality](./quality-guidelines.md);
sampled browser frames are not all-device GPU or native WebView evidence.

## 7. Wrong vs Correct

Wrong: `parseFloat(cssTime) / 1000`, a misleading spring alias for a tween, or
copying handlers into each page. Correct: parse the unit through the shared
owner and compose the semantic Button/Collapsible with its owned visual target.
Modal callers must additionally follow the explicit source/session contract in
[Dialog Lifecycle](./dialog-lifecycle.md), rather than infer a last global click.
