# Fix GitHub CI merge-queue event topology

## Goal

Remove duplicate Required CI authorities for the same code state and make
GitHub Merge Queue deterministic. Pull requests and `merge_group` candidates
must remain protected by the stable `CI / Required` aggregate, while ordinary
branch pushes keep only a lightweight Conventional Commit policy check.

## Requirements

- `.github/workflows/ci.yml` must stop listening to ordinary `push` events.
  Required CI is owned by `pull_request`, `merge_group(checks_requested)`, and
  explicit `workflow_dispatch` diagnostics only.
- `merge_group` is the only Required CI authority for a Merge Queue candidate.
  A `gh-readonly-queue/**` push must not create a second `CI / Required` run.
- Preserve the current affected-domain classifier for PR and merge-group runs.
  `workflow_dispatch` remains an explicit full diagnostic run.
- Remove push-specific `before`/unreachable-before handling from the Required CI
  workflow because push is no longer one of its events.
- Add a separate lightweight push workflow that runs Conventional Commit
  validation on branch pushes without starting classifier/domain jobs. It must
  explicitly ignore `gh-readonly-queue/**` refs and must not publish a check
  named `CI / Required`.
- Keep the existing commit-convention job inside Required CI for PR titles,
  pull-request commit ranges, and merge-group commit ranges.
- Include event identity in Required CI concurrency keys so future trigger
  expansion cannot make different event types cancel each other accidentally.
  This is defense in depth; it is not a substitute for removing push from
  Required CI.
- Do not add top-level `paths`/`paths-ignore` filters to Required CI. The stable
  aggregate must always materialize for PR/merge-group events.
- Do not change the current Merge Queue method, Required check name, Rust cache
  policy, docsSpec domain split, or product test content in this P0 slice.
- Update the maintained GitHub CI SPEC to reflect the final event model and
  remove statements that all branch/main/dev pushes receive Full CI.
- Preserve GitHub Merge Governance semantics: Auto-merge is the executor only
  after Trellis/SPEC/prearchive/archive readiness; Merge Queue remains the
  latest-main integration authority.

## Acceptance Criteria

- [x] `ci.yml` has no `push` trigger and still has `pull_request`,
  `merge_group: checks_requested`, and `workflow_dispatch`.
- [x] `ci.yml` no longer carries push SHAs, push fallback logic, or
  `push => event_force_full` behavior.
- [x] `workflow_dispatch` still produces `forceFull=true` with every current CI
  domain requested.
- [x] Required CI concurrency includes event identity and does not share a key
  between different event kinds.
- [x] A separate push workflow runs only checkout + Node + commit-message
  verification + diagnostic aggregation, ignores `gh-readonly-queue/**`, and
  has a distinct check/job name.
- [x] Static workflow tests prove there is exactly one Required-CI event model
  and the lightweight push workflow cannot create `CI / Required`.
- [x] Existing classifier, required-gate, CI toolchain, repository contract, and
  merge-group trigger tests remain green.
- [x] `.trellis/spec/backend/github-ci-workflow.md` and related indexes/contracts
  match the implementation with no stale full-push claims.
- [x] Full Trellis prearchive passes with the task's direct session binding.
- [ ] Task is archived before the exact-head PR is handed to Auto-merge/Merge Queue.

## Post-archive GitHub closeout

These are real-host validation requirements for the final archived exact head;
they are deliberately **not** prerequisites for changing the task itself to
completed/archive, because the project merge-governance contract requires task
archive before Auto-merge/Merge Queue handoff.

- Hosted PR CI demonstrates the expected classifier plan for this CI-control
  change.
- The final `dev/laiyongjie` push produces `Commit Convention / Push` but no
  push-event `CI / Required` workflow.
- Merge Queue produces one Required authority from `merge_group`; the
  corresponding `gh-readonly-queue/**` ref does not create a push CI run.
- After merge, the `main` push likewise does not create Required product CI;
  final main SHA is read back and clean `dev/laiyongjie` is fast-forwarded.

## Evidence Baseline

- PR #142 head `871fdee92ed81b61f948fb9603eb340ee547f6a1` produced both:
  - pull-request CI run `32703935399` (~2.5 min), and
  - push CI run `32703934089` (~18.3 min).
- Merge Queue candidate/final main SHA
  `d2340abc48094a766ce23615d95195d7bae12e45` produced:
  - merge-group run `32704207859`, cancelled, and
  - queue-ref push run `32704208101`, full CI, success after ~18 min.
- The same final main SHA then produced another full push run `32705765951`.
- Fast-forwarding `dev/laiyongjie` to that already-green main SHA immediately
  started another redundant full push run `32709555068`; it was manually
  cancelled because it carried no PR/Merge Queue Required authority.
- Local source confirms `ci.yml` currently listens to `push`, uses `github.ref`
  in a cross-event concurrency key, and sets `event_force_full=true` for push.

## Out of Scope

- Splitting docs/spec into a new lighter contract job (P1 follow-up).
- Enabling target/sccache/Cargo build-artifact caching (P2 experiment only after
  a new post-deduplication baseline exists).
- Rewriting product tests or reducing the actual affected-domain coverage.
- Changing the Merge Queue `MERGE` topology or `max_entries_to_merge=1` policy.

