# Issue #55 Change Plan — Detailed design

Status: revision 8 passed independent detailed-design review at Round 8 (`0 P0
/ 0 P1 / 0 P2`). Product review passed at revision 18 and architecture review
passed at revision 23. No implementation, test, build, browser, server,
renderer, or native runtime command is authorized until
`DESIGN_FREEZE=PASS` is recorded.

## 1. Frozen inputs and closure rule

- Repository base: `4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287`.
- UCP terminal handoff: `6859e9ce04970008f4cf8b3d4883b4f70316291a`.
- Existing source implementation freeze:
  `ca552f4d918cacc734f81f7efdef70619da139b8`.
- #35 SecretRef handoff: pending. Until an owner-declared immutable SHA is
  locally verified, only the frozen narrow port, fake counters, synthetic refs,
  and proven credential-free production paths may exist.
- #41 is a downstream consumer. It first receives a docs-only design-contract
  SHA explicitly marked non-consumable, then a separate source-contract SHA
  containing the runnable ledger/worker/DTO/decoder/fixture/guard seam. It must
  not create a shadow Plan or job store.
- First real product slice: Codex Provider create, edit, and switch in normal
  mode. Preview does not execute a Plan, call a model/Provider network client,
  or change Provider/current/live/tray/cache state.
- Completion requires all PRD AC-01..AC-21, including the credential-artifact
  clauses under AC-21. A dependency-gated path may be implemented and verified
  as typed-disabled, but it is not called production-enabled until its exact
  handoff and native evidence exist.

The source closure checklist is:

1. one v2 Plan/job/event ledger, one Provider writer, one confirmation;
2. exact public/private split and three canonical digest vectors;
3. preview writes only its immutable Plan record and no job/event/effect;
4. admission invalidates on expiry, target/baseline/source/secret/precondition
   drift and never recomputes intent;
5. post-admission pre-effect drift terminalizes the owning job as no-effect;
6. create/edit/switch plus all direct Codex bypasses use the protected gate;
7. persisted discovery, cancel, orphan readback, retention, sanitized backup,
   and failure paths are closed;
8. clean/warning/expired/drift/unsupported/secret-missing and all closed
   artifact/candidate states have four-locale accessible projections;
9. focused module gates pass before integration/native evidence;
10. final exact-SHA review, push, GitHub readback, and evidence manifest complete
    without merging main or deploying.

### 1.1 #41 handoff receipts

The early design receipt is
`research/handoffs/issue-41-design-contract.md`. It records
`handoffKind=design_only_non_consumable`, `DESIGN_CONTRACT_HANDOFF_SHA`, branch,
base/source-input SHAs, hashes/paths of PRD/process/design/detailed-design/specs,
schema/canonical/baseline/resource/persistence/reason/one-confirmation decisions,
open #35 facts, #41 thread `01a0004d-52f1-7a30-a137-730bd102c0a1`, sent time,
and readback. It is sent after DESIGN_FREEZE and does not block #55 source work;
#41 may use it for planning only and must not compile/integrate against it.

The consumable receipt is
`research/handoffs/issue-41-consumable-contract.md`. It is created after commit
6 in §13, when the exact SHA contains Rust DTO/canonicalization, TS decoder,
shared fixture, persistence/read/admission/worker/CAS/event/recovery APIs,
device epoch, registered commands, Provider guard, and synchronized specs. It
records `handoffKind=source_contract_consumable`,
`CONSUMABLE_CONTRACT_HANDOFF_SHA`, exact included paths/hashes, local ref,
compatibility commands/results, owner/open items, producer static review on the
same SHA, and #41 readback fields `ackSha`, `consumerBranch`,
`consumerBaseSha`, `compatibilityStatus=pass|blocked`, severity counts, and
`seamFindings`. Its required path manifest includes:

```text
src-tauri/src/change_plan.rs
src-tauri/src/change_plan/**
src-tauri/src/database/dao/change_plan.rs
src-tauri/src/database/schema.rs
src-tauri/src/commands/change_plan.rs
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
src-tauri/src/services/provider/change_commit.rs
src-tauri/src/services/provider/mod.rs
src/lib/api/change-plan.ts
src/lib/query/change-plan.ts
tests/fixtures/changePlanDtoContract.v2.json
tests/fixtures/changePlanCanonicalV2.json
.trellis/spec/backend/unified-change-plan.md
.trellis/spec/backend/codex-provider-configuration.md
.trellis/spec/frontend/index.md
```

#41 verifies:

```text
rtk git cat-file -e <CONSUMABLE_CONTRACT_HANDOFF_SHA>^{commit}
rtk git diff --name-status <ISSUE41_BASE_SHA>..<CONSUMABLE_CONTRACT_HANDOFF_SHA> -- src-tauri/src/change_plan.rs src-tauri/src/change_plan src-tauri/src/database/dao/change_plan.rs src-tauri/src/database/schema.rs src-tauri/src/commands/change_plan.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/services/provider/change_commit.rs src-tauri/src/services/provider/mod.rs src/lib/api/change-plan.ts src/lib/query/change-plan.ts tests/fixtures/changePlanDtoContract.v2.json tests/fixtures/changePlanCanonicalV2.json .trellis/spec/backend/unified-change-plan.md .trellis/spec/backend/codex-provider-configuration.md .trellis/spec/frontend/index.md
rtk mise run rust:test -- change_plan_contract_v2
rtk mise run rust:test -- change_plan_store_v2
rtk mise run rust:test -- change_plan_registration_v2
rtk mise run rust:test -- change_plan_provider_guard_v2
rtk mise run test:unit -- tests/integration/change-plan-cross-layer.test.ts
```

The main thread sends both receipts through the exact #41 thread. The design
notification never blocks #55 source work. #41 integration is permitted only if
`ackSha == CONSUMABLE_CONTRACT_HANDOFF_SHA`, the expected consumer branch/base
match, producer and consumer seam reviews are both `0/0/0`,
`compatibilityStatus=pass`, all path hashes/commands pass, and
`seamFindings=[]`. Missing/mismatched paths, nonzero exits, the wrong SHA,
`blocked`, or any P0/P1/P2 keeps #41 integration blocked.

## 2. Current-to-target source map

### 2.1 Change Plan backend

| File | Detailed change | Owner |
| --- | --- | --- |
| `src-tauri/src/change_plan.rs` | Keep as the public composition root and v1 compatibility decoder. Remove v1 full-Provider/live-auth digest authorization. Re-export the v2 closed DTO/service only. | CP-core |
| `src-tauri/src/change_plan/contract.rs` | Add all `deny_unknown_fields` v2 request, public projection, private envelope, lifecycle, admission, job, resource, reason, capability, event, purge, and recovery DTOs. | CP-core |
| `src-tauri/src/change_plan/canonical.rs` | Add the only canonical JSON encoder and the intent/baseline/Plan digest constructors; load the shared vectors. | CP-core |
| `src-tauri/src/change_plan/projection.rs` | Add total private-model-to-public-projection conversion and v1 compatibility projection. No DAO/UI default reconstruction. | CP-core |
| `src-tauri/src/change_plan/inspection.rs` | Read-only resource readers, frozen fingerprints, preconditions, source versions, secret metadata status, and deterministic sorted drift reasons. | CP-core |
| `src-tauri/src/change_plan/codex_provider.rs` | Closed create/edit/switch adapter, exact operation matrix, pure provider preparation, expected resource actions, readback predicates, and unsupported/no-change classification. | Provider-adapter |
| `src-tauri/src/change_plan/coordination.rs` | In-process `ChangeMutationCoordinator`, durable provider epoch access, worker instance lease/epoch, cancellation/effect CAS, sync suppression/quarantine. | CP-worker |
| `src-tauri/src/change_plan/admission.rs` | Fixed decision order, envelope re-decode/digest validation, ready invalidation/expiry, atomic consume + owning job creation, same-identity retry. | CP-worker |
| `src-tauri/src/change_plan/worker.rs` | CAS claim, pre-effect recheck, SecretRef resolve port call, effect permit, private commit dispatch, readback, terminal classification, orphan reconcile, recovery recheck. | CP-worker |
| `src-tauri/src/change_plan/capabilities.rs` | Operation capability matrix and #35 dependency reasons. | CP-core |
| `src-tauri/src/change_plan/test_support.rs` | `cfg(test)` clocks, fault points, effect spies, fake readers/writers/secret port; never linked into production IPC. | CP-tests |
| `src-tauri/src/database/dao/change_plan.rs` | Replace v1-only DAO with schema-dispatched v1/v2 reads, atomic CAS transitions/events, scope discovery, retention/purge, epoch row operations. | CP-store |
| `src-tauri/src/database/schema.rs` | Add nullable v2 columns and `change_coordination` idempotently while retaining `SCHEMA_VERSION=16`; add no v17 migration. This lands in the DB-runtime owner epoch after CP-store's DAO-only SHA. | DB-runtime/compat |
| `src-tauri/src/database/backup.rs` | Skip/preserve the four local tables, sanitize application backups, inventory/rewrite unsafe app backups, guard restore/import while Plan/job authority is live, preserve+increment local epoch. | DB-runtime/compat |
| `src-tauri/src/database/mod.rs` | Keep schema 16; expose only narrow database compatibility/open helpers needed by the guarded bootstrap. | DB-runtime/compat |
| `src-tauri/src/store.rs` | `AppState` owns `Arc<DatabaseRuntime>`, never `Arc<Database>` or a raw connection. | DB-runtime |
| `src-tauri/src/commands/change_plan.rs` | Register the complete v2 command set; all arguments are identity/revision/request DTOs and all errors are closed safe enums. | CP-IPC |
| `src-tauri/src/commands/mod.rs` / `src-tauri/src/lib.rs` | Re-export/register each command exactly once; initialize locks/scanner/recovery before business services. | Integration |

`src-tauri/src/change_plan.rs` remains a file module and declares submodules in
`src-tauri/src/change_plan/`; this avoids a high-conflict file move while #35
and #41 are active.

### 2.2 Provider authority and cutover

| File | Detailed change | Owner |
| --- | --- | --- |
| `src-tauri/src/services/provider/mod.rs` | Keep public facade; register new mutation/projection/commit modules. Public add/update/switch/add-draft/Universal writers fail closed for protected Codex operations. | Provider-adapter |
| `src-tauri/src/services/provider/mutation.rs` | `PreparedProviderMutationV2` and pure `prepare_provider_mutation`. | Provider-adapter |
| `src-tauri/src/services/provider/codex_projection.rs` | Pure auth/config/catalog/common/MCP planner and exact readback from injected snapshots. | Provider-adapter |
| `src-tauri/src/services/provider/change_commit.rs` | `ProtectedCodexMutationGate`, Provider-owned coordinator, apply seam, private commit, non-Clone/non-serde `EffectPermit`. | Provider-adapter |
| `src-tauri/src/services/provider/live.rs` / `src-tauri/src/codex_config.rs` | Split pure injected-snapshot projection from IO commit; the current `prepare_codex_config_text_with_model_catalog` is effectful and cannot serve as planner. Reuse only actually pure preparation such as `prepare_codex_provider_live_config`. | Provider-adapter |
| `src-tauri/src/services/provider/endpoints.rs` | Add pure normalized endpoint-set preparation and transaction-owned replacement; public Codex endpoint writes are gated. | Provider-adapter |
| `src-tauri/src/commands/provider.rs` | Guard all six add/update/switch commands plus endpoint and legacy Universal writes before first effect; preserve non-Codex behavior. | Entrypoint-cutover |
| `src-tauri/src/tray.rs` | Codex Provider click emits/focuses safe Plan UI request; navigation failure has zero proxy/menu/provider effects. | Entrypoint-cutover |
| `src-tauri/src/services/profile.rs` / `src-tauri/src/commands/profile.rs` | Reject a Profile containing a Codex Provider delta before autosave/proxy/MCP/current-profile/event effects. | Entrypoint-cutover |
| `src-tauri/src/deeplink/provider.rs` / `src-tauri/src/commands/deeplink.rs` | Parse a closed safe Codex draft, reject secret/cross-resource/unknown fields, and route to Plan UI before Provider/endpoint persistence. | Entrypoint-cutover |
| `src-tauri/src/commands/config.rs` / `src-tauri/src/services/proxy.rs` | Join common-config and proxy hot-switch writers to coordinator/epoch; protected create/edit/switch effects require the private permit. | Entrypoint-cutover |
| `src-tauri/src/settings.rs` | Add a non-repairing exact reader for Plan inspection; the existing repairing effective-current helper is forbidden in planner/query code. | Provider-adapter |
| `src-tauri/src/provider.rs` | Keep stored legacy models private; add prepared/redacted Provider and Universal safe-view types only where domain ownership requires them. | Provider-adapter |
| `src-tauri/src/database/dao/providers.rs` / `universal_providers.rs` | Add versioned CAS readers/writers and private raw reads. Public serialization never returns stored credential data. | Provider-adapter |

### 2.3 SecretRef, Universal, artifact, and DB compatibility

| File | Detailed change | Owner |
| --- | --- | --- |
| `src-tauri/src/change_plan/secret_port.rs` | Define the narrow `SecretRefPort` trait and fixture implementation. The production adapter is added only from #35 exact SHA. | Dependency-integration |
| `src-tauri/src/universal_mutation.rs` | Closed Create/Edit/Duplicate/Delete/Sync request, safe view/revision token, impact snapshot, private one-use permit, reference-native storage projection. | Universal |
| `src-tauri/src/commands/universal_mutation.rs` | Reachable async `mutate_universal_provider` command, safe list/get projections, post-commit bounded event; legacy writes return `universal_mutation_v2_required`. | Universal-IPC |
| `src-tauri/src/credential_artifact.rs` | Composition root and safe IPC projection exports. | Artifact |
| `src-tauri/src/credential_artifact/contract.rs` | Closed source/candidate/attempt/step/lineage/integrity/ack DTOs and legal-combination validation. | Artifact |
| `src-tauri/src/credential_artifact/store.rs` | Device-local `credential-artifacts-v1.sqlite3` schema v1, CAS transactions, strict open/integrity/version handling. | Artifact |
| `src-tauri/src/credential_artifact/lock.rs` | Stable config-dir `credential-artifacts-v1.lock` cross-process exclusive lock and universal order enforcement. | Artifact |
| `src-tauri/src/credential_artifact/scanner.rs` | Acquire global lock first, then enumerate/reread every identity and persist only sticky integrity overlays. | Artifact |
| `src-tauri/src/credential_artifact/actions.rs` | Source migrate/delete and candidate apply/delete with immutable attempts, effect-start fencing, no replay, #35 receipts, safe outcomes. | Artifact |
| `src-tauri/src/credential_artifact/recovery.rs` | Readback-only source/candidate interruption recovery, write-once `DbCompletionAckV1`, receipt clear CAS, authority-unavailable projection. | Artifact |
| `src-tauri/src/credential_artifact/gc.rs` | Joint source/candidate retention and pair-only purge; NeverPublished-only source purge. | Artifact |
| `src-tauri/src/database/compatibility.rs` | Stable marker/lock, deny-unknown header inspection, maintenance drain, v16-to-new-layout staged replacement, Ready receipt, no old-binary SQLite open. | DB-compat |
| `src-tauri/src/database/runtime.rs` | Central closeable/reopenable `DatabaseRuntime` and maintenance admission/drain; replaces scattered assumptions around one permanently live `Mutex<Connection>`. | DB-compat |
| `src-tauri/src/database/remote_effect.rs` | Linear WebDAV/S3 remote-effect tokens plus fsynced non-secret attempt/recovery receipts that gate maintenance and startup reconciliation. | DB-runtime |
| `src-tauri/src/commands/credential_artifact.rs` | Safe list/get/action/recheck commands and bounded invalidation event. | Artifact-IPC |
| `src-tauri/src/services/sync_protocol.rs`, `webdav_sync.rs`, `s3_sync.rs` | Stage remote copies, enter maintenance, preserve local authority, replace/reopen/reinspect, and publish success only after readback. | DB-runtime |
| `src-tauri/src/services/webdav_auto_sync.rs`, `s3_auto_sync.rs` | Participate in drain and sync suppression; never enqueue from ledger/local-authority rows or before replacement readback. | DB-runtime |
| `src-tauri/src/commands/import_export.rs`, `webdav_sync.rs`, `s3_sync.rs` | Route every local/remote import, backup, and restore through `DatabaseRuntime`; no direct live-connection replacement. | DB-runtime |
| `src-tauri/src/commands/sync_support.rs`, `usage.rs` | Post-import Provider/settings sync waits for verified reopen; usage rebuild becomes one maintenance transaction instead of backup-reset-import with escaped handles. | DB-runtime |
| `src-tauri/src/proxy/server.rs`, `provider_router.rs`, `failover_switch.rs`, `usage/logger.rs`, `response_processor.rs` | Replace retained `Arc<Database>` with runtime handles; request/logger paths reacquire typed short guards. `response_processor.rs` is an indirect production participant, not a direct-connection caller. | DB-runtime |
| `src-tauri/src/services/skill.rs` | Replace operation-scoped `&Arc<Database>` with typed runtime guards and prohibit a guard across filesystem/network awaits. | DB-runtime |
| `src-tauri/src/claude_desktop_config.rs` | Remove every production `&Database` entrypoint; file/config work uses a connection-free activity lease and short runtime closures. DB-runtime migrates the boundary, then Provider-adapter serially owns Provider semantics. | DB-runtime then Provider-adapter, serialized |
| `src-tauri/src/codex_history_migration.rs` | Separate FyAgent-main-DB reads/writes from Codex-state SQLite authority; main-DB phases use an activity lease, external phases use `ExternalSqliteAuthority::CodexState`, and the one-shot is stop/join registered. | DB-runtime |
| `src-tauri/src/session_manager/providers/{codex,hermes,opencode}.rs` | Classify external SQLite scan/load/delete callsites; they never join FyAgent main-DB maintenance unless the same operation separately acquires a main-DB activity. | DB-runtime classification |
| `src-tauri/src/services/webdav_sync/archive.rs` | Join skills snapshot/rollback to the same database replacement transaction; publish neither half before verified reopen/readback. | DB-runtime |
| `src-tauri/src/lib.rs` | Integration-main-only serialized wiring migrates startup/periodic backup, Codex history, session-sync, and auto-sync captures after the DB-runtime SHA. | Integration |
| `src-tauri/Cargo.toml` / `Cargo.lock` | Add only reviewed direct dependencies needed for OS file locks, zeroization, IDNA/NFC normalization; lockfile change stays with this module commit. | Integration |

The dependency choices are frozen as `fs2 = 0.4` for portable advisory file
locking, `zeroize = 1` for attempt-memory wrappers, `idna = 1` and
`unicode-normalization = 0.1` for the binding-key contract. If #35 ships an
equivalent already-vendored primitive, integration replaces—not duplicates—the
dependency after compatibility review and returns to detailed-design review if
observable bytes or lock semantics change.

The inventory is equality-grade and separates main-DB authority from indirect
participants and unrelated external SQLite files. The exact 25 production
FyAgent main-DB `.conn` / `lock_conn!` callers at source freeze `ca552f4d` are:

```text
commands/usage.rs
database/backup.rs
database/dao/{change_plan,failover,mcp,profiles,prompts,providers,proxy,
  settings,skills,stream_check,universal_providers,usage_rollup}.rs
database/{migration,mod,schema}.rs
proxy/usage/logger.rs
services/{model_pricing,session_usage,session_usage_codex,
  session_usage_gemini,session_usage_grokbuild,session_usage_opencode,
  usage_stats}.rs
```

The retained-holder inventory is exactly `store.rs` (`AppState`),
`services/proxy.rs` (`ProxyService`), `proxy/server.rs` (`ProxyState`),
`proxy/provider_router.rs`, and `proxy/failover_switch.rs`. Background captures
are `lib.rs`, `services/webdav_auto_sync.rs`, and `services/s3_auto_sync.rs`.
Operation-scoped captures are
`commands/{import_export,webdav_sync,s3_sync,usage,sync_support}.rs` and
`services/skill.rs`. `proxy/response_processor.rs` participates indirectly
through `UsageLogger`; its direct connection matches, plus the matches in
`change_plan.rs` and `services/provider/mod.rs`, are test-only and are recorded
as such rather than misclassified as production holders.

The legacy borrowed-boundary inventory is separate and exact. Production
explicit `&Database`/`&Arc<Database>` entrypoints occur in:

```text
claude_desktop_config.rs
codex_history_migration.rs
settings.rs
proxy/usage/logger.rs
services/{model_pricing,provider/live,proxy,skill,session_usage,
  session_usage_codex,session_usage_gemini,session_usage_grokbuild,
  session_usage_opencode,sync_protocol,webdav_sync,s3_sync,
  webdav_auto_sync,s3_auto_sync}.rs
commands/{import_export,sync_support,usage,webdav_sync,s3_sync}.rs
lib.rs
```

The baseline additionally records all 19 production `impl Database` blocks:
`database/{backup,migration,mod,schema}.rs`, the twelve DAO modules named in
the direct-caller set, `services/session_usage_codex.rs`, and the two
`services/usage_stats.rs` blocks. Expected production
`explicitBorrowedFunctions`, `implDatabaseBlocks`, and `taskCaptures` are empty:
the implementation removes the legacy `Database` facade rather than preserving
an alias that could expose or retain a connection. Domain helpers receive
`DatabaseRuntime` plus a typed activity or a closure-borrowed connection only.
The manifest names the exact baseline function/impl/capture identity, not merely
the file, so a renamed or newly introduced borrowed boundary fails equality.

`legacyDatabasePathUses` is a syntax-aware associated-call/import/re-export/
alias/trait class, not a regex subset. Its production associated-call baseline
is exactly:

```text
commands/import_export.rs:
  Database::list_backups, Database::rename_backup, Database::delete_backup
lib.rs:
  crate::database::Database::stored_user_version_exceeds_supported,
  crate::database::Database::init
codex_history_migration.rs:
  Database::table_exists (3 callsites), Database::has_column (3 callsites)
```

The three backup calls move to runtime-owned backup closures; the two bootstrap
calls move to compatibility/runtime free entrypoints; the six Codex-history
calls move to pure `external_sqlite::{table_exists,has_column}` helpers tagged
`ExternalSqliteAuthority::CodexState`. The production `pub use Database` and
every imported/qualified/aliased legacy type in the union of the direct,
borrowed, holder, and capture files are separate baseline identities and are
removed. Baseline trait/alias implementations are empty but still scanned.
Test-associated calls and imports are recorded under their exact syntax-aware
cfg item/range and move to the runtime test harness or pure schema/external
helpers. Expected production and test legacy type/path sets are both empty;
comments and string literals never count as syntax uses.

The copy/replacement inventory is exactly:

```text
database/{mod,backup,migration,schema,runtime,compatibility}.rs
commands/{import_export,usage,webdav_sync,s3_sync,sync_support}.rs
services/{sync_protocol,webdav_sync,s3_sync,webdav_auto_sync,s3_auto_sync}.rs
services/webdav_sync/archive.rs
lib.rs
store.rs
```

`database/backup.rs` may not return `Result<Connection>`; main/staged/backup
connections are created and consumed inside runtime-owned closures.
`database/mod.rs` may not open the live database before the compatibility
header/marker guard. `database/migration.rs` receives a borrowed candidate
connection. Raw SQLite access to the separate Codex, Hermes, and OpenCode
databases in `codex_history_migration.rs`,
`services/session_usage_opencode.rs`, and
`session_manager/providers/{codex,hermes,opencode}.rs` is classified under the
closed `ExternalSqliteAuthority` manifest and never joins the FyAgent-main-DB
participant count.

`scripts/change-plan/database-runtime-inventory.json` freezes baseline and
expected sets for `arcHolders`, `backgroundCaptures`, `operationCaptures`,
`explicitBorrowedFunctions`, `implDatabaseBlocks`, `taskCaptures`,
`legacyDatabasePathUses`,
`directMainConnectionCallers`, `rawOwnedConnections`,
`externalSqliteAuthorities`, `copyReplacementParticipants`, participant enum
variants, maintenance reasons, and the disposition of each baseline item.
`scripts/change-plan/check-database-runtime-inventory.mjs` discovers the same
classes from both `ca552f4d` and the current tree, compares sorted sets for
equality (never subset membership), and emits a unified diff. The public
read-only task is:

```text
rtk mise run change-plan:db-inventory -- --baseline-sha ca552f4d918cacc734f81f7efdef70619da139b8 --manifest scripts/change-plan/database-runtime-inventory.json
```

It fails on an unclassified production `Arc<Database>`, `&Database`,
`impl Database`, `Database::…` associated path, `Database` import/re-export,
type alias/trait implementation, `.conn`, `lock_conn!`,
`Mutex<Connection>`, escaped `Result<Connection>`, main-DB `Connection::open*`,
participant/reason variant, holder, or replacement entrypoint. Test-only and
external-authority classifications are explicit manifest entries. DB-runtime
owns this full inventory except that Integration main serially owns `lib.rs`
after the DB-runtime commit. Test classification includes `database/tests.rs`,
`change_plan.rs`, `services/provider/mod.rs`,
`proxy/response_processor.rs`, `src-tauri/tests/support.rs`, and the complete
`services/proxy.rs` test module at baseline lines 3225-7379 plus its two
production-area test-only items. The checker uses syntax-aware cfg ranges and
cannot truncate a file at its first `#[cfg(test)]`; expected tests also use the
runtime harness and retain no old facade escape.

### 2.4 Renderer and shared fixtures

| File | Detailed change | Owner |
| --- | --- | --- |
| `src/lib/api/change-plan.ts` | Replace permissive v1 schemas with strict discriminated v2 Zod decoders from `unknown`; export typed API only. | FE-platform |
| `src/lib/query/change-plan.ts` | Add Plan/job/scope discovery, abandon/cancel/recheck, sequence-safe invalidation, expiry timer, snapshot-wins policy. | FE-platform |
| `src/lib/api/credential-artifacts.ts` / `src/lib/query/credential-artifacts.ts` | Strict safe source/candidate DTOs and authoritative list/get/event refetch. | FE-platform |
| `src/components/change-plan/ChangePlanFlow.tsx` | Make the flow a reducer over backend projections; support create/edit/switch and reload discovery, with no renderer authority. | FE-product |
| `src/components/change-plan/ChangePlanPanel.tsx` | Full-screen shell, focus trap/return, stable header/footer, interaction lock. | FE-product |
| `src/components/change-plan/ChangePlanPreview.tsx` | Resource/action/risk/warning/backup/credential/privacy/expiry preview. | FE-product |
| `src/components/change-plan/ChangePlanLifecycleNotice.tsx` | No-change/expired/drift/unsupported/secret dependency projections. | FE-product |
| `src/components/change-plan/ChangePlanJobProgress.tsx` | Planned/running/cancelled/reconciling/terminal/recovery actions. | FE-product |
| `src/components/change-plan/ChangePlanResourceResults.tsx` | Per-resource authoritative readback and limitation/recovery status. | FE-product |
| `src/components/change-plan/ChangePlanSafetyBanner.tsx` | Root-discovered source/candidate safety alert independent of Provider rows. | FE-product |
| `src/components/change-plan/CredentialArtifactPanel.tsx` / `CredentialCandidateCard.tsx` | Total safe source/candidate projections and only backend-authorized actions. | FE-product |
| `src/hooks/useProviderActions.ts` | Route Codex create/edit/switch drafts to Plan, not direct mutations; preserve other apps. | FE-cutover |
| `src/components/providers/AddProviderDialog.tsx` / `EditProviderDialog.tsx` / `src/App.tsx` | Keep local draft, preallocate/create scope ID, mount one Plan flow, discover/resume by scope. | FE-cutover |
| `src/lib/query/mutations.ts` | Codex add/update no longer call legacy `_with_result`; they hand a typed launch draft to the Plan launcher. | FE-cutover |
| `src/hooks/useChangePlanLauncher.ts` / `src/lib/change-plan/providerDraft.ts` | Unify Provider UI/tray/deep-link launch intent and reject raw auth/usage/path/receipt fields before request construction. | FE-cutover |
| `src/components/providers/forms/EndpointSpeedTest.tsx` | For Codex create/edit, probe endpoints but mutate only the form-owned draft endpoint set; never call endpoint persistence APIs. Non-Codex retains legacy mode. | FE-cutover |
| `src/components/providers/forms/CodexFormFields.tsx`, `ProviderForm.tsx`, `hooks/useSpeedTestEndpoints.ts` | Thread the normalized draft endpoint set and `persistenceMode=draft` through Codex form state into the Plan request. | FE-cutover |
| `src/lib/api/vscode.ts` | Keep probe/read APIs; direct Codex add/remove is typed `change_plan_required` and never used by a Codex form after cutover. | FE-cutover |
| `src/components/DeepLinkImportDialog.tsx` / `src/lib/api/deeplink.ts` | Replace only the Codex Provider branch with closed safe draft-to-Plan routing; keep other deep-link resource behavior. | FE-cutover |
| `src/components/universal/*` / `src/lib/api/providers.ts` / `src/types.ts` | Replace raw Universal CRUD/two-call sync with safe view and one mutation command; secret-bearing paths typed-disabled until #35. | FE-universal |
| `src/lib/query/universal-providers.ts` | Query/mutation owner for safe list/get and one `mutate` call; invalidates only after a committed outcome. | FE-universal |
| `src/i18n/locales/{zh,zh-TW,en,ja}.json` | Add key-identical copy for every closed Plan/job/artifact/candidate state and action. | FE-product |
| `tests/fixtures/changePlanDtoContract.v2.json` | Rust-authored public DTO/job/event/capability fixture with sentinels. | Contract-tests |
| `tests/fixtures/changePlanCanonicalV2.json` | Canonical bytes plus three expected digests for create-only/create-select/edit/switch. | Contract-tests |
| `tests/fixtures/changePlanPrivateEnvelope.v2.json` | Backend-only decode/digest fixture; never imported by renderer production code. | Contract-tests |
| `tests/fixtures/credentialArtifactContract.v1.json` | Every legal safe lifecycle/outcome plus illegal combinations and sentinel set. | Contract-tests |
| `.mise/tasks/change-plan.toml` / `mise.toml` | Register DB inventory, Chromium lock/prepare, and four evidence tasks with exact usage/effects; include the task file exactly once. | Evidence/task-runtime |
| `.trellis/spec/backend/task-runner-contract.md` / `docs/fyagent/development/mise-tasks.md` | Add the exact include/task/effect/offline contract; regenerate the canonical task document with the repository generator in the same commit. | Evidence/task-runtime |
| `scripts/tasks/task-contract-check.mjs` / `scripts/tasks/host-native.mjs` | Require all new public tasks and add the current-host debug executable resolver plus atomic same-SHA build receipt. | Evidence/task-runtime |
| `scripts/change-plan/evidence.mjs` | Own `ActiveEvidenceSessionV1`, repo lock/pointer/CAS, source-clean admission, readiness, child lifecycle, capture manifests, crash recovery, preview, cleanup, and `--apply` atomic publish. | Evidence/task-runtime |
| `scripts/change-plan/chromium.mjs` / `scripts/change-plan/chromium-lock.v1.json` | Preview/apply the only reviewed lock and prepare its current-host cache; evidence modes are offline-only. | Evidence/task-runtime |
| `scripts/change-plan/check-database-runtime-inventory.mjs` / `database-runtime-inventory.json` | Compare baseline and current DB holder/caller/participant sets for exact equality. | DB-runtime/compat |
| `package.json` / `pnpm-lock.yaml` / `pnpm-workspace.yaml` | Exact non-range Playwright pin and install-script policy; no implicit browser download. | Integration |
| `tests/e2e/change-plan/{fixtures,renderer.spec,browser.spec}.ts` / `playwright.change-plan.config.ts` | Deterministic state seeding, fixed viewport renderer capture, and browser interaction assertions using only the locked executable. | Evidence/task-runtime |
| `tests/{miseTaskContract,taskDocs,localBuildBoundary,changePlanEvidenceContract}.test.ts` | Task metadata/docs, build receipt, offline browser, transaction/publish, and release-unreachability contracts. | Evidence/task-runtime |
| `src-tauri/src/main.rs` | Debug-only evidence dispatch before Windows user context, Linux WebKit setup, Tauri, store, window, tray, auto-sync, and HTTP initialization; otherwise unchanged. | Integration |
| `src-tauri/src/lib.rs` | Debug-only evidence export/wiring and zero IPC registration, serialized with DB-runtime and final integration wiring. | Integration |
| `src-tauri/src/change_plan.rs` / `src-tauri/src/change_plan/evidence.rs` | `cfg(debug_assertions)`-only module declaration plus headless native/failure runner using real service/DAO/file paths; absent from release and IPC. | CP-core then Evidence/task-runtime, serialized |
| `src-tauri/src/config.rs` / `src-tauri/src/app_store.rs` | Evidence authority precedes persisted config/store paths and fails closed on a partial or invalid envelope. | Evidence/task-runtime |
| `src-tauri/src/codex_config.rs` / `src-tauri/src/settings.rs` | Evidence Codex/settings paths precede persisted overrides; Provider-adapter owns these shared files serially. | Provider-adapter |
| `src-tauri/.gitignore` | Retain the existing `/target/` rule as the sole build-receipt ignore authority; final evidence remains tracked and no repository-local evidence temp path is ignored. | Evidence/task-runtime |
| `.trellis/tasks/08-14-issue-55-change-plan-mainline/research/evidence-inputs.v1.json` / `research/{evidence-manifest,active-evidence-pointer,active-evidence-session,prepared-publication-receipt,final-evidence-snapshot,terminal-evidence-receipt}.schema.json` / `.trellis/tasks/08-14-issue-55-change-plan-mainline/evidence/README.md` | Tracked dependency/design binding, separate non-circular authority/receipt schemas, and publish parent; no repository-local temp directory or ignored final evidence. | Evidence/task-runtime |

## 3. Exact v2 contract and invariants

### 3.1 Request and operation scope

`ChangePlanRequestV2` is an internally tagged, deny-unknown enum:

```text
CodexProviderCreate {
  schemaVersion=2, actorCode, providerId, activation=create_only|create_and_select,
  draft: CodexProviderDraftV2
}
CodexProviderEdit {
  schemaVersion=2, actorCode, providerId, expectedProviderVersion,
  draft: CodexProviderDraftV2
}
CodexProviderSwitch {
  schemaVersion=2, actorCode, targetProviderId
}
```

`CodexProviderDraftV2` has exact normalized non-secret definition fields,
endpoint set, and `ProviderCredentialIntentV1`; it rejects unknowns and raw auth,
API key, token, script, config URL, cross-resource, and absolute-path fields.
`OperationScopeV2={app,operation,subjectId}` is stored explicitly and is the only
latest-discovery key. Create uses the preallocated Provider ID as subject.

### 3.2 Public Plan and private envelope

`PlanPublicProjectionV2` emits every field, using explicit `null`:

```text
schemaVersion, canonicalizationVersion, operationVersion,
planId, planDigest, intentDigest, baselineDigest,
operation, operationScope, intentProjection,
createdAt, expiresAt, status, owningJobId, planRevision,
actorCode, sourceVersions,
affectedResources[], orderedActions[], readbackPredicates[],
recoveryModes[], risks[], warnings[], preconditions[], recoveryHints[],
credentialStatus, privacyNotes[], evidenceNotes[]
```

The private `PlanExecutionEnvelopeV2` stores the exact prepared non-secret
payload, sorted resource expectations, source versions, opaque ref/version
requirements, action parameters, readback predicates, recovery envelopes, and
preconditions. It contains no value, reversible value hash, raw live auth,
absolute path, unrestricted error, or lease. The total projection function is
the only constructor of public DTOs.

Lifecycle is closed to `ready|expired|invalidated|abandoned|consumed`.
`expired|invalidated|abandoned` store their retention anchor/reasons and never
own a job. `consumed` may own exactly one job. Unknown or illegal combinations
fail closed before projection or action derivation.

### 3.3 Canonical identity

`CanonicalValueV2` accepts only null, bool, UTF-8 string, signed i64, array, and
object. It rejects float, out-of-range number, duplicate set key, and unknown
field. Objects sort keys by UTF-8 bytes. Semantic arrays retain declared order;
set arrays sort uniquely by their contract key. Optional fields encode as null.
No implicit Unicode normalization occurs except in the explicitly versioned
Universal binding-key constructor.

Authoritative domains and output are exact:

```text
sha256:<64 lowercase hex>
SHA-256(domain UTF-8 || 0x00 || canonical JSON bytes)

fyagent.change-plan.intent.v2
fyagent.change-plan.baseline.v2
fyagent.change-plan.plan.v2
```

`planDigest` excludes Plan ID, timestamps, expiry, actor, localization, safe
labels, lifecycle, owning job, and presentation order. It includes operation
version, intent/baseline digests, ordered executable actions, resource codes,
private credential requirement digests, preconditions, recovery modes, risks,
warnings, and effect boundaries. Admission re-decodes the envelope and rebuilds
all three canonical inputs; stored digest strings alone never authorize work.

### 3.4 Job and admission

`ChangeJobSnapshotV2` contains identity, schema/operation, monotonic revision and
eventSeq, status, resultCode, observedState, steps/resources, effectStartedAt,
cancelState, worker phase/epoch, recovery, safe diagnostics/reasons,
liveConfigChanged, created/updated/terminal timestamps. Closed statuses are
`planned|running|cancelled|reconciling|succeeded|warning|failed`.

Admission decision order is exact:

1. missing Plan;
2. caller digest versus persisted identity and recomputed envelope digests;
3. consumed same identity returns owning job, otherwise consumed;
4. persisted non-ready lifecycle projection;
5. schema/operation/mode/risk support;
6. expiry (`now >= expiresAt`);
7. fresh sorted resource/source/secret/precondition inspection;
8. one transaction CASes ready to consumed, binds owning job, creates planned
   job and first event.

Every rejection before step 8 creates no job/event/writer effect. Expiry and
drift may update only lifecycle metadata/reasons. `apply_change_plan` accepts
only `planId + planDigest`, returns the planned snapshot immediately, and
schedules the worker; it never accepts a draft or intent.

## 4. Side-effect-free planning and Provider preparation

### 4.1 Read ports and forbidden calls

The planner receives explicit read-only ports:

```text
ProviderSnapshotReader
CurrentProviderReader
DeviceCurrentReader
CodexLiveSnapshotReader
CommonConfigSnapshotReader
ManagedMcpSnapshotReader
ProxyModeReader
ProviderEpochReader
SecretRefMetadataReader
Clock
PlanStore
```

Each reader returns bytes/identity/version or a typed unreadable status and is
forbidden from repair-on-read. The planner has no `AppHandle`, network client,
Provider writer, tray/cache updater, sync publisher, backup API, model fetcher,
or secret resolver. `PlanStore::insert_v2` is the sole allowed write and inserts
only the immutable Plan payload/lifecycle row after all inspection and digest
construction succeeds. `no_change`, unsupported, dependency-unavailable, or
inspection failure inserts nothing executable.

The current v1 `provider_definition_digest(&Provider)` and
`live_projection_digest(read_live_settings(...))` cannot be reused because
those values can contain credential material or value-derived fingerprints.
Preparation must first produce a typed redacted non-secret definition and
separate opaque credential requirements. Live auth contributes only safe
presence/ref/version/source-version predicates supplied by #35; material
correctness is checked as a boolean/stable code only inside the native resolve/
write/readback closure and is never persisted as a digest.

### 4.2 `prepare_provider_mutation`

The pure function receives a draft, optional original safe snapshot, operation,
activation policy, and injected common/live snapshots. It returns:

```text
PreparedProviderMutationV2 {
  operationVersion,
  providerId,
  expectedProviderVersion,
  normalizedNonSecretDefinition,
  normalizedEndpointSet,
  credentialRequirements,
  exactResourceExpectations,
  orderedActions,
  readbackPredicates,
  warnings, risks, preconditions, recoveryModes
}
```

Create-only writes only the Provider row and endpoints. Create-and-select/edit
current/switch declare every actual source backfill, current marker, live file,
common config, and managed MCP action. No implicit action is legal. Proxy
takeover, official target switch, critical risk, malformed/secret-shaped draft,
or required unreadable input returns a typed non-executable outcome before Plan
insert.

The same prepared payload feeds both preview and the private commit. Apply never
reloads target semantics by ID. Reinspection uses the envelope's expected
identity/version/fingerprint and does not rebuild normalized intent.

### 4.3 Side-effect spy contract

`ChangeEffectSpyV2` is test-only and counts at least:

```text
plan_rows, plan_lifecycle_updates, job_rows, event_rows,
provider_rows, endpoint_rows, db_current, device_current,
live_catalog, live_auth, live_config, common_config, managed_mcp,
backup_create, backup_restore, tray_refresh, renderer_cache_publish,
business_sync_enqueue, provider_network, model_network,
secret_resolve, credential_artifact_write
```

Preview tests snapshot managed resources before/after and assert only
`plan_rows=1`; all other counters are zero. Unsupported/no-change/rejected
preview asserts every counter is zero. Lifecycle abandon/expiry/invalidation
tests allow only `plan_lifecycle_updates=1` and no Plan payload rewrite.

## 5. Persistence, compatibility, retention, and backup

### 5.1 Main ledger schema

`schema.rs::create_tables_on_conn` adds the v2 columns listed in `design.md`
with `add_column_if_missing`, nullable and without SQL defaults. It creates
`change_coordination` idempotently. `SCHEMA_VERSION` remains 16 and no migration
match arm is added.

V2 `change_plans` rows use:

- `schema_version=2`, `operation='v2_managed'`, legacy `status='consumed'`;
- redacted inert placeholders in mandatory legacy columns;
- exact public/envelope JSON, three digests, versions, lifecycle, reasons,
  revision, actor, source versions, scope, owning job, and retention anchors.

V1 rows are detected only by `schema_version IS NULL`. A ready v1 Plan returns
`unsupported_schema` and requires preview. A terminal v1 job is projected
read-only; a nonterminal v1 job may run predicate readback only and never replay
its writer. V1 bytes are never rewritten by v2 code.

All v2 job transitions use a single DAO CAS predicate over prior revision,
status, effect-start, cancel-state, owner/epoch, and phase. A successful
transaction increments revision/eventSeq exactly once, updates snapshot fields,
and appends exactly one event. CAS miss reloads and returns the authoritative
snapshot; it never guesses.

### 5.2 Coordination epoch

`change_coordination('codex_provider')` is seeded if absent. Every managed
Provider/current/live/common/MCP/endpoint/import/restore mutation holds
`ChangeMutationCoordinator`, increments the epoch in the same narrow authority
window, and releases only after readback/publication. Read-only Plan inspection,
admission, and effect gate bind/read this row. Overflow at i64 max is a typed
fail-closed error.

Remote import and restore preserve the local epoch and write
`max(local_before, imported_or_zero)+1`. They reject while any ready Plan or
nonterminal/recovery-required job exists. The row is never accepted from remote
authority.

`register_db_change_hook` explicitly ignores the three ledger tables and
`change_coordination`; otherwise a permitted Plan insert would trigger WebDAV/
S3 activity and violate side-effect-free preview. During an admitted effect it
also routes through the coordinator's suppression/coalescing guard. The hook
must never be the first owner of mutation semantics.

### 5.3 Export, backup, and retention

`SYNC_SKIP_TABLES` and `SYNC_PRESERVE_TABLES` include `change_plans`,
`change_jobs`, `change_job_events`, and `change_coordination`. Full SQL export,
diagnostics, WebDAV, and S3 omit ledger data. Application-managed SQLite backup
uses a temporary same-directory database, performs SQLite backup, drops the four
tables, verifies absence, fsyncs, and atomically publishes it. The raw copy is
never exposed as the final backup. Upgrade maintenance inventories and removes
or rewrites older app-managed backups containing the tables; failed sanitation
does not publish a replacement.

Injected-clock retention rules:

- ready expiry: purgeable 24 hours after `expiresAt`;
- abandoned/invalidated: 24 hours after the stored matching anchor;
- terminal Plan/job: 30 days after owning job `terminalAt`;
- nonterminal, recovery-required, unacknowledged completion, inconsistent pair,
  or active recovery envelope: never timed-purge;
- explicit clearance is a separate typed revision-CAS operation and scrubs any
application-managed legacy backup named by the inventory.

Main connection ownership moves behind `DatabaseRuntime`, whose maintenance
gate can stop admissions, drain workers/sync/readers, close every connection,
replace the database, and reopen after exact compatibility reinspection. DAO
code receives a scoped connection guard and cannot cache a connection across a
maintenance boundary. This is one coordinated refactor owned by the
DB-runtime/compat epoch after CP-store's DAO-only release SHA, not per-DAO
patches or simultaneous writers.

Connection access remains closure-based, while a separate connection-free
activity lease covers a complete file/network/async operation:

```text
DatabaseRuntime::begin_activity(participant)
  -> Result<DbActivityLease, DbRuntimeError>
DatabaseRuntime::read(&lease, |conn: &Connection| -> Result<T,E>)
  -> Result<T, DbAccessError<E>>
DatabaseRuntime::write(&lease, |conn: &mut Connection| -> Result<T,E>)
  -> Result<T, DbAccessError<E>>
DatabaseRuntime::fence_publication(producedGeneration, participant)
  -> Result<DbPublicationPermit, DbRuntimeError>
DbActivityLease::begin_remote_effect(remoteKind, snapshotDigest, attemptId)
  -> Result<RemoteStaging, TransitionFailure<DbActivityLease>>
RemoteStaging::put_object(&mut self, kind, bytes, expectedDigest)
  -> Result<RemoteObjectReceipt, DbRemoteTransitionError>
RemoteStaging::seal_objects(self)
  -> Result<RemoteObjectsVerified, TransitionFailure<RemoteStaging>>
RemoteStaging::abort_and_quarantine(self)
  -> Result<RemoteQuarantined, TransitionFailure<RemoteStaging>>
RemoteObjectsVerified::publish_manifest(self, manifest)
  -> Result<RemoteManifestPublished, TransitionFailure<RemoteObjectsVerified>>
RemoteManifestPublished::readback_and_ack(self)
  -> Result<RemoteTerminal, TransitionFailure<RemoteManifestPublished>>
RemoteManifestPublished::persist_reconcile_required(self, error)
  -> Result<RemoteRecoveryPending, TransitionFailure<RemoteManifestPublished>>
DatabaseRuntime::begin_maintenance(authority, reason, deadline)
  -> Result<MaintenancePermit, DbRuntimeError>
MaintenancePermit::close_and_take(self)
  -> Result<ClosedDatabase, TransitionFailure<MaintenancePermit>>
ClosedDatabase::install_verified(self, candidate, compatibilityReceipt)
  -> Result<InstalledDatabase, TransitionFailure<ClosedDatabase>>
InstalledDatabase::open_and_reinspect(self)
  -> Result<ReadyToPublish, TransitionFailure<InstalledDatabase>>
ReadyToPublish::prepare_workers(self, deadline)
  -> Result<ReadyToPublish, TransitionFailure<ReadyToPublish>>
ReadyToPublish::publish(self)
  -> Result<PublishedGeneration, TransitionFailure<ReadyToPublish>>
```

`participant` and `reason` are closed enums rather than strings:

```text
DbParticipant =
  Bootstrap(CompatibilityInspect|SchemaMigration|SeedAndCleanup|AutoVacuumRebuild)
| Dao(ChangePlan|Failover|Mcp|Profiles|Prompts|Providers|Proxy|Settings|Skills|
      StreamCheck|UniversalProviders|UsageRollup)
| Command(Usage|ImportExport|WebDavSync|S3Sync|PostImportSync)
| Proxy(Service|Router|FailoverSwitch|ResponseProcessor|UsageLogger)
| Service(ClaudeDesktopConfig|ProviderLive|Settings|Skill|ModelPricing|
          SessionUsage|UsageStats|CodexHistoryMigration|SyncProtocol)
| Background(PeriodicBackup|PeriodicSessionSync|WebDavAutoSync|S3AutoSync|
             CodexHistoryMigration)
| ChangePlanWorker

DbMaintenanceReason = CompatibilityUpgrade|ManualSqlImport|NamedBackupRestore|
  WebDavSnapshotApply|S3SnapshotApply|CandidateDatabaseApply|AutoVacuumRebuild

ExternalSqliteAuthority = CodexState|HermesState|OpenCodeState
```

`DbActivityLease` contains only runtime identity, activity ID, participant, and
generation. It is `Send`, `!Sync`, and `!Clone`, may cross local IO or `await`,
and never contains a connection/statement/transaction/guard. `T`/`E` are
`Send + 'static`; the higher-ranked synchronous closure prevents a connection
borrow from escaping. Lease `Drop`, including panic unwind, decrements the
typed active count and notifies the drain.

An operation acquires its lease before the first main-DB snapshot and holds it
through its last local file/DB commit and publication fence. It never holds a
connection guard across IO/await. WebDAV/S3 downloads stage and validate before
acquiring; uploads retain the connection-free lease from snapshot through every
remote object PUT, authoritative manifest PUT/readback, acknowledgement, and
local result fence. Skill download precedes the activity, while local file+DB
commit is inside it. `rebuild_codex_usage` holds one activity across backup,
reset, and reimport. Every async result, hook, tray/cache/event, and sync signal
carries its producer generation and needs a `DbPublicationPermit`; once
maintenance requests stop, late old-generation results are quarantined rather
than published.

Before its first remote write, manual or auto upload persists
`DbRemoteEffectReceiptV1` and moves the lease into a `#[must_use]`, `!Clone`
linear state machine:

```text
RemoteStaging -> RemoteObjectsVerified -> RemoteManifestPublished
              -> RemoteTerminal
              -> RemoteQuarantined
RemoteManifestPublished -> RemoteRecoveryPending
```

Every state token owns the connection-free activity lease, participant,
generation, snapshot digest, remote kind, attempt ID, and durable receipt
revision. Each operation borrows the token or consumes it and returns the next
token; failure returns `TransitionFailure<StateToken>` so `?` cannot silently
discard authority. PUTs update/read back immutable attempt/digest-qualified
object receipts. The prior manifest never references partial attempt objects;
the manifest is the sole authority and is published only by
`RemoteObjectsVerified`. Terminal/quarantined readback is required before the
activity count is released.

The durable receipt owner is
`src-tauri/src/database/remote_effect.rs` in the DB-runtime lane. Its stable
path is
`<app-config>/database-remote-effects-v1/attempts/<lowercase-uuid>.v1.json`,
written with create-new/atomic replace/file+directory fsync and a stable
directory lock. Deny-unknown fields are `schemaVersion`, attempt/participant/
generation/snapshot/remote kind, state/revision, object kind/key/digest/etag/
readback, manifest key/digest/etag/readback, ack/reconcile/quarantine status,
created/updated timestamps, and bounded safe error code. No credentials, URLs,
paths outside the owned relative namespace, or content bytes are stored.

Cancellation and caught errors must explicitly call abort/quarantine or
persist-reconcile. If panic/future cancellation drops a nonterminal token,
`Drop` performs only a synchronous fail-closed action: atomically marks the
already-created receipt `RecoveryRequired` and increments the runtime durable
recovery gate; it never tries to await network cleanup. Process death leaves
the pre-effect receipt nonterminal. Startup scans these receipts before DB
maintenance/new remote-effect admission and enters
`OpenRemoteRecoveryPending(g)`: ordinary local DB activity may continue, but
replacement and new uploads are rejected until the existing WebDAV/S3 service
reads back objects/manifest and reaches `RemoteTerminal` or
`RemoteQuarantined`. Offline/credential failure stays typed recovery-required;
it never clears the gate by timeout. Only terminal receipt fsync/readback
decrements the recovery gate and allows maintenance.

After manifest response loss, recovery reads the authoritative manifest and
either completes the ack or quarantines an unreferenced attempt; it never
blindly republishes. Cleanup/quarantine failure remains durable
`RemoteRecoveryPending`. Thus error, cancellation, panic, and restart preserve
authority just as the success path does, and no old-generation remote manifest
can become authoritative after local replacement.

Background loops are registered as closed `DbWorkerKind` handles with an
epoch-bound cancellation token, join handle, and start barrier. Maintenance
transitions
`Open(g) -> StopRequested -> Draining -> Drained -> Closing -> Closed ->
Installing -> ReadyToPublish(g+1) -> Open(g+1)`. It closes activity admission,
fences publication, snapshots worker handles without holding the runtime gate,
cancels and joins WebDAV/S3 auto-sync, periodic backup/session sync, and the
Codex-history one-shot, then waits for active leases to reach zero. Proxy HTTP
remains up: new DB activity returns `AdmissionClosed`; existing leased requests
drain. Gate/registry/connection locks are never held across await.

Before close, stop/join/drain failure returns the typed transition token,
restarts the old workers behind generation `g`, and reopens only after they are
ready; it performs zero close/replace. After close, install/reinspect/reopen or
worker-prepare failure leaves `FailedClosed(epoch, phase)` and requires durable
marker recovery. `TransitionFailure<S>` returns both the linear state token and
a closed `DbRuntimeError`; dropping it cannot reopen authority. The error enum
is exactly `AdmissionClosed | MaintenanceBusy | WorkerStopFailed |
WorkerJoinDeadlineExceeded | DrainDeadlineExceeded | UnknownActivity |
LeaseGenerationMismatch | ConnectionSlotMissing | ConnectionCloseFailed |
CandidateReceiptMismatch | CandidateCompatibilityRejected | InstallFailed |
ReopenFailed | PublicationFenceRejected | GenerationExhausted |
InvalidTransition | RecoveryRequired`, each with only its typed participant,
generation/epoch/phase, safe code, or bounded count fields. IPC maps only the
closed safe code.

Lock order is global artifact lock when applicable, cross-process DB
compatibility file lock, process-local maintenance serial, runtime gate, then
connection slot. Runtime receives a verified `DbMaintenanceAuthority` and does
not acquire the artifact lock itself. Replacement/fsync/inspect occurs under
linear tokens without gate/slot locks. Publication inserts the verified
connection, stores `g+1`, and releases new-worker start barriers in one
infallible critical section with no IO or fault point.

After migration, production static checks allow no FyAgent main/staged/backup
DB `.conn`, `lock_conn!`, `Mutex<Connection>`, direct `Connection::open*`, or
returned `rusqlite::Connection` outside `database/runtime.rs`; schema,
migration, and backup helpers accept a borrowed connection only inside its
closure. Separately classified Codex/Hermes/OpenCode SQLite callsites use only
`ExternalSqliteAuthority` entries and do not join the main-DB drain. A compile
test proves no statement/transaction/guard can outlive the closure, and the
inventory task proves both immutable-baseline discovery and current expected
sets are exactly equal to the manifest.

Named barrier tests include
`database_runtime_proxy_service_holder_reacquires_after_reopen`,
`database_runtime_proxy_state_router_failover_drain`,
`database_runtime_periodic_backup_task_drain`,
`database_runtime_periodic_session_sync_task_drain`,
`database_runtime_codex_history_migration_drain`,
`database_runtime_manual_export_read_drain`,
`database_runtime_webdav_upload_snapshot_read_drain`,
`database_runtime_s3_upload_snapshot_read_drain`,
`database_runtime_webdav_auto_sync_drain`,
`database_runtime_s3_auto_sync_drain`,
`database_runtime_usage_rebuild_backup_reset_import_drain`,
`database_runtime_post_import_sync_waits_for_verified_reopen`,
`database_runtime_webdav_skills_rollback_on_db_install_failure`,
`database_runtime_s3_skills_rollback_on_db_install_failure`, and
`database_runtime_old_generation_auto_sync_signal_is_quarantined`.

Activity/borrowed-boundary tests are
`db_activity_admission_closed`, `db_activity_drop_notifies_drain`,
`db_activity_panic_releases_count`, `db_activity_generation_mismatch`,
`db_activity_cannot_return_connection_borrow`,
`usage_logger_does_not_hold_connection_across_await`,
`claude_desktop_apply_holds_connection_free_activity`,
`provider_live_commit_holds_connection_free_activity`,
`skill_download_occurs_before_db_activity`,
`skill_file_db_commit_blocks_replacement`,
`opencode_sync_separates_external_and_main_db_handles`, and
`codex_history_external_db_never_joins_main_drain`. Each registered worker has
an independent `*_stop_join_timeout` fault for WebDAV/S3 auto-sync, periodic
backup/session sync, and Codex-history migration. Transition tests cover stop
failure restarting old generation, join/drain timeout with zero close/replace,
connection-close failure, install/reinspect/worker-prepare failed-closed state,
single infallible publication, and exactly-one generation increment.

Pause points after snapshot/read and before local/external success publication
cover manual WebDAV/S3 upload, both auto-sync loops, SkillService,
periodic session sync, and Codex-history migration. Maintenance either remains
blocked until the activity ends or requests cancellation and rejects the old
generation publication; it may never observe zero between DB read and a later
authoritative side effect.

Each of manual WebDAV, auto WebDAV, manual S3, and auto S3 has distinct faults
`after_snapshot`, `during_database_put`, `during_skills_put`,
`before_manifest_put`, and `after_manifest_before_ack`. Assertions prove the
lease/remote permit remains counted, replacement cannot pass `Draining`, prior
manifest never points at partial attempt objects, cancelled attempts are
quarantined/read back, and one manifest+ack terminal receipt precedes lease
release. For all four upload classes, separate injected failures cover every
object PUT error/readback mismatch, cancellation and panic, manifest response
loss, ack-receipt fsync failure, cleanup/quarantine failure, process kill after
each durable revision, and restart offline/online recovery. Each proves either
terminal/quarantined readback before activity release or a nonterminal durable
receipt whose recovery gate continues to block replacement/new upload across
restart.

Concurrency/bootstrap faults additionally cover new-admission loss to
maintenance, participant panic lease release, stale-generation guard rejection,
timeout with zero close/replace, reopen failure holding admission closed, hook
suppression across close/install/reopen, header inspection before SQLite open,
pre-migration backup failure, auto-vacuum exclusivity, external SQLite
non-participation, and candidate database apply. Each holds its class at the
barrier, proves maintenance times out with no replacement, releases it, then
proves one close/replace/reinspect/reopen. WebDAV/S3 download may stage bytes
before drain but cannot apply, notify hooks, enqueue another sync, or report
success until post-reopen readback; failure removes/quarantines the stage and
leaves authority closed or the old generation intact according to the frozen
marker state.

## 6. Worker, cancellation, effects, readback, and recovery

### 6.1 Worker lease and claim

One cross-process `ChangeWorkerLease` plus durable monotonically increasing
instance epoch owns scheduling. Admission creates an unowned planned job. A
worker CAS-claims only `{planned, owner=null, effectStartedAt=null}` and moves to
`running/pre_effect`. Query/list paths are pure reads and never claim, reconcile,
or wait on a writer lock.

At startup, after the exclusive process lease:

- planned/running with no effect start becomes
  `interrupted_before_effect/no_effect`;
- effect-started orphan becomes `reconciling` under a new epoch and performs
  readback only;
- a registered current-process worker is never stolen;
- CAS miss reloads/stops.

### 6.2 Effect gate

The claimed worker re-decodes/re-digests the stored envelope, then enters the
Provider coordinator. It rechecks all resource fingerprints, source versions,
epoch, preconditions, mode/risk, permissions/readability, and secret metadata.
Any mismatch terminalizes the owning job as
`failed/pre_effect_validation_failed/no_effect/recovery=none` with sorted safe
reasons. It does not turn a consumed Plan back into an admission rejection.

After a successful #35 compatibility handoff, the Provider-owned outer seam
prepares the operation-bound capability, performs any required physical
confirmation, takes the Provider coordinator lease, rechecks the final baseline,
then resolves material only inside the private commit closure. The material is
never returned to `worker.rs`, never crosses an await outside #35's approved
closure, and is zeroized/released on every exit. Before that handoff, any input
not proven credential-free terminates before admission as
`dependency_unavailable`; after admission, a resolve/lifetime failure produces
the one owning terminal no-effect job.

Cancellation and effect start compete on one CAS tuple. Before effect,
cancel wins as `cancelled/no_effect`; after the CAS that writes
`effectStartedAt`, cancel returns `too_late` and cannot alter the job. The CAS
returns an unforgeable module-private `EffectPermit` bound to job/plan/digest/
operation/payload/resource set/worker epoch. Only `commit_prepared_change` can
consume it once. Permit forgery, reuse, wrong action, wrong payload, or stale
epoch is a zero-write error.

### 6.3 Commit/readback order

Under the Provider-owned critical section:

1. prepare all target bytes and validate every expected fingerprint;
2. create/fsync the declared recovery envelope and old-byte backups as the first
   effect;
3. execute ordered SQLite actions, then device/live/common/MCP actions once;
4. suppress/coalesce business sync for the entire effect/readback window;
5. read every declared resource independently of writer return;
6. classify exact target, exact baseline/no-effect, partial/mixed, or unreadable;
7. atomically persist terminal snapshot/event and release/quarantine sync;
8. enqueue at most one final sync only for a fully read-backed safe terminal.

There is no automatic restore, inverse, compensate, or writer replay. Manual
recovery hints name backup scope and limits. `recheck_change_recovery` verifies
job ID/revision and executes only the frozen readers; its spies must show zero
writer/backup/restore/compensation/secret-resolve effect.

## 7. Protected source cutover

`ProtectedCodexMutationGate` is consulted by every public Provider writer and
source entrypoint. Its pure precedence is:

1. classify proxy takeover, official target, or critical risk and return the
   specific typed unsupported outcome;
2. for supported normal-mode Codex create/edit/switch, return or route
   `change_plan_required` before first effect;
3. non-Codex and separately named Codex delete/import-default/live-remove/
   official-seed/proxy-control/sort/last-used operations retain their existing
   behavior, but every managed writer joins the coordinator/epoch.

The cutover inventory and required proof are:

| Entry | Required behavior | Zero-effect assertions |
| --- | --- | --- |
| six add/update/switch Tauri commands | typed unsupported first, otherwise `change_plan_required` | Provider/hook/endpoint/writer |
| public ProviderService add/update/switch/add-draft/endpoints | same fail-closed gate; private permit seam only bypass | Provider/file/DB/endpoint |
| tray Codex provider click | focus/open exact safe switch Plan request | proxy/menu/current/provider |
| Profile apply with Codex delta | whole profile unsaved; edit/remove-delta actions | autosave/proxy/MCP/current-profile/events |
| Codex Provider deep link | closed safe draft to Plan UI; activation is intent only | Plan/Provider/draft/endpoint/switch/network on reject/navigation failure |
| old UCP executor | delegates only to v2 identity/envelope path | legacy public switch never called |
| Universal legacy CRUD/sync | `universal_mutation_v2_required` | Universal/child/event/cache/epoch |

Static tests enumerate the real registrations and callsites rather than matching
one string occurrence. A missed caller is caught again by the service-level gate.

For Codex create/edit, `EndpointSpeedTest` receives
`persistenceMode="draft"`, `draftEndpoints`, and `onDraftEndpointsChange`. Its
probe button may call the existing read-only network probe only after the user
explicitly requests it; add/remove changes local form state and makes no IPC.
The form-to-Plan constructor canonicalizes that exact set, and the private
Provider transaction replaces endpoint rows only after admitted apply. Static
and spy fixtures prove neither the Codex form nor speed-test path invokes
`add_custom_endpoint`/`remove_custom_endpoint`; a positive control proves the
explicit speed probe still runs without persistence.

## 8. SecretRef and Universal mutation integration

### 8.1 #35 compatibility gate

The placeholder `SecretRefPort` exposes safe metadata inspection and an opaque
native-only apply capability; it does not define storage, material, confirmation
UI, or recovery semantics. Before wiring production:

1. receive the #35 owner message naming exact immutable SHA/ref and public paths;
2. resolve the SHA locally and record a name-status/diff conflict budget against
   this source freeze;
3. compare exact ref format, owner/sink binding, revision, capability lifecycle,
   prepare/confirm/resolve ordering, error matrix, redaction, and zeroization;
4. run only #35's focused contract tests after its integration commit;
5. return to design review if observable digest bytes, persisted fields, or
   effect-gate ordering differ.

There is no fallback to plaintext, a local key map, environment variables, or a
second secret store. Before the gate, only `ProviderCredentialIntentV1::None`
with all source/target/live/recovery inputs proven credential-free can create an
executable Plan. `Clear` is not automatically credential-free when a legacy
value or binding exists.

### 8.2 Universal one-command mutation

`mutate_universal_provider(UniversalMutationRequestV1)` replaces renderer
`upsert -> sync`. Every variant structurally requires its expected absence,
opaque revision token, provider epoch, proposed safe draft, and sync flag as
defined in `design.md`; forbidden fields cannot deserialize.

Safe list/get returns `UniversalProviderMutationViewV1`. The revision token
domain-binds the redacted Universal fingerprint, provider epoch, actual child
presence/redacted digest, and expected materialization. TypeScript treats it as
opaque. The private mutation reads one `UniversalCodexImpactSnapshotV1` under
the coordinator before any write and either commits the entire Universal plus
all allowed children once or writes nothing. A stale token, epoch, membership,
or actual-child observation is `universal_revision_changed` plus a fresh safe
view.

`UniversalCredentialIntentV1` and backend-private
`ProviderCredentialIntentV1` remain different enums/domains. Only the #35
adapter may translate after validating the opaque binding token. Persistent
credential state uses the #35-owned closed `None|SecretRef|NeedsLocalRebind`
discriminator; no ref enters the legacy `api_key` string. Imported safe fields
may commit as `NeedsLocalRebind`, but no child materializes until secure rebind.

`UniversalCredentialBindingKeyV1` normalization and vectors are implemented in
one backend constructor. The TypeScript fixture validates returned bytes/digest
metadata but never authorizes reuse. Every field/version/digest mismatch becomes
`NeedsLocalRebind`.

The exact reachable command is:

```text
async mutate_universal_provider(
  app_handle: AppHandle,
  state: State<AppState>,
  request: UniversalMutationRequestV1
) -> Result<UniversalMutationOutcomeV1, UniversalMutationErrorV1>
```

It lives in `commands/universal_mutation.rs`, is re-exported by
`commands/mod.rs`, and appears once in `lib.rs::generate_handler!` in the same
atomic cutover commit. `src/lib/api/providers.ts` exposes only
`universalProvidersApi.mutate(request)` for writes; the new query module calls
it once and never chains upsert/sync. Existing `get_universal_providers` and
`get_universal_provider` names remain registered but return strict safe views.
Legacy `upsert_universal_provider`, `delete_universal_provider`, and
`sync_universal_provider` remain temporarily registered only to return
`universal_mutation_v2_required` before state access/effects. Registration and
cross-layer fixtures prove one new command, strict variant decode, safe reads,
legacy zero-write rejection, and absence of renderer `upsert -> sync`.

## 9. Credential artifact and database compatibility implementation

### 9.1 Sidecar and lock ownership

`CredentialArtifactStoreV1` opens only the device-local sidecar with schema 1.
It owns source records, candidate bindings, source/candidate attempts, steps,
owner epochs, and acknowledgement bytes. Unknown schema, corrupt store,
permission failure, or integrity failure exposes no actions and never falls back
to memory or main-DB reconstruction.

All source/candidate actions, scanner, GC, and DB replacement recovery acquire
the stable config-dir `CredentialArtifactIntegrityLockV1` before any identity
peek, enumeration, or preflight and retain it through external effects,
readback, acknowledgement, and authority publication. Relationship IDs never
choose lock authority. The only legal nested order is:

```text
artifact integrity exclusive
  -> main DB maintenance drain
    -> DbCompatibilityLockV1 exclusive
      -> short sidecar/main transactions and file publication
```

Optional per-ID locks are performance-only inside the global lock. Static
assertions reject `peek source ID`, `source-artifact action lock`, relationship-
derived lock selection, and enumeration before the global lock. Split
source-A/candidate-C/source-B fixtures
race scanner/list/get with every action/GC path.

### 9.2 Source and candidate action engine

The contract types and legal transitions are copied exactly from `design.md`;
implementation does not compress them into generic strings. Key enforcement:

- each accepted request binds ID, original expected revision, action, immutable
  attempt ID, expected content digest, and owner epoch;
- before each external effect one CAS records effect-start and idempotency key;
- no owner may issue that effect again after effect-start;
- post-effect paths are readback-only;
- candidate lineage is write-once `NeverPublished -> Published` and never
  reverses;
- pair-integrity `Inconsistent` is sticky, pins survivors indefinitely, and
  suppresses every mutation/GC/retry control;
- exact original-request replay returns the persisted active/terminal snapshot
  while current; later action yields `candidate_action_superseded` plus only the
  current safe view;
- acknowledged apply/no-effect is byte-immutable and self-loops after Ready
  receipt clear; it can never later become Applied from lookalike main rows;
- delete-candidate recovery may become Deleted only, never Applied;
- source delete never deletes/rolls back candidate/main; candidate delete never
  deletes source/main.

`CredentialArtifactIntegrityScannerV1` acquires the global lock, freshly
enumerates all source/candidate identities, rereads every connected/mismatched
side, and may CAS only a new sticky overlay/revision. It changes no bytes, refs,
attempt, main DB, or lifecycle. Event publication occurs only after the overlay
commit. Persistence failure returns a store-unavailable safe view with no
actions.

GC uses one global-lock sidecar transaction. A Published pair purges only
together after both Deleted, all files/ref ownership/actions/recovery/receipts
are clear, and 30 days have elapsed from the later terminal/receipt anchor.
NeverPublished source-only purge requires the exact persisted lineage and proof
that no candidate/ref/action/effect receipt ever exists. Missing counterpart is
inconsistency, not purge permission.

### 9.3 DB compatibility and replacement

Before normal SQLite open, `database/compatibility.rs` obtains the stable lock
and interprets the marker/header only. Marker states are the closed
`BootstrapPending|MigrationPending|ReplacementPending(CandidateApply)|Ready`
union from `design.md`. Missing-marker fallback reads only the exact header when
WAL/SHM/hot journal are absent. Newer/unknown/corrupt/permission/identity/
generation/lock states fail closed without `Database::init`, DDL, hooks,
business reads/writes, sync, or network.

Candidate apply checkpoints and closes the staged DB, freezes identity/content,
fsyncs `ReplacementPending`, atomically replaces main, then publishes `Ready`
with the matching completion receipt. A newer replacement is forbidden until
sidecar acknowledgement and exact marker/attempt CAS clear the receipt.

`DbReplacementRecoveryV1` acquires artifact integrity first and compatibility
exclusive second, then freshly enumerates/rereads marker and every observed
sidecar identity. It classifies exact prior, exact target plus query-only
projection, mixed/unreadable, or authority unavailable; it never initializes,
migrates, invokes #35, rebuilds, or replays. Exact prior records immutable
observed-no-effect; exact target records immutable applied; marker/sidecar/ack
mismatch keeps normal DB closed. Old binaries inspect the stable compatibility
header/marker and stop at `database_upgrade_required` before SQLite open.

No ordinary plaintext DB backup is created for this migration/replacement.
Sanitized transfer/backup/export excludes sidecar, ref/binding, recovery, and
private Plan authority.

The existing `Database::stored_user_version_exceeds_supported` is not a safe
guard because it opens SQLite. Startup is reordered to build a pre-DB bootstrap
context, acquire artifact integrity, recover replacement under compatibility
exclusive, acquire the process-lifetime compatibility shared lease, inspect,
and only then call `DatabaseRuntime::open`. UCP supervisor starts after AppState;
WebDAV/S3 workers start last.

Marker publication uses a dedicated durable atomic writer: create same-dir temp
with restrictive permissions, write all bytes, `sync_all` the file, atomic
replace, then fsync the parent directory where supported. The existing generic
`config::atomic_write` (`flush + rename`) is not accepted for marker authority.

The #35 reference-native main-DB migration is a later `user_version > 16` and
`DB_COMPAT_VERSION > 6` step. It may be enabled only after an immutable safe
predecessor `MIGRATION_GUARD_BASELINE_SHA` is built/tested and the #35 contract
SHA is integrated. This additive #55 ledger work does not reserve or claim v17.

## 10. IPC and renderer state model

### 10.1 Registered commands and events

The v2 command set is registered exactly once:

```text
get_change_plan_capabilities
create_change_plan
get_change_plan
find_latest_change_plan
abandon_change_plan
find_latest_change_job
apply_change_plan
cancel_change_job
get_change_job
list_recoverable_change_jobs
recheck_change_recovery
purge_change_history

mutate_universal_provider

list_credential_artifacts
get_credential_artifact
migrate_credential_artifact
delete_credential_artifact
recheck_credential_artifact
list_credential_candidates
get_credential_candidate
apply_sanitized_candidate
delete_sanitized_candidate
recheck_sanitized_candidate
```

Events carry only `{jobId,eventSeq}` or
`{authorityKind,authorityId,revision}`. The listener is registered before the
first discovery query. A valid newer event invalidates/refetches; foreign,
duplicate, stale, or unknown events are ignored. Snapshot always wins.

### 10.2 Frontend ownership

`ChangePlanFlow` receives one typed `ChangePlanLaunchIntent` for create/edit/
switch. It stores only the form draft until Plan creation, then Plan ID, digest,
scope, and job ID. Backend queries own lifecycle/job truth. Reload finds the
latest Plan/job by safe scope. Abandon closes a backend ready Plan before the
renderer discards its draft. Navigation failure retains the draft and performs
no write.

The flow uses the existing Prompt/Memory V2 full-screen pattern through
`src/components/common/FullScreenPanel.tsx`: single portal, stable header/footer,
interaction lock, scrollable detail, and serial reload. It does not enlarge the
current narrow Dialog into a second pattern. Candidate-only survivors are
root-discovered and shown through a Provider-page safety banner that opens an
independent full-screen safety panel; they do not depend on a source Provider
row.

The component split is:

- `ChangePlanFlow.tsx`: reducer/orchestration only;
- `ChangePlanPanel.tsx`: full-screen shell and focus boundary;
- `ChangePlanPreview.tsx`: exact before/after summary and affected resources;
- `ChangePlanLifecycleNotice.tsx`: expired/drift/unsupported/secret/no-change;
- `ChangePlanJobProgress.tsx`: durable steps/cancel/reconcile/terminal;
- `ChangePlanResourceResults.tsx`: per-resource readback;
- `ChangePlanSafetyBanner.tsx`: root-level artifact/candidate discovery;
- `CredentialArtifactPanel.tsx` and `CredentialCandidateCard.tsx`: total safe
  artifact/candidate projections.

The existing `ChangePlanFlow.tsx` path is retained but its v1 Dialog interior is
replaced; its root single-instance mount and terminal callback concept remain.
Every other component in the list is new. No `PlanPreview.tsx`, `PlanStatus.tsx`,
or `CredentialArtifactRecovery.tsx` file is created. `FullScreenPanel.tsx` is
reused without changing its public visual archetype unless prototype review
identifies a concrete defect.

All actionable views use heading initial focus, `role=dialog` or `role=alert`
where appropriate, focus trap/return, labelled keyboard actions, icon plus text
status, and no color-only meaning. Expiry uses a local timer only to request a
backend refresh; it never authorizes or persists state itself.

### 10.3 Required visual/state projections

At minimum the table-driven renderer covers:

| Backend truth | Visible answer and allowed actions |
| --- | --- |
| clean/warning ready | what changes, resources, actions, backup/recovery, credential status, privacy/evidence, expiry; exactly one confirm |
| no-change | successful informational result; close/edit only, no confirm |
| expired | original expiry, fresh preview/edit only |
| invalidated/drift | sorted safe reasons, fresh preview only |
| unsupported mode/risk/operation | specific reason/next step, no direct fallback |
| secret missing/dependency unavailable | repair/wait/#35 secure entry, then fresh preview; old Plan cannot confirm |
| planned/running/reconciling | authoritative progress; cancel only if backend says allowed; no duplicate submit |
| terminal no-effect/partial/unknown | exact observed state and manual recovery truth; recheck only when allowed |
| artifact/candidate closed states | exact total mapping frozen in `design.md`, including observed-no-effect, uncertain, deleted before/after apply, superseded, authority unavailable, and pair-inconsistent overlay |

Four locale files have identical key sets. Safe-code interpolation is allowlist
based; backend errors, paths, refs, versions, receipts, fingerprints, binding
tokens, and values are never interpolated.

## 11. Test and evidence design

### 11.1 Focused backend modules

Tests live next to the owning modules unless cross-crate registration is the
subject. Each family is independently filterable:

- `change_plan_contract_v2`: deny-unknown/illegal combinations, public/private
  sentinels, v1 dispatch;
- `change_plan_canonical_v2`: all four language-neutral vectors, key/array/i64/
  null/duplicate/Unicode cases, semantic equality/difference;
- `change_plan_preview_side_effects`: create/edit/switch/no-change/unsupported/
  dependency states and the complete spy vector;
- `change_plan_store_v2`: additive v16 columns, row discriminator, admission
  transaction, CAS/events, discovery, abandon/expiry/invalidate/retention;
- `change_plan_worker_v2`: claim/cancel/effect race, pre-effect drift, secret
  failure, permit binding/reuse, readback classification, orphan reconcile;
- `change_plan_provider_cutover`: six commands, service gate, endpoints, tray,
  profile, deep link, old UCP, Universal with entry-specific spies;
- `change_plan_backup_sync`: skip/preserve, sanitized backup, restore guard,
  epoch preservation, no private sentinel;
- `credential_artifact_v1`: every source/candidate transition/outcome, pair
  integrity, response loss, superseded action, joint GC;
- `credential_artifact_concurrency`: two owners, global-lock order, split-brain,
  scanner/action/GC races, no deadlock/effect replay;
- `db_compatibility_v1`: marker/header/WAL/journal/newer-version/lock/fault matrix,
  exact-prior/target/ambiguous/authority-unavailable recovery;
- `universal_mutation_v1`: closed requests, token/impact CAS, credential intent
  separation, binding-key vectors, typed-disable and reference-native paths.

Fault injection pauses at every named durable boundary. A test that merely
returns an error without proving counters/readback is insufficient.

### 11.2 Frontend and cross-layer tests

Retain `changePlanDtoContract.v1.json` byte-for-byte. Add v2 fixtures and:

- `tests/lib/change-plan.test.ts`: schema dispatch, strict enums/invariants,
  query keys/events/timer;
- `tests/components/change-plan/ChangePlanProjection.test.tsx`: every Plan/job
  state in four locales, allowed actions, focus/keyboard/screen-reader semantics;
- `tests/components/change-plan/CredentialSafety.test.tsx`: every artifact/
  candidate state, overlay precedence, sentinel exclusion;
- `tests/hooks/useChangePlanLauncher.test.tsx`: create/edit/switch draft routing,
  reload/resume, no legacy mutation;
- `tests/hooks/useCredentialArtifacts.test.tsx`: list/get/event refetch and
  candidate-only survivor;
- `tests/integration/change-plan-cross-layer.test.ts`: Rust-authored fixtures and
  exact command registration;
- `tests/integration/change-plan-entry-cutover.test.tsx`: all renderer/native
  source callsites and zero fallback;
- extend `tests/integration/App.test.tsx`, Add/Edit/DeepLink/Universal dialog,
  Provider action/mutation, locale parity, and MSW/Tauri mock coverage.

The generated visual reference is exactly
`research/prototype/generated-change-plan-reference.png`; the high-fidelity
prototype is `research/prototype/change-plan-prototype.html`; its source/asset
receipt is `research/prototype/manifest.json`; independent findings live in
`reviews/usability-review.md`. FE-product owns the image/prototype/manifest and
the usability reviewer owns only the review file. All are labelled `prototype`,
never runtime evidence. After source freeze, fresh screenshots are required for
clean, warning, expired/drift, secret dependency, recovery, and candidate safety
at the frozen viewport. Any touched UI source invalidates those captures.

### 11.3 Isolated runtime/evidence harness

Implementation adds `.mise/tasks/change-plan.toml` to `mise.toml`, pins
`@playwright/test = 1.61.1` (and exact `playwright`/`playwright-core` 1.61.1)
in `package.json`/`pnpm-lock.yaml`, and adds `playwright` to
`pnpm-workspace.yaml` `ignoredBuiltDependencies`. It exposes a lock-update
preview, explicit cache preparation, and four evidence tasks:

```text
rtk mise run change-plan:chromium:lock
rtk mise run change-plan:chromium:lock --apply
rtk mise run change-plan:chromium:prepare
rtk mise run change-plan:evidence:renderer
rtk mise run change-plan:evidence:browser
rtk mise run change-plan:evidence:native
rtk mise run change-plan:evidence:failure
rtk mise run change-plan:evidence:failure --apply
```

All tasks have fixed Node argv and no interactive/raw/confirm metadata. DB
inventory declares its two flags. Chromium lock and evidence failure declare
only formal `--apply`; both are `preview-by-default`, print deterministic diffs
without it, and perform their one tracked write only with it. Prepare is
`dependency-environment`; renderer/browser/native are
`ephemeral-environment`. Evidence modes do not depend on prepare and never
download a browser. `REQUIRED_TASKS` contains all seven tasks;
`PARAMETERIZED_TASKS` contains DB inventory, Chromium lock, and failure;
`RAW_TASKS` is unchanged and `check` references none of them. The owning spec,
checker, exact mise include, and generated documentation change in the same
commit, followed by `tasks:docs:generate --apply`, docs check, and validation.

The exact task metadata is:

| Task | Description | `FYAGENT_TASK_EFFECT` |
| --- | --- | --- |
| `change-plan:db-inventory` | Verify the frozen and current Change Plan database runtime inventories are exactly classified | `read-only` |
| `change-plan:chromium:lock` | Preview or update the reviewed macOS Chromium lock for Change Plan evidence | `preview-by-default` |
| `change-plan:chromium:prepare` | Provision the current-host Chromium cache from the reviewed Change Plan browser lock | `dependency-environment` |
| `change-plan:evidence:renderer` | Create or resume the active session and stage deterministic Change Plan renderer evidence | `ephemeral-environment` |
| `change-plan:evidence:browser` | Join the active session and stage deterministic Change Plan browser evidence | `ephemeral-environment` |
| `change-plan:evidence:native` | Join the active session and stage same-SHA native Change Plan evidence | `ephemeral-environment` |
| `change-plan:evidence:failure` | Stage failure evidence and preview or apply atomic publication of the complete active session | `preview-by-default` |

The exact task runs are `node scripts/change-plan/check-database-runtime-inventory.mjs`,
`node scripts/change-plan/chromium.mjs lock|prepare`, and
`node scripts/change-plan/evidence.mjs renderer|browser|native|failure`.
DB inventory usage is `--baseline-sha <sha> --manifest <path>`; Chromium lock's
`--apply` help is “Atomically write scripts/change-plan/chromium-lock.v1.json;
otherwise print the candidate and diff”; failure's `--apply` help is “From an
already prepared failure preview, atomically publish the verified complete
evidence set; without --apply, capture or resume failure evidence, prepare the
publication, and print its diff without writing the repository”. Mise supplies
`usage_apply`; scripts reject all caller evidence/session/source env and unknown
argv. The default failure invocation accepts `native`, `failure`,
`publish_preparing`, or `publish_prepared`; it completes or resumes the fourth
mode and preparation and is an exact no-op at a byte-identical
`publish_prepared`. The `--apply` form accepts only `publish_prepared`; calling
it directly from `native` is an out-of-order, zero-write failure.

The only lock is `scripts/change-plan/chromium-lock.v1.json`, contract
`fyagent.change-plan.chromium-lock.v1`. It binds exact package/core 1.61.1,
pnpm integrity, `playwright-core/browsers.json` path/hash,
`chromium-headless-shell` revision 1228 and browser 149.0.7827.55, plus closed
`macos-arm64` (`mac26-arm64`) and `macos-x64` (`mac26`) entries. Their only
trusted URLs are respectively
`https://cdn.playwright.dev/builds/cft/149.0.7827.55/mac-arm64/chrome-headless-shell-mac-arm64.zip`
and
`https://cdn.playwright.dev/builds/cft/149.0.7827.55/mac-x64/chrome-headless-shell-mac-x64.zip`;
redirects may not change host. Each committed entry contains nonzero concrete
archive size/SHA-256, payload root, executable relative path/size/SHA-256,
Mach-O machine, and canonical payload-tree SHA-256. Zero, placeholder, unknown
host, range version, or mismatched core/revision is invalid.

`change-plan:chromium:lock` reads the exact package/pnpm lock/browsers JSON,
downloads both reviewed archives to a repo-sibling candidate, validates archive
and extraction limits, native formats and every file, then prints canonical
JSON plus diff. `--apply` alone uses the shared atomic writer with preimage CAS
to update that one lock file; it never changes package/workspace/pnpm locks.
Payload digest records NFC/POSIX-sorted dirs, regular file executable bit/size/
hash, and relative in-root symlinks; absolute/`..` links, case-fold collision,
duplicate entry, device/FIFO/socket, or extraction-limit overflow fail.

`change-plan:chromium:prepare` is repository-read-only. It reads the committed
lock and prepares only the current host beneath
`<authorityRoot>/chromium-cache/<lockSha256>/<hostKey>/` via `.partial` plus
atomic rename. Evidence preflight sets `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1`,
accepts only that explicit verified executable, launches/closes `about:blank`
and checks `browser.version()`. Absence/drift fails with the prepare command;
there is no install call, system Chrome, user cache, or download fallback.
Browser contexts reject every request except `127.0.0.1`/`localhost`. An x64
lock may be generated on arm64, but x64 runtime evidence requires native x64
prepare/readiness.

All four evidence commands use one `ActiveEvidenceSessionV1` authority without
caller argv or environment. Let `repoRealpath` be NFC-normalized repository
realpath without trailing separator and
`repoKey=sha256("fyagent-change-plan-evidence-repo-v1\0" + repoRealpath)`.
The exact same-device root is
`<dirname(repoRealpath)>/.fyagent-change-plan-evidence-v1/<repoKey>/` with
`0700` dirs/`0600` files. It owns `repo.lock`, immutable
`active-session.v1.json`, and
`sessions/<uuid>/active-evidence-session.v1.json` plus
`partials/`, `modes/`, `sandboxes/`, and `publish/`. There is no repository
`evidence/tmp/` or ignored final evidence.

The repo lock is an fsynced owner JSON atomically hard-linked to `repo.lock` and
held only around pointer/record CAS, mode rename, and final publish. Acquisition
waits at most ten seconds. Only a same-host owner with PID definitively `ESRCH`
may be renamed into `.stale-locks`; live/`EPERM`/foreign/corrupt/PID-reuse-
ambiguous owners fail closed. Unlock validates device/inode/nonce, unlinks, and
fsyncs the authority root.

Renderer is the sole creator: under lock and only with no pointer it captures
the current clean HEAD as `SOURCE_HEAD`, `HEAD^{tree}`, creates a lowercase UUID
session, claims renderer, fsyncs the record, then publishes the immutable
pointer last. Browser/native/failure are strict joiners that locate only this
pointer. Record CAS is exactly
`(sessionId,bindingDigest,revision,state,activeClaim.claimId|null)` over the
closed sequence `renderer -> browser -> native -> failure -> publish_preparing
-> publish_prepared -> publishing -> destination_renamed -> destination_fsynced ->
cleanup_pending -> terminal_snapshot_pending -> terminal_receipt_pending ->
published`, with terminal
`aborted`. Out-of-order join, live claim, mixed
session, corrupt pointer/record, or mismatched receipt is zero-write failure;
later-state retry is an exact no-op when its immutable receipt matches.

Each claim has 256-bit ID/owner token, attempt, host, PID, frozen `preparedAt`,
`recoveryEpoch`, optional recovery owner, and literal partial/prepared/receipt
paths. A mode
writes `partials/<mode>.<claimId>`, fsyncs files/directories, then under lock
validates the claim, renames to `modes/<mode>`, fsyncs the parent, records the
receipt hash, clears the claim, and advances revision/state. Crash in partial
cleans only a dead claim's partial; crash after rename/before CAS verifies the
immutable receipt and completes CAS; crash after CAS resumes as no-op. Two
renderers yield one session and one busy result. Stale binding with no live
claim may be CAS-aborted, cleaned, given an immutable abort receipt, and have
its active pointer unlinked only by renderer after that receipt fsync;
`publishing` or any later state may be recovered only by failure `--apply`.

Failure preparation is itself claim-owned. It CASes `failure ->
publish_preparing` with claim ID and literal partial path
`sessions/<sessionId>/publish/<SOURCE_HEAD>.partial.<claimId>/`, literal
claim-qualified prepared path
`sessions/<sessionId>/publish/<SOURCE_HEAD>.prepared.<claimId>/`, and receipt
path
`sessions/<sessionId>/prepared-publication-receipts/<claimId>.v1.json`.
Renderer creates and fsyncs the empty receipt parent with the session. The
preparer writes every destination file into the partial, fsyncs each file and
directory bottom-up, freezes the destination manifest/file-list/root digest,
requires the prepared path to be absent, renames partial to prepared, and
fsyncs `publish/`.

`PreparedPublicationReceiptV1` validates against the tracked deny-unknown
`research/prepared-publication-receipt.schema.json` and uses canonical JSON. Its
closed fields are `schemaVersion=1`,
`contract=fyagent.change-plan.prepared-publication-receipt.v1`, `repoKey`,
`sessionId`, `bindingDigest`, `sourceHead`, `sourceTree`, `claimId`, `attempt`,
`recoveryEpoch`,
`recordPreimage{revision,state=publish_preparing,activeClaimId,sha256}`,
`preparedRelativePath`, `manifestRelativePath`, `manifestSha256`, sorted
`destinationEntries[{path,size,sha256}]`, `fileListDigest`, `rootDigest`,
`preparedDevice`, `preparedInode`, and
`createdAt=activeClaim.preparedAt`. Paths must be NFC POSIX
relative descendants of the session, entries are byte-path sorted and unique,
and unknown, absolute, symlink, non-regular, duplicate, or case-fold-colliding
entries fail closed.

The receipt writer opens
`prepared-publication-receipts/.<claimId>.tmp.<ownerToken>` with
create-new/`0600`/no-follow, writes the complete canonical bytes, fsyncs that
file, atomically hard-links it without replacement to `<claimId>.v1.json`,
fsyncs the receipt parent, unlinks the temp, and fsyncs the parent again. It
then reopens the final path no-follow, requires one regular link, reparses the
schema, and compares its canonical bytes and SHA-256 with the in-memory value.
Any existing final path must already be byte-identical and bound to the same
claim/preimage or the operation fails; it is never replaced. Only after this
readback, under the repo lock, does the same claim revalidate the record
preimage, prepared device/inode/tree, manifest, entry list and all digests and
CAS to `publish_prepared`, recording `preparedFromPreimageSha256`,
`preparedClaimId`, `preparedRecoveryEpoch`, prepared/receipt paths, receipt
SHA-256, file-list/root digests and clearing the active claim.

`publish_preparing` recovery is a closed matrix. A partial with no prepared or
receipt is resumed only by its live owner; after same-host PID death is proven,
the recovery process deletes only that claim-qualified partial, fsyncs
`publish/`, and CASes the exact record preimage to a new claim ID/paths before
rebuilding. A prepared directory with no final receipt may be resumed only by
CAS-transferring recovery ownership of the *same* persisted claim (incremented
`recoveryEpoch`, exact old owner/preimage, proven-dead PID); the recovery owner
rehashes every byte against the four mode receipts and frozen manifest,
re-fsyncs the prepared tree and `publish/`, and, if equal, executes the receipt
protocol above. A claim-owned receipt temp with no final receipt is either
published when its complete canonical bytes match, or removed and regenerated
only after the prepared tree revalidates. A valid final receipt with a leftover
same-inode/same-byte temp removes only that temp, fsyncs the receipt parent,
then performs final readback; any unequal or foreign temp is zero-write. An
invalid final receipt, receipt without prepared directory, incomplete/unequal
prepared tree, or same-claim path collision first CASes the exact
`publish_preparing` record to a `recoveryDisposition=quarantine` intent holding
`quarantineRelativePath` and the sorted source-path/inventory digest. Only that
recovery owner then moves the exact claim-owned bytes to
`publish/quarantine/<claimId>.<revision>/`, fsyncs the parent, and CAS-aborts.
A crash after intent, after any rename, or after parent fsync but before abort is
resumed only by verifying the recorded source/quarantine inventories and
completing the same move/fsync/`aborted` CAS; it never restores or adopts those
bytes. If
ownership/preimage cannot be proved, recovery is strictly zero-write. A valid
prepared directory plus read-back receipt before CAS is revalidated by the same
claim/recovery owner and completes the single `publish_preparing ->
publish_prepared` CAS. A stale revision, different claim, live/foreign/
ambiguous PID, mismatched binding, or path outside the recorded claim paths is
always zero-write; no new claim adopts old partial, prepared, temp, receipt, or
quarantine bytes.

Fault oracles cover every destination directory/file create and write, each
file fsync, each bottom-up directory fsync, partial-to-prepared rename,
`publish/` fsync, receipt temp create/write/file-fsync, no-replace link, receipt
parent fsync, temp unlink/second parent fsync, receipt reopen/readback, and the
pre/post prepared CAS. At each boundary the fixture asserts exact filesystem
bytes, record revision/state/claim, and one of only: live-owner resume,
dead-owner rebuild with a new claim, same-claim verified completion,
claim-owned quarantine plus `aborted`, or zero-write rejection.

`failure --apply` accepts only `publish_prepared` with that durable receipt and
never constructs, repairs, or adopts missing prepared bytes itself.

Normal renderer/browser/native/failure and pre-rename publish admission require
a fully clean porcelain-v2 worktree. Recovery from `destination_renamed` through
`published` uses a different closed rule: the only permitted dirt is the exact
untracked destination path set stored in `publication.destinationEntries`, and
its recalculated file-list/root digest must equal the bound destination receipt;
any tracked modification, deletion, rename, extra untracked path, unequal byte,
or path outside that one destination is zero-write rejection. This recovery
rule can complete CAS/cleanup after a rename crash without weakening source
cleanliness for any other mode.

The record's `publication` object contains `preparedFromPreimageSha256`,
`preparedClaimId`, `preparedRecoveryEpoch`, prepared/receipt paths and receipt
SHA-256, destination relative path, SOURCE_HEAD, session/binding/root/file-list
digests, sorted
`destinationEntries[{porcelainCode,path,size,sha256}]`, destination device/inode,
rename/fsync timestamps, cleanup checklist/digest/lastError,
`finalSnapshotRelativePath/hash`, and `terminalReceiptRelativePath/hash`.
All three evidence authority schemas are separate deny-unknown canonical JSON:

- immutable `ActiveEvidencePointerV1` contains only repo/session/record path,
  initial binding digest and createdAt;
- immutable `FinalEvidenceSnapshotV1` at
  `sessions/<sessionId>/final-evidence-snapshot.v1.json` contains the
  `terminal_snapshot_pending` record preimage revision/hash, binding,
  destination receipt and cleanup-complete receipt;
- immutable `TerminalEvidenceReceiptV1` at
  `terminal-receipts/<SOURCE_HEAD>.<sessionId>.v1.json` contains repo/session,
  active-pointer SHA-256, final-snapshot path/SHA-256, destination/root digest,
  record path, schema and publishedAt. It deliberately does not hash the final
  mutable-record revision.

After receipt fsync, the final record CASes to `published` and stores the
terminal-receipt path/hash. The one-directional graph is final record -> terminal
receipt -> final snapshot -> record preimage, while the terminal receipt also
binds the initial active-pointer hash. No object hashes bytes that later embed
its own hash.

Tracked
`.trellis/tasks/08-14-issue-55-change-plan-mainline/research/evidence-inputs.v1.json`
is committed before source freeze and fixes design-freeze SHA, UCP SHA, #35
present immutable SHA or closed unavailable reason, and contract/spec receipt
hashes. Renderer binds its Git blob/SHA-256 plus design-freeze blob, current
HEAD/tree, dependency receipts, Chromium-lock and pnpm-lock blobs, host/target,
host-native build receipt digest, and executable digest. Every joiner recomputes
all bindings under lock before claim. Caller `SOURCE_FREEZE_SHA`, session/root,
config/Codex/store/output/mode environment variables are rejected; native child
evidence env is derived only from the claimed record.

The script never reassigns `HOME`/`CODEX_HOME`; it passes the complete closed
debug envelope:

```text
FYAGENT_EVIDENCE_MODE=native|failure
FYAGENT_EVIDENCE_SESSION_ROOT
FYAGENT_EVIDENCE_CONFIG_ROOT
FYAGENT_EVIDENCE_CODEX_ROOT
FYAGENT_EVIDENCE_STORE_PATH
FYAGENT_EVIDENCE_OUTPUT
FYAGENT_EVIDENCE_NETWORK=deny
```

`EvidencePathAuthorityV1` canonicalizes all fields once into a `OnceLock`;
every root must be a distinct, non-symlink descendant of the session root.
Missing, partial, overlapping, escaping, or malformed envelopes fail before
any fallback. In debug builds, `main.rs` dispatches the headless native/failure
runner before Windows user context, Linux WebKit, Tauri, store, window, tray,
auto-sync, or HTTP initialization. Accessor order is evidence authority, then
app-store/settings override, then production fallback. `app_store.rs` uses only
the explicit evidence store. Every symbol/string/branch is
`cfg(debug_assertions)`; release has no module, export, dispatch, or IPC. Known
user FyAgent/Codex paths are opened read-only only for pre/post identity/mtime
sentinels; any change fails cleanup.

Renderer mode starts Vite on `127.0.0.1:41755 --strictPort` with deterministic
MSW/Tauri fixtures, waits for a JSON readiness endpoint for at most 60 seconds,
and uses Playwright Chromium at `1440x960`, DPR 1, reduced motion, fixed UTC
clock/font/locale. It captures each of clean, warning, expired, drift,
unsupported, secret-missing, running/recovery, and candidate-safety in
`zh|zh-TW|en|ja`. Browser mode uses the same isolation but runs interaction
specs for focus trap/return, keyboard-only confirm/re-preview/cancel/recheck,
reload/event refetch, one-confirmation, and zero duplicate submit. It saves DOM
state, console/network logs, screenshots, and assertion counts separately.

`build:debug` extends the existing host-native planner to atomically write
`src-tauri/target/<target>/debug/fyagent-build-receipt.v1.json`, already covered
by `src-tauri/.gitignore` `/target/`. The receipt binds schema/operation,
HEAD/tree, clean-before/after, OS/arch/target, exact argv, rustc/rustdoc
identities, regular non-symlink realpath-inside executable, size/SHA-256,
native format/machine, and completion time. Native and failure require current
HEAD/tree/host/target/argv/executable size/hash to equal the receipt; failure
also requires the receipt-file and executable digests recorded by native.
Renderer/browser between build and native do not invalidate this same-SHA
predicate, so no “immediately preceding” claim remains.

Native mode launches that exact debug binary with the explicit evidence roots. The
`#[cfg(debug_assertions)]`-only `change_plan/evidence.rs` constructs the real
`DatabaseRuntime`, Provider preparation/admission/worker/DAO and exact local
Codex file readers/writers for credential-free create-only, create-and-select,
edit-current/non-current, and switch. It writes `ready.json` only after the
native DB/file sandbox is open, writes per-scenario readback/counter receipts,
restarts once against the same sandbox for discovery/recovery, then exits with a
terminal status. The code is not registered in IPC; release-source and
registration checks prove the runner is unreachable when `debug_assertions` is
false.

Failure mode launches the same hashed debug build with the frozen named fault points:
preview writer/network traps, admission drift, cancel/effect race, each resource
boundary, DB participant drain, marker/sidecar/ack failure, exact-prior/target/
ambiguous recovery, source-A/candidate-C/source-B race, and response loss. It
records counters and authoritative readback for every case. An injected failure
is labelled `failure_path`; a real OS permission/lock case is separately tagged.

Every mode uses a 120-second per-state/process timeout, kills and awaits every
child, proves port/process release, deletes only validated session descendants,
and exits nonzero on missing readiness/artifact/cleanup. Failure without
`--apply` completes the fourth mode, claims `failure -> publish_preparing`,
verifies bindings/receipts, assembles/fsyncs the claim-owned directory and
prepared receipt, then CASes to `publish_prepared` with exact digests and prints
the file/size/hash diff. It does not write the repository.

`failure --apply` first acquires the repo lock and reads the unchanged
`publish_prepared` record. Before any state write it no-follow opens and
schema/canonical-byte validates the prepared receipt, verifies its recorded
claim and `publish_preparing` preimage against the prepared fields in the
current record, lstat-checks the prepared device/inode, and recomputes the
prepared tree, destination manifest, sorted entry list, file-list/root digests,
HEAD/tree, binding, four modes and build receipt under the normal cleanliness
rule. Any mismatch releases the lock, leaves `publish_prepared` byte-identical,
and returns a zero-write error. Only after every equality passes may the exact
record preimage CAS `publish_prepared -> publishing`; while retaining the lock,
it then renames the verified complete directory once into
`.trellis/tasks/08-14-issue-55-change-plan-mainline/evidence/<SOURCE_HEAD>/`,
CASes `destination_renamed`, fsyncs destination/parent and CASes
`destination_fsynced`, then enters `cleanup_pending`. It cleans only session
sandboxes/partials/publish staging and fsyncs the session root; failure remains
retryable in `cleanup_pending` and is not accepted terminal evidence.

After cleanup, it CASes `terminal_snapshot_pending`, writes/fsyncs the immutable
final snapshot from that exact record preimage, CASes
`terminal_receipt_pending` with the snapshot path/hash, writes/fsyncs the
terminal receipt, then CASes the record to `published` with receipt path/hash.
Only after the final record fsync does it unlink/fsync the active pointer. For a
crash after snapshot, receipt, record CAS, or pointer unlink, failure locates
authority from the active pointer when present, otherwise from the exact
destination manifest's session ID and deterministic terminal-receipt path.
The legal transient both-files state requires byte/hash agreement and resumes by
unlinking active; a mismatched pair is fail-closed.

Thus crash or cleanup failure never removes the only recovery authority. Crash
before destination rename retries; crash after rename uses the bound dirty-set
rule; crash after any CAS/create/fsync resumes the next step. A later renderer
first verifies any same-HEAD terminal receipt and returns already-published; a
different clean HEAD may create a new active session without mutating retained
terminal receipts. Unequal destination, `EXDEV`, missing mode, mixed session,
unrelated dirt, or mismatched pointer/receipt never advances. Only successful
`--apply` creates the expected dirty evidence set for its dedicated commit.

`manifest.json` validates against `research/evidence-manifest.schema.json` and
contains exact SHA, design/dependency SHAs, host/OS/arch, argv, start/end,
terminal exit/assertion count, fixture IDs, paths/hashes, cleanup result, user-
state sentinels, one evidence class, plus required
`evidenceSessionId`, `evidenceBindingDigest`, and
`terminalReceiptRelativePath = terminal-receipts/<SOURCE_HEAD>.<sessionId>.v1.json`.
Those authority fields are frozen before manifest/root hashing. No generated/
prototype asset can be listed as runtime evidence.

After active-pointer unlink, failure resolves only the exact destination
`evidence/<current-clean-HEAD>/manifest.json`, validates its session/binding and
canonical terminal locator, then reads that receipt. Zero matching receipts is
`terminal_receipt_missing`; one exact destination/manifest/receipt/snapshot/
record chain is authoritative; more than one published receipt for the same
SOURCE_HEAD, a foreign session/binding, path escape, or any digest mismatch is
`terminal_receipt_ambiguous` and zero-write. A same-HEAD terminal receipt makes
renderer return already-published; a different clean HEAD may create a new
session.

`changePlanEvidenceContract` faults cover concurrent renderer, live/dead/
foreign/corrupt repo lock, every pointer/record fsync/rename crash, wrong
HEAD/tree/design/dependency/Chromium/build receipt, forbidden caller env,
out-of-order/idempotent mode retry, every partial/mode/CAS crash, stale/dead
claim and PID ambiguity, mixed-session assembly, failure default zero repo
write, direct `failure --apply` from `native` out-of-order/zero-write,
preview-then-apply reachability, byte-identical repeated preview, every
publish-directory file create/write/fsync and bottom-up directory fsync,
partial-to-claim-qualified-prepared rename and parent fsync, receipt temp
create/write/file-fsync/no-replace-link/parent-fsync/unlink/second-parent-fsync/
readback, partial-only, prepared-without-receipt, torn/corrupt receipt,
valid-receipt-before-CAS, stale/mismatched claim/preimage, receipt mismatch
leaving `publish_prepared` unchanged, and pre/post prepared CAS, actual porcelain-v2 output
after destination rename, recovery with only
the exact bound dirty set, unrelated/unequal dirt rejection, crashes after
rename/destination fsync/record-published CAS and during every cleanup item,
final-snapshot create/fsync, terminal-receipt create/fsync, final-record CAS and
active-pointer unlink, exact pre/post bytes and one-way hashes, legal both-files
recovery, one reachable terminal receipt,
post-unlink zero/one/duplicate/foreign/mismatched receipt selection, unequal
destination and `EXDEV`. Chromium
faults cover exact package/core/browsers JSON, both host URL/archive/executable/
Mach-O/tree hashes, byte-identical lock preview, `--apply` single-file CAS/
rollback, prepare zero source write, and cache-missing/hash-drift zero download/
zero system-browser fallback. Task tests assert exact required/parameterized
sets, usage/effects, include, and generated-doc rows.

### 11.4 Exact command ladder after DESIGN_FREEZE

The implementation phase uses repository Node `24.19.0` and locked `mise`.
Commands are run only after freeze, in this order, and every long-running session
is polled to a terminal exit:

```text
rtk mise run env:check
rtk mise run change-plan:db-inventory -- --baseline-sha ca552f4d918cacc734f81f7efdef70619da139b8 --manifest scripts/change-plan/database-runtime-inventory.json
rtk mise run rust:test -- change_plan_contract_v2
rtk mise run rust:test -- change_plan_canonical_v2
rtk mise run rust:test -- change_plan_store_v2
rtk mise run rust:test -- change_plan_preview_side_effects
rtk mise run test:unit -- tests/lib/change-plan.test.ts tests/integration/change-plan-cross-layer.test.ts

rtk mise run rust:test -- change_plan_worker_v2
rtk mise run rust:test -- change_plan_provider_cutover
rtk mise run rust:test -- credential_artifact_v1
rtk mise run rust:test -- credential_artifact_concurrency
rtk mise run rust:test -- db_compatibility_v1
rtk mise run rust:test -- db_activity_v1
rtk mise run rust:test -- universal_mutation_v1
rtk mise run rust:test -- change_plan_evidence_path_authority
rtk mise run rust:test -- change_plan_evidence_store_isolation
rtk mise run test:unit -- tests/components/change-plan tests/hooks/useChangePlanLauncher.test.tsx tests/hooks/useCredentialArtifacts.test.tsx tests/integration/change-plan-entry-cutover.test.tsx tests/integration/App.test.tsx
rtk mise run test:unit -- tests/miseTaskContract.test.ts tests/taskDocs.test.ts tests/localBuildBoundary.test.ts tests/changePlanEvidenceContract.test.ts

# Evidence/task metadata tracked writes, then commit before SOURCE_HEAD capture.
rtk mise run change-plan:chromium:lock
rtk mise run change-plan:chromium:lock --apply
rtk mise run tasks:docs:generate --apply
rtk mise run tasks:docs:check
rtk mise run tasks:validate

rtk mise run typecheck
rtk mise run format:check
rtk mise run test:i18n
rtk mise run test:desktop:mock
rtk mise run build:renderer
rtk mise run rust:fmt:check
rtk mise run rust:check
rtk mise run rust:clippy
rtk mise run rust:test
rtk mise run tasks:docs:check
rtk mise run tasks:validate
rtk mise run check:contracts
rtk mise run check
rtk mise run change-plan:chromium:prepare
rtk mise run build:debug
rtk mise run change-plan:evidence:renderer
rtk mise run change-plan:evidence:browser
rtk mise run change-plan:evidence:native
rtk mise run change-plan:evidence:failure
rtk mise run change-plan:evidence:failure --apply
rtk proxy python3 ./.trellis/scripts/task.py validate 08-14-issue-55-change-plan-mainline
rtk proxy python3 ./.trellis/scripts/task.py current --source
rtk git diff --check
rtk git status --short --branch
rtk git diff --name-status 4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287..HEAD
```

The Chromium lock apply, generated docs, and `evidence-inputs.v1.json` are
reviewed and committed before the source-freeze HEAD is captured; no tracked
write occurs between that capture and renderer. Focused groups run after their
module commits, not during design. Final `mise run check` and all four evidence
tasks are fresh on that source SHA; only final `failure --apply` writes the
evidence directory afterward. The UCP session ID gap is not reused. Every
yielded process is polled to terminal exit.

Evidence labels remain distinct:

- `source_report`: generated source/static inventory only;
- `code_audit`: reviewer/static assertions;
- `runtime_screenshot`: renderer/browser visual state;
- `native_runtime`: built Tauri process with real local read/write/readback;
- `failure_path`: deterministic injected or real failure with counters;
- `UAT`: user-performed confirmation; never inferred from automated evidence.

## 12. Ownership, conflict budget, and integration order

Only one writer may own a file at a time. Workers are told they are not alone,
must not revert other edits, and must rebase their assumptions on committed
immutable SHAs. Shared hotspots remain with the main integration owner.

After DESIGN_FREEZE and before any prototype/source/test write, Trellis context
is made real. `implement.jsonl` removes `_example` and contains these exact
spec/research files: backend/frontend indexes; backend UCP, Codex Provider,
deep-link, development-environment and task-runner specs; and this task's
ownership/handoff, UCP-gap, dependency-contract and provider-create/edit maps.
`check.jsonl` contains the same owning backend specs, frontend component/hook/
state/type/quality specs, and ownership/dependency maps. The gate runs
`task.py validate`, `task.py start`, `task.py current --source`, and checks
`task.json.status=in_progress` exactly as listed in `implement.md`. Each lane
loads `trellis-before-dev` plus its mapped specs before its first edit.

| Lane | Exclusive files/responsibility | Must not edit |
| --- | --- | --- |
| CP-core | `change_plan/{contract,canonical,projection,capabilities,inspection}.rs`, v2 fixtures | Provider writer, schema, renderer |
| CP-store | `database/dao/change_plan.rs` and ledger-specific DAO logic only; publishes a DAO-only SHA before DB-runtime claims shared database files | schema/backup/mod, Provider/UI/runtime shared files |
| DB-runtime/compat | `database/{runtime,remote_effect,compatibility,mod,schema,migration,backup}.rs`, exact inventory/checker, all direct/borrowed/impl/capture/copy boundaries in §2.3, `claude_desktop_config.rs`, `codex_history_migration.rs`, proxy/logger/Skill/session/sync/external-SQLite callsites, and `store.rs`; `lib.rs` is serialized integration | UI and concurrent Provider/`lib.rs` edits |
| CP-worker | `change_plan/{coordination,admission,worker,test_support}.rs` | Provider private commit implementation |
| Provider-adapter | after the DB-runtime release SHA, `services/provider/{mutation,codex_projection,change_commit}.rs`, surgical owned ranges in `mod.rs`/`live.rs`/`codex_config.rs`/`settings.rs`, `services/proxy.rs`, and `claude_desktop_config.rs` | Ledger schema/renderer or concurrent DB-runtime edits |
| Artifact | `credential_artifact.rs`, `credential_artifact/**`, artifact command module | Main Plan DTO/UI except shared safe fixture |
| Universal | `universal_mutation.rs`, surgical private DAO/provider model ranges | Artifact/Plan ledger |
| FE-platform | `lib/api/query` Change Plan/artifact files, strict decoders/query | visual components/i18n |
| FE-product | `components/change-plan/**`, four locale `changePlan` ranges | API/provider hook |
| FE-cutover | launcher/providerDraft, Codex branches of Provider/DeepLink/Universal forms | backend core |
| Evidence/task-runtime | `.mise/tasks/change-plan.toml`, task contract/checker/generated docs, evidence/chromium scripts and locks, Playwright config/tests, `config.rs`, `app_store.rs`, evidence manifest/publish parent | Provider-owned `codex_config.rs`/`settings.rs`, CP-core `change_plan.rs`, Integration `main.rs`/`lib.rs`/package locks |
| Integration main | `lib.rs`, `commands/mod.rs`, `commands/provider.rs`, `tray.rs`, profile/config/proxy entrypoint ranges not owned by DB-runtime, `App.tsx`, `mutations.ts`, shared indexes, Cargo/package locks, final fixtures/evidence | delegates no overlapping writer; wires DB-runtime only after its commit |

Shared ownership is an epoch, never a simultaneous writer. CP-store releases
its DAO-only SHA; DB-runtime claims and releases its runtime SHA; Provider then
serially edits `services/proxy.rs`, `provider/live.rs`,
`claude_desktop_config.rs`, `codex_config.rs`, and `settings.rs`; CP-core serially adds the
debug-only `change_plan.rs` declaration; Evidence owns its private files and
path authority; Integration finally owns `main.rs`, `lib.rs`, and package/lock
wiring. Every transition records the predecessor SHA before the next claim.

Conflict budget against active dependencies:

| Surface | #35 risk | #41 risk | Resolution |
| --- | --- | --- | --- |
| `services/provider/mod.rs` and Provider lease/order | high | high | main owner integrates one immutable SHA at a time; no parallel edits |
| Provider/Universal credential models and DAO | high | medium | #35 owns material/storage contract; #55 owns Plan/resource projection; explicit adapter only |
| `database/schema.rs`, `database/mod.rs`, backup/restore | high | high | #55 additive v16 first; later #35 migration after guard SHA; #41 consumes APIs only |
| `change_plan*` DTO/DAO/commands/event/query | low from #35 | high | #55 sole canonical owner; #41 extends after handoff |
| `lib.rs`, App/provider hooks/locales/MSW | medium | high | main integration owner, serial commits |
| artifact sidecar | #35 receipts only | low | narrow trait/receipt adapter; no shared DB table |

Dependency compatibility is performed at exact SHAs, never moving branch tips.
A material signature/ordering/schema conflict stops that integration and returns
to design review; it is not patched with a shadow type or duplicate storage.

## 13. Small-commit plan

Each commit must pass `git diff --check` plus its named focused static/module
gate after `DESIGN_FREEZE`:

1. `docs(change-plan): freeze issue 55 product and technical contract`
   - task docs/specs/reviews/freeze receipt only.
2. `feat(change-plan): add v2 closed contracts and canonical vectors`
   - CP-core DTO/canonical/projection/capabilities/shared fixtures.
3. `feat(change-plan): persist v2 ledger and coordination authority`
   - CP-store DAO-only CAS/discovery/retention/epoch logic and store fixtures;
     publish its release SHA without editing schema/backup/mod.
4. `feat(database): add compatibility runtime and durable replacement guard`
   - from the CP-store release SHA, DB-runtime owns additive v16 columns,
     schema/backup/sync filters, pre-open marker/header/lock, closeable runtime,
     exhaustive inventory migration, startup order, and barrier/fault tests.
5. `feat(provider): prepare exact codex mutation authority`
   - pure preparation/projection, coordinator/epoch, private permit/commit and
     create/edit/switch adapters, but no public cutover flag flips yet.
6. `feat(change-plan): add admission worker platform and strict renderer port`
   - generic commands, immediate planned response, supervisor/readback/event,
     TS decoder/query/launcher primitives and fixtures; still no public route
     flips.
7. `feat(credentials): add quarantined artifact and candidate authority`
   - sidecar/global lock/scanner/actions/recovery/GC/IPC and fixtures.
8. `feat(universal): replace split mutations with revision-bound command`
   - safe views, impact/token/permit, typed dependency gating; #35 integration
     is a separate follow-up commit when its SHA exists.
9. Atomic cutover commits; each includes renderer route/API removal, native
   entry guard, public service gate, reachable private commit path, and focused
   zero-effect tests in the same commit:
   - `feat(change-plan): atomically cut over codex switch`
   - `feat(change-plan): atomically cut over codex create and edit endpoints`
   - `feat(change-plan): atomically cut over tray profile and deep link`
   - `feat(universal): atomically cut over universal mutation IPC`
   The cutover flag is never committed between backend and frontend halves.
10. `feat(change-plan-ui): complete full-screen workflow and safety states`
    - visual-only/detail components, i18n, a11y, Universal safe UI and mocks;
      authority routing already landed atomically in step 9.
11. `feat(change-plan): add debug-only isolated evidence authority`
    - serialized Rust path/startup seam, headless runner, host-native executable
      resolver/build receipt, release-unreachability and focused tests.
12. `test(change-plan): add locked offline evidence task runtime`
    - mise/task contract/checker/generated docs, exact Playwright pin, Chromium
      lock/explicit prepare, repo-sibling evidence transaction, Playwright and
      contract tests; run `tasks:docs:generate --apply`, docs check, and task
      validation in this commit.
13. `test(change-plan): publish final same-sha evidence`
    - source freeze first; four modes stage outside the repo and failure
      atomically publishes only the complete manifest/artifact set.
14. `docs(change-plan): record final evidence and downstream handoff`
    - exact SHA receipts, review, GitHub readback, task archive metadata.

No commit mixes #35 handoff integration with unrelated product/UI work. A later
source edit invalidates affected module evidence; a UI source edit invalidates
touched screenshots. Final source freeze is followed by evidence generation,
not vice versa.

## 14. PRD traceability and freeze blockers

| Requirement | Implementation owner | Primary evidence |
| --- | --- | --- |
| R-01 / AC-02 | inspection + PlanStore + effect spy | `change_plan_preview_side_effects`, resource snapshots |
| R-02..R-04 / AC-03..04,10 | contract/canonical/projection/codex adapter | shared DTO/private/vector fixtures |
| R-05 / AC-17, AC-21 credential clauses | SecretRef port + #35 adapter + Universal/artifact | exact-SHA compatibility, fault/sentinel/native evidence |
| R-06..R-08 / AC-05..08,15..19 | DAO/admission/worker/runtime | CAS/race/clock/orphan/recovery tests |
| R-09 / AC-09..09a | FE product/platform | table-driven four-locale a11y + runtime captures |
| R-10 / AC-01,11,20..21 | Provider adapter/cutover/Universal/deep link | entrypoint scan, per-entry spies, native readback |
| R-11..R-14 / AC-12..16 | test/evidence/retention/compatibility | module ladder, sanitized artifacts, final manifest |

Detailed-design review must return `0 P0 / 0 P1 / 0 P2` on this complete file,
`prd.md`, `process-state-machine.md`, `design.md`, specs, and the current source
map. Freeze is blocked by any unresolved finding, ambiguous file ownership,
unlisted bypass, unbounded secret/material path, missing terminal command, or
contradictory schema/lock order. #35's pending implementation does not block
freezing this narrow port and typed-disabled behavior, but it blocks claiming
secret-bearing create/edit/switch, reference-native Universal migration, and
their native acceptance as production-enabled.

## Revision 2 closure summary

Detailed-design round 1 failed with `0 P0 / 8 P1 / 1 P2`. Revision 2 closes the
findings by: removing the last relationship-lock alternative and obtaining
architecture PASS; enumerating every DB handle/copy participant under one
`DatabaseRuntime` owner/API/fault matrix; making Codex endpoints draft-only and
cutovers operation-atomic; registering the Universal mutation end to end;
splitting every Rust test to one legal filter; freezing isolated renderer/
browser/native/failure evidence tasks and artifacts; separating non-blocking
design notification from a closed consumable #41 gate; adding exact Trellis
context/activation/before-dev steps; and selecting one renderer/prototype file
map. Detailed-design round 2 then failed with `0 P0 / 3 P1 / 1 P2`.

## Revision 3 closure summary

Revision 3 replaces the claimed DB list with equality-checked, typed inventories
for all 25 direct callers, retained/background/operation holders, copy paths,
external SQLite authorities, participant/reason enums, owners, and named barrier
faults. It makes evidence path authority source-complete from pre-Tauri debug
dispatch through config/store/Codex/settings accessors; uses repo-sibling staging
and one final atomic publish; binds native/failure to a same-SHA host-native
build receipt; and closes the exact mise include, task metadata, generated docs,
task checker, Playwright pin, explicit Chromium prepare, and offline hash
preflight contracts. Task metadata now records architecture Round 23. Detailed-
design round 3 then failed with `0 P0 / 4 P1 / 0 P2`.

## Revision 4 closure summary

Revision 4 adds equality classes for every production borrowed `Database`
function, impl block, task capture, and exact test range; removes the legacy
facade; and assigns Claude Desktop/Codex-history/shared Provider owner epochs.
It adds a connection-free async `DbActivityLease`, closed stop/join registry,
generation publication fence, linear `Result` transition tokens, failed-closed
states, lock order, and named pause/fault tests. Evidence now has an exact
repo-scoped lock/pointer/record/CAS session authority and tracked input binding.
Playwright is literal 1.61.1; the closed macOS Chromium lock has a reproducible
preview/apply bootstrap, while prepare is repo-read-only. Failure publication
uses the existing `preview-by-default + --apply` policy, with exact task sets,
usage, docs, and crash tests. Detailed-design round 4 then failed with
`0 P0 / 3 P1 / 0 P2`.

## Revision 5 closure summary

Revision 5 adds the missing syntax-aware `legacyDatabasePathUses` equality
class for associated calls, imports/re-exports, aliases, traits and exact cfg
ranges, with explicit backup/bootstrap/external-helper dispositions and empty
expected legacy sets. WebDAV/S3 manual and auto uploads now retain a linear
generation/snapshot-bound remote permit from snapshot through every immutable
object PUT, authoritative manifest readback, ack and cleanup, so maintenance
cannot replace underneath an irreversible old-generation effect. Evidence
publication now has destination-renamed/fsynced, record-published,
cleanup-pending and terminal states; retry admits only the bound destination
dirty digest and retains the active pointer until cleanup. Detailed-design round
5 then failed with `0 P0 / 2 P1 / 0 P2`.

## Revision 6 closure summary

Revision 6 makes every remote upload phase a `#[must_use]` linear token that
returns authority on error, persists a non-secret effect receipt before the
first PUT, and synchronously raises a durable recovery gate on Drop/panic;
startup/restart blocks maintenance and new uploads until terminal/quarantined
readback. Evidence now uses three distinct schemas and a one-directional hash
graph: immutable active pointer, immutable final snapshot of a fixed record
preimage, immutable terminal receipt, then final mutable-record CAS. Failure
preview now reaches `publish_prepared`; apply creates/fsyncs snapshot/receipt,
finalizes the record, and only then unlinks the active pointer, with every legal
one- or two-authority crash state specified. Detailed-design round 6 then failed
with `0 P0 / 1 P1 / 0 P2`.

## Revision 7 closure summary

Revision 7 adds claim-owned `publish_preparing`: the complete directory,
fsyncs, manifest/root digest and prepared-publication receipt exist before the
CAS to `publish_prepared`, with deterministic dead-claim cleanup and crash
completion. The destination manifest now carries session ID, binding digest and
canonical terminal-receipt locator before hashing; the final snapshot has a
literal path. Post-unlink recovery defines zero/one/multiple/foreign/mismatched
receipt selection, and contract faults cover every assembly write/fsync,
prepared receipt/CAS and no-active lookup case. Round-7 re-review is pending;
the re-review failed with `0 P0 / 2 P1 / 0 P2`.

## Revision 8 closure summary

Revision 8 adds the exact deny-unknown `PreparedPublicationReceiptV1` schema,
claim-qualified prepared/receipt paths, canonical fields, no-replace atomic
publication, file and parent-directory fsyncs, final readback, and a closed
partial/prepared/temp/corrupt/valid/stale recovery matrix with byte/state/result
oracles. It also makes the public ladder executable: default `failure` captures
and prepares from `native`, repeated preview is idempotent, and the separate
`failure --apply` validates the receipt, prepared tree, manifest, file list,
root, record preimage and all bindings before the first `publishing` CAS. Direct
apply from `native` and any receipt mismatch are zero-write. Round-8 re-review
passed with `0 P0 / 0 P1 / 0 P2`; all runtime commands remain forbidden until
the main-thread freeze receipt records `DESIGN_FREEZE=PASS`.
