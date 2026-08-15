# Issue #55 integrated delivery — Baseline-bound Change Plan and Codex slice

## Goal

This task is the integrated delivery card that consumes the Plan-model contract
from #55 and preview/confirmation/adapter contracts from #56/#57/#58, hands the
shared ledger/job contract to downstream #41, and delivers the first Codex
vertical slice tracked by #63. Completing this card
does not automatically close any related Issue; final governance maps each
Issue's own acceptance criteria and updates only the evidence actually earned.

Deliver a user-visible, side-effect-free Change Plan flow for Codex Provider
create, edit, and switch. A user reviews one concrete, immutable plan, confirms
that exact plan once, and later sees a durable result derived from local
readback. The preview must never silently become a different operation between
review and apply.

## User problem and value

Today, a Provider action can move directly from an editable form or switch
button into a mutation. The existing switch-only UCP slice adds a useful plan
ledger and readback job, but it does not cover create/edit, expose the complete
plan contract, distinguish all invalidation reasons, or guarantee that the
writer consumes the exact payload that was reviewed.

Before confirming, the completed product lets the user answer:

1. What will change?
2. Which managed resources are affected?
3. Will a backup be created; if not, what automatic compensation or manual
   recovery limits apply, and which credential references are required?
4. Why is the preview clean, warning-bearing, unsupported, expired, drifted, or
   blocked by a missing credential reference?
5. What privacy boundary applies, and what is deliberately not sent over the
   network?

Ordinary scenarios:

- Create: save a new Provider without changing the current route, or explicitly
  save and select it in the same reviewed plan.
- Edit: change a non-current Provider without touching live routing, or edit the
  current Provider with all resulting current/live resources visible.
- Switch: review the exact current-to-target route change and restart guidance.

## Confirmed decisions and immutable inputs

- First real slice: normal-mode Codex Provider `create_only`,
  `create_and_select`, `edit`, and `switch`.
- There is no implicit activation when no current Provider exists. Unchanged edit
  returns `no_changes`; switch to current returns `target_already_current`.
- Proxy takeover and `critical` risk are unsupported in the first slice. Info
  and warning use one explicit confirmation; no second confirmation is added.
- Switching to the built-in official Provider is also unsupported in the first
  slice because the legacy writer can clean `auth.json`; that resource is not
  admitted until its baseline/readback/risk/recovery contract exists.
- Preview and apply do not probe Providers, fetch models, send model requests,
  or otherwise initiate outbound Provider traffic.
- One user confirmation binds exact `planId + planDigest`. Apply accepts no
  intent or form draft and does not recompute semantic meaning.
- Drift, expiry, source/baseline/target/secretRef change invalidates the plan and
  requires a fresh preview. There is no force-apply.
- Preview may persist immutable plan payload and lifecycle metadata. It has zero
  side effect on managed Provider state, current routing, live files, settings,
  tray/cache, jobs/events, backups, or external services.
- `ProviderService` remains the only domain writer. Change Plan may add an
  exact-payload/CAS seam inside it but does not clone writer logic.
- #35 owns SecretBackend. Until its immutable handoff exists, this task uses an
  opaque non-secret secretRef port and fixtures only. Production admission that
  must add, replace, or resolve a secretRef returns `dependency_unavailable`;
  secret-bearing native apply acceptance remains blocked.
- #41 consumes the shared ledger/job and owns its V2 apply workspace. It must not
  build a second store or state machine.
- Before the first effect boundary a planned job may be cancelled. After that,
  UI may close while the job continues; app interruption is resolved by
  readback-only reconciliation on next start. Manual recovery required links to a local,
  no-network help surface with a redacted diagnostic ID.
- UCP terminal handoff is `6859e9ce04970008f4cf8b3d4883b4f70316291a`;
  source review is fixed at `ca552f4d918cacc734f81f7efdef70619da139b8`.

## In scope

- One authoritative Plan schema and canonical digest contract.
- Side-effect-free plan generation for normal-mode Codex create/edit/switch.
- Immutable plan persistence, lifecycle metadata, retention/purge, and public
  read/discovery APIs.
- Complete affected-resource fingerprints for every supported writer effect.
- Typed invalidation and zero-write pre-admission behavior.
- One-confirmation admission that immediately returns a planned job.
- Exact reviewed-payload execution, readback, recovery classification, and
  interruption reconciliation without unknown-write replay.
- Complete preview/query/job UI projections and four-locale accessibility.
- Generated visual reference, high-fidelity prototype, and usability review
  after design freeze.
- Focused module tests followed by cross-layer, browser/renderer, native,
  failure-path, side-effect spy/fault, Trellis, and Git verification.
- Small commits and a pushed dedicated branch. PR creation is allowed only after
  final review; main is not merged and nothing is deployed.

## Out of scope

- WorkBuddy adapter, other AppTypes, and other Agents.
- Provider delete, legacy import-default/bulk import, sort/failover, and
  non-Codex additive-app flows. Codex deep-link import in this slice is only
  zero-persistence safe draft-to-Plan routing, not a legacy import writer.
- A Universal-to-UCP adapter. Any Universal operation that could materialize,
  update, remove, or resync a Codex Provider is disabled as a whole until that
  adapter and #35 exist; it cannot retain the old direct path.
- Proxy takeover, proxy backup/route, official-target switch, auth.json cleanup,
  and critical-risk flows.
- Provider reachability, model discovery, real model requests, or real usage
  observation.
- Cloud secrets, credential migration, hardware-key flows, or another
  SecretBackend.
- Generic transaction DSL, dynamic adapter registry, or cross-domain undo.
- Main merge, release, signing/notarization, deployment, Windows UAT, or claims
  of production/user acceptance.

## Requirements

### R-01 Side-effect-free preview

Plan generation may read authoritative local state and persist only immutable
plan payload plus lifecycle metadata. It creates no job/event/backup; changes no
Provider/current/settings/live/tray/cache state; and calls no Provider/model
network client. Abandon/expiry/invalidation may update only lifecycle metadata.

### R-02 Public projection and private execution envelope

`PlanPublicProjection` is safe for IPC/query/events/DOM and contains:

- `schemaVersion`, `canonicalizationVersion`, `operationVersion`
- `planId`, `planDigest`, `intentDigest`, `baselineDigest`
- typed operation, safe normalized intent projection, `createdAt`, `expiresAt`
- lifecycle `status`, optional `owningJobId`, `planRevision`
- opaque local `actorCode` and structured `sourceVersions`
- safe `affectedResources`, ordered actions with `actionId` and effect boundary
- target readback predicates and typed recovery modes
- risks, warnings, precondition codes, recovery hints, privacy/evidence notes
- opaque/redacted credential-reference status, never a secret value

`PlanExecutionEnvelope` is backend-only local persistence and contains exact
normalized non-secret Provider payload, full baseline fingerprints, executable
action parameters/readback predicates, opaque secretRef identity/version
requirements, and recovery instructions. It never crosses IPC/events/query
cache/DOM/logs/diagnostic export.

Neither representation contains a secret value, reversible secret hash, raw
live config, absolute path, or unrestricted backend error. Raw non-secret
Provider settings exist only in the private envelope so create/edit remains
exact across reload.

Public Plan authority includes read by Plan ID, latest discovery by safe
`{app,operation,subjectId}` (create uses its preallocated Provider ID), and
revision-CAS abandon. Abandon may change only `ready` with no owning job to
`abandoned` when `expiresAt > now`, set `abandonedAt`, and increment Plan
revision. At `expiresAt <= now`, the transaction returns/persists typed expired
using the original expiry retention anchor and never writes `abandonedAt`; it
creates no job/event/writer/managed effect.

### R-03 Canonical semantic identity

Three fixed-vector contracts own `intentDigest`, `baselineDigest`, and
`planDigest`. Canonicalization version, operation version, domain separator,
field inclusion/exclusion, object-key ordering, array ordering, and encoding are
public contract. Plan digest covers every executable action, resource predicate,
secretRef requirement, precondition, recovery mode, and warning/risk code.
Equivalent normalized intent/baseline yields equal semantic digests and a unique
plan ID. Timestamps, expiry, plan ID, localized copy, and presentation order are
excluded.

### R-04 Complete affected-resource baseline

Every supported action enumerates all potential effects: Provider definition,
source Provider backfill when applicable, DB current, device current, Codex live
projection, common configuration, managed MCP projection, and secretRef
metadata. Each resource has stable key, expected version/fingerprint, operation,
readback predicate, effect boundary, and
`recoveryMode=none|manual_required` with backup timing/scope/limits. The first
slice makes no automatic inverse, compensation, or restore promise.
Any proxy takeover effect fails closed as `unsupported_mode`.

### R-05 Credential-reference boundary

Public projection shows only safe reference status. Private envelope stores
opaque secretRef identity and minimum presence/version metadata. Missing/changed
metadata or missing resolver capability fails before admission with no job.
Actual lease acquisition and lifetime validation belongs to the #35 port and
occurs only after admission, after digest/resource recheck, and before the effect
gate. A lease acquisition/lifetime failure terminalizes the existing job as
`status=failed`, `resultCode=dependency_unavailable`, `observedState=no_effect`,
`recovery=none`; the Plan remains consumed and owning job remains readable.
Writer, backup, and managed-write counters stay zero. After the dependency is
repaired, the user must create and confirm a new preview. Before that port is
frozen by exact SHA, every source, target, live-auth, prepared-projection, and
recovery input must be proven credential-free or preview returns
`dependency_unavailable`.

Credential intents are two distinct schema-v1 closed types. Backend-private
Change Plan uses `ProviderCredentialIntentV1 = None |
Preserve{secretRef,expectedVersion} | Replace{secretRef,expectedVersion} |
Clear`; the Universal safe mutation wire uses
`UniversalCredentialIntentV1 = None | Clear |
Preserve{opaqueBindingToken} | Replace{secretRef,expectedVersion}`. They have
separate canonical domains, reject unknown/mixed fields, and cannot be implicitly
converted; only #35 maps the Universal wire intent to an internal requirement.
Credential dependency outcomes carry one safe reason code:
`secret_backend_unavailable | credential_migration_required |
credential_rebind_required`. The code changes recovery guidance only and never
contains a ref, token, path, value, or backend error.
Local ref reuse additionally requires the backend canonical
`UniversalCredentialBindingKeyV1` digest. It binds Universal ID, primary
credential slot, normalized provider type, and the sorted app/auth-scheme/
normalized-endpoint destinations derived from safe projection. ID or a generic
“credential required” flag is never enough; any version/field/digest mismatch
becomes `NeedsLocalRebind`. The digest contains no secret input and remains
opaque to TypeScript.

### R-06 Lifecycle and typed admission rejection

Persisted lifecycle is distinct from caller/admission rejection. Caller wrong
digest never changes Plan status. Admission distinguishes at minimum:
`plan_not_found`, `invalid_digest`, `expired`, `consumed`, `resource_changed`,
`source_version_changed`, `resource_unreadable`, `permission_denied`,
`secret_ref_missing`, `secret_ref_changed`, `dependency_unavailable`,
`unsupported_schema`, `unsupported_operation`, `unsupported_mode`,
`unsupported_risk`, and `precondition_failed`. Drift returns deterministic
sorted `reasons[] {resourceKey, code}`. All admission rejection paths create no
job and call no writer; a post-admission secret lease failure is instead a typed
no-effect terminal job.

Persisted non-ready lifecycle has explicit outcomes:
`abandoned -> plan_abandoned`, `expired -> expired`, and
`invalidated -> plan_invalidated` with stored reasons. Only `ready` continues to
fresh inspection.

Admission decision order is fixed: missing Plan; caller/stored digest mismatch;
consumed Plan handling; ready-Plan schema/operation/mode/risk; expiry; current
resource/source/secret/precondition inspection; atomic admission. A consumed
Plan with matching identity and owningJobId returns that job. A consumed legacy
or orphaned Plan with no owning job returns `consumed`. Expiry or drift on a
ready Plan atomically persists lifecycle `expired` or `invalidated` plus reasons
before returning the rejection.

### R-07 One confirmation, exact execution, and idempotent retry

Renderer sends only `planId + planDigest`. Admission atomically checks schema,
canonical payload digest, expiry, status, baseline, target, source versions,
secretRefs, and preconditions; binds `owningJobId`; consumes once; creates one
planned job; and returns that snapshot immediately. Worker consumes only the
private stored envelope. Mutable Provider ID reload may not change semantics.
Retrying the same identity returns the owning job; same plan ID with another
digest returns `invalid_digest`; neither starts a second writer.

After admission and before effect, any stored-digest, resource CAS, source,
precondition, or required-readability failure terminalizes the existing job as
`failed + pre_effect_validation_failed + sorted typed reasons + no_effect +
recovery=none`. The Plan stays consumed; writer, backup, and managed-write
counters stay zero; the user must create and confirm a new preview.

### R-08 Durable read, cancellation, and recovery

Public APIs read/discover Plan and owning/recoverable jobs after renderer reload.
Snapshot is authoritative; events only invalidate/refetch. Cancellation is
allowed only before first effect. Interruption triggers readback-only reconcile,
never writer replay. Required readback unavailable is fixed as
`status=failed`, `resultCode=readback_unavailable`, `observedState=unknown`,
`recovery=manual_required`. Recovery is manual in this slice. After remediation, a
fenced readback-only recheck may clear recovery/sync quarantine; it never calls a
writer or restores a backup.

### R-09 User-visible state clarity

Renderer covers `draft`, `generating`, `generation_failed`, clean/warning
preview, `abandoned`, `admission_pending`, `expired`, `drift`, `unsupported`,
`secret_missing`, `planned`, `running`, `cancelled`, `reconciling`, terminal
`dependency_unavailable/no_effect`, terminal
`pre_effect_validation_failed/no_effect`,
`profile_change_plan_required`, `universal_codex_change_plan_unavailable`,
`universal_safe_import_rebind_required`,
`universal_credential_rebind_required/no_effect`,
`legacy_credential_artifact_blocked`,
`credential_artifact_preparing`,
`credential_artifact_reconciling`,
`credential_artifact_needs_help`,
`credential_artifact_candidate_deleted`,
`credential_artifact_source_deleted`,
`credential_artifact_rejected`,
`credential_artifact_pair_inconsistent`,
`credential_artifact_store_unavailable`,
`sanitized_candidate_ready`, `sanitized_candidate_applying`,
`sanitized_candidate_deleting`, `sanitized_candidate_apply_needs_help`,
`sanitized_candidate_delete_needs_help`, `sanitized_candidate_applied`,
`sanitized_candidate_deleted`, `sanitized_candidate_deleted_after_apply`,
`sanitized_candidate_action_superseded`,
`candidate_apply_authority_unavailable`,
`database_maintenance_pending`,
`database_upgrade_required`,
`database_compatibility_unknown`,
`universal_codex_transfer_unavailable`,
`job_not_found`, `query_unavailable`, and
`readback_unknown` projections. `profile_change_plan_required` states that none
of this Profile's changes were saved; available actions are return to edit or
remove the Codex Provider delta. It does not claim Profile-to-Plan conversion
until #41 supplies that adapter.
`universal_codex_change_plan_unavailable` states that no Universal or per-app
change was saved. New Codex membership is disabled and a legacy Codex-linked
Universal row remains read-only in this slice. For create/edit/duplicate/save or
save-and-sync, the UI may offer “create/edit an independent Codex Provider” in
the app-specific Plan flow; doing so cancels rather than continues the Universal
operation. Delete/remove-membership/resync/manual-sync can only return/cancel
and explain that the Universal-to-UCP adapter is required. Non-Codex-only
Universal operations continue before #35 only when they are proven
credential-free, have no actual Codex child, and use
`UniversalCredentialIntentV1=None|Clear`. A legacy plaintext row,
`Preserve`/`Replace`, or sync that needs a
credential returns `dependency_unavailable`; the UI states that no Universal or
per-app change was saved and asks the user to repair/wait for the dependency.
`universal_credential_rebind_required/no_effect` states that an import or
sanitized restore committed safe non-secret fields but deliberately carried no
credential; this device needs a new local binding, and the attempted Universal
mutation saved nothing. Its only continuation is #35's secure-entry/rebind flow,
then reload the safe view and retry; it never asks for a credential in ordinary
form fields.
Immediately after safe import/restore, before any mutation attempt,
`universal_safe_import_rebind_required` states that safe fields were committed,
no credential or child projection was transferred, and local rebind is required.
`legacy_credential_artifact_blocked` covers old SQL/WebDAV/S3/app backups: the
artifact remains isolated and unchanged, ordinary import/restore/sync/export is
disabled, and the user may start #35 secure staged migration or use an existing
source-specific confirmed delete action. Migration publishes a separately named
sanitized candidate only; it never imports/restores main DB or overwrites/deletes
the original. Applying the candidate is a separate explicit transfer. No secret
preview or automatic delete is offered.
Every blocked artifact is referenced only by a backend opaque artifact ID and
revision. Reload may list/read it safely; migrate/delete revalidate the exact
content/manifest/ETag and CAS the revision. An artifact changed since inspection
stays isolated and must be refreshed. It has no timed source deletion.
Artifact authority lives in one device-local sidecar, not the replaceable main
DB. If that store is unavailable before any effect,
`credential_artifact_store_unavailable` disables migrate/delete and leaves
sources/main untouched. A post-publish candidate-apply sidecar/ack failure is
instead `candidate_apply_authority_unavailable`: FyAgent cannot verify local
candidate-apply authority; main may still be prior or may already contain the
candidate, remains closed, and apply will not repeat. It offers only local
repair/help or exit. Once an artifact effect
starts, UI shows `credential_artifact_reconciling`; it cannot retry. Readback
either confirms the secret/candidate/delete receipt, reports observed no-effect,
or persists `credential_artifact_needs_help` for determined no-effect,
ambiguous, or unavailable truth. That state offers only local help/manual
resolution or fenced readback-only recheck—never migrate/delete/retry. Only an
explicit confirmed source delete removes an artifact, and its quarantine record
cannot time-purge while the source exists.
`sanitized_candidate_ready` says a separate safe candidate and local credential
bindings are pinned; the original remains unchanged. Apply is a separate explicit
action. Applying states that main DB may change or may already have changed;
deleting states that only the candidate and proven-unreferenced pins may change,
never the original source or main DB. Neither state shows a duplicate action.
Apply/delete needs-help copy retains that action distinction and permits only
manual help or fenced readback-only recheck. Apply `observed_no_effect` says
recovery verified the exact prior DB, the candidate was not applied, and the
attempt will not replay; only `ambiguous|readback_unavailable` says truth is
uncertain. Applied confirms main DB readback but still does not delete the
original. It offers an explicit delete-sanitized-candidate action that removes
candidate recovery material without rolling back main DB or deleting source,
plus the separately confirmed source-delete action. Deleted-before-apply states that main DB
is unchanged; deleted-after-apply states that the applied main DB state remains.
Both state that the original source remains. The source record then permits only
keep-isolated or a separate confirmed source delete; it cannot remigrate the
same source record. A confirmed original-source delete says candidate and main
DB state were unaffected. Source delete is allowed while the candidate is pinned,
applied, or already deleted; it is blocked while candidate apply/delete is active
or needs help. Candidate/ref pins do not time-expire before apply or
explicit candidate delete; public UI never exposes refs/receipts/private
bindings.
Source/candidate authority is retained as one pair for as long as either source
or candidate content, pins, allowed actions, or recovery depends on it—even past
30 days. Once a candidate has ever been published, only after both are explicitly
Deleted and no dependency remains may one joint 30-day GC remove both records; a
missing counterpart is an error, never a signal to continue. A source whose
persistent lineage is exactly NeverPublished may use source-only 30-day GC after
explicit deletion, but any publish effect/file/receipt blocks that exception.
If Published lineage lacks or mismatches its counterpart,
`credential_artifact_pair_inconsistent` says: “Local quarantine records are
inconsistent. FyAgent retained the remaining record and deleted or reconstructed
nothing.” It offers only local help, exit, and safe-view reload; recreate,
remigrate, apply, delete, GC, and retry are absent, and surviving authority stays
pinned indefinitely.
Safe candidate list/get is backend-authoritative and rediscoverable after reload,
including when the source record is missing. Events only invalidate/refetch the
safe query; they never supply authority. Pair-inconsistent candidate-only views
expose no lifecycle action or private lineage/ref/receipt.
If an old action receipt is superseded, copy says: “A newer candidate action
superseded this earlier request. Nothing was repeated. The current candidate
state is shown.” Only dismiss/review and actions derived from the current safe
view are available; historical controls never render.
`database_upgrade_required` uses the safe predecessor's stable `dbUpgrade`
surface and states: “This data requires a newer compatible FyAgent. This build
inspected only compatibility metadata and did not initialize, migrate, or modify
business data.” It may read only
an atomic compatibility marker or SQLite main-file header, without opening
SQLite or touching DB/WAL/SHM. It permits only local upgrade instructions, a compatible
newer-build install action when a verified local installer is available, or
exit. It offers no continue, config-folder mutation, downgrade, ordinary
rollback, or backup restore. The surface is keyboard/screen-reader accessible
and complete in `zh`, `zh-TW`, `en`, and `ja`.
If marker/header inspection itself is unavailable or malformed,
`database_compatibility_unknown` shows the same bounded surface and says the app
could not safely determine compatibility; it never continues into initialization.
A compatible but abandoned `bootstrap_pending`/`migration_pending` uses safe
reason `interrupted_bootstrap|interrupted_migration` and neutral copy: a prior
startup or migration may be incomplete; this build inspected metadata only.
Actions are local help, compatible-build guidance, or exit—never automatic
resume/init.
A known candidate-apply `replacement_pending` is not treated as a generic
migration. Before business services start, narrow local readback classifies the
exact prior DB as no-effect, the exact target/projection as applied, or any
mixed/unreadable state as needs-help; it never repeats apply. Ambiguous state
keeps normal DB closed and uses the existing compatibility-unknown plus candidate
needs-help surfaces.
`universal_codex_transfer_unavailable` says the staged import/backup may change a
managed Codex child, so nothing was imported/restored and the artifact remains
isolated until a Universal-to-UCP adapter exists.
`database_maintenance_pending` appears only for an explicit running
import/restore/replacement: new DB work is paused while handles drain. The user
may wait or close; no second mutation is accepted. Drain/lock/reinspection
failure leaves the original DB unchanged and returns the prior safe screen.
It shows exact changes, affected resources, conditional backup/recovery,
credential status, privacy boundary, expiry, warnings, and re-preview reason.
Unknown schema/status fails closed; expiry uses a timer and authoritative check.
Planning may also return `no_change(code=no_changes|target_already_current)`;
this is a successful non-executable outcome with only close/return-to-edit, not a
generation failure.

### R-10 Codex operation semantics

Supported typed intents are `create_only`, `create_and_select`, `edit`, and
`switch`. Edit explicitly distinguishes current/non-current resource sets.
`no_changes`, `target_already_current`, unsupported mode, and unsupported risk
produce no executable Plan. Supported hosts route create/edit/switch through the
same Plan/admission/job flow; ProviderService retains lossless TOML/domain
validation and remains the single writer. Cutover covers both plain and
`_with_result` IPCs plus native tray, profile, provider deep link, old UCP, and
endpoint writers. Tray opens a safe switch Plan request before any proxy/menu
write. Profile Codex deltas fail closed before all profile effects until #41
uses UCP; Codex deep links open/return a draft-to-Plan request before persistence.
Public legacy ProviderService mutation methods also fail closed; an active Plan
or job does not grant write authority, and only its owning worker's private
effect-permit commit can write.

The protected Codex create/edit/switch family includes every target/mode/risk
subcase. Proxy takeover, official-target switch, and critical risk therefore
return their typed unsupported result with zero direct fallback. Legacy routing
is limited to non-Codex apps and separately named Codex operation families that
are not create/edit/switch: delete, import-default, sort/last-used metadata,
proxy failover control, remove-from-live, and official-seed. Any such path that
begins to implement Codex create/edit/switch semantics
must first join this cutover.

Universal save/delete/sync uses one backend operation that binds the current
Universal fingerprint, Provider state epoch, and actual materialized Codex child
before its first write. The renderer cannot authorize with a preflight or chain
legacy upsert/sync IPCs. Allowed non-Codex-only commit structurally excludes the
Codex branch. Existing `get_universal_providers` and
`get_universal_provider` command names return only a redacted safe mutation
view; no plaintext read IPC remains, and raw stored readers are private and
non-serializable. Secret-bearing Universal mutation additionally waits for #35's
exact-SHA adapter and migration to reference-native Universal/child storage.
Closed `UniversalCredentialIntentV1` covers `None`, `Clear`,
`Preserve{opaqueBindingToken}`, and `Replace{secretRef,expectedVersion}`; exact
leases resolve after resource CAS and
before the one-use permit, never persist, and are zeroized after the attempt.
Failure is typed `dependency_unavailable/no_effect` with zero writes.

#35's persisted `UniversalCredentialStorageV1` discriminator lives only in new
reference-native storage, never inside legacy `api_key`. It distinguishes
proven `None`, bound `SecretRef`, and required-but-absent `NeedsLocalRebind`;
safe view/revision/CAS bind the exact variant and only #35 secure rebind may
create `SecretRef`. Forward migration must
establish a newer DB compatibility marker, verify/clear plaintext, and enable
new readers/writers without a mixed writable state. The accepted safe predecessor
then shows “database version too new; upgrade” after marker/header inspection and
before SQLite open/DDL/business-read/write/sync/network activity. Inspection
error is `database_compatibility_unknown` and also fail closed. Downgrade
after migration is unsupported;
ordinary rollback retains the safe parser/adapter/guards. Migration cannot
create an ordinary plaintext database backup. Sync/export/diagnostics/sanitized
backup show credential status only, while remote import preserves local
bindings or commits safe fields as `NeedsLocalRebind`.

Compatibility inspection and SQLite lifetime share a cross-process lock. Fresh
bootstrap is allowed only when DB, marker, WAL, SHM, and journal are all absent;
all migrations/replacements hold the exclusive lock from pending marker through
DB checkpoint/close and ready marker. Existing-DB fallback is allowed only with
no sidecars/hot journal and a valid exact header; lock/WAL/journal/header/identity
errors fail closed. Compatible pending without a live exclusive owner is
`database_compatibility_unknown`, not resume, for bootstrap/migration. A closed
CandidateApply ReplacementPending is the sole exception: it permits only the
narrow pre-service readback/finalization described above and never repeats the
business effect.

Migration remains disabled until a safe `dbUpgrade` predecessor is committed,
released as the minimum supported pre-migration build, and recorded by immutable
`MIGRATION_GUARD_BASELINE_SHA`. Earlier binaries receive only the narrower
successful-inspection pre-schema-write guarantee and may retain legacy recovery
copy/actions or continue on inspection error;
they are not accepted as migration predecessors.

Every SQL export/import, WebDAV/S3 upload/download, app-managed backup create/
list/restore, and legacy backup path uses a safe Universal transfer rather than
raw `settings` credentials. #35 allocates a new `DB_COMPAT_VERSION > 6` and
`db-vN` remote layout with no old/new dual-write. All inbound data stages in a
temporary DB, migrates/validates before main replacement, preserves matching
device-local refs only by stable ID + requirement digest, persists
`NeedsLocalRebind` otherwise, and never lowers the compatibility marker. Existing
pre-safe/unknown app backups are quarantined as
`legacy_credential_backup_blocked`; ordinary/raw restore is unavailable.

All four transfer families join the same Universal Codex impact snapshot before
main replacement. Staged/local membership, actual child presence/epoch/redacted
digest, and projected child digest are compared; `apps.codex=false` does not hide
an orphan child. Before the Universal-to-UCP adapter, any create/update/delete/
membership/projection difference quarantines the whole transfer with zero main
DB/child/epoch/marker/sync/cache/event writes. An allowed no-impact transfer
excludes every staged Universal Codex child row and preserves the exact local
membership and child.

Guard precedence is fixed and side-effect-free: first classify target/mode/risk.
Proxy takeover, official-target switch, or critical risk returns its specific
typed unsupported result. Only a supported normal-mode create/edit/switch request
that entered a legacy write path returns/routes `change_plan_required`.

### R-11 Failure and side-effect evidence

Spies/faults cover business DB, Provider rows, settings/live files, tray/cache,
jobs/events, backups, and outbound Provider adapters. Tests cover writer error,
target/source/secret drift, post-write mismatch, interruption, baseline restored,
target reached, partial/third state, readback unavailable, manual recovery
required/readback-only recheck still inconsistent, and
idempotent retry. A deterministic hook mutates target after validation but before
first effect; writer and managed-write counters must remain zero.

### R-12 Evidence honesty

Evidence labels are `source_report`, `code_audit`, `prototype`,
`runtime_screenshot`, `local_readback`, `native_runtime`, `failure_path`, and
`UAT`. Generated images and mock/browser prototypes are `prototype`, not runtime
or UAT. `liveConfigChanged` is only restart guidance; local readback is not real
Agent usage.

### R-13 Risk, recovery, and support policy

Info/warning use one explicit confirmation; critical is unsupported. Each action
declares recovery mode, backup timing, scope, and limitation. Backup creation is
the first durable effect when present; otherwise the first writer action is the
boundary. Cancellation is allowed in planned/running-precheck only, then an
atomic effect gate closes cancellation before backup/apply. Backups support
manual recovery hints only. Manual recovery required offers a local help route,
readback-only recheck, and redacted diagnostic ID without raw
config/path/secret export.

### R-14 Retention and v1 compatibility

Plan ledger is local sensitive metadata. Actor is an opaque local code. Private
envelopes never enter diagnostics/export. Ready-expired, abandoned, and
invalidated unconsumed Plans are retained at most 24 hours from `expiresAt`,
`abandonedAt`, and `invalidatedAt` respectively. Only terminal jobs and their consumed Plans are
retained at most 30 days from `terminalAt`. Nonterminal or recovery-required jobs
retain the minimum recovery envelope until reconciliation becomes terminal or
the user explicitly confirms local clearance; timed purge never removes them.
Local purge is available subject to that confirmation. Schema-v1 unconsumed Plans are never executed and require re-preview;
terminal v1 jobs remain read-only discoverable. Schema v2 supersedes switch-only
execution without rewriting history.

## Acceptance criteria

- **AC-01:** Normal-mode create-only/create-and-select/edit/switch produce a
  persisted readable preview before any managed write; unsupported/no-op cases
  produce no executable Plan.
- **AC-02:** Snapshots and spies prove preview changes only Plan payload/lifecycle
  metadata; all managed resources, job/event/backup/tray/cache/network counters
  remain unchanged.
- **AC-03:** Public projection round-trips Rust -> persistence -> IPC -> TS via a
  redacted shared fixture. A separate backend-only fixture proves the private
  envelope never crosses IPC. Fixed vectors prove all three digests.
- **AC-04:** Same normalized intent/baseline yields equal semantic digests and a
  distinct Plan ID; any action/resource/secretRef/precondition/recovery/risk
  semantic change changes Plan digest.
- **AC-05:** Every R-06 rejection and persisted non-ready lifecycle outcome has
  typed deterministic output, zero new job,
  zero writer, and zero managed side effect. Same-identity retry returns only the
  owning job.
- **AC-06:** One confirm sends only Plan identity; admission returns planned
  snapshot before execution; worker cannot accept/recompute intent.
- **AC-07:** Deterministic post-admission/pre-effect stored-digest, target/source
  CAS, precondition, and readability faults terminalize the existing owning job
  as `failed + pre_effect_validation_failed + typed reasons + no_effect +
  recovery=none`; Plan remains consumed and writer/backup/managed-write counters
  stay zero, closing TOCTOU without misreporting an admission rejection.
- **AC-08:** Plan/job rediscovery survives renderer reload; missed, duplicate, or
  foreign events do not corrupt state or duplicate apply.
- **AC-09:** Every R-09 projection is independently rendered, keyboard
  accessible, translated in all four locales, and offers only valid actions.
- **AC-09a:** `no_changes` and `target_already_current` render independent
  `no_change` outcomes with no confirm/apply action and do not look like errors.
- **AC-10:** Preview shows resources, conditional backup/recovery, credential
  status, privacy, expiry, warnings, and re-preview reasons without secret/raw
  path leakage.
- **AC-11:** Supported operations use ProviderService as sole writer, derive
  terminal result from authoritative readback, and make no Provider/model request.
- **AC-12:** Focused backend/frontend modules pass before any integration;
  integration/native/failure evidence is generated only after source/design
  freeze.
- **AC-13:** Detailed-design commands reach terminal results for lint,
  typecheck, unit, integration, browser, renderer, native/Tauri, Trellis, and Git
  diff, including operation × mode × failure coverage. Earlier UCP check output is
  not reused because its recorded session lacked terminal readback.
- **AC-14:** Final review maps Issue #55 and related contract ACs to files, tests,
  exact SHAs, and evidence level; branch is pushed/read back, with no main merge
  or deployment.
- **AC-15:** A deterministic cancel-vs-worker race proves cancellation succeeds
  in planned/precheck with zero durable effect, while the atomic gate prevents a
  cancelled result once backup/apply begins.
- **AC-16:** Clock-controlled tests prove ready expiry, abandoned, and
  invalidated 24h anchors plus terminalAt-30d retention and purge,
  and prove nonterminal/recovery-required evidence is never removed by timed
  purge; sentinel
  tests prove private envelope exclusion from diagnostics/export; compatibility
  tests prove v1 unconsumed Plan requires re-preview and terminal v1 job remains
  read-only discoverable.
- **AC-17:** A post-admission secret lease acquisition/lifetime fault leaves the
  Plan consumed and one owning terminal job with
  `failed + dependency_unavailable + no_effect + recovery=none`; writer, backup,
  and every managed-write counter remain zero. UI requires dependency repair,
  then a new preview and confirmation.
- **AC-18:** After dispatching on `schemaVersion`, v2 Rust persisted/IPC and
  TypeScript decoders accept only recovery `none|manual_required` and reject
  every other v2 value. V1 compatibility fixtures map exact legacy
  `not_needed|succeeded -> none` and `recovery_required -> manual_required`
  while preserving the safe legacy result code.
  Backup produces manual hints only. `recheck_change_recovery` has zero writer,
  restore, compensate, backup, and managed-write counters and may update only
  readback/recovery observation plus the corresponding fenced snapshot/event.
- **AC-19:** Latest Plan discovery is a pure read over exact operation scope.
  Revision-CAS abandon changes only unexpired ready lifecycle
  metadata/abandonedAt and has zero
  job/event/writer/backup/tray/cache/sync/managed effects; stale or non-ready
  abandon returns a typed no-change outcome. Injected-clock cases cover
  `now == expiresAt`, stale renderer, and abandon-vs-expiry races; expired wins
  without writing `abandonedAt` or resetting its retention anchor.
- **AC-20:** Exact-source cutover inventory and static callsite scan cover all
  six add/update/switch IPC commands, native tray, profile apply, provider deep
  link, old UCP executor, endpoint writers, and public ProviderService. Every
  supported normal-mode Codex bypass returns/routes `change_plan_required`
  before its first effect; proxy/official/critical cases return their specific
  typed unsupported result first. Per-entry spies prove zero
  provider/writer/effect; tray also has zero
  proxy/menu writes and profile has zero autosave/proxy/MCP/profile writes.
  Positive navigation asserts tray focuses the app and opens the Plan UI with
  exact safe `{operation=switch,targetProviderId}`. Deep link preserves its
  draft ID and every allowed safe field in the draft-to-Plan UI while excluding
  secret values. Navigation/renderer failure still produces zero proxy/menu/
  Provider/endpoint write. Profile renders translated, accessible
  `profile_change_plan_required`, says the whole apply was unsaved, and offers
  only return-to-edit or remove-Codex-delta.
- **AC-21:** A full Codex deep-link fixture maps field-for-field to closed
  `CodexDeepLinkPlanDraftV1`; `enabled` changes only activation intent. API key,
  config/configUrl/configFormat, every usage/script/token field, notes,
  cross-resource fields, unknowns, and `activationApproved=true` are rejected
  together with URL userinfo/query/fragment (including encoded credential
  sentinels), without network, secret staging, Plan insert, Provider draft, endpoint, or
  switch writes. Universal create/edit/duplicate/save-and-sync/delete/manual-sync
  use one revision/Provider-epoch-bound backend mutation rather than renderer
  `upsert -> sync`. A closed Create/Edit/Duplicate/Delete/Sync request enum makes
  ID, expected absence or opaque revision token, Provider epoch, proposed safe
  draft, and sync flag required/forbidden per operation. Backend safe list/get
  supplies the opaque token and redacted view; TypeScript cannot recompute it.
  Its impact snapshot binds the redacted Universal fingerprint,
  prior/proposed Codex membership, and actual `universal-codex-{id}` child
  presence/epoch/redacted digest before any universal/per-app/event/cache/epoch
  or other-app write. `apps.codex=false + existing child`, stale revision,
  two-step interruption, and preflight race are blocked whole-operation; allowed
  non-Codex commit structurally skips Codex save/delete. Legacy write IPCs are
  guarded, all three public Service writers fail closed, and only a private
  action/ID/payload/token/epoch-bound one-use permit commit can write. Stale or
  invalid request, permit forgery/reuse/misbinding, and production visibility
  checks are zero-write. The existing Universal list/get command names return
  only the safe mutation view; plaintext Universal credentials are absent from
  IPC/query cache/events/DOM/logs/diagnostics and raw stored readers are
  private/non-serde. Credential cases cover both separately named v1 intent
  enums and `None|Clear|Preserve|Replace`,
  legacy plaintext migration, reference-native Universal/child persistence,
  ref/version/lease expiry or resolver failure, and attempt-memory zeroization.
  Before #35's exact handoff and migration, only a proven credential-free
  non-Codex operation with no actual Codex child and
  `UniversalCredentialIntentV1=None|Clear` succeeds;
  every secret-bearing variant is typed `dependency_unavailable/no_effect` with
  zero writes. UI disables new Codex membership and renders legacy Codex-linked
  rows read-only.
  Migration/rollback fixtures prove the immutable safe predecessor SHA stops at
  `db_version_too_new` after marker/header inspection but before SQLite open/
  DDL/business reads/writes/sync/network, no ref/binding token is
  interpreted as an API key, downgrade is blocked, no ordinary plaintext backup
  is created, and sync/export/diagnostics/sanitized backup/remote import retain
  only the safe/local projection.
  `database_upgrade_required` is asserted in all four locales with initial
  heading focus, semantic alert/description, fully labelled keyboard actions,
  and no continue/downgrade/rollback/restore action. Its old-binary fixture has
  zero DDL/business-read/DAO/service/write/sync/network activity. Import/restore
  cases distinguish retained local binding from `NeedsLocalRebind`; safe fields
  are committed first, and the
  latter renders `universal_credential_rebind_required/no_effect`, routes only
  to #35 secure entry, reloads the safe view after successful rebind, and proves
  exported/sanitized artifacts contain no ref/token/value sentinel.
  The DB-copy matrix covers SQL, WebDAV, S3, current backup, and existing backup
  fixtures: new remote generation/layout, no dual-write, staging-first migration,
  main/marker unchanged on failure, local-ref row merge, no raw safety backup,
  quarantined legacy artifacts, and exact `None` versus `NeedsLocalRebind`.
  Closed transfer outcomes survive reload. Four-locale accessible UI distinguishes
  `universal_safe_import_rebind_required` (safe fields committed, no child) from
  a later mutation `/no_effect`, and renders every
  `legacy_credential_artifact_blocked` kind with only staged migration, keep
  isolated, or its existing confirmed delete action—never secret preview,
  automatic delete, raw restore, or silent import.
  Binding-key vectors cover provider type, primary slot, sorted consumer apps,
  per-app auth scheme, normalized endpoint, and every mismatch-to-rebind case;
  TS cannot authorize the digest. Artifact fixtures cover multiple same-kind
  items, opaque ID/revision, content/manifest/ETag change, owner/reload/
  interruption, source retention, explicit delete, closed rejection codes, and
  device-local sidecar/main-replacement exclusion. Attempt fixtures cover dual
  owners, pre-effect lease takeover, delete-after-start crash, SecretRef-created
  crash, candidate-publish boundary, post-effect readback, store corruption, and
  prove no create/publish/delete replay. Transfer impact fixtures cover each copy family with
  `apps.codex=false + staged child`, local orphan child, membership and child
  create/update/delete/projected-digest differences, plus no-impact preservation.
  Guard fixtures cover all-absent fresh bootstrap, bootstrap/pending/ready marker,
  missing-marker header fallback, WAL-only newer user-version, hot journal,
  change-counter mismatch, corrupt/truncated/permission/identity/generation/
  lock errors, concurrent migrator at every boundary, zero DB/WAL/SHM touch on
  rejection, and `database_compatibility_unknown` without `Database::init`.
  Every public `CredentialArtifactLifecycleV1` and
  `CredentialArtifactActionOutcomeV1` variant has an explicit total projection
  in `zh|zh-TW|en|ja`, initial focus/alert semantics, keyboard/screen-reader
  actions, reload persistence, and backend-derived allowed actions. This includes
  action-specific preparing/reconciling/needs-help, candidate-published,
  candidate-deleted, original-source-deleted, rejected, pair-inconsistent, and
  pre-effect store-unavailable;
  `Detected` is asserted internal-only. Original-source-deleted copy says the
  candidate/main DB were unaffected. Candidate-deleted source records allow only
  keep-isolated or a separate confirmed source delete and can never remigrate.
  Source-delete fixtures cover candidate Pinned, Applied, and Deleted and prove
  candidate/ref/main state is unchanged after reload; candidate Applying and
  NeedsHelp reject with zero source/candidate/main effect.
  Deterministic races between source delete and candidate apply/delete prove one
  global integrity-lock/CAS winner, no deadlock, and no crossed effects; source
  PreEffect/Reconciling/NeedsHelp blocks candidate actions. Candidate-delete
  crash fixtures read the stored #35 discard attempt and atomically publish both
  terminal records without replay.
  No post-effect migrate/delete/retry control renders. Safe view, IPC, cache,
  events, DOM, logs, and diagnostics exclude path/URL/ETag/digest/receipt/ref/
  value sentinels. Migrate creates a distinct candidate; fixtures prove the
  original source and record survive until separately confirmed delete.
  `database_maintenance_pending` and every public candidate lifecycle/outcome
  variant have exhaustive four-locale/a11y/reload/allowed-action fixtures:
  ready, action-specific applying/deleting, action-specific needs-help, applied,
  both deleted `wasApplied` projections, rejected, and store-unavailable.
  Applying/deleting have no duplicate submit; needs-help has readback-only
  recheck. Apply needs-help separately asserts known `observed_no_effect` copy
  and uncertain `ambiguous|readback_unavailable` copy. Applied does not imply
  original deletion and exposes explicit delete-candidate; its fixture proves
  Applied -> delete -> Deleted keeps main/source unchanged and enables joint GC
  only after source is also Deleted. A response-loss fixture
  repeats `delete_sanitized_candidate` with the original request revision and
  receives the same persisted Deleted snapshot while cleanup/#35 discard counters
  stay unchanged; action/revision mismatch returns
  `candidate_action_conflict|candidate_revision_changed`. Deleted and all other
  candidate public surfaces exclude SecretRef, SecretRef version, creation/
  discard receipt, private content binding, locator, digest, and value sentinels.
  Candidate apply crash fixtures pause after ReplacementPending, main publish/
  fsync, Ready receipt, sidecar acknowledgement, and marker clear. Startup runs only narrow readback:
  exact prior -> needs-help/no-effect, exact target+projection -> Applied, mixed/
  unreadable -> needs-help with normal DB closed; apply/#35/business-service
  counters remain zero. Ready completion identity survives until sidecar ack.
  Exact-prior NeedsHelp and exact-target Applied both persist a marker/attempt/
  outcome/generation acknowledgement; Ready-before-ack and ack-before-clear
  crashes recover, while mismatch/store-unavailable never clear or admit.
  For unresolved apply, crash -> NeedsHelp -> repeated recheck may resolve exact
  target to Applied or exact prior to acknowledged observed-no-effect NeedsHelp;
  after that no-effect ack/receipt clear, every recheck self-loops even if later
  unrelated rows resemble the old target. Delete NeedsHelp may resolve only
  Deleted. Ack bytes never change. Original-request retry returns the attempt
  ledger's exact snapshot while its result revision remains current. A later valid action makes the old receipt
  `candidate_action_superseded` and returns only the current safe view. GC
  fixtures at >30 days retain source-deleted +
  pinned/applied-candidate and live-source + applied-candidate pairs; only a
  global-lock transaction after both Deleted and all dependencies cleared removes
  both records/receipts together for paired history. Never-published source GC
  requires the persisted NeverPublished lineage plus no candidate artifact/
  action/effect receipt; fixtures cover its positive purge, publish-boundary
  crash, Published+missing-counterpart corruption, paired control, and concurrent
  GC/action fencing.
  `candidate_apply_authority_unavailable` fixtures at Ready-before-ack and
  ack-before-clear use neutral prior-or-target/main-closed/no-replay copy, while
  pre-effect store-unavailable alone claims no main/source effect. Pair-integrity
  missing/mismatch fixtures retain the survivor indefinitely and show only help/
  exit/reload. Both projections have `zh|zh-TW|en|ja`, initial focus/alert,
  keyboard/screen-reader labels, reload persistence, private-sentinel exclusion,
  and zero replay/recreate/remigrate/apply/delete/GC/retry counters.
  Candidate safe discovery fixtures cover list/get after source-record loss,
  startup/reload, stale cache plus event-refetch, candidate-only pair-inconsistent
  rendering, and zero-effect mutation attempts without a pre-known candidate ID.
  Integrity-scanner fixtures cover startup and list/get/action preflight,
  global-lock/CAS races (including source-A/candidate-C/source-B split identity),
  overlay-only metadata delta and event-after-commit;
  file/ref/attempt/main/lifecycle counters stay zero and overlay persistence
  failure exposes no actions. Static owning-spec assertions reject `peek source
  ID`, `source-artifact action lock`, relationship-derived recovery locks, and
  enumeration before the stable global lock, so narrower Provider text cannot
  reopen either race.
  Superseded copy/actions are asserted in `zh|zh-TW|en|ja` with initial focus,
  semantic alert/description, keyboard/screen-reader labels, reload persistence,
  dismiss/review plus current-view actions only, no historical controls, and no
  secret/ref/receipt/private-binding sentinel.
  Create/edit/duplicate/save/save-and-sync may open a separate
  app-specific Codex Plan draft and explicitly cancel the Universal operation;
  delete/remove/resync/manual-sync offer no Plan action and require waiting for
  the Universal-to-UCP adapter.

## Dependencies and deferred facts

- #35 exact-SHA handoff is required before secret-bearing create/edit/switch
  production admission, Universal list/edit credential migration and
  reference-native Universal/child storage, and native acceptance can close.
  Until then, fixture/schema work proceeds; only an all-inputs credential-free
  switch or a proven credential-free non-Codex Universal operation with no
  actual Codex child and `UniversalCredentialIntentV1=None|Clear` may be
  eligible. Every secret-bearing variant fails closed with
  `dependency_unavailable`.
- #41 receives an exact-SHA handoff after schema/digest/baseline,
  persistence/read, invalid reasons, and one-confirmation freeze.
- Old UCP is implementation input, not current completion evidence. Duplicate
  `/worktrees/ucp` contains an inverse staged patch and is excluded.

## Resolved product decisions

- Integrated delivery card; related Issues remain independently governed.
- Explicit create-only/create-and-select; no implicit activation.
- Normal mode only; proxy takeover unsupported.
- Official-target switch is unsupported until codex_auth_projection is modeled.
- Info/warning one confirmation; critical unsupported.
- Cancellation ends at first effect; later closure/interruption uses discovery
  and readback-only reconciliation.
- Public projection and private execution envelope have separate privacy/fixture
  contracts.
- Secret-bearing native apply is blocked until #35 exact-SHA handoff.
