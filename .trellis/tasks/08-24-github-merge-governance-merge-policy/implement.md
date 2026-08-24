# Merge governance correction implementation plan

1. Read and cross-check current governance SPEC, `upstream-sync.md`, CI SPEC,
   `CONTRIBUTING.md`, and live GitHub repository/protection/ruleset state.
2. Update `github-merge-governance.md` from SQUASH to MERGE and add:
   - upstream two-parent ancestry as the hard constraint;
   - first-parent mainline browsing;
   - commit hygiene vs merge topology separation;
   - no temporary per-PR ruleset flipping.
3. Update backend index summary and keep CI SPEC ownership split correct.
4. Update English/Chinese contributor merge guidance.
5. Change live repository merge settings to merge-commit-only + Auto-merge.
6. Change only the queue `merge_method` to `MERGE`; preserve all other queue
   parameters.
7. Perform authoritative live readback of repository settings, main protection,
   queue ruleset, and `merge_group` CI trigger.
8. Run task validation, diff check, and repository contract suite.
9. Run direct-session prearchive, archive the task, then rerun post-archive
   contracts in the staged final repository shape.
10. Commit the correction and archive/journal evidence, push the exact #142
    head, then enable `Merge when ready` with `--match-head-commit`.
11. Do not manually poll after queue handoff unless investigating a failure;
    GitHub Merge Queue owns latest-main validation.

## Files expected to change

- `.trellis/spec/backend/github-merge-governance.md` — canonical policy.
- `.trellis/spec/backend/index.md` — summary text.
- `.trellis/spec/backend/github-ci-workflow.md` — only if wording still implies
  the old merge method; CI mechanics remain unchanged.
- `CONTRIBUTING.md` — public contributor policy in both languages.
- this Trellis task directory / later archive.

## Explicitly out of scope

- CI job topology or Required evaluator changes.
- Merge Queue concurrency/timeout tuning beyond the already reviewed values.
- Review-count or conversation-resolution policy changes.
- Direct main pushes or `--admin` bypass.
- SecretRef/#132 work until governance correction is handed to the queue.
