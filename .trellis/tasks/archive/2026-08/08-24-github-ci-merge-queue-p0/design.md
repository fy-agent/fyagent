# GitHub CI / Merge Queue P0 design

## Root cause

The current single `CI` workflow listens to `pull_request`, `push`,
`merge_group`, and `workflow_dispatch`. Push event policy then overwrites the
classifier result with a Full CI plan. On Merge Queue refs, GitHub emits both a
`merge_group` event and a branch `push`; both runs use the same `github.ref` in
the concurrency key. With `cancel-in-progress=true`, the two event authorities
can cancel one another. A cancelled Required run can eject the PR from Merge
Queue before the intended merge-group check completes.

The architectural error is not the classifier. It is assigning three roles to
one workflow/event surface:

1. PR Required CI,
2. Merge Queue Required CI,
3. branch-push policy/full diagnostics.

## Target event topology

```text
branch push
  -> Commit Convention / Push
     (lightweight, no CI / Required)

pull_request
  -> Commit Convention
  -> Classify Changes
  -> affected domain jobs
  -> CI / Required

merge_group
  -> Commit Convention
  -> Classify Changes
  -> affected domain jobs
  -> CI / Required

workflow_dispatch
  -> Commit Convention
  -> Classify Changes
  -> Full domain plan
  -> CI / Required
```

## Required CI workflow

Triggers:

```yaml
on:
  pull_request:
  merge_group:
    types: [checks_requested]
  workflow_dispatch:
```

Concurrency includes event identity:

```text
ci + workflow + event_name + PR-number/merge-group-ref/dispatch-run
```

PR and merge-group base/head identity remains explicit. Manual dispatch uses
`github.sha` as both base/head, then event policy promotes all domains to Full.
There is no push base/head branch in this workflow.

## Lightweight push policy workflow

Use a separate workflow with branch pushes only and ignore
`gh-readonly-queue/**`. It checks out full history, resolves `before` and head,
uses the same forty-zero/unreachable-before fallback already reviewed for
pushes, and invokes `verify-commit-messages.mjs` only. It has no classifier,
product tests, repository release contracts, or `CI / Required` aggregate.

The fallback is retained here because force-push history can still make
`github.event.before` unreachable. This concern belongs to push policy after the
split, not to Required CI.

## Why no main post-merge Full CI in this slice

Merge Queue already verifies the exact candidate that enters `main` against the
latest base and queued predecessors. Re-running the same SHA as unconditional
Full Required CI after merge adds large cost without adding a second admission
decision. Formal Release already owns its own native compile/release evidence.

If future operations need a post-merge diagnostic, add a separately named,
non-required affected-domain workflow. Do not reintroduce push into
`CI / Required`.

## Safety boundaries

- No top-level path filters on Required CI.
- Classifier failure remains fail-closed and starts all conditional diagnostics
  through existing job `if` logic.
- Required gate still validates requested-vs-skipped job conclusions.
- Merge Queue continues to require exactly the stable `CI / Required` check.
- No cache-policy change is coupled to this event-topology fix.
