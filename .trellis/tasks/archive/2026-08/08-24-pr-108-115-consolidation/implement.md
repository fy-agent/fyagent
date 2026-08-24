# Implementation plan

## Closure checklist

1. Freeze baseline and source ledger.
2. Complete three disjoint implementation waves: Change Plan backend, Agent readiness, Apply/Grok UI.
3. Run one serialized integration wave for schema, registration, ports and cross-layer wiring.
4. Fix local gate failures; obtain Grok/Gemini and independent Trellis review receipts.
5. Create one commit and Draft PR; wait for initial `CI / Required` green.
6. Comment on and close the still-open source PRs plus derived #130; read back #108/#109/#112/#113/#114/#115/#130 as closed/unmerged.
7. Write governance evidence, amend the same commit with `--force-with-lease`, prove product digest unchanged.
8. Wait for final CI, set Ready for Review, freeze branch, hand off to `python-rust`.

## Ownership

- `08-24-consolidate-secret-change-plan`: Change Plan domain/service/DAO and focused unit tests; no shared schema/registration.
- `08-24-consolidate-agent-install`: readiness domain/command/UI owned files and focused tests; no shared registration.
- `08-24-consolidate-apply-grok-ui`: Apply UI and Grok copy/tests; no native shared registration.
- `08-24-consolidate-integration-governance`: schema v20, sync lists, command/lib/ACL registration, FeaturePorts composition and integration tests.
- Root: task governance, diff review, all Git/GitHub operations, external audit orchestration and closure truth.

## Progress log

- 2026-08-24: root claimed the parent task on baseline `e94307cd`; isolated worktree and branch created; no product implementation started before contracts were frozen.
- 2026-08-24: all four child tasks completed. Final product digest `cd38c076…`; aggregate local gates passed; Grok PASS; Trellis PASS_WITH_FIXES with all P1 fixes reverified; Gemini honestly recorded BLOCKED/INCONCLUSIVE.
- 2026-08-24: GitHub `origin/main` was freshly fetched before commit preparation and remained `e94307cd` / Schema v19. Single commit, Draft PR, hosted CI and source-PR governance remain pending.
- 2026-08-24: created Draft PR #135 at initial head `5b2d904b`; both push and pull-request `CI / Required` runs passed, including hosted Linux, macOS, Windows X64 and Windows ARM64 jobs.
- 2026-08-24: after the initial Required gate, added per-PR migration readbacks and closed #108/#113/#115/#130. #109/#112/#114 were already closed. All seven read back `closed + merged=false`; out-of-scope Drafts #132/#134 were left unchanged.
- 2026-08-24: governance-only head `3c27acaf` passed both push run `32664954837` and pull-request run `32664957087`, including both Required aggregators. The final task-state amend must pass a fresh latest-head Required gate before PR #135 may become Ready.
