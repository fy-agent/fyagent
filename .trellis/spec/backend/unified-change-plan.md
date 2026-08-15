# Unified Change Plan v2 contract

## 1. Scope / trigger

Read this contract before changing Change Plan tables, canonical digests,
admission, worker ownership, Provider mutation coordination, job snapshots and
events, reconciliation, Tauri commands, TypeScript decoders/queries, or a
protected production entry.

This module owns the single Plan/job/event ledger and one-confirmation handshake.
Operation domains extend it through a closed enum variant and domain adapter.
They must not create a second ledger, lifecycle, worker, admission path, or
confirmation flow. The first protected domain is Codex Provider
create-only/create-and-select/edit/switch. WorkBuddy and model/network probes are
outside it.

## 2. Command and authority surface

```text
get_change_plan_capabilities() -> ChangePlanCapabilities
create_change_plan(request) -> PlanCreationOutcome
get_change_plan(planId) -> PlanPublicProjection
find_latest_change_plan(operationScope) -> PlanPublicProjection?
abandon_change_plan(planId, expectedRevision) -> PlanLifecycleOutcome
find_latest_change_job(operationScope) -> ChangeJobSnapshot?
apply_change_plan(planId, planDigest) -> AdmissionOutcome
cancel_change_job(jobId) -> ChangeJobSnapshot
get_change_job(jobId) -> ChangeJobSnapshot
list_recoverable_change_jobs() -> ChangeJobSnapshot[]
recheck_change_recovery(jobId, expectedRevision) -> ChangeJobSnapshot
purge_change_history(request) -> PurgeOutcome
change-job://updated -> { jobId, eventSeq }
```

Backend snapshots are authority. Events only invalidate/refetch. Renderer state
is a view projection and never authorizes execution or reconstructs a job state
machine.

## 3. Plan contract

- Preview may persist only the immutable Plan row. It cannot mutate a Provider,
  current selection, live/common/MCP files, endpoint rows, backup, job/event,
  tray/cache, external sync, process, or Provider/model network request.
- Public projection and private execution envelope are separate typed DTOs.
  Public data has safe codes/labels only. Content fingerprints, CAS tokens,
  exact non-secret payload, absolute paths, and opaque secretRef identity/version
  remain backend-only. Neither DTO holds secret values.
- Canonical contract is `fyagent.change-plan.v2`. Rust constructs closed typed
  intent, baseline, and Plan inputs; SHA-256 uses distinct domains. Admission
  reconstructs all three inputs from the private envelope and compares digests.
  TypeScript never authorizes by recomputing them.
- Arrays with semantic order preserve it; declared sets sort by stable key and
  reject duplicate keys. Canonical values allow strings, booleans, null, and
  signed 64-bit integers only. Optional fields emit explicit null. Unicode is not
  normalized. Dynamic non-secret JSON is recursive; Codex TOML is typed before
  canonicalization; live baselines use exact-byte hashes.
- Shared language-neutral vectors include canonical bytes and three digests for
  create-only, create-and-select, edit, and switch.
- `planId` is unique. `planDigest` is stable for equivalent intent/baseline and
  excludes ID, timestamps/expiry, actor, locale/display text, lifecycle, job,
  and presentation order. Admission requires exact Plan ID plus digest once.
- Latest Plan discovery is a pure read scoped by safe
  `{app,operation,subjectId}`; create uses its preallocated Provider ID.
  Abandon uses an injected clock and CASes only matching-revision v2 `ready`
  with no owning job and `expires_at>now` to `abandoned`, sets `abandoned_at`,
  and increments revision. At `expires_at<=now`, the same transaction persists
  typed `expired` and never writes `abandoned_at`. It creates no
  job/event/writer/backup/tray/cache/sync/managed effect.

## 4. Admission, worker, cancel, and reconciliation

- Digest mismatch, expiry, replay, source/target/baseline/secretRef metadata or
  precondition drift, unsupported capability, or dependency-metadata absence at
  admission creates no job and calls no writer. The user must re-preview.
- Admission consumes the ready v2 lifecycle and creates exactly one unowned
  planned job in one SQLite transaction, then returns that planned snapshot.
- The native worker CAS-claims a job with owner instance, worker epoch, phase,
  revision, event sequence, status, effect marker, and cancel state. Every later
  transition CASes that tuple, increments revision/eventSeq, updates snapshot,
  and appends one event atomically.
- `get_change_job` is always a pure read and never steals or reconciles an active
  owner. Startup/supervisor recovery must first prove a job orphaned.
- Orphaned work with no effect becomes terminal
  `interrupted_before_effect/no_effect`. Effect-started work is readback-only
  reconciled at a new fenced epoch. No recovery path replays a writer.
- Cancel and effect start arbitrate the same pre-effect CAS. Only one wins.
  After effect start, cancel is `too_late`.
- The worker passes only stored secret requirements and an effect-gate handle to
  outer ProviderService. In one Provider-owned coordinator critical section,
  ProviderService rechecks resources/source/preconditions/readability, resolves a minimum-lifetime #35
  lease for exact ref/version, wins effect CAS/permit, calls its private commit,
  and zeroizes on every exit. The worker never observes/passes a lease. Resolve
  failure terminalizes the already-created job as
  `failed + dependency_unavailable + no_effect + recovery=none`; the Plan stays
  consumed and writer/backup/managed-write counters stay zero. A lease is held
  only inside ProviderService attempt memory and never persisted/logged.
- Any stored-digest, resource/source CAS, precondition, or required-readability
  failure discovered after admission and before effect terminalizes the existing
  job as `failed + pre_effect_validation_failed + typed reasons + no_effect +
  recovery=none`. The Plan remains consumed and writer/backup/managed-write
  counters remain zero; this is not an admission rejection.
- ProviderService consumes the admitted exact payload and expected CAS under the
  shared Provider mutation coordinator. It does not reload semantics by ID.
  Mutation return is not success evidence; required resource readback is.

## 5. Resource and writer ownership

The operation adapter declares every Provider row/endpoint, DB/device current,
Codex catalog/auth/config, common config, managed MCP, source backfill, and proxy
precondition it will read or write. It freezes action order, reader, all managed
writers, baseline/CAS, readback criticality, recovery, and sync disposition.

All FyAgent-managed writers of those resources share the mutation coordinator
and increment an app-scoped Provider state epoch in the device-local
`change_coordination` table, including legacy writers during migration, official
seed, endpoint commands, settings current, import, and DB restore. Preview,
admission, and effect gate read that same SSOT. Import/restore preserves the
local epoch and writes `max(local, restored)+1`; a remote epoch is ignored.
External file writers cannot join; effect-gate and post-readback detection are
required, not an absolute zero-window claim.

The first v2 slice has no automatic inverse, compensate, or backup restore.
Recovery modes are only `none` or `manual_required`. Backups support displayed
manual hints. `recheck_change_recovery` uses owner/revision fencing and readback
only; a safe result may release sync quarantine, but it never calls a writer.
Unknown/unavailable required readback is never green and never causes retry.

## 6. Secrets and capabilities

#35 owns concrete SecretBackend, resolver lease, migration, and redacted
list/edit DTOs. This module consumes its immutable exact-SHA contract through an
opaque port; it never invents plaintext fallback or another credential store.

The backend-private, deny-unknown `ProviderCredentialIntentV1` is exactly
`None | Preserve{secretRef,expectedVersion} |
Replace{secretRef,expectedVersion} | Clear`. Its canonical domain is
`fyagent.change-plan.provider-credential-intent.v1`; tag/ref/version participate
in the private intent/Plan digest and never cross the public projection. It is
not the Universal wire enum, and no implicit serde conversion is allowed.

Generic commands remain registered. Capability discovery and request rejection
use one typed meaning. Before #35, only a switch whose target, source backfill,
existing live auth, prepared projection, and recovery inputs are all proven
credential-free may be enabled. Legacy plaintext/unknown credential state and
secret-bearing switch/create/edit return `dependency_unavailable` before a ready
Plan or job. Recovery envelopes contain only ref/version or #35-owned sealed
artifacts, never lease/plaintext. Protected UI entries never fall back to direct
mutation.

## 7. Persistence, local-only data, and retention

Schema-v16 initialization remains additive and idempotent; it does not claim
v17. New nullable columns distinguish v1 (`schema_version IS NULL`) from v2
(`schema_version=2`). V2 rows write inert legacy `status='consumed'` plus an
unknown operation sentinel so an old binary fails closed.

The Plan/job/event and `change_coordination` tables are device-local. WebDAV/S3
sync skips and locally preserves them; remote import cannot overwrite them.
SQL/diagnostic exports omit them. App-managed DB backups are sanitized before
publication. Provider effects suppress/coalesce business sync; safe read-backed
terminal state enqueues once, no-effect enqueues none, and
partial/recovery-required state quarantines sync.

Ready-expired, abandoned, and invalidated Plans are purgeable 24 hours from their
explicit anchors. Terminal job and owning Plan are purgeable 30 days only from
job `terminal_at`. Nonterminal/recovery-required evidence is not timed-purged.

## 8. All-entry protected-operation cutover

Renderer, native entry points, and writer authority cut over in the same commit
per Codex operation. All six Tauri commands—`add_provider`,
`add_provider_with_result`, `update_provider`, `update_provider_with_result`,
`switch_provider`, and `switch_provider_with_result`—use one pure precedence
before ProviderService or mutation hooks after cutover: specific typed
unsupported for proxy takeover/official-target switch/critical risk; otherwise
`change_plan_required` for a supported normal-mode legacy write.

The inventory also includes native tray switch, profile apply, provider deep
link, old UCP executor, and public endpoint create/edit writers. Tray routes a
safe switch request to the Plan UI before proxy flags/menu/provider writes.
Profile apply rejects a Codex Provider delta before autosave/proxy/MCP/profile or
Provider effects until #41 supplies its UCP adapter, and UI says the whole apply
was unsaved. Codex deep link routes draft ID and allowlisted safe fields—but no
secrets—before provider/endpoint persistence. Endpoint form edits are draft-only.

Public legacy ProviderService methods fail closed for cut-over protected Codex
operations even without an active Plan/job. Only module-private prepared commit
with an unforgeable `EffectPermit` may write. A compatibility wrapper may create
a Plan only; it cannot call a direct writer. All Codex create/edit/switch
subcases, including proxy takeover, official-target switch, and critical risk,
are protected and typed fail-closed. Prior routing remains only for non-Codex and
separately named non-create/edit/switch Codex families: delete, import-default,
live-remove, official-seed, proxy failover, and sort/last-used;
they still join coordinator/epoch.

Universal save/delete/sync uses one coordinator-owned
`mutate_universal_provider` command with a closed Create/Edit/Duplicate/Delete/
Sync enum that structurally requires IDs, expected absence/opaque revision token,
Provider epoch, safe proposed draft, and sync flag only on valid variants.
The existing `get_universal_providers` and `get_universal_provider` command
names return only `UniversalProviderMutationViewV1`; those backend-authored safe
views own the opaque token/epoch and redact credentials, and TypeScript never
recomputes them. No plaintext Universal read command remains registered. Raw
DAO/service readers return a non-serde `StoredUniversalProvider`, remain
module-private, and cannot cross IPC/query cache/events/DOM/logs/diagnostics.
Legacy upsert/delete/sync IPCs and all three public ProviderService writers
return `universal_mutation_v2_required`.
Its revision-bound impact snapshot includes Universal redacted
fingerprint, Provider epoch, old/new membership, expected materialization, and
actual `universal-codex-{id}` presence/epoch/redacted digest without hashing
plaintext. Actual child presence always blocks, including membership=false.
Blocked whole operations precede any universal/per-app/event/cache/epoch/
other-app write; allowed no-child non-Codex commit structurally omits Codex
save/delete and calls only module-private commit with a non-Clone/non-serde
one-use permit bound to action, IDs, exact payload digest, snapshot token, and
epoch. Stale/invalid requests are typed zero-write. Deep
link routing follows the closed `CodexDeepLinkPlanDraftV1` and rejection table in
`deeplink-import-security.md`; it never calls add-draft/endpoint/switch.
Universal create/edit/duplicate/save may instead open a separate app-specific
Codex Plan while cancelling the Universal operation; delete/remove/resync/
manual-sync may not. Deep-link safe URLs reject userinfo/query/fragment.

Secret-bearing Universal mutation additionally depends on a #35 exact-SHA
credential adapter and migration. Its closed, deny-unknown
`UniversalCredentialIntentV1` is `None | Clear |
Preserve{opaqueBindingToken} | Replace{secretRef,expectedVersion}`. It uses the
separate canonical domain `fyagent.universal-credential-intent.v1`; only #35 may
map it to an internal prepared requirement, and mixed/unknown Provider/Universal
tags fail before state access.
The adapter inspects opaque reference metadata and prepares reference-native
Universal and projected-child storage; Universal and Claude/Gemini/Codex child
rows never persist plaintext credentials after cutover. After resource CAS and
immediately before the permit/effect boundary, the coordinator resolves exact
minimum-lifetime leases and passes them by value only to
`commit_universal_mutation(preparedExactMutation, secretLeases, permit)`.
Resolver, migration, version, or lifetime failure returns typed
`dependency_unavailable/no_effect` with zero Universal/child/event/cache/epoch/
other-app writes; leases are attempt-only and zeroized. Before that exact #35
handoff and migration, legacy plaintext, `Preserve`, `Replace`, or any sync that
requires a credential is disabled even for non-Codex apps. Only a proven
credential-free non-Codex operation with no actual Codex child and
`UniversalCredentialIntentV1=None|Clear` may continue.

#35 persists only the closed `UniversalCredentialStorageV1 =
None{schemaVersion=1} |
SecretRef{schemaVersion=1,opaqueRef,expectedVersion,bindingKeyDigest} |
NeedsLocalRebind{schemaVersion=1,requirementCode=credential_required,
source=remote_import|sanitized_restore|legacy_staging,
expectedBindingKeyDigest}` in new storage; it never
puts a ref/binding token into legacy `api_key`. `None` proves credential-free;
the rebind variant proves a credential is required but absent. Safe view,
revision/prepared digests, epoch/CAS, and fixtures bind the discriminator; only
#35 secure rebind changes it to `SecretRef`.

The binding digest domain `fyagent.universal-credential-binding.v1` covers
schema, Universal ID, `primary_api_key` slot, normalized provider type, and
sorted per-app `{app,authSchemeCode,scheme,IDNA-host,effectivePort,
normalizedBasePath}` derived from the pure child projection. The constructor
applies NFC/closed-code/IDNA2008/effective-port/RFC3986 path preprocessing and
rejects userinfo/query/fragment/unknowns; the existing v2 canonical UTF-8 JSON
encoder then performs no implicit Unicode normalization. Hash is exactly
`SHA-256(domain || 0x00 || canonicalJsonBytes)`. Transfer recomputes it; local
storage and CAS use it. Any version/field/vector/digest mismatch is
`NeedsLocalRebind`; ID or `required` alone never preserves a ref. TypeScript
treats backend authority as opaque.

Path order is normative: reject userinfo/query/fragment/bad escapes; normalize
literal Unicode to NFC and uppercase-percent-encode its UTF-8; uppercase existing
escapes; decode only unreserved bytes including `%2E`; then remove literal dot
segments. Encoded slash/backslash stay `%2F|%5C`; repeated slashes and trailing
slash are preserved; empty becomes `/`. Vectors are
`/a/%2e/b/%2e%2e/c/ -> /a/c/`, `/a/%2f/b/%5c -> /a/%2F/b/%5C`,
`/a//b -> /a//b`, `/cafe\u0301/ -> /caf%C3%A9/`, `/a/b/.. -> /a/`,
and empty -> `/`.

Normative vector A canonical bytes (decomposed `cafe\u0301`, Unicode host,
default HTTPS port, and dot/percent/trailing path are already preprocessed):

```text
{"consumerDestinations":[{"app":"claude","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"xn--bcher-kva.example","normalizedBasePath":"/a/b/","scheme":"https"}},{"app":"codex","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"api.example.com","normalizedBasePath":"/v1","scheme":"https"}}],"credentialSlot":"primary_api_key","providerType":"custom","schemaVersion":1,"universalId":"café"}
sha256:9f537327bab07e3a8834832fe24d439222b1d91dbf170e000899c273d8452d51
```

Vector B changes only Codex port to 8443:

```text
{"consumerDestinations":[{"app":"claude","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"xn--bcher-kva.example","normalizedBasePath":"/a/b/","scheme":"https"}},{"app":"codex","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":8443,"host":"api.example.com","normalizedBasePath":"/v1","scheme":"https"}}],"credentialSlot":"primary_api_key","providerType":"custom","schemaVersion":1,"universalId":"café"}
sha256:2142b5f03e1d35ffe5f7b8df800d9ab88e38469833267c8ff8312b79400162d3
```

Forward migration allocates a fresh database `user_version`, verifies and clears
all legacy plaintext before enabling the new reader/writer, and fails closed
without a mixed writable state. It cannot enable until an immutable
`MIGRATION_GUARD_BASELINE_SHA` containing safe `dbUpgrade` UI plus the existing
future-version preflight is released as the minimum supported predecessor and
used by the acceptance fixture. Exact source `ca552f4d` guarantees only
the successful-inspection pre-schema-write path; it uses default SQLite open and
can continue initialization on inspection error, so it is not the accepted
safe-UX predecessor.
The accepted predecessor and all migrations/replacements share one cross-process
`DbCompatibilityLockV1`: normal apps hold a shared lease for their full DB
lifetime; bootstrap/migration/replacement holds exclusive from inspection through
DB checkpoint/close and ready-marker publication. This removes inspect/open and
migrator TOCTOU.

The stable lock file is config-directory `fyagent.db.compat.lock`, independent
of DB inode. Running replacement enters maintenance, stops admissions, drains
workers/sync/readers, closes all SQLite handles/hooks, releases shared, acquires
exclusive, and fully reinspects before effects. After ready publication it
releases exclusive, reacquires shared, reinspects, reopens, then resumes. There
is no in-place lock upgrade; drain/acquire/reinspect failure has zero replacement
effect.

Closed tagged `DbCompatibilityMarkerV1` variants are:

- `BootstrapPending`: revision/bootstrap ID, target generation=1, target
  application ID `0x46594147`, target/min-compatible user versions; it forbids
  observed DB/identity/migration fields.
- `MigrationPending`: revision/migration ID, required platform file identity,
  observed and target generation/application ID/user version, and min-compatible
  version. Observed application ID is legacy `0` or `0x46594147`; target is
  always `0x46594147`; target generation is observed+1.
- `ReplacementPending`: revision/replacement ID and one closed CandidateApply
  receipt containing source artifact ID, candidate ID/generation, attempt ID, original request
  revision, expected candidate/main-projection digests, exact prior/target DB
  identities/generations/content digests, application/user/min-compatible
  versions, and start time.
- `Ready`: revision, required file identity/generation,
  `applicationId=0x46594147`, user/min-compatible versions, plus tagged
  `completionReceipt=None|CandidateApply{replacement/candidate/attempt/request/
  projection identity,outcome,observedGeneration,completedAt}`; it forbids
  target, observed, bootstrap, and migration fields.

Versions are nonnegative i32; revisions/generations positive i64 with exact +1
monotonicity and no wrap. Legacy application ID 0 is accepted only by
marker-absent fallback or pending observed state. Canonical checksum covers all
other variant fields. Temp fsync + rename + directory fsync publication occurs
under exclusive lock. Pending precedes credential effects; ready follows DB
commit/checkpoint/close.

Candidate apply stages/checkpoints/closes the exact target, then fsyncs
ReplacementPending before the one main-file publish. Before normal services,
`DbReplacementRecoveryV1` acquires global
`CredentialArtifactIntegrityLockV1`, then compatibility exclusive, and rereads
the marker plus every observed relationship before joining the immutable
candidate attempt. Disputed IDs never choose lock identity. It only reads
identity/digests. Exact prior means observed no-effect;
exact target additionally permits a hook-free query-only projection read and
means applied; any mixed/unreadable state stays pending/needs-help. It never
initializes, migrates, invokes #35, rebuilds, or replays replacement. Ready first
retains the full completion receipt; matching Applied or
NeedsHelp(apply,observed_no_effect) persists DbCompletionAck with its marker and
sidecar revisions; then an exact marker+sidecar CAS may clear it. New replacement is blocked
until clearing. Crash at pending/main fsync/ready/sidecar boundaries repeats only
readback/finalization. Missing/corrupt sidecar or acknowledgement failure retains
the marker receipt, keeps normal DB/services closed, and reports the distinct
neutral `candidate_apply_authority_unavailable`; pre-effect StoreUnavailable
alone retains its no-effect promise.

Fresh bootstrap is allowed only when DB, marker, WAL, SHM, and journal are all
absent. Valid supported ready marker admits SQLite while shared lock remains.
Newer pending/ready returns upgrade-required. Marker-absent existing DB is
admitted only by an exact 100-byte main-header parser and only with no sidecars/
hot journal; parser validates magic, page/read-write versions, application ID,
user/schema versions, and `changeCounter==versionValidFor`. WAL/SHM/nonempty
journal, invalid/mismatched marker/header/file identity/generation, permission,
truncation, or lock failure is `database_compatibility_unknown`; no SQLite open
or `Database::init`. File IO never touches DB/WAL/SHM/journal.
Compatible bootstrap/migration pending with no live exclusive owner is
`database_compatibility_unknown(interrupted_bootstrap|interrupted_migration)`;
ReplacementPending is handled only by the narrow recovery above; ambiguous/
unavailable recovery maps to compatibility unknown plus candidate needs-help and
keeps ordinary DB closed. Lock timeout is `lock_busy`. No generic automatic
resume/open/init exists. Safe unknown
copy says a prior startup/migration may be incomplete and offers only local help,
compatible-build guidance, or exit.
Ordinary rollback/downgrade is forbidden after the marker or any reference-native
row exists; safe parser/adapter, guards, and schema stay installed. The generic
plaintext pre-migration backup is replaced by a #35-owned sealed local artifact
or no-backup fail-closed path. Sync/export/diagnostics/sanitized backup expose
only credential requirement status. Remote import may commit safe non-secret
fields plus `NeedsLocalRebind`; it is an import effect, not a Universal mutation.

Credential dependency results expose only
`secret_backend_unavailable|credential_migration_required|
credential_rebind_required`. Missing local binding after import/sanitized
restore persists `NeedsLocalRebind`; a later blocked Universal mutation is
`credential_rebind_required/no_effect`. The renderer maps it to
`universal_credential_rebind_required`, enters only #35 secure rebind, then
reloads the safe view before retry. Ordinary forms never receive the secret.
An old-binary future-version failure maps to `database_upgrade_required` on the
safe predecessor's `dbUpgrade` surface: it says this build did not initialize,
migrate, or modify business data, and that the data requires a newer compatible
FyAgent (it reads only marker/header metadata), and
offers only local upgrade guidance, an
already-local verified compatible installer when available, or exit. Continue,
config-folder mutation, downgrade, ordinary rollback, and restore are absent.

All DB copy/replacement paths use `UniversalCredentialTransferV1`, which parses
the exact Universal value in the `settings` blob into safe non-secret fields plus
`credentialRequirement=none|required`; malformed/unknown input rejects. It
contains no value/ref/binding token/lease/fingerprint.
Its persisted/read-back result is the closed
`UniversalCredentialTransferOutcomeV1 = committed |
committed_rebind_required | migration_required{artifactKind} |
rejected{code}`, where artifact kind is
`sql_import|webdav_v6|s3_v6|app_backup`. The migration-required projection is
`legacy_credential_artifact_blocked`; it leaves the artifact isolated and permits
only #35 staged migration or an existing source-specific confirmed delete.

Artifact actions use separate schema-v1/deny-unknown contracts.
`CredentialArtifactLifecycleV1` is exactly `Detected |
MigrationRequired{reason} | PreEffect{attemptId,action,ownerId,ownerEpoch,
leaseExpiresAt} | Reconciling{attemptId,action,ownerEpoch,currentStepId,effectStartedAt}
| NeedsHelp{attemptId,action,observed_no_effect|ambiguous|readback_unavailable,
lastReadbackAt} | CandidateReady{candidateId,revision,generation,readyAt} |
CandidateDeleted{candidateId,candidateRevision,wasApplied,deletedAt} |
Deleted{attemptId,deletedAt} | Rejected{code,rejectedAt}`.
`CredentialArtifactActionOutcomeV1` is `Accepted | Rejected | Reconciling |
NeedsHelp | CandidateReady | Deleted | StoreUnavailable` with the matching safe
view(s); Accepted/Rejected/Reconciling/NeedsHelp/StoreUnavailable carry the
requested `migrate|delete` action. `CredentialArtifactSafeViewV1` exposes schema, artifact ID/kind,
revision, safe lifecycle/display/reason, backend-derived allowed actions,
timestamps, and optional safe candidate ID/revision/generation only.

Revision-CAS transitions are Detected→MigrationRequired/Rejected;
MigrationRequired→PreEffect; PreEffect→MigrationRequired before effect or
Reconciling after effect-start; Reconciling→CandidateReady/Deleted/NeedsHelp;
NeedsHelp→NeedsHelp/CandidateReady/Deleted by readback-only recheck. CandidateReady
stays for apply and transitions to CandidateDeleted after successful candidate
delete; CandidateDeleted permits only keep-isolated or confirmed source delete,
never remigration. CandidateReady permits confirmed source delete only while its
candidate is Pinned or Applied; candidate Applying/NeedsHelp blocks it. Source
delete from CandidateReady/CandidateDeleted leaves candidate/ref/main unchanged.
Deleted/Rejected are terminal. StoreUnavailable
is an outcome, not invented lifecycle. Illegal lifecycle/step/receipt combinations
deny decoding; post-effect can never return to MigrationRequired/new attempt.

Each private source record carries monotonic
`candidateLineage=NeverPublished|Published{candidateId,candidateGeneration,
publishAttemptId,privatePublishReceipt}`. New records start NeverPublished; the
CandidateReady/candidate-record publish transaction changes it once to Published,
and CandidateDeleted/Deleted retain it. Publish effect-start/file/receipt with
NeverPublished requires readback. Published plus missing/mismatched candidate is
corruption/needs-help, never remigration or GC authority.

Both private source and candidate records carry closed
`pairIntegrity=Intact|Inconsistent{safeCode,detectedAt}`. Missing/mismatched
Published counterpart or lineage/identity mismatch marks every survivor
Inconsistent and pins it indefinitely. This safety overlay takes precedence over
the underlying lifecycle and permits only local help, exit, and safe reload—no
recreate/remigrate/apply/delete/GC/retry. Public views expose only safe code;
lineage/receipt remains private.

The sole `CredentialArtifactStoreV1` is a separate device-local SQLite sidecar,
schema v1. Stable config-dir `CredentialArtifactIntegrityLockV1` is its global
cross-process authority lock; every artifact/candidate action, scanner, GC, and
replacement recovery holds it exclusively across preflight/effect/readback/
publication. Lock identity never uses disputed relationship IDs. The fixed order
is integrity → maintenance → DB compatibility → sidecar/main. Main-DB replacement never replaces
it. It is excluded from sync/export/transfer/backup. Open/integrity/version
failure before action admission is `artifact_store_unavailable` with no fallback
authority or effects. The same failure after replacement pending/publish is
`candidate_apply_authority_unavailable`, retains marker uncertainty, and has no
further effect/replay.

Each private record adds source locator/content binding/generation and ordered
create-secret/publish-candidate/delete-source/cleanup steps to the lifecycle.
Each step persists `effect_started`, timestamp, idempotency key, and private
receipt slot before the external effect. Lease takeover is allowed only in
PreEffect; once started, a reconcile owner may only read #35 attempt
receipt, candidate attempt/generation manifest, or source existence/content
binding. It never reissues the effect. Missing source is delete success only
after recorded delete start; candidate/secret ambiguity is readback/manual, not
replay. Cleanup is a separately recorded idempotent step or manual.

Observed no-effect, ambiguous, and unavailable readback persist as
`credential_artifact_needs_help` with a safe reason and permit only local help/
manual resolution or revision-fenced `recheck_credential_artifact`, which is
readback-only and cannot reset/retry an effect. `migrate_credential_artifact`
publishes a separately named sanitized candidate/record only; main DB and
original source stay unchanged. Candidate apply is a separate explicit transfer.
Only confirmed `delete_credential_artifact` may contain `delete_source`.

Private `CandidateCredentialBindingV1` binds candidate ID/revision/generation/
content, source artifact/revision, and sorted Universal ID + binding digest +
SecretRef/version + creation attempt/receipt entries plus per-binding discard
attempt/effect-start/status/private-receipt steps for delete recovery. A separate
unique `(candidateId,requestRevision)` `CandidateActionAttemptV1` persists
positive monotonic attempt revision, immutable action/attempt ID/expected-candidate digest and
PreEffect|EffectStarted|NeedsHelp|Terminal state; Terminal retains result revision,
typed result, and exact safe snapshot. Apply resolution can persist closed
`DbCompletionAckV1{sidecarAttemptRevision,markerRevision,replacementId,attemptId,
outcome,observedDbGeneration}`. Lifecycle is
Pinned|Applying{action,attemptId,priorMainDbGeneration?}|
Applied|NeedsHelp{action,attemptId,priorMainDbGeneration?}|Deleted. Pinned permits apply or
delete; Applied permits delete only. Apply forbids a prior generation; delete
after apply requires it through terminal readback. Candidate files expose no refs.
Candidate NeedsHelp reason is exactly
`observed_no_effect|ambiguous|readback_unavailable`.
Closed `CredentialCandidateSafeViewV1` exposes only ID/revision/generation,
safe lifecycle/display/reason/actions/timestamps. Closed
`CredentialCandidateActionOutcomeV1` is Accepted|Rejected|Applying|NeedsHelp|
Applied|Deleted|StoreUnavailable, with action on every nonterminal command
outcome. Pinned→Applying is revision-CAS; pre-effect
failure returns Pinned. Apply post-effect resolves only Applied/NeedsHelp;
delete post-effect only Deleted/NeedsHelp. Unacked apply NeedsHelp while
ReplacementPending may remain, resolve exact prior to acknowledged
observed-no-effect NeedsHelp, or exact target to acknowledged Applied. After the
no-effect receipt clears, its immutable ack forces all rechecks to self-loop;
current main rows cannot reclassify it. Delete NeedsHelp may remain or resolve
Deleted, never Applied. Applied duplicate returns the same snapshot;
Deleted is terminal with `wasApplied`/generation. Illegal variants/actions deny.
`list_credential_candidates()` and `get_credential_candidate(candidateId)` are
backend-authoritative sidecar safety reads and include candidate-only survivors.
Startup and every list/get/action run `CredentialArtifactIntegrityScannerV1`:
under the global lock enumerate and reread all observed IDs, and CAS only a newly found
sticky Inconsistent overlay/revision. This is the sole query write; it cannot
change files/refs/attempts/main/lifecycle. Persistence failure returns
store-unavailable with zero actions.
Safe `credential-artifact://authority-updated{kind,id,revision}` only invalidates
and refetches the snapshot. Pair Inconsistent suppresses all lifecycle actions;
IPC/query cache/event/DOM/log/diagnostic sentinels exclude private fields.
`apply_sanitized_candidate(candidateId,expectedRevision)` uses fixed lock order
global artifact-integrity lock → maintenance drain → DB compatibility exclusive → artifact
CAS → staged/main DB; it revalidates manifest/content/binding/ref receipts,
compatibility/baseline/Codex impact, resolves leases, records effect-start, and
publishes reference-native rows once through staged replacement. Marker uses
ReplacementPending with exact prior/target identities/content/projection digest,
then Ready with a retained candidate/attempt completion receipt. Narrow
exclusive-lock startup recovery classifies exact prior/exact target/ambiguous
without normal services or effect replay. Matching Applied or
NeedsHelp(apply,observed_no_effect) atomically records DbCompletionAck; ambiguous/
unavailable/delete states forbid it. Receipt clear requires exact Ready-marker +
sidecar-attempt-revision CAS under the same lock order; mismatch/store failure
keeps admission closed. Once written, ack bytes are immutable across attempt
revisions. Duplicate success returns Applied and never
reapplies. Pre-effect sidecar unavailable/drift is zero-write; post-publish
sidecar failure is authority-unavailable and causes no further effect/replay.

Command handling checks the attempt ledger before current revision. An exact
repeated candidate action with the same original request revision returns its
active or persisted terminal snapshot and never repeats publish/cleanup/#35
discard while its result revision remains current. Multiple NeedsHelp rechecks
retain the same immutable request identity. A later valid action makes the old
receipt `candidate_action_superseded` and returns only the current safe view;
historical allowed actions never render. Another action at that request revision is
`candidate_action_conflict`; another revision is
`candidate_revision_changed`. Candidate delete updates the source record to
CandidateDeleted. `wasApplied=false` means main DB unchanged;
`wasApplied=true` means applied main DB state remains; original source remains
in both cases. The same source record cannot remigrate.

Source and candidate actions acquire the stable config-directory global
`CredentialArtifactIntegrityLockV1` before any identity read and retain it
exclusively through preflight, every external effect, readback, acknowledgement,
and authority publication while re-reading/CASing both sidecar records. Any
per-ID or source lock is non-authoritative and may be nested only inside this
global lock. Source delete is permitted with
candidate Pinned/Applied/Deleted and blocked by candidate Applying/NeedsHelp;
candidate actions are permitted with source CandidateReady/Deleted and blocked
by source PreEffect/Reconciling/NeedsHelp. Candidate delete records discard
effect-start before #35, reconciles by attempt receipt without replay, then
atomically publishes candidate terminal/action receipt plus source
CandidateDeleted. Already-Deleted source stays Deleted. Rejected candidate code
is closed to candidate not-found/revision/action conflict/action-in-progress,
action-superseded, source-action-in-progress, binding/ref/baseline drift, maintenance/compatibility/
Universal-impact, permission, and missing-ref cases; unknown fails closed.

Pinned candidate/refs have no timed purge, independent of original source.
Explicit candidate delete discards only proven-unreferenced refs through recorded
idempotent receipt; ambiguity is NeedsHelp. Source deletion never invalidates a
pinned/applied candidate.

Public projection is a total closed mapping. Artifact
MigrationRequired/PreEffect/Reconciling/NeedsHelp/CandidateReady/
CandidateDeleted/Deleted/Rejected map respectively to
`legacy_credential_artifact_blocked`, action-specific
`credential_artifact_preparing`, action-specific
`credential_artifact_reconciling`, action/reason-specific
`credential_artifact_needs_help`, `sanitized_candidate_ready`,
`credential_artifact_candidate_deleted`,
`credential_artifact_source_deleted`, and `credential_artifact_rejected`;
Detected is internal-only and pre-effect StoreUnavailable maps to
`credential_artifact_store_unavailable`. Pair Inconsistent overrides lifecycle
as `credential_artifact_pair_inconsistent`, with retained/no-delete-or-rebuild
copy and help/exit/reload only. Candidate Pinned,
Applying(apply), Applying(delete), NeedsHelp(apply), NeedsHelp(delete), Applied,
Deleted(false), and Deleted(true) map to ready/applying/deleting/apply-needs-help/
delete-needs-help/applied/deleted/deleted-after-apply. Apply needs-help copy
distinguishes determined observed-no-effect from ambiguous/unavailable; Applied
permits explicit candidate delete without main/source rollback. Superseded
rejection has exact no-repeat/current-state copy and only current-view controls;
other Rejected outcomes add only a safe action-rejected alert to the current lifecycle; Accepted and terminal
outcomes project the embedded persisted lifecycle. Unknown or missing action
fails closed. All Deleted public views omit SecretRef/version/receipt/private
binding/locator/digest/value while retaining only safe idempotency fields.
Post-publish sidecar/ack failure maps instead to
`candidate_apply_authority_unavailable`: neutral prior-or-target/main-closed/no-
replay copy with local repair/help or exit only.

Safe list/read expose no locator/binding/receipt/secret. Migrate/delete require
`artifactId+expectedRevision`, private-binding revalidation, and owner CAS.
Source deletion is explicit only. Source/candidate metadata never purge
independently: either record/content/ref/action/recovery dependency pins both.
One global-lock sidecar transaction may purge both records/attempt receipts only
when source and candidate are Deleted, both files absent, refs released or
main-owned, no pending/needs-help/DB completion receipt exists, and 30 days pass
from the later terminal/receipt anchor. Missing counterpart is corruption, not
authority. Artifact-only 30-day GC requires exact lineage NeverPublished,
Deleted/source absence, no candidate file/row/ref/action/effect-start/receipt,
and the global integrity lock. Published is permanent and requires intact-pair GC;
missing counterpart is corruption. Closed codes add
`artifact_store_unavailable|readback_unavailable|pair_integrity_inconsistent` to
artifact/source/generation/schema/integrity/inspection/permission/dependency/
migration; unknown fails closed.

- SQL export emits only the transfer. Import stages a temporary DB, runs #35
  migration/validation, merges matching device-local refs, raises the marker,
  then atomically replaces; failure leaves main unchanged and creates no raw
  safety backup.
- #35 allocates `DB_COMPAT_VERSION > 6` and a new `db-vN` WebDAV/S3 layout. New
  clients never dual-read/write `db-v6`; old clients cannot see the new layout.
  Legacy remote data requires explicit staging migration or typed rejection.
- App-managed backup create is sanitized/staged and carries compatibility
  metadata. Restore validates/migrates/merges local refs before replacement.
  Pre-safe/unknown existing backups are inventoried as
  `legacy_credential_backup_blocked` and quarantined from ordinary restore/sync/
  export until #35 creates a newly sanitized artifact or the user deletes them.
- Candidate markers are `max(local,staged,required)` and never decrease. Stable
  ID plus credential-requirement digest must match before a local ref is retained;
  otherwise `NeedsLocalRebind`. A declared `none` maps to `None` only when the
  transfer schema proves it credential-free.

Transfer uses `UniversalTransferCodexImpactSnapshotV1` per Universal ID, binding
local/staged Universal presence/fingerprint and Codex membership, local actual
child presence/epoch/redacted digest, staged actual child presence/redacted
digest, staged safe-field projected child digest, and create/update/delete/
unchanged action. Stage scans child ID and versioned provenance; presence counts
even with `apps.codex=false`. Before a Universal-to-UCP adapter, any membership,
child, or projected-digest difference is
`universal_codex_transfer_unavailable`: whole transfer is quarantined and main
DB/local child/epoch/marker/sync/cache/events stay unchanged. Allowed no-impact
transfer structurally drops every staged Universal Codex child and reinjects the
exact local child/membership. SQL/WebDAV/S3/current/legacy-backup fixtures cover
staged orphan child, local orphan child, membership and child create/update/
delete differences, projected changes, and no-impact control.

## 9. v1 compatibility

- New code rejects v1 ready Plans with `unsupported_schema/repreview`.
- V1 terminal jobs remain available through compatibility projection.
- V1 consumed nonterminal/orphaned jobs may be classified by their old stored
  predicates using readback only; no writer replay.
- Old code must fail closed on v2 rows. It is not required to render v2 history.
- The accepted predecessor at `MIGRATION_GUARD_BASELINE_SHA` is blocked by the
  marker/header before SQLite open/schema/business-data/Universal reads or writes.
  Inspection error is `database_compatibility_unknown` and fail closed. Earlier
  binaries receive only the exact-source successful-inspection pre-schema-write
  guarantee and are not accepted as migration
  predecessors. No binary sees an opaque ref/binding token as `api_key`;
  downgrade after migration is unsupported.
- Recovery parsing dispatches on schema first. V1 exact
  `not_needed|succeeded` projects to v2 `none`, and v1 `recovery_required`
  projects to `manual_required`, while retaining the safe legacy result code.
  V2 persisted/wire decoding accepts only `none|manual_required`.

## 10. Required verification

- Contract/store: canonical vectors in Rust and TypeScript, additive v16 fields,
  v1/v2 discriminator and downgrade fail-close, atomic admission/replay,
  lifecycle/retention, three legacy recovery mappings, strict v2 recovery enum,
  latest Plan discovery/revision-CAS abandon, and sentinel redaction.
- Side-effect spies: preview writes only the Plan row and produces zero Provider,
  current/live/common/MCP, endpoint, backup, job/event, tray/cache, sync, process,
  or Provider/model network effects.
- Concurrency: worker claim/revision/event CAS, duplicate admission, cancel/effect
  race, every typed admission-to-effect digest/resource/source/precondition/
  readability drift, active-owner query, pre-effect interruption, effect-started
  readback reconciliation, and writer exactly once.
- Resource/failure matrix: every operation/currentness combination, required and
  auxiliary readback, partial writes, unavailable readback, backup/recovery, and
  sync quarantine/coalescing.
- Privacy/export: synthetic secret/path/ref/fingerprint markers are absent from
  public IPC, events, logs, sync SQL, regular/diagnostic export, and sanitized
  app-managed backup; remote import preserves local ledger.
- Entry/UI: clean/warning/expired/drift/unsupported/secret-missing/no-change,
  one confirmation, reload/discovery, monotonic event invalidation, and zero
  direct mutation fallback. Direct supported normal-mode legacy Codex IPC after
  cutover returns `change_plan_required`; unsupported subcases return their
  specific code; both have zero writer/effect counters. Registration/callsite
  scans and spies cover all six commands, tray (including proxy flags), profile,
  deep link, old UCP executor, endpoint writers, and public ProviderService.
  Positive acceptance proves tray focuses/opens the exact target Plan UI and
  deep link preserves only allowlisted safe draft fields; navigation failure is
  zero-write. Profile renders translated accessible whole-apply-unsaved state.
- Recovery UI covers `database_upgrade_required` and
  `universal_credential_rebind_required/no_effect`. The former has `zh|zh-TW|en|
  ja`, initial heading focus, semantic alert/description, labelled keyboard
  actions, no continue/downgrade/rollback/restore, and zero pre-DDL/business-read/
  DAO/service/write/sync/network activity at exact
  `MIGRATION_GUARD_BASELINE_SHA`. Import/restore fixtures distinguish retained
  local binding from persisted `NeedsLocalRebind`; a later rejected mutation
  reports `credential_rebind_required/no_effect`, routes only to #35 secure
  entry, reloads the safe view after rebind, and proves all exported/sanitized
  artifacts omit ref/token/value sentinels.
- Transfer UI covers `universal_safe_import_rebind_required` immediately after a
  committed safe import, later mutation `/no_effect`, and
  `legacy_credential_artifact_blocked` for every artifact kind. Reload preserves
  the persisted outcome; no secret preview, automatic delete, or raw restore is
  available.
- Compatibility/artifact recovery UI explicitly covers unknown/interrupted/
  lock-busy, store-unavailable, reconciling, observed-no-effect, ambiguous, and
  readback-unavailable in `zh|zh-TW|en|ja`, with focus/alert/keyboard/screen-
  reader semantics and reload persistence. No post-effect migrate/delete/retry
  control renders. Safe view/IPC/cache/event/DOM/log/diagnostic sentinels exclude
  path/URL/ETag/digest/receipt/ref/value. Migration-candidate fixtures prove the
  source/record remains until separately confirmed delete.
  Post-publish `candidate_apply_authority_unavailable` has neutral prior-or-
  target/main-closed/no-replay copy, help/repair/exit only, and separate
  four-locale/a11y/reload/private-sentinel fixtures at Ready-before-ack and
  ack-before-clear; it never reuses pre-effect no-effect StoreUnavailable copy.
  Pair-inconsistent fixtures for each missing/mismatch code show retained/no-
  reconstruct-or-delete truth with help/exit/reload only, pin authority
  indefinitely, and prove zero recreate/remigrate/apply/delete/GC/retry effects.
- Maintenance and every public artifact/candidate lifecycle and action-outcome
  projection are exhaustive in four locales, accessible, reload-stable, and
  expose only backend-derived actions. Fixtures include preparing/reconciling,
  source/candidate deleted, rejected/store-unavailable, action-specific candidate
  applying/deleting/needs-help, applied, and both `wasApplied` deleted cases;
  Detected is internal-only. Applying/deleting expose no duplicate submit;
  needs-help exposes only manual/readback-only action and distinguishes determined
  no-effect from uncertain readback; applied never implies source delete and
  offers explicit candidate delete with no main/source rollback. Response-loss duplicate candidate delete returns the same
  result without cleanup/discard replay; action/revision conflicts are typed.
  Source-delete fixtures cover candidate Pinned/Applied/Deleted with unchanged
  candidate/ref/main state after reload, and Applying/NeedsHelp rejection with
  zero effect.
  Source-delete versus candidate apply/delete races prove one global-lock/CAS
  winner, no deadlock/crossed effect, source-active rejection, discard-attempt
  readback without replay, and atomic candidate/source terminal publication.
  Candidate-apply crash fixtures pause after ReplacementPending, main publish,
  Ready receipt, sidecar acknowledgement, and marker clear; narrow recovery yields exact-prior
  no-effect, exact-target Applied, or ambiguous NeedsHelp with normal DB closed
  and zero effect/service/#35 counters. Exact-prior and exact-target both cover
  Ready-before-ack and ack-before-clear; marker/attempt/outcome/generation mismatch
  and sidecar unavailable never clear or admit. Unresolved apply rechecks cover
  exact-target Applied versus exact-prior acknowledged NeedsHelp; post-clear
  no-effect rechecks self-loop despite unrelated target-like main mutations and
  ack bytes never change. Delete rechecks resolve only Deleted. Original-request
  retry stays exact while current, with superseded/current-safe-view after a later
  action. Candidate list/get/event fixtures rediscover a source-missing standalone
  candidate after restart, reject stale-cache mutations, and expose no private
  sentinel. Integrity-scanner fixtures prove startup/list/get/action global-lock
  CAS, including source-A/candidate-C/source-B split-brain, overlay-only
  revision/event-after-commit, zero file/ref/attempt/main/
  lifecycle changes, and fail-closed persistence error. Static spec assertions
  reject source-ID-derived recovery locks, `peek source ID`, and any
  enumerate-before-global-lock scanner sequence, including the stale phrase
  `source-artifact action lock` and equivalent relationship-selected authority.
  >30-day GC fixtures retain
  either surviving counterpart;
  only paired both-Deleted/no-dependency joint purge succeeds. Unpaired fixtures
  cover positive NeverPublished GC, publish-boundary crash, Published+missing
  counterpart corruption, paired control, and concurrent GC/action fencing.
  Superseded copy and current-view-only actions have full four-locale/a11y/reload
  coverage and never expose historical controls or private sentinels.
  Candidate safe views, including Deleted, omit refs/versions/creation/discard
  receipts/private bindings/locators/digests/values.
- Deep-link shared full-input fixture exactly equals
  `CodexDeepLinkPlanDraftV1`; forbidden secret/config/usage/unknown inputs have
  zero network/staging/persistence. Universal matrix includes
  membership=false+existing-child, stale revision/epoch, interrupted old
  upsert/sync, preflight race, and credential-free/no-child/non-Codex
  `None|Clear` success; blocked cases have
  zero Universal/per-app/event/cache/epoch/other-app writes and allowed commit
  never calls Codex save/delete. Closed-variant fixtures reject invalid field
  combinations; safe-view tokens are backend-only. Direct Service bypass and
  permit forge/clone/serde/reuse/wrong-binding/visibility checks fail closed.
- Universal read-surface sentinels prove the existing list/get command names
  return only safe views and no plaintext credential crosses IPC/query cache/
  events/DOM/logs/diagnostics; raw readers are private and non-serde. Credential
  cases cover `None`, `Clear`, `Preserve`, `Replace`, legacy migration,
  reference-native persistence, ref/version/lease expiry and resolver failure,
  attempt-memory zeroization, pre-#35 credential-free success, and pre-#35
  secret-bearing zero-write rejection.
- Credential schema fixtures cover every `ProviderCredentialIntentV1` and
  `UniversalCredentialIntentV1` variant, distinct domain digests,
  schema-first/deny-unknown decoding, and illegal cross-domain payloads.
  Migration/rollback fixtures run exact `MIGRATION_GUARD_BASELINE_SHA` against a
  migrated DB and prove `db_version_too_new` after version inspection but before
  DDL/business-read/write/sync/network; they also prove
  no ref/binding token is used as an API key, no ordinary plaintext backup is
  created, downgrade is blocked, and sync/export/diagnostics/sanitized backup/
  remote import preserve only the allowed safe/local projection.
- Copy/replacement fixtures cover the exact Universal value inside `settings`,
  SQL export/import, WebDAV/S3 upload/download at new `DB_COMPAT_VERSION` and
  `db-vN`, app-managed backup create/list/restore, and pre-safe existing backups.
  They prove staging-before-replace, no v6 dual-write, marker monotonicity,
  matching local-ref preservation, `None` versus `NeedsLocalRebind`, raw safety
  backup prohibition, legacy-backup quarantine, and main/marker unchanged on
  every rejected or failed path.
- Guard fixtures cover all-absent fresh bootstrap, bootstrap/pending/ready atomic
  marker, marker-absent header fallback, WAL-only newer user-version, hot journal,
  change-counter mismatch, corrupt/truncated/permission/identity/generation/lock
  errors, and a concurrent migrator paused at each boundary. Rejections prove
  zero SQLite open/DB-WAL-SHM-journal touch/DDL/business-read/write/sync/network
  plus no `Database::init`.
  Runtime replacement fixtures drain/close all handles, release shared, race a
  reader/migrator, reacquire exclusive, fully reinspect, publish ready, reacquire
  shared/reinspect/reopen, and prove no in-place upgrade, deadlock, or unchecked
  TOCTOU. Tagged-marker fixtures reject every forbidden field/range/generation/
  application-ID combination, including legacy-0 misuse.
- Binding fixtures freeze canonical domain/fields and equal vectors, then vary
  ID, slot, provider type, consumer apps, auth scheme, endpoint scheme/host/port/
  path, version, and digest; every mismatch is `NeedsLocalRebind`, and TS cannot
  authorize reuse.
  Path fixtures assert unreserved decoding before dot removal and exact encoded-
  dot/slash/backslash, repeated/trailing slash, Unicode NFC/UTF-8 encoding, and
  empty-path vectors.
- Artifact fixtures cover multiple same-kind records, opaque ID/revision,
  private bytes/manifest/ETag changes, sidecar integrity/version failure,
  pre-effect lease takeover, dual owner, delete-after-start crash, SecretRef-
  created crash, candidate-publish boundary, post-effect readback without replay,
  main replacement survival, source retention, explicit delete, closed codes,
  terminal retention, and device-local exclusion. Transfer-impact fixtures cover each copy family,
  staged child with membership=false, local orphan child, membership and child
  create/update/delete/projected changes, whole-transfer zero-write quarantine,
  staged-child exclusion, exact local preservation, and no-impact control.
- Artifact schema fixtures cover every lifecycle/action outcome/safe-view variant,
  legal revision transition, forbidden state/step/receipt combination, and
  store-unavailable without fabricated state. Candidate-binding/apply fixtures
  cover private candidate→binding→ref receipt map, fixed lock order, crash at
  each marker/publish boundary, duplicate apply, binding drift, deleted original,
  sidecar unavailable, pin/retention, explicit candidate delete and ambiguous
  cleanup without replay.
