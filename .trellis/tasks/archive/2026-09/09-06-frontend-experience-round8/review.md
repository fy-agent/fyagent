# Round-eight implementation review

## Boundary and reuse

The work is isolated on `fix/round8-responsive-layout` from planning commit
`fba857c8`. The user approved the final plan before task activation. No dependency,
native command, theme, target registry or persisted data changes are made.
The existing panel library owns resizing; CSS Grid/container queries own local
composition; Buttons retain press/origin semantics. Bulk display is extracted
from the two existing identical sections, not a new mutation implementation.

## Failure evidence and corrections

- `/tmp/fyagent-round8-before.log`: six WebKit failures on unchanged product
  code. Four A→B→A cases reached a 215.17px row against a 43px content-based
  upper bound; two width tests found a 558px auxiliary rail against its new
  intended 360px maximum. No exceptions or horizontal overflow were needed
  to expose these defects.
- Explicit intrinsic grid rows remove inflation without fixed heights or
  remounts. Shared bulk groups use atomic action pairs, consistent container
  admission and a safe stacked baseline. The smallest layout experiments
  remain in the planning research; no synthetic success/reset timer was adopted.
- `DETAIL_PANE_SIZING` supplies both management pages. The existing adapter
  selects the middle flexible pane and bounded pixel rails; supported handles
  implement reset. Other two-pane defaults and catalog aliases remain intact.
- Local info grids use a 256px floor and at most two columns; labels are capped
  at min(30%,8em), with remaining space for values. Short cards keep natural
  height. Copy-only paths remain hidden and callable by their real accessible
  names, not a newly revealed path. Auth and Models retain their existing
  role-specific narrow forms and definitions.
- The first unit run overlapped browser work: an unchanged production scanner
  timed out at its existing 5s limit, and a legacy CSS assertion expected the
  replaced max-content label rule. The latter now asserts the shared cap plus
  the existing copy-only min-content safeguard. An isolated full rerun passed
  1,562 tests/one existing skip; no timeout or scanner contract was changed.
- Additional window/draft coverage found an actual WebKit ResizeObserver error.
  `/tmp/fyagent-round8-resize-owner.log` records 900→868px, unchanged 756px
  height, `semantic:false`, immediately followed by the loop error in three
  of four runs. The content-resize owner was animating a CSS viewport clamp.
  It now settles non-semantic width changes while keeping semantic widths,
  intrinsic height animation and source-entry retargeting. This avoids the
  offending write instead of hiding errors or deferring every observation.
  Temporary observer/logging probes are not production code. Browser health
  reporting also uses a nonempty error message when WebKit supplies an empty
  stack, so future failures are readable rather than a blank string.

## Verification ownership

Responsive browser tests cover both themes/engines, actual rail drag, keyboard
and reset, repeated width boundaries, enlarged text, local-import metadata,
hidden-route restoration and the original MCP draft node. The same tests enter
the serial production configuration. Full retained suites and existing motion,
scroll, source, theme and performance budgets remain required.

The worktree runs the full frontend correctness/browser/production gates.
After those pass, its reviewed implementation commit is merged into the main
checkout. The complete repository/native prearchive gate then runs on that
merged tree using its existing native build cache, before final work evidence
and archival. This avoids a second Rust build cache without sharing target
overrides, changing guards, or substituting frontend tests for native checks.

No real credentials, native picker, minimum-version WebView/GPU, signatures,
push, deployment or release are part of this verification. Final executed
results, merge ancestry and safe cleanup are recorded below when available.

## Frontend verification and review passes

Composition review confirms the same seven target IDs/order, callbacks, locks
and native methods; the moved bulk block has no mutation state. The old unused
Skills/MCP size declarations are gone, while two-pane defaults and live catalog
reporting aliases remain. Density rules are shared by role rather than applied
as a universal width percentage. Light/dark screenshots were inspected with
the same synthetic data; natural-height cards and uniform action rows remain
legible, without exposing the copy-only path.

Lifecycle review separated the newly reproduced viewport-clamp observer error
from semantic step animation. The focused deterministic hook cases passed 3/3;
the unchanged rapid-resize browser case then passed six consecutive WebKit runs
without diagnostic probes or warning suppression. The temporary hash-navigation
probe was replaced with the actual owned navigation link, preserving router
intent rather than bypassing it.

Canonical typecheck/lint/format all passed. `/tmp/fyagent-round8-final-unit.log`
records 177 files, 1,565 passes and one existing skip. The complete browser task
passed production boot/timing (2 cases) and all 526 functional cases across four
Chromium projects plus WebKit (`/tmp/fyagent-round8-final-browser.log`). The new
10 density scenarios are selected alongside, not in place of, retained coverage.
Final production performance and merged repository gates follow below.

The first complete production run rejected two cases (33 passed, 2 failed;
`/tmp/fyagent-round8-performance.log`): stale-preview cancellation sent Escape
after only the fixture request existed, and normal step-frame p95 measured
49.9ms against the unchanged 33.4ms target. Three isolated normal step repeats
passed at 33.4ms with no long tasks; six unchanged cancellation repeats passed.
These repeats establish variability, not a proven machine/GPU root cause.
The stale-preview case now first establishes the committed modal/cancel focus
while keeping its backend Promise pending. It still asserts cancellation,
late-response rejection, fresh-session admission and no unintended deletion;
the separate interrupted-entry tests remain unchanged. No product delay,
timeout increase, frame-budget change or console suppression was introduced.
The original failing run is retained; full production verification must pass
again and is not replaced by these passing subsets.

## Final isolated production verification

Two consecutive complete serial production runs passed **35/35** each:
`/tmp/fyagent-round8-performance-final-1.log` and `-final-2.log`. No product
source changed between these runs. Both execute all ten new density cases and
the retained 25 origin/navigation/presentation/state/theme cases. The final
local-import screenshot was also inspected: long description is contained,
source/card widths remain useful, the short assignment card keeps natural
height, and the installation path remains copy-only. The test additionally
checks enlarged-font bulk buttons against their own row bounds.

At 1232x700, each run contains 42 navigation revisits per CPU condition and
20 warm motion cycles: normal navigation p95 **28.5/28.3ms**, 4x p95
**47.2/49.6ms**; modal warm-frame p95 **33.4ms** at both rates; step resize
**33.4ms normal / 50ms at 4x**. Theme thirds are **16.7–16.8ms**. Keep the
normal 50.1ms individual frame and stress long tasks (navigation up to104ms,
presentation52ms) visible in the record; these are not universal60fps claims.
The earlier 49.9ms normal p95 failure remains evidence of timing variability,
not a deleted result or permission to widen budgets.

Next: commit this verified implementation, fast-forward the clean main branch,
run the complete repository prearchive and final functional browser task there,
then record those exact outcomes before archival and non-forced cleanup.

## Merged repository and cleanup

Implementation commit `a929195aeaef7756266d503f6f3f8302d260b017` was fast-forwarded
into the previously clean `dev/laiyongjie` checkout. Its exact source/config/test
tree is unchanged after the isolated production runs. The main checkout's full
`check:prearchive` exited0 (`/tmp/fyagent-round8-merged-prearchive.log`):177 unit
files,1,565 passes/one existing skip; Rust3,495 passes/zero failures/six existing
ignores; desktop mock7, release contracts611/one skip, native Fetch4, plus strict
types, lint, formatting, graph/build/toolchain checks. The current task context
was explicitly passed to the gate, not bypassed after its initial missing-pointer
refusal. Final functional execution on the merged checkout is recorded below.

All three registered linked worktrees were clean and their heads were verified
as ancestors of the main checkout before removal. The implementation tree held
only generated ignored dependencies/build/Python/session state; the two detached
baseline copies had no tracked, untracked or ignored additions. No live process
used those working directories. Non-forced `git worktree remove` removed each;
`git branch -d` removed only the merged implementation branch. The registry now
contains only the main checkout and dry-run pruning reports no stale entries.
No arbitrary clean/reset, force deletion, history rewrite or unrelated branch
deletion was used.

The merged checkout's canonical `mise run test:browser` also exited0:
production boot/timing2 and **526 functional cases passed**
(`/tmp/fyagent-round8-merged-browser.log`). All newly tightened cancellation
readiness and enlarged-font bounds are included. Source, tests, configuration
and scripts still match implementation commit `a929195a`; only task evidence
is changed for archival. Six updated SPEC files have43 valid relative links,
and all nine context entries resolve. No native source or dependency lock
changed relative to the approved baseline.

The task now enters administrative closure: preserve the full implementation
commit in both evidence fields, archive, repair the three self-research context
references, run no-exclusion contracts, record the journal, and verify a clean
main checkout with no active task or linked-worktree residue.

## Closure evidence

The task is completed and archived. Its three self-research context references
now resolve under the archive directory; all nine entries validate and both
work-commit fields point to the existing ancestor `a929195a`. No-exclusion
`mise run check:contracts` exited0 after archival
(`/tmp/fyagent-round8-postarchive-contracts.log`). Session78 was written by the
repository recorder with the implementation and integrated-verification commits.
The final bookkeeping commit contains only this archive evidence and the journal;
the final delivery check verifies no active tasks, a clean main checkout and no
linked-worktree or implementation-branch residue.

## Supplemental merged profiling

An additional main-checkout run on implementation `a929195a`
(`/tmp/fyagent-round8-main-final-performance.log`) passed 34 cases and rejected
the normal step-resize frame budget: 49.9ms p95 versus 33.4ms. This overlaps the
postarchive contract runner recorded in `/tmp/fyagent-round8-postarchive-contracts.log`;
the overlap is observed, not proof of a GPU or scheduler root cause. All ten
density cases and the remaining source, navigation, modal and theme cases passed.
Do not replace this failed run with the two earlier passing isolated runs or
describe every measurement as passing.

The final main-checkout run waited until three consecutive process-admission
samples found no competing verification job. It then passed all 35 cases with
exit code 0 (`/tmp/fyagent-round8-main-isolated-performance.log`). Product code,
test assertions and build configuration still match implementation `a929195a`;
no budget, timeout, animation or assertion was changed for the rerun.
Navigation p95 was 28.2ms / 47.5ms at 1x / 4x, modal frame p95 was 33.4ms at
both rates, and step frame p95 was 33.4ms / 50.0ms. Normal step maximum was
50.1ms; stress navigation long tasks were 101ms and 51ms, with a 55ms modal
long task. Theme thirds remained 16.7–16.8ms. These final results supplement,
not erase, the failed run and do not prove stable native GPU performance.
