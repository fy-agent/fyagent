# Issue #35 — Codex secret call graph / ownership closure

## 0. Closure status and evidence boundary

- `CALL_GRAPH_CLOSURE=PENDING_STATIC_CLEARANCE`
- `FILE_OWNERSHIP_CLOSURE=PENDING_STATIC_CLEARANCE`
- `DESIGN_FREEZE=PENDING`
- `HANDOFF_CONSUMABLE=NO`
- Evidence level: `source_report + code_audit` only.
- Audited base: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`.
- This audit did **not** run tests, builds, dependency resolution, browser, renderer, server, native runtime or screenshots.

The registered source/consumer/sink inventory is a static design candidate, not a prose completeness claim. Call-graph closure remains pending until the exact-path generator reproduces the registered base paths, current cross-document static findings are cleared and independent reviewers re-read the same immutable design commit with P0/P1/P2 at zero. #35 is the upstream contract owner, so its design freeze does **not** wait for #55 or #41 to implement compatibility; doing so would create a circular dependency. After #35 publishes the immutable handoff, the following become implementation/source-freeze gates:

1. #55 must replace secret-bearing Provider/live digests and the direct `ProviderService::switch` writer at a new immutable source SHA before its code is integrated.
2. #41 must replace exact-byte secret-bearing backups with structural placeholders and publish an immutable implementation handoff SHA before source freeze. Its current task directory is untracked design work at base `afc317a7...`, not a consumable implementation handoff.
3. Shared registration/Provider/proxy/import files must be integrated serially by one `main integration` owner; they cannot be assigned again to #35, #55 or #41 workers. #35 adds no SQLite schema version or secret tables.

This document is normative for call-graph and file ownership closure. It does not replace `secret-contract/v1`, #55's canonical Change Plan contract or #41's Configuration Apply contract.

## 1. Immutable source/readback record

| Authority | Exact object | Static readback | Decision |
| --- | --- | --- | --- |
| #35 implementation base | `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab` | current dedicated worktree HEAD | only implementation base for #35 |
| #55 implementation/source contract | `ca552f4d918cacc734f81f7efdef70619da139b8` | locally readable; last source/test implementation commit | compatibility input, not secret-safe as-is |
| #55 final branch readback | `6859e9ce04970008f4cf8b3d4883b4f70316291a` | 2026-08-15 remote readback: merge-base/ancestor `ca552f4d`, ahead 3 / behind 0; later commits do not supply a compatible #35 contract | incompatible input; successor SHA still required |
| #35/#55 merge-base | `f424ceff8f085673d00b8fd191045cb965987408` | same for `afc317a7` vs both #55 SHAs | branches diverged; no wholesale merge |
| #41 current design worktree | `/Users/serendipity/.codex/worktrees/issue41/fyagent` at `afc317a7...` | `.trellis/tasks/08-14-issue-41-configuration-apply/` is untracked | non-immutable; cannot be a freeze dependency |

The following #55 blobs are byte-identical at `ca552f4d` and `6859e9ce`:

| File | Blob SHA |
| --- | --- |
| `src-tauri/src/change_plan.rs` | `524f185f69280be7abae690db9a963529d9b467e` |
| `src-tauri/src/commands/change_plan.rs` | `de94c718c75d545b2d7a420dd5f075e5a3988673` |
| `src-tauri/src/database/dao/change_plan.rs` | `f7b6c9ac73a5a6c8654d8c930db1fe2249b6d7ba` |
| `src-tauri/src/database/schema.rs` | `2b0d09aac482e55bd1ce8528c84631dee058ddec` |
| `src-tauri/src/lib.rs` | `9c2f8930cd8abe058df8cd6b94e02160968bfcb3` |
| `src-tauri/src/services/provider/live.rs` | `e3bf651006fd5f8f50497ca06959f4df631115fe` |
| `src-tauri/src/services/provider/mod.rs` | `4f71a65dd460bb57ca144b7ee33f1f447d196f2b` |

### #55 exact conflict/readback surface

`4bfee69c..ca552f4d` adds or changes these implementation surfaces:

- backend: `change_plan.rs`, `commands/change_plan.rs`, `database/dao/change_plan.rs`, `commands/mod.rs`, `database/dao/mod.rs`, `database/schema.rs`, `lib.rs`;
- legacy renderer: `App.tsx`, `components/change-plan/ChangePlanFlow.tsx`, `hooks/useProviderActions.ts`, locale JSON, `lib/api/change-plan.ts`, `lib/query/change-plan.ts`, their indexes and tests/fixtures/MSW handlers.

At `ca552f4d`:

- `provider_definition_digest` serializes the complete secret-bearing `Provider`;
- `live_projection_digest` hashes `read_live_settings(Codex)`;
- `inspect_codex_switch` hashes current/target/live/effective target projections;
- `apply_codex_switch` calls `ProviderService::switch` directly.

Therefore neither `ca552f4d` nor `6859e9ce` is a #35-compatible source contract. Integration must adapt the exact source subset after #55 publishes a secret-safe successor; it must not merge the branch wholesale, because `afc317a7` also carries the isolated V2 Prompt/Memory shell absent from #55's integration base.

## 2. Decision vocabulary and non-negotiable invariants

Every row below has exactly one outcome and one owner:

- **CR — controlled resolve**: public/native caller supplies only owner/ref/plan/capability metadata. #35 revalidates binding and capability immediately before use, acquires material only inside the named native consumer closure, and returns only a typed non-sensitive result.
- **RJ — reject**: stable error, `effect=none`, no DB/live/network/backup/event mutation. The rejected input is never echoed.
- **SP — sanitized projection / placeholder**: only non-sensitive structure, owner/ref display, revisions, availability and stable codes may cross or persist. No material, material-derived digest, raw path, raw OS/upstream error, empty-value substitute or `[REDACTED]` value is inserted into an applyable config.

Owner labels are closed to exactly: `#35 module`, `#55`, `#41`, `main integration`.

Normative invariants:

1. Codex primary API-key material must never enter renderer state, public IPC/event payloads, Provider JSON, Change Plan/job rows, audit rows, proxy logs, usage cache, tray state, diagnostics, crash logs, fixtures, sync payloads or backups.
2. A digest of material is still material-derived data and is forbidden. Only a digest of a typed, sanitized structural projection is permitted.
3. `SecretMaterial` is neither `Serialize`, `Deserialize`, `Clone` nor ordinary `Debug`. A resolver never returns `String`, `Vec<u8>` or a public getter.
4. A prepared capability contains no material and is bound to plan, operation, owner, ref, record revision, binding-set CAS, backend instance/generation, device generation, capability revision, consumer, sink and expiry. Consume is single-use and revalidation occurs after the Provider lease and immediately before mutation. A backend read leaves only as an `AuthorizedBackendRead` that owns the same complete sealed scope and can be consumed only by its named callback; scope, role/slot or consumer substitution fails before material acquisition.
5. OS keyring may persist to a reviewed external config sink only when the frozen backend capability says `persistentTargetProjection=true` and an immutable #55 plan names that exact sink. Hardware with `persistentTargetProjection=false` must reject every file/env projection.
6. Official Codex OAuth material is not converted into `codexApiKey`; `auth.json` is preserved in place where possible and is never copied into FyAgent DB/live-backup/sync/export artifacts.
7. A secret-readiness failure is terminal and circuit-neutral: no network request, no Provider/current-state mutation and no silent failover to another Provider. Only a genuine transport/upstream failure may advance the existing failover queue.
8. A public Codex DTO cannot contain a generic raw `serde_json::Value` capable of carrying credentials. Internal `Provider` remains private to native code; public and mutation DTOs are separate types.
9. New Codex writes containing inline secret fields are rejected. Legacy plaintext remains internally recoverable until verified migration or explicit user disposition, but it is always hidden from public projections.
10. No Codex primary secret is supplied to user-authored JavaScript or a terminal launcher in this MVP.
11. Candidate activation carries exactly one `LegacyActivationComparisonPolicy`: automatic migration and `legacyScrubExistingBinding` use `candidateEquality`; user-approved conflict replace/reconcile/rotate uses `explicitReplacement`. Only the first requires old source values to equal the candidate. Both require the exact source set and structural revisions; the second scrubs the explicitly approved old sources without asserting equality to the new candidate.
12. Ordinary activation accepts only current-owner/current-source authority. `sqlImportStaging|dbRestoreStaging|syncDownloadStaging` may enter only a dedicated typed staged-import token/projection/admission; the exact `ImportCutoverCoordinatorContext` is constructed only after authority-match and #35 confirmation, then becomes the sole authority for staged validation/scrub/readback/cutover. Staged origins are never normalized to `providerRow` or accepted by ordinary readiness/runtime.
13. Startup order is exact: `open store → no-backup DB preflight → same AppState/SecretService → reconcile → app.manage/static command registration → Clean sanitized backup → publish gate → workers`. The opaque `OpenedDeviceLocalSecretStore` holds one lifetime lock and is consumed unchanged by the one production AppState. `Blocked` creates no backup and starts/publishes no workers or consumers; no path injection, reopen or preflight-before-lock backup is permitted.
14. Public recovery is one `SecretRecoveryKind`-discriminated union: `activationCleanup|captureCompensation|deleteFinalization|ownerDetachFinalization`. Non-terminal recovery maps to `completeRecovery → get_secret_cleanup_impact + retry_secret_cleanup`; drift maps to `refreshRecoveryImpact`. Every `SecretUserAction` has exactly one `SECRET_ACTION_DESTINATIONS_V1` destination, including the exact non-#35 `mainIntegrationCommand` for staged cutover resume; generic legacy action strings cannot bypass this map, and an unknown or unmapped action is a decoder failure, never generic retry.
15. Candidate explicit discard and expiry share one immutable `discardCandidate` journal. Backend delete/already-missing and subsequent fresh missing readback are independently authorized and independently checkpointed; neither authorization covers the other. Until both plus state commit complete, the candidate remains reachable as `verifiedPendingPlan + pendingTerminalDisposition=discarded|expired + SECRET_OPERATION_RECOVERY_REQUIRED + discardCandidate`; retry cannot relabel the disposition. Only terminal cleanup yields `discarded|expired`, removes the pending field and maps an expired candidate to a summary-refresh-first fresh capture flow.
16. Backend/device revocation can be persisted only by an explicit `Revoke` authorization plus a non-clone, consuming receipt minted by the exact registered backend handle after it validates `centralRevocation=true`; the authorization/receipt binds ref, device-store instance, record, binding-set, exact registered backend object, device and capability CAS plus `source=centralBackend|deviceAdministration` and time. State commit fresh-revalidates and consumes both. Probe/not-found/locked/denied/unavailable never implies or persists revoked; OS keyring reports no central revocation capability.
17. Provider deletion detaches one owner and retains the secret. Bound/unbound preview/confirmation is no-value and freezes remaining owners, orphan result, `secretRetained=true`, the separate secret-delete route and exact owner/binding revisions; a legacy owner is blocked and receives no impact id. Stale/expired/replayed Provider impact returns Provider-domain `PROVIDER_DELETE_IMPACT_STALE + refreshProviderDeleteImpact + effect=none` before either authority mutates. It is not `SecretErrorCode`/`SecretUserAction`; `refreshDeleteImpact` remains only the separate secret-deletion action.
18. Renderer Provider flows use closed token-free feature-draft/public/mutation DTOs. Add/Edit/forms/cards/feature hooks never receive or emit full secret-bearing Provider state; owner/ref usage and model-fetch requests contain no value. Empty `OPENAI_API_KEY` is a forbidden substitute, not a sanitized credential.
19. Codex deep-link rejection is native and pre-event: raw, percent-encoded and remote-config secret shapes fail before parse/merge/provider construction/event dispatch. Renderer preview code never decodes, masks or displays them, and stable errors never echo a generated canary.
20. Sync/import canonical order is exact: `temp authority/token + material-free projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact ImportCutoverCoordinatorContext → staged source validation/scrub/readback → cutover → live owner/binding finalize`. Main integration constructs the context immediately after #35 confirmation; it is the sole authority for every validation, scrub/readback and cutover step, so none is reachable before or outside it. Public resume input is only `{stageId,expectedResumeCas:{revision,digest}}` and every result returns `currentResumeCas:{revision,digest}`; full live-object identity remains an internal preimage. Old nonce/admission/CAS, wrong object, cancel/expiry/replay and every pre-cutover crash are effect-none/zero-write.
21. `codex_history_migration` creates no raw `settingsConfig` backup before the startup no-value gate. After `Clean`, new history generations contain only structural placeholders/non-secret config; existing generations are immutable scan/report-only evidence.
22. Typed capture adds no command. `list_secret_backend_options` first reads the native owner, complete legacy-source set and binding snapshot, then mints one opaque single-use `SecretCaptureIntentId`; `begin_secret_capture` accepts only that intent id plus the selected backend. `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` resolve to that same list→begin flow. Terminal expiry first refreshes summary/owner-card state and only then mints fresh capture or rotation authority; no old candidate/intent/capability is reused.
23. Every delete and every subsequent fresh missing readback in capture compensation, candidate discard, explicit delete and activation cleanup is a distinct authorized step with a distinct durable checkpoint. A prior operation intent, delete authorization or backend missing observation cannot authorize the readback. Every read/delete scope binds the device-store instance and exact registered backend object.
24. `SNV7-001..005` are Provider-primary Level 1/2 closure requirements owned only by `main integration`: Codex env projection, common-config legacy resolution, public Provider type split, Codex override rejection/owner-private transport, and no-network/closed diagnostics. `SNV7-006` classifies MCP `env`/`http_headers` as `codexMcpEnvOrHeaderCredential` Level 3 adjacent debt; it is no-regression inventory, not Provider-primary PASS scope.
25. Initial staged activation and staged cutover resume are different closed result types. `resume_staged_import_cutover` request data is exactly `{stageId,expectedResumeCas:{revision,digest}}`. Every result data arm contains exactly `stageId|currentResumeCas|status|action|issue`: `activated|alreadyActivated` has `issue=null`, while `recoveryRequired` has its typed issue. Result data forbids `schemaVersion`, `auditEventId`, candidate id, owner, secret ref, owner/credential summary, initial-activation fields and every unlisted field. The common envelope owns version/command id; audit is independent.
26. `LegacySourceCoverageReceipt` is opaque `pub(crate)`, non-Clone/non-Serde/non-Debug and move-only so store, Provider and #35 sibling modules can only name/move/consume it; its fields are private. Its `pub(crate)` `checked_from_complete_inventory_authority` factory consumes an unforgeable `CompleteLegacySourceInventoryAuthority` privately constructed only by main-integration `CodexLegacySourceInventoryBridge`. One receipt atomically binds non-value-derived `LegacySourceInventoryRevision`, fixed 11-domain `CompleteLegacySourceCoverageIdentity`, exact `CurrentLegacySourceExpectations`, and category/state-only adjacent observations so proof cannot detach from data. Each domain has structural revision/presence/count. Exact refs may carry only opaque non-value-derived `LegacySourceLocationId`; raw path/raw locator/value/value-derived digest is forbidden. Startup, every summary/readiness projection, capture options and claimed-intent revalidation, and Provider-delete preview/confirm each obtain/revalidate a new bridge receipt; missing/stale/incomplete coverage or empty-without-all-11-domain proof blocks effect-none.
27. Durable `DeviceInstanceId` and process-local `DeviceSecretStoreInstanceId` are distinct: the former persists the device namespace, while the latter binds one lifetime-locked open and expires at teardown. `#35 module` owns `SecretBackendRegistry` plus the private operation broker; external adapters/main integration cannot mint either authority. Backend operation policy is exactly `CaptureVerify|Validate|ResolveForApply|Delete|Revoke`; every fresh missing readback uses `Validate`, never a sixth Missing operation or probe.
28. Composition is phased. #35 core trait/API files must compile and finish focused core tests without #55/#41/main concrete types; `backend.rs` cannot name a not-yet-existing external type. Each #55/#41/main adapter type lands under its own canonical owner, then the sole `main integration` adapter/composition owner wires the published seams and runs the full Rust gate.
29. Static registration is exactly 15 #35 secret commands plus the separate main-integration `resume_staged_import_cutover` handler. The handler is outside `SecretCommandName` and never counts as command 16; `src-tauri/src/lib.rs` asserts both exact sets, and staged phase-crash UAT invokes the separate handler.
30. Verification manifests require native macOS and native Windows locked Rust 1.85.0 `--all-targets` checks against the exact lock. Rust 1.97.1 output cannot substitute for either MSRV result.
31. `ARR-001`: explicit discard and expiry use exact `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}` as distinct one-shot authorization slots, mapped respectively to `Delete` and `Validate`. Delete/already-missing first persists `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}`; only a later fresh missing receipt consuming that CAS can reach `missingReadbackVerified` and then immutable terminal `discarded|expired`. Candidate discard stays inside the existing journal and adds no recovery kind.
32. `ARR-002`: normal activation retains `ActivationOldRecordDeleteCheckpoint`, recovery retains `RecoveryOldRecordDeleteCheckpoint`, and crash-visible state uses `ActivationOldRecordDurableCheckpoint`; each preserves `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`. `ActivationOldRecordDeleteApplied` separately retains the postcondition. The independent `Validate` missing receipt consumes that CAS and commits terminal supersession atomically; `revokedAt` is exactly the retained `backendCompletedAt`.
33. `ARR-003`: staged resume digest preimage includes journal `operationId` and exact `StagedImportResumePhase::{Intent,SourcesScrubbed,CutoverCommitted,LiveOwnerMinted,LocalBindingFinalized}`. Required fields are cumulative: `intent` none; `sourcesScrubbed` adds `stagedSourceSetCasAfterScrub`; `cutoverCommitted` retains it and adds `cutoverReceiptId`; `liveOwnerMinted` retains both and adds `promotedLiveOwner`; `localBindingFinalized` retains the same three. Other fields are structurally forbidden. Canonical fixtures are `staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`; every fresh nonce/admission or phase/checkpoint transition increments revision and recomputes CAS.
34. Slot totals are fixed: normal activation plus recovery owns 10 authorization/confirmation slots; candidate discard owns 2 more; aggregate total is 12. Five delete→missing pairs account for 10 delete/missing slots. These are slots, not operations/journals/recovery kinds: the frozen totals remain five hardware operations, 8 journals and 4 recovery kinds.

### 2.1 PV7/CAV7 synchronization readback

| Finding | This call graph's exact synchronized rule |
| --- | --- |
| `PV7-001` | public staged resume is only `{stageId,expectedResumeCas:{revision,digest}}`; full identity is internal preimage; result returns same-shaped `currentResumeCas`; fresh nonce/admission makes every old request zero-write stale |
| `PV7-002` | terminal expiry first refreshes summary/owner card, then mints fresh capture/rotation authority; old candidate/operation/intent is never reused |
| `PV7-003` | legacy conflict, retry and backend choice share the native-snapshot single-use `SecretCaptureIntentId` registry; renderer sends only intent id + exact backend selection |
| `CAV7-001` | staged order is temp token/projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact `ImportCutoverCoordinatorContext` → staged source validation/scrub/readback → cutover → live owner/binding finalize; context is the sole authority for all validation/scrub/readback/cutover operations |
| `CAV7-002..004` | delete finalization, capture compensation, candidate discard and activation/activation recovery split delete from fresh missing readback into different authorization/receipt/checkpoint slots |
| `CAV7-005` | only explicit `Revoke` authorization may mint/persist a revocation receipt; ordinary probe/hint cannot |
| `CAV7-006` | each scope/receipt binds the lifetime device-store instance and exact registered backend object; generation is rechecked before consumption |
| `CAV7-007..010` | private per-operation broker/context authority, sibling-usable opaque bootstrap token and exhaustive private internal-error/action factories remain mandatory; none adds a command, journal or recovery kind |
| `ARR-001` | candidate discard/expiry has exact `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`, durable `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}` and fresh missing-before-terminal enforcement |
| `ARR-002` | activation/recovery runtime checkpoints, `ActivationOldRecordDeleteApplied` and `ActivationOldRecordDurableCheckpoint` retain disposition/completedAt/CAS and postcondition; fresh missing commits supersession with `revokedAt=backendCompletedAt` |
| `ARR-003` | staged resume preimage binds operation id plus exact cumulative-field `StagedImportResumePhase`; five named `staged_resume_*_v1` fixtures and every-transition fresh CAS are mandatory |

## 3. End-to-end authority graph

```text
legacy current DB/live sources
  -> registered native source inventory + exact-path baseline
  -> conflict/equality adjudication
  -> native capture or verified migration
  -> backend write + readback
  -> unbound SecretRecord + verifiedPendingPlan candidate
  -> #55 immutable candidate-activation plan/admission
     with comparisonPolicy=candidateEquality|explicitReplacement
  -> activation-specific prepare/confirm before #41 lease
  -> #41 activation lease + final baseline
  -> #35 exact occurrence/revision fresh-read
     candidateEquality: constant-time compare each admitted current value to candidate
     explicitReplacement: verify exact approved source set/revisions; do not compare old values to candidate
  -> #35 journaled exact binding CAS + scrub/readback every approved LegacySourceRef
  -> release activation lease

renderer / #55 preview
  -> now-bound owner + structural configuration + SecretSummary/readiness
  -> separate immutable #55 apply plan (no material-derived digest)
  -> #41 prepare target and rollback capabilities before lease
  -> #41 Provider lease + final #55 baseline check
  -> #41 sanitized structural backup
  -> #35 single-use resolve_for_apply(consumer, exact sink, closure)
  -> existing native writer / HTTP header construction
  -> in-closure readback comparison
  -> typed boolean/stable receipt only
  -> #55/#41 durable state + sanitized renderer snapshot

startup
  -> OpenedDeviceLocalSecretStore (validated root + lifetime lock, no keyring read)
  -> DB open_preflight_without_backup
  -> one production AppState/SecretService consumes the same opened handle
  -> reconcile current DB/device-local state and derive Clean|Blocked
  -> app.manage + static command registration
  -> Clean only: sanitized structural/history backup
  -> publish gate
  -> start workers/consumers
  -> Blocked: no backup, publish, worker or consumer

SQL import / restore / sync download
  -> register one temp DB live object without staged source validation
  -> temp authority/token + material-free StagedSecretImportActivationProjection
  -> #55 dedicated staged admission (no raw path/value/value digest)
  -> main-integration authority-match receipt
  -> #35 prepare_staged_import / confirm_staged_import
  -> construct exact ImportCutoverCoordinatorContext
  -> staged source validation + exact scrub/readback
  -> cutover consumes admitted staged CAS and returns cutover receipt
  -> live DAO owner token + live owner/device-local binding finalize
  -> post-cutover crash: registered resume accepts only
     {stageId,expectedResumeCas:{revision,digest}}
     and returns currentResumeCas:{revision,digest}
  -> only then ordinary bound-owner readiness/runtime
```

There is no edge from `SecretService` to renderer, a generic Provider getter, a generic JavaScript engine, a generic terminal environment builder, or a backup/export serializer.

## 4. Registered Codex secret source inventory

### 4.1 Canonical and TOML locations

| Source category | Exact current location | Inventory rule | Resolution |
| --- | --- | --- | --- |
| auth JSON API key | `Provider.settings_config/auth/OPENAI_API_KEY`; live `~/.codex/auth.json` equivalent read by `read_codex_live_settings` | canonical legacy primary source; whitespace/empty are invalid, never lowercased | migrate/compare, then scrub; **#35 module** |
| TOML top level | `config` string → `experimental_bearer_token` | enumerate regardless of active provider | migrate/compare, then scrub; **#35 module** |
| TOML regular active table | `[model_providers.<active>].experimental_bearer_token` | enumerate, but active status gives no precedence | migrate/compare, then scrub; **#35 module** |
| TOML regular inactive tables | every `[model_providers.<id>].experimental_bearer_token` where `<id>` is not active | enumerate every table; it is not stale/excludable | migrate/compare, then scrub; **#35 module** |
| TOML inline tables | `model_providers = { id = { experimental_bearer_token = ... } }` for every entry | enumerate every inline table and mixed table-like form | migrate/compare, then scrub; **#35 module** |
| malformed/non-string/duplicate TOML | any of the above with parse failure, duplicate key or non-string value | do not partially parse and do not pick a winner | **RJ** `SECRET_LEGACY_SOURCE_INVALID`, effect none; **#35 module** |

The current native `extract_codex_experimental_bearer_token` / `remove_codex_experimental_bearer_token_if` and renderer `extractCodexExperimentalBearerToken` / `updateCodexExperimentalBearerToken` cover only an active regular table plus top-level fallback. They cannot be used as the migration inventory/scrubber. The new inventory must enumerate and report every regular and inline table before any write.

The current contract includes `providerConfigTomlInlineTable` in `LegacySourceCategory`; static freeze review must preserve it. A count without stable category/location identity is insufficient for conflict review. Location identity is an internal typed key, not a raw filesystem path in public DTOs.

### 4.2 Other current value-bearing shapes

| Source | Current path | Decision | Sole owner |
| --- | --- | --- | --- |
| proxy legacy env alias | `settings_config/env/OPENAI_API_KEY` in `proxy/providers/codex.rs::extract_key` | **RJ** as noncanonical Codex shape; explicit capture/reconcile only, then scrub | main integration |
| proxy top aliases | `settings_config/apiKey`, `settings_config/api_key` | **RJ**; never fallback | main integration |
| proxy nested aliases | `settings_config/config/apiKey`, `settings_config/config/api_key` when `config` is an object | **RJ**; never fallback | main integration |
| renderer preset auth factory | `src/config/codexProviderPresets.ts::generateThirdPartyAuth` and preset `auth` objects create `OPENAI_API_KEY` value slots | replace with token-free preset structure plus credential-required metadata; never put captured material in a preset/fixture | main integration |
| renderer Codex form state | `src/components/providers/forms/hooks/useCodexConfigState.ts` reads auth/TOML fallback into `codexApiKey`, exposes change/reveal state and rewrites auth/TOML | remove Codex value state/parser/updater from renderer; route native capture, owner summary and readiness only | main integration |
| renderer TOML token helpers | `src/utils/providerConfigUtils.ts::{extractCodexExperimentalBearerToken,updateCodexExperimentalBearerToken}` selects and rewrites active/top-level material | not an inventory/scrub authority; Codex token operations move to native typed source inventory under admitted policy | main integration |
| renderer Provider feature drafts | `AddProviderDialog.tsx`, `EditProviderDialog.tsx`, `useCodexProviderFeatures.ts`, `CodexFormFields.tsx`, `CodexConfigSections.tsx`, `CodexConfigEditor.tsx` build/pass full `Provider.settingsConfig` through feature analysis/patch/save | replace with closed token-free feature-draft and mutation DTOs; feature state/public DTO exposes only structural capability/readiness/owner summaries | main integration |
| renderer Provider display/request surfaces | `ProviderCard.tsx` inspects `OPENAI_API_KEY`; `src/lib/api/usage.ts` and `CodexFormFields.tsx` can pass API-key values into usage/model-fetch calls | card consumes token-free public readiness; fixed usage/model-fetch calls carry exact owner/provider identity only and resolve inside the native fixed consumer | main integration |
| empty-key template substitute | `src/config/codexTemplates.ts` emits `auth: { OPENAI_API_KEY: "" }` | delete the field; empty string, `[REDACTED]` and value hash are not token-free credential representations | main integration |
| legacy live sync/backfill | `src-tauri/src/services/config.rs::sync_codex_live` clones auth, writes live, extracts bearer token and writes restored auth/config back into Provider | direct Codex sync **RJ**; non-secret config backfill only; exact live secret write through #55/#41/#35 | main integration |
| UsageScript primary override | `meta.usageScript.apiKey` | new input **RJ**; legacy value may scrub only after constant-time equality with canonical bound primary key | main integration |
| UsageScript login token | `meta.usageScript.accessToken` | unsupported secret purpose in #35 MVP; **RJ**, no coercion into primary key | main integration |
| coding-plan AK/SK | `meta.usageScript.accessKeyId`, `secretAccessKey` | separate credential purpose; **RJ** for Codex #35 path and report as adjacent debt | main integration |
| UniversalProvider shared key | `universal_providers.api_key` / `UniversalProvider.apiKey` | no automatic Codex copy; sanitized public view and **RJ** Codex sync until explicit ref-backed binding exists | main integration |
| deep-link query/remote config | `apiKey`, `usageApiKey`, embedded settings/config token reaches `deepLinkConfigPreview.ts`/`DeepLinkImportDialog.tsx`, which currently decodes/masks/previews it | raw, encoded and remote-config Codex secret forms **RJ** at native ingress before parse/merge/event; renderer never receives or masks the value; stable no-echo error only | main integration |
| full SQL/import/sync/restore | `services/sync_protocol.rs` callers can mutate Skills/main DB from legacy Provider JSON, old proxy backup, foreign refs/secret rows | temp authority/token + material-free projection → #55 staged admission → authority-match receipt → #35 prepare/confirm → construct exact `ImportCutoverCoordinatorContext` → staged source validation/scrub/readback → admitted-CAS cutover → live owner/binding finalize; context is the sole authority from validation through cutover, and unresolved authority, old nonce/admission/CAS or any pre-cutover failure is **RJ/effect-none** | main integration |
| startup history migration backup | `codex_history_migration.rs::backup_provider_settings_config` serializes raw Provider `settingsConfig`; `lib.rs` invokes migration during startup | no call before the no-value gate; after `Clean`, write structural placeholder/non-secret config only; existing generations remain scan/report-only | main integration |
| OS keyring | exact backend instance + opaque backend locator reachable only by `SecretRef` | authoritative source after binding; **CR** | #35 module |
| hardware | registered backend instance/device generation | authoritative only if registered; no fallback; **CR** to allowed process-memory consumers, file/env **RJ** when persistent projection is false | #35 module |
| official OAuth auth | Codex-owned OAuth fields in `auth.json`; managed OAuth manager | preserve in place; never migrate as `codexApiKey`, never backup/project | main integration |
| checked-in Codex source fixtures | `src-tauri/tests/provider_service.rs`, `src-tauri/tests/import_export_sync.rs`, `src-tauri/tests/provider_commands.rs`, `src-tauri/tests/mcp_commands.rs`, `tests/utils/providerConfigUtils.codex.test.ts`, `tests/utils/deepLinkConfigPreview.test.ts`, `tests/hooks/useCodexProviderFeatures.test.tsx`, `tests/hooks/useCodexConfigState.catalog.test.ts`, `tests/hooks/useAddProviderMutation.test.tsx`, `tests/components/AddProviderDialog.test.tsx`, `tests/components/EditProviderDialog.test.tsx`, `tests/components/DeepLinkImportDialog.test.tsx`, `tests/components/CodexFormFields.capabilities.test.tsx`, `tests/config/codexTemplates.test.ts`, `tests/config/therouterProviderPresets.test.ts`, `tests/config/subrouterProviderPresets.test.ts`, `tests/config/xaiOauthProviderPresets.test.ts` encode plaintext, full-Provider, masked-preview or empty-key expectations | replace secret-shaped literals with runtime-generated canaries and invert expectations to token-free DTOs, registered native rejection, structural scrub/placeholders and owner/ref writers; no test-only waiver | main integration |

### 4.2.1 Supplemental no-value coverage domains

The exact path register in §9.4 remains exhaustive; this table classifies the fixed domain identities that the sole main-integration `CodexLegacySourceInventoryBridge` must cover before its private `CompleteLegacySourceInventoryAuthority` can mint an opaque `LegacySourceCoverageReceipt`. It is not a second source allowlist and does not move any path or writer.

| Fixed receipt domain | Closed domain identities | Legal authority | Forbidden |
| --- | --- | --- | --- |
| current Provider/live | `currentProviderLiveScrubbable` | structural revision/presence/count plus exact current `CurrentLegacySourceExpectations` are atomically bound in the same receipt; refs use only opaque non-value-derived `LegacySourceLocationId` | staged origins, raw path/raw locator/value/value-derived digest, aggregate-only count |
| supplemental environment | `processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile` | structural revision/presence/count plus category/state-only adjacent observations are atomically bound in the same receipt | conversion to `LegacySourceRef`, scrub/delete authority, environment name/value, registry/file path, backup payload |
| supplemental common config | `commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge` | structural revision/presence/count plus category/state-only adjacent observations are atomically bound in the same receipt | conversion to `LegacySourceRef`, raw TOML/JSON, storage key/path/raw locator/value-derived digest, automatic migration/scrub authority |

The receipt atomically carries non-value-derived `LegacySourceInventoryRevision`, one fixed complete `CompleteLegacySourceCoverageIdentity` with all 11 domains exactly once, current exact expectations and adjacent observations; every domain identity records only structural revision/presence/count. It is not a bag of positive observations: a zero-occurrence result is complete only when all 11 absent-domain identities and the inventory revision revalidate. The public `LegacySourceCoverageView` is a no-authority projection. Static privacy/compile-fail assertions prove sibling modules cannot field-construct, destructure, clone/serialize or fabricate the authority required by the `pub(crate)` factory. Startup reconciliation, each owner summary/readiness projection, `list_secret_backend_options` and begin/claim revalidation, and Provider-delete preview and confirm each invoke the bridge again. Missing, stale, incomplete, moved or unregistered coverage blocks before candidate/impact id mint, backend access, Provider mutation, backup, worker or consumer publication.

### 4.3 Source reconciliation algorithm

For one Codex Provider owner, `CodexLegacySourceInventoryBridge` freshly inventories all 11 domains and mints one atomic `LegacySourceCoverageReceipt`. The consumer moves that receipt without inspection; the bridge revalidates its revision/domain identity against the exact current expectations and adjacent observations already bound inside it before any operation uses them. Any present supplemental domain prevents migration/capture/owner-ready/Provider-delete progress until its closed resolution path completes; it never enters a scrub set. The current-source algorithm then acts once:

1. Zero non-empty canonical occurrences: owner is `unbound` unless a valid binding exists.
2. One unique byte value across one or more canonical current locations and no binding: stage a verified migration candidate with `comparisonPolicy=candidateEquality`; activate only through #55. At activation, fresh-read the exact set/revisions and constant-time compare each admitted value with the candidate, then scrub each admitted occurrence.
3. More than one distinct byte value: `sourcesConflict`; keep legacy bytes internal, expose only a sanitized conflict summary, and write nothing.
4. Existing binding plus inline values: resolve the existing binding and constant-time compare with each current occurrence. Equality permits a scrub-only candidate with `comparisonPolicy=candidateEquality`; any difference is `bindingConflict`; locked/denied/unavailable is `bindingComparisonPending`. Probe/presence alone never permits scrub.
5. User resolution of `sourcesConflict`/`bindingConflict`, explicit replace/reconcile and rotate stages a verified candidate with `comparisonPolicy=explicitReplacement`. The plan must display/freeze the exact old source set and revisions. Activation revalidates that structure and scrubs the approved old sources, but it **does not** require old source bytes to equal the new candidate.
6. A legacy UsageScript `apiKey` may join automatic scrub-only only under `candidateEquality`. A distinct value can be removed only through an explicit replacement plan; `accessToken`, AK/SK and other purposes remain separate unsupported-purpose debt and are never coerced into the primary key.
7. Backend write/readback and durable operation phase create only an unbound record/candidate. #55 admitted activation later precedes device-local binding CAS and exact source scrub. Unknown outcome is reconciled from the non-material operation journal on restart.
8. Public projection runs before migration status is known, so blocked/failed migration cannot leak plaintext.
9. Ordinary activation accepts only `CurrentLegacySourceExpectations` (`providerRow|liveAuth|liveConfig`). Staged origins may seed a candidate only through a dedicated staged-import projection/admission/cutover flow; they never enter ordinary `legacySourcesToScrub`.

No active-location priority, last-write-wins rule, provider category guess, empty-string substitution or silent deletion is allowed.

### 4.4 Closed Codex live sinks

The only v1 `CodexLiveSecretSinkId` literals are:

| Sink ID | Exact logical slot | Forbidden substitutions |
| --- | --- | --- |
| `codexAuthJsonOpenAiApiKey` | API-key slot in Codex `auth.json` | OAuth fields, whole-file bytes, path string |
| `codexConfigTomlExperimentalBearerToken` | `experimental_bearer_token` slot in Codex `config.toml` | another TOML key/table, whole-file bytes, path string |

Every target/rollback credential projection carries exactly one `liveSinkId`. #55 hashes the exact role/ID pair in admission; #41 binds it into the matching owner-private writer construction, structural readback and final baseline. Generic `externalConfigFile` remains only a capability class and never identifies the mutation target. Unknown or missing IDs fail before backup, Provider lease mutation, material acquisition or target write.

## 5. Source → consumer → sink decision matrix

### 5.1 Provider CRUD, DTO and startup/live paths

| Source → current call path | Current sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| DB `providers.settings_config` → DAO `get_all_providers/get_provider_by_id` → `commands::get_providers` | raw `Provider` serialized to renderer | **SP** through exact `CodexProviderPublicDto`; Codex settings are a closed token-free projection and `SecretOwnerCredentialSummary` is joined separately | main integration | Rust serialization test injects canary into every legacy location and walks every Codex list/get result |
| renderer full Provider → add/update/add-draft/update-settings → DAO save | plaintext persisted and may be applied | exact `CodexProviderMutationDto` has no auth/token fields; unknown secret-shaped paths **RJ** before DB/live writes | main integration | add/update/draft table test: each forbidden field returns stable code and DB/live snapshots remain byte-identical |
| Add/Edit dialogs → `useCodexProviderFeatures` analysis/patch → Codex form/editor/card/save | full Provider/settings draft and key-shaped placeholders circulate in renderer/public feature APIs | closed token-free feature-draft/public/mutation DTOs only. Feature patch returns structural TOML/config plus capability/readiness state; save carries no auth/token/value field, and `codexTemplates.ts` omits `OPENAI_API_KEY` entirely | main integration | named feature/form/dialog/template fixtures use runtime-generated canaries; full Provider, unknown secret field and empty-key substitute all reject or serialize absent with no DB/live write |
| `App.tsx` generic delete dialog → `providersApi.deleteWithResult` → query mutation → Provider DAO delete | user sees only “irreversible”; binding/orphan/retention impact across SQLite + device-local authority is hidden | bound/unbound first fetches no-value `CodexProviderDeleteImpactDto`; legacy is blocked before preview and receives no `providerDeleteImpactId`. Confirm returns only the opaque id and consumes its exact revisions/CAS once. Changed/expired/replayed Provider impact returns `PROVIDER_DELETE_IMPACT_STALE`, `action=refreshProviderDeleteImpact`, `effect=none`; this Provider-domain envelope/action is not `SecretErrorCode`/`SecretUserAction`, while `refreshDeleteImpact` is reserved for separate secret deletion | main integration | UI renders remaining owners/orphan/`secretRetained`/separate delete in all locales; legacy has no confirmable impact id; cancel/stale/replay are effect-none; schema tests reject Provider error/action in secret unions and vice versa |
| admitted Provider owner detach | crash ambiguity across Provider DB + device-local authority | durable detach intent → exact Provider DB transaction → durable provider-finalized checkpoint → device-local owner authority detach/binding-set CAS → terminal audit. Backend entry and every other owner remain; orphan secret deletion is a separate impact/confirmation operation | main integration | phase-by-phase restart; shared-ref and orphan matrices prove `secretRetained=true` and backend entry survives |
| `read_live_provider_settings(Codex)` → `Value` IPC | live secret to renderer | **SP** exact `CodexLiveStructuralProjection`; legacy raw command rejects Codex branch | main integration | command schema + canary absence test |
| startup `lib.rs`/database init → default import → `read_codex_live_settings` → Provider DAO | DB may make a raw preflight backup, publish consumers or start workers before secret authority/reconcile is clean | exact order: open `OpenedDeviceLocalSecretStore` → `open_preflight_without_backup` → one AppState/SecretService consumes that handle → reconcile → `app.manage`/static command registration → on `Clean` only create sanitized backup → publish gate → workers. `Blocked` creates no backup and starts/publishes no worker or consumer | main integration | ordering/object-identity test records each step; legacy key + locked store yields Blocked, raw-backup spy=0, worker/consumer/publish counts=0, one handle/no reopen/no path injection |
| `lib.rs` startup → `codex_history_migration.rs::backup_provider_settings_config` | raw Provider `settingsConfig` written into a history generation | history migration is downstream of the Clean no-value gate. New backups contain structural placeholder/non-secret config only; existing secret-bearing generations are discovered and reported, never rewritten/deleted | main integration | inline history/lib tests inject a runtime canary, prove zero pre-gate backup and absent canary in new generations; historical fixture bytes/hash stay unchanged while report count/category is present |
| `src-tauri/src/services/config.rs::sync_codex_live` → `restore_codex_provider_token_for_backfill` / `restore_codex_settings_for_backfill` | live/config token lifted into stored Provider auth and config | remove Codex token backfill and direct secret sync; only non-secret settings backfill remains | main integration | revised `provider_service.rs` + `import_export_sync.rs` generated-canary tests prove Provider JSON token-free and direct sync rejected |
| `ConfigService::sync_current_providers_to_live` / `sync_current_to_live` / `ProviderService::switch` | unplanned raw write and current-state mutation | Codex direct entry **RJ**; only immutable #55 plan → #41 apply path may write | main integration | direct-call/command tests return unsupported transition, effect none; existing sync fixtures no longer assert plaintext propagation |
| Provider/public event and mutation result | internal Provider may escape through future command/event | public result types contain IDs/status only; compile-time command registry test forbids `Provider` return/field on Codex routes | main integration | contract-schema scanner |

`Provider` may remain `Serialize` temporarily for internal DB mechanics, but no renderer-facing Codex function may have `Provider`, `UniversalProvider` or raw `Value` in its signature. The stronger end state is to remove `Serialize` from internal `Provider`; until then, the registry scanner is a blocking gate, not a convention.

### 5.2 Usage, balance, coding plan and model fetch

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `queryProviderUsage` → `resolve_native_credentials` → fixed `token_plan` adapter | native HTTP auth header | **CR** as `usageProbe`; request carries owner/provider id, never value | main integration | fixed-adapter test proves one resolve, header use, stable result and zero public canary |
| `queryProviderUsage` → balance branch → `services::balance::get_balance` | HTTP header; upstream body currently enters error | **CR** as `usageProbe`; map auth/status/body to stable code, never raw body | main integration | 401/500/body-canary test; result/cache/tray/log all clean |
| public `commands/balance.rs::get_balance(base_url, api_key)` | value IPC | Codex branch/value form **RJ**; a new owner/ref request delegates to fixed native consumer | main integration | IPC decoder rejects `apiKey`; no request sent |
| `queryProviderUsage` → generic `ProviderService::query_usage` → `usage_script::execute_usage_script` | template replacement places key in QuickJS source/request; extractor may return it through `extra/invalidMessage` → UsageResult/cache/tray/renderer | **RJ** for Codex primary secret. Credential-free scripts may run without resolver; no secret-bearing script fallback | main integration | malicious extractor returning `{{apiKey}}` is rejected before resolve/eval/network |
| `testUsageScript(apiKey/accessToken, ...)` | value IPC and arbitrary script | Codex inline values and primary-key fallback **RJ**; credential-free test only | main integration | command tests for explicit value and omitted-value fallback, both effect none |
| `src/lib/api/usage.ts` / `ProviderCard.tsx` → usage queries or script test | renderer API accepts/derives key-shaped fields and card reads secret-bearing Provider config | Codex fixed usage request contains owner/provider identity only; card renders token-free public readiness. Generic script-test values are rejected and cannot fall back to the Provider binding | main integration | API/card/feature fixtures inject a runtime canary and prove request/public state/cache/log absence; locked/denied creates no request |
| `UsageScriptModal.tsx` Provider credentials → `src/lib/api/subscription.ts::getCodingPlanQuota(baseUrl,apiKey,...)` → `commands/coding_plan.rs` → fixed Kimi/Zhipu/MiniMax adapters | Provider primary key crosses renderer/public IPC as a value and becomes a native HTTP header | replace the value chain with an existing-owner/provider request. #35 mints the exact binding token; the fixed adapter consumes `AuthorizedBackendRead` with `consumer=FixedRuntimeConsumer::CodingPlanUsageProbe` and closed `CodingPlanPrimaryAdapter` at final header/send. This consumer belongs to `usageProbe/codex_feature_runtime`, not adjacent debt | main integration | cross-layer request schema rejects `apiKey`; per-adapter test proves one owner-scoped resolve, redirect count=0, stable result/error and public/cache/log canary-zero |
| ZenMux hand-entered `apiKey/baseUrl`; Volcengine `accessKeyId/secretAccessKey`; team/login IDs/tokens | independent credentials and signed/team request paths | separate purposes outside Provider-primary #35; never coerce into or resolve through `codexApiKey`. Keep as explicit adjacent repository debt | main integration | purpose-confusion tests prove fixed primary adapter cannot accept these fields and independent branch cannot consume a Provider binding |
| `CodexFormFields.tsx` → `fetchModelsForConfig(baseUrl, codexApiKey, ...)` → `fetch_models_for_config(api_key)` → `services::model_fetch::fetch_models` | renderer value state/IPC, cloned log secret list, HTTP header, raw response preview | replace the whole fixed path with owner/provider request and **CR** as `modelFetch`; base URL/adapter metadata is validated structurally, material resolves only in the owner-private final-send callback, stable errors only | main integration | form/API/IPC fixtures reject `apiKey`; owner/ref request, one resolve, redirect-none, request/body/error canary scan and locked/denied no-network tests |

The old Codex branch of `Provider::resolve_usage_credentials` and `ProviderService::extract_credentials` must stop returning a `String` key. Non-Codex branches are adjacent debt and cannot be cited as repository-global closure.

All credential-bearing proxy, fixed usage/balance and model-fetch transports use a dedicated client with `redirect::Policy::none()`. A 3xx maps to a stable upstream result; it cannot cause a second request or forward Authorization. Focused tests use a redirecting first server and a second-server request counter and require counter `0` for every fixed consumer.

### 5.3 Proxy, adapter, forwarder and failover

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `CodexAdapter::extract_key` → `extract_auth` → owned `AuthInfo { api_key }` | long-lived clone/debug-capable material | Codex adapter returns a non-sensitive `AuthRequirement`/header strategy, never material | main integration | adapter type/compile test has no secret field and legacy aliases do not resolve |
| `forward_with_retry` → provider router → adapter auth → `forward` | outbound `Authorization`/`x-api-key` | per attempt **CR** as `proxyRequest`; resolve at the final header/send boundary and drop immediately | main integration | in-memory backend count=1 per attempted Provider; request succeeds; all logs/events clean |
| resolver missing/locked/denied/confirmation/revoked | existing retry loop may try next Provider | terminal, circuit-neutral failure; no network and no failover advance | main integration | queue of two Providers proves second is not attempted on secret failure |
| real transport/upstream failure after authorized send | failover queue | existing network failover may advance; each new Provider performs its own independent **CR** | main integration | two-provider transport failure test; no material reused across attempts |
| generic `AuthInfo` for other adapters | non-Codex adjacent credentials | retain temporarily outside Codex scope; remove ordinary `Debug`/secret-bearing logs where shared | main integration | repository inventory baseline, not feature PASS |
| proxy request/usage logs | URL/status/body/model/error may carry reflected material | **SP** stable fields only; strip headers and raw bodies/errors before DB/log | main integration | upstream reflection canary absent from `proxy_request_logs`, files and events |

### 5.4 Proxy takeover, backup, restore and crash recovery

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `start_with_takeover/set_takeover_for_app` → `backup_live_configs` → `save_live_backup` | raw `{auth, config}` in DB | Codex backup is an **SP** structural bundle with token slots replaced by typed owner/ref/location/revision placeholders | main integration | DB backup row canary-zero test and typed round-trip |
| takeover → Codex placeholder writer | live file | require clean migration/binding and immutable plan. OS-keyring provider may project only if capability permits; hardware false **RJ** | main integration | backend capability matrix, exact sink and effect-none tests |
| official OAuth `auth.json` during takeover | copying OAuth tokens to DB backup or overwriting login | preserve file in place/config-only takeover; never DB-backup it. If safe preservation cannot be proven, **RJ** takeover | main integration | OAuth fixture remains byte-identical and backup has no auth bytes |
| `update_live_backup_from_provider(_inner)` during provider update/switch | Provider settings serialized into live backup | rebuild only sanitized structure + placeholder; never raw Provider auth/TOML token | main integration | Provider canary absent from backup after hot switch |
| `restore_live_configs` / disable takeover | raw backup written back | #41 recovery obtains a fresh capability and rehydrates placeholder inside #35 closure; capability itself is never persisted | #41 | restart/recovery test with expired old capability and fresh preparation |
| `recover_from_crash` at startup | automatic secret replay before readiness | reconcile structural job only; no automatic material replay. Recovery becomes an explicit/fresh #41 operation; unsupported backend stays blocked | #41 | crash at every phase; no capability/material in journal/backup |
| stale takeover marker without backup | placeholder cleanup may destroy intent | typed reconcile: cleanup only verified proxy placeholders; never infer/delete user token | main integration | marker-only and user-edited-token tests |

The existing #41 exact-byte backup contract is incompatible with this matrix. “Backend-only” does not make a plaintext backup acceptable.

### 5.5 Terminal, UniversalProvider, deep links and failover UI

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `open_provider_terminal` → `extract_env_vars_from_config` → terminal env/temp settings/script | child environment and persistent temp JSON/script | Codex **RJ** at command boundary in MVP; no resolve and no temp/script creation | main integration | filesystem/process spy proves zero side effects |
| `UniversalProvider.apiKey` → `to_codex_provider` → `auth.OPENAI_API_KEY` → DAO | shared plaintext copied into Codex Provider | sanitized public projection; Codex conversion/sync **RJ** until explicit Codex owner binding exists. Do not auto-migrate shared key while other apps use it | main integration | `apps.codex=true` legacy row creates no Codex Provider and reports migration-required |
| Universal CRUD commands returning/accepting `UniversalProvider` | renderer value | Codex-enabled rows use public/mutation DTO without key; legacy stored key remains hidden pending cross-app migration | main integration | list/get/upsert/sync contract tests with canary |
| native deep-link parser/remote-config merge/provider builder → event → `deepLinkConfigPreview.ts` / `DeepLinkImportDialog.tsx` → add/switch | renderer currently decodes, masks and previews `apiKey`, `usageApiKey`, usage token or embedded settings/config material | Codex links are metadata-only. Native ingress rejects raw query, percent-encoded and downloaded/merged remote-config secret shapes before parse/merge/provider construction/event dispatch; renderer never decodes/masks/previews the rejected value and no DB/live mutation occurs | main integration | generated-canary negative table covers raw, encoded, nested auth/TOML and remote config; every case returns the same stable no-echo code and event/preview/provider-writer spies remain zero |
| `get_available_providers_for_failover` → `Vec<Provider>` | raw settings to renderer | **SP** failover candidate DTO (`id/name/readiness` only) | main integration | list canary-zero/schema test |
| internal router/DAO failover Provider | native selection | internal Provider contains no inline Codex secret after migration; secret readiness failure stops failover as above | main integration | router test for ready/missing/locked states |

`providerTerminal` may remain a wire-reserved future `SecretConsumer`, but it must not appear in any v1 record's `allowedConsumers`, and no Codex command may request `childProcessEnvironment` in this MVP.

### 5.6 Import, export, sync and database restore

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| Provider/manual export | exported JSON/SQL | **SP** non-secret Provider structure plus `credentialRequired`; omit material, backend locator and device-local ref identity | main integration | export archive canary scan + re-import becomes unbound |
| `export_sql_string_for_sync` / WebDAV / S3 | cross-device SQL | provider rows are sanitized; #35 record/ref/binding/audit/journal never exist in SQL, so snapshot has nothing to skip/import. Upload/download must leave device-local state hash/revision unchanged | main integration | two-device round-trip retains local bindings and SQL contains no secret-authority identifiers |
| `services/sync_protocol.rs` → WebDAV/S3/archive callers → Skills/main DB mutation | protocol helpers can apply downloaded state before the Codex staged secret gate | callers may register/open only one temp live object first, without staged source validation. After temp authority/token+projection, #55 admission, authority-match receipt and #35 prepare/confirm, main integration constructs the exact `ImportCutoverCoordinatorContext`. That context is the sole authority for staged source validation/scrub/readback and admitted-CAS cutover; live owner/binding finalize follows the cutover receipt | main integration | order/port spies prove no validation/scrub/readback/cutover before context construction and prove cancel/expiry/replay/old nonce/admission/CAS/foreign-ref/scrub failure/pre-cutover crash leave Skills/main DB/live/device-local hashes and revisions byte-identical |
| SQL import / binary restore / WebDAV-S3 download → temp DB | new owner has no current DAO authority; staging origin may contain plaintext or foreign refs | ImportCoordinator registers the temp live object without staged source validation and mints a `StagedSecretOwnerToken` plus material-free staged projection. Full object identity is held only in an internal preimage; public projection/receipt contains stable IDs/CAS but no path, value or whole-object digest. Source validation waits for the exact context; foreign secret authority is **RJ** | main integration | fabricated/replayed/wrong-temp token and projection tests; staged DTO/fixture scan proves no raw path/value/value digest/ref authority; ordering spy keeps source-validation port at zero before context construction |
| #55 staged admission → authority-match receipt → #35 staged prepare/confirm → construct exact `ImportCutoverCoordinatorContext` → validation/scrub/readback → DB cutover → live finalize | ordinary activation cannot authorize staging refs or atomic cutover | #55 first freezes the material-free staged projection/source-set CAS and admits it. Main integration proves admission matches its internal temp authority and mints a consuming receipt; #35 prepare/confirm is reachable only from that receipt. Immediately after #35 confirmation, main integration constructs the exact `ImportCutoverCoordinatorContext`; this sole authority gates all staged source validation/scrub/readback and admitted-CAS cutover. Only the cutover receipt permits live DAO owner token and device-local binding finalize | main integration | confirmation cancel/expiry/replay and every pre-cutover failure keep Skills/main DB/live/local binding exact unchanged; order spies prove no #35 prepare before authority-match, no validation/scrub/readback/cutover before context, and no live finalize before cutover receipt |
| initial staged activation result | initial activation is not crash resume and has its own closed result type | never decode or return the initial-activation DTO from `resume_staged_import_cutover`; its candidate/owner projection, if legal for the initial flow, remains unreachable from every resume arm | #35 module | Rust/TS discriminant tests reject cross-decoding between initial activation and resume result types; main integration consumes only the published resume type |
| staged cutover crash/restart | cutover may commit while local binding/final receipt remains incomplete | request data is exactly `{stageId,expectedResumeCas:{revision,digest}}`. Every separate closed resume result data arm is exactly `{stageId,currentResumeCas,status,action,issue}`: `activated|alreadyActivated` has `issue=null`; `recoveryRequired` has its typed issue. Version/command id remain in the common envelope and audit is independent, so result data forbids `schemaVersion`, `auditEventId`, candidate/owner/ref/summary or any unlisted field. Internal CAS preimage includes journal `operationId` plus cumulative-field `StagedImportResumePhase`: `intent`; `sourcesScrubbed+stagedSourceSetCasAfterScrub`; `cutoverCommitted+stagedSourceSetCasAfterScrub+cutoverReceiptId`; `liveOwnerMinted+stagedSourceSetCasAfterScrub+cutoverReceiptId+promotedLiveOwner`; `localBindingFinalized` with those same three checkpoint fields. Cross-arm missing/extra fields are forbidden. Wrong object, old nonce/admission/CAS, stale or replayed request is zero-write/effect-none | main integration | exact five fixtures `staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`; required/forbidden field scan; every fresh nonce/admission and phase/checkpoint transition increments revision/new CAS; exact-CAS resume reaches terminal while stale/wrong-object/replayed resume returns current CAS with no forbidden field and leaves all authorities unchanged/consumers blocked |
| manual/ordinary candidate activation | accidentally accepting `sqlImportStaging|dbRestoreStaging|syncDownloadStaging` would let a temp owner reach current readiness/runtime | accepts only `CurrentLegacySourceExpectations` (`providerRow|liveAuth|liveConfig`). Staged origins are never normalized or passed to ordinary activation/readiness/runtime | main integration | decoder/compile tests reject staged origin in ordinary activation and current origin in staged-only fields |
| remote sync cutover → post-import sync | current flow can replace main DB then report live write as warning | no “import succeeded with post-sync warning.” Same AppState/SecretService refreshes after admitted staged finalize; only then may #55/#41 schedule a separate live apply | main integration | blocked preflight/cutover keeps main DB/live bytes and device-local revisions unchanged; AppState construction count remains one |
| periodic local DB backup | DB/WAL copy | allowed only after invariant that Provider/proxy rows contain no material; it contains no #35 device-local records/refs/bindings under any recovery mode | main integration | pre-backup invariant failure blocks snapshot; absence + artifact canary scan |
| `run_post_import_sync` / `sync_current_providers_live` | direct raw writer | Codex direct sync **RJ**; schedule immutable #55 plan/#41 apply instead | main integration | no direct ProviderService sync call for Codex |
| old user-created/FyAgent-managed backups/exports | historical artifact | v1 inventory/report only; no rewrite/delete projection or command | main integration | read-only scan report with counts/categories, no raw values/path |

### 5.7 Diagnostics, logs, errors, cache, tray and crash files

| Source → current call path | Sink/risk | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `AppError`/`String` conversions in provider/usage/model/proxy/import paths | renderer/log receives OS/upstream/config content | secret boundary returns stable code + retry/action/effect only; no arbitrary message | main integration | error matrix with canary in OS/upstream error |
| `usage_script` non-2xx response preview | `UsageResult.error` → cache/tray/renderer | Codex secret-bearing JS rejected; fixed consumers map status to stable code and discard body | main integration | reflected-body canary scan |
| `services::usage_cache` / tray summary | cloned UsageResult | only typed numeric/plan fields from fixed consumer; no raw `extra/invalidMessage` from secret-bearing script | main integration | cache/tray serialization test |
| `frontendLogger.ts` | renderer console/plugin log | renderer never receives material; keep redaction as defense, not authority | main integration | fixture event/log scan |
| `panic_hook.rs` → `crash.log` | panic payload may interpolate settings/error | secret types have redacted formatting; panic hook applies defense-in-depth redaction and bounded stable context | main integration | induced safe panic with canary-bearing upstream error; crash file clean |
| proxy request log DB / plugin logs | request/response/error reflection | headers/body/raw config forbidden; stable metadata only | main integration | DB + file + event scan |
| diagnostic/public health views | Provider/live/backend details | **SP** availability/capability/counts and stable issues; no material, value-derived digest, raw ref/path/error | main integration | DTO forbidden-key and canary tests |
| audit/journal | lifecycle/operation records | typed IDs/revisions/stable codes only; never material or material-derived digest | #35 module | schema + serialization + crash-phase tests |
| backend/device revocation signal | adapter may report missing/error or an unauthenticated reason | exact registered handle first validates revocation capability, then accepts an explicit `Revoke` authorization and returns a non-clone consuming receipt bound to ref/device-store instance/record/binding-set/exact registered backend object/device/capability CAS plus closed source/time; state commit fresh-revalidates and consumes both. Probe/not-found/locked/denied/unavailable remain their own non-persistable states | #35 module | forged/stale/cross-ref/cross-store/cross-backend authorization/receipt rejection; probe or OS-keyring missing never persists revoked; hardware receipt readback preserves source/time without raw detail |
| `SecretUserAction` in errors/recovery/UI and generic legacy action strings | prose/source-dependent choices or old aliases can bypass the canonical destination or become a dead retry loop | `SECRET_ACTION_DESTINATIONS_V1 satisfies Record<SecretUserAction, SecretActionDestination>` is canonical: each secret action has one fresh command, fixed flow, confirmation continuation, `mainIntegrationCommand` or guidance. `completeRecovery` is impact→retry; `resumeStagedImportCutover` is exactly `resume_staged_import_cutover({stageId,expectedResumeCas:{revision,digest}})`. `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` all resolve to `list_secret_backend_options` (native snapshot + fresh single-use `SecretCaptureIntentId`) → `begin_secret_capture(intentId,selectedBackend)`; terminal expiry prepends summary/owner-card refresh. Provider-domain `refreshProviderDeleteImpact` never enters this table | #35 module | Rust/TS/fixture key parity; begin rejects raw owner/legacy/binding/backend snapshot and reused intent; every capture action hits the same list→begin flow; terminal expiry refreshes before mint; old ids/admissions/CAS replay zero writes |

## 6. #55 Change Plan closure

### 6.1 Plan definition and digest

#55 owns plan schema, canonical DTO/decoder, plan/job DAO and all plan/job digests. It must replace the current source behavior as follows:

| Current #55 path | Required replacement | Outcome | Sole owner | Focused evidence |
| --- | --- | --- | --- | --- |
| `provider_definition_digest(&Provider)` | #55 owns its closed token-free Provider structural projection; its credential member is exactly #35 `SecretApplyPlanProjection`, and its live member is exactly `CodexLiveStructuralProjection`. Digest only non-secret config, owner/ref identity, binding/revision/capability contract and affected resources | **SP** | #55 | two Providers differing only in material produce the same structural digest; revision/ref change alters it |
| `codex_projection_for_digest(Value)` | parse into a type; reject forbidden keys rather than delete a partial list after serialization | **SP/RJ** malformed | #55 | property/fixture test covers every legacy source location |
| `live_projection_digest(read_live_settings)` | digest `CodexLiveStructuralProjection` with every token/OAuth field removed before hashing | **SP** | #55 | canary changes do not affect digest; non-secret config change does |
| plan public DTO | readiness, backend capability, binding-set CAS, exact affected resources/sink; no raw config/value-derived fingerprint | **SP** | #55 | canonical Rust/TS/fixture parity test |
| candidate activation projection/admission | include the exact `LegacyActivationComparisonPolicy` in projection, affected resources, admission and plan digest. `candidateEquality` and `explicitReplacement` are distinct immutable plans | **SP** | #55 | policy flip changes digest; equality fixtures reject unequal current values while explicit replacement accepts them only with unchanged source set/revisions |
| staged import activation projection/admission | use `StagedSecretImportActivationProjection` with stage/source-set CAS and dedicated operation discriminant; never decode it as ordinary activation or live apply, and never carry path/value/value digest | **SP/RJ** malformed | #55 | Rust/TS/fixture parity plus cross-discriminant, raw-field and staged/current-origin negative tests |
| `apply_codex_switch` direct `ProviderService::switch` | call #41 canonical coordinator with immutable plan id/digest; no #55 resolver | **RJ** direct writer | #55 | writer spy proves only #41 port is called |
| post-write `inspect_codex_switch` material hash | consume #41 typed `ReadbackMatched`/stable failure receipt | **SP** | #55 | receipt schema contains no value/digest/path/detail |

#55 may persist a digest of the sanitized projection; it may not persist a digest of a secret, secret-bearing Provider, raw live bytes, a capability, or a backend locator. It does not resolve material.

### 6.2 #55 source handoff gate

A successor to `ca552f4d` is consumable only when:

1. its immutable SHA is named in #35/#41 handoff docs;
2. the exact `change_plan.rs`, command, DAO, schema, Provider-writer seam and canonical Rust/TS fixture blobs are read back;
3. static diff proves no full Provider/live serialization reaches a digest or row;
4. direct `ProviderService::switch` is absent from the plan service;
5. source/test changes between source SHA and final branch SHA are byte-identical or explicitly re-reviewed.

## 7. #41 prepare / lease / backup / readback / recovery closure

#41 owns the coordinator and Provider write/readback/recovery adapter. Candidate activation and live apply are separate immutable plans and separate leases; an apply projection can only name an already-bound owner:

1. #55 atomically admits the candidate-activation plan/job.
2. #41 asks #35 for activation-specific preparation and completes any old-record hardware confirmation. No material is acquired and no lease is held.
3. #41 acquires the activation Provider lease and asks #55 for final activation admission/baseline verification.
4. #41 passes an opaque already-held lease/final-baseline context into #35. Before durable intent or CAS, #35 resolves the complete current-source expectation set and verifies each revision. For `candidateEquality`, it fresh-reads the candidate/backend and constant-time compares every admitted value. For `explicitReplacement`, it verifies the exact approved old source set/revisions and candidate authority but does not assert that old values equal the new candidate.
5. #35 commits device-local binding CAS, scrubs exactly the admitted refs through the passed transaction port, performs structural scrub readback and returns a typed result; it does not write the live target. #41 releases the activation lease.
6. The now-bound owner is re-read. #55 creates and admits a distinct `SecretApplyPlanProjection` for target and optional rollback.
7. #41 prepares target and rollback/current capabilities and completes optional confirmation before a new Provider lease; no material is acquired.
8. Under the apply lease, #41 rechecks the apply plan/final baseline, creates sanitized structural backup and checks cancellation before the first live mutation.
9. #41 consumes the target capability through #35 with an opaque apply coordinator context. The exact owner-private consuming executor writes once and reads back; only a typed result exits.
10. On failure, rollback uses its separately prepared capability if still valid. Otherwise the job records a stable recovery requirement; it never persists capability/material. #41 releases the lease, then refreshes.

| #41 path | Frozen decision | Sole owner | Focused evidence |
| --- | --- | --- | --- |
| activation prepare | activation bundle contains no material; binds candidate/admission/comparison policy/old-record cleanup expectations and completes any old-record hardware confirmation before lease | #41 | candidate-equality vs explicit-replacement fixtures; cancel/expiry/replay leaves binding/Provider unchanged |
| apply prepare | capability contains no material; bind all revisions/consumer/sink/expiry only after owner is bound and a separate apply plan exists | #41 | rotate/delete/lock after prepare causes stable pre-write failure |
| Provider lease | one per app; acquired after confirmation and before final baseline | #41 | concurrency test proves one writer and no keyring call while waiting |
| candidate activation coordination | #41 supplies held activation lease + final baseline to the #35-owned service API; #35 applies the admitted comparison policy before intent/CAS, never acquires lease or writes the live target, and the lease is released before apply readiness | #41 | equality policy catches value drift; explicit replacement accepts unequal old values only with unchanged exact set/revisions; crash partial maps exact cleanup state |
| backup | **SP** typed placeholders only; no exact secret-bearing bytes | #41 | serialized bundle canary-zero + restore round-trip |
| write | only #35 `resolve_for_apply` closure; exact plan sink | #41 | capability single-consume and sink mismatch tests |
| readback | equality happens in closure; receipt is boolean/count/stable code only | #41 | mismatched live value never appears in job/event/log |
| rollback | use separately prepared current/rollback capability; no value from backup | #41 | target write failure restores structurally and records typed receipt |
| crash recovery | persist phase/placeholder only; restart requires fresh prepare and compatibility recheck | #41 | phase-by-phase restart matrix |
| hardware/no projection | file/env target rejected before backup/lease mutation | #41 | `persistentTargetProjection=false` effect-none test |

The current #41 design's secret-bearing exact-byte `ApplyBackupBundle` must be revised before its “frozen” label can be consumed.

General operation recovery uses the same public impact/retry entry but is a closed kind union, not an activation-only bag:

| Recovery kind | Authority/execution rule | Sole owner | Focused evidence |
| --- | --- | --- | --- |
| `activationCleanup` | prepare active-read, old-delete and old-missing authorizations before a #41-held Provider lease. `RecoveryOldRecordDeleteCheckpoint` and crash-visible `ActivationOldRecordDurableCheckpoint` persist `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`; only separately authorized `Validate` missing readback consumes that CAS. Missing receipt and terminal supersession commit atomically with `revokedAt=backendCompletedAt`; consume exact recovery CAS under lease | #41 | structural scrub/delete/readback crash matrix; pre-terminal crash preserves all three checkpoint fields plus `verifyOldRecordMissing`; terminal preserves exact supersession time; cross-slot substitution and stale-CAS effect-none |
| `captureCompensation` | local-only probe, independently authorized candidate delete, then separately authorized fresh missing readback; no Provider lease; terminal only after both durable checkpoints | #35 module | present/missing/unknown + delete/readback authorization substitution + confirmation cancel/expiry matrix |
| `deleteFinalization` | local-only replay of an admitted user delete; deletion and fresh missing readback have separate authorization/checkpoints; preserve truthful explicitly authorized Revoke source/time and never collapse accidental missing | #35 module | crash after intent/delete/readback/state checkpoints; cross-step authorization rejection; revocation provenance readback |
| `ownerDetachFinalization` | consume only an unforgeable already-held Provider detach context and exact impact; backend record remains | main integration | crash after Provider commit completes local detach; other owners/orphan record retained |

Every kind returns material-free remaining steps, exact recovery CAS, its variant-appropriate owner/candidate identity and exactly one closed action. `SECRET_ACTION_DESTINATIONS_V1` is a total `Record<SecretUserAction, SecretActionDestination>`; decoder/UI mappings cover command, command-flow, confirmation, main-integration and guidance discriminants. Normal activation uses `ActivationOldRecordDeleteCheckpoint`; recovery uses `RecoveryOldRecordDeleteCheckpoint`; `ActivationOldRecordDeleteApplied` carries the normal-flow postcondition and `ActivationOldRecordDurableCheckpoint` is crash-visible. All retain the three-field delete checkpoint and terminal timestamp rule; neither path may retain CAS alone or reconstruct disposition/time after crash.

Candidate explicit discard/expiry is not a fifth recovery kind. Each invocation prepares distinct candidate-delete (`Delete`) and candidate-missing-readback (`Validate`) slots. Delete/already-missing persists `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}` in the existing discard journal; only a fresh missing authorization/receipt consuming that CAS advances `missingReadbackVerified`. Until that receipt and state commit finish, the candidate remains `verifiedPendingPlan` with immutable `pendingTerminalDisposition=discarded|expired` and reachable `discardCandidate`; retry cannot relabel it. Only then may it become terminal and remove the pending field.

The count is over slots, not operation enums: normal activation plus all recovery kinds expose 10 authorization/confirmation slots; candidate discard adds 2, yielding 12. The five delete→missing pairs contribute 10 delete/missing slots. Hardware policy remains exactly `CaptureVerify|Validate|ResolveForApply|Delete|Revoke`, and journal/recovery totals remain 8/4.

Staged import is also not ordinary activation. `main integration` owns the ImportCoordinator, temp-object token, exact `ImportCutoverCoordinatorContext`, cutover, live owner/binding finalize and exact-CAS resume; #55 owns the `StagedSecretImportActivationProjection` schema plus dedicated immutable plan/admission contract. Main integration constructs the context only after authority-match and #35 confirmation, and that context is the sole authority before every staged source validation/scrub/readback/cutover. The flow does not acquire or impersonate a #41 Provider lease and does not transfer import ownership to #41. The temp-owner token never authorizes live readiness/runtime, and no staged DTO, journal, receipt or digest contains raw path, value or value-derived digest.

## 8. V2 adapter and composition closure

V2 has four non-overlapping lanes:

| Lane | Exact boundary | Sole owner | Focused evidence |
| --- | --- | --- | --- |
| credentials | credential types/decoder/port/browser fixture under `shared/data/credentials`; Tauri invoke only in `shared/platform/tauri/credentials.ts`; credentials panel below Models | #35 module | decoder rejects forbidden/unknown secret fields; browser fixtures contain no secret-shaped values |
| canonical Change Plan | one canonical Change Plan schema/decoder/port/browser/Tauri adapter; no shadow #41 facade | #55 | Rust/TS/canonical fixture parity and plan digest fixture |
| apply workspace | reducer/hook/workspace consumes canonical #55 snapshots and #35 readiness summaries; no invoke and no parallel wire schema | #41 | revision/job acceptance, listen→query recovery, stable failure rendering |
| composition | `Page.tsx`, platform export, router/DEV mount and architecture allowlist | main integration | V2 architecture test and production route test |

The previous proposed file `src/v2/shared/data/credentials/tauri.ts` violates the existing V2 boundary. The only allowed location is `src/v2/shared/platform/tauri/credentials.ts`.

V2 DTOs, events and fixtures contain references/readiness only. Browser fixtures must generate neutral IDs and status objects; they must not contain realistic `sk-*`, token or password literals.

## 9. Exact file-owner and focused-test matrix

No file below has two writers. A lane may request a change in another lane's contract, but only that file's listed owner edits it.

### 9.1 `#35 module`

| Exact files | Responsibility | Focused tests/evidence |
| --- | --- | --- |
| `src-tauri/src/secret/types.rs`, `src-tauri/src/secret/error.rs`, `src-tauri/src/secret/material.rs` | exact secret types, stable errors, non-serializable/zeroizing material | inline module tests + compile/serde contract tests |
| `src-tauri/src/secret/backend.rs`, `src-tauri/src/secret/platform/mod.rs`, `src-tauri/src/secret/platform/macos.rs`, `src-tauri/src/secret/platform/windows.rs` | #35-owned backend registry/core trait API, five-operation policy, scope-bound authorized read/delete/missing-as-Validate and validated revocation observations; candidate discard exposes separate delete/missing slots and `CandidateDiscardDeleteCheckpoint`; normal/recovery activation retain their named runtime/durable checkpoints; core contains no reference to not-yet-existing #55/#41/main concrete types | independent #35 core compile + focused in-memory/scope/operation-policy tests first; three-field checkpoint substitution/crash/timestamp tests; exact `10+2=12` slot and 10 delete/missing-slot assertions; later native macOS/Windows evidence and composition tests |
| `src-tauri/src/secret/capture/mod.rs`, `src-tauri/src/secret/capture/macos.rs`, `src-tauri/src/secret/capture/windows.rs` | native secure capture only | cancellation/concurrency/unit seam; later platform UAT |
| `src-tauri/src/secret/device_store/mod.rs`, `src-tauri/src/secret/device_store/schema.rs`, `src-tauri/src/secret/device_store/atomic.rs`, `src-tauri/src/secret/device_store/journal.rs`, `src-tauri/src/secret/device_store/reconcile.rs`, `src-tauri/src/secret/operation.rs`, `src-tauri/src/secret/service.rs` | device-local record/candidate/binding/audit authority, four-kind recovery union, durable operation/expiry phases, binding-set CAS, prepare/consume | kind/action totality, candidate pending-vs-terminal expiry, crash-phase, CAS and single-consume tests |
| `src-tauri/src/secret/migration.rs`, `src-tauri/src/secret/redaction.rs`, `src-tauri/src/secret/testing.rs`, `src-tauri/src/secret/mod.rs` | registered source inventory/reconcile/policy-aware scrub, public projection primitives, fake backend | all registered TOML/JSON locations, both comparison policies, conflicts, unknown comparison and canary tests |
| `src-tauri/src/commands/secret.rs` | no-value secret lifecycle/readiness IPC | command DTO/effect-none tests |
| `src/v2/shared/data/credentials/types.ts`, `src/v2/shared/data/credentials/decoder.ts`, `src/v2/shared/data/credentials/port.ts`, `src/v2/shared/data/credentials/browser.ts`, `src/v2/shared/data/credentials/index.ts`, `src/v2/shared/platform/tauri/credentials.ts` | V2 no-value contract and adapters | `tests/v2/shared/data/credentials/types.test.ts`, `decoder.test.ts`, `browser.test.ts` |
| `src/v2/pages/models/credentials/CredentialsPanel.tsx`, `src/v2/pages/models/credentials/credentials.css`, `src/v2/pages/models/credentials/prototype.ts`, `src/v2/pages/models/credentials/index.ts` | credentials UI subsection only | `tests/v2/pages/models/credentials/CredentialsPanel.test.tsx` |
| `tests/v2-browser/credentials.spec.ts` | credentials browser interaction/four-viewport evidence | focused Playwright spec after visual freeze |
| `scripts/tasks/secret-surface-scan.mjs`, `tests/scripts/secret-surface-scan.test.ts`, `.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-baseline.json` | four scanner levels and exact inventory baseline | scanner self-tests with generated canary/negative fixtures |

### 9.2 `#55`

| Exact files | Responsibility | Focused tests/evidence |
| --- | --- | --- |
| `src-tauri/src/change_plan.rs`, `src-tauri/src/change_plan/secret_admission.rs`, `src-tauri/src/commands/change_plan.rs`, `src-tauri/src/database/dao/change_plan.rs` | sanitized plan/digest/job/command contract, comparison-policy and staged-import discriminants, owner-private admission token factory and #41 apply coordinator port | existing #55 Rust tests revised for policy/staged discriminant parity, secret-independent digests and no direct writer |
| `src/components/change-plan/ChangePlanFlow.tsx`, `src/lib/api/change-plan.ts`, `src/lib/query/change-plan.ts` | canonical legacy Change Plan consumer | `tests/components/change-plan.test.tsx`, `tests/lib/change-plan.test.ts`, `tests/integration/change-plan-cross-layer.test.ts` |
| `tests/fixtures/changePlanDtoContract.v1.json` | single canonical shared fixture until explicit schema version bump | Rust/TS byte/schema parity |
| `src/v2/shared/data/change-plan/types.ts`, `src/v2/shared/data/change-plan/decoder.ts`, `src/v2/shared/data/change-plan/port.ts`, `src/v2/shared/data/change-plan/browser.ts`, `src/v2/shared/data/change-plan/index.ts`, `src/v2/shared/platform/tauri/change-plan.ts` | canonical V2 Change Plan adapter | `tests/v2/shared/data/change-plan/*.test.ts` |

Shared index/registration files are deliberately excluded from #55 ownership.

### 9.3 `#41`

| Exact files | Responsibility | Focused tests/evidence |
| --- | --- | --- |
| `src-tauri/src/services/configuration_apply/runtime.rs`, `src-tauri/src/services/configuration_apply/backup.rs`, `src-tauri/src/services/configuration_apply/provider.rs`, `src-tauri/src/services/configuration_apply/mod.rs` | prepare/lease/sanitized backup/write/readback/rollback/recovery coordinator | `src-tauri/tests/configuration_apply.rs` |
| `src/v2/pages/models/apply/reducer.ts`, `src/v2/pages/models/apply/useConfigurationApply.ts`, `src/v2/pages/models/apply/ApplyWorkspace.tsx`, `src/v2/pages/models/apply/fixtures.ts`, `src/v2/pages/models/apply/apply-workspace.css`, `src/v2/pages/models/apply/index.ts` | apply job UI consuming canonical #55 contract | existing #41 V2 apply tests |
| `tests/v2/shared/platform/changePlanSchema.test.ts`, `tests/v2/pages/models/apply/reducer.test.ts`, `tests/v2/pages/models/apply/useConfigurationApply.test.tsx`, `tests/v2/pages/models/apply/ApplyWorkspace.test.tsx`, `tests/v2-browser/configuration-apply.spec.ts`, `tests/changePlanDtoContract.test.ts` | #41 state, schema and browser evidence | exact named suites |

#41 does not edit #55's canonical DTO/DAO/commands. It submits contract requirements; #55 publishes the compatible source.

Composition order is a gate, not a suggested merge strategy: #35 core trait/API and focused core tests first; #55 adapter types under §9.2 next; #41 adapter types under §9.3 next; main-owned Provider/proxy/import/startup adapter types under §9.4 next; only then may the single `main integration` composition owner connect published seams and run the full Rust gate. `src-tauri/src/secret/backend.rs` cannot predeclare, seal or otherwise reference a concrete external adapter type before that type exists in its canonical owner.

### 9.4 `main integration`

| Exact files | Responsibility | Focused tests/evidence |
| --- | --- | --- |
| `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` | dependency adjudication across active native lanes | locked dependency check/native platform jobs |
| `src-tauri/src/database/schema.rs`, `src-tauri/src/database/mod.rs`, `src-tauri/src/database/dao/mod.rs`, `src-tauri/src/database/tests.rs`, `src-tauri/src/database/backup.rs` | preserve the separately adjudicated Prompt/Memory v17 lane; implement `open_preflight_without_backup`/sanitized backup; expose only required narrow DAO tokens; prove #35 adds no secret schema/table/version | existing schema migration + too-new gate; raw-backup spy=0 before no-value readback gate; static absence of secret tables |
| `src-tauri/src/store.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/services/mod.rs`, `src-tauri/tests/support.rs`, `src-tauri/tests/deeplink_import.rs` | enforce `open store → no-backup DB preflight → same AppState/SecretService → reconcile → app.manage/static registration → Clean sanitized backup → publish gate → workers`; register exactly 15 #35 handlers plus the separate main-integration `resume_staged_import_cutover`; Blocked publishes/starts nothing | opened-handle object identity/no-reopen/no-path-injection; ordered step recorder; `lib.rs` static 15+1 set/count assertion; resume not in `SecretCommandName`; staged phase-crash UAT; Blocked backup/publish/worker/consumer counts all zero |
| `src-tauri/src/provider.rs`, `src-tauri/src/database/dao/providers.rs`, `src-tauri/src/commands/provider.rs`, `src-tauri/src/services/provider/mod.rs`, `src-tauri/src/services/provider/live.rs`, `src-tauri/src/services/provider/usage.rs`, `src-tauri/src/services/config.rs`, `src-tauri/tests/provider_service.rs`, `src-tauri/tests/import_export_sync.rs`, `src-tauri/tests/provider_commands.rs`, `src-tauri/tests/mcp_commands.rs` | internal/public Provider split, CRUD, Provider-domain delete stale envelope/action, live/backfill/switch and usage seams; remove ConfigService plaintext Codex sync/backfill and rewrite existing command/value fixtures | `src-tauri/tests/secret_provider_integration.rs`; named base tests use runtime-generated canaries to prove token-free Provider storage, no direct sync, no Provider/secret error-union confusion and exact sink behavior |
| `src-tauri/src/codex_history_migration.rs`, `src-tauri/src/lib.rs` and their inline tests | forbid pre-gate raw `settingsConfig` history backup; after Clean write structural placeholders/non-secret config only; retain existing generations as scan/report-only | zero pre-gate backup spy; generated-canary absent from new generation; historical bytes/hash unchanged with typed report count/category |
| `src-tauri/src/codex_config.rs` | registered token-free config preparation, policy-aware source scrub and exact live writer integration | all-location parser/scrubber/readback tests for `candidateEquality` and `explicitReplacement` |
| `src-tauri/src/usage_script.rs`, `src-tauri/src/services/balance.rs`, `src-tauri/src/services/coding_plan.rs`, `src-tauri/src/services/model_fetch.rs`, `src-tauri/src/commands/balance.rs`, `src-tauri/src/commands/coding_plan.rs`, `src-tauri/src/commands/model_fetch.rs` | fixed consumers, including Provider-primary coding-plan as `consumer=FixedRuntimeConsumer::CodingPlanUsageProbe` with closed `CodingPlanPrimaryAdapter` under `usageProbe`; generic JS/value IPC rejection; stable upstream errors | `src-tauri/tests/secret_consumer_integration.rs`: owner/ref request, one resolve, redirect-none, no-network on readiness failure and purpose-confusion cases |
| `src-tauri/src/proxy/mod.rs`, `src-tauri/src/proxy/providers/mod.rs`, `src-tauri/src/proxy/providers/adapter.rs`, `src-tauri/src/proxy/providers/auth.rs`, `src-tauri/src/proxy/providers/codex.rs`, `src-tauri/src/proxy/forwarder.rs`, `src-tauri/src/proxy/provider_router.rs`, `src-tauri/src/proxy/failover_switch.rs` | narrow runtime-token re-export, non-sensitive auth strategy, per-attempt resolve and failover semantics | `src-tauri/tests/secret_proxy_integration.rs` |
| `src-tauri/src/services/proxy.rs`, `src-tauri/src/database/dao/proxy.rs` | takeover and structural live-backup integration | `src-tauri/tests/secret_takeover_integration.rs` |
| `src-tauri/src/commands/misc.rs` | Codex terminal rejection | terminal no-side-effect test |
| `src-tauri/src/commands/failover.rs`, `src-tauri/src/database/dao/failover.rs` | safe failover DTO/public list | failover public contract test |
| `src-tauri/src/database/dao/universal_providers.rs`, `src-tauri/src/commands/provider.rs`, `src-tauri/src/services/provider/mod.rs` | UniversalProvider public/mutation/Codex conversion gate | universal Codex negative integration test |
| `src-tauri/src/deeplink/mod.rs`, `src-tauri/src/deeplink/parser.rs`, `src-tauri/src/deeplink/provider.rs`, `src-tauri/src/deeplink/tests.rs`, `src-tauri/src/commands/deeplink.rs`, `src/utils/deepLinkConfigPreview.ts`, `src/components/DeepLinkImportDialog.tsx`, `tests/utils/deepLinkConfigPreview.test.ts`, `tests/components/DeepLinkImportDialog.test.tsx` | reject raw/encoded/remote-config Codex secret shapes natively before parse/merge/event; renderer has no secret decode/mask/preview path | runtime-generated raw/encoded/nested/remote canary table; no echo, event, preview, Provider build, DB or live write |
| `src-tauri/src/commands/import_export.rs`, `src-tauri/src/commands/sync_support.rs`, `src-tauri/src/commands/webdav_sync.rs`, `src-tauri/src/commands/s3_sync.rs`, `src-tauri/src/database/backup.rs`, `src-tauri/src/services/sync_protocol.rs`, `src-tauri/src/services/webdav_sync.rs`, `src-tauri/src/services/webdav_sync/archive.rs`, `src-tauri/src/services/s3_sync.rs` | temp authority/token+projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact `ImportCutoverCoordinatorContext` → staged source validation/scrub/readback → admitted-CAS cutover → live owner/binding finalize; exact `{stageId,expectedResumeCas}` resume whose digest preimage includes journal operation id and cumulative-field `StagedImportResumePhase`; ordinary activation stays current-only | `src-tauri/tests/secret_import_sync_restore.rs` plus exact five named `staged_resume_*_v1` digest fixtures, required/forbidden phase-field scan, new CAS on every nonce/admission/phase/checkpoint transition, cancellation/expiry/replay/old nonce-admission-CAS, staged-token forgery, context-before-validation/cutover ordering, Skills/main DB effect-none, and every pre/post-cutover crash phase |
| `src-tauri/src/panic_hook.rs`, `src-tauri/src/services/usage_cache.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/proxy/usage/logger.rs`, `src/lib/frontendLogger.ts` | stable/log-safe/cache-safe diagnostic sinks | runtime artifact canary integration test |
| `src/config/codexProviderPresets.ts`, `src/components/providers/forms/ProviderForm.tsx`, `src/components/providers/forms/hooks/useApiKeyState.ts`, `src/components/providers/forms/hooks/useCodexConfigState.ts`, `src/utils/providerConfigUtils.ts`, `tests/utils/providerConfigUtils.codex.test.ts`, `src/components/universal/UniversalProviderFormModal.tsx`, `src/components/universal/UniversalProviderPanel.tsx` | remove Codex value factories/state/TOML value helpers and route native capture/readiness; block UniversalProvider Codex copies; rewrite existing token helper fixtures | focused renderer tests; the named utility test becomes registered discovery/structural-helper generated-canary coverage with no material-returning API |
| `src/components/providers/AddProviderDialog.tsx`, `src/components/providers/EditProviderDialog.tsx`, `src/hooks/useCodexProviderFeatures.ts`, `src/components/providers/forms/CodexFormFields.tsx`, `src/components/providers/forms/CodexConfigSections.tsx`, `src/components/providers/forms/CodexConfigEditor.tsx`, `src/components/providers/ProviderCard.tsx`, `src/lib/api/usage.ts`, `src/config/codexTemplates.ts`, `tests/hooks/useCodexProviderFeatures.test.tsx`, `tests/hooks/useCodexConfigState.catalog.test.ts`, `tests/hooks/useAddProviderMutation.test.tsx`, `tests/components/AddProviderDialog.test.tsx`, `tests/components/EditProviderDialog.test.tsx`, `tests/components/CodexFormFields.capabilities.test.tsx`, `tests/config/codexTemplates.test.ts` | token-free feature draft/public/mutation DTOs; owner/ref usage/model-fetch requests; public readiness-only card/form state; remove empty `OPENAI_API_KEY` substitute | runtime-generated-canary negative fixtures; full Provider/secret field/empty substitute rejected or absent; fixed request has owner only and no network on blocked readiness |
| `tests/config/therouterProviderPresets.test.ts`, `tests/config/subrouterProviderPresets.test.ts`, `tests/config/xaiOauthProviderPresets.test.ts` | convert checked preset/template expectations to token-free structural assertions; managed OAuth stays separate from `codexApiKey` | runtime-generated canary absent; no empty/redacted secret substitute; structure/capability expectations preserved |
| `src/components/UsageScriptModal.tsx`, `src/lib/api/subscription.ts`, `src-tauri/src/commands/coding_plan.rs` | replace Provider-primary coding-plan value chain with owner/ref no-value request; preserve only typed independent ZenMux/Volc/team branches as adjacent debt | focused modal/API/IPC tests prove fixed adapters send owner only, independent credentials never enter primary branch, and no renderer/API fixture receives Provider material |
| `src/App.tsx`, `src/hooks/useProviderActions.ts`, `src/lib/api/providers.ts`, `src/lib/query/mutations.ts`, `src/i18n/locales/en.json`, `src/i18n/locales/ja.json`, `src/i18n/locales/zh.json`, `src/i18n/locales/zh-TW.json`, `tests/integration/App.test.tsx`, `tests/hooks/useProviderActions.test.tsx`, `tests/lib/providersApi.codexFeatures.test.ts`, `tests/msw/handlers.ts` | bound/unbound no-value Provider impact → retention/orphan preview → exact confirm → durable detach; legacy blocked/no impact id; stale uses Provider-only `PROVIDER_DELETE_IMPACT_STALE + refreshProviderDeleteImpact`, never secret `refreshDeleteImpact` | all-locale impact facts; legacy/no-id; cancel/stale/replay effect-none; Provider/secret error-action decoder separation; no secret-delete call |
| `src/lib/api/model-fetch.ts`, `src/lib/api/failover.ts`, `src/lib/api/deeplink.ts`, `src/lib/api/vscode.ts`, `src/hooks/useProviderActions.ts`, `src/lib/api/index.ts`, `src/lib/query/index.ts` | remaining public API integration and #55 adapter composition | Vitest/MSW/cross-layer tests |
| `src/v2/pages/models/Page.tsx`, `src/v2/shared/platform/index.ts`, `src/v2/app/router.tsx`, `tests/v2/app/architecture.test.ts` | compose credentials/plan/apply lanes and preserve V2 invoke boundary | V2 architecture + page composition tests |
| `package.json`, `.mise/tasks/contracts.toml`, `docs/fyagent/development/mise-tasks.md`, `.github/workflows/ci.yml` | register scanners/gates/native matrix once | task validation/docs check/CI readback |

#### SNV7 supplemental exact-path / owner / generator floor

These are **127 path/category entries, 111 unique exact paths, across six categories**. Every row has the sole canonical writer `main integration`; repeated files across behavioral categories do not create another writer. The exact-path generator emits the category ID, exact path, canonical owner and focused-evidence ID for every entry, compares sorted output to this register, and fails on omission/addition/move. This is a future source-freeze floor, not current implementation or runtime evidence.

| ID | Exact existing paths | Canonical owner | Frozen decision and focused negative evidence |
| --- | --- | --- | --- |
| `SNV7-001` (16) | `src-tauri/src/services/env_checker.rs`; `src-tauri/src/commands/env.rs`; `src-tauri/src/services/env_manager.rs`; `src-tauri/src/services/mod.rs`; `src-tauri/src/commands/mod.rs`; `src-tauri/src/lib.rs`; `src/lib/api/env.ts`; `src/types/env.ts`; `src/components/env/EnvWarningBanner.tsx`; `src/App.tsx`; `tests/codexWindowsUserScopeContract.test.ts`; `tests/integration/App.test.tsx`; `src/i18n/locales/en.json`; `src/i18n/locales/ja.json`; `src/i18n/locales/zh.json`; `src/i18n/locales/zh-TW.json` | `main integration` | Codex `OPENAI_*` exposes only presence/name/stable source category. The bridge records `processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile` only as four fixed supplemental domain identities with structural revision/presence/count; they never become `LegacySourceRef`. Receipt/view/IPC/UI has no value/digest/raw locator/absolute path. Startup/summary/capture/Provider-delete each fresh-revalidate through `CodexLegacySourceInventoryBridge`; Codex delete/restore creates no plaintext backup. Generated-canary, fixed-11-domain and empty-without-complete-proof effect-none tests |
| `SNV7-002` (21) | `src-tauri/src/app_config.rs`; `src-tauri/src/database/migration.rs`; `src-tauri/src/database/dao/settings.rs`; `src-tauri/src/commands/config.rs`; `src-tauri/src/commands/provider.rs`; `src-tauri/src/services/provider/mod.rs`; `src-tauri/src/services/provider/live.rs`; `src-tauri/src/services/proxy.rs`; `src-tauri/src/lib.rs`; `src/lib/api/config.ts`; `src/components/providers/forms/hooks/useCodexCommonConfig.ts`; `src/components/providers/forms/CodexCommonConfigModal.tsx`; `src/components/providers/forms/CodexConfigEditor.tsx`; `src/components/providers/forms/ProviderForm.tsx`; `src-tauri/tests/app_config_load.rs`; `src-tauri/tests/provider_service.rs`; `src-tauri/src/database/tests.rs`; `src-tauri/tests/import_export_sync.rs`; `tests/hooks/useCommonConfigSave.test.tsx`; `tests/components/CommonConfigModalBehavior.test.tsx`; `tests/components/CommonConfigEditor.test.tsx` | `main integration` | New Codex secret-bearing TOML rejects before DB/localStorage/live writes. The bridge records `commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge` only as six fixed supplemental domain identities with structural revision/presence/count; they never become `LegacySourceRef`. Public `LegacySourceCoverageView` is no-authority and has no raw snippet/path. Startup/summary/capture/Provider-delete each fresh-revalidate through `CodexLegacySourceInventoryBridge`; generated-canary, fixed-11-domain and absent/stale/empty-without-proof cases remain zero-write |
| `SNV7-003` (20) | `src/types.ts`; `src/lib/schemas/provider.ts`; `src/lib/query/queries.ts`; `src/lib/query/mutations.ts`; `src/lib/api/providers.ts`; `src/components/providers/ProviderList.tsx`; `src/hooks/useDragSort.ts`; `src/components/providers/forms/CodexFormFields.tsx`; `src/components/providers/forms/shared/ApiKeySection.tsx`; `src/components/providers/forms/ApiKeyInput.tsx`; `tests/msw/state.ts`; `tests/msw/handlers.ts`; `tests/components/ProviderList.test.tsx`; `tests/hooks/useDragSort.test.tsx`; `tests/hooks/useUpdateProviderMutation.test.tsx`; `tests/hooks/useAddProviderMutation.test.tsx`; `tests/components/CodexFormFields.capabilities.test.tsx`; `tests/components/AddProviderDialog.test.tsx`; `tests/components/EditProviderDialog.test.tsx`; `tests/lib/providersApi.codexFeatures.test.ts` | `main integration` | Codex internal/public/mutation types are distinct; public/query/list/sort/MSW objects have no `settingsConfig`; mutation rejects it before write. Codex never renders shared `ApiKeySection`/`ApiKeyInput`. List/update/sort/add/edit tests inject a generated canary and prove schema/DOM/cache/network payload zero matches plus closed unknown-field rejection |
| `SNV7-004` (13) | `src/lib/requestOverrides.ts`; `src/components/providers/forms/ProviderForm.tsx`; `src/components/providers/forms/LocalProxyRequestOverridesField.tsx`; `src/components/providers/forms/CodexFormFields.tsx`; `src/components/providers/forms/ClaudeFormFields.tsx`; `src/components/providers/forms/GrokBuildProviderForm.tsx`; `src-tauri/src/provider.rs`; `src-tauri/src/proxy/forwarder.rs`; `src-tauri/src/proxy/hyper_client.rs`; `src-tauri/src/proxy/providers/codex.rs`; `src-tauri/src/services/proxy.rs`; `tests/lib/requestOverrides.test.ts`; `src-tauri/tests/provider_service.rs` | `main integration` | New Codex arbitrary secret/header/body override mutation rejects before persistence; an existing Codex row containing one fails closed before proxy/network. Primary material crosses only an owner-private single-send zeroizing transport and never shared raw `Vec`/header maps. Reflection-canary tests inspect request/result/log/error; retained non-Codex overrides are explicitly Level 3 debt |
| `SNV7-005` (22) | `src-tauri/src/commands/stream_check.rs`; `src-tauri/src/services/stream_check.rs`; `src-tauri/src/database/dao/stream_check.rs`; `src-tauri/src/database/schema.rs`; `src/lib/api/connectivity-check.ts`; `src/hooks/useStreamCheck.ts`; `src/components/providers/ProviderList.tsx`; `tests/components/ProviderList.test.tsx`; `src-tauri/src/proxy/error.rs`; `src-tauri/src/proxy/types.rs`; `src-tauri/src/commands/proxy.rs`; `src-tauri/src/services/proxy.rs`; `src-tauri/src/database/dao/proxy.rs`; `src/types/proxy.ts`; `src/lib/api/proxy.ts`; `src/lib/query/proxy.ts`; `src/components/proxy/ProxyPanel.tsx`; `tests/hooks/useProxyStatus.test.tsx`; `src-tauri/src/database/dao/failover.rs`; `src-tauri/src/commands/failover.rs`; `src/lib/api/failover.ts`; `src/lib/query/failover.ts` | `main integration` | Codex performs no active secret-bearing stream check. Proxy/failover/health diagnostics map before DB/UI to closed status/category/latency only; no raw URL/upstream error/body/message. Network and DB spies remain zero on blocked Codex checks; generated reflection canary has zero matches in DTO, DB, toast, query cache, log and HTTP error body |
| `SNV7-006` (35) | `src-tauri/src/app_config.rs`; `src-tauri/src/services/mcp.rs`; `src-tauri/src/database/dao/mcp.rs`; `src-tauri/src/database/schema.rs`; `src-tauri/src/database/migration.rs`; `src-tauri/src/commands/mcp.rs`; `src-tauri/src/mcp/mod.rs`; `src-tauri/src/mcp/codex.rs`; `src-tauri/src/codex_config.rs`; `src-tauri/src/lib.rs`; `src/types.ts`; `src/lib/api/mcp.ts`; `src/hooks/useMcp.ts`; `src/components/mcp/McpWizardModal.tsx`; `src/components/mcp/UnifiedMcpPanel.tsx`; `src/components/mcp/McpFormModal.tsx`; `src/components/mcp/useMcpValidation.ts`; `src-tauri/src/database/backup.rs`; `src-tauri/src/commands/import_export.rs`; `src-tauri/src/services/sync_protocol.rs`; `src-tauri/src/services/webdav.rs`; `src-tauri/src/services/s3.rs`; `src-tauri/src/services/webdav_sync.rs`; `src-tauri/src/services/webdav_sync/archive.rs`; `src-tauri/src/services/s3_sync.rs`; `src-tauri/src/commands/webdav_sync.rs`; `src-tauri/src/commands/s3_sync.rs`; `src-tauri/tests/mcp_commands.rs`; `src-tauri/tests/import_export_sync.rs`; `tests/components/UnifiedMcpPanel.test.tsx`; `tests/components/McpFormModal.test.tsx`; `tests/hooks/useMcpBulkToggle.test.tsx`; `tests/hooks/useMcpValidation.test.tsx`; `tests/msw/state.ts`; `tests/msw/handlers.ts` | `main integration` | Classify Codex MCP `env` and `http_headers` as `codexMcpEnvOrHeaderCredential` Level 3 adjacent debt across raw unified server JSON, SQLite `server_config`, commands/UI, live `~/.codex/config.toml`, legacy `config.json`, DB backup/export, WebDAV/S3 sync/import and fixtures. Replace static `Bearer top-secret`/similar literals with runtime-generated canaries; preserve exact occurrence count/category and fail every new/moved occurrence. This row does not count toward Provider-primary Level 2 PASS |

`main integration` owns only adapter/composition after the other owners publish compilable types. Its ordered receipt is `#35 core compiled/focused → #55 adapter SHA → #41 adapter SHA → main-owned adapter SHA → composition SHA → full Rust gate`. It may not move an adapter type into `backend.rs`, add a provisional external-type reference there, or use composition to conceal an owner whose focused gate has not passed. Within that canonical writer, `CodexLegacySourceInventoryBridge` is the single inventory bridge and the only holder of `CompleteLegacySourceInventoryAuthority`; store, Provider and #35 siblings may name/move/consume `LegacySourceCoverageReceipt` but cannot mint, inspect or maintain a parallel inventory.

Where a file appears in several responsibility rows (for example `commands/provider.rs`), its sole owner is still `main integration`; rows partition behavior, not writers.

At base SHA `afc317a7`, the mechanical `AppState::new(` inventory is exactly 32 callsites across eight files, all already owned above: `commands/import_export.rs`, `commands/sync_support.rs`, `deeplink/tests.rs`, `lib.rs`, `services/provider/mod.rs`, `services/proxy.rs`, `tests/deeplink_import.rs`, and `tests/support.rs`. Main integration migrates all 32 to production construction or the feature-gated opaque test builder; source freeze requires the old-call count to be zero and rejects any unlisted callsite. There is no current external AppState struct literal; the new private `secret_service`/construction-seal field preserves that invariant while existing field visibility remains unchanged.

## 10. Scanner levels and truthful acceptance claims

### Level 1 — `contract_schema` (strict PASS required)

Scan Rust/TS public DTOs, commands, events, canonical fixtures and browser fixtures for #35/#55/#41/PublicProvider/feature-draft/mutation/failover/universal/V2. Provider-delete impact/confirm and its Provider-domain stale envelope/action, four-kind recovery, typed `SecretCaptureIntentId` list→begin flow, staged-import token/projection/admission/authority-match/prepare-confirm/construct-exact-`ImportCutoverCoordinatorContext`/validation-scrub-readback/cutover/live-finalize/exact-CAS-resume, and fixed owner/ref usage/model-fetch schemas are mandatory. ARR-001..003 add exact candidate/activation/recovery three-field checkpoints, cumulative `StagedImportResumePhase`, five named digest fixtures, every-transition CAS and `10+2=12`/10-delete-missing slot assertions to this level; none changes five operations, 8 journals or 4 recovery kinds. `SNV7-001..005` also require closed Codex env/common-config/public-Provider/override/diagnostic schemas. Reject material fields, full Provider/raw `Value` on Codex public routes, empty/redacted secret substitutes, raw paths/URLs/errors/bodies, arbitrary error/detail fields, secret-bearing command args and value-derived digest inputs. Provider error/action literals cannot decode as `SecretErrorCode`/`SecretUserAction`, and generic legacy actions cannot bypass the canonical destination map.

### Level 2 — `codex_feature_runtime` (strict PASS required)

Use a run-generated unique canary and scan the complete enumerated Codex feature artifact set:

- renderer IPC requests/responses, events, state snapshots and screenshots when runtime evidence is authorized;
- Provider/universal/change-plan/job/audit/journal/proxy-backup/proxy-log/usage-cache DB rows, including WAL/SHM;
- the exact device-local secret root recursively, including `store.lock`, `state.json`, `journal/**`, `audit/**`, recognized durable-replace temp files and validated Windows `.retired-*` journal tombstones;
- exports/import staging/sync payload/local backup/recovery bundles;
- app/plugin/proxy logs and `crash.log`;
- temp/config/script files and child-process captures;
- test/browser fixtures and reports.

Token-free feature drafts/forms/cards, fixed owner/ref usage/model-fetch/coding-plan requests/results/headers/cache/logs, native-rejected deep-link ingress/event/preview state, staged sync temp/cutover/resume artifacts, new history generations, Provider-delete preview/confirm state, and `SNV7-001..005` env/common-config/public-Provider/override/diagnostic artifacts are part of this set. Codex MCP `env`/`http_headers` (`codexMcpEnvOrHeaderCredential`), ZenMux hand-entered key, Volcengine AK/SK and independent team/login material remain Level 3 debt, but cannot appear in the Provider-primary adapter branch or be counted toward Level 2 PASS.

The runtime-artifact manifest records each required root and enumeration result. An unexpectedly absent or unreadable device-local root/file fails closed; the scanner may not silently omit it. Each reviewed live-file exception is mapped internally from its closed `CodexLiveSecretSinkId` and never published as an absolute path.

The allowed sink is explicit, not a scanner waiver. During an approved OS-keyring apply, the exact reviewed Codex live file may contain the canary and the authoritative OS entry contains it. The harness must assert exactly that named sink, then cleanup and assert zero residual occurrences. Hardware with no persistent projection expects zero file/env occurrences and an effect-none rejection.

### Level 3 — `repository_static_inventory` (baseline/report + no-regression)

Record exact AST/path/category occurrences for adjacent credential domains. `codexMcpEnvOrHeaderCredential` is a mandatory named category covering unified MCP JSON, SQLite, IPC/UI, Codex live TOML, export/backup/WebDAV/S3 sync and fixtures. New occurrences or moved/unclassified occurrences fail. Existing debt remains nonzero and visible; it is not converted into broad regex/path allowlists or a Provider-primary Level 2 exemption.

### Level 4 — `repository_runtime_global` (`NOT_CLAIMED`)

This level cannot pass until all adjacent credential domains migrate. Neither Level 1 nor Level 2 may be described as “FyAgent/repository globally secret-free.”

### Stale/legacy exclusion policy

- Adjacent debt excluded only from the **Codex Provider-primary feature PASS**: `codexMcpEnvOrHeaderCredential`, WebDAV password, S3 secret access key, non-Codex Provider credentials, managed OAuth managers, non-Codex UniversalProvider conversion, ZenMux's separately hand-entered key/base URL, Volcengine account AK/SK and independent team/login credentials.
- Those paths remain included in repository inventory/no-regression and prevent a global claim.
- No Provider-primary Codex bypass is excludable: inactive/inline TOML, renderer presets/form state/token helpers, full feature drafts/forms/cards, empty-key templates, raw Provider commands, usage/balance/Provider-primary coding-plan/model fetch, UniversalProvider Codex conversion, Provider-delete preview/confirm, deep-link native ingress and renderer preview, failover, proxy, startup history/import/backfill, staged protocol/cutover/resume, Codex env/common-config, arbitrary request override and diagnostic/error paths are all in Level 2. MCP debt remains exact Level 3 inventory/no-regression, not an exclusion from inventory.
- Checked-in `sk-*`/token fixtures in Codex paths are replaced with generated canaries; they are not blanket allowlisted.
- `.git`, `node_modules`, `target` and generated build outputs may be excluded from the **source** scan, but runtime artifact roots are scanned explicitly.
- “Legacy,” “inactive,” “test-only,” “empty,” “redacted/masked UI,” “backend-only” and “hashed” are not secret-safety exclusions.

## 11. Source-freeze files and registered commands

### 11.1 Files that must exist at source freeze

1. `.trellis/tasks/08-14-issue-35-secret-backend/research/design-freeze-receipt.md` — exact #35 contract SHA, P0/P1/P2=0 readback.
2. `.trellis/tasks/08-14-issue-35-secret-backend/research/source-freeze-manifest.json` — base/freeze SHA, #55 compatible source/final SHA, #41 implementation handoff SHA, no-v17 confirmation, sorted changed paths/owner/blob SHA, phased core→owner-adapter→main-composition receipts, opaque-`pub(crate)` `LegacySourceCoverageReceipt` with private fields and `pub(crate)` factory, sole `CodexLegacySourceInventoryBridge` constructor for unforgeable `CompleteLegacySourceInventoryAuthority`, non-value-derived `LegacySourceInventoryRevision`, exact fixed-11-domain `CompleteLegacySourceCoverageIdentity`, atomic current-expectation/adjacent-observation binding, internal non-value-derived `LegacySourceLocationId` allowance, raw-path/raw-locator/value/value-derived-digest bans, per-consumer fresh-revalidation assertions, exact named ARR-001/002 runtime/durable three-field checkpoints and fresh-`Validate` missing receipts, exact `10+2=12`/10-delete-missing slot assertions, ARR-003 journal operation id plus cumulative-field `StagedImportResumePhase`, five named `staged_resume_*_v1` fixtures and every-transition CAS changes, exact 15+1 registration sets, and native macOS/native Windows Rust 1.85.0 locked `--all-targets` host/lock/result records.
3. `.trellis/tasks/08-14-issue-35-secret-backend/research/secret-surface-baseline.json` — scanner version, four levels, exact source/runtime roots, the generator's sorted path/category entries, adjacent-debt AST entries and zero broad Codex waivers.
4. `scripts/tasks/secret-surface-scan.mjs` and `tests/scripts/secret-surface-scan.test.ts`.
5. Source-gate registrations (not part of the eight evidence-host tasks):
   - `secret:scan:contract`
   - `secret:scan:inventory`
6. The canonical eight evidence tasks, identical to `execution-plan.md` and `device-local-secret-store.md`:
   - `secret:native:macos:crud`
   - `secret:native:macos:uat`
   - `secret:native:windows:crud`
   - `secret:native:windows:failure`
   - `secret:native:windows:uat`
   - `secret:scan:codex -- <runtime-artifact-manifest>`
   - `secret:artifact:scan` (evidence-host wrapper around `secret:scan:codex`)
   - `secret:evidence:verify`
7. Generated `docs/fyagent/development/mise-tasks.md` readback.
8. `.github/workflows/ci.yml` entries for strict contract/inventory gates, an independent #35-core compile/focused-test lane, post-owner main-composition full Rust gate, `src-tauri/src/lib.rs` static 15+1 registration assertion, staged resume phase-crash UAT, and reviewed native macOS/Windows evidence lanes. A host-only unit test or current Rust 1.97.1 result is not the required native-store/MSRV evidence.

The source-inventory generator has a checked exact-path fixture and compares its sorted output to the §9.4 owner/evidence register. The V6/V7 floor includes every path below:

- live/value/helper base: `src-tauri/src/services/config.rs`, `src/config/codexProviderPresets.ts`, `src/components/providers/forms/hooks/useCodexConfigState.ts`, `src/utils/providerConfigUtils.ts`, `src-tauri/tests/provider_service.rs`, `src-tauri/tests/import_export_sync.rs`, `tests/utils/providerConfigUtils.codex.test.ts`;
- feature draft/public/mutation and fixed requests: `src/components/providers/AddProviderDialog.tsx`, `src/components/providers/EditProviderDialog.tsx`, `src/hooks/useCodexProviderFeatures.ts`, `src/components/providers/forms/CodexFormFields.tsx`, `src/components/providers/forms/CodexConfigSections.tsx`, `src/components/providers/forms/CodexConfigEditor.tsx`, `src/components/providers/ProviderCard.tsx`, `src/lib/api/usage.ts`, `src/config/codexTemplates.ts`;
- coding-plan primary chain: `src/components/UsageScriptModal.tsx`, `src/lib/api/subscription.ts`, `src-tauri/src/commands/coding_plan.rs`, `src-tauri/src/services/coding_plan.rs`;
- native-rejected deep link: `src/utils/deepLinkConfigPreview.ts`, `src/components/DeepLinkImportDialog.tsx`, `tests/utils/deepLinkConfigPreview.test.ts`, `tests/components/DeepLinkImportDialog.test.tsx`;
- staged sync/history startup: `src-tauri/src/services/sync_protocol.rs`, `src-tauri/src/services/webdav_sync.rs`, `src-tauri/src/services/webdav_sync/archive.rs`, `src-tauri/src/services/s3_sync.rs`, `src-tauri/src/codex_history_migration.rs`, `src-tauri/src/lib.rs`;
- Provider delete: `src/App.tsx`, `src/hooks/useProviderActions.ts`, `src/lib/api/providers.ts`, `src/lib/query/mutations.ts`, `src/i18n/locales/en.json`, `src/i18n/locales/ja.json`, `src/i18n/locales/zh.json`, `src/i18n/locales/zh-TW.json`, `tests/integration/App.test.tsx`, `tests/hooks/useProviderActions.test.tsx`, `tests/lib/providersApi.codexFeatures.test.ts`, `tests/msw/handlers.ts`;
- expanded checked fixtures: `src-tauri/tests/provider_commands.rs`, `src-tauri/tests/mcp_commands.rs`, `tests/hooks/useCodexProviderFeatures.test.tsx`, `tests/hooks/useCodexConfigState.catalog.test.ts`, `tests/hooks/useAddProviderMutation.test.tsx`, `tests/components/AddProviderDialog.test.tsx`, `tests/components/EditProviderDialog.test.tsx`, `tests/components/CodexFormFields.capabilities.test.tsx`, `tests/config/codexTemplates.test.ts`, `tests/config/therouterProviderPresets.test.ts`, `tests/config/subrouterProviderPresets.test.ts`, `tests/config/xaiOauthProviderPresets.test.ts`.
- `SNV7-001..006`: all 127 exact path/category entries in §9.4's supplemental register, including env process/HKCU/HKLM/shell/backup handling, common-config legacy artifacts and localStorage/live merge, the complete public Provider list/update/sort chain, request-override transport, stream/proxy/failover diagnostics, and MCP DB/live/export/sync/fixture debt.

A missing path, a path present only in prose, an unclassified moved occurrence, or a generated inventory entry without exactly one of the four owner literals and focused evidence fails `secret:scan:inventory`.

The #41 SHA field cannot be populated from its current untracked design directory. This does not block #35 design freeze; it blocks the later integrated source freeze until an immutable compatible implementation handoff exists.

### 11.2 Readback/freeze commands

These commands are the required future closure sequence; they were **not** executed as tests/builds by this static worker:

```bash
rtk git rev-parse HEAD
rtk git merge-base afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab HEAD
rtk git diff --name-status afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab...HEAD
rtk git diff --check afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab...HEAD
rtk git merge-base --is-ancestor <compatible-55-source-sha> <compatible-55-final-sha>
rtk git diff --exit-code <compatible-55-source-sha>..<compatible-55-final-sha> -- src-tauri src tests package.json .mise .github
rtk mise run env:check
rtk mise run system:check
rtk mise run tasks:validate
rtk mise run secret:scan:contract
rtk mise run secret:scan:inventory
rustup run 1.85.0 cargo check --locked --all-targets --manifest-path src-tauri/Cargo.toml  # native macOS host
rustup run 1.85.0 cargo check --locked --all-targets --manifest-path src-tauri/Cargo.toml  # native Windows host
rtk mise run rust:fmt:check
rtk mise run rust:check
rtk mise run rust:clippy
rtk mise run rust:test -- secret_
rtk mise run test:unit -- secret
rtk mise run lint:v2
rtk mise run typecheck:v2
rtk mise run test:v2
rtk mise run build:renderer
rtk mise run check
```

`secret:scan:codex` is the low-level runtime scanner with required artifact roots/exact allowed-sink manifest. `secret:artifact:scan` is the native-evidence wrapper that verifies host/SHA, enumerates roots, then invokes `secret:scan:codex`; both are registered, and neither is replaced by `mise run check`.

The two Rust 1.85.0 commands above are distinct native-host manifest records, not one cross-compile result repeated twice. Each records OS/architecture, `rustc -Vv`, exact `Cargo.lock` SHA, `--locked --all-targets`, exit result and immutable source SHA. They are future closure commands and were not run by this static writer; Rust 1.97.1 cannot satisfy either record.

### 11.3 Manifest enforcement

Source freeze fails if:

- any changed path lacks exactly one owner from the four-label set;
- a worker changed a shared main-integration file;
- #35 core cannot compile/focus-test before external adapters exist, `backend.rs` references an absent #55/#41/main type, an adapter type lands outside its canonical owner, or full Rust runs before the sole main-integration composition step;
- #55/#41 dependency SHA or blob readback differs;
- a Codex source/consumer/sink row in this document lacks a focused test/evidence mapping;
- the generated exact inventory does not equal the sorted registered path/category baseline, including every V6/V7 feature draft/form/card/API/template, fixed-consumer IPC, deep-link native/render boundary, sync protocol caller, startup history backup, delete UI, checked fixture and all 127 `SNV7-001..006` entries named in §9.4;
- startup ordering differs from the eight-step sequence, a Blocked path creates a backup/publishes a consumer/starts a worker, or a new history generation contains raw `settingsConfig`;
- staged order differs from `temp authority/token+projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact ImportCutoverCoordinatorContext → staged source validation/scrub/readback → cutover → live owner/binding finalize`, the exact context is absent/not sole authority/constructed after any validation-scrub-readback-cutover step, Skills/main DB cutover is reachable earlier, resume accepts anything beyond `{stageId,expectedResumeCas:{revision,digest}}`, digest preimage omits journal `operationId`, `StagedImportResumePhase` differs from exact `intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized`, any arm violates its cumulative required/forbidden CAS/receipt/promoted-owner fields, any named `staged_resume_*_v1` fixture is absent, any fresh nonce/admission or phase/checkpoint transition reuses CAS, or stale CAS performs a write;
- initial staged activation and resume share a result type; request data contains anything beyond `{stageId,expectedResumeCas:{revision,digest}}`; any result data arm differs from exact `stageId|currentResumeCas|status|action|issue`; an `activated|alreadyActivated` arm has non-null issue; a `recoveryRequired` arm lacks its typed issue; or result data exposes `schemaVersion`, `auditEventId`, candidate/owner/ref/summary/initial-activation fields;
- static registration is not exactly 15 #35 handlers plus one separately owned `resume_staged_import_cutover`, the resume handler enters `SecretCommandName`/is counted as command 16, `lib.rs` lacks the exact-set assertion, or phase-crash UAT does not invoke it;
- durable `DeviceInstanceId` is conflated with process-local `DeviceSecretStoreInstanceId`, registry/broker authority is minted outside #35, backend policy differs from the five closed operations, or missing readback uses anything other than independently authorized `Validate` after durable delete;
- candidate discard/expiry changes/combines `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`, maps them other than `Delete|Validate`, reaches missing without durable `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}`, or becomes terminal before the fresh missing receipt consumes that CAS;
- normal activation or `activationCleanup` lacks `ActivationOldRecordDeleteCheckpoint|RecoveryOldRecordDeleteCheckpoint|ActivationOldRecordDurableCheckpoint`, loses the `ActivationOldRecordDeleteApplied` postcondition, retains fewer than the exact disposition/completedAt/CAS fields, lets fresh missing bypass that CAS, does not commit supersession atomically with terminal state, or has `revokedAt != backendCompletedAt`;
- slot counts differ from activation+recovery 10 plus candidate-discard 2 equals 12, or the five delete→missing pairs do not total 10 delete/missing slots; these counts must not change the five operations, 8 journals or 4 recovery kinds;
- `LegacySourceCoverageReceipt` is not opaque `pub(crate)` with private fields or its checked factory is not `pub(crate)`; anything except `CodexLegacySourceInventoryBridge` can construct `CompleteLegacySourceInventoryAuthority`; revision/11-domain identity/current expectations/adjacent observations are not atomically bound; `inventoryRevision` or `LegacySourceLocationId` is value-derived; `CompleteLegacySourceCoverageIdentity` is not exactly the fixed 11 domains once each; a domain lacks structural revision/presence/count; the receipt carries raw path/raw locator/value/value-derived digest; an empty collection/count is treated as complete proof; or startup/summary/capture/Provider-delete skips per-attempt bridge revalidation;
- either native macOS or native Windows Rust 1.85.0 exact-lock `--all-targets` record is absent/failing, or a Rust 1.97.1 result is substituted;
- `list_secret_backend_options` does not mint a single-use snapshot-bound `SecretCaptureIntentId`, `begin_secret_capture` accepts owner/legacy/binding data instead of only intent id + selected backend, any of `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` bypasses list→begin, or terminal expiry reuses old authority without summary/owner-card refresh;
- a capture/delete/activation-cleanup delete shares authorization/checkpoint with fresh missing readback, a probe can persist `revoked` without explicit `Revoke` authorization, or record/read/delete scope is not bound to the device-store instance and exact registered backend object;
- Codex env/common-config/public-Provider/override/diagnostic paths retain values, raw paths/URLs/errors/bodies, secret-bearing network checks, shared API-key UI or shared raw transport; or a new/moved `codexMcpEnvOrHeaderCredential` occurrence is not failed as Level 3 debt;
- Provider-domain `PROVIDER_DELETE_IMPACT_STALE`/`refreshProviderDeleteImpact` enters a secret error/action union, `refreshDeleteImpact` is used for owner detach, or a legacy Provider receives an impact id;
- a new secret-shaped source appears outside the inventory baseline;
- scanner output calls feature scope repository-global;
- schema numbering/registration is not adjudicated with Prompt/Memory and #55/#41 lanes.

## 12. Gates carried forward

### #35 design-freeze gates

1. Static review must verify the current contract retains inline-TOML source identity and excludes Codex terminal from every runtime `allowedConsumers` set; both changes are present in the working-tree design, while the enum remains wire-reserved only.
2. Main authority docs must adopt device-local state/journal and explicitly withdraw unilateral v17/schema ownership.
3. Static review must re-read the exact-path generator fixture and the policy/startup/import/recovery/delete closures added here: token-free feature DTOs and owner/ref usage/model-fetch requests; native pre-event deep-link rejection; primary coding-plan in scope; branched comparison policy/current-only activation; the eight-step one-handle startup with Blocked effect-none and gated structural history backup; opaque bridge-minted no-value receipt with non-value-derived revision, fixed complete 11-domain identity atomically bound to exact current expectations and adjacent observations, allowed internal non-value-derived location IDs, raw locator/value bans, empty-proof and per-consumer fresh-revalidation assertions; ARR-001 candidate two-slot/three-field checkpoint; ARR-002 named runtime/durable checkpoints and exact supersession time; ARR-003 operation-id plus cumulative five-phase preimage, five named fixtures and every-transition CAS; exact `10+2=12`/10-delete-missing slot assertions without changing five operations/8 journals/4 recovery kinds; split initial/resume results; exact 15+1 registration; durable/process instance separation; #35-owned registry/broker and five-operation/missing→Validate policy; core→owner adapters→main composition; native macOS/Windows Rust 1.85 locked all-targets records; the frozen staged admission→authority-match→#35 prepare→construct exact `ImportCutoverCoordinatorContext`→validation/scrub/readback→cutover→live-finalize order plus exact public resume CAS; typed capture intent routing; independent delete/readback authorization; explicit Revoke authority; `SNV7-001..006`; total secret action mapping; separate Provider-delete stale action; pending expiry not terminal.
4. Product/architecture/detailed reviewers must re-read the same immutable design commit and reach `P0=0, P1=0, P2=0` before `DESIGN_FREEZE=PASS`.

### Post-handoff implementation/source-freeze gates

1. #55 must publish a secret-safe immutable successor to `ca552f4d`; `6859e9ce` is readback evidence only.
2. #41 must replace raw backup/readback/recovery semantics and publish an immutable compatible implementation handoff.
3. Main integration must own all shared existing files; Prompt/Memory retains its adjudicated v17, and #35 adds no secret SQLite schema.
