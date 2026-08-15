# Change Plan process and state machine

Product-level flow authority for this integrated delivery. Technical types and
file ownership are frozen later.

## 1. End-to-end flow

```text
edit draft
  -> generating preview(intent)
  -> inspect authoritative local baseline
  -> canonicalize intent/baseline/actions/resources
  -> persist private envelope + safe projection as ready Plan
  -> show clean/warning preview
  -> confirm exactly planId + planDigest once
  -> admission_pending
  -> atomic admission + immediate planned snapshot
  -> background worker consumes stored immutable envelope
  -> precheck under exact-payload/CAS writer boundary
  -> resolve exact #35 lease in attempt memory
  -> atomic effect gate
  -> optional backup                    # first durable effect when present
  -> apply ordered actions once         # first durable effect only without backup
  -> read back every affected resource
  -> terminal result or readback-only reconciliation
```

Preview has zero side effect on managed target state. Its only durable write is
Plan payload/lifecycle metadata; it creates no job/event/backup and refreshes no
tray/cache. Admission receives no form draft/intent. Apply never rebuilds
actions from a mutable Provider ID.

## 2. Preview and Plan lifecycle

Renderer-only preview phases: `draft | generating | generation_failed`.

Planning may return non-error `no_change(no_changes|target_already_current)`.
It persists no executable Plan and offers only close or return-to-edit.

| Persisted state | Meaning | Next action |
| --- | --- | --- |
| `ready` | Persisted, unconsumed, unexpired, valid at last inspection | confirm or abandon |
| `abandoned` | User discarded preview; lifecycle metadata only changed | new preview |
| `invalidated` | One or more resource/source/secret/precondition facts changed | re-preview only |
| `expired` | Current time >= expiresAt | re-preview only |
| `consumed` | One owning job is bound | open owning job or new preview |

Invalidated Plan carries sorted `reasons[] {resourceKey, code}`. Caller wrong
digest does not mutate lifecycle status. Old schema-v1 unconsumed Plan requires
re-preview; terminal v1 jobs stay readable.

Renderer reload discovers the latest Plan through safe
`{app,operation,subjectId}` scope; create uses its preallocated Provider ID.
Explicit abandon CASes only matching-revision `ready` with no owning job to
`abandoned` when `expires_at > now`, writes `abandoned_at`, and has zero
job/event/writer/managed effect. At or after expiry, the same transaction
persists/returns `expired`, keeps `expires_at` as retention anchor, and never
writes `abandoned_at`.

## 3. Admission

One transaction/critical section:

1. Load persisted Plan by ID; otherwise `plan_not_found`.
2. Recompute digest from stored canonical envelope and compare stored/caller
   digest; mismatch returns `invalid_digest` without lifecycle change.
3. If consumed and identity matches with owningJobId, return owning job. If a
   legacy/orphaned consumed Plan has no owning job, return `consumed`.
4. Resolve persisted lifecycle: abandoned -> `plan_abandoned`; expired ->
   `expired`; invalidated -> `plan_invalidated` with stored reasons; only ready
   continues. Then require supported schema/canonicalization/operation/mode/risk.
5. If expired, atomically persist `expired` and reject.
6. Reinspect every resource/source/target fingerprint, secretRef metadata, and
   typed precondition. On real drift, atomically persist `invalidated` with
   sorted reasons and reject. Missing #35 metadata/capability returns dependency
   unavailable.
7. Atomically consume, bind owningJobId, and create one planned job referencing
   immutable Plan revision.
8. Return planned snapshot immediately.

Failure before step 7 creates no job, writer call, or managed target side effect;
lifecycle expiry/invalidation metadata is the only permitted write. Same
Plan ID/digest retry returns the owning job; another digest rejects.
After admission, a lease acquisition/lifetime failure terminalizes that owning
job as `failed + dependency_unavailable + no_effect + recovery=none`; it does
not unconsume the Plan. Writer, backup, and managed-write counters remain zero.

## 4. Job lifecycle

```text
admission_pending -> planned
planned -> cancelled                         # allowed before worker precheck
planned -> running/precheck/secret-lease -> cancelled
                                             # allowed before atomic effect gate
        -> pre_effect_validation_failed/no_effect
                                             # digest/CAS/resource/source/precondition/readability
        -> dependency_unavailable/no_effect  # lease failure, existing job
        -> atomic effect gate
        -> running/backup?                   # backup start is first durable effect
        -> running/apply                     # first effect boundary
        -> running/readback
        -> succeeded | warning | failed

interrupted before effect -> interrupted_before_effect/no_effect
interrupted after effect  -> reconciling/readback
                          -> succeeded | warning | failed
```

Step states: `pending | running | succeeded | failed | skipped`. Snapshot
revision/event sequence are monotonic. After first effect, UI can close but may
not claim cancellation. App interruption leaves a discoverable job and next
start performs readback-only reconciliation. `job_not_found` and
`query_unavailable` are query/UI projections, not job states.
The effect gate and cancellation transition share one atomic owner so cancel and
worker cannot both win. Without backup, the first apply write is the effect.

## 5. Terminal truth

| Observation | Classification |
| --- | --- |
| All required target predicates match | `succeeded` |
| Target reached but bounded auxiliary observation unavailable | `warning` |
| Original baseline observed after failed attempt | `failed`, code `baseline_restored`, recovery `none` |
| Secret lease acquisition/lifetime failure before effect | `failed`, code `dependency_unavailable`, observed `no_effect`, recovery `none`; Plan remains consumed |
| Digest/CAS/resource/source/precondition/readability failure after admission and before effect | `failed`, code `pre_effect_validation_failed`, sorted typed reasons, observed `no_effect`, recovery `none`; Plan remains consumed |
| Mixed/third state or partial effects | `failed`, recovery `manual_required` |
| Required readback unavailable | `failed`, code `readback_unavailable`, observed `unknown`, recovery `manual_required` |
| Writer error but target fully reached | `warning`; never retry writer |

`liveConfigChanged` selects only restart guidance. Usage remains
`not_observed`.

## 6. Operation semantics

- `create_only`: exact non-secret Provider payload, expected absence/conflict
  fingerprint, secretRef requirements; no current/live effect.
- `create_and_select`: explicit create plus route/live/common/MCP actions.
- `edit`: exact payload, original identity/version, explicit current or
  non-current affected-resource set, secretRef requirements.
- `switch`: frozen target definition plus current route/live/common/MCP resource
  fingerprints.

`no_changes` and `target_already_current` create no executable Plan. Proxy
takeover, official-target switch, and critical risk return unsupported. Worker consumes frozen envelope.
If writer currently reloads by ID, design adds an exact-payload/CAS seam inside
ProviderService rather than copying write logic.

Before #35, switch is eligible only when target, source backfill, existing live
auth, prepared projection, and recovery inputs are all credential-free. After
#35, the lease is explicit writer input and is never persisted in the Plan/job/
recovery envelope.

### Protected entry routing

- Both plain and `_with_result` add/update/switch IPC commands run the pure
  target/mode/risk classifier first. Proxy takeover, official-target switch, or
  critical risk returns its specific typed unsupported result; only supported
  normal-mode legacy writes return `change_plan_required`.
- A Codex tray Provider click emits a safe switch Plan request and focuses the
  app before any proxy flag, menu, Provider, or live write; the request binds the
  exact safe target and opens the switch Plan UI.
- Profile apply with a Codex Provider delta fails before autosave, proxy, MCP,
  current-profile, or Provider effects until #41 contributes its UCP adapter;
  the whole apply is reported unsaved.
- A Codex provider deep link routes to a draft-to-Plan request before Provider or
  endpoint persistence, preserving draft ID and allowed safe fields but no
  secret values.
- Universal save/delete/sync is one backend mutation, not renderer
  `upsert -> sync`. A closed operation variant carries target/source ID, expected
  absence or backend opaque revision token, Provider epoch, and proposed safe
  draft only where valid. Existing `get_universal_providers` and
  `get_universal_provider` command names return only the redacted safe view and
  are the sole token authority; no plaintext read IPC remains, while raw stored
  readers are private/non-serde. Under the coordinator it binds expected Universal
  fingerprint, Provider epoch, old/new membership, and actual
  `universal-codex-{id}` child presence/epoch/redacted digest before its first
  write. Actual child presence always makes Codex impact true, including
  `apps.codex=false`. Blocked operations write nothing; allowed non-Codex commit
  structurally skips Codex save/delete. Legacy IPC/Service writers fail closed;
  only a private action/ID/payload/token/epoch-bound permit commit can write.
  Secret-bearing mutation also requires #35's exact-SHA adapter and migration:
  `UniversalCredentialIntentV1=None|Clear|Preserve{opaqueBindingToken}|
  Replace{secretRef,expectedVersion}` projects to reference-native
  Universal/child storage. It is a distinct deny-unknown wire enum from the
  backend-private `ProviderCredentialIntentV1`; only #35 may convert it. The
  coordinator resolves exact minimum-lifetime leases after resource CAS and
  before the permit, passes them only to the private commit, and zeroizes them
  after the attempt. Adapter/migration/ref/version/lifetime failure returns
  `dependency_unavailable/no_effect` with zero writes. Before #35, only a proven
  credential-free non-Codex operation with no actual Codex child and
  `None|Clear` may continue; legacy plaintext, `Preserve`, `Replace`, and
  credential-requiring sync are disabled even for non-Codex apps.
  Forward migration writes a closed `UniversalCredentialStorageV1 =
  None|SecretRef|NeedsLocalRebind` plus a fresh DB compatibility marker only
  after legacy plaintext is verified and cleared;
  it never places a ref token in legacy `api_key` or an ordinary plaintext
  backup. `None` is proven credential-free; `NeedsLocalRebind` is
  required-but-absent and participates in safe view/revision/CAS. Once migrated,
  local ref reuse also requires the canonical
  `UniversalCredentialBindingKeyV1` digest over Universal ID, primary slot,
  normalized provider type, and sorted per-app auth scheme/normalized endpoint;
  any mismatch stays `NeedsLocalRebind`. Once migrated, downgrade is forbidden.
  Migration cannot enable until an immutable safe
  `MIGRATION_GUARD_BASELINE_SHA` is the released minimum predecessor. That SHA
  reads an atomic compatibility marker or SQLite main-file header without
  opening SQLite/touching DB/WAL/SHM and stops at `db_version_too_new` before
  business reads/writes/sync/network. Inspection errors return
  `database_compatibility_unknown` and never enter `Database::init`; earlier
  binaries have only a successful-inspection pre-schema-write guarantee.
  A process-lifetime shared compatibility lock closes inspection-to-open; fresh
  bootstrap requires DB/marker/WAL/SHM/journal all absent, while bootstrap,
  migration, and replacement hold exclusive through ready-marker publication.
  Marker-absent existing DB with WAL/SHM/hot journal or invalid header fails
  closed.
  Sync/export/diagnostics/sanitized backup expose requirement status only.
  Dependency outcomes expose only
  `secret_backend_unavailable|credential_migration_required|
  credential_rebind_required`. Import/sanitized restore with no matching local
  binding commits safe fields plus `NeedsLocalRebind`; a later Universal
  mutation returns `credential_rebind_required/no_effect`. #35 secure entry must
  complete before the renderer reloads the safe view and retries.
  SQL, WebDAV/S3, and app-managed backup inputs all stage/migrate/validate before
  main replacement. #35 owns a new `DB_COMPAT_VERSION>6`/`db-vN` layout, no
  dual-write, row-level local-ref preservation, monotonic marker, and quarantine
  of `legacy_credential_backup_blocked`; raw restore/safety backup is forbidden.
  Each quarantined artifact has backend opaque `artifactId+revision`, private
  content/manifest/ETag binding, and persisted owner CAS. List/read are safe;
  migrate/delete revalidate the binding, and a changed artifact stays isolated.
  Authority is one device-local sidecar that survives main replacement. Lease
  takeover is pre-effect only; every create-secret/publish-candidate/delete step
  persists effect-started and an idempotency/receipt slot first. Post-effect
  interruption enters readback-only reconciliation and never reissues an effect.
  Secure migrate publishes a separately named sanitized candidate only; applying
  it is another explicit transfer. It never overwrites/deletes the original or
  main DB. `delete_source` belongs only to confirmed delete. No timed source or
  source-record deletion exists while the original remains.
  Private candidate binding maps candidate ID/revision/generation and each
  Universal binding digest to the pinned SecretRef/version/creation receipt.
  Apply uses global-artifact-integrity→maintenance→DB lock order, revalidates every gate, records
  effect-start, writes reference-native rows once, and reconciles by main marker/
  candidate manifest readback. Duplicate/crash never replays; candidate/ref pins
  persist until applied or explicitly deleted. Applying and needs-help persist
  the action discriminator (`apply|delete_candidate`) and one immutable attempt
  row retains original request revision/attempt/digest through all rechecks.
  Terminal receipt ledger stores result revision plus exact safe snapshot: an
  original apply/delete retry after response loss returns that snapshot even
  after repeated rechecks while its result revision remains current, with no
  cleanup/publish repeat. A later valid action makes the old receipt superseded
  and returns only the current safe view; action or revision mismatch fails typed.
  Candidate deletion CASes the source record to
  candidate-deleted. The original remains isolated and can only be kept or
  explicitly source-deleted; that source record cannot be remigrated.
  Independent source delete uses the same global integrity lock: candidate Pinned, Applied,
  or Deleted permits confirmed source `PreEffect -> Reconciling -> Deleted`
  without changing candidate/ref/main state; candidate Applying or NeedsHelp
  rejects source delete with zero effect. Candidate delete itself may start from
  Pinned or Applied and carries the prior main generation when applied. Source
  PreEffect/Reconciling/NeedsHelp symmetrically blocks candidate actions; an
  already Deleted source does not. Candidate cleanup persists #35 discard attempt
  before effect and atomically closes candidate/source records after readback.
  Candidate apply writes an exact ReplacementPending receipt before one staged
  main publish, then Ready with completion identity. Before services, exclusive
  narrow recovery classifies exact prior/target/ambiguous by readback only and
  never replays. Exact-prior NeedsHelp and exact-target Applied persist the
  closed DbCompletionAck before an exact marker+sidecar CAS clears Ready receipt;
  mismatch/store failure keeps admission closed. A cleared no-effect ack is
  immutable and subsequent recheck self-loops; it cannot attribute a later main
  match to the old attempt. Unresolved apply can resolve exact target/applied or
  exact prior/no-effect; delete recheck can resolve only Deleted.
  Backend safe candidate list/get plus invalidate/refetch events rediscover a
  candidate-only survivor after source-record loss; Inconsistent suppresses all
  mutation controls. Source/candidate/attempt authority is jointly pinned until both
  are Deleted and one global-lock GC transaction proves no file/ref/action/
  recovery dependency, then waits 30 days and purges both together.
  Before main replacement, the transfer compares staged/local Universal Codex
  membership, actual child presence/epoch/redacted digest, and projected child
  digest. Any difference before a Universal-to-UCP adapter quarantines the whole
  transfer with zero main/child/epoch/marker/sync/cache/event effects. Allowed
  no-impact paths drop staged Universal Codex child rows and preserve exact local
  membership/child, including orphan-child detection when membership is false.
  Create/edit/duplicate/save may offer a
  separate app-specific Codex Plan draft while cancelling the Universal
  operation; delete/remove/resync/manual-sync cannot.
- Public legacy ProviderService mutation methods fail closed; an active Plan/job
  grants no authority. Only the owning worker's private effect-permit commit may
  perform a protected Codex mutation.

All Codex create/edit/switch subcases are protected: proxy takeover,
official-target switch, and critical risk return typed unsupported with no
fallback. Prior routing remains only for non-Codex and separately named Codex
families that are not create/edit/switch: delete, import-default,
sort/last-used metadata, proxy failover control, remove-from-live, and official
seed. No protected entry uses absence of an active Plan/job as permission to
write.

## 7. UI projection

| Projection | Message/behavior | Action |
| --- | --- | --- |
| draft/generating/generation-failed | Edit, generate, or retry preview | preview/retry |
| no change | Nothing needs to change / already current | close or return to edit |
| clean/warning preview | Review exact changes and any warnings | one confirm or abandon |
| expired/drift | Preview cannot be reused | re-preview |
| unsupported | Host/schema/mode/risk not supported | close/upgrade; no direct fallback |
| pre-admission secret metadata/capability unavailable | Credential reference cannot be inspected | repair/wait, then re-preview |
| admission pending/planned | Validating or queued before first effect | cancel only while allowed |
| running | Applying and reading back | close UI; no duplicate submit |
| cancelled | Ended before managed effect | return to edit |
| reconciling | Reading state after interruption | wait; never replay |
| terminal dependency unavailable/no effect | Admitted lease could not be acquired; owning job is retained and nothing was written | repair dependency, then new preview and confirmation |
| terminal pre-effect validation failed/no effect | Admitted baseline or stored contract changed before effect; owning job is retained and nothing was written | inspect reasons, then new preview and confirmation |
| profile change plan required | This Profile was not applied and none of its changes were saved because it contains a protected Codex Provider delta | return to edit or remove Codex delta; #41 later adds Plan conversion |
| Universal Codex create/edit unavailable | No Universal or per-app change was saved | cancel Universal operation; optionally create/edit a separate app-specific Codex Provider through Plan |
| Universal Codex delete/remove/resync unavailable | No Universal or per-app change was saved | return/cancel and wait for Universal-to-UCP adapter; no app-specific Plan action |
| Universal credential dependency unavailable/no effect | This Universal change needs credential migration or resolution and nothing was saved | repair/wait for #35 dependency, then reload the safe view and retry |
| Universal safe import needs rebind | Safe non-secret fields were imported; no credential or child projection was transferred | enter #35 secure rebind, then reload the safe view before any mutation |
| Universal credential rebind required/no effect | Imported/restored safe fields were saved without a credential; this attempted Universal mutation saved nothing and this device needs a local binding | enter #35 secure rebind, then reload the safe view and retry |
| Legacy credential artifact blocked | An old SQL/WebDAV/S3/app backup may contain legacy credential data and remains isolated; nothing was imported/restored | run #35 secure staged migration, keep isolated, or use the source's existing confirmed delete action |
| Credential artifact preparing | The named migrate/delete action was accepted but no effect has started | wait or close; no duplicate action |
| Credential artifact reconciling | The named migration/delete crossed its effect boundary and is being read back | wait or close; never retry the effect |
| Credential artifact needs help | Action-specific readback proves no effect or remains ambiguous/unavailable; the source remains quarantined | local help/manual resolution or fenced readback-only recheck; no migrate/delete/retry/replay |
| Credential artifact candidate deleted | Sanitized candidate is gone; original source remains isolated and main DB truth follows `wasApplied` | keep isolated or separately confirm source delete; no remigration |
| Credential artifact source deleted | The original source was explicitly removed; candidate and main DB were not deleted or rolled back | close; candidate actions remain independently governed |
| Credential artifact rejected | The safe rejection code blocked the action and source remains isolated | reload/help as backend-authorized; no automatic retry |
| Credential artifact pair inconsistent | Local quarantine records are inconsistent; FyAgent retained the remaining record and deleted or reconstructed nothing | local help, exit, or safe reload only; no recreate/remigrate/apply/delete/GC/retry |
| Credential artifact store unavailable — pre-effect | Quarantine authority cannot be safely read before effect; sources and main DB remain untouched | repair local store/permissions or exit; no migrate/delete |
| Sanitized candidate ready | A separate sanitized candidate and local bindings are pinned; original source is unchanged | explicitly apply candidate, explicitly delete candidate, or leave pinned |
| Sanitized candidate applying | Apply was accepted; main DB may change or may already have changed | wait or close; no duplicate apply |
| Sanitized candidate deleting | Candidate cleanup was accepted; original and main DB remain unchanged | wait or close; no duplicate delete |
| Sanitized candidate apply needs help — observed no effect | Recovery verified that the main DB is still the exact prior version; candidate was not applied and this attempt will not replay | local help/manual resolution or fenced readback-only recheck; no retry/replay |
| Sanitized candidate apply needs help — uncertain | Main DB apply readback is ambiguous or unavailable | local help/manual resolution or fenced readback-only recheck; no replay |
| Sanitized candidate delete needs help | Candidate cleanup readback is ambiguous or unavailable; original/main are unchanged | local help/manual resolution or fenced readback-only recheck; no replay |
| Sanitized candidate applied | Main DB readback matches candidate and local bindings; original source remains | explicitly delete sanitized candidate recovery material, separately confirm source delete, or close |
| Sanitized candidate deleted before apply | Candidate/pins are gone; main DB and original source are unchanged | close; source can only stay isolated or be explicitly deleted |
| Sanitized candidate deleted after apply | Candidate is gone; already-applied main DB state and original source remain | close; source can only stay isolated or be explicitly deleted |
| Candidate action superseded | A newer candidate action superseded this earlier request; nothing was repeated and the current candidate state is shown | dismiss/review, then only actions derived from current safe view; no historical controls |
| Candidate apply authority unavailable | FyAgent cannot verify local candidate-apply authority; main DB may still be prior or may already contain the candidate, remains closed, and apply will not repeat | local repair/help or exit only; no apply/delete/retry/replay |
| Database maintenance pending | An explicit import/restore/replacement is draining DB work before exclusive maintenance | wait or close; no second DB mutation |
| Database upgrade required | This data requires a newer compatible FyAgent; this build inspected compatibility metadata only and did not initialize, migrate, or modify business data | view local upgrade instructions; use a verified local compatible installer when available; or exit—never continue, downgrade, ordinary rollback, or restore |
| Database compatibility unknown | Compatibility could not be proven, or a prior compatible bootstrap/migration is incomplete; this build inspected metadata only and initialization did not run | local help/permissions, compatible-build guidance, or exit—never continue or auto-resume |
| Universal Codex transfer unavailable | The staged SQL/sync/backup would change a managed Codex child; nothing was imported/restored and the artifact remains isolated | keep isolated and wait for Universal-to-UCP adapter |
| job/query unavailable | Result cannot be read | retry query/local help |
| terminal/readback unknown | Exact applied/warning/baseline/needs-help truth | manual help or readback-only recheck |

Expiry uses timer plus authoritative backend validation. Unknown wire values
fail closed. Manual recovery required links to local help with a redacted
diagnostic ID.

The `database_upgrade_required` row is the stable `dbUpgrade` recovery surface.
It owns initial heading focus, alert/description semantics, labelled keyboard
actions, and complete `zh|zh-TW|en|ja` copy. It exposes no config-folder mutation
or backup-restore action and performs no DDL/business-data query/DAO/service/
sync/writer/network work. Acceptance runs the exact safe
`MIGRATION_GUARD_BASELINE_SHA`; older binaries are not claimed to have this UI.
Transfer/rebind/artifact-blocked projections are likewise complete in four
locales, preserve the backend persisted outcome across reload, and never render
credential material. A blocked artifact remains isolated until an explicit safe
action completes. Every public artifact/candidate lifecycle and action-outcome
variant maps to exactly one row above or to the current lifecycle plus a safe
rejection alert. `Detected` is internal-only. Unknown variants or a missing
`apply|delete_candidate` discriminator fail closed. Deleted rows retain only
opaque identity/revision, `wasApplied`, and safe timestamps; they expose no
SecretRef/version/receipt/private binding or source locator.
Observed-no-effect and uncertain candidate help branches, Applied candidate
delete, and superseded-current-view behavior are complete in `zh|zh-TW|en|ja`,
keyboard/screen-reader accessible, and reload-stable. Superseded never revives
historical actions.
Candidate-apply authority unavailable and pair inconsistent have distinct
neutral copy, the bounded actions above, the same locale/a11y/reload coverage,
and no private lineage/receipt fields.

## 8. Privacy, retention, and evidence

Public projection/job/event/query/log/DOM contain only safe IDs, codes, digests,
timestamps, and redacted labels. Private execution envelope may persist exact
non-secret Provider settings and opaque secretRef requirements, but never crosses
IPC/log/diagnostics. No representation contains secret value, reversible secret
hash, raw live config, absolute path, or unrestricted backend error.

Ready-expired, abandoned, or invalidated unconsumed Plans purge within 24 hours
from their explicit lifecycle anchors. Terminal jobs and their consumed Plans
purge within 30 days from terminalAt. Nonterminal or
recovery-required jobs retain the minimum recovery envelope until terminal
reconciliation or explicit user-confirmed clearance; timed purge never removes
them. Manual remediation can be followed by a fenced readback-only recovery
recheck; no automatic restore/replay exists. Preview/apply makes no Provider/model
request. Generated/mock/browser material is `prototype`; real local readback,
native launch, and Agent use remain distinct evidence levels.

Artifact/candidate authority has a separate joint-retention rule: neither side
can time-purge while the other record/file/ref/action/recovery still depends on
it. Only source Deleted + candidate Deleted with all dependencies cleared enters
one global-lock 30-day GC for a Published lineage; both records and action
receipts purge atomically. Exact NeverPublished lineage alone may source-GC after
explicit deletion, 30 days, and proof of no candidate file/row/ref/action/effect
receipt. Published plus a missing counterpart is corruption/needs-help.
