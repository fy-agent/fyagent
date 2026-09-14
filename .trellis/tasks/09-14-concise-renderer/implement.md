# Execution and verification

## Plan

- [x] Inspect clean checkout, Trellis workflow and frontend owner contracts; research community feedback and primary writing guidance.
- [x] Audit eight routes and shared secondary controls; record keep/remove/consolidate decisions.
- [x] Simplify Skills/MCP detail hierarchy and redundant assignment metadata using existing owners.
- [x] Simplify Agent/Auth/Health/model/prompt/memory panels, dialogs and empty/loading states without changing workflow authority.
- [x] Adjust existing role CSS only where removal exposes unnecessary space or hierarchy.
- [x] Add focused copy/layout regressions and update retired text assertions without dropping behavior/safety assertions.
- [x] Run initial frontend/browser checks and review captured layouts. Their initial results and baseline failures are retained in `verification-initial.md`; the authorized complete repair and final results are in `verification.md`.
- [x] Update frontend copy/visual/feature SPEC owners and validate task context; subsequently resolve the reproduced baseline failures under the user's completion authorization.
- [x] Record final results and limitations, review the full diff and prepare `commit-plan.md`.
- [x] Obtain local work-commit/archive approval through the user's request to complete all remaining work.
- [ ] Commit the reviewed task scope, archive the task, validate archived references and record the session. Do not push.

## Gates

### Remaining completion work (authorized)

- [x] Review native changes since the platform seal, repair the stale Windows
  source contract with boundary-sensitive negative coverage, update only reviewed
  individual source digests and rerun the real scanner.
- [x] Reproduce React warnings, fix asynchronous test completion and guard
  lifetime, and verify repeated full renderer tests with no unexpected warnings.
- [x] Execute full local prearchive, release, browser and serial production
  performance checks; fix actionable failures without lowering thresholds.
- [ ] Update backend/frontend SPEC and final verification/commit plan, commit
  task-scoped work locally, archive, journal and run canonical postarchive checks.

Use repository-owned `mise` commands. Run compilation/unit gates separately from performance profiling. Use targeted runs while iterating, then the complete frontend and browser gates. Do not update approved visual baselines automatically or weaken budgets/assertions to pass.

Before archive, review the complete diff for functionality, safety warnings, accessible names, secret/redaction boundaries, source attribution and unchanged native authority. A failed or unexecuted check must remain explicitly recorded.
