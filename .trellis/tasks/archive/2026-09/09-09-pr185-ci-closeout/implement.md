# Execution

- [x] Inspect PR #185, its failing Actions log and existing worktrees.
- [x] Isolate this session after detecting an unrelated concurrent modification.
- [x] Characterize the unchanged browser test and establish a failing backing-brightness regression.
- [x] Add a focused regression using the existing fixture and raster sampler.
- [x] Verify frontend behavior and the repository contracts; inspect the original feature/spec boundaries.
- [x] Update the surface contract and reconcile the original parent/child task metadata.
- [x] Split the oversized Models owner into a focused Grok subscription spec and update its index/manifests.
- [x] Add four failing lifecycle regressions and repair the unmounted/StrictMode scan races exposed by full frontend validation.
- [x] Rebase the three historical plans' executable research references and relative links when their task directories move.
- [x] Commit implementation/spec changes before archiving the obsolete child plans and this repair task.
- [x] Validate the archived tree with no active-task exclusion.

The journal entry is generated after this archive commit by `add_session.py`,
using the three implementation/spec commits rather than archival commit IDs.
Remote delivery after the archive commit: push the replacement PR, follow
required CI and merge-queue checks through actual main inclusion, close #185,
then verify eligible ref/worktree cleanup. Record receipts on that PR; none of
these external operations is claimed by a pre-push checkbox.

## Validation

Use the repository's mise-managed toolchain and frozen pnpm/uv lockfiles.
Run the focused dark-theme WebKit regression before/after the correction,
then the existing Chromium/WebKit appearance and affected Agents/subscription
tests, frontend checks, and prearchive/full repository-contract checks.
GitHub CI provides a fresh check of the complete original feature plus repair
on the required backend platforms. Record every result and any unsupported
real-account/native acceptance separately in `research/verification.md`.

Use an ignored Playwright `defineConfig` extension only to isolate local port
and output paths from concurrent sessions; do not commit this diagnostic config
or change production CI configuration.
