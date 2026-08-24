# GitHub Merge Governance Contract

## 1. Scope / Trigger

This contract owns the repository-level **decision and execution boundary for
merging pull requests into `main`**. It applies whenever a PR may enter the
default branch, whenever repository merge settings/rulesets change, and when a
long-lived development branch is synchronized after a mainline merge.

It does **not** replace the executable CI contract in
[GitHub CI Workflow](./github-ci-workflow.md). CI decides whether the requested
checks passed. Trellis lifecycle decides whether project work is ready to hand
to GitHub's merge executor.

The central rule is:

> Auto-merge is the final executor, not the merge-ready decision-maker.

The project deliberately uses Merge Queue instead of `strict=true` branch
up-to-date checks. GitHub's queue tests the candidate against the latest
`main` plus earlier queued changes, avoiding repeated manual branch updates
while preserving latest-base integration evidence.

The queue uses **merge commits**, not squash or rebase. This is a repository
contract rather than a cosmetic preference: [CC Switch Upstream
Synchronization](./upstream-sync.md) requires approved upstream integrations to
remain explicit two-parent merge commits with the verified upstream commit/tag
preserved in `main` ancestry. Squash and rebase may preserve the resulting tree
but irreversibly destroy that topology and provenance.

Merge commits also preserve meaningful engineering boundaries in large
architecture/integration PRs. Normal mainline browsing should use:

```bash
git log --first-parent main
```

With `max_entries_to_merge=1`, this remains one visible mainline boundary per
accepted PR. Full DAG traversal is reserved for debugging, archaeology, and
provenance work.

Commit hygiene and merge topology are separate concerns. Before a PR becomes
merge-ready, meaningless fixup/checkpoint commits should be folded or removed
when practical, while meaningful implementation/test/docs/refactor boundaries
may remain. The repository must not use a global squash policy as a substitute
for branch-level commit hygiene.

## 2. Signatures

### Repository merge settings

The effective repository settings are:

```text
allow_auto_merge    = true
allow_merge_commit  = true
allow_squash_merge  = false
allow_rebase_merge  = false
```

Accepted PR work enters `main` through one explicit merge-commit boundary.
The PR branch's meaningful internal commits and ancestry remain reachable from
that merge commit.

### `main` protection and queue

`main` requires the stable status check:

```text
CI / Required
```

The classic required-status-check setting remains loose:

```text
strict = false
```

Latest-base integration is instead owned by the active `main` Merge Queue
ruleset:

```text
merge_method                   = MERGE
grouping_strategy              = ALLGREEN
max_entries_to_build           = 2
max_entries_to_merge           = 1
min_entries_to_merge           = 1
min_entries_to_merge_wait_min  = 0
check_response_timeout_minutes = 30
```

The CI workflow must continue to expose:

```yaml
merge_group:
  types: [checks_requested]
```

### Merge-ready CLI boundary

Only after the Trellis lifecycle in section 3 is complete may automation run:

```text
gh pr merge <pr> --auto --match-head-commit <exact-pr-head-sha>
```

When `main` requires Merge Queue, GitHub either waits for unmet PR requirements
or adds the eligible PR to the queue. The queue's `MERGE` policy owns the final
merge method. `--admin` is forbidden in the normal project workflow.

Do not temporarily flip the queue between `SQUASH` and `MERGE` for individual
PRs. Merge Queue policy is shared mutable repository state; per-PR method
flipping creates concurrency races and makes audit evidence ambiguous.

## 3. Contracts

### Merge-readiness lifecycle

The required order is:

```text
implementation complete
  -> focused/full tests and review complete
  -> applicable SPEC converged with final behavior
  -> Trellis task/context validation
  -> direct-session prearchive gate
  -> task archive
  -> post-archive contract/readback check
  -> final diff / worktree / base-drift check
  -> push exact PR head
  -> enable Merge when ready / auto-merge for that exact head
  -> Merge Queue creates merge_group against latest main
  -> CI / Required passes on merge_group
  -> merge commit into main
  -> main push CI passes for the resulting merge SHA
```

The exact PR head passed to GitHub must equal the reviewed/pushed head. A new
commit after readiness evidence invalidates that handoff and requires the
applicable lifecycle checks again before auto-merge is re-enabled.

### Responsibility split

- **Trellis task + SPEC** own scope, durable behavior, review conclusions,
  prearchive evidence, task closure, and whether the work may be handed to the
  merge executor.
- **PR exact-head CI** proves the pushed PR SHA satisfies hosted checks for its
  current PR comparison.
- **Merge Queue** proves the candidate still satisfies required checks when
  combined with the latest `main` and earlier queued changes.
- **main push CI** is post-merge evidence for the exact resulting main SHA.
- None of those evidence levels may impersonate another.

### Long-lived `dev/laiyongjie` synchronization

After a successful mainline merge and successful main push CI:

1. Read back exact remote SHAs for `main` and `dev/laiyongjie`.
2. If `dev/laiyongjie` has **zero independent commits** and is an ancestor of
   final `main`, fast-forward it to the final main SHA.
3. If the local `dev/laiyongjie` checkout is clean and tracks the same remote,
   update it with `--ff-only`.
4. Final readback must show remote `main`, remote `dev/laiyongjie`, and the
   intended local checkout at the same SHA with ahead/behind `0/0`.
5. If dev has independent commits, do **not** force-move or discard them; route
   those commits through their own PR to `main`, then repeat synchronization.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| PR created but Trellis task/spec/prearchive/archive is incomplete | Do not enable auto-merge or queue entry |
| `CI / Required` is green before task archive | Treat as CI evidence only; merge remains blocked |
| Task is archived but working tree/spec has new drift | Stop; repair and rerun applicable gates before handoff |
| PR head changes after reviewed exact SHA | Previous handoff is stale; do not merge under old evidence |
| Merge Queue/ruleset is missing or no longer uses `MERGE` | Stop; restore/explicitly review repository policy before merging |
| Workflow stops handling `merge_group` | Merge Queue cannot satisfy required CI; fix CI before queue use |
| Queue merge group fails/conflicts | Let GitHub remove/block the candidate; fix on the PR branch and repeat readiness |
| `gh pr merge --admin` would bypass queue/protection | Forbidden; never use it for ordinary FyAgent work |
| Direct push to `main` is proposed | Forbidden; use PR + Merge Queue |
| A special PR appears to require squash/rebase or temporary queue-method flipping | Stop; keep queue policy stable and resolve commit hygiene/topology on the PR branch |
| `dev/laiyongjie` is behind final main with zero unique commits | Fast-forward dev to final main after main CI succeeds |
| `dev/laiyongjie` has unique commits | Do not force-update; create/finish a PR for those commits first |
| Post-merge main push CI fails | Main has merged but closeout is not green; investigate before declaring completion/syncing dependent work |

## 5. Good / Base / Bad Cases

- **Good:** implementation, SPEC, Trellis prearchive and archive are complete;
  the exact head is pushed; auto-merge is enabled with an exact-head guard;
  Merge Queue validates `merge_group`; one PR enters `main` through one merge
  commit; main push CI passes; clean dev is then fast-forwarded to the same SHA.
- **Base:** the PR head is ready but hosted PR checks are still running. It is
  valid to enable `Merge when ready` **only because** the local/Trellis
  lifecycle is already closed. GitHub waits and later queues the PR.
- **Bad:** enable auto-merge as soon as a PR is opened and rely on green CI to
  imply that SPEC, task archive, review, and exact-head lifecycle work happened.
- **Bad:** use `strict=true` plus repeated manual update-branch cycles as a
  second latest-main authority while Merge Queue is enabled.
- **Bad:** use `--admin`, direct push to `main`, squash/rebase merge, or
  temporary queue-method changes to make a blocked/special change land faster.
- **Bad:** keep meaningless `fix test`/`fix lint` checkpoint noise solely
  because MERGE preserves branch history; clean such noise before merge-ready
  when practical.
- **Bad:** squash an upstream-sync PR whose verified upstream commit must remain
  an ancestor of `main`; matching tree contents do not preserve provenance.
- **Bad:** force `dev/laiyongjie` to `main` when dev contains independent work.

## 6. Tests Required

For changes to this governance contract or repository merge configuration:

1. Read back repository merge settings and assert auto-merge + merge commits
   are enabled while squash/rebase merge are disabled.
2. Read back `main` protections/rulesets and assert `CI / Required` remains the
   required check, classic strict mode remains false, and Merge Queue is active
   with the parameters in section 2.
3. `tests/ciWorkflow.test.ts` and repository CI contracts must continue to
   prove the `merge_group: checks_requested` trigger and merge-group base/head
   classification semantics.
4. `mise run check:contracts` must pass for maintained task/docs/CI contracts.
5. A Trellis task that changes this policy must pass direct-session prearchive,
   archive successfully, and pass post-archive contracts before its PR is made
   merge-ready.
6. For a real queued PR, GitHub readback must show the queue-required
   `CI / Required` success before merge; post-merge readback must bind main push
   CI to the resulting main SHA.
7. When synchronizing `dev/laiyongjie`, assert ancestry/unique-commit counts
   before update and exact SHA equality plus `0/0` ahead/behind afterward.
8. Upstream synchronization evidence must continue to prove an explicit
   two-parent merge and `git merge-base --is-ancestor <verified-upstream> <main>`.
9. Mainline readability guidance should use `git log --first-parent main`; do
   not require history rewriting merely to make the default full-DAG log flat.

Repository settings are remote configuration, so local static tests do not
claim to prove the live GitHub settings. Live API/readback is required when
those settings are changed or used as merge evidence.

## 7. Wrong vs Correct

### Wrong

```text
open PR
  -> immediately enable auto-merge
  -> CI green
  -> merge
  -> maybe update SPEC/task later
```

```text
gh pr merge <pr> --admin
```

```text
git push origin HEAD:main
```

### Correct

```text
implementation/test/review
  -> SPEC converged
  -> Trellis prearchive
  -> task archive
  -> post-archive/final diff checks
  -> push exact head
  -> gh pr merge <pr> --auto --match-head-commit <sha>
  -> Merge Queue merge_group + CI / Required
  -> merge commit into main
  -> main push CI
  -> fast-forward clean dev/laiyongjie to final main
```

This sequence preserves one authority per layer: Trellis decides readiness,
CI proves checks, Merge Queue proves latest-base integration, and GitHub alone
executes the protected mainline merge.
