# Codify GitHub merge governance

## Goal

Persist the repository's current GitHub merge policy and the Trellis
merge-readiness lifecycle as one maintained contract, so future work does not
equate `CI / Required == green` with permission to merge.

## Requirements

1. Add a maintained backend code-spec for repository merge governance.
2. Record the effective repository policy exactly: auto-merge enabled,
   squash-only PR merges, merge/rebase merge disabled, `main` protected by
   `CI / Required`, and Merge Queue required for `main`.
3. Record the current Merge Queue parameters and the required
   `merge_group: checks_requested` CI trigger.
4. Define the Trellis lifecycle as the merge-readiness authority: implementation
   and tests complete, applicable SPEC converged, prearchive passed, task
   archived, final diff checked, and exact head pushed before enabling
   `Merge when ready` / auto-merge for that PR.
5. Define Merge Queue as the latest-main integration verifier, not as a
   replacement for Trellis lifecycle or local evidence.
6. Forbid `--admin` queue bypass and direct pushes to `main` as normal project
   workflow.
7. Define post-merge synchronization: after the final `main` merge SHA and its
   push CI are successful, long-lived `dev/laiyongjie` must be fast-forwarded
   to final `main` when it has no independent commits; any independent commits
   require their own PR rather than force-moving the branch.
8. Link the existing GitHub CI spec to the new governance owner without
   duplicating CI implementation details.

## Acceptance Criteria

- [x] `.trellis/spec/backend/github-merge-governance.md` contains all seven
      mandatory code-spec sections and the exact current settings.
- [x] `.trellis/spec/backend/github-ci-workflow.md` points merge-policy readers
      to the new governance spec while retaining CI execution authority.
- [x] `.trellis/spec/backend/index.md` links the new spec.
- [x] The spec clearly says auto-merge is only the final executor; it is not the
      merge-ready decision-maker.
- [x] The spec requires exact-head binding and forbids `--admin` bypass.
- [x] The spec records the final `main` / `dev/laiyongjie` synchronization rule.
- [ ] `task.py validate`, `mise run check:contracts`, direct-session
      prearchive, and post-archive contracts all pass. The first three are
      complete; post-archive remains the final lifecycle check.
- [ ] This task is archived before its PR is made merge-ready.

## Notes

- Source-of-truth readback on 2026-08-24 confirmed the GitHub repository and
  ruleset settings before this task was created.
- GitHub's official Merge Queue documentation confirms that queue candidates
  are tested against the latest base plus earlier queued changes and that
  Actions workflows must handle `merge_group` events.
- This task documents the policy; it does not change product behavior.
