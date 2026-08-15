# Issue #35 detailed design overview

## 0. Evidence boundary

This is a static implementation design. No test, build, dependency resolution, browser, renderer, server, native runtime or screenshot may run before all review P0/P1/P2 findings are closed and `DESIGN_FREEZE` is recorded.

Exact DTOs/signatures live in `secret-contract-v1.md`; exact local schema/journal/native APIs live in `device-local-secret-store.md`; the exhaustive existing-file call graph and owner matrix lives in `research/codex-secret-call-graph.md`. This document freezes integration sequence and verifies there is one writer per file.

## 1. File ownership

### 1.1 #35 secret-module owner — new files only

```text
src-tauri/src/secret/types.rs
src-tauri/src/secret/error.rs
src-tauri/src/secret/material.rs
src-tauri/src/secret/backend.rs
src-tauri/src/secret/operation.rs
src-tauri/src/secret/service.rs
src-tauri/src/secret/device_store/mod.rs
src-tauri/src/secret/device_store/schema.rs
src-tauri/src/secret/device_store/atomic.rs
src-tauri/src/secret/device_store/journal.rs
src-tauri/src/secret/device_store/reconcile.rs
src-tauri/src/secret/platform/mod.rs
src-tauri/src/secret/platform/macos.rs
src-tauri/src/secret/platform/windows.rs
src-tauri/src/secret/capture/mod.rs
src-tauri/src/secret/capture/macos.rs
src-tauri/src/secret/capture/windows.rs
src-tauri/src/secret/migration.rs
src-tauri/src/secret/redaction.rs
src-tauri/src/secret/testing.rs
src-tauri/src/secret/mod.rs
src-tauri/src/commands/secret.rs
```

Canonical manifest owner for every path in §§1.1–1.3 is the single literal `#35 module`. Implementation may use temporary subworkers for secret core, platform/capture, V2 and scanner only after assigning disjoint exact paths; those subworker names never enter the changed-path manifest. This owner does not edit existing Provider/proxy/import/schema/registration files.

### 1.2 #35 V2 credentials owner — new files only

```text
src/v2/shared/data/credentials/types.ts
src/v2/shared/data/credentials/decoder.ts
src/v2/shared/data/credentials/port.ts
src/v2/shared/data/credentials/browser.ts
src/v2/shared/data/credentials/index.ts
src/v2/shared/platform/tauri/credentials.ts
src/v2/pages/models/credentials/CredentialsPanel.tsx
src/v2/pages/models/credentials/credentials.css
src/v2/pages/models/credentials/prototype.ts
src/v2/pages/models/credentials/index.ts
tests/v2/shared/data/credentials/types.test.ts
tests/v2/shared/data/credentials/decoder.test.ts
tests/v2/shared/data/credentials/browser.test.ts
tests/v2/pages/models/credentials/CredentialsPanel.test.tsx
tests/v2-browser/credentials.spec.ts
```

Only `shared/platform/tauri/credentials.ts` may directly import/invoke Tauri. The data port depends on that adapter; the previous `shared/data/credentials/tauri.ts` path is forbidden by the existing V2 architecture gate.

### 1.3 #35 scanner owner — new files only

```text
scripts/tasks/secret-surface-scan.mjs
tests/scripts/secret-surface-scan.test.ts
.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-baseline.json
```

### 1.4 `main integration` — all shared existing files

Canonical manifest owner is `main integration`; its executor is `root/MainIntegrationOwner`. That executor serially owns only the exact existing integration/CI paths listed in `research/codex-secret-call-graph.md` §9.4: Cargo/lock, AppState/startup/registration, Provider/DAO/service/live/usage, codex config, balance/model-fetch/coding-plan/usage-script, proxy/takeover/failover, terminal, UniversalProvider, deep link, import/export/restore/WebDAV/S3/sync-protocol cutover, Codex history/template migration, diagnostic/cache/tray/log, legacy renderer/API and V2 composition. The exact set also includes Codex `OPENAI_*` environment inspection/removal, common-config JSON/SQLite/localStorage/live merge, shared public Provider types/schema/query/list/sort/MSW chain, request-override/raw HTTP transport rejection, stream-check/proxy diagnostic projection, and Codex MCP env/header Level-3 inventory. Add/Edit dialogs, Provider list/card, Codex feature hook/form/sections/editor/templates, usage API and deep-link preview/dialog plus their existing tests/fixtures are named in §9.4. This task authority directory and the #35 baseline/scanner paths are not covered by a generic shared-files label.

No worker may edit these files. `database/schema.rs` remains with the existing Prompt/Memory v17 integration; #35 adds no schema/table/version and only verifies absence of secret state in SQLite.

### 1.5 External lane owners

- #55 exclusively owns its Change Plan domain/DAO/commands/canonical DTO and digest changes, including the owner-private admission factory at `src-tauri/src/change_plan/secret_admission.rs` (`crate::change_plan::secret_admission`).
- #41 exclusively owns Configuration Apply coordinator, sanitized backup, Provider lease/write/readback/recovery and apply UI; opaque contexts/ports live at `src-tauri/src/services/configuration_apply/provider.rs` (`crate::services::configuration_apply::provider`).
- Shared registration/composition keeps canonical owner `main integration` and executor `root/MainIntegrationOwner` even if it connects #35/#55/#41.
- #35 first lands a core-only trait/type/backend module that has no source reference to not-yet-present #55/#41/main-integration callback types and can pass its focused core gate alone. Each external lane then lands its immutable owner types. Only the single `main integration` owner adds the adapter/composition module and static registration; full-crate Rust verification runs after that composition. `secret/backend.rs` never pre-creates or directly imports another lane's unpublished type.

Any owner change requires stopping the affected writer and updating this matrix before edits. Changed-path manifest must resolve every path to exactly one owner.

## 2. AppState and dependency construction

Target shape:

```text
AppState {
  db: Arc<Database>,
  secret_service: Arc<SecretService>,
  ...existing fields
}
```

Production boot has two non-substitutable construction steps: `SecretBootstrap::open(&AppHandle) -> Result<OpenedDeviceLocalSecretStore>` resolves and locks the private root before SQLite open; `AppState::new_production(db: Arc<Database>, app_handle: AppHandle, opened_store: OpenedDeviceLocalSecretStore) -> Result<AppState>` consumes that exact non-cloneable handle. DB preflight receives only a field-private, non-clone `pub(crate)` opaque token borrowed from the opened handle (or an equivalent handle method), never a private type illegally exposed across sibling modules and never a path. DB open uses `open_preflight_without_backup`; the constructor cannot reopen from a path or emit an automatic backup. The same service reconciles journal/legacy state plus the complete no-value supplemental-source coverage receipt before returning a private `PreparedProductionAppState`. Crate-root setup then consumes it, calls `app.manage`, proves registration of exactly 15 #35 commands plus the separately owned `resume_staged_import_cutover` handler, and only for a Clean outcome creates the first sanitized backup, publishes the consumer gate and starts workers. A Blocked outcome still manages the same AppState and publishes a scrubbed blocker but starts no backup/worker/consumer; later repair resumes with the same service/handle. The handle moves into `SecretService.store_lifetime=Production(...)`, so the service Arc held by AppState retains the lock until teardown. A private production-only assembly value moves `SecretServiceDeps { authority, backends, operationBroker: Arc<BackendOperationBroker>, changePlans, mutationGate, capture, clock, ids }` into the service; `SecretService` directly retains that sole broker Arc, whose private registries cannot be injected, replaced or extracted by a caller. `SecretService` neither owns the DB nor acquires a Provider lease. Existing AppState fields and their visibility remain unchanged; the additive `secret_service`/construction-seal fields and token are store-private. Test/integration crates use the sole `#[cfg(any(test, feature="test-hooks"))]` public opaque `fyagent_lib::test_support::AppStateBuilder::new()`; only `with_database(Arc<Database>)` may preserve an existing non-secret DB identity. Secret behavior accepts only a closed no-value fixture mode (`inMemory|lockedRead|deniedRead|backendUnavailable|verifyMismatchOnce|oldDeleteFailOnce`) and internally constructs matching deps and broker; there is no broker/registry setter or extractor, and raw root/path, `SecretTestDeps`, traits, material and service constructors are not public. The #41/main-integration coordinator receives managed DB/service access and alone creates opaque already-held Provider/import contexts.

Every current `AppState::new(db)` call site in startup/tests/sync support is inventoried and migrated. Runtime import/sync reuses managed AppState; ordinary tests never touch real Keychain/CredMan. Real native tests are ignored plus env-gated.

## 3. Thread and lock sequence

Local-only order is:

```text
StoreLifetimeFileLock
  -> capture reservation (guard released before dialog)
  -> SecretMutationGate
  -> durable journal/state I/O
  -> exact backend-instance mutex
```

Cross-authority activation/import order is:

```text
#41-held Provider lease or main-integration-held ImportCutoverCoordinatorContext
  -> final admitted baseline receipt
  -> SecretMutationGate
  -> durable journal/state I/O + exact backend-instance mutex
  -> use the passed lease-bound transaction port; never acquire Provider/DB inside the gate
```

- UI dialog: Tauri main thread + oneshot; no service/DB/backend lock and no I/O.
- local file and platform store calls: `tauri::async_runtime::spawn_blocking`.
- `SecretMaterial` never crosses await.
- #41 order is one-way for each distinct operation: activation-specific prepare/confirm → activation lease/baseline → SecretMutationGate/compare/CAS/scrub → release; then apply readiness/plan → apply prepare/confirm → new lease/baseline/backup → SecretMutationGate/backend read → owner-private consuming executor/readback → release.
- secret code holding mutation gate never tries to acquire Provider lease; Provider code never shows dialog while holding lease/DB lock.
- staged import order is fixed: temp object scan/token/projection → #55 admission → main-integration authority-match receipt → #35 prepare/confirm → already-held cutover context. It binds `tempDatabaseDurableObjectId + fresh process nonce + stage/owner/row revision` into admission, backend scope, context and single-use `StagedSecretOwnerToken`; only after cutover does DAO mint a current `ExistingSecretOwnerToken`. Neither token can stand in for the other. Every cancel/prepared failure consumes backend state and terminates that admission. Crash resume accepts only `stageId + expectedResumeCas{revision,digest}` and every closed result arm returns `stageId + currentResumeCas + status/action/issue` without candidate/owner/ref/summary; it proves the durable stage, reconciles the old admission terminal state and mints a fresh live identity/recovery admission plus new CAS before continuing the exact checkpoint. Stale/replay is zero-write.

## 4. Public command boundary

Exact requests/results/errors are in `secret-contract-v1.md` §9. Commands include list summaries/backend options, begin capture/replace/rotate, candidate list/discard, policy lock, delete impact/delete, legacy scan/reconcile report and audit page.

Transport wrapper generates `commandId` before decoding, rejects schema/unknown fields without echo and returns only `SecretCommandSuccess<T>` or `SecretCommandError`. Native-only candidate activation and prepare/confirm/resolve are not Tauri commands. Backend-option discovery atomically snapshots current owner/binding/legacy state and returns a single-use capture-intent id; begin-capture accepts only that id plus the selected registered backend. `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` all enter this typed flow, so renderer never fabricates `OwnerBindingExpectation`. A cancelled/expired/consumed readiness maps to a fresh executable destination rather than repeating its old operation id: delete refreshes delete impact, recovery refreshes recovery impact, apply/activation reopens its Change Plan, and staged import uses its exact main-integration resume CAS. Generic commandless `retry` is forbidden; each retryable code/context names one command/fixed-runtime route or becomes non-retryable `none`.

The renderer cannot supply timestamp, operation id, backend locator, ref display or material. `SecretRefDisplay` is Serialize/output-only in Rust and has no request/test-fixture decoder; service-generated identifiers use strict newtypes. Concrete Agent requests fail before store/backend access. Public field names also pass the canonical forbidden-key scanner; non-sensitive binding structure uses `bindingState`/`ownerBindingState`, never the ambiguous forbidden key `credential`.

## 5. Backend and sealed material use

`SecretBackend` exact trait covers instance/capability, write, read, verify/probe, prepare/confirm and delete/revoke. Read is required for post-write verification and existing-binding legacy equality. Durable records/journals use strict `DeviceInstanceId=dev_*`; process-local `DeviceSecretStoreInstanceId` is regenerated per opened store, is non-persistent/non-Serde/non-Clone and seals only live handle/scope/pending/receipt. Every live handle validates both identities plus an exact private registered-object binding; every raw platform return includes observed backend/device generations, which are checked before material/receipt exits the wrapper. `SecretService` holds one stateful `Arc<BackendOperationBroker>` that privately owns capture-intent, capability and pending registries and atomically performs mint/claim/fresh-revalidate/terminalize. Each authorized read callback is paired to its exact operation/owner/ref/consumer/sink scope before material is exposed; operation-context fields/factories are private and consume opaque plan/readiness/journal/runtime/staged authority, so a crate sibling cannot assemble a caller-chosen scope from scalars. Missing hardware instance never falls back. The hardware policy schema has five operations; every fresh missing-readback uses `validate` confirmation policy while retaining its own authorization and durable checkpoint. Ordinary read/probe can return only a non-persistable revocation hint. Central/device revocation requires explicit `observe_revocation_once` consuming Revoke authorization and minting a non-clone receipt bound to the registered object plus complete store/ref/record/binding/backend/device/capability CAS; persistence fresh-revalidates and consumes it. Not-found/locked/denied/unavailable never synthesize or transplant revocation.

`SecretMaterial(Zeroizing<Vec<u8>>)` has no public byte getter. A sealed, named implementation set receives a borrow only inside the service call and returns a closed non-sensitive result; the result type prevents returning String/Vec/material. Type safety cannot prove that an allowed writer never copies bytes into its exact authorized sink, because projection/header construction is its purpose. Compile/static tests prove no material traits/generic getter/result escape, while an implementation allowlist + code audit verifies every transient copy is limited to the plan-bound sink and is not retained in Provider/job/event/log/cache.

Async proxy/HTTP consumers use dedicated service methods that perform final header construction and exactly one request send inside the native boundary; they never borrow material across await or store it in Provider/AuthInfo. Primary coding-plan, usage/balance and model-fetch each have a named closed request type rather than a generic URL/header executor.

## 6. Operation sequences

### 6.1 Capture candidate

```text
list backend options(owner,purpose,intent)
  -> native snapshots owner/binding/legacy authority
  -> single-use SecretCaptureIntentId
  -> renderer selects registered backend
  -> begin capture(intent id, backend id)
  -> claim/revalidate exact snapshot
  -> native dialog
  -> durable intent
  -> backend write/read/constant-time verify
  -> backendApplied
  -> local state: record + verifiedPendingPlan candidate
  -> stateFinalized + audit + terminal
```

Crash at intent with present-but-unverified entry compensates delete. Crash after backendApplied may finish candidate only if expectations still match. StateFinalized candidate is never auto-bound.

### 6.2 Activate candidate

Only a single-consume #55 candidate-activation admission can start. #41 prepares the candidate read/compare authorization plus, when applicable, independent old-record delete and old-record missing-readback authorizations; it completes every required hardware confirmation before acquiring the activation lease, then passes an opaque already-held lease/final-baseline context plus the consuming prepared bundle:

```text
re-read plan/candidate/record/backend/capability/owner expectations
  -> resolve complete LegacySourceExpectation set and per-source revisions
  -> fresh-read each exact source plus candidate/backend record
  -> candidateEquality: constant-time compare every old value with candidate
     explicitReplacement: validate approved exact source set/revisions only
  -> durable intent
  -> exact local binding CAS + candidate activated
  -> stateFinalized
  -> passed lease-bound Provider transaction scrubs only the exact approved LegacySourceRef set
  -> structural scrub readback
  -> providerFinalized
  -> optional old-entry delete
  -> durable delete checkpoint
  -> independently authorized fresh missing readback
  -> supersession tombstone
  -> audit + terminal
```

After binding switch but before providerFinalized, the owner stays `bindingState=bound`; its ref is `availability=stale` with `SECRET_OPERATION_RECOVERY_REQUIRED`, and the candidate is `cleanupRequired`. Every Codex material consumer remains blocked. Recovery/explicit cleanup may finish the already-admitted scrub but never creates a new approval.

Capture compensation, intentional-delete finalization and post-Provider-commit owner detach are also durable recovery kinds. Device-local state, recovery CAS preimage, journal pointer, impact/result and startup dispatch are the same four-arm tagged union; they do not collapse all failures into activation cleanup. Capture compensation and delete finalization each prepare delete and missing-readback as separate slots and persist `backendApplied` between them; deleteFinalization can execute an admitted delete after an intent-only crash. Exactly eight operation journals exist and there is no generic ninth recovery operation. Candidate explicit discard/expiry remains `verifiedPendingPlan` with immutable `pendingTerminalDisposition=discarded|expired` and a reachable discard action until backend absence is confirmed; then and only then becomes terminal, drops the pending field and returns `refreshSummary` so any subsequent capture/rotation mints fresh authority.

### 6.3 Rotate

Impact contains exact owners/binding revisions/CAS. Capture creates an unbound new candidate. Approved activation switches the full set once. After that switch, old-delete or fresh-missing-readback failure never rolls back: every owner remains bound to the new ref, but that active ref is `stale + SECRET_OPERATION_RECOVERY_REQUIRED`, the candidate is `cleanupRequired` and every consumer is blocked until both durable checkpoints are terminal; the old record remains a pending cleanup subject and is never called superseded merely from a delete return value.

### 6.4 Delete/revoke

Impact/operation/CAS is rechecked. Durable intent precedes delete. User delete recovery has an admitted-delete slot followed by an independently authorized fresh missing readback and only then creates revoked tombstone source=userDelete. Central/device revoke uses explicit `observe_revocation_once` after consuming Revoke authorization; a read/probe hint alone cannot persist state. Unexpected item-not-found without admitted revoke remains missing. Bindings remain for impact/recovery. Provider-row deletion is separate and reads binding plus legacy occurrences orthogonally. Any legacy occurrence returns `blockedLegacyResolutionRequired`, no impact id/delete authority and a no-value source summary whose action enters the typed capture-intent flow. Only a no-legacy bound/unbound preview can proceed; the bound arm states `secretRetained=true`, sorted remaining owners/orphan state and the need for a separate backend-delete confirmation. Crash after Provider commit uses `ownerDetachFinalization`.

### 6.5 Prepare and resolve

Candidate activation and live apply are separate #55 plans and separate Provider leases. Only after activation has released its lease and the owner is bound may #55 create `SecretApplyPlanProjection`. `PreparedSecretCapability` is process-local, by-value, single-use and material-free. Its id remains operation-module private: the bundle's `claim_role_for_revalidation` atomically moves registry state `prepared → revalidating` before moving that exact role slot and returns one claimed object; discard uses the same owner to terminalize it. Revalidation checks plan/operation/owner/ref/record/binding-set/backend/device/capability/consumer/sink/expiry after the apply Provider lease and immediately before acquisition. Any mismatch invokes no writer and mutates no target.

Hardware prepare returns operation-scoped pending confirmation/step. Cancel, expiry and replay clear/terminate the pending entry without altering stable summary. Confirmation step never enters list state, job store or backup.

Codex v1 admits exactly two path-free `CodexLiveSecretSinkId` values: `codexAuthJsonOpenAiApiKey` for the API-key slot in `auth.json`, and `codexConfigTomlExperimentalBearerToken` for the bearer-token slot in `config.toml`. Each target/rollback credential projection has exactly one `liveSinkId`. The same ID is bound into #55's plan digest, #41's owner-private role-specific writer, readback receipt and final baseline; no path string, OAuth field or inferred/unknown sink may substitute.

Cleanup is a third operation-scoped preparation boundary, not an exception to lock order. `get_cleanup_impact` identifies the exact recovery CAS/steps; `prepare_cleanup` creates material-free active-record compare plus optional old-record delete and fresh-missing-readback authorizations, `confirm_cleanup` completes every hardware prompt with no Provider lease held, and only then may #41 obtain the cleanup lease and pass a consuming bundle into `retry_cleanup`. Local capture/delete recovery uses the same independent delete/readback slot discipline without a Provider lease. Cancel/expiry/replay invokes no scrub/delete and leaves the existing recovery row unchanged. No UI opens while any Provider lease, `SecretMutationGate` or backend mutex is held.

## 7. Codex public/private split

- `Provider` stays internal. Codex renderer routes use exact `CodexProviderPublicDto`, mutations use `CodexProviderMutationDto`, live read uses `CodexLiveStructuralProjection`, and credential summary is joined from device-local authority. Shared `src/types`, schema, query/list/sort/drag, MSW and update fixtures consume the public DTO for Codex and never retain internal `settingsConfig`.
- Codex mutation DTO has no auth/token field and rejects unknown secret-shaped fields before DB/live mutation.
- raw `read_live_provider_settings`, failover `Vec<Provider>`, UniversalProvider conversion and deep-link credential ingress are blocked/sanitized for Codex.
- Codex shared API-key input is disconnected. `OPENAI_*` environment inspection returns name/presence/stable source only and creates no plaintext backup; secret-bearing common-config TOML is rejected for new input and existing JSON/SQLite/localStorage/live occurrences become blocked typed legacy sources. A canonical inventory keeps current-scrubbable `LegacySourceRef` occurrences separate from adjacent-blocked process/HKCU/HKLM/shell/common-config observations. Its `pub(crate)` opaque receipt is nameable by store/provider siblings but field-private and can be minted only by the named main-integration bridge consuming a private full-inventory authority; it binds a non-value-derived inventory revision and exact current-plus-ten-supplemental-domain structural revision/presence/count proof. Startup Clean, capture/owner projection and Provider-delete blocking each fresh-revalidate through that bridge. Empty observations without all-domain proof are incomplete, and no observation/receipt carries a value, path, locator or value-derived digest.
- arbitrary Codex request header/body override is rejected and existing hits fail closed; main secret never enters shared `HeaderMap`/raw transport buffers. Stream-check/proxy health maps URL/network/upstream data to closed status/category/latency before DB/query/UI.
- all TOML top-level, active, inactive and inline token occurrences are enumerated; active table has no precedence.
- proxy adapter returns non-sensitive auth requirement, not `AuthInfo(String)`; final forwarder resolves per attempt.
- secret readiness failure is terminal/circuit-neutral; only network/upstream failure advances failover.
- generic QuickJS usage and terminal are forbidden consumers in MVP; fixed usage/balance/model-fetch and the primary coding-plan API-key path are dedicated native consumers. The latter is exactly `codingPlanUsageProbe` with a closed `CodingPlanPrimaryAdapter`; generic `UsageProbeKind` remains `Usage|Balance` only.
- proxy, fixed usage/balance/model-fetch and primary coding-plan create a dedicated credential-bearing client with `redirect::Policy::none()`; redirects are not followed, Authorization is never forwarded to a second origin/request, and 3xx mapping is stable and non-sensitive.
- takeover/live backup stores structural owner/ref placeholders only; OAuth is preserved in place and never copied. Codex MCP env/header material is a separately named Level-3 debt with exact DB/live/export/sync occurrences; it is never counted as Provider-primary Level-2 success.

Each row and its focused test is enumerated in `research/codex-secret-call-graph.md` §5–§9; no omitted shared path may be treated as follow-up inside `codex_feature_runtime`.

## 8. Legacy reconcile

Discovery builds a typed list before any action. Source categories cover Provider/live auth, TOML top/active/inactive/inline, staging origin and unsupported aliases/purposes.

```text
no binding + one distinct value -> verified candidate, no bind/scrub
no binding + distinct values -> sourcesConflict
binding + every occurrence constant-time equal -> scrub-only candidate
binding + different occurrence -> bindingConflict
binding + locked/denied/unavailable read -> bindingComparisonPending
```

Automatic one-value migration/scrub-only uses `candidateEquality`. Conflict recovery first requests backend options; native snapshots the exact current owner-binding and complete legacy expectations into a single-use capture intent, then the selected backend opens native replacement capture and an approved `explicitReplacement` activation. It requires exact old source set/revisions but deliberately does not require the old and new values to match. Renderer/generic retry never chooses a value or constructs the hidden expectation. Public Provider projection is scrubbed even while internal plaintext is retained for recovery.

Startup opens one `OpenedDeviceLocalSecretStore` before DB preflight and consumes it into AppState; an empty DB writes token-free Provider metadata before a current-owner token can stage the live candidate. The same service reconciles journals plus current/supplemental source coverage before `app.manage/static registration`; the registration receipt proves 15 #35 commands and the separate main-integration resume handler, and sanitized backup/gate/workers follow only a Clean outcome. Every hot import/restore/sync path uses this exact order: durable temp-object identity + fresh process nonce + `StagedSecretOwnerToken`/projection → #55 admission → main-integration authority-match receipt → #35 prepare/confirm → `ImportCutoverCoordinatorContext` → cutover receipt → post-cutover current-owner token. Post-crash resume accepts only stage id plus revision/digest CAS and every result arm returns current CAS without candidate/owner/ref/summary; it reopens/proves the durable stage, terminalizes old admission and creates fresh process/recovery authority and new CAS, never reusing an old nonce, admission or pending bundle. Export/backup cannot publish while migration/recovery is unresolved. Historical/user-owned artifacts are v1 scan/report-only and have no cleanup projection/command; only current registered sources named by an approved activation may be scrubbed.

## 9. V2 UI/data sequence

Credentials panel consumes a pure port:

```text
browser fixture OR platform/tauri credentials adapter
  -> strict decoder
  -> owner/ref/candidate/readiness state
  -> list / status card / candidate plan / impact confirmation
```

No text/password value state exists. UX explicitly distinguishes policy/backend lock, missing/revoked, candidate/ready, cleanup required and unavailable hardware. Error actions are total and unique for the exact condition/source and never replay a terminal readiness id. Destructive dialogs show all owner impacts and no-fallback result. Provider deletion with no legacy separately says the secret is retained, lists remaining owners/orphan state and links to a separate secret-delete flow; any legacy occurrence instead shows a blocked no-impact-id resolution card and no confirm button. Four target viewports and keyboard/screen-reader state are frozen in the later visual brief; generated reference is not runtime evidence.

## 10. Focused module tests after freeze

Planned suites (not run during design review):

| Module | Focused coverage |
| --- | --- |
| `secret/types,error,material` | strict ids/serde/envelopes, invalid states, no material traits |
| `device_store` | permissions/ACL seam, canonical hash, atomic replace, lifetime opened handle, exactly eight tagged journals, four tagged recovery/CAS arms, every crash phase, binding CAS |
| `platform` | in-memory default; gated macOS/Windows CRUD/readback/delete/missing |
| `capture` | cancel/concurrency/shutdown/oneshot; gated native UAT |
| `service` | capture-intent claim, candidate, activate, rotate, explicit-discard/expiry disposition, independent delete/missing-readback checkpoints, explicit revoke scope receipt, capability replay/expiry/drift, device-store + exact registered-handle binding, sealed results |
| `migration` | every JSON/TOML/env/common-config location, equality/explicit replacement/conflict, staged-owner/admission/prepare/import-cutover/discard/revision-digest resume authority, restore/sync/sync-protocol cutover |
| Provider/consumers | public DTO full query/list/sort/MSW chain, mutation/request-override reject, delete legacy-blocked/retention previews, Add/Edit/card/forms/templates/usage API, env/common-config, stream/proxy stable diagnostics, proxy/failover/fixed usage/balance/model-fetch/coding-plan, terminal/deeplink renderer-before-preview/universal negative paths |
| V2 | decoder/port/browser/panel, actions, keyboard, four viewports |
| scanner | four levels, exact canary paths, baseline/no-regression and no broad waiver |

Formal source freeze adds full integration order: migrate → candidate/plan → activation → readiness/apply → proxy/usage/model-fetch/coding-plan → rotate → delete/revoke/provider-detach → import/restore/sync → artifact scan.

## 11. Scanner semantics

- Level 1 `contract_schema`: schema/AST/fixture forbidden keys, raw Value and material-shaped command args.
- Level 2 `codex_feature_runtime`: generated canary in every enumerated Codex runtime artifact; exact reviewed sink is asserted then cleaned, not allowlisted.
- Level 2 manifest must explicitly enumerate the device-local root and its compiled truth: `state.json`, `journal/**`, `audit/**`, recognized durable-replace temp files and validated Windows `.retired-*` tombstones. A required root that is absent unexpectedly or unreadable fails closed; it is never silently skipped.
- Level 3 `repository_static_inventory`: exact adjacent-debt baseline/no-regression, including the named `codexMcpEnvOrHeaderCredential` DB/live/export/sync chain and generated-canary fixtures.
- Level 4 `repository_runtime_global=NOT_CLAIMED`.

Existing Codex test literals are replaced with generated canaries. Adjacent debt can be excluded only from Level 2 and remains visible in Level 3. New literal in a baselined file still fails.

## 12. Native/failure evidence plan

- Source-freeze commit passes local gates, then a pre-evidence push publishes only the dedicated branch SHA; remote readback must equal it.
- Windows x64 detached-checks out exact SHA with clean worktree. ARM64 is not an MVP acceptance substitute.
- Windows real CredMan CRUD/replace/delete/missing = `native_runtime`; CredUI accept/cancel uses JSON `evidence_class=uat` (human label `UAT`). The fixed all-pass failure set has separate `result=pass` items for real never-written/deleted-ref missing, injected locked, injected denied, injected backend unavailable, injected post-write verification failure with compensation cleanup, injected post-switch old-delete failure with recovery, and real interactive capture cancel. Each failure item uses `evidence_class=failure_path` with truthful `evidence_origin=real_os|fault_injection`; capture cancel also has a separate UAT item, and a real denial is additional only when reproducibly induced.
- macOS real non-sync Keychain CRUD/missing = `native_runtime`; CRUD asserts explicit `AccessibleWhenUnlockedThisDeviceOnly` protection and fresh missing readback. NSSecureTextField accept/cancel uses JSON `evidence_class=uat` (human label `UAT`).
- every native run uses random ref, explicit cleanup/readback and artifact scan.
- CI compile/unit is never promoted to native_runtime/UAT. No Windows native + failure evidence means non-DONE.

The finalized exact commands/manifest live in `research/native-evidence-plan.md` and `execution-plan.md`. Any source, harness, fixture, authority or task-contract change invalidates all downstream evidence. After source freeze, the only non-source commits are separately allowlisted: `E` adds sanitized evidence files/index only, `V` adds the final review only, and `G` updates narrow governance/readback fields only. Each tier has its own diff verifier/readback and none may alter `FREEZE_SHA` content.

## 13. Design review closure map

- Product scope/Agent/confirmation/lock/revocation/legacy/destructive/artifact/hardware: `prd.md` + exact contract.
- Crash/sync/v17/capability/provider boundary/import: device-store + call graph + §§1–8.
- TS/Rust compile shape/native APIs/MSRV/AppState/threading/V2 path/evidence/scanner: exact contract + OS research + §§1–12.

Only independent reviewers may mark their findings closed after reading the same immutable design commit. This overview does not self-approve freeze.
