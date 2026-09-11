# Nightly closeout verification

Baseline: `origin/main@11c1333902bf6de8b6bb8acc5b2e8d9d56329fd3`.
Published PR #187 input: `398628bc95999138b2c859e382989747ea1309f8`.
Host: macOS Apple Silicon, 2026-09-10.

## Reproduction and repair

Hosted run `34385278528`, Frontend Checks job `102580619306`, failed three
unit assertions. A focused local run reproduced all three: two stale raster
counts (121 instead of the reviewed inventory's 122) and concrete workstation
home paths in the archived #52 PRD/task metadata.

The repair updates those two counts, explicitly pins the already-reviewed
health screenshot path and SHA-256, replaces concrete home paths with portable
placeholders, and refreshes the reviewed test file's structure identity. The
source diff contains no platform dispatch, scanner exclusion, budget, dependency
or product behavior change. Existing inventory mutation and privacy tests remain.

The first repair rerun correctly rejected the changed test file's old identity
seal. That failure is retained, the exact diff was reviewed, and only that
manifest digest was updated. The final complete gate then passed.

The hosted browser setup also failed because a package-index download had a
Hash Sum mismatch; subsequent browser startup reported a missing executable.
This is separate from the three source-controlled assertion failures. No package
verification or browser assertion was weakened to accommodate it.

## Final complete local gate

From the directly bound current session:

```text
mise run python:run -- mise run check:prearchive --exclude-active-task .trellis/tasks/09-10-night-validation-closeout
```

Exit 0. Includes environment/toolchain checks, TypeScript, ESLint, formatting,
186 unit files (1641 passed / 1 existing skipped), 7 desktop mock assertions,
visual-manifest preflight, Rust formatting/check/Clippy and 19 Rust suites
(3562 passed / 6 existing ignored; main library 3237 passed / 5 ignored),
repository/platform/Python/version/release contracts and native-fetch mocks.
The release-contract subset has 616 passed / 1 existing skipped, and its
native-fetch subset has 4 passed; these overlap the wider unit gate and are not
additional unique feature counts.

Log: `/tmp/fyagent-night-audit-20260910/full-gate.log`.
Earlier failures remain in `focused-before.log`, `focused-after.log` and
`full-gate-before-seal-refresh.log` in the same local evidence directory.

## Browser and post-archive checks

Serial `mise run python:run -- mise run test:browser` exited 0: both production
boot cases and all 576 Chromium/WebKit functional cases passed (4.4 minutes for
the interaction suite). Log: `/tmp/fyagent-night-audit-20260910/browser-final.log`.
Product source, native source, configuration and lockfiles are byte unchanged
from the published #187 head. Canonical post-archive contracts and the focused
tracked-documentation/inventory tests are required before push; their final
receipts are retained in the local-only evidence packet.

## Branch preparation and evidence boundary

The published #187 head is four commits ahead of the current main and
`git merge-tree --write-tree --name-only origin/main codex/issue-52-health-center`
exits 0 without conflicts. The Grok reuse and Prompt/Memory closeout branches
are already main ancestors. The older #172 branch has 20 conflict paths and
is not part of this delivery. #180 explicitly remains an audit-only draft.

44 pre-existing worktrees were independently read back: HEAD, full status list
and tracked-diff SHA-256 remained unchanged. Local-only branch receipts and
the separate coordination task are kept outside the repository under the
user's task-artifact directory; concrete workstation paths are not committed.

The earlier #52 macOS isolated native actions/readback remain bounded by their
archived report; this test/metadata repair does not claim new native acceptance.
Two supplemental account-animation performance tests still have the documented
33.4ms budget failures on both baseline and #52. No new performance sampling,
Windows interactive acceptance, real-account request, signing or Release is
claimed. These are explicit follow-up items in the coordination packet.

The new-session task was prepared but not launched: this tool surface lacks
native thread creation, and CUA explicitly denies operating the Codex app.
No new session identifier was fabricated.

## Spec review

No new runtime contract is introduced. Existing supported-platform identity
seals, workstation-path privacy, post-archive verification and merge-governance
specs already require the corrected behavior; no SPEC rewrite is necessary.
