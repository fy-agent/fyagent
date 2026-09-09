# Design

## Evidence and boundary

PR #185 at `2956da243d1e79f401d946852dadba8f30d28043` fails CI run
`34229637275`, Frontend Checks job `102072461491`. Browser testing reports
535 passed and one WebKit `blue-themes.spec.ts` failure: Agents-page
`Grok Build` text is white over sampled RGB(111,141,164), ratio
3.487260310188897 against the existing 4.5 threshold. Other frontend steps,
repository contracts and both backend jobs passed in that run.

The initial investigation boundary is the Agent directory's existing CSS
surface role, its browser regression and the owning appearance/surface spec.
Do not change the raster sampler or theme tokens merely to suppress an error.
Reproduce before selecting a correction. Use existing shared surface tokens,
Playwright and its current test fixtures; no new material or accessibility engine.

The unchanged whole-page test passed 3/3 on macOS, so the specific Linux
compositor/font cause is not asserted. A deterministic replay of the observed
RGB(111,141,164) brightness under the real Grok directory card fails before
the correction: title 3.4523:1 and supporting text 3.1603:1. Change only the
card's backing from the ambient `--fy-surface-soft` to existing paired
`--fy-surface-inset`. The regression retains the production text, fixture,
sampler and 4.5:1 threshold; its parent backing is an explicit stress input,
not a claim that the exact hosted-runner rendering was reproduced locally.

Task context validation also found `frontend/models.md` at 35,311 bytes,
above the configured 32,768-byte per-file injection limit. Extract the Grok
subscription contract into `frontend/grok-subscription.md`, keep the Models
port facade and a discovery pointer, and route the index/manifests to the
focused owner. Preserve every binding, failure and test invariant rather than
increasing the injection limit or allowing the contract tail to be truncated.

## Scan lifecycle discovered by the complete frontend gate

The complete unit gate exposed an unhandled `window is not defined` rejection
at `useAgentDirectoryScan.ts:214` after router-shell test teardown. The hook
guarded visibility but not unmount. Existing `start` callbacks could also
refetch after disposal, and StrictMode replay restored an older idle state
over the synchronous start guard. Four deterministic hook regressions fail on
the original code: late success, late rejection, retained start and duplicate
StrictMode scan admission.

Reuse a local effect-owned mounted ref to ignore disposed-owner continuation
and preserve newer scan request IDs when effects synchronize render state.
Keep hidden-owner buffering/reconciliation and backend job authority intact.
React's official effect cleanup guidance is the reference; no cancellation
framework, global error suppression or test retry is added.

## Task records

The archived `08-31-grok-first-class-iteration` parent explicitly supersedes
three August 31 child execution plans. Those children still report
`in_progress` and empty descriptions. The archive script intentionally clears
child `parent` references when archiving the parent; preserve that behavior.
Record their
superseded resolution, original scope and successor evidence before archival.
Administrative completion of these obsolete plans must not assert their old
WorkBuddy or real-account acceptance criteria passed. Preserve original PRDs
and historical research; update executable manifests only where relocation
actually invalidates a reference.

## Delivery and safety

Work in an isolated branch rooted at PR #185, retaining its four commits.
One pre-existing worktree developed a concurrent CSS edit during inspection;
leave it untouched. The local browser port and artifact directory are shared
by existing configuration, so use an ignored Playwright configuration derived
with `defineConfig` for diagnostic isolation only. Tests, fixtures, engines,
thresholds, retry count and application build stay unchanged.

Push a replacement PR to main, inspect all required checks and use ordinary
GitHub merge policy with an exact head match. Confirm `mergedAt` and the merge
commit on origin/main before closing #185. Freshly query open PR heads/bases
and remote SHAs before branch deletion. Preserve GitHub's active merge queue,
the README-serving star-history branch and any concurrent dirty worktree.

## Rollback

The local repair is independently revertible without removing the original
feature. No secret storage, data schema or public API migration is introduced.
Record deleted branch SHAs in the delivery evidence; never force-update main
or alter branch protection to complete the merge.
