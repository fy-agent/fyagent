# WorkBuddy Change Plan adapter implementation

## Order

1. Add `workbuddy_models_save` to the closed operation enum and
   operation-scoped resource descriptors. Keep Codex sequences exact.
2. Add `create_workbuddy_save_plan` plus a process-private draft of the
   save request. Plan inspect is read-only.
3. Apply routes WorkBuddy through the typed executor and the existing
   locked WorkBuddy writer. Internally satisfy overwrite; treat revision
   drift as stale.
4. Wire Models 「保存并应用」 to create/preview/confirm. Keep chip-remove
   on `save_workbuddy_models`. Remove the save-path overwrite dialog.
5. Cover preview (no API Key), confirm payload, stale regenerate, and
   rollback copy with focused tests. Update browser fixture.
6. Update SPEC. Run V2 gates plus `test:v2:browser` before prearchive.
7. Archive, push exact head, open replacement PR, close old #139 as
   superseded, then exact-head Merge Queue. Keep #66 open.

## Validation

```bash
mise run typecheck:v2
mise run lint:v2
mise run test:v2
mise run rust:test -- change_plan
```

## Risky files

- `src-tauri/src/services/change_plan/service.rs` — do not take the
  Codex provider lock for WorkBuddy apply
- `src/v2/shared/features/change-plans.ts` — parsers stay closed and
  operation-scoped
- WorkBuddy overwrite token must not leak to the renderer
- Chip-remove must keep writing immediately

## Rollback

Revert the WorkBuddy-vertical commits on `dev/workbuddy-change-plan`.
Do not roll back #148 Codex upsert, #147 UI, #146 executor, #145
SecretRef, #140 Quick Setup, or #135 ledger.
