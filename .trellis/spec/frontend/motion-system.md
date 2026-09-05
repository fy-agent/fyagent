# Motion, Press Feedback and Dialog Presence

## 1. Scope / Trigger

Read before changing V2 press gestures, modal origin/exit, conditional dialog
sessions, live motion preferences, notification presentation or transition
tokens. Geometry and navigation authority remain with their existing owners;
animation never controls whether a native operation succeeded.

Owners are `shared/ui/motion.ts`, `Button.tsx`, `usePressFeedback.ts`,
`Dialog.tsx`, `dialogOrigin.ts`, `useDialogState.ts`, `useMediaQuery.ts`, `ToastViewport.tsx`, and
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
})
ConfirmDialog({ open, title, description, pending?, onConfirm, onCancel, originRef? })
useDialogState<T>(initial?: T | null)
  // -> [value, stable React setter, fresh-session key]
dialogOriginGeometry(source: HTMLElement | null, destination: DOMRect)
  // -> { x, y, scaleX, scaleY, sourced }
useMediaQuery(query: string, fallback?: boolean): boolean
useReducedMotion(): boolean
motionDuration(role: "press" | "dialog-enter" | "dialog-exit" | "content" | "toast"): number
ToastViewport({ messages: readonly ToastMessage[] })
```

`motion.ts` is the sole direct Motion import owner. It exports the selection
spring, press recovery spring, bounded press scale and common surface curve.
CSS owns named duration tokens; `motionDuration` reads milliseconds and returns
seconds at the interaction boundary. Missing/invalid durations mean no travel,
not a guessed delay. Do not mix stiffness/damping/mass with duration/bounce
within the same spring definition.

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
- Selection/collapse use the shared selection spring. Tooltips/popovers use
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
- Only `.fy-dialog-material` maps full source position and scale. Form/text
  remains a separate foreground, using a small translation/fade rather than
  stretching a whole credential form into a button. Do not copy DOM, labels,
  pixels, inputs or credentials to a transition store.
- Radix retains modal, focus and scroll ownership until the decorative exit
  completes. The business `open` state is already false; exiting form/action
  DOM disappears immediately and cannot submit again. The final measured
  material rectangle may remain briefly, not an editable stale form.
- Start from the committed Content node: Radix Portal can mount after its
  parent's initial layout effect. Immediate/zero-layout completion crosses
  one microtask commit boundary before `safeToRemove`, because Motion records
  exiting keys in a parent layout effect. This is not a fixed animation delay.
- Cancel superseded generations on reopen/cleanup. Resize settles existing
  geometry without stretching stale coordinates. A hidden persistent route
  removes the portal immediately instead of animating into another page.
- A conditionally mounted dialog owner must be under `AnimatePresence`, with
  nested Dialog propagation enabled. Use a fresh session key for a reopened
  editor; retaining an old exiting component must not resurrect discarded
  field values. Permanent controlled dialogs retain only their existing,
  explicitly owned reset/recovery policy.
- `data-motion-phase` reports open/exit and `data-motion-settled="true"`
  identifies a settled open surface for geometry/contrast evidence. Neither
  attribute is a native success or security signal.
- Enhanced glass runs at rest. During motion the same backing's standard
  frosted styling remains, avoiding expensive source copying or a second
  rendering engine.

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

| Condition                                             | Required result                                                          |
| ----------------------------------------------------- | ------------------------------------------------------------------------ |
| Source control opens a dialog after async work        | Use that explicit original source, not whichever element is now focused. |
| Source moved, vanished or scrolled out before close   | Re-measure; return to the current valid box or use neutral exit.         |
| Close occurs while entering                           | Cancel superseded animations; remove form/actions and finish one exit.   |
| Same conditional editor is reopened                   | Fresh session key; no old draft/secret resurrection.                     |
| System reduced-motion changes during travel           | Settle current visuals and release any completed exit.                   |
| Portal commits after parent mount                     | Committed node starts the animation; no silent skipped entrance.         |
| Zero-duration exit                                    | Complete after presence bookkeeping; do not leave a focus/scroll lock.   |
| Right click, secondary touch, disabled/hidden control | No duplicate action or new press admission.                              |
| Another modal opens during old focus return           | Never focus outside the newer modal.                                     |
| Navigation occurs during a transition                 | Preserve URL/selection authority and hidden-route query isolation.       |

## 5. Good / Base / Bad Cases

Good: an account action passes one source ref through its view into Dialog;
closing removes the form, returns only the backing, then restores focus.
Base: an automatic status dialog has no actionable origin and uses neutral
fade/limited scale. Bad: infer origin from the last global click, animate a
screen capture of a password form, or wait for an animation before admitting
the actual business action.

## 6. Tests Required

- Shared origin/session tests cover exact geometry, clipping, hidden/removal,
  remeasurement and fresh conditional sessions.
  `PrimaryBlockerOrigin.test.tsx` verifies matching, one-shot consumption,
  programmatic neutral fallback and unchanged blocked navigation.
- Dialog tests retain third-round keyboard/focus safeguards and zero-duration
  unmount. Tests must verify actual modal/scroll cleanup, not only callbacks.
- Browser motion tests sample material transforms, verify foreground scale,
  press limits and unchanged neighbour boxes, and exercise mouse, Enter,
  Space, touch, reduced motion, invalid sources and interrupted exits.
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
