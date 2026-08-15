# Source audit — Issue #35

## Audit envelope

- Audited: 2026-08-14 Asia/Shanghai.
- Dedicated worktree: `/Users/serendipity/.codex/worktrees/issue-35-secret-backend`.
- Branch: `codex/issue-35-secret-backend`.
- Immutable implementation base: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab` (`origin/codex/prompt-memory-v2-main-pr`).
- Live remote observations at initial audit time:
  - `origin/main`: `4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287`
  - `origin/codex/prompt-memory-v2-main-pr`: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`
  - `origin/codex/unified-change-plan-codex-switch`: initially `4bfee69c...`; refreshed below.
- GitHub authority: [Issue #35](https://github.com/fy-agent/fyagent/issues/35), current body plus both comments read before design and refreshed with IDs/times/digests in `research/issue-35-authority.md`.

## Refreshed downstream/source facts

On 2026-08-14 the local remote readback proved:

- final #55 branch `origin/codex/unified-change-plan-codex-switch` = `6859e9ce04970008f4cf8b3d4883b4f70316291a`;
- #55 implementation-contract/source SHA = `ca552f4d918cacc734f81f7efdef70619da139b8`;
- 2026-08-15 remote relation readback confirms `ca552f4d...` as merge-base/ancestor and the branch as ahead 3 / behind 0;
- #55 kept `SCHEMA_VERSION=16` and did not reserve v17;
- its current definition/live projection digests still consume secret-bearing `Provider`/live shapes, so #35 freeze must publish a compatibility change rather than claim the final #55 tree is already value-free.

Prompt/Memory native integration independently has `DESIGN_FREEZE=PASS` and reserves SQLite v17 in its immutable design tree `e12f07a2ffb59d316984ca00040f782e10e1f5a1`; implementation commits already stage that migration. #35 therefore withdraws its earlier unilateral v17 reservation. The revised design uses a device-local, non-sensitive state/journal outside SQLite and leaves Prompt/Memory schema ownership untouched.

This report is `source_report`, not runtime evidence.

## Worktree and writer audit

The repository root already contained three unrelated untracked paths before this task:

```text
.trellis/tasks/08-13-prompt-memory-feature-wave-skill/
docs/images/视觉-1/
docs/images/视觉/
```

They remain outside this task. Other dirty worktrees include active #41, #55-adjacent, resolver, issue 73/77, controlled-write and community lanes. No file from those worktrees is an implementation input and no writer is authorized outside the dedicated #35 worktree.

Existing branch ownership relevant to #35:

| Lane | Existing owner surface | #35 rule |
| --- | --- | --- |
| #55 Unified Change Plan | `change_plan.rs`, provider commands/service, legacy Provider UI/hooks/API, schema tests | Consume frozen secret contract; do not edit its worktree. #35 keeps new secret modules isolated and publishes an immutable handoff. |
| #41 Configuration Apply | apply job/coordinator, Provider lease, backup/readback/recovery and V2 apply workspace | Consume prepare/resolve one-shot capability and stable error mapping; never store material in job/event/backup. Do not edit its worktree. |
| Prompt/Memory V2 + native | V2 shell/tokens/pages/standalone builder plus the separately frozen SQLite v17 lane | Reuse shell/tokens; isolate Models/Credentials; #35 adds no schema/version and does not modify Prompt/Memory owner files. |

## Current code facts

### Secret-bearing source today

- `Provider.settings_config` is a generic `serde_json::Value` persisted as `providers.settings_config`.
- Codex canonical storage currently reads `auth.OPENAI_API_KEY` and can also recover `experimental_bearer_token` from TOML.
- `Provider::resolve_usage_credentials` returns a copied `String` API key for balance/usage paths.
- provider save/switch/live-backfill paths can move a live token back into the Provider record.
- V1 `useApiKeyState` and `ProviderForm` keep the key in renderer state and submit it with the full Provider.
- public provider commands return serializable `Provider` objects; a failed migration therefore needs an explicit redacted view or serialization boundary.

The 2026-08-15 V6 supplemental static scan expanded the exact source surface beyond those initial examples:

- Add/Edit dialogs, `useCodexProviderFeatures`, Codex forms/sections/editor, Provider card, `src/lib/api/usage.ts` and `src/config/codexTemplates.ts` still inspect, pass or synthesize key-shaped Provider fields;
- `deepLinkConfigPreview.ts` and `DeepLinkImportDialog.tsx` decode/merge/mask a Codex secret after native ingress instead of rejecting before the renderer event;
- `src-tauri/src/services/sync_protocol.rs` can reach Skills/main-DB mutation from WebDAV/S3 restore before the staged secret gate is complete;
- `src-tauri/src/codex_history_migration.rs` can serialize raw `settingsConfig` into a startup backup;
- existing provider/MCP/renderer/config fixtures still encode plaintext, backfill, masked-preview or empty-key behavior.

The exhaustive current path list, single owner and generator floor now live only in `research/codex-secret-call-graph.md` §9.4 and `research/secret-surface-inventory.md`; this initial source report does not create a narrower allowlist.

### 2026-08-15 V7 supplemental facts (`SNV7-001..006`)

This worker performed a read-only source audit on the current working tree and added **127 exact path/category entries (111 unique exact paths) across six categories** to the §9.4 generator/owner floor. The findings are design inputs only; no dependency resolution, test, build, browser, renderer, server, native runtime or screenshot was run.

1. `SNV7-001` — the Codex `OPENAI_*` environment chain currently spans `env_checker.rs`, `commands/env.rs`, `env_manager.rs`, TS API/types and `EnvWarningBanner`. It can carry names, values and absolute source/backup paths; the manager writes plaintext JSON backups. Process environment, Windows HKCU/HKLM and enumerated shell files are exact sources. The frozen product decision is presence/name/stable source category only, no value/path IPC/UI and no Codex plaintext delete/restore backup.
2. `SNV7-002` — Codex common config is a raw string across `app_config.rs`, legacy migration, settings DAO, config commands/API, `useCodexCommonConfig`, modal/editor, provider live merge and tests. Legacy `config.json`, `.bak`, `.migrated`, SQLite `common_config_codex`, localStorage and live config can all carry it. New secret-bearing TOML must reject pre-write; existing hits become no-value blocked legacy resolution with exact category/revision and never return through raw IPC/localStorage.
3. `SNV7-003` — the public Provider chain still uses `Provider.settingsConfig` through TS type/schema/query/list/sort, Codex form/shared API-key components and MSW/list/update/sort fixtures. The frozen decision is separate Codex internal/public/mutation types, no public `settingsConfig`, and no Codex reachability to shared API-key input.
4. `SNV7-004` — `requestOverrides.ts`/`ProviderForm` can create arbitrary header/body overrides; `provider.rs`, proxy forwarder and hyper client preserve them in generic maps/bytes. Codex mutation must reject them and legacy Codex rows fail closed. Provider-primary material can cross only an owner-private single-send zeroizing transport; any retained non-Codex override is Level 3 debt.
5. `SNV7-005` — stream check currently reaches a network path and stores/renders message/error fields; proxy/failover status/error paths can carry raw upstream URL/error/body/message. Codex cannot run an active secret-bearing check. Diagnostics must be mapped before DB/UI to closed status/category/latency, with generated reflection-canary zero-match.
6. `SNV7-006` — Codex MCP `env` and `http_headers` flow through `McpServer.server`, SQLite `mcp_servers.server_config`, service/commands/UI, `mcp/codex.rs`, live `config.toml`, import/export/backup/WebDAV/S3 sync and fixtures. It is classified as `codexMcpEnvOrHeaderCredential` Level 3 adjacent debt. Static literals such as the existing bearer-token fixture must become runtime-generated canaries. New/moved occurrences fail inventory, but this debt is not counted as Provider-primary Level 2 PASS.

The same V7 reconciliation freezes three cross-flow corrections. Staged import order is `temp authority/token+projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact ImportCutoverCoordinatorContext → staged source validation/scrub/readback → cutover → live owner/binding finalize`; the context is constructed before and is the sole authority for every validation/scrub/readback/cutover step. Public resume is only `{stageId,expectedResumeCas:{revision,digest}}` and returns same-shaped `currentResumeCas`; old nonce/admission/CAS is zero-write. Capture is the existing `list_secret_backend_options` native-snapshot/single-use-`SecretCaptureIntentId` → `begin_secret_capture(intentId,selectedBackend)` flow for `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict`, with summary/owner-card refresh before fresh authority on terminal expiry. Delete and fresh missing readback are separately authorized/checkpointed; persistent revoke requires explicit `Revoke` authorization; all read/delete scope binds device-store instance plus the exact registered backend object.

### 2026-08-15 V9 final-review alignment

- The staged initial-activation result and `resume_staged_import_cutover` result must be different closed types. Resume request data is exactly `{stageId,expectedResumeCas:{revision,digest}}`; every result data arm is exactly `{stageId,currentResumeCas,status,action,issue}`. `activated|alreadyActivated` has `issue=null`; `recoveryRequired` has its typed issue. Result data has no `schemaVersion`, `auditEventId`, candidate, owner, ref, summary or unlisted field; the common envelope owns version/command id and audit is independent. The resume handler is main-integration-owned and outside the 15 #35 command enum/count.
- `LegacySourceCoverageReceipt` is opaque `pub(crate)`, non-Clone/non-Serde/non-Debug, move-only and field-private; store, Provider and #35 sibling modules can only name/move/consume it. Its `pub(crate)` `checked_from_complete_inventory_authority` factory consumes private unforgeable `CompleteLegacySourceInventoryAuthority`, which only main-integration `CodexLegacySourceInventoryBridge` constructs. The receipt atomically binds non-value-derived `LegacySourceInventoryRevision`, fixed complete `CompleteLegacySourceCoverageIdentity`, exact `CurrentLegacySourceExpectations` and category/state-only `AdjacentBlockedLegacySourceObservation` rows. The identity has exactly `currentProviderLiveScrubbable|processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile|commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge`, each with structural revision/presence/count. Internal refs may carry non-value-derived opaque `LegacySourceLocationId`; raw path/raw locator/value/value-derived digest is absent. Startup, each summary/readiness projection, capture options plus claimed-intent revalidation, and Provider-delete preview plus confirm obtain/revalidate a fresh atomic bridge receipt; zero counts without explicit complete all-11-domain proof remain blocked.
- File composition is sequenced: independently compilable/focused-tested #35 core traits/APIs; then #55, #41 and main adapter types in their own owner files; then one main-integration composition step and full Rust gate. `src-tauri/src/secret/backend.rs` cannot refer to concrete external types that have not landed.
- Registration evidence is exactly `15 #35 handlers + 1 separate main-integration resume_staged_import_cutover handler`; the latter is not command 16. `src-tauri/src/lib.rs` needs a static exact-set assertion and the resume path needs phase-by-phase crash UAT.
- Durable `DeviceInstanceId` and process-local `DeviceSecretStoreInstanceId` are different authorities. #35 owns the backend registry/private operation broker. The backend operation set is exactly `CaptureVerify|Validate|ResolveForApply|Delete|Revoke`; fresh missing readback maps to independently authorized `Validate` after durable delete, never Missing/probe.
- The exact verification manifest needs native macOS and native Windows Rust 1.85.0 `cargo check --locked --all-targets` records with source/lock/toolchain/host identity. Current Rust 1.97.1 evidence cannot replace either record.

### Schema and runtime facts

- The audited base, live `origin/main`, and frozen #55 remote all use SQLite `SCHEMA_VERSION = 16`.
- #55 adds change-plan records without reserving schema v17 in its frozen remote.
- `AppState::new` is used by production and many tests; production backend construction must be injectable without making unit tests touch real keyrings.
- canonical repository execution currently uses Node 24.19.0, pnpm 10.12.3, Rust 1.97.1 and `mise run ...`; this does not satisfy the separate locked Rust 1.85.0 `--all-targets` MSRV check required on matching native macOS and Windows runners.

### V2 facts

- `src/v2/pages/models/Page.tsx` is currently empty.
- V2 may not import legacy `src/components`, hooks, API or i18n; Tauri access belongs under `src/v2/shared/platform/tauri`.
- the active V2 visual system is deep-blue Developer Tool using `--fy-*` tokens and a three-region workspace pattern.

## Base choice

`afc317a7...` was selected instead of live `origin/main` because it is the immutable branch containing the accepted V2 Prompt/Memory shell and current repository toolchain closure required by this task. The divergence from live main is an explicit integration risk, not hidden freshness.

## Conflict budget

| Surface | Budget | Mitigation |
| --- | --- | --- |
| `src-tauri/src/database/{mod.rs,schema.rs,tests.rs}` | Prohibited for #35 secret worker | #35 adds no schema/version/table and leaves Prompt/Memory v17 untouched. Canonical owner `main integration` (executor `root/MainIntegrationOwner`) may only make reviewed shared integration changes and must prove no secret state entered SQLite. |
| `src-tauri/src/lib.rs`, `commands/mod.rs`, `store.rs` | Expected, high shared | Canonical owner `main integration` (executor `root/MainIntegrationOwner`) edits serially after module APIs stabilize; no parallel worker owns registration/AppState. |
| Provider/codex/proxy/import shared files | Expected, high with #55/#41 | Canonical owner `main integration` (executor `root/MainIntegrationOwner`) follows the exhaustive call-graph owner map. Do not edit #55/#41 worktrees; integrate only immutable compatible successor SHAs. |
| `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` | Expected, medium | Root-only dependency commit after freeze; direct SecurityFramework/Windows/zeroize, no keyring facade/store fallback. |
| V2 credentials new files | Low | #35 V2 worker owns only exact new credentials data/panel/browser paths; Tauri invoke stays under `shared/platform/tauri`. |
| V2 Page/router/platform index and legacy Provider UI/API | Expected, shared | Canonical owner `main integration` (executor `root/MainIntegrationOwner`) composes after #35/#55/#41 lanes stabilize; no worker edits these shared files. |
| `SNV7-001` env native/TS/UI/locales/backups | Expected, high shared | Sole writer `main integration` (`root/MainIntegrationOwner`). #35 supplies closed DTO/authority requirements only. No env worker may independently edit commands/registration/App/UI; generated-canary and no-plaintext-backup evidence lands in the main integration commit. |
| `SNV7-002` common-config migration/DAO/commands/hooks/modal/live merge | Expected, high shared with Provider/proxy/startup | Sole writer `main integration`. Reject-new and blocked-legacy behavior must land with the Provider/public split; no worker may edit migration/settings/provider/proxy files in parallel. |
| `SNV7-003` public Provider TS/schema/query/list/sort/shared input/MSW | Expected, highest overlap with #55 legacy UI and #41 composition | Sole writer `main integration`. One serial change owns internal/public/mutation split and all list/update/sort fixtures; #55/#41 consume the published types and do not patch these files. |
| `SNV7-004` request overrides/form/native proxy/hyper transport | Expected, high proxy/form overlap | Sole writer `main integration`. Codex rejection and owner-private transport integrate only after #35 native interface stabilizes; retained non-Codex behavior is classified, not opportunistically refactored. |
| `SNV7-005` stream check/proxy/failover DAO/command/API/UI | Expected, high diagnostic overlap | Sole writer `main integration`. Closed mapping must precede DB/UI in one serial patch; no separate diagnostics worker owns shared proxy/failover files. |
| `SNV7-006` MCP app/service/DAO/command/UI/live/export/sync/fixtures | Expected, broad adjacent debt | Sole writer `main integration`. This lane inventories/replaces fixtures and enforces no-regression only; it must not expand #35 Provider-primary implementation scope or claim MCP migration complete. |
| #35 isolated new module files | Low and exclusive | `#35 module` may own only the exact new `src-tauri/src/secret/**`, `src-tauri/src/commands/secret.rs`, `src-tauri/tests/secret_*` and contract fixture surfaces registered in the call graph. It does not own any existing shared file in `SNV7-001..006`. |
| #55 compatible successor | Immutable dependency, no shared writer | #55 owns only its canonical Change Plan contract/DAO/command files and publishes a new compatible source/final SHA. `6859e9ce` with merge-base/ancestor `ca552f4d`, ahead 3 / behind 0 remains incompatible input; main integration adapts it serially. |
| #41 compatible successor | Immutable dependency, no shared writer | #41 owns its configuration-apply runtime/backup/provider coordinator surfaces and publishes an immutable handoff. Shared Provider/UI/API files remain with main integration. |
| #35 core vs external adapters | Strict compile boundary | #35 core trait/API and focused tests land first. `backend.rs` owns registry/five-operation primitives only and may not name absent #55/#41/main concrete types. Each external adapter type lands under its canonical owner; sole `main integration` composes only published types, then runs full Rust. |
| supplemental coverage receipt | High shared discovery boundary | `main integration` is the file owner, but only named `CodexLegacySourceInventoryBridge` can construct private `CompleteLegacySourceInventoryAuthority`. The opaque `pub(crate)` receipt and its `pub(crate)` checked factory atomically bind non-value-derived revision, fixed-11-domain identity, exact current expectations and adjacent observations; fields remain private. Store/Provider/#35 siblings cannot fabricate the authority and only name/move/consume the receipt. Startup/summary/capture/Provider-delete cannot maintain partial inventories, detach proof from data or treat zero counts as complete proof. |
| command registration and staged resume | Shared crate-root boundary | #35 owns exactly 15 secret commands. `main integration` owns the separate resume handler and `lib.rs` 15+1 exact-set assertion; resume never enters `SecretCommandName` or imports the initial activation result type. |
| Rust MSRV verification | Native-host evidence boundary | `main integration` records exact-lock Rust 1.85.0 `--all-targets` on native macOS and native Windows after composition. Current 1.97.1, cross-compile-only or one-host evidence cannot close this row. |

The source-freeze audit must compare every changed path against this budget and the §9.4 generated owner register. Every new existing-source path must have exactly one canonical owner; multiple behavioral rows never mean multiple writers. It also verifies core→owner adapters→main composition order; exact five-field resume result data with null/typed issue arms and envelope/audit separation; receipt/factory `pub(crate)` with private fields; the sole named bridge constructor for unforgeable authority; atomic non-value-derived revision + fixed complete 11-domain identity + exact current expectations + adjacent observations; domain structural revision/presence/count; internal non-value-derived location IDs but no raw path/raw locator/value/value-derived digest; empty-proof rejection and fresh startup/summary/capture/Provider-delete bridge calls; 15+1 registration; durable/process identity separation; #35 registry/broker and five-operation policy; and both native Rust 1.85.0 records. Any new overlap, unregistered path, changed owner, or worker edit to a shared main-integration file requires re-review before implementation continues.

## Reference process adopted

The task follows the repository's Prompt/Memory retrospective:

1. design freeze before source implementation;
2. immutable dependency SHA and conflict budget;
3. one owner per file;
4. small commits per module;
5. module tests before full integration;
6. source freeze before formal runtime evidence.
