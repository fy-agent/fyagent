# One origin lifecycle across all callers

## Shared ownership

Dialog remains the sole Radix modal/focus/portal wrapper. dialogPresentation
owns native animation tracks, useDialogResize owns content sizing, and feature
state owns preview/cancellation/mutation. Neither raw DOM measurements nor
animation completion imply native operation success.

Distinguish intrinsic-content notifications while entering from actual viewport
change or visibility/reduced-motion interruption. Intrinsic preview changes
must not call the origin-settle shortcut. Rebase the decorative geometry toward
the updated destination from its currently rendered viewport position, keeping
one entry timeline and its remaining duration/foreground handoff. Do not restart
a full420ms animation on every observer callback, nor delay the native request
or create a fake minimum-loading timer. Only after entry does ordinary content
resize use its existing320ms role. ResizeObserver must not watch its own writes.

If entry is interrupted by user close or viewport/preference loss, existing
safe settlement still applies. Keep partial-start cancellation and rejected
finished-promise handling. No second animation queue, spring solver or focus trap.

## Source admission and ephemeral controls

Keep explicit feature-owned origin refs. Capture the real element before state
changes or asynchronous work. Add a narrowly scoped capture to the existing
origin adapter for transient controls: finite viewport rectangle, corner radii
and existing allowlisted material strings only. The capture is one-use and
owned by the initiating open intent/session; discard on close/cancel/supersede.
Never retain labels, innerHTML, images, input values, secret data or URL resources.

Live controls are remeasured. A menu item that unmounts may supply its captured
opening geometry and the owner's explicit stable menu trigger as return/focus
anchor. The association must be passed by that menu, not guessed from text or
activeElement. Removed/scrolled-away controls without a valid explicit return
target use neutral close; document this truth rather than animate toward stale
coordinates. Nested dialogs retain independent source and focus ownership.

User-driven wrappers require an origin contract; automatic/programmatic roots
must be explicitly neutral with a reviewed rationale. TypeScript/AST checks
cover all direct Dialog and wrapper usages, with maintained invocation coverage
for callbacks and asynchronous branches. They complement, not replace, actual
geometry/lifecycle tests. Switch/checkbox/asChild captures must occur before
checked-change callbacks, without synthesizing another business click.

## Preview and destructive safety

Account removal keeps previewId/accountId/revision/canApply semantics. Preview
completion belongs only to the requesting account/session; a cancelled or
replaced request cannot overwrite current presentation/state. No actual
removal request occurs in browser fixture origin tests until a separate explicit
confirmation test admits it. Preserve cancel-first focus and late-modal focus
protection, step state and default immediate secret teardown.

## Verification / rollback

Test response delivery at immediate,16,80,120,252ms and after entry; include
success/failure/late-after-close. Sample physical production timing and world
rectangles across the handoff, not only data-motion-origin. Cover all parent
matrix categories; file pickers use synthetic controlled responses and do not
pretend to prove native picker HIL. Keep existing reduced-motion neutral behavior.
Rollback as one source-lifecycle/adapter/caller change set, with its SPEC/tests.
