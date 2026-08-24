# Correct main merge governance to merge commits

## Goal

Supersede the just-recorded squash-only mainline policy with a policy that
preserves FyAgent's existing ancestry/provenance contracts while retaining the
new Merge Queue, Auto-merge, exact-head handoff, and Trellis merge-ready gate.

## Requirements

1. Change the canonical mainline policy to `Merge Queue + MERGE` with one PR per
   merge group (`max_entries_to_merge=1`).
2. Keep Auto-merge enabled, `CI / Required` required, classic `strict=false`,
   Merge Queue enabled, and all existing queue parameters except merge method.
3. Repository merge methods must become:

   ```text
   allow_auto_merge    = true
   allow_merge_commit  = true
   allow_squash_merge  = false
   allow_rebase_merge  = false
   ```

4. The active `main-merge-queue` ruleset must use `merge_method=MERGE`.
5. Preserve the existing Trellis lifecycle rule: Auto-merge is an executor,
   never the merge-ready authority; only archived/exact-head work may be handed
   to the queue.
6. Explain the hard repository constraint: `.trellis/spec/backend/upstream-sync.md`
   requires upstream integrations to remain explicit two-parent merge commits
   with upstream ancestry preserved. Global squash/rebase merge is therefore
   incompatible with an existing durable FyAgent contract.
7. Separate commit hygiene from merge topology. PR branches should clean up
   meaningless fixup/checkpoint commits before merge-ready when practical, but
   mainline topology must not erase meaningful commits or ancestry to enforce a
   cosmetic linear log.
8. Define `git log --first-parent main` as the normal one-PR/one-boundary view;
   full DAG traversal remains available for engineering forensics.
9. Update `CONTRIBUTING.md` in both English and Chinese so contributor guidance
   matches live GitHub policy and no longer claims squash-only.
10. Update the GitHub merge-governance SPEC and its backend index/CI cross-link
    without changing CI execution semantics.
11. Read back live GitHub settings after every remote policy mutation; API write
    success is not sufficient evidence.
12. Do not use `--admin`, direct `main` pushes, or temporary ruleset method
    flipping for special PRs.
13. Keep #142 open and update its branch to the corrected final policy rather
    than merging the stale squash-only state.

## Acceptance Criteria

- [x] Repository live settings match `auto=true`, `merge=true`, `squash=false`,
      `rebase=false`.
- [x] `main-merge-queue` is active with `merge_method=MERGE` and all other
      validated queue parameters unchanged.
- [x] `.trellis/spec/backend/github-merge-governance.md` records the MERGE
      policy, first-parent reading model, commit-hygiene separation, upstream
      ancestry rationale, exact-head queue lifecycle, and dev synchronization.
- [x] `CONTRIBUTING.md` English and Chinese sections no longer claim accepted
      changes are squash-merged and instead describe Merge Queue + merge-commit
      mainline behavior.
- [x] Existing `upstream-sync.md` two-parent contract remains unchanged and is
      explicitly referenced by governance SPEC.
- [x] `CI / Required`, `strict=false`, `merge_group: checks_requested`,
      Auto-merge readiness gate, and `--admin` prohibition remain intact.
- [ ] `task.py validate`, `git diff --check`, `mise run check:contracts`,
      direct-session prearchive, archive, and post-archive contracts pass. All
      prearchive gates are complete; archive/post-archive remain.
- [ ] #142 is only made merge-ready after the corrected task is archived and
      its exact final head is pushed.

## Notes

- This task supersedes the merge-method conclusion in archived task
  `08-24-github-merge-governance-spec`; that archive remains historical
  evidence and is not rewritten.
- Three engineering review passes converged on `Merge Queue + MERGE +
  max_entries_to_merge=1`; the decisive constraint is existing upstream
  ancestry/provenance, not aesthetic preference.
