# Codex Provider Change Plan vertical implementation

## Order

1. Map current-main Models Codex Provider create/edit/switch onto the
   existing `change-plans` port. Identify which operations the typed
   executor already admits.
2. Wire those actions to zero-write plan creation and the existing Apply
   workspace. Do not add a second confirm payload.
3. Cover drift/regenerate, fail-closed secret capability, and cancel-button
   absence with focused tests.
4. Update SPEC only for the landed Provider vertical.
5. Run focused V2 tests, `mise run typecheck:v2`, `mise run lint:v2`, and
   `mise run test:v2`. Full `mise run check` plus `mise run test:v2:browser`
   before prearchive.
6. Archive, push exact head, open replacement PR, close old #137 as
   superseded, then exact-head Merge Queue.

Landed:

- Second registered operation `codex_provider_upsert_and_switch`
- `create_codex_provider_upsert_plan` + process-private upsert draft
- Codex Models save no longer calls `applyQuickSetupWithResult`
- Apply preview/confirm/polling reused; no cancel button
- Issue #63 stays open

## Validation

```bash
mise run typecheck:v2
mise run lint:v2
mise run test:v2
```

## Risky files

- Models Provider save/switch handlers — do not keep a bypass write path
- `src/v2/shared/features/change-plans.ts` — parsers stay closed
- `.trellis/spec/frontend/v2-agent-models.md` — do not revert #147

## Rollback

Revert the Provider-vertical commits on `dev/codex-provider-change-plan`.
Do not roll back #147 UI, #146 typed executor, #145 SecretRef, #140 Quick
Setup, or #135 Change Plan ledger.
