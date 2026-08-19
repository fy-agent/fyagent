# Codex Provider create/edit code map

Evidence level: `code_audit` at source freeze `ca552f4d`. No tests, build,
runtime, fetch, or edits were used to derive this map.

## Current entry chains

Create flows from `App.tsx` -> `AddProviderDialog` -> `ProviderForm` ->
`useProviderActions` -> renderer mutation/API -> Tauri provider command ->
`ProviderService::add`. Edit follows the same layers through
`EditProviderDialog` and `ProviderService::update`. Both are direct mutations;
only switch uses Change Plan.

Current create generates Provider ID and createdAt in renderer. The legacy writer
may implicitly select the new Provider when no current exists. Current edit reads
the effective current Provider and may read full live `{auth, config}` into the
renderer before save.

## Managed and indirect effects

Potential effects include:

- Provider DB row and custom endpoints;
- DB/device current Provider and Codex auth/config/catalog files;
- common configuration and managed MCP projection;
- proxy live backup/takeover state;
- renderer query invalidation, tray rebuild, restart coordinator;
- DB mutation hooks that can trigger asynchronous WebDAV/S3 auto-sync.

The endpoint speed-test editor can add/remove endpoint DB rows before the main
form is confirmed. It must become draft-only for Change Plan.

ProviderService itself makes no Provider/model request during save, but DB hooks
may trigger external backup traffic. Plan spies therefore cover both Provider
clients and auto-sync hooks.

## Credential boundary blocker

Credential material may exist in `settingsConfig.auth`, TOML bearer tokens,
Codex auth.json token fields, usage scripts/headers/body, local proxy request
overrides, and managed OAuth bindings. Current Provider list/read/edit DTOs can
put those values into renderer query/form state and ordinary SQLite.

Therefore create/edit cannot be made production-safe by only adding two Change
Plan operations. Before #35 exact-SHA handoff and companion redacted list/edit
DTOs:

- no plaintext fallback;
- no current DB/live secret copied into Plan;
- no accountId treated as immutable secretRef;
- no create/edit production IPC or UI routing;
- only opaque secretRef port, redacted drafts, and fixtures are allowed.

## Recommended minimal seams

1. Keep shared job/admission/reconcile in the existing Change Plan owner.
2. Add operation-specific Codex mutation adapter for create/edit inspect,
   resource predicates, readback, and classification.
3. Extract Provider writer pure preparation/validation from add/update, reused by
   legacy writer and Plan inspect.
4. Add explicit create policy: store-only, make-current, legacy-if-no-current;
   Change Plan uses only the first two.
5. Split pure expected auth/config/catalog projection from commit-time file IO.
6. Make endpoint editing draft-only and commit endpoint set in the owning
   Provider transaction.
7. Route renderer create/edit only after backend DTO + #35 contract freeze.

## Conflict budget

- Single owner: Change Plan core, schema/DAO/commands, Rust/TS DTO, shared fixture.
- Provider writer owner: provider service modules, codex config projection,
  provider DAO/endpoints.
- Frontend integration owner: App/actions/dialogs/form/API/query/i18n after the
  backend contract freeze.
- New operation adapter and focused fixtures are low-conflict files.
- Migration version must coordinate with parallel Prompt/Memory work; this task
  may not independently claim schema v17.
