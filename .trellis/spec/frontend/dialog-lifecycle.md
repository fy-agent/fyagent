# Dialog Origin, Presence and Content Resize

## 1. Scope / Trigger

Read before changing modal origins, conditional sessions, presence, content
resize or focus/scroll release. Owners under `shared/ui` are `Dialog.tsx`,
`dialogPresentation.ts`, `useDialogResize.ts`, `dialogOrigin.ts` and
`useDialogState.ts`; `app/styles/controls.css` supplies the layered surface.
Global tokens/press/media preferences belong to [Motion](./motion-system.md),
material to [Surfaces](./surfaces-responsive.md), typography to
[Visual Language](./visual-language.md). Animation never grants native authority.

## 2. Signatures

```ts
interface DialogOriginRef {
  current: HTMLElement | null;
  snapshot?: DialogOriginSnapshot; // one-use transient opening geometry/material only
  returnTarget?: HTMLElement | null; // explicitly supplied persistent control
}

Button / GlassButton / IconButton / PressableButton({
  ...nativeButtonProps, dialogOriginRef?: DialogOriginRef,
  dialogReturnRef?: RefObject<HTMLElement>,
  pressVisualRef?: RefObject<HTMLElement>,
})
Dialog({
  open, onOpenChange, title, description?, children?, actions?,
  size?: "standard" | "comfortable" | "wide",
  initialFocusRef?: RefObject<HTMLElement>, originRef: DialogOriginRef | undefined,
  presentationKey?: string | number, // content stage, never editor/session identity
  exitContent?: "clear" | "fade", // clear by default; reviewed non-credential content only
})
ConfirmDialog({ open, title, description, pending?, onConfirm, onCancel, originRef? })
  // onConfirm: MouseEventHandler<HTMLButtonElement>; native click, no synthetic dispatch
captureDialogOrigin(ref: DialogOriginRef, source: HTMLElement, returnTarget?: HTMLElement | null): void
useDialogState<T>(initial?: T | null)
  // -> [value, stable React setter, fresh-session key]
dialogOriginGeometry(source: HTMLElement | null, destination: DOMRect)
  // -> { x, y, scaleX, scaleY, sourced }
runDialogPresentation({ planes, source, windowNode, entering, first, duration, capturedOrigin? })
  // duration is milliseconds; -> { finished, contentFinished, retarget, cancel }
runDialogResize({ windowNode, contentNode, from, target, duration })
  // -> { finished, cancel(freeze?: boolean) }; same session, no scale/copy
```

Signatures above summarize the typed source interfaces. Use the shared Motion
duration parser; WAAPI accepts milliseconds only at this presentation boundary.

## 3. Contracts

### Explicit modal origin and presentation

- A caller records its actual control before changing open state or awaiting
  work. `Button.dialogOriginRef` captures `event.currentTarget`; shared tabs
  may resolve their own exact semantic trigger. An asynchronous dialog keeps
  that original reference. No document-wide last-click cache or arbitrary
  `activeElement` guess supplies animation geometry.
- Every production Dialog/wrapper call supplies `originRef` explicitly; automatic
  cases pass undefined deliberately, not by an omitted prop. The TypeScript/AST
  coverage check complements physical frame tests; a trigger data attribute
  alone cannot prove that its animation was not immediately cancelled.
- Transient menu or asynchronous-reflow controls may call `captureDialogOrigin` with their explicitly
  owned persistent return control (`Button.dialogReturnRef`). Only a finite
  visible rectangle, four corner radii and allowlisted non-resource CSS material
  are retained. No DOM clone, text, identity label, credential value or image is
  copied. The first matching opening consumes the capture once. Exits remeasure
  the real source or explicit return target; invalid targets remain neutral.
  Revalidate captured finite geometry against the current viewport at consumption:
  an async picker/preview can outlive a resize, so old snapshot admission is not
  permanent permission to fly outside the visible window.
- Directory install-target pickers stay mounted with `open={pickingTarget !== null}`.
  Confirm starts the native job and sets open false in the same click; do not
  wait for `run()` to settle. `dialogReturnRef` must point at a host that remains
  connected while the slot swaps from the trigger button to busy status. Putting
  the return ref on the button that unmounts on busy yields a disconnected
  source and a neutral fade, which hides the card progress the user needs.
- Switch uses the existing PressableButton asChild capture before Radix's
  checked-change handler. A modal-producing switch opts into bounded click
  geometry with its own element as the return anchor: async status feedback can
  move the live control outside a compact viewport before the dialog opens.
  This does not relax capture or return bounds. WorkBuddy assignment/trust and bulk controls carry
  the actual initiating ref through async work. A chained confirmation captures
  its own confirm control before it unmounts, with the original persistent
  control as its return anchor, rather than attributing the next dialog to an
  unrelated last click.
- MCP discovery install and editor save explicitly hand their final submit
  source into the follow-up trust notice. `InstallTargetDialog.onConfirm`
  receives `(target, MouseEvent<HTMLButtonElement>)`; MCP InstallDialog's
  `onInstall` receives `(values, apps, event)`. Business owners capture that
  control with the known catalog/editor return anchor before awaiting writes.
  The notice consumes an operation-owned origin copy, not an unrelated page
  ref. Ordinary callbacks may ignore the event; no duplicate click or new
  native authorization is introduced. Multi-step install dialogs supply
  `presentationKey={step}` just as the login dialog does.
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
- A closed, empty Dialog does not enroll in its ancestor's propagation barrier.
  Keep participation while an actual layer opens/exits and release it from
  onExitComplete. Otherwise a never-opened sibling confirmation can retain an
  invisible, completed modal and its scroll lock forever. Prop-state adjustment
  is guarded before children render, not a cascading synchronization effect.
- `data-motion-phase` reports open/exit and `data-motion-settled="true"`
  identifies a settled open surface for geometry/contrast evidence. Neither
  attribute is a native success or security signal.
- The same CSS glass backing remains through motion and rest; enhancement
  changes rim emphasis only. Never swap in a displacement renderer at the last
  frame or copy page/form content into the optical layer.

### Same-session content changes

- Login `presentationKey` identifies the step/session stage; a changed stage
  or dialog size variant does not reopen/remount the session. `useDialogResize`
  captures actual current size, measures the new natural target and invokes
  `runDialogResize` for 320ms from the shared token.
- Unlike source opening, the fixed dialog's actual width/height change here.
  Foreground text is never scale-projected. Only body opacity moves from 0.5 to
  1; actions stay visible, usable and within the window while their own press
  feedback plays. No outgoing input/value/DOM snapshot is retained.
- Observe intrinsic header/body/action rows, not the animated root itself.
  Notifications caused by the current size animation update observations but
  do not start another animation. On completion reconcile natural content once.
  Explicit stage changes work even without ResizeObserver.
- During source entrance, intrinsic async content changes retarget the existing
  geometry track instead of invoking the viewport-settle shortcut. Rebase local
  coordinates against old/new centered window positions before paint; preserve
  the remaining entry deadline and independent foreground/overlay handoff.
  Do not cancel/recreate all tracks, restart 420ms, delay native preview delivery
  or add a minimum loading timer. Real viewport changes still settle promptly.
- A settled dialog's non-semantic width change is a CSS viewport clamp, not a
  content-height transition. `useDialogResize` settles it without tweening back
  to the old width. WebKit may deliver this observation before the window resize
  event; writing the old width inside that delivery would resize the observed
  body again and report an observer loop. Explicit size/presentationKey changes
  and intrinsic height changes still animate; entry retargeting is unchanged.
- Reversal captures the intermediate rect before cancelling old effects.
  Closing freezes current size for the existing return track while immediately
  revoking actions and sensitive content. Resize, reduced motion and document
  hiding settle; old completions cannot alter a newer session.
- `data-content-motion="resizing"` and absence of `data-motion-settled` expose
  presentation state for evidence only. They confer no business write authority.

### Accessibility and failure behavior

- Live system reduced-motion changes settle travel/rebound immediately; use
  the existing media subscription owner, not periodic polling. CSS and JS
  must agree. Do not defer native readiness behind animation frames.
- Preserve cancel-first focus, selected-tab restoration after rejected
  navigation, and protection against an old close frame stealing focus from
  a newer modal. Focus restoration uses `preventScroll` and rejects hidden,
  disconnected or disabled targets.
- A queued close-focus callback cannot override an explicit newer editor focus,
  even when there is no newer modal. Keep the original focus-at-close identity
  and check before applying deferred restoration.
- Engine failure settles the surface with a bounded diagnostic. It must not
  strand a modal lock, fabricate action success or expose raw business errors.

## 4. Validation & Error Matrix

| Condition                                             | Required result                                                                   |
| ----------------------------------------------------- | --------------------------------------------------------------------------------- |
| Source control opens a dialog after async work        | Use that explicit original source, not whichever element is now focused.          |
| Source moved, vanished or scrolled out before close   | Re-measure; return to the current valid box or use neutral exit.                  |
| Confirm starts work that replaces the trigger with status | Keep a connected `dialogReturnRef` host; fly back to that host, not a fade.   |
| Close occurs while entering                           | Freeze current geometry, revoke interaction/secrets and finish one exit.          |
| Explicit non-credential fade exit                     | Inert/aria-hidden body retires within 80ms; action DOM disappears immediately.    |
| CSS optimizer emits seconds instead of milliseconds   | Preserve physical duration; production timing test must still see 420ms.          |
| A later native animation track throws                 | Cancel all started tracks, handle rejection and settle safely.                    |
| Same conditional editor is reopened                   | Fresh session key; no old draft/secret resurrection.                              |
| System reduced-motion changes during travel           | Settle current visuals and release any completed exit.                            |
| Portal commits after parent mount                     | Committed node starts the animation; no silent skipped entrance.                  |
| Zero-duration exit                                    | Complete after presence bookkeeping; do not leave a focus/scroll lock.            |
| Right click, secondary touch, disabled/hidden control | No duplicate action or new press admission.                                       |
| Another modal opens during old focus return           | Never focus outside the newer modal.                                              |
| Navigation occurs during a transition                 | Preserve URL/selection authority and hidden-route query isolation.                |
| A step changes inside an open dialog                  | Keep session identity; animate actual size with unscaled text and bounded footer. |
| Step reverses or closes during size change            | Continue from current geometry; revoke cancelled interactions immediately.        |
| One button unmounts                                   | Other controls keep their independent Motion style subscriptions.                 |

## 5. Good / Base / Bad Cases

Good: an account action passes one source ref through its view into Dialog;
closing removes the form, returns only the backing, then restores focus.
Base: an automatic status dialog has no actionable origin and uses neutral
fade/limited geometry. Bad: infer origin from the last global click, animate a
screen capture of a password form, or wait for an animation before admitting
the actual business action.

## 6. Tests Required

- `mcp-followup-origins.spec.ts` exercises catalog installation and editor-save
  follow-up notices with one synthetic authoritative write, actual sourced
  presentation and complete modal cleanup in Chromium and WebKit.

- `dialog-origins.spec.ts` checks immediate/120/300ms removal preview delivery,
  actual completed entry duration, zero unintended mutations, cancelled preview
  isolation, both WorkBuddy assignment paths and transient-menu return/focus in
  Chromium/WebKit. Cover chained confirmation handoff as well as persistent
  buttons. The production wrapper inventory must remain nonempty and complete.
- `dialogPresentation.test.ts` verifies viewport-coordinate rebasing, remaining
  duration, same-track ownership and cancellation. Dialog tests verify a
  never-opened sibling cannot hold the outer presence barrier, and an old close
  frame cannot steal newly chosen editor focus. No warning suppression replaces
  these lifetime assertions.

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
- Press/isolation, disclosure and lens-specific assertions remain in
  [Motion](./motion-system.md); do not duplicate gesture ownership here.
- `state-motion.spec.ts` covers intermediate step frames, unchanged dialog
  identity/choice, reverse/close, missing observer, declarative disclosure and
  lens revisit versus true tab travel. Native resize tracks are captured at
  creation and sampled at explicit times; JS-driven geometry uses the shared
  controlled-clock testing policy in [Quality](./quality-guidelines.md).
  Neither changes the production animation or proves real-time performance.
  `state-performance.spec.ts` samples a
  cold and twenty warm next/back pairs at 1x/4x, separately from opening/closing;
  normal warm frame p95 remains 33.4ms, with layout costs reported explicitly.
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
