# Verification

Evidence date: 2026-08-24 (Asia/Shanghai)

## Decision evidence

Three review passes converged on `Merge Queue + MERGE +
max_entries_to_merge=1`.

The decisive repository-local evidence is the existing
`.trellis/spec/backend/upstream-sync.md` contract:

```text
The upstream integration commit is an explicit two-parent merge.
Do not squash, rebase, or reconstruct the upstream history.
```

Historical commit `f4462765e9b3a2efd1deb13aabf3ce349166a058`
is an existing two-parent upstream merge example. A squash/rebase mainline
policy can preserve its tree but cannot preserve equivalent future upstream
ancestry, so global squash-only is incompatible with an existing durable
FyAgent contract.

## Live GitHub mutation and readback

Before correction:

```text
allow_auto_merge    = true
allow_merge_commit  = false
allow_squash_merge  = true
allow_rebase_merge  = false
queue merge_method  = SQUASH
```

The mutation was applied in a compatibility-safe order:

1. enable merge commits while temporarily retaining squash;
2. switch only the active queue's `merge_method` to `MERGE`;
3. disable squash.

Authoritative readback after mutation:

```text
allow_auto_merge    = true
allow_merge_commit  = true
allow_squash_merge  = false
allow_rebase_merge  = false

required check      = CI / Required
strict              = false
enforce_admins      = true
force push main     = disabled

main-merge-queue:
  enforcement                    = active
  merge_method                   = MERGE
  grouping_strategy              = ALLGREEN
  max_entries_to_build           = 2
  max_entries_to_merge           = 1
  min_entries_to_merge           = 1
  min_entries_to_merge_wait_min  = 0
  check_response_timeout_minutes = 30
  bypass_actors                  = []
```

`.github/workflows/ci.yml` still contains:

```yaml
merge_group:
  types: [checks_requested]
```

PR #142 remained `OPEN`, unmerged, and had `autoMergeRequest=null` during the
correction, so the stale squash-only SPEC was not handed to the queue.

## Repository drift scan

Maintained docs/spec scan found one non-current mention in the Release SPEC.
It was clarified to say **historical squash-merged commits** rather than imply
that squash remains current mainline policy. The current merge method is now
owned exclusively by `github-merge-governance.md`.

## Pending lifecycle evidence

- `task.py validate`: pass.
- `git diff --check`: pass.
- `mise run check:contracts`: pass; contract tests 510 passed / 1 skipped,
  native fetch 4/4, supported-platform scanner passed.
- Direct-session `mise run check:prearchive -- --exclude-active-task
  .trellis/tasks/08-24-github-merge-governance-merge-policy`: pass. Full local
  gate passed: frontend 1489 passed / 1 skipped, Rust library 2847 passed /
  5 ignored plus integration/helper suites, release contracts 510 passed /
  1 skipped, native fetch 4/4.
- Task archived to
  `.trellis/tasks/archive/2026-08/08-24-github-merge-governance-merge-policy`;
  active task count is zero.
- Post-archive `mise run check:contracts`: pass in the staged canonical archive
  shape; contract tests 510 passed / 1 skipped, native fetch 4/4,
  supported-platform scanner passed with 2104 current files.
- corrected exact #142 head push and Merge Queue handoff
