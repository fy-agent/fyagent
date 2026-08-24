# WorkBuddy Change Plan adapter design

## Ownership

Reuse `FeaturePorts.changePlans` and Models Apply. WorkBuddy save becomes
`workbuddy_models_save`. Codex switch / upsert stay registered and
unchanged. All three share Apply preview, `{ planId, planDigest }`
confirm, and bounded `getChangeJob` polling.

Confirmed touch points:

- Native third adapter + `create_workbuddy_save_plan`
- Process-private save draft (includes API Key); schema v20 ledger stays
  credential-free
- Operation-scoped `readSet` / `writeSet` / job resources so Codex
  four-resource jobs keep parsing
- WorkBuddy Models save no longer calls `saveModels` for 「保存并应用」
- Chip-remove keeps `save_workbuddy_models`
- `.trellis/spec` for executor, WorkBuddy configuration, and V2 Models

Reuse `changePlanErrors.ts` and Apply workspace. Add a page-local
`WorkBuddySavePlanWorkspace` next to `CodexSavePlanWorkspace`.

## Contract

- Preview is zero-write.
- Confirm is `{ planId, planDigest }` only.
- Job truth is `getChangeJob` + existing Apply projection.
- Public plan shows canonical base URL, model ID summary, overwrite
  impact, and backup/restore note. No API Key.
- Overwrite token stays adapter-internal. If existing IDs would be
  updated, the plan lists that risk; apply writes once.
- Revision mismatch at apply is `stale`, not a renderer overwrite dialog.
- `secretCapability=secret_dependency_unavailable` blocks confirm.
- No Models cancel button.
- Usage evidence remains `not_observed`.
- Reserved `targetProviderId` = `fyagent-v2-workbuddy-models` because the
  closed plan DTO still requires a nonempty target id.

## Resource set

```text
readSet  = work_buddy_models_config, work_buddy_backup
writeSet = work_buddy_models_config, work_buddy_backup
```

Frontend parsers accept the union of Codex and WorkBuddy kinds, and
require job/adapter sequences to match the plan's `operationType`.

## Compatibility

- Do not change schema v20, SecretRef, Codex upsert, or #147 preview.
- Do not rename `change-plans` back to `change-plan`.
- Do not merge Draft #139.
- Do not take the Codex Provider mutation lock on the WorkBuddy apply
  path. Use the existing WorkBuddy write lock / locked saver.

## Explicit non-goals

- Chip-remove Change Plan
- Restoring Draft #139 CSS/page tree or UCP proof stack
- Closing #66 from the PR title
