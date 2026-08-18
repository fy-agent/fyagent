# secretRef contract handoff — DRAFT / DO NOT CONSUME

## Authority

- Upstream owner: GitHub Issue #35.
- Implementation base SHA: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`.
- Contract authority SHA: `PENDING_DESIGN_FREEZE`.
- Freeze receipt: `PENDING` at `research/design-freeze-receipt.md`.
- Contract version: `secret-contract/v1`.
- Consumers: Issue #55 Change Plan and Issue #41 Configuration Apply.

This working-tree file is not an authority. It becomes consumable only after all three static reviews show P0/P1/P2=0 on one immutable design commit and a later receipt names that exact SHA/file digest set. #35 freezes upstream first; it does not wait for #55/#41 to implement compatibility.

## Normative files in the future immutable handoff

1. `prd.md` — MVP/product/evidence boundary.
2. `secret-contract-v1.md` — exact TS/Rust wire, native-only signatures, IDs, state/error/action/audit matrices.
3. `device-local-secret-store.md` — no-v17 authority, journal/reconcile, platform APIs and startup/import order.
4. `research/codex-secret-call-graph.md` — mechanically registered Codex sources/consumers/sinks, owner map and #55/#41 deltas.
5. `research/os-keyring-options.md` — direct Keychain/CredMan/capture/MSRV decision.
6. `research/native-evidence-plan.md` — source-freeze/native/failure/UAT gates.

## Storage and owner boundary

- v1 runtime accepts only `provider/codex + codexApiKey + primaryApiKey`.
- `agent` is wire-reserved; every concrete request returns `SECRET_OWNER_KIND_UNSUPPORTED, effect=none`.
- Provider SQLite rows contain token-free Provider configuration only.
- record/ref/binding/candidate/journal/audit authority is device-local under `app_local_data_dir/device-local/secrets/v1` and is excluded from SQL/backup/WebDAV/S3 sync.
- #35 adds no SQLite schema/version and does not reserve v17.
- `secretRef` is `sec_` + lowercase UUIDv4 simple hex and never derived from value/owner/backend/hash.
- Startup mints an opaque `OpenedDeviceLocalSecretStore` from the immutable app-local root **before** DB open and holds its validated lifetime lock. The exact sequence is `open store → open_preflight_without_backup → same AppState/SecretService → reconcile → app.manage/static command registration → Clean sanitized backup → publish gate → workers`. The one production AppState consumes that same opened handle exactly once—no `PathBuf`, reopen or second AppState. `Blocked` produces no backup and starts/publishes no worker or consumer.
- `codex_history_migration` is downstream of the Clean no-value gate. It may create only structural-placeholder/non-secret-config generations. Existing generations are immutable scan/report-only artifacts; startup never rewrites or deletes them and never copies raw `settingsConfig` into a new backup.
- Current Provider authority and staged-import authority are different opaque types. Renderer `OwnerId` text never mints either one.
- Renderer Provider work uses closed token-free feature-draft/public/mutation DTOs. Add/Edit dialogs, feature hooks, forms/editors and cards never receive or emit full secret-bearing Provider state. Fixed usage/model-fetch requests carry owner/provider identity only. Empty `OPENAI_API_KEY`, `[REDACTED]` and hashes are forbidden substitutes.
- Durable `DeviceInstanceId` is the persisted device namespace. Process-local `DeviceSecretStoreInstanceId` is minted for one lifetime-locked store open, has no serde/text identity and expires at teardown. They are not aliases and cannot substitute for one another in a scope, receipt or replay check.
- `#35 module` owns `SecretBackendRegistry` and the private operation broker. The exact backend policy is `CaptureVerify|Validate|ResolveForApply|Delete|Revoke`; every post-delete fresh missing readback uses a separately authorized `Validate` slot after the durable delete checkpoint, never a sixth Missing operation or ordinary probe.
- `LegacySourceCoverageReceipt` is opaque `pub(crate)`, non-Clone/non-Serde/non-Debug and move-only: store, Provider and #35 sibling modules may name/move/consume it, but all fields are private. Its `pub(crate)` `checked_from_complete_inventory_authority` factory consumes private unforgeable `CompleteLegacySourceInventoryAuthority`, which only main-integration `CodexLegacySourceInventoryBridge` can construct. The receipt atomically binds non-value-derived `LegacySourceInventoryRevision`, fixed complete `CompleteLegacySourceCoverageIdentity`, exact `CurrentLegacySourceExpectations` and category/state-only `AdjacentBlockedLegacySourceObservation` rows; proof cannot detach from data. The identity covers exactly `currentProviderLiveScrubbable` and the ten frozen supplemental domains, each with structural revision/presence/count. Current refs may carry only opaque non-value-derived `LegacySourceLocationId`; raw path, raw locator, value and value-derived digest are forbidden. Supplemental observations never become refs. Public `LegacySourceCoverageView` has no authority. Startup, every owner summary/readiness projection, capture options and claimed-intent revalidation, and Provider-delete preview/confirm each obtain/revalidate a fresh atomic bridge receipt. An empty set/count without explicit complete all-11-domain absence proof is blocked.
- `ARR-001`: candidate discard/expiry prepares exact `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`, mapped respectively to `Delete` and `Validate`. Delete/already-missing persists `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}`; only a fresh missing receipt consuming that CAS can reach `missingReadbackVerified` and terminal immutable disposition.
- `ARR-002`: normal activation and recovery respectively use `ActivationOldRecordDeleteCheckpoint` and `RecoveryOldRecordDeleteCheckpoint`; `ActivationOldRecordDeleteApplied` preserves the postcondition and `ActivationOldRecordDurableCheckpoint` preserves the exact crash-visible `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`. Fresh `Validate` missing consumes the CAS, and terminal supersession atomically records `revokedAt=backendCompletedAt`.
- `ARR-003`: staged resume preimage binds journal `operationId` plus cumulative-field `StagedImportResumePhase::{Intent,SourcesScrubbed,CutoverCommitted,LiveOwnerMinted,LocalBindingFinalized}`. Exact fixtures are `staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`; every fresh nonce/admission or phase/checkpoint transition increments revision/new CAS.
- Slot counts are fixed at activation+recovery 10 plus candidate-discard 2 equals 12; five delete→missing pairs account for 10 delete/missing slots. These counts do not alter five hardware operations, 8 journals or 4 recovery kinds.

## Consumer sequence

```text
#55 candidate activation
  legacy/unbound owner -> verified candidate
  -> SecretCandidateActivationProjection(comparisonPolicy)
  -> immutable activation plan admission

#41 activation operation
  activation-specific candidate-read + optional old-record-delete prepare/confirmation
  -> activation Provider lease + final admission/baseline
  -> #35 exact current source-set/revision fresh-read
     candidateEquality: constant-time compare every admitted value to candidate
     explicitReplacement: exact approved set/revisions, no old-value=candidate assertion
  -> #35 binding CAS + exact Provider scrub/readback through held transaction context
  -> release activation lease

#55 separate live apply
  re-read now-bound owner -> typed readiness
  -> SecretApplyPlanProjection -> immutable apply plan admission

#41 live apply operation
  prepare target capability and optional rollback capability before a new lease
  -> optional operation-scoped hardware confirmation
  -> apply Provider lease + final plan/baseline recheck
  -> sanitized structural backup
  -> #35 resolve_for_apply(one-shot material-free capability, existing writer)
  -> #35 revalidates all revisions/sink immediately before material acquisition
  -> exact owner-private consuming writer + readback
  -> typed non-sensitive result only

staged SQL / restore / sync import
  register one temp DB live object without staged source validation
  -> temp authority/token + material-free StagedSecretImportActivationProjection
  -> #55 dedicated immutable staged admission
  -> main-integration authority-match receipt
  -> #35 prepare_staged_import / confirm_staged_import
  -> construct exact ImportCutoverCoordinatorContext
  -> staged source fresh-read/policy check + exact scrub/readback
  -> cutover consumes admitted staged CAS and returns main DB cutover receipt
  -> live DAO owner token + live owner/binding finalize
  -> post-cutover crash: registered resume accepts only
     {stageId,expectedResumeCas:{revision,digest}}
     and returns currentResumeCas:{revision,digest}
  -> only then ordinary bound-owner readiness/runtime
```

`activate_candidate_from_change_plan`, `prepare_for_apply`, `confirm_for_apply` and `resolve_for_apply` are native-only and are never Tauri commands. A prepared capability is not serializable/persistable and contains no material.

Initial staged activation and crash resume expose separate closed result types. `resume_staged_import_cutover` request data is exactly `{stageId,expectedResumeCas:{revision,digest}}`. Every result data arm is exactly `{stageId,currentResumeCas,status,action,issue}`: `activated|alreadyActivated` returns `issue=null`, while `recoveryRequired` returns its typed issue. Result data cannot contain `schemaVersion`, `auditEventId`, candidate id, owner, secret ref, credential/owner summary, any initial-activation payload or any unlisted field. The common command envelope owns version/command id and audit is independent. The resume handler is a main-integration handler, not one of the 15 #35 `SecretCommandName` entries.

`CodexLiveSecretSinkId` is a closed, path-free downstream contract. V1 has exactly `codexAuthJsonOpenAiApiKey` (the API-key slot in `auth.json`) and `codexConfigTomlExperimentalBearerToken` (the bearer-token slot in `config.toml`). Every target/rollback credential projection carries exactly one `liveSinkId`. #55 includes that exact role/ID pair in plan admission/digest; #41 binds it into role-specific writer construction/readback and final baseline. Unknown sink IDs, OAuth fields, paths and inferred aliases are rejected before backup or lease mutation.

All native material reads are scope-bound. The registered backend may return only a consuming `AuthorizedBackendRead` that owns the same sealed operation/ref/store/record/binding/backend/device/capability/role-or-slot/consumer/sink/expiry scope prepared for the named operation. It has no bytes/material getter and can be consumed only by the corresponding backend-sealed apply, fixed-runtime, activation-compare or recovery callback. Scope substitution, replay or expiry fails before the callback/material acquisition.

Codex deep links are metadata-only at native ingress. Raw query, percent-encoded and downloaded/merged remote-config secret fields are rejected before parse, merge, Provider construction or event dispatch. `deepLinkConfigPreview.ts` and `DeepLinkImportDialog.tsx` therefore never decode, mask or preview a Codex secret. Generated-canary negative fixtures require a stable no-echo error and zero event/preview/DB/live writes.

## Candidate semantics for create/edit/rotate

- Capture is a typed two-command flow within the frozen 15-command surface. `list_secret_backend_options` reads the native owner, complete legacy-source set and binding snapshot itself and then mints one opaque, single-use, snapshot-bound `SecretCaptureIntentId`. `begin_secret_capture` accepts only `{captureIntentId, selectedBackend}`; owner text, legacy occurrences, binding snapshots, candidate/ref material and backend capability claims are rejected at this boundary.
- Before options or intent mint, capture obtains a fresh atomic `LegacySourceCoverageReceipt` through `CodexLegacySourceInventoryBridge`; begin/claim re-enters the bridge and obtains/revalidates a new receipt rather than trusting the intent snapshot. Its current exact expectations and adjacent observations remain bound to the same 11-domain proof. Supplemental observations cannot become `LegacySourceRef` or capture scrub authority; missing/stale coverage or zero counts without complete proof blocks without intent id or backend access.
- `retryCapture`, `captureReplacement`, `chooseBackend` and `resolveLegacyConflict` all resolve to that same `list_secret_backend_options → begin_secret_capture` flow; no action maps directly to begin with renderer-supplied source state. Terminal expiry first refreshes the current summary/owner card and only then mints fresh capture or rotation authority. It never reuses the expired candidate, capture intent, operation id, capability or admission.
- Native capture/reconcile writes and verifies a new backend entry only after durable intent.
- Result is `verifiedPendingPlan`; it changes no binding, Provider row or live target.
- #55 plan names the exact activation projection, owners, revisions and one `LegacyActivationComparisonPolicy`.
- Automatic one-value migration and `legacyScrubExistingBinding` use `candidateEquality`: before intent/CAS #35 fresh-reads the complete admitted current source set and candidate backend record, then constant-time compares each value with the candidate.
- User-approved conflict replace/reconcile/rotate uses `explicitReplacement`: #35 verifies the exact approved old source set and structural revisions plus candidate authority, but old values are being replaced and are **not** required to equal the new candidate. Ordinary retry cannot silently select this policy.
- #41 completes activation-specific prepare/confirmation, obtains an activation Provider lease, rechecks #55 baseline, then calls #35; #35 applies the admitted policy, activates exact bindings and scrubs only approved refs without acquiring the lease itself.
- #41 releases the activation lease. Only then may #55 create a separate apply plan for the now-bound owner; #41 later uses a new apply lease/writer stage. Candidate activation never writes the live target or becomes a #35/#55 direct Provider writer.
- Binding-set drift produces `SECRET_DEPENDENCY_CHANGED, effect=none`.

The normal activation projection is current-only: its scrub set accepts `providerRow|liveAuth|liveConfig`, never `sqlImportStaging|dbRestoreStaging|syncDownloadStaging`. Staged sources can appear only in `StagedSecretImportActivationProjection`; neither staged token nor projection authorizes public readiness, proxy, usage, coding-plan, model fetch or live apply. Staged wire/durable objects contain no raw path, material, value, source-value digest or whole-file/DB digest.

`sync_protocol.rs` and its WebDAV/S3/archive callers cannot reach Skills or main-DB mutation ports until `temp authority/token + material-free projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact ImportCutoverCoordinatorContext → staged source validation/scrub/readback → cutover → live owner/binding finalize` has advanced through the required checkpoint. The authority-match receipt is consuming and #35 never accepts a raw admission or renderer-supplied temp identity. Main integration constructs the exact context immediately after #35 confirmation; that context is the sole authority for every later staged source validation, scrub/readback and cutover, so none of those operations is reachable before or outside it. Cancel, confirmation expiry, replay, old nonce/admission/CAS and every pre-cutover crash are `effect=none`/zero-write. Exactly one registered resume request shape exists: `{stageId,expectedResumeCas:{revision,digest}}`; every result returns `currentResumeCas:{revision,digest}`. The internal digest preimage includes journal `operationId` and cumulative `StagedImportResumePhase`: `intent` has no checkpoint field; `sourcesScrubbed` adds `stagedSourceSetCasAfterScrub`; `cutoverCommitted` retains it and adds `cutoverReceiptId`; `liveOwnerMinted` retains both and adds `promotedLiveOwner`; `localBindingFinalized` retains the same three. Missing/extra fields reject. Exact fixtures are `staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`; every fresh nonce/admission and every phase/checkpoint transition increments revision/new CAS. Wrong object, stale or replayed resume remains effect-none and keeps consumers blocked until terminal.

## Capability matrix

| Capability | OS keyring MVP | Future hardware |
| --- | --- | --- |
| instance | current host/user, per device instance | exact registered device/plugin instance |
| storage residency | `osProtectedStore` | `hardwareOnly` |
| physical confirmation | v1 operations `never` | per operation `never/optional/required` |
| allowed consumers | reviewed native apply/proxy/fixed usage (including Provider-primary coding-plan)/model fetch | per-record explicit subset |
| persistent target projection | allowed only for exact reviewed sink | commonly false; false is a hard reject |
| central/device revocation | false; no observation accepted | per record/instance capability with validated observation |
| fallback | always false | always false; never OS-keyring fallback |

No hardware adapter is implemented by #35 MVP. If none is registered, Add/Replace hides hardware; existing hardware binding is unavailable/device mismatch.

Provider-primary coding-plan fixed adapters use an owner/ref no-value request and the closed `consumer=FixedRuntimeConsumer::CodingPlanUsageProbe` with `CodingPlanPrimaryAdapter`; that consumer belongs to `usageProbe/codex_feature_runtime`. Generic `UsageProbeKind` remains usage/balance-only. ZenMux's separately hand-entered key/base URL, Volcengine AK/SK and independent team/login credentials are different purposes and remain adjacent debt; they can neither consume nor stand in for the Codex primary binding.

Revocation is explicitly authorized and observation-driven, never inferred. Persistent `revoked` requires an explicit `Revoke` authorization plus `BackendRevocationObservation`; the observation is a non-clone, non-serde, consuming scope receipt—not a free `{source,time}` DTO. Only the exact registered backend object may mint it after validating `centralRevocation=true + SourceAndTime`; authorization and observation bind ref, device-store instance, record, binding-set, backend object/device and capability CAS plus the closed source/time. Device authority fresh-revalidates and consumes both by value. Probe output is non-persistable. OS keyring has no central-revocation capability. Not-found, locked, denied, unavailable, caller-supplied ref/source/time or an arbitrary adapter message remains its own state and never fabricates `revoked`.

## Presence, stable state and operation state

- `presence = present | missing | unknown`; locked/denied/unavailable never become missing.
- stable availability = `ready | missing | locked | denied | stale | revoked | unavailable`.
- `locked` requires `lockSource=fyAgentPolicy|backend`.
- `revoked` requires source/time/action and differs from unexpected `missing`.
- migration is owner-level legacy state.
- `confirmationRequired` and `HardwareConfirmStep` are operation-only; stable summaries/cache/state never contain them.

## Provider detach, general recovery and candidate expiry

Provider deletion is owner detach, not secret deletion. A bound/unbound `CodexProviderDeleteImpactDto` is no-value and must be fetched before confirmation. It freezes a native `providerDeleteImpactId`, Provider/owner revisions, expiry and—when bound—the exact binding-set CAS; the bound variant also shows `remainingOwners`, `becomesOrphan`, `secretRetained=true` and `separateSecretDeleteAction=get_secret_delete_impact`. A legacy owner is blocked before preview and receives **no impact id**. `CodexProviderDeleteConfirmRequestDto` returns only the opaque impact id, which is claimed/revalidated once. Drift, expiry or replay returns the complete Provider-owned envelope `PROVIDER_DELETE_IMPACT_STALE + refreshProviderDeleteImpact + effect=none`. That error/action is not a `SecretErrorCode` or `SecretUserAction`; `refreshDeleteImpact` remains exclusively the action for separate secret deletion. `CodexProviderDeleteResultDto` either reports `providerDeletedSecretRetained` or an `ownerDetachFinalization` recovery pointer. The backend record, orphaned or shared, survives until a separate secret-delete impact/confirmation command succeeds.

`SecretRecoveryKind`, `SecretRecoveryPointer.kind`, `SecretRecoveryImpact` and `SecretRecoveryResult` form one closed union:

| Kind | Execution authority |
| --- | --- |
| `activationCleanup` | active-record compare/scrub and optional old delete; old-record delete and subsequent fresh missing readback have separate authorization/checkpoints prepared before the #41-held Provider lease |
| `captureCompensation` | local-only probe, independently authorized candidate delete, then separately authorized fresh missing readback; outcome stays non-terminal until both checkpoints persist |
| `deleteFinalization` | local-only completion of an already-admitted user delete; delete and fresh missing readback are independently authorized/checkpointed; preserves only explicitly authorized Revoke provenance |
| `ownerDetachFinalization` | exact already-held Provider detach context; completes local owner/binding CAS and retains the backend secret |

Every impact carries its kind, exact recovery CAS, material-free remaining steps and the fields legal only for that variant. Every result repeats the kind and is either terminal or returns a non-empty remaining-step set. Non-terminal recovery maps to `completeRecovery`, whose `SECRET_ACTION_DESTINATIONS_V1` destination is exactly the two-command flow `get_secret_cleanup_impact → retry_secret_cleanup`; CAS drift maps to `refreshRecoveryImpact`. Delete and fresh missing readback are never one step: delete consumes `Delete`, persists its exact three-field checkpoint, and only then may a separate missing-readback authorization consume the closed `Validate` operation and its `deleteAppliedCas`. Normal/recovery activation use the named runtime/durable checkpoints above; missing receipt and terminal supersession commit atomically with `revokedAt=backendCompletedAt`. Missing never adds a backend operation and probe is not evidence. The complete `SecretUserAction` union is keyed by the same total destination table: every action resolves to one registered secret command, fixed command flow, native-confirmation continuation, exact non-#35 `mainIntegrationCommand` or explicit external guidance. `resumeStagedImportCutover` maps only to the separate main-integration `resume_staged_import_cutover({stageId,expectedResumeCas:{revision,digest}})` handler, returning the closed resume result with same-shaped current CAS. TS/Rust decoders and UI use closed total switches; unknown action, missing destination or a row that offers multiple runtime choices fails closed.

Candidate explicit discard and expiry are deliberately outside this recovery union. The first `discardCandidate` intent fixes `pendingTerminalDisposition=discarded|expired` and prepares distinct delete/`Validate`-missing slots. Backend delete/already-missing persists `CandidateDiscardDeleteCheckpoint {deleteDisposition,backendCompletedAt,deleteAppliedCas}`; only the later separately authorized fresh missing receipt may consume its CAS. Until `missingReadbackVerified` and state commit finish, public state remains `verifiedPendingPlan` with that immutable disposition, `SECRET_OPERATION_RECOVERY_REQUIRED` and the sole action `discardCandidate`; retry cannot relabel it. Only the terminal journal may expose `discarded|expired` and remove the pending field. Expiry then maps to summary/owner-card refresh followed by a fresh capture/rotation list→begin flow, never direct reuse.

Terminal confirmation routing is context-total and always creates fresh authority: capture→`retryCapture`, rotate→`retryRotation`, candidate discard→`discardCandidate`, secret delete→`refreshDeleteImpact`, recovery→`refreshRecoveryImpact`, apply/activation→`reopenChangePlan`, staged import→`resumeStagedImportCutover`. Cached legacy summaries never use invocation-dependent `retry`: single-value/comparison-pending→`refreshSummary`, source-invalid/source-conflict→`resolveLegacyConflict`, binding-conflict→`captureReplacement`, approval-required→`reopenChangePlan`. `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` use the concrete native-snapshot list→single-use-intent→begin flow; a terminal expiry prepends summary/owner-card refresh. No route reuses a consumed operation id, capture intent, capability, pending confirmation, admission, nonce or CAS.

## Downstream-stable error families

The draft literal/matrix authority lives in `secret-contract-v1.md`. Until design freeze, consumers must treat this handoff as non-consumable; the intended families below still preserve no raw message/detail:

| Family | Required effect |
| --- | --- |
| secret request/owner/purpose/candidate/plan/delete-impact invalid or stale | `effect=none`; exact `refreshSummary`, secret-delete-only `refreshDeleteImpact`, `refreshRecoveryImpact` or `reopenChangePlan` from the canonical destination matrix |
| Provider owner-detach impact stale/expired/replayed | Provider-domain `PROVIDER_DELETE_IMPACT_STALE + refreshProviderDeleteImpact + effect=none`; never decode either literal through the secret error/action unions |
| generic legacy action/alias | must normalize to one canonical `SECRET_ACTION_DESTINATIONS_V1` entry or fail closed; it cannot introduce a renderer-chosen fallback destination |
| migration/conflict/comparison pending | no value choice, no writer; explicit reconcile action |
| missing/locked/denied/unavailable/device mismatch | no material, no target/network mutation, no fallback |
| revoked/stale/recovery required | exact source/kind selects one frozen action; never folded to missing or a runtime choice |
| confirmation cancel/expiry/replay | stable binding/target unchanged |
| dependency/record/backend/capability changed | first mutation not invoked |
| write/read/verify/delete failure | exact pre/post-switch partial semantics from matrix |
| projection forbidden | plan blocked before backup/lease mutation |

Codes not enumerated for a command are converted to `SECRET_INTERNAL`; arbitrary OS/upstream/config text is never added to the envelope.

`SECRET_BACKEND_UNAVAILABLE` likewise carries a closed reason when action differs: registered mappings choose `chooseBackend`, `reconnectDevice`, `openBackendSettings` or another single frozen destination; it cannot map every case to a generic retry loop. Revocation actions are selected by the validated observation source at contract construction time, not by renderer branching over a prose suggestion.

## #55 required compatibility delta

Known local readback:

- source/contract SHA `ca552f4d918cacc734f81f7efdef70619da139b8`;
- final branch SHA `6859e9ce04970008f4cf8b3d4883b4f70316291a`;
- 2026-08-15 remote relation readback: `ca552f4d` is merge-base/ancestor, branch is ahead 3 / behind 0;
- that relation is still an incompatible input, not a #35-compatible handoff.

These SHAs are inputs, not compatible outputs. They still hash secret-bearing Provider/live projections and call `ProviderService::switch` directly. A compatible successor must:

1. retain a #55-owned closed token-free Provider structural projection whose credential member is exactly #35 `SecretApplyPlanProjection` and whose live member is exactly `CodexLiveStructuralProjection`; every credential value field is absent;
2. digest only sanitized structure; value/material/secret-bearing bytes and their digest are forbidden;
3. carry owner/ref, record/binding-set/backend/device/capability revisions, consumer/sink and the exact closed `liveSinkId` for each target/rollback role; include the role/ID pair in plan digest and affected-resource identity without path strings;
4. create an independent candidate-activation plan for create/edit/rotate, freeze `comparisonPolicy=candidateEquality|explicitReplacement` in projection/admission/digest, and never apply candidate-equality rules to an explicit replacement;
5. accept the different material-free `StagedSecretImportActivationProjection` after main integration has registered one temp live object without staged source validation and minted its internal temp authority/token; issue the dedicated staged admission **before** #35 staged prepare/confirm. Main integration must consume an authority-match receipt between admission and #35, then construct the exact `ImportCutoverCoordinatorContext` immediately after #35 confirmation and before any staged source validation/scrub/readback/cutover. That context is the sole authority for those operations. The projection binds stage/source-set CAS and import cutover authority, contains no raw path/value/value digest/full identity, and cannot be decoded as ordinary `secretCandidateActivation`;
6. after activation/lease release, create a separate apply plan for the now-bound owner—never embed an unbound or staged owner/candidate in `SecretApplyPlanProjection`;
7. delegate admitted apply to #41 coordinator; no #55 resolver or direct writer;
8. consume #41 typed write/readback receipt, never re-hash live material.

#55 compatible immutable SHA blocks later code integration/source freeze, not upstream #35 design freeze.

## #41 required compatibility delta

A compatible implementation must:

1. for candidate activation, use the activation-specific prepare/confirm/cancel bundle before the activation Provider lease, including candidate read/compare authorization and any old-record hardware delete expectation;
2. under that lease, recheck the activation admission/baseline and invoke #35 with an opaque already-held transaction context; #35 applies the admitted `candidateEquality|explicitReplacement` policy before intent/CAS and activation writes no live target;
3. release the activation lease, re-read the now-bound owner and accept only a separate `SecretApplyPlanProjection`;
4. prepare apply target and optional rollback capabilities and complete hardware confirmation before a new apply lease;
5. under the apply lease, recheck plan/baseline including each role's exact `liveSinkId`, create structural placeholder backup only, then consume #35 target capability once inside the matching owner-private writer/readback boundary;
6. never persist material, capability, backend locator, exact secret-bearing backup or value-derived digest;
7. reprepare after restart; rollback uses a separately prepared capability;
8. reject hardware `persistentTargetProjection=false` before mutation.
9. for `activationCleanup`, prepare/confirm active-record compare and old-record delete authorization before a cleanup Provider lease; after delete, consume a separate fresh-missing-readback authorization and persist its distinct checkpoint before terminal state. No cleanup UI opens while a lease is held. Other recovery kinds follow their closed local-only or already-held detach authority and must not be coerced into activation cleanup.
10. reject staged-owner tokens/projections at every ordinary activation/apply/readiness entry. `main integration` ImportCoordinator—not #41—constructs the exact `ImportCutoverCoordinatorContext` after #35 confirmation and before source validation, then uses that sole authority to validate/scrub/read back the temp live object and perform cutover; it converts to a live DAO owner token and finalizes the live binding only after the cutover receipt.

#41's current untracked design work is not an immutable dependency. Its future compatible SHA blocks integrated source freeze, not #35 design freeze.

## Owner publication and composition gate

The compilation order is fixed and independently receipted:

1. `#35 module` publishes core traits/APIs and passes focused core compilation/tests without importing any #55/#41/main concrete adapter type. In particular, `src-tauri/src/secret/backend.rs` cannot reference, seal or instantiate a concrete external type that has not landed.
2. #55 publishes its Change Plan adapter types in #55-owned files; #41 publishes apply adapter types in #41-owned files; main-owned Provider/proxy/import/startup adapter types land in their §9.4 owner files. Each owner passes its focused gate before composition.
3. The sole `main integration` adapter/composition owner wires only those published types, registers exactly 15 #35 handlers plus the separate `resume_staged_import_cutover` handler, asserts both sets in `src-tauri/src/lib.rs`, then runs the full Rust gate and staged phase-crash UAT.

Within main integration, `CodexLegacySourceInventoryBridge` is the single coverage inventory bridge and the sole constructor of `CompleteLegacySourceInventoryAuthority`. Canonical ownership of a shared file does not authorize another adapter/store/Provider/#35 module to field-construct or inspect `LegacySourceCoverageReceipt`; although the checked factory is `pub(crate)`, compile-fail/privacy assertions prove those siblings cannot fabricate its authority argument and can only name, move and consume the opaque receipt.

Source freeze additionally requires matching-host macOS and Windows records for Rust 1.85.0 `cargo check --locked --all-targets` against the exact `Cargo.lock` and immutable source SHA. A Rust 1.97.1 gate, cross-compile-only result or one-host result cannot substitute.

## `SNV7-001..006` main-integration handoff delta

All exact paths and focused evidence are frozen in `codex-secret-call-graph.md` §9.4. Every file below has one canonical writer, `main integration`; neither #35 module, #55 nor #41 may independently edit a shared surface.

| ID | Required downstream contract |
| --- | --- |
| `SNV7-001` Codex env | `OPENAI_*` scan/list/delete/restore projects only `{name,present,stableSourceCategory}`. Process env, Windows HKCU/HKLM and shell-file category occupy fixed receipt domains `processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile`, each with structural revision/presence/count and never `LegacySourceRef`; values and absolute source/backup paths are absent from receipt/command/API/type/UI/event/error. Startup/summary/capture/Provider-delete each fresh-revalidate through the sole bridge. Codex delete/restore must not make a plaintext backup. |
| `SNV7-002` common config | A new Codex TOML snippet containing a secret-shaped/value-bearing field rejects before SQLite, localStorage, live config, export or sync writes. Existing `config.json`, `.bak`, `.migrated`, `settings.common_config_codex`, localStorage and live-merge occupy fixed receipt domains `commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge`, each with structural revision/presence/count and never `LegacySourceRef`. `LegacySourceCoverageView`/API/hook/modal is no-authority and never carries raw TOML/path. |
| `SNV7-003` public Provider | Codex internal, public and mutation types are distinct. Public list/query/sort/MSW objects and mutation input have no `settingsConfig`; unknown/secret-shaped fields reject before writes. Codex does not render shared `ApiKeySection`/`ApiKeyInput`. The existing list/update/sort/add/edit fixture chain is rewritten as generated-canary negative coverage. |
| `SNV7-004` request overrides | New Codex `local_proxy_request_overrides` secret/header/body mutation rejects; an existing Codex row with any such override fails closed before proxy/network work. Primary material uses only the owner-private, single-send, zeroizing transport at the final send boundary; it never enters shared raw `Vec`/header maps. If non-Codex overrides remain, their exact occurrences stay Level 3 debt. |
| `SNV7-005` diagnostics | Codex never performs an active secret-bearing stream check. Stream/proxy/failover diagnostics are mapped before persistence/UI to a closed status/category/latency DTO, with no raw URL, upstream error, body or message. Reflection-canary and network/DB/UI/log spies enforce zero leak and zero network on blocked Codex checks. |
| `SNV7-006` MCP debt | Codex MCP `env`/`http_headers` is `codexMcpEnvOrHeaderCredential`, explicit Level 3 adjacent debt spanning unified server JSON, SQLite `server_config`, command/API/UI, live Codex TOML, legacy config, DB backup/export, WebDAV/S3 sync/import and fixtures. Replace checked-in static secret literals with runtime-generated canaries; exact occurrence count/category is no-regression and any addition/move fails. It is not Provider-primary Level 2 PASS scope. |

These deltas do not add secret commands, hardware operations, journals, recovery kinds or schema ownership: totals remain 15 #35 commands, five hardware operations, 8 journals, 4 recovery kinds, no #35 v17 and no fallback. Authorization/confirmation slot accounting is separately `activation+recovery=10`, `candidate-discard=2`, aggregate `12`; the five delete→missing pairs account for 10 delete/missing slots. `resume_staged_import_cutover` is one additional separately owned main-integration handler and is explicitly not command 16. They also do not make this handoff consumable; authority SHA and receipt remain `PENDING`.

The staged/capture/resume/recovery rules in this handoff are synchronized to device-store findings `PV7-001..003`, `CAV7-001..010` and `ARR-001..003`: exact public resume CAS with operation-id/five-phase preimage, summary-refresh-first terminal expiry, one typed capture-intent registry, admission-before-#35 staged order, exact context construction after #35 and before every staged validation/scrub/readback/cutover, split delete/readback authority/checkpoints, durable three-field candidate/activation/recovery delete provenance, explicit Revoke, store-instance/exact-backend binding and private per-operation authority. A conflicting downstream interpretation is incompatible and must not consume this draft.

## Forbidden fields/data

Forbidden in #55/#41/public Provider/V2 plan/job/query/cache/event/log/receipt/fixture/diagnostic schemas:

```text
secret, secretValue, value, apiKey, api_key, openaiApiKey,
experimentalBearerToken, token, accessToken, refreshToken, accessKey,
secretKey, password, authorization, credential, privateKey,
credentialBlob, backendLocator, rawError, rawMessage, rawConfig,
providerSettings, liveSettings, absolutePath, materialDigest
```

This is a verbatim source-spelling copy of `secret-contract-v1.md` `FORBIDDEN_SEMANTIC_FIELDS_V1`; separator/case variants canonicalize to the same set. Any future change must update both files in one design candidate.

Do not insert empty string, `[REDACTED]`, partial value, value hash or raw `serde_json::Value` as a substitute. Exact contract names for refs, display suffix, revisions, stable codes/actions, device display and sanitized structural digest are allowed.

## Scanner/claim boundary

- `contract_schema` and `codex_feature_runtime` are future required gates, not current PASS claims. They include token-free feature draft/public/mutation DTOs; fixed owner/ref usage/model-fetch/coding-plan requests; native-rejected deep links; the typed capture intent flow; four-kind recovery with separate delete/readback authorization and exact ARR-001/002 named checkpoints; explicit Revoke authority; staged import `token+projection → #55 admission → authority-match → #35 prepare/confirm → construct exact ImportCutoverCoordinatorContext → source validation/scrub/readback → cutover → live owner/binding finalize` plus ARR-003 `StagedImportResumePhase`, five named fixtures and every-transition CAS; startup history backup; Provider-delete impact/confirm plus its separate Provider-domain stale envelope/action; and `SNV7-001..005`.
- Contract/source freeze also rejects a shared initial/resume result type; resume request data beyond exact `stageId+expectedResumeCas`; resume result data other than exact `stageId|currentResumeCas|status|action|issue`; non-null issue for `activated|alreadyActivated`; absent/untyped issue for `recoveryRequired`; version/command-id/audit/candidate/owner/ref/summary in result data; candidate discard without exact `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`→`Delete|Validate` mapping or `CandidateDiscardDeleteCheckpoint`; missing readback before the durable checkpoint; absent/malformed `ActivationOldRecordDeleteCheckpoint|RecoveryOldRecordDeleteCheckpoint|ActivationOldRecordDeleteApplied|ActivationOldRecordDurableCheckpoint`; terminal supersession not atomic or `revokedAt != backendCompletedAt`; staged preimage without operation id, exact cumulative-field `StagedImportResumePhase`, five named fixtures or every-transition new CAS; slot counts other than `10+2=12` and 10 delete/missing slots; a non-opaque/non-`pub(crate)` coverage receipt or non-`pub(crate)` checked factory; externally constructible fields; authority construction outside `CodexLegacySourceInventoryBridge`; detached revision/identity/current-expectation/adjacent-observation data; value-derived inventory revision or `LegacySourceLocationId`; a non-exact 11-domain `CompleteLegacySourceCoverageIdentity`; missing structural revision/presence/count; raw path/raw locator/value/value-derived-digest receipt content; empty-without-complete-proof acceptance; skipped per-attempt bridge revalidation for startup/summary/capture/Provider-delete; supplemental observations decoded as scrub refs; `backend.rs` predeclaring external adapter types; owner/composition order drift; registration other than 15 #35 + separate main resume; durable/process-instance conflation; registry/broker ownership drift; operation policy other than the closed five or missing-readback other than `Validate`; and missing native macOS/Windows locked Rust 1.85.0 `--all-targets` records.
- adjacent credential domains remain in `repository_static_inventory` baseline/no-regression.
- Codex MCP `env`/`http_headers` is explicitly `codexMcpEnvOrHeaderCredential` Level 3 adjacent debt. Its DB/live/export/sync/UI/fixture occurrences remain exact no-regression entries; new/moved occurrences fail. It is not counted toward Provider-primary Level 2 PASS.
- `repository_runtime_global` remains `NOT_CLAIMED`.
- plan stage never resolves a secret, contacts upstream or mutates DB/live/backend.
- the exact inventory generator must emit every registered source/value/fixture path in `codex-secret-call-graph.md` §9.4, including all 127 `SNV7-001..006` path/category entries, expanded Provider feature UI/API, deep-link renderer, sync protocol/history startup and named command/form/template/preset fixtures; prose presence without a generated path/category/owner/evidence entry fails source freeze.
- every named checked fixture uses a runtime-generated canary plus token-free/structural negative assertions. Empty/redacted values, masked renderer previews and test-only waivers do not count as safe output.

## Open items before handoff becomes consumable

- exact contract agent final static self-check and main-owner reconciliation;
- product/architecture/detailed re-review with all P0/P1/P2 closed;
- immutable design authority commit and separate freeze receipt commit;
- message send + readback to #55/#41.

Post-handoff open compatibility items (#55/#41 successor SHAs, shared integration, Windows runtime evidence) are implementation/source-freeze gates and do not make the upstream contract draft consumable early.
