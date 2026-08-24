# Implementation plan

1. Split branch-push Conventional Commit enforcement into a dedicated
   lightweight workflow and add static trigger/security tests for it.
2. Remove `push` from `.github/workflows/ci.yml`, remove its push-specific
   identity/fallback/full-run branches, and retain Full only for
   `workflow_dispatch`.
3. Add `github.event_name` to Required CI concurrency identity.
4. Update `tests/ciWorkflow.test.ts` and `tests/githubWorkflowTriggers.test.ts`
   so event topology and collected-step contracts fail closed if push is
   accidentally reintroduced.
5. Update `.trellis/spec/backend/github-ci-workflow.md` and any directly related
   maintained references.
6. Run focused workflow/classifier/required-gate tests, then full repository
   contracts/checks.
7. Complete direct-session prearchive, archive the task, push the exact head,
   and hand the PR to Auto-merge/Merge Queue.
8. Verify hosted behavior:
   - PR head has no Full push Required CI duplicate;
   - merge-group SHA has exactly one `CI / Required` authority;
   - branch push only produces the lightweight push-policy workflow.

## Stop conditions

- Any change that weakens Required domain coverage rather than removing
  duplicate event execution.
- Any proposal to use top-level path filtering for Required CI.
- Any need to alter Merge Queue merge topology or cache policy; those are
  separate tasks.
