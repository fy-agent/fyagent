# Canonical V2 Change Plan UI implementation

## Order

1. Extend `createApplyViewModel` with four-section preview model, partial-result
   projection, and `eventSeq` presentation. Keep fail-closed secret/expiry
   confirm gates.
2. Update `ApplyWorkspace` to render the four preview sections, partial truth,
   and event sequence. Do not add a cancel control.
3. Update `ChangePlanWorkspace` to poll `getChangeJob` while the current job is
   non-terminal. Honour `requestRevision`, unmount cleanup, and target changes.
4. Extend
   `tests/v2/pages/models/apply/ApplyWorkspace.test.tsx` and the Change Plan
   workspace/port tests for preview, polling, partial result, and cancel
   absence.
5. Update `.trellis/spec/frontend/v2-agent-models.md` to the landed contract.
6. Run focused V2 tests, then `mise run typecheck:v2`, `mise run lint:v2`, and
   `mise run test:v2` as needed. Full `mise run check` plus
   `mise run test:v2:browser` before prearchive.
7. Direct-session prearchive, archive, post-archive, push exact head, open
   replacement PR, close old #136 as superseded, then exact-head Merge Queue.

## Validation

```bash
mise run test:unit -- tests/v2/pages/models/apply/ApplyWorkspace.test.tsx
mise run typecheck:v2
mise run lint:v2
mise run test:v2
```

Before merge-ready:

```bash
mise run test:v2:browser
mise run check
```

## Risky files

- `src/v2/pages/models/apply/ChangePlanWorkspace.tsx` — polling vs revision
  races
- `src/v2/pages/models/apply/view-model.ts` — do not weaken fail-closed copy
- `src/v2/shared/features/change-plans.ts` — parsers stay closed; this slice
  should not need parser changes
- `.trellis/spec/frontend/v2-agent-models.md` — do not revert #140/#146 rules

## Rollback

Revert the UI/SPEC commits on `dev/v2-change-plan-ui`. Do not roll back #146
typed executor, #145 SecretRef, #140 Quick Setup, or #135 Change Plan ledger.

## Start gate

Do not run `task.py start` until the user explicitly approves this planning
summary.
