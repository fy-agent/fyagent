# Unified Change Plan Codex Switch Recovery — Implementation Plan

## Closure checklist

1. Freeze current-main baseline and recover only the three backend commits from
   the proven Codex switch slice.
2. Resolve current-main API/schema conflicts without importing obsolete V1 UI
   or old task history.
3. Promote persistence to a real v20 migration with fresh/upgrade coverage.
4. Restore focused redaction, zero-write, replay, stale, readback, and reconcile
   tests; fix path sanitization for both separator families.
5. Run focused gates, then repository format/clippy/Rust/tests/contracts and
   Trellis validation; review the cumulative branch diff.
6. Commit, push, open the replacement PR, wait for required CI, fix failures,
   then close #114 as superseded. Do not merge `main`.

## Transplant set

- `63de4fec`: backend contracts and persistence skeleton.
- `6169369a`: side-effect-free Codex plan creation and command.
- `c3039383`: one-time apply, readback, recovery, and commands.
- Select backend-only test/spec improvements from later commits after the
  current-main port is stable; do not cherry-pick V1 renderer commits.

## Focused verification

```text
mise run rust:test -- change_plan_contract
mise run rust:test -- change_plan_store
mise run rust:test -- codex_provider_switch_plan
mise run rust:test -- change_plan_no_side_effects
mise run rust:test -- codex_provider_change_job
mise run rust:test -- change_plan_reconciliation
```

## Final verification

```text
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run test:unit -- tests/remainingPlatformSurface.test.ts
mise run test:contracts
git diff --check origin/main...HEAD
python3 .trellis/scripts/task.py validate 08-24-issue-55-codex-switch-recovery
```

The exact runnable task names are rechecked against `mise tasks` before the
final gate. A missing or renamed task is corrected to the repository-owned
equivalent rather than bypassed with an ad-hoc toolchain.
