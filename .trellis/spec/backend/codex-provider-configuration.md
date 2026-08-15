# Codex Provider Configuration Contract

## 1. Scope / Trigger

Read this contract before changing Codex Provider TOML analysis or mutation,
native capability controls, vendor-specific model projection, session-resume
command construction, provider warnings, or the `liveConfigChanged` result.
It owns the Codex provider configuration domain only. Trusted Codex Desktop
discovery, installation, process restart, and launch are owned by
[Codex Desktop Installer](./codex-desktop-installer.md); application version and
release metadata are owned by their dedicated contracts.

## 2. Signatures

```text
add_provider_with_result(provider, app, addToLive?)
update_provider_with_result(provider, app, originalId?)
delete_provider_with_result(id, app)
switch_provider_with_result(id, app)
import_default_config_with_result(app)
  -> { value, liveConfigChanged, app, warningCodes? }

prepare_provider_mutation(redactedDraft, original?, createPolicy, secretRequirements)
  -> PreparedProviderMutation

plan_codex_projection(preparedMutation, injectedSnapshots)
  -> PreparedCodexProjection

apply_prepared_change(exactPayload, expectedResourceCAS, secretRequirements, effectGate)
  -> MutationAttemptResult

commit_prepared_change(exactPayload, expectedResourceCAS, secretLeases, effectPermit)
  -> MutationAttemptResult              # private Provider IO seam

get_universal_providers() -> Map<id, UniversalProviderMutationViewV1>
get_universal_provider(id) -> UniversalProviderMutationViewV1?
mutate_universal_provider(request: UniversalMutationRequestV1)
  -> UniversalMutationOutcome
commit_universal_mutation(preparedExactMutation, secretLeases, universalMutationPermit)
  -> UniversalMutationOutcome           # private, consumes permit

analyze_codex_provider_features(app: "codex", provider, isNew?)
  -> CodexProviderFeatureState

patch_codex_provider_features(app: "codex", provider, intent, isNew?)
  -> {
       tomlText,
       state,
       imageExtensionConfigured?,
       codexNativeCapabilitiesGeneratedProvider?
     }
```

Feature commands reject every `app` other than Codex. No provider command may
accept or return a filesystem path, process identifier, launch command,
credential-bearing diagnostic, or generic application-version field.

Successful Codex add/update mutations may return these stable warning codes:

```text
CODEX_WEBSOCKET_NON_GPT_MODEL
CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED
```

### Protected mutation and Change Plan ownership

- Codex create/edit/switch production entries are protected by
  [Unified Change Plan](./unified-change-plan.md). Once an entry is migrated it
  cannot fall back to these legacy direct commands when Plan capability is
  unavailable.
- The signatures above remain callable for compatibility, but each cut-over
  Codex operation is guarded at every Tauri boundary. Both plain and
  `_with_result` forms of add/update/switch return stable
  results before ProviderService or mutation hooks: the pure classifier returns
  specific typed unsupported for proxy takeover, official-target switch, or
  critical risk; supported normal-mode legacy writes return
  `change_plan_required`. Renderer and backend guards switch in the same commit.
  Prior routing remains only for non-Codex
  and separately named non-create/edit/switch Codex families: delete,
  import-default, live-remove, official-seed, proxy failover, and
  sort/last-used metadata.
- `prepare_provider_mutation` and `plan_codex_projection` are pure over injected
  snapshots. They cannot write DB/files, persist endpoints, invoke a process,
  refresh cache/tray, create backup/job/event, initiate business sync, or make a
  Provider/model request.
- Create policy is closed: `StoreOnly | MakeCurrent | LegacyIfNoCurrent`.
  Change Plan uses only the first two; legacy add alone may retain the third
  during migration.
- ProviderService is the sole writer. The outer `apply_prepared_change` receives
  exact admitted non-secret payload/CAS plus secret requirements and owns one
  Provider critical section. Inside it, ProviderService rechecks CAS, resolves
  exact ref/version into minimum-lifetime dependency-owned leases, wins the
  effect CAS/permit, and calls the private `commit_prepared_change` with explicit
  leases and the unforgeable permit. Neither seam reloads semantic payload by
  ID. Leases remain only in attempt memory and are zeroized on every exit;
  recovery stores ref/version or a #35-owned sealed artifact, never
  lease/plaintext.
- Change Plan uses the backend-private, schema-v1, deny-unknown
  `ProviderCredentialIntentV1 = None |
  Preserve{secretRef,expectedVersion} | Replace{secretRef,expectedVersion} |
  Clear`, canonicalized under
  `fyagent.change-plan.provider-credential-intent.v1`. Universal uses a distinct
  wire enum and no implicit cast/serde conversion exists.
- Provider row/endpoints, DB/device current, Codex catalog/auth/config, common
  config, managed MCP, and optional source backfill all participate in the shared
  mutation coordinator and the app-scoped, device-local Provider state epoch in
  `change_coordination`. Legacy/direct writers, official seed, endpoint commands,
  import, and restore must join or be disabled while a Plan/job owns the
  resources. Import/restore preserves local epoch and writes
  `max(local, restored)+1`; remote values never lower or replace it.
- Native tray, profile apply, provider deep link, old UCP executor, and public
  endpoint writers are part of the same Codex cutover inventory. Each guards
  before its first side effect; tray cannot clear proxy flags first, profile
  cannot autosave/disable proxy/toggle MCP first, and deep link cannot add a
  draft/endpoint first. Tray may only emit a safe exact-target request and focus
  the Plan UI; deep link forwards only allowlisted safe draft fields; profile
  returns an all-changes-unsaved outcome and waits for #41's UCP adapter.
- Public legacy ProviderService add/update/switch/add-draft/endpoint writers
  fail closed for a cut-over protected Codex operation even with no active Plan.
  Only the module-private commit seam with an unforgeable `EffectPermit` may
  perform it; test-only permit construction is not in production builds.
- Universal create/edit/duplicate/save/delete/sync uses one
  `mutate_universal_provider(UniversalMutationRequestV1)` backend operation.
  The request is a `deny_unknown_fields` closed enum: Create requires target ID,
  expected absence+epoch, safe proposed draft and sync flag; Edit requires ID,
  revision token, safe proposed draft and sync flag; Duplicate requires source
  ID/token, new ID/expected absence/epoch, safe proposed draft and sync flag;
  Delete and Sync require ID+revision token and forbid proposed payload.
  Invalid combinations fail before state access. Safe list/get views expose
  redacted draft, opaque backend-authored revision token, observed Provider
  epoch, and safe actual-child status; they never expose API key. TypeScript
  cannot compute a token. Stale returns `universal_revision_changed` plus fresh
  safe view with zero writes. Existing `get_universal_providers` and
  `get_universal_provider` command names return these safe views after cutover;
  no plaintext read IPC remains. Stored/DAO readers are module-private and their
  type is not serde-serializable.
- Legacy upsert/delete/sync write IPCs and public
  `ProviderService::upsert_universal/delete_universal/sync_universal_to_apps`
  are guarded to prevent a two-call or direct-service TOCTOU. Under the
  coordinator, the
  operation binds expected Universal redacted fingerprint, Provider state epoch,
  old/new membership, expected materialization, and actual
  `universal-codex-{id}` child presence/epoch/redacted definition digest. It
  never hashes plaintext API key; credential state is a safe code. Actual child
  presence always counts as Codex impact, even when membership is false.
  Blocked operations return `universal_codex_change_plan_unavailable` before the
  universal row, any per-app row, event, cache, epoch, or other-app write.
  Matching non-Codex-only operation receives a module-private, non-Clone,
  non-serde one-use permit bound to action, IDs, exact prepared payload digest,
  snapshot token, and epoch. Only
  `commit_universal_mutation(preparedExactMutation, secretLeases, permit)`
  writes; it consumes the permit by value and structurally omits Codex
  save/delete.
  AddProviderDialog/UniversalPanel create/edit/duplicate/save-and-sync/delete/
  manual-sync paths call only this compound command.
  New Codex membership is disabled; legacy Codex-linked rows are read-only;
  proven credential-free non-Codex-only Universal operations retain their path
  subject to the #35 gate below. Create/edit/duplicate/
  save may open a separate app-specific Codex Plan and cancel the Universal
  operation. Delete/remove/resync/manual-sync offers only cancel/return and
  adapter-required guidance; it cannot open an inapplicable Plan.
- #35 must provide an exact-SHA Universal credential adapter and migration before
  secret-bearing Universal mutation is enabled. The adapter inspects opaque
  binding token/ref/version, prepares reference-native Universal and child
  storage, and resolves minimum-lifetime leases after CAS but before permit.
  Resolver/migration/expiry failure is typed `dependency_unavailable/no_effect`
  with zero writes; the outer seam zeroizes leases on every exit. Before the
  handoff, legacy plaintext, Preserve/Replace, or sync needing a credential is
  disabled even for non-Codex. Only proven credential-free non-Codex with no
  Codex child and `UniversalCredentialIntentV1=None|Clear` may continue.
- The Universal wire enum is the closed, deny-unknown
  `UniversalCredentialIntentV1 = None | Clear |
  Preserve{opaqueBindingToken} | Replace{secretRef,expectedVersion}`, with
  canonical domain `fyagent.universal-credential-intent.v1`. Only the #35
  adapter maps it to an internal prepared requirement; mixed Provider/Universal
  fields and unknown schemas fail before state access.
- #35 persists a closed `UniversalCredentialStorageV1` discriminator in new
  reference-native storage, never inside legacy `api_key`. Its variants are
  `None`, `SecretRef{opaqueRef,expectedVersion,bindingKeyDigest}`, and
  `NeedsLocalRebind{credential_required,source,expectedBindingKeyDigest}`;
  `None` is proven
  credential-free, while rebind is required-but-absent. Safe view,
  revision/prepared digests, epoch/CAS, and fixtures bind the variant; only #35
  secure rebind may produce `SecretRef`.
  Binding domain `fyagent.universal-credential-binding.v1` canonically covers
  schema, Universal ID, primary slot, normalized provider type, and sorted pure-
  projection per-app auth scheme plus normalized endpoint. The constructor
  applies NFC/IDNA/effective-port/RFC3986 path preprocessing, then the existing
  v2 canonical UTF-8 JSON encoder and exact `domain || 0x00 || bytes` hash. The
  owning UCP spec freezes exact canonical bytes and digests for Unicode/default-
  port/dot/percent/trailing-path plus port-mismatch vectors. Transfer recomputes
  it; local storage/CAS use it; any version/field/digest mismatch is
  `NeedsLocalRebind`. ID/required alone is never sufficient, and TS treats the
  backend digest as opaque.
  The owning path order decodes unreserved `%2E` before dot removal, never
  decodes `%2F|%5C`, preserves repeated/trailing slashes, and freezes encoded-dot,
  encoded-slash, Unicode, repeated-slash and empty-path vectors.
  Migration reserves a fresh database `user_version`, converts/verifies all
  required legacy values, clears plaintext, and enables the new reader/writer
  only after its marker is committed. It remains disabled until an immutable
  `MIGRATION_GUARD_BASELINE_SHA` with safe `dbUpgrade` UX and the future-version
  preflight is released as minimum supported predecessor and used in acceptance.
  Exact source `ca552f4d` uses default SQLite open and can continue initialization
  on inspection error; it supplies only a successful-inspection pre-schema-write
  observation and is not the safe UX baseline. The accepted predecessor checks
  closed atomic `DbCompatibilityMarkerV1` under process-lifetime shared/
  migration-exclusive `DbCompatibilityLockV1`. Marker binds state, platform file
  identity, generation, application ID, observed/target/min reader versions,
  revision/migration ID, and checksum. Fresh bootstrap requires DB/marker/all
  sidecars absent. Marker-absent fallback parses the exact 100-byte main header
  only when WAL/SHM/hot journal are absent; all mismatch/corruption/permission/
  lock errors return `database_compatibility_unknown` without SQLite open,
  DB/WAL/SHM touch, or `Database::init`. #35 holds exclusive lock from pending
  marker through DB checkpoint/close and ready marker.
  The stable config-dir lock never follows DB inode. Runtime replacement enters
  maintenance, stops admissions/drains workers/sync/readers, closes all handles,
  releases shared, acquires exclusive, fully reinspects, replaces, publishes
  ready, then releases/reacquires shared, reinspects and reopens. No in-place
  upgrade or unchecked release/reacquire is allowed.
  Marker is a deny-unknown tagged union: BootstrapPending forbids observed DB
  fields; MigrationPending requires migration ID/file identity and observed/
  target generation/app-ID/user-version with target generation +1;
  ReplacementPending(CandidateApply) binds source artifact/candidate/generation/attempt/original
  request/expected candidate and main projection plus exact prior/target DB
  identities/generations/content digests; Ready requires identity/generation/FYAG
  `0x46594147` and tagged None|CandidateApply completion receipt while forbidding
  target/migration fields.
  Legacy app ID 0 is allowed only in marker-absent fallback/pending observed
  state; integer ranges and revision/generation monotonicity are closed.
  Compatible pending without a live exclusive owner is
  `database_compatibility_unknown(interrupted_bootstrap|interrupted_migration)`;
  lock timeout is `lock_busy`. Neither path auto-resumes or initializes.
  ReplacementPending is excluded from that generic branch. Before services,
  `DbReplacementRecoveryV1` first acquires the stable global
  `CredentialArtifactIntegrityLockV1`, without peeking or deriving authority
  from any source/candidate ID, then acquires compatibility exclusive. Under
  both locks it freshly enumerates and rereads the marker plus every observed
  source/candidate identity, and classifies exact prior as no-effect,
  exact target plus hook-free query-only projection as applied, and anything
  mixed/unreadable as needs-help while keeping normal DB closed. It never rebuilds,
  invokes #35, initializes, or replays. Ready retains completion identity until
  matching Applied or NeedsHelp(apply,observed_no_effect) persists a closed
  marker/sidecar acknowledgement; no new replacement may
  overwrite it. Missing/corrupt sidecar keeps normal DB/services closed and the
  receipt intact. Post-publish sidecar/ack failure is the distinct neutral
  `candidate_apply_authority_unavailable`, not pre-effect
  `credential_artifact_store_unavailable`; it says main may be prior or target
  and apply will not repeat, with help/repair/exit only.
  Migration cannot use an ordinary plaintext DB backup; it requires a #35-owned
  sealed local artifact or fails closed. Once marker/rows exist, downgrade is
  forbidden and rollback retains the safe parser/adapter, read/write guards,
  schema, and future-version gate. Sync/export/diagnostics/sanitized backup emit
  status only; remote import can commit safe fields plus
  `NeedsLocalRebind` and never overwrites a local ref.
- Safe dependency reasons are exactly
  `secret_backend_unavailable|credential_migration_required|
  credential_rebind_required`. Import/sanitized restore without a matching local
  binding persists `NeedsLocalRebind`; a later blocked Universal mutation returns
  `credential_rebind_required/no_effect`. UI enters only #35
  secure rebind, reloads the safe view after success, and never collects a secret
  in an ordinary Provider form. The future-version startup result projects to
  `database_upgrade_required` on stable `dbUpgrade`: four-locale accessible copy
  states the data requires a newer compatible FyAgent and no business data was
  initialized, migrated, or modified (only marker/header metadata was read), and
  allows only local upgrade guidance, an
  already-local verified compatible installer when available, or exit—never
  continue, config-folder mutation, downgrade, rollback, or restore.
- `UniversalCredentialTransferV1` owns every DB copy/replacement surface because
  Universal plaintext currently sits inside the `settings` blob. It parses the
  exact Universal settings value to safe non-secret fields plus
  `credentialRequirement=none|required`; malformed/unknown data fails closed,
  and no value/ref/binding token/lease/fingerprint is transferable.
  SQL import/export stages and migrates a temporary DB before replacement and
  never creates a raw safety backup. #35 allocates `DB_COMPAT_VERSION > 6` plus a
  new `db-vN` WebDAV/S3 layout with no automatic dual-read/write of `db-v6`;
  legacy remote data requires explicit staging migration or typed rejection.
  App-managed backup create/restore is sanitized and staged; pre-safe/unknown
  backups are inventoried as `legacy_credential_backup_blocked` and quarantined
  from ordinary restore/sync/export until safely remigrated or deleted. Candidate
  markers use `max(local,staged,required)`; local refs survive only a stable-ID +
  credential-requirement-digest match, otherwise `NeedsLocalRebind`.
  The persisted/read-back transfer outcome is closed as
  `committed|committed_rebind_required|migration_required{sql_import|webdav_v6|
  s3_v6|app_backup}|rejected{code}`. The renderer cannot collapse rebind or
  migration-required into `None`; blocked artifacts remain isolated and offer
  only #35 staged migration or an existing source-specific confirmed delete.
  `CredentialArtifactStoreV1` is one separate device-local SQLite sidecar with
  its own lock; main replacement never replaces it and sync/export/transfer/
  backup exclude it. Store failure is fail-closed. Records include opaque ID/
  revision, private binding/generation, attempt ID, owner epoch/lease, effect
  phase, and ordered effect steps with started timestamp, idempotency key, and
  private receipt. Safe list/read expose no private fields. Migrate/delete
  require ID+expectedRevision/binding/owner CAS. Lease takeover is pre-effect
  only; post-effect reconcile reads #35 attempt receipt, candidate manifest, or
  source binding and never reissues create/publish/delete. Determined no-effect
  and ambiguous/unavailable readback persist as needs-help with only local/manual
  or fenced readback-only recheck. Migration publishes a separate sanitized
  candidate and never applies/overwrites/deletes the original; candidate apply
  is a new explicit transfer, and delete-source exists only in confirmed delete.
  Source/candidate metadata cannot purge independently. One global-lock sidecar
  GC transaction may purge both only after source+candidate Deleted, both files
  absent, refs released/main-owned, no active/needs-help/DB completion receipt,
  and 30 days from the later terminal/action-receipt anchor. Only a source proven
  by persistent `candidateLineage=NeverPublished` and no candidate file/row/ref/
  action/effect receipt may use artifact-only terminal GC. Candidate publication
  atomically changes lineage once to
  `Published{candidateId,generation,publishAttemptId,privatePublishReceipt}` and
  source CandidateDeleted/Deleted retain it; Published+missing counterpart is
  corruption/needs-help.
  Source and candidate records carry closed pairIntegrity Intact/Inconsistent
  with safe missing/mismatch codes. Inconsistent overrides lifecycle UI, pins
  every survivor indefinitely, and permits only local help, exit, and safe
  reload—no recreate/remigrate/apply/delete/GC/retry. Public views expose no
  private lineage/receipt; actions reject typed `pair_integrity_inconsistent`.
  Closed errors
  include store/readback unavailability and pair integrity; unknown fails closed.
  Separate deny-unknown `CredentialArtifactLifecycleV1` covers Detected,
  MigrationRequired, PreEffect, Reconciling, NeedsHelp with three reasons,
  CandidateReady, CandidateDeleted, Deleted and Rejected; Reconciling/NeedsHelp
  retain `migrate|delete`. `CredentialArtifactActionOutcomeV1`
  covers Accepted/Rejected/Reconciling/NeedsHelp/CandidateReady/Deleted/
  StoreUnavailable and carries the command action where nonterminal. Exact revision-CAS transitions and safe-view fields make
  post-effect return-to-retry structurally impossible.
  Private `CandidateCredentialBindingV1` ties candidate ID/revision/generation/
  content and source revision to each Universal binding digest, SecretRef/version
  and creation receipt plus per-binding discard attempt/effect-start/status/
  private receipt. Unique `(candidateId,requestRevision)`
  `CandidateActionAttemptV1` persists immutable action/attempt/digest through
  PreEffect/EffectStarted/NeedsHelp/Terminal and retains a positive monotonic
  attempt revision, result revision, outcome, and exact safe snapshot. Closed
  `DbCompletionAckV1` binds sidecar attempt revision, marker revision,
  replacement/attempt, outcome, and observed DB generation and is immutable once
  written. Stable config-dir `CredentialArtifactIntegrityLockV1` is the exclusive
  outer lock for every artifact/candidate action, scanner, GC, and recovery; its
  identity never depends on disputed source/candidate links. Candidate apply uses
  integrity→maintenance→DB lock order,
  revalidates all bindings/gates, resolves leases, records effect-start and
  publishes reference-native rows once. Crash/duplicate apply is readback-only;
  candidate/ref pins have no pre-apply timed purge, source deletion is independent,
  and explicit candidate cleanup is idempotent or NeedsHelp.
  Candidate safe view/outcome are separate deny-unknown types covering Pinned/
  Applying{action,attemptId,priorMainDbGeneration?}/Applied/
  NeedsHelp{action,attemptId,priorMainDbGeneration?}/Deleted and Accepted/Rejected/Applying/NeedsHelp/
  Applied/Deleted/StoreUnavailable. Revision transitions prohibit post-effect
  return to Pinned; candidate NeedsHelp reason is exactly
  `observed_no_effect|ambiguous|readback_unavailable`. Determined no-effect copy
  says exact prior DB/candidate not applied/no replay; uncertain copy is reserved
  for ambiguous/unavailable.
  Recheck is action/reason/ack-specific: unacked apply under ReplacementPending
  may remain unresolved, resolve exact prior to acknowledged no-effect NeedsHelp,
  or exact target to acknowledged Applied; after no-effect receipt clear it can
  only self-loop and cannot inspect later main rows for attribution. Delete
  NeedsHelp may remain or resolve Deleted only. Later apply requires a new
  authorized attempt/marker, never ack rewrite.
  Backend-authoritative `list_credential_candidates` and
  `get_credential_candidate` enumerate safe candidate-only survivors after
  restart. Startup/list/get/action integrity preflight first acquires the global
  integrity lock, then freshly and completely enumerates IDs and rereads all
  observed identities under that lock. No pre-lock ID set may authorize or bound
  the scan. It may CAS only a newly detected sticky
  Inconsistent overlay/revision; persistence failure exposes zero actions. Safe
  authority-updated events only follow that commit and invalidate/refetch; pair
  Inconsistent forces zero actions and all public surfaces retain sentinels.
  Duplicate Applied is idempotent and Deleted records whether
  it had been applied. Attempt-ledger lookup precedes current-revision checks;
  exact same-action/original-revision retries return the active or persisted
  terminal snapshot while its result revision remains current; a later valid
  action returns `candidate_action_superseded` plus only the current safe view.
  Candidate delete response loss never replays
  cleanup/#35 discard. Action/revision mismatches are
  `candidate_action_conflict|candidate_revision_changed`. Successful candidate
  delete makes the source CandidateDeleted: it may be kept or explicitly
  source-deleted but never remigrated.
  Confirmed source delete is also legal while the candidate is Pinned or Applied,
  and leaves candidate/ref/main unchanged; candidate Applying/NeedsHelp blocks it.
  Candidate delete starts from Pinned or Applied and preserves the prior main
  generation through any delete needs-help/readback path.
  Applied UI exposes explicit delete-candidate; it removes candidate recovery
  material without rolling back main DB or deleting source. Superseded action
  copy says a newer action won/nothing repeated/current state shown and permits
  only dismiss/review plus current-view actions.
  Source/candidate actions share the global artifact-integrity lock and CAS all
  observed records. Candidate actions reject while source is PreEffect/Reconciling/
  NeedsHelp; source deletion already complete does not block candidate apply or
  delete and source bytes are never reread. Candidate delete records #35 discard
  effect-start before the call, uses attempt readback only after interruption,
  and publishes candidate terminal receipt plus source CandidateDeleted in one
  sidecar transaction; an already Deleted source remains Deleted.
  Candidate apply stages/closes a target DB, fsyncs ReplacementPending before its
  single publish, then Ready with completion receipt. Startup recovery handles
  pending/main/ready/sidecar crash boundaries by readback only. Matching Applied
  or NeedsHelp(apply,observed_no_effect) writes DbCompletionAck; ambiguous/
  unavailable/delete forbids it. Receipt clear requires exact Ready marker +
  sidecar attempt revision CAS; mismatch/store failure keeps admission closed.
- Every transfer computes `UniversalTransferCodexImpactSnapshotV1` across
  staged/local membership, actual child presence/epoch/redacted digest, and the
  projected child digest from staged safe fields. Child ID/provenance presence
  counts even when `apps.codex=false`. Before a Universal-to-UCP adapter, any
  membership/child/projected-digest difference quarantines the whole transfer as
  `universal_codex_transfer_unavailable` with zero main DB/child/epoch/marker/
  sync/cache/event effects. Allowed no-impact paths drop all staged Universal
  Codex child rows and preserve/reinject exact local membership/child. SQL,
  WebDAV/S3, current backup, and legacy backup share this gate.
- Endpoint editing remains draft-only until Provider apply. Speed tests may make
  an explicit user-requested probe but cannot persist endpoint rows.
- Concrete credentials and redacted Provider list/edit DTOs are owned by #35.
  Before its exact-SHA integration, only a switch whose target, source backfill,
  existing live auth, prepared projection, and recovery inputs are all proven
  credential-free may be enabled; plaintext/unknown credential state is not
  converted into a Plan or reloaded during apply.

## 3. Contracts

### Lossless TOML and native capabilities

- Every Codex Provider exposes image-extension and WebSocket controls in the
  existing, initially collapsed advanced region. Provider ID, `base_url`,
  credentials, official/managed classification, OAuth type, proxy takeover,
  `wire_api`, and `meta.apiFormat` do not make a valid TOML draft ineligible.
- A fixed official Provider is identified only by `category == "official"` or
  ID `codex-official`. Names, URLs, and `requires_openai_auth` are not
  classifiers.
- Analysis and patching use `toml_edit` and preserve comments, blank lines,
  table and field order, unrelated fields, and unrelated headers. An invalid
  complete TOML document keeps both controls visible but disabled and blocks
  capability writes; it is never reconstructed from parsed form state.
- An invalid `http_headers` or `supports_websockets` field is a non-blocking,
  non-sensitive diagnostic. Ordinary saves preserve the invalid field. Only an
  explicit operation on the corresponding control may repair it.
- The image capability owns only the case-insensitive
  `x-openai-actor-authorization` header whose value is exactly
  `local-image-extension`. Enabling removes every case variant and writes one
  canonical key. Disabling removes every variant and then removes an empty
  header table. Other valid header entries survive.
- If `http_headers` is not a string map, explicit image enable replaces that
  field with the managed map and explicit disable deletes it. No unrelated save
  performs this repair.
- WebSocket configuration is format-agnostic. Enabling always writes boolean
  `supports_websockets = true`; disabling removes the field rather than writing
  `false`. Responses, Chat, Anthropic, managed OAuth, official, and proxy
  Providers remain saveable with the field present.

### Migration metadata and official-provider ownership

- `ProviderMeta.imageExtensionConfigured` is migration-only private metadata.
  For a non-official Provider, missing metadata plus no managed/conflicting
  header is a legacy pending-on draft; no bulk migration writes live TOML.
  The first successful new-provider save or explicit historical choice marks
  the row configured. Displayed state still derives from TOML.
- A fixed official Provider defaults both native capabilities off. Merely
  opening or saving it creates no Provider table.
- The first actual enable creates `model_provider = "custom"` and a minimal
  table with `name = "OpenAI"`, `requires_openai_auth = true`, and
  `wire_api = "responses"` when no suitable table exists.
- `ProviderMeta.codexNativeCapabilitiesGeneratedProvider` claims ownership only
  when the capability operation created that table. A pre-existing inactive
  `custom` table may be reused but is never claimed.
- When both controls are off, an owned table is removed only if it still has
  the exact managed shape and no user fields. Otherwise only capability-owned
  fields are removed. An explicit Provider table takes precedence over unified
  Codex session-history injection.

### Vendor projection and safe session resume

- A native Responses Provider receives a vendor model catalog only when the
  active `base_url` parses as HTTPS to a reviewed hostname. The DeepSeek rule
  permits exactly `deepseek.com` and its dot-delimited subdomains.
- Scheme, hostname, and authority are parsed structurally. Substrings, paths,
  or user information such as `deepseek.com.evil.example`,
  `notdeepseek.com`, or `deepseek.com@evil.example` retain the neutral native
  template and receive no vendor harness instructions.
- Session resume crosses a shell-command boundary. Every persisted session ID
  passes the shared fail-closed validator before command construction. It must
  be nonempty ASCII; its first character is alphanumeric or `_`, and every
  remaining character is alphanumeric, `_`, `-`, or `.`.
- An unsafe ID remains visible in session history but has no `resumeCommand`.
  Do not quote or escape it into a shell string. A wider future grammar requires
  typed argv plus platform-specific launch/copy handling.

### Warnings, proxy projection, and live mutation evidence

- Warning codes are computed from the final saved Provider only when
  WebSocket is `true`. Inspect nonempty top-level `model`, `review_model`, and
  `modelCatalog.models[].model`; use the segment after the final `/` and accept
  an ASCII case-insensitive `gpt-` prefix. Any recognizable non-GPT model emits
  the model warning; no recognizable models do not.
- Active Codex proxy takeover adds the proxy warning. Warnings are omitted for
  switches, failed saves, and empty-risk results. They communicate configuration
  risk, not a transport failure or a claim that the local HTTP/SSE proxy
  supports WebSocket Upgrade.
- Normal and official proxy projections preserve explicit WebSocket state and
  the managed image header while continuing to rewrite routing `base_url` and
  `wire_api` under the proxy contract.
- `liveConfigChanged` is `true` only when a successful operation changes the
  final bytes of the current interactive user's `~/.codex/config.toml`.
  It contains no bytes, digest, path, or credential. Non-Codex mutations return
  `false`. The renderer may use the flag to offer the trusted restart flow from
  [Codex Desktop Installer](./codex-desktop-installer.md), but saving and
  restarting remain separate outcomes.

## 4. Validation & Error Matrix

| Condition                                                                                            | Required result                                                                                                         |
| ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| A non-Codex app calls a native-feature command                                                       | Reject before TOML analysis or mutation.                                                                                |
| The complete Codex TOML document is invalid                                                          | Keep controls visible but disabled; never reconstruct or overwrite the document.                                        |
| A managed header or WebSocket field has an invalid shape                                             | Preserve it on unrelated saves; show a non-sensitive diagnostic; repair only on an explicit matching control operation. |
| Chat, Anthropic, official, managed, or proxied Provider saves with WebSocket enabled                 | Save successfully and preserve the explicit choice; add applicable warning codes without rewriting it.                  |
| An official Provider has empty TOML and both capabilities remain off                                 | Preserve empty TOML and create no table or ownership metadata.                                                          |
| A persisted session ID fails the conservative ASCII grammar                                          | Omit `resumeCommand`; never interpolate the raw ID into a shell command.                                                |
| A DeepSeek-looking URL has HTTP, user information, a suffix-confusion hostname, or only a path match | Use the neutral template; grant no vendor behavior.                                                                     |
| A mutation succeeds but final live Codex bytes do not change                                         | Return `liveConfigChanged: false`; do not offer an automatic restart.                                                   |
| Validation, CAS, secret resolution, or cancellation fails before effect permit                       | Preserve prior live bytes; report typed no-effect; omit risk/restart success signals.                                   |
| A mutation fails after effect permit / first durable effect                                           | Perform authoritative per-resource readback; preserve actual observed state; mixed/partial is `manual_required`; never auto-restore/replay or emit success/restart signals. |

## 5. Good / Base / Bad Cases

- Good: explicit image enable normalizes only the managed header while
  preserving comments, custom headers, table order, and unrelated Provider
  fields.
- Base: a valid Provider contains no recognizable models. WebSocket remains
  enabled, the save succeeds, and no non-GPT warning is invented.
- Good: `https://api.deepseek.com/v1` matches the reviewed hostname rule;
  `https://deepseek.com.evil.example/v1` does not.
- Bad: derive official-provider identity from display name, rewrite invalid TOML
  from form state, use proxy preservation as proof of WebSocket transport, or
  quote an unsafe persisted session ID into a command string.

## 6. Tests Required

- Rust/TOML fixtures cover lossless unrelated edits, complete-document failure,
  invalid field shapes, case-variant header normalization, empty-table cleanup,
  WebSocket enable/remove, and official minimal-table ownership/cleanup.
- Migration fixtures cover pending legacy rows, explicit choices, newly created
  Providers, reused unowned tables, and exact owned-shape retirement.
- Hostname fixtures cover the approved HTTPS host and subdomains plus scheme,
  user-info, substring, suffix, and path-confusion rejections.
- Session fixtures cover ordinary UUID/provider-prefixed IDs and every rejected
  empty, leading-hyphen, non-ASCII, whitespace, quote, separator, and control
  character class.
- Result tests cover byte-exact `liveConfigChanged`, non-Codex false results,
  warning ordering/deduplication, GPT/non-GPT catalogs, proxy warnings, switches,
  pre-effect failed saves, and each post-effect partial-write/readback outcome.
  Renderer tests prove only successful changed Codex saves can offer the
  separate trusted restart flow; partial/manual-required outcomes cannot.
- Prepared-mutation tests prove preview has zero DB/current/live/common/MCP,
  endpoint, backup, cache/tray, sync, process, and Provider/model network effects;
  apply receives the exact admitted payload, passes all resource CAS, and calls
  the writer at most once.
- Secret/effect tests prove the outer seam resolves exact ref/version and minimum
  lifetime only after resource CAS, resolve failure creates no effect permit,
  the private commit seam requires explicit leases plus permit, and every exit
  zeroizes leases. Neither seam may persist/log a lease or recover plaintext.
- Entry tests cover `StoreOnly`, `MakeCurrent`, current/non-current edit,
  draft-only endpoints, credential-free switch, dependency-unavailable secret
  state, and no protected direct-command fallback.
- Cutover tests invoke all six add/update/switch Tauri commands and every native
  tray/profile/deep-link/old-UCP/endpoint entry independently. Each protected
  supported normal-mode Codex legacy call returns/routes `change_plan_required`
  before side effects, while proxy/official/critical cases return their specific
  typed unsupported result first; tray
  proxy/provider/writer/effect and profile autosave/proxy/MCP/profile counters
  remain zero. A static registration/callsite scan has no unclassified protected
  direct writer.
- Universal matrix tests cover before/after Codex membership and projection
  change, `apps.codex=false + existing child`, stale revision/epoch, interrupted
  legacy two-step, and preflight race across save/delete/sync plus UI create/
  edit/duplicate/save-and-sync/manual-sync. Blocked whole operations keep
  universal/per-app/event/cache/epoch and other-app counters at zero. A no-child
  credential-free non-Codex-only `None|Clear` control stays green and proves the
  private commit never calls Codex save/delete.
- Contract tests cover every closed request variant and forbidden-field
  combination, backend safe-view/token sourcing, stale token/epoch/absence, and
  no TypeScript digest authority. Direct calls to all three legacy public Service
  writers return `universal_mutation_v2_required`. Visibility/compile-fail and
  runtime tests cover permit forgery, clone/serde, reuse, wrong action/ID/payload,
  and static callsites; production code outside the module cannot construct or
  reach the private commit.
- Read-surface sentinels prove command registration, IPC, TS query/cache, event,
  DOM, log, and diagnostics contain no Universal API key; raw stored readers are
  private/non-serde. Credential tests cover None/Clear/Preserve/Replace, legacy
  migration, exact ref/version, lease expiry/resolve failure, reference-native
  persistence, attempt zeroization, and pre-#35 credential-free-only gating.
- Credential fixtures cover both separately named v1 intent enums, all variants,
  distinct canonical domains, deny-unknown/schema-first decoding, and illegal
  cross-domain payloads. Storage migration fixtures prove exact
  `MIGRATION_GUARD_BASELINE_SHA` returns `db_version_too_new` after version
  inspection but before DDL/business-read/write/sync/network, ref/binding
  tokens never become API keys, ordinary plaintext backups are not created,
  downgrade is blocked, and sync/export/diagnostics/sanitized backup/import use
  only the safe/local projection.
- UI fixtures cover all three dependency reason codes, retained-binding versus
  persisted `NeedsLocalRebind`, later `credential_rebind_required/no_effect`,
  #35-only secure rebind followed by safe-view
  reload, and absence of ref/token/value sentinels. The stable `dbUpgrade`
  recovery surface is complete in four locales, keyboard/screen-reader
  accessible, has no continue/downgrade/rollback/restore action, and has zero
  pre-DDL/business-read/DAO/service/write/sync/network counters.
- Copy/replacement fixtures parse the Universal `settings` value and cover SQL,
  WebDAV/S3 new generation/layout, current app-managed backup, and quarantined
  pre-safe backups. They assert temporary staging, no `db-v6` dual-write, safe
  transfer only, monotonic marker, stable-ID+requirement-digest local-ref merge,
  exact `None`/`NeedsLocalRebind`, no raw safety backup, and unchanged main DB on
  failure.
- Transfer-outcome fixtures persist/read back every committed/rebind/migration-
  required/rejected variant and artifact kind. Four-locale accessible UI keeps
  safe-import rebind distinct from later mutation `/no_effect`; blocked artifacts
  expose no secret and allow no automatic delete/raw restore/silent import.
- Four-locale accessibility/reload fixtures cover compatibility interrupted/
  unknown/lock-busy plus artifact store-unavailable/reconciling/observed-no-
  effect/ambiguous/readback-unavailable. No post-effect migrate/delete/retry is
  rendered; public surfaces omit path/URL/ETag/digest/receipt/ref/value. Separate
  candidate publication preserves original source/record until confirmed delete.
- Maintenance plus every public artifact/candidate lifecycle and action-outcome
  projection are exhaustive in four locales with a11y, reload, allowed-action,
  and sentinel coverage. This includes action-specific preparing/reconciling,
  artifact/candidate deleted and rejected/store-unavailable, candidate
  applying/deleting/needs-help, applied, and both `wasApplied` deleted cases;
  Detected remains internal-only. There is no duplicate submit or post-effect
  replay; observed-no-effect versus uncertain needs-help copy is distinct;
  Applied exposes candidate delete without source/main rollback; source/main
  effects are stated exactly, and response-loss candidate
  delete returns its persisted receipt without cleanup/discard replay. Deleted
  views expose no SecretRef/version/receipt/private binding/locator/digest/value.
  Source-delete fixtures cover candidate Pinned/Applied/Deleted with unchanged
  candidate/ref/main state after reload, plus Applying/NeedsHelp rejection with
  zero effect.
  Source-delete versus candidate apply/delete races prove a single global-lock/
  CAS winner, no deadlock/crossed effect, source-active rejection, discard
  receipt readback without replay, and atomic candidate/source terminal publish.
  Candidate apply pauses after ReplacementPending/main publish/Ready/sidecar;
  pre-service recovery proves exact-prior, exact-target, and ambiguous outcomes
  with no replay/services/#35. Both exact branches cover Ready-before-ack and
  ack-before-clear; ack/marker mismatch and sidecar unavailable never clear or
  admit. Unresolved apply recheck resolves only exact-target Applied or exact-
  prior acknowledged NeedsHelp; after receipt clear the latter self-loops despite
  unrelated target-like DB changes and ack bytes never change. Delete recheck
  resolves only Deleted. Original-request retry remains exact while current;
  after a later action it returns superseded plus current safe view. Candidate
  list/get/invalidate-refetch fixtures rediscover source-missing survivors after
  restart and stale-cache mutation attempts remain zero-effect. Integrity-scanner
  startup/list/get/action races prove global-lock CAS, including source-A/
  candidate-C/source-B split identity, overlay-only revision/event,
  zero file/ref/attempt/main/lifecycle effects, and fail-closed persistence.
  Static owning-spec assertions reject the removed `peek source ID`,
  relationship-derived recovery-lock, and enumerate-before-global-lock
  sequences, including `source-artifact action lock` and equivalent
  relationship-selected authority, so no narrower text can reopen the
  split-lock or stale-scan path.
  >30-day
  source-deleted+pinned/applied and live-source+applied pairs remain pinned; only
  paired both-Deleted/no-dependency joint GC purges both. NeverPublished-only
  source GC, publish-boundary crash, Published+missing counterpart, paired
  control, and concurrent GC/action fixtures freeze lineage authority.
  Superseded copy/current-view-only actions have four-locale/a11y/reload/sentinel
  fixtures and never render historical controls.
  Post-publish candidate authority unavailable and pair-integrity inconsistent
  each have distinct four-locale/a11y/reload/private-sentinel fixtures, neutral
  effect truth, only their bounded help/exit/reload actions, and zero replay/
  recreate/remigrate/delete/GC effects. Pre-effect StoreUnavailable alone claims
  unchanged main/source.
- Guard fixtures cover all-absent fresh bootstrap, bootstrap/pending/ready marker,
  missing-marker header fallback, WAL-only version, hot journal, change-counter,
  identity/generation/lock and concurrent migrator cases, with no SQLite open or
  DB/WAL/SHM/journal touch and no `Database::init` on rejection. Binding vectors vary every canonical scope
  field and require `NeedsLocalRebind` on mismatch.
  Maintenance-transition fixtures cover drain/close/release-shared/acquire-
  exclusive/reinspect/replace/ready/reacquire-shared/reopen with competing
  reader/migrator and no lock upgrade/deadlock/TOCTOU. Tagged marker fixtures
  reject illegal fields, ranges, generations, and application IDs.
- Artifact tests cover multiple same-kind IDs, revision/content/manifest/ETag
  CAS, sidecar failure, owner epoch/dual owner/pre-effect takeover, delete/
  SecretRef/candidate post-effect crashes, readback-only no-replay, main
  replacement survival, source retention/explicit delete, closed errors,
  retention, and local-only exclusion. Transfer Codex-impact tests cover
  every copy family, membership=false+staged child, local orphan, create/update/
  delete/projection differences, whole-transfer zero-write quarantine, staged
  child exclusion, exact local preservation, and no-impact control.
- Artifact lifecycle/outcome decoders and revision transitions are exhaustive;
  illegal state/step/receipt combinations fail closed. Candidate tests cover the
  private ref-receipt handoff, integrity→maintenance→DB lock order, crash/
  duplicate/drift/source-deleted/sidecar-unavailable apply, pins/retention,
  explicit cleanup, and no replay. Path vectors cover `%2e` ordering, encoded
  slash/backslash, repeated/trailing slash, Unicode, and empty path.

## 7. Wrong vs Correct

Wrong:

```text
provider URL contains "deepseek.com" -> enable vendor behavior
session resume = "codex resume '" + persistedId + "'"
save succeeded -> liveConfigChanged = true
```

Correct:

```text
parsed HTTPS hostname matches reviewed host rule -> vendor behavior
persisted ID passes conservative ASCII grammar -> construct established command
successful final live bytes differ -> liveConfigChanged = true
```
