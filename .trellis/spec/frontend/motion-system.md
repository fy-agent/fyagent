# Motion, Press Feedback and Dialog Presence

## 1. Scope / Trigger

Read before changing V2 press gestures, modal origin/exit, conditional dialog
sessions, live motion preferences, notification presentation or transition
tokens. Geometry and navigation authority remain with their existing owners;
animation never controls whether a native operation succeeded.

Owners are `shared/ui/motion.ts`, `Button.tsx`, `usePressFeedback.ts`,
`Dialog.tsx`, `dialogPresentation.ts`, `dialogOrigin.ts`, `useDialogState.ts`, `useMediaQuery.ts`, `ToastViewport.tsx`, and
`app/styles/{tokens,motion,controls}.css`. Glass optics and readable backing
remain in [Surfaces and Container Response](./surfaces-responsive.md);
typography and focus-return rules remain in [Visual Language](./visual-language.md).

## 2. Signatures

```ts
interface DialogOriginRef { current: HTMLElement | null }

Button / GlassButton / IconButton / PressableButton({
  ...nativeButtonProps, dialogOriginRef?: DialogOriginRef,
})
Dialog({
  open, onOpenChange, title, description?, children?, actions?,
  size?: "standard" | "comfortable" | "wide",
  initialFocusRef?: RefObject<HTMLElement>, originRef?: DialogOriginRef,
  exitContent?: "clear" | "fade", // clear by default; reviewed non-credential content only
})
ConfirmDialog({ open, title, description, pending?, onConfirm, onCancel, originRef? })
useDialogState<T>(initial?: T | null)
  // -> [value, stable React setter, fresh-session key]
dialogOriginGeometry(source: HTMLElement | null, destination: DOMRect)
  // -> { x, y, scaleX, scaleY, sourced }
useMediaQuery(query: string, fallback?: boolean): boolean
useReducedMotion(): boolean
parseMotionDuration(value: string): number // seconds; invalid input -> 0
motionDuration(role: "press" | "dialog-enter" | "dialog-exit" | "content" | "toast"): number
fySelectionTransition // tween; shared spatial curve, not a spring
runDialogPresentation({ planes, source, windowNode, entering, first, duration })
  // duration is milliseconds; -> { finished, contentFinished, cancel }
ToastViewport({ messages: readonly ToastMessage[] })
```

`motion.ts` is the sole direct Motion import owner. It exports the selection
tween, press recovery spring, bounded press scale and shared `.32,.72,0,1`
spatial curve. `fySpatialEasing` derives the native CSS string from that tuple;
the CSS token must match and is checked in tests. CSS owns named duration
tokens; `parseMotionDuration` accepts one finite nonnegative `ms` or `s` value
and returns seconds. Optimizers may serialize `420ms` as `.42s`; `parseFloat`
followed by unconditional division by 1000 is prohibited. Missing, compound,
unitless, negative or nonfinite durations mean no travel, not a guessed delay.
Native WAAPI receives milliseconds only at its adapter boundary. Do not mix
stiffness/damping/mass with duration/bounce within a spring definition.

## 3. Contracts

### Press and transition roles

- Native buttons/links remain the semantic action owners. Motion's `press`
  filters non-primary pointers and supplies Enter feedback; native button
  Space adds visual feedback only. Never synthesize a second business click.
- `usePressFeedback` registers one gesture per host, uses live admission refs
  for disabled/hidden/reduced state and cancels animations on cleanup. A
  separate visual target allows a navigation label to compress without
  corrupting its measured SelectionLens host rectangle.
- Press target and hard visual limits come from `fyPressScale`; release uses
  `fyPressRecovery`. The accepted maximum is below 1.005. Transform changes
  must not change layout slots or move neighbouring controls. A quick click
  may finish a small dip visually, but the action is not delayed until rebound.
- Selection/collapse use `fySelectionTransition` (300ms tween) and the shared
  spatial curve. Do not retain a misleading spring alias. Tooltips/popovers use
  Radix CSS presence with their own transform origins. Content arrival does
  not delay route commit or keep an outgoing page interactive. Window resize
  is not slowed by a decorative transition.
- `ToastViewport` owns presentation only. FeatureProvider retains its timer,
  message state and cleanup; exiting messages stop accessibility announcements.
  Zero-duration and reduced-motion toasts appear without an invisible frame.

### Explicit modal origin and presentation

- A caller records its actual control before changing open state or awaiting
  work. `Button.dialogOriginRef` captures `event.currentTarget`; shared tabs
  may resolve their own exact semantic trigger. An asynchronous dialog keeps
  that original reference. No document-wide last-click cache or arbitrary
  `activeElement` guess supplies animation geometry.
- Guarded sidebar navigation carries one explicit destination-matched intent
  through the existing PrimaryBlocker context. `usePrimaryNavigationOrigin`
  records the actual owned link, the blocker consumes it once, and
  `usePrimaryBlockerOrigin` supplies the confirmation source. Unmatched or
  programmatic/history transitions remain neutral; blocker rules and route
  admission are unchanged. Do not promote this to a global last-click store.
- At entry and return, measure the referenced element. Disconnected, zero-size,
  hidden/inert, transparent, off-window, or clipped/scrolled-away sources use
  a neutral transition. Do not fly toward a different control with similar text.
  Bounds admit at most one device pixel of rounding tolerance, not a general
  allowance for genuinely clipped sources.
- Only `.fy-dialog-material` interpolates source position, width, height and
  four corner radii, inside a layout/style-contained decorative subtree. The
  actual Radix window and text keep final layout; do not scale-compress forms.
  Source/target material and foreground opacity have separate native tracks.
  Only allowlisted CSS material strings are sampled; resource URLs, DOM,
  labels, pixels, input values and credentials are not copied or retained.
- A sourced first entrance gives the real press up to 80ms of visual lead,
  inside the same 420ms presentation. Geometry then reaches its destination;
  foreground handoff runs from 252 to 420ms. This never delays the initiating
  business action. Exit lasts 360ms: foreground yields in at most 80ms,
  geometry starts after 16ms, and material returns to the real source across
  the final 90ms. Relative track timings follow retuned duration tokens, while
  optional content retention remains capped at 80ms.
- Radix retains modal, focus and scroll ownership until the decorative exit
  completes. Business close/cancel runs immediately and actions disappear.
  Default `exitContent="clear"` immediately removes the body, especially for
  credential/session/editor content. Only reviewed non-credential presentation
  may opt into `"fade"`: its original body stays inert, aria-hidden and event-
  blocked for at most 80ms, then is destroyed. No presentation copy is created.
  The pre-session login provider picker is the reviewed opt-in; live login
  sessions/device codes and MCP editors are not. Never widen this to preserve
  secrets for cosmetic continuity.
- Start from the committed Content node: Radix Portal can mount after its
  parent's initial layout effect. Immediate/zero-layout completion crosses
  one microtask commit boundary before `safeToRemove`, because Motion records
  exiting keys in a parent layout effect. This is not a fixed animation delay.
- During entrance, invisible controls are inert and event-blocked; focus stays
  on Radix Content until handoff completes, then moves to the requested cancel
  or first usable control only if focus still belongs to that root. Escape can
  cancel an unfinished entrance. Reduced/no-animation environments retain
  immediate access. Do not wait for animation to report native readiness.
- Cancel superseded generations on reopen/cleanup. Preserve current computed
  geometry/opacity before cancellation when reversing; do not reset to a
  fully open rectangle. Every already-started track is cancelled if a later
  track throws, and rejected finished promises are handled. Resize settles existing
  geometry without stretching stale coordinates. A hidden persistent route
  removes the portal immediately instead of animating into another page;
  document visibility loss also settles rather than waiting on suspended frames.
- A conditionally mounted dialog owner must be under `AnimatePresence`, with
  nested Dialog propagation enabled. Use a fresh session key for a reopened
  editor; retaining an old exiting component must not resurrect discarded
  field values. Permanent controlled dialogs retain only their existing,
  explicitly owned reset/recovery policy.
- `data-motion-phase` reports open/exit and `data-motion-settled="true"`
  identifies a settled open surface for geometry/contrast evidence. Neither
  attribute is a native success or security signal.
- The same CSS glass backing remains through motion and rest; enhancement
  changes rim emphasis only. Never swap in a displacement renderer at the last
  frame or copy page/form content into the optical layer.

### Accessibility and failure behavior

- Live system reduced-motion changes settle travel/rebound immediately; use
  the existing media subscription owner, not periodic polling. CSS and JS
  must agree. Do not defer native readiness behind animation frames.
- Preserve cancel-first focus, selected-tab restoration after rejected
  navigation, and protection against an old close frame stealing focus from
  a newer modal. Focus restoration uses `preventScroll` and rejects hidden,
  disconnected or disabled targets.
- Engine failure settles the surface with a bounded diagnostic. It must not
  strand a modal lock, fabricate action success or expose raw business errors.

## 4. Validation & Error Matrix

| Condition                                             | Required result                                                                |
| ----------------------------------------------------- | ------------------------------------------------------------------------------ |
| Source control opens a dialog after async work        | Use that explicit original source, not whichever element is now focused.       |
| Source moved, vanished or scrolled out before close   | Re-measure; return to the current valid box or use neutral exit.               |
| Close occurs while entering                           | Freeze current geometry, revoke interaction/secrets and finish one exit.       |
| Explicit non-credential fade exit                     | Inert/aria-hidden body retires within 80ms; action DOM disappears immediately. |
| CSS optimizer emits seconds instead of milliseconds   | Preserve physical duration; production timing test must still see 420ms.       |
| A later native animation track throws                 | Cancel all started tracks, handle rejection and settle safely.                 |
| Same conditional editor is reopened                   | Fresh session key; no old draft/secret resurrection.                           |
| System reduced-motion changes during travel           | Settle current visuals and release any completed exit.                         |
| Portal commits after parent mount                     | Committed node starts the animation; no silent skipped entrance.               |
| Zero-duration exit                                    | Complete after presence bookkeeping; do not leave a focus/scroll lock.         |
| Right click, secondary touch, disabled/hidden control | No duplicate action or new press admission.                                    |
| Another modal opens during old focus return           | Never focus outside the newer modal.                                           |
| Navigation occurs during a transition                 | Preserve URL/selection authority and hidden-route query isolation.             |

## 5. Good / Base / Bad Cases

Good: an account action passes one source ref through its view into Dialog;
closing removes the form, returns only the backing, then restores focus.
Base: an automatic status dialog has no actionable origin and uses neutral
fade/limited geometry. Bad: infer origin from the last global click, animate a
screen capture of a password form, or wait for an animation before admitting
the actual business action.

## 6. Tests Required

- Shared origin/session tests cover exact geometry, clipping, hidden/removal,
  remeasurement and fresh conditional sessions.
  `PrimaryBlockerOrigin.test.tsx` verifies matching, one-shot consumption,
  programmatic neutral fallback and unchanged blocked navigation.
- Dialog tests retain third-round keyboard/focus safeguards and zero-duration
  unmount. Tests must verify actual modal/scroll cleanup, not only callbacks.
- `motionDuration.test.ts` covers ms/s/exponents and invalid input;
  `dialogPresentation.test.ts` checks track endpoints, no content/resource
  copying, cancellation, partial-start failures and the 80ms exit cap.
- Browser motion tests sample material geometry, verify unscaled foreground,
  press limits and unchanged neighbour boxes, and exercise mouse, Enter,
  Space, touch, reduced motion, invalid sources and interrupted exits.
- `presentation-choreography.spec.ts` pauses real native tracks at source,
  handoff and return frames. `presentation-performance.spec.ts` runs the actual
  production build, verifies physical time units, separates cold entry from
  20 warm open/close cycles at 1x/4x CPU cost, and checks cleanup. Normal warm
  frame interval p95 target is 33.4ms. Do not disable animations to meet it.
- Re-run existing form/confirmation security tests, all four browser viewports,
  production boot and navigation performance, route chunk checks and full gates.
  Compare latency to the same method's baseline; animation end is not the
  definition of input/route readiness.
- Contrast screenshots use settled geometry. Sampling browser fixtures is
  not proof of minimum native WebView/GPU behavior or all real-user data.

## 7. Wrong vs Correct

```tsx
// Wrong: last global click + a copied animated credential form.
// Correct: capture the actual owned control, forward only its element ref.
const originRef = useRef<HTMLElement | null>(null);
<Button dialogOriginRef={originRef} onClick={openEditor}>Edit</Button>
<Dialog open={open} originRef={originRef} onOpenChange={setOpen} title="Edit">
  <FeatureOwnedForm />
</Dialog>
```

Wrong: call `safeToRemove` synchronously from the child's first exit layout
effect or leave closed editors mounted under an unchanged reusable key.
Correct: respect Motion's presence registration order and use fresh keys for
conditional editor sessions; Radix remains the sole modal/focus owner.
