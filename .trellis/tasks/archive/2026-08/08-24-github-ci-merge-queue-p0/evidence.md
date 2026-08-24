# Evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Remote baseline

| SHA / context | Event | Run | Result | Meaning |
| --- | --- | ---: | --- | --- |
| `871fdee92ed8…` (#142 head) | `pull_request` | 32703935399 | success | affected-domain PR CI |
| `871fdee92ed8…` (#142 head) | `push` | 32703934089 | success | redundant Full branch-push CI |
| `d2340abc4809…` (queue candidate) | `merge_group` | 32704207859 | cancelled | intended Merge Queue Required authority lost race |
| `d2340abc4809…` (queue ref) | `push` | 32704208101 | success | queue-ref Full push survived instead |
| `d2340abc4809…` (`main`) | `push` | 32705765951 | success | duplicate post-merge Full CI |
| `d2340abc4809…` (`dev/laiyongjie`) | `push` | 32709555068 | cancelled manually | redundant Full CI triggered by dev fast-forward |

## Local structural baseline

Current `.github/workflows/ci.yml` before this task:

- triggers `pull_request`, `push`, `merge_group`, `workflow_dispatch`;
- concurrency falls back to `github.ref` without event identity;
- both queue-ref `push` and `merge_group` therefore resolve the same group;
- `push` sets `event_force_full=true` and overwrites every classifier domain to
  `true`.

This task treats these as root-cause evidence, not as a reason to weaken the
classifier or Required gate.

## Local implementation evidence

- Required `.github/workflows/ci.yml` now listens only to `pull_request`,
  `merge_group(checks_requested)`, and `workflow_dispatch`; push SHA/fallback
  branches were removed from the Required workflow.
- `workflow_dispatch` remains the only event-level Full CI override.
- Required concurrency now includes `github.event_name`.
- `.github/workflows/commit-convention-push.yml` owns ordinary branch-push
  Conventional Commit policy and explicitly ignores `gh-readonly-queue/**`.
  It contains no classifier, dependency install, Cargo invocation, or
  `CI / Required` result.
- Focused workflow/classifier/gate tests: **70/70 PASS** across
  `githubWorkflowTriggers`, `ciWorkflow`, `verifyCommitMessages`,
  `requiredCiGate`, and `classifyChanges`.
- `mise run check:contracts`: **PASS** after updating the CI/Merge Governance
  SPECs, backend SPEC index, and maintained developer CI guide.
- Direct-session
  `mise run check:prearchive -- --exclude-active-task .trellis/tasks/08-24-github-ci-merge-queue-p0`:
  **PASS**. The run covered 1491 frontend unit tests (+1 skipped), i18n,
  desktop mock/visual preflight, Rust formatting/check/Clippy/tests (2847 passed,
  5 ignored in the main Rust suite), task/docs/platform contracts, release
  contracts, and native-fetch contracts.
- Post-archive `mise run check:contracts`: **PASS** on the canonical archive
  shape with no active-task exclusion.
- Redundant dev-sync push run `32709555068` was cancelled after confirming it
  was another old-policy Full Push CI with no PR/Merge Queue authority.

