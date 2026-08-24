# Verification

## Live GitHub readback

Read on 2026-08-24 after the repository merge-policy change:

```text
allow_auto_merge   = true
allow_squash_merge = true
allow_merge_commit = false
allow_rebase_merge = false
```

`main` protection:

```text
required check = CI / Required
strict         = false
required approving reviews = 0
```

Active ruleset `main-merge-queue` (`id=21271876`) targets
`refs/heads/main` and reports:

```text
merge_method                   = SQUASH
grouping_strategy              = ALLGREEN
max_entries_to_build           = 2
max_entries_to_merge           = 1
min_entries_to_merge           = 1
min_entries_to_merge_wait_min  = 0
check_response_timeout_minutes = 30
```

`.github/workflows/ci.yml` still contains:

```yaml
merge_group:
  types: [checks_requested]
```

## GitHub behavior cross-check

Official GitHub documentation was rechecked before writing the SPEC:

- Merge Queue validates queued changes against the latest target branch and
  earlier queued changes before merge.
- GitHub Actions required checks for Merge Queue require the separate
  `merge_group` event.
- When a target branch requires Merge Queue, `gh pr merge` can add an eligible
  PR to the queue or enable auto-merge while requirements are still pending.
- Squash merge condenses PR work into one mainline commit, matching this
  repository's contribution convention.

## Local repository evidence

- `task.py validate`: pass.
- `git diff --check`: pass.
- `mise run check:contracts`: pass; contract tests 510 passed / 1 skipped,
  native fetch 4/4, supported-platform scanner passed.
- Direct-session `mise run check:prearchive -- --exclude-active-task
  .trellis/tasks/08-24-github-merge-governance-spec`: pass. The complete local
  gate also passed: frontend 1489 passed / 1 skipped, Rust library 2847 passed /
  5 ignored plus integration/helper suites, and release contracts 510 passed /
  1 skipped.
- Trellis archive completed to
  `.trellis/tasks/archive/2026-08/08-24-github-merge-governance-spec` with zero
  active tasks remaining.
- Post-archive `mise run check:contracts`: pass after staging the canonical
  archive move so the repository scanner could verify regular-file Git modes;
  contract tests 510 passed / 1 skipped and native fetch 4/4.
- No product/runtime code changed in this task.
