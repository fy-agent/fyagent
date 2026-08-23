# Implementation plan

## Baseline

- Branch base: `9b9e40b3` (`codex/issue-63-codex-provider-vertical`).
- Product dependencies: #130/#132/#134/#136/#137 stacked recovery chain.
- No schema-v21, Provider/AppType coupling, apply-time network call, generic
  undo system, or merge/release action.

## Steps

1. Extend closed backend/TypeScript enums and strict parsers for the WorkBuddy
   operation, business step and resources; initialize job resources by adapter.
2. Add WorkBuddy pure preview/private snapshot, lock-held writer orchestration,
   semantic readback and baseline restoration using existing atomic primitives.
3. Implement the typed WorkBuddy adapter and operation-specific apply,
   reconciliation and classification without changing Codex behavior.
4. Add the Tauri command/ACL/port and route V2 WorkBuddy save/delete through
   the operation-aware shared preview/job component.
5. Add focused contract, canary, drift, internal-token, recovery and UI tests;
   update executable specs and run repository/native evidence gates.

## Closure evidence

- Commit: `29ff57e2be0a66b1d4362347563ae27c32ecd825`.
- Draft PR: <https://github.com/fy-agent/fyagent/pull/139>, stacked on
  `codex/issue-63-codex-provider-vertical` / PR #137.
- `mise run check`: passed after the intentional Tauri handler-count update.
- `mise run lint:v2`, `mise run typecheck:v2`, `mise run test:v2`: passed.
- `mise run test:v2:browser`: 116 passed.
- Focused Rust Change Plan suite: 37 passed, 1 ignored.
- Focused Rust WorkBuddy suite: 62 passed.

## macOS native UAT

Ran the debug Tauri app against an isolated `FYAGENT_TEST_HOME` and a seeded
WorkBuddy document.

- Preview: exactly one ready plan, zero jobs, unchanged `models.json` SHA-256,
  no backup, and no synthetic API-key canary in the DB or log.
- Apply: one confirmation, one job, seven persisted snapshots, all five phases
  succeeded, and both WorkBuddy resources read back as `matched`.
- Result: `succeeded/applied`, restart `not_required`, usage evidence
  `not_observed`, recovery `not_needed`.
- The recovery backup SHA-256 exactly matched the pre-apply baseline; the new
  canary remained absent from DB, log and backup.
- The installed `/Applications/FyAgent.app` process was restored after the
  isolated debug run.

## Remaining boundary

- Windows matching-host WorkBuddy HIL is not available on this macOS host.
  Therefore PR #139 remains Draft and Issue #66 remains OPEN; no main, release,
  or issue-closure claim is made.
