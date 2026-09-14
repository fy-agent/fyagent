# Post-archive merge handoff

## Boundary

This Trellis task closes at the exact-head readiness boundary required by
`github-merge-governance.md`: implementation/SPEC convergence, direct-session
prearchive, work commit, task archive, journal, post-archive contracts, final
diff/readback and clean worktree.

The same interactive session then continues as the post-archive merge executor.
Remote PR, exact-head hosted CI, Merge Queue and final `main` readback are later
evidence levels and must not be represented as facts in the archive commit.

## Required sequence

```text
frozen reviewed HEAD
  -> push dev/laiyongjie
  -> create/update PR to main
  -> inspect exact-head hosted checks
  -> gh pr merge --auto --match-head-commit <exact-head>
  -> Merge Queue creates merge_group against latest main
  -> CI / Required passes for merge_group
  -> one merge commit enters main
  -> read back remote main SHA and ancestry
```

## Failure handling

- A failing PR or merge-group check is inspected at the exact job/step. Fixes
  stay scoped to the audited six-commit intent and receive regression evidence.
- Any new commit invalidates the previous handoff. Re-run the applicable full
  local gate, update durable SPEC/task evidence if behavior changed, archive a
  follow-up Trellis task when required, and enable auto-merge only for the new
  exact head.
- Never use `--admin`, direct push to `main`, squash/rebase, temporary merge
  policy changes, skipped required checks or a stale head guard.
- If `origin/main` advances, Merge Queue—not manual update-branch churn—is the
  latest-main authority unless GitHub reports a real conflict that requires a
  branch fix.

## Final readback

The merge executor records:

- PR number and URL;
- frozen PR head SHA;
- exact-head required-check result;
- merge-group `CI / Required` result;
- resulting `origin/main` merge SHA;
- ancestry/readback proving the merged branch result is contained by main;
- confirmation that `dev/laiyongjie` remains available.
