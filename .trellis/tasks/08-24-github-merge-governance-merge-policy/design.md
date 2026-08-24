# Merge governance correction design

## Decision boundary

Keep the recently introduced safety architecture:

```text
Trellis merge-ready lifecycle
  -> exact PR head
  -> Auto-merge / Merge when ready
  -> Merge Queue
  -> merge_group CI / Required
  -> protected main
```

Change only the final topology from `SQUASH` to `MERGE`.

## Why MERGE is the repository-wide method

FyAgent has two materially different PR populations:

1. Small focused product/bug PRs where squash is convenient.
2. Large architecture/integration/upstream PRs where commit boundaries and Git
   ancestry are durable engineering evidence.

GitHub Merge Queue exposes one merge method at the queue/ruleset level. Runtime
flipping between SQUASH and MERGE for special PRs creates a shared mutable
policy race and is not acceptable.

The hard constraint is upstream provenance. `upstream-sync.md` requires a
verified upstream tag/commit to remain an ancestor of an explicit two-parent
merge commit. Squash and rebase can preserve tree contents but destroy that
topology. Therefore the uniform queue method must be `MERGE`.

## Mainline readability

Merge commits do not require noisy daily history. The canonical operator view
is:

```bash
git log --first-parent main
```

With `max_entries_to_merge=1`, this yields one mainline boundary per accepted
PR while the full DAG remains available for forensic/debug/provenance work.

## Commit hygiene

Commit hygiene is a pre-merge branch concern, not a reason to erase topology.

- Remove/fold meaningless fixup/checkpoint commits when practical before the
  task becomes merge-ready.
- Preserve meaningful implementation/test/docs/refactor boundaries.
- Do not use global squash to clean up branch history after the fact.

This correction PR itself will not destructively rewrite the long-lived
`dev/laiyongjie` branch. It will add a clear correction commit because the
earlier squash-only task was already committed and archived before the newer
review arrived.

## GitHub configuration

Repository:

```text
allow_auto_merge    = true
allow_merge_commit  = true
allow_squash_merge  = false
allow_rebase_merge  = false
```

Queue:

```text
merge_method                   = MERGE
grouping_strategy              = ALLGREEN
max_entries_to_build           = 2
max_entries_to_merge           = 1
min_entries_to_merge           = 1
min_entries_to_merge_wait_min  = 0
check_response_timeout_minutes = 30
```

Classic status protection stays:

```text
required check = CI / Required
strict         = false
```

CI retains `merge_group: checks_requested`.

## Rollback

If the live settings fail to apply or read back inconsistently, stop before
making #142 merge-ready. Do not partially compensate by disabling Merge Queue
or using `--admin`. Restore the last known reviewed policy explicitly, then
re-evaluate.
