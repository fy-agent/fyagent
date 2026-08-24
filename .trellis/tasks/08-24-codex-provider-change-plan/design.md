# Codex Provider Change Plan vertical design

## Ownership

Reuse `FeaturePorts.changePlans` and Models Apply. Codex Quick Setup save
is `codex_provider_upsert_and_switch`; existing-Provider switch stays
`codex_provider_switch`. Both share Apply preview, `{ planId, planDigest }`
confirm, and bounded `getChangeJob` polling.

Confirmed touch points:

- Native second adapter + `create_codex_provider_upsert_plan`
- Process-private upsert draft; schema v20 ledger unchanged
- Codex Models form no longer calls `applyQuickSetupWithResult`
- Closed V2 parsers accept both registered adapter ids
- `.trellis/spec/frontend/v2-agent-models.md` and executor SPEC

Do not create shared chrome unless a second current consumer exists.
`changePlanErrors.ts` is shared by switch and save Apply hosts.

## Contract

- Preview is zero-write.
- Confirm is `{ planId, planDigest }` only.
- Job truth is `getChangeJob` + existing Apply projection.
- `secretCapability=secret_dependency_unavailable` blocks confirm.
- No Models cancel button.
- No connectivity / model probe on the apply path.
- Usage evidence remains `not_observed`.

## Compatibility

- Do not change schema v20, SecretRef, Quick Setup targeted patch, or
  #147 preview/polling.
- Do not rename `change-plans` back to `change-plan`.
- Do not merge Draft #137.

## Explicit non-goals

- WorkBuddy adapter
- Restoring Draft #137 CSS/page tree
- Expanding wire DTO unless current parsers already require a field that
  the UI cannot project
