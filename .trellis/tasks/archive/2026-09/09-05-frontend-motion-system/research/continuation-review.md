# Continued implementation review

The existing uncommitted Button, origin geometry, live media query and press
feedback modules were preserved for review. This continuation adds focused
geometry/preference/session tests before accepting their integration. It does
not infer author identity from file timestamps or overwrite unrelated work.

Primary references rechecked for the locked dependency integration:

- https://motion.dev/docs/radix — controlled state, `asChild`, `forceMount`,
  and presence-based exit; examples are not proof that every force-mounted
  Dialog preserves this project's focus and secret lifetimes.
- https://www.radix-ui.com/primitives/docs/guides/animation — CSS presence is
  supported for mount/unmount; plain transitions alone are not an exit owner.
- https://motion.dev/docs/press — primary-pointer filtering and Enter support;
  native Space needs a visual-only supplement without another synthetic click.
- https://motion.dev/docs/react-transitions — physics parameters and
  duration/bounce are alternative spring definitions, not additive controls.

Review boundary: shared UI interaction modules, explicit source plumbing in
their existing feature callers, role tokens/styles, and tests/SPEC. No native
write, credentials, router authority, window policy or dependency version change.

Required checks include a source removed/scrolled/hidden before exit, fresh
conditional-dialog sessions, live reduced-motion updates, input gesture
duplicate suppression, source-to-material geometry without text deformation,
and combined production navigation performance after material and motion.

The V2 test environment bridge was compared with Vitest's upstream
`packages/vitest/src/integrations/env/jsdom.ts::patchAddEventListener` (official
source: https://github.com/vitest-dev/vitest/blob/main/packages/vitest/src/integrations/env/jsdom.ts).
It translates only the DOM listener's signal while keeping Node Request/fetch
native. A localized `unknown` cast prevents TypeScript incorrectly narrowing
the second realm's same-named AbortSignal to `never`; cancellation is verified
for multiple targets and already-aborted signals, not silently dropped.

Focused Dialog/Auth regressions exposed a genuine zero-duration exit race:
the child's layout effect called `safeToRemove` before AnimatePresence's
parent layout effect populated its exiting-key map. The installed 12.23.25
`AnimatePresence/index.mjs::onExit` returns early when that key is absent.
The fix defers immediate completion by one commit microtask and invalidates
superseded generations, rather than lengthening tests or adding a fake delay.
Normal Motion completion still owns timed removal. Reduced-motion and jsdom
zero-layout cases must exercise this path and release the modal/scroll lock.

The real-browser audit also caught a portal timing defect that unit tests
could not establish visually: Radix's deferred Portal commit occurred after
the parent layer's mount effect, leaving entry geometry uninitialized. The
animation now starts from the committed Content node via a callback-ref state,
not from a mount-only ref assumption. A required browser assertion checks
`data-motion-origin=trigger` on the actual account dialog, and sampled material
transforms must converge to the final dialog while text remains unscaled.

Node admission also distinguishes a real new Content element from Radix's
transient composed-ref detach/attach callbacks. Counting every null-to-node
callback as a new mount caused a render loop; the committed-node identity is
retained across transient detaches while the live ref is cleared normally.
DOM mutation stays in refs rather than mutating a useState-returned object,
and settled visual state is derived from the completed animation phase.

The origin audit identified guarded sidebar navigation as a user-triggered
confirmation, not an automatic dialog. Its source now passes through the
existing PrimaryBlocker context as a one-shot intent matched to the next
destination; it does not observe global clicks, add a second router state, or
change blocker rules. Native/programmatic transitions without that matching
intent remain neutral. Prompt/Memory local controls keep their own origin refs.
