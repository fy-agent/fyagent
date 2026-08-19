# Issue #55 Change Plan — Technical design

Status: architecture review PASS at revision 23. Product review through revision 18 is
PASS; revision 15 delta review found `0 P0 / 3 P1 / 1 P2`, and revision 15b
re-review closed those items but found `0 P0 / 1 P1 / 0 P2`; revision 15c
closed the remaining transition gap and passed product review. Architecture
round 15 found `0 P0 / 3 P1 / 0 P2`; revision 16 closes those durability gaps.
Its product delta review found `0 P0 / 2 P1 / 1 P2`; revision 16b closed those
presentation/action gaps and passed product review. Architecture round 16 found
`0 P0 / 2 P1 / 0 P2`; revision 17 closed those acknowledgement/lineage gaps,
but product delta found `0 P0 / 2 P1 / 0 P2`; revision 17b closes the two safe
failure-projection gaps.
Architecture round 17 then found `0 P0 / 2 P1 / 0 P2`; revision 18 closes the
acknowledged-no-effect recheck and candidate-discovery gaps.
Architecture round 18 found `0 P0 / 1 P1 / 0 P2`; revision 19 replaces disputed
relationship-derived locks with one stable global artifact-integrity lock.
Architecture round 19 found `0 P0 / 1 P1 / 0 P2`; revision 20 removes two stale
Provider-owner sequences so recovery/scanning acquire that stable lock before
any ID peek or enumeration and retain it through publication.
Detailed-design round 1 found one additional stale owning-spec phrase;
revision 21 replaces it with the exact global integrity-lock lifetime and adds
the phrase to static rejection assertions. No behavior changes from revision 20.
Architecture round 21 passed `0 P0 / 0 P1 / 0 P2`. Revision 22 clarifies that
the early #41 design receipt is non-consumable and the later exact source SHA is
the only integration authority.
Architecture round 22 found `0 P0 / 2 P1 / 0 P2`; revision 23 makes the design
notification non-blocking and freezes a closed exact-SHA compatibility PASS
predicate for the consumable handoff.
Architecture
review round 1 found `2 P0 / 6 P1 / 1 P2`; round 2 closed both P0 but retained
`4 P1`; round 3 retained `4 P1 / 1 P2`; round 4 retained `3 P1`; round 5
retained `2 P1`; round 6 retained `2 P1`; round 7 retained `1 P1`. The revision
8 review retained `2 P1`, round 9 retained `2 P1`, round 10 retained `2 P1`, and
round 11 retained `3 P1`, round 12 retained `4 P1`, round 13 retained `3 P1`, and
round 14 retained `3 P1 / 1 P2`, round 15 retained `3 P1`, and round 16 retained
`2 P1`. No implementation,
test, build, browser, server, or runtime verification has run in this design
phase; two SHA-256 contract vectors were generated from frozen bytes.

## 1. Objectives and invariants

The design extends the existing UCP switch foundation at handoff `6859e9ce` and
source freeze `ca552f4d`. It keeps one logical Plan/job/event ledger, one Provider
writer, one Rust-to-TypeScript wire contract, and one user confirmation. It adds
the complete v2 schema, exact execution envelope, typed lifecycle/admission,
public read/discovery, and the Codex create/edit operation seams.

Hard invariants:

- Preview changes no managed target and initiates no Provider/model request,
  process, tray/cache refresh, job/event, or backup.
- Public projection is redacted; private envelope is backend-only; neither holds
  secret values.
- Admission receives only Plan identity and returns a planned snapshot before
  execution.
- Writer consumes exact validated semantics; no post-precheck Provider ID reload
  may change meaning.
- ProviderService remains the sole Provider writer.
- Query snapshot is authority; events only invalidate/refetch.
- Unknown outcomes reconcile by readback and never replay a writer.
- A persisted worker claim and revision CAS make worker, cancel, query, and
  reconcile mutually ordered; a query never steals a live job.
- The three Plan ledger tables and their private envelope are device-local:
  business sync, SQL/diagnostic export, remote import, and app-managed backups
  cannot copy or overwrite them.
- #35 owns concrete SecretBackend. No production create/edit capability
  enablement or renderer routing until its exact-SHA handoff plus redacted
  list/edit DTO boundary is integrated.

## 2. Options considered

### Option A — Extend the existing UCP core in place (selected)

Keep `change_plans`, `change_jobs`, and `change_job_events` as the only logical
ledger. Add versioned v2 payload/lifecycle/job fields additively, split the large
orchestrator into closed operation modules, and add an exact-payload/CAS seam to
ProviderService.

Benefits: reuses proven switch admission/readback, preserves terminal v1 history,
avoids a competing state machine, minimizes IPC/UI migration, and supports
incremental rollback. Cost: compatibility code must distinguish physical v1
columns from logical v2 payload/lifecycle.

### Option B — Build independent v2 tables/service and migrate later

Create a new Plan store/job state machine for create/edit/switch.

Rejected: duplicates ownership, risks divergent replay/recovery semantics, forces
#41 to choose between ledgers, and violates the single-owner contract. Cleaner
tables do not justify the operational split.

### Option C — Wait for all of #35 and #41, then redesign together

Rejected as the default: it would block the independent #55 schema/digest/
baseline/read API and switch hardening. The chosen plan freezes an opaque secret
port now, keeps secret-bearing production admission fail closed, and hands the
shared contract to #41 early. Concrete create/edit production wiring waits only
at its named dependency gate.

## 3. Module architecture

```text
Renderer draft
  -> typed ChangePlanRequest (no persistence in renderer)
  -> Tauri create_change_plan
  -> ChangePlanService
       -> CodexProviderOperationAdapter (closed enum)
            -> side-effect-free inspectors
            -> ProviderMutationPreparer (pure validation/normalization)
            -> CodexProjectionPlanner (pure expected bytes/projections)
       -> SecretRefPort (opaque metadata only; fixture until #35)
       -> CanonicalDigestV2
       -> ChangePlanStore (one logical v1/v2 ledger)
  <- PlanPublicProjection

confirm(planId, planDigest)
  -> ChangePlanAdmission
       -> persisted envelope digest check
       -> lifecycle/idempotency decision
       -> resource/source/secret/precondition reinspection
       -> consume + owning job + planned snapshot atomically
  <- planned job immediately

ChangeJobWorker
  -> CAS claim {ownerInstanceId, workerEpoch, phase, revision}
  -> cancellation/effect gate
  -> ProviderService exact-payload/CAS writer seam (one writer)
  -> authoritative per-resource readback
  -> durable snapshot/event
  -> readback-only reconciliation after interruption
```

### 3.1 Change Plan core

Move versioned contract/canonicalization/lifecycle/admission primitives into a
`change_plan` module tree while retaining the existing public service owner:

- `contract`: v2 Rust DTOs, closed enums, visibility projections.
- `canonical`: deterministic construction and three domain-separated digests.
- `store`: DAO-facing records, lifecycle/idempotency/retention operations.
- `admission`: ordered decision table and atomic job creation.
- `worker`: cancellation/effect gate, operation dispatch, readback/reconcile.
- `codex_provider`: create/edit/switch adapter and resource matrices.

The adapter set is a closed enum, not a dynamic registry. WorkBuddy is not added.

### 3.2 Provider-owned seams

Extract pure preparation used by both legacy add/update and Plan inspection:

```text
prepare_provider_mutation(draft, original?, policy)
  -> PreparedProviderMutation {
       normalized non-secret provider payload,
       credential intent,
       warnings,
       expected resource actions
     }
```

Create policy is `StoreOnly | MakeCurrent | LegacyIfNoCurrent`; Change Plan uses
only the first two. Legacy direct add retains its current compatibility policy
until the production entry is switched.

Split Codex projection from commit:

- planner: pure expected auth/config/catalog/common/MCP projections from injected
  local snapshots; it cannot write, invoke Codex, fetch, or mutate caches.
- committer: existing IO under ProviderService, fed the admitted exact envelope.

Custom endpoints remain local draft until Provider apply; the owning Provider DB
transaction replaces the endpoint set atomically. Form speed tests may probe a
user-selected endpoint but cannot persist endpoint rows before Plan apply.

### 3.3 SecretRef port

Technical placeholder, not a second credential system:

```text
OpaqueSecretRef
SecretRefVersion
ProviderCredentialIntentV1 =
  None
  | Preserve { secretRef, expectedVersion }
  | Replace { secretRef, expectedVersion }
  | Clear
SecretRefPort.inspect(requirement) -> presence/version status
SecretRefPort.resolve_for_apply(requirement) -> dependency-owned lease
```

`ProviderCredentialIntentV1` is a schema-v1, deny-unknown, backend-private
Change Plan enum. Its canonical domain is
`fyagent.change-plan.provider-credential-intent.v1`; the exact variant tag and
opaque ref/version fields participate in intent/Plan digests but never enter the
public projection. `None` is the only credential-free variant. It is not the
Universal wire enum below and no serde cast or implicit conversion exists.

Only types, fixtures, and fake counters exist before #35 handoff. The generic
Change Plan commands remain registered so capability discovery and rejection
have one wire meaning. `get_change_plan_capabilities` marks create/edit and any
secret-bearing switch disabled until resolution, migration, and redacted
list/edit DTOs are available. Disabled requests return
`rejected(dependency_unavailable)` before a ready Plan or job exists; the UI does
not route into confirmation, and there is no plaintext/direct-mutation fallback.

Before #35 integration, a switch is executable only if every target, source
backfill, existing live-auth, prepared projection, and recovery input is proven
credential-free. Merely having plaintext/unknown credentials in any of them
makes preview reject with `dependency_unavailable`; no ready Plan is created.
After #35, switch/create/edit bind the frozen ref identity/version in the private
envelope. Preview/admission inspect only presence/version metadata. The worker
passes only the exact requirements and an effect-gate handle. The outer
ProviderService seam owns one coordinator critical section and performs all
resource/source/precondition/readability rechecks, dependency-owned
minimum-lifetime lease resolution, effect CAS/permit, private commit, and
zeroization on every exit. The worker performs the pure stored-envelope/digest
check before the call but never observes or passes a lease. A lease is scoped to
one attempt and is never persisted/logged.

Metadata absence or ref/version mismatch before admission rejects with no job.
A resolve/lifetime failure after admission terminalizes the existing job as
`status=failed`, `resultCode=dependency_unavailable`,
`observedState=no_effect`, `recovery=none`; the Plan remains consumed and is
never re-labelled invalidated. Writer, backup, and managed-write counters remain
zero. Recovery envelopes may store only ref/version metadata
or a #35-owned sealed recovery artifact, never a lease or plaintext value.

## 4. Contract model

### 4.1 Private record and public projection

`StoredChangePlanV2` owns:

- public projection JSON;
- private execution envelope JSON;
- schema/canonicalization/operation versions;
- three digests;
- lifecycle status/reasons/revision/owning job;
- timestamps and retention metadata.

Public JSON never contains exact Provider settings, content fingerprints, CAS
tokens, absolute paths, or secretRef identity/version. Private JSON may contain
exact non-secret settings and opaque ref/version requirements, but no secret
value. A single total conversion function creates the public projection from the
typed private model; DAO and renderer do not reconstruct presentation defaults.
Serialization and IPC sentinel tests reject any occurrence of injected secret,
path, ref-identity, or private fingerprint markers.

The resource DTOs are deliberately distinct:

```text
AffectedResourcePublic {
  resourceKeyCode, kind, actionCode,
  readbackCode, effectBoundaryCode,
  recoveryModeCode, limitationCode
}

ResourceExpectation {                 // backend only
  resourceKey, kind, action,
  expectedFingerprint, sourceVersion, casToken,
  readerCode, writerCodes[], readbackCriticality,
  effectBoundary, recoveryEnvelope,
  syncDisposition
}
```

### 4.2 Canonical JSON and digests

Contract ID: `fyagent.change-plan.v2`. Authoritative canonical input types are
closed Rust structs with `deny_unknown_fields`; all schema fields are emitted.
An optional value is encoded as an explicit JSON `null`, never by omission.

```text
IntentDigestInputV2 {
  contractId, schemaVersion, canonicalizationVersion, operationVersion,
  operationIntent: one of {
    create { app, providerId, activation, nonSecretDefinition,
             endpointSet, credentialRequirements },
    edit   { app, providerId, nonSecretDefinition,
             endpointSet, credentialRequirements },
    switch { app, targetProviderId, frozenTargetDefinition,
             credentialRequirements }
  }
}

BaselineDigestInputV2 {
  contractId, schemaVersion, canonicalizationVersion,
  resourceExpectations[], sourceVersions[], preconditions[]
}

PlanDigestInputV2 {
  contractId, schemaVersion, canonicalizationVersion, operationVersion,
  operation, intentDigest, baselineDigest,
  orderedActions[], affectedResourceCodes[],
  credentialRequirementDigests[], preconditionCodes[],
  recoveryModes[], riskCodes[], warningCodes[], effectBoundaries[]
}
```

- Encoding: UTF-8 JSON, no insignificant whitespace.
- Object keys: sorted by UTF-8 byte order.
- Semantic arrays preserve order. Set-like inputs are sorted by their declared
  stable key after recursively canonicalizing each member; duplicate keys are a
  validation error, not silently de-duplicated.
- Scalars: strings/booleans/null and signed 64-bit integers only; floating values
  and integers outside `i64` are rejected.
- Strings: JSON escaping, no implicit Unicode normalization.
- Dynamic non-secret Provider JSON is recursively converted to the same value
  domain. Unknown secret-shaped fields are rejected by the redaction schema.
  Codex TOML is parsed into the typed prepared projection before canonicalization;
  raw TOML text/comments are not intent semantics. Live-file baselines use exact
  byte fingerprints so any outside edit is detected.
- Hash: `SHA-256(domain || 0x00 || canonical_bytes)` rendered
  `sha256:<64 lowercase hex>`.

Domains:

- `fyagent.change-plan.intent.v2`
- `fyagent.change-plan.baseline.v2`
- `fyagent.change-plan.plan.v2`

Credential requirements in canonical input contain opaque ref identity/version
only inside the backend model. Public JSON receives a safe requirement code and
label; the Plan digest binds the private requirement through its domain-separated
digest. Plan digest excludes Plan ID, timestamps/expiry, actor, safe display
labels, localization, lifecycle status, owning job, and presentation order.

Fixed vectors live as one language-neutral fixture read by Rust and TypeScript.
It includes canonical bytes and all three expected digests for `create_only`,
`create_and_select`, `edit`, and `switch`, using synthetic secret refs only. Rust
alone computes authoritative digests; TypeScript validates the returned
shape/vector metadata and never authorizes apply by recomputing a digest.
Admission decodes the persisted private envelope into these typed inputs,
reconstructs all three canonical byte sequences, and compares the digests; it
never authorizes from stored digest strings alone.

### 4.3 Resource model

Supported normal-mode matrix:

- create-only: Provider row + endpoint set; no current/live effect.
- create-and-select: create-only plus DB/device current, live auth/config/catalog
  safe projections, common config, managed MCP as the prepared writer declares.
- edit non-current: Provider row + endpoint set only.
- edit current: Provider row/endpoints plus every prepared current/live/common/MCP
  effect.
- switch custom target: current route, source/target definition predicates,
  device/current/live/common/MCP effects declared by preparation.

Official target, proxy takeover, and critical risk reject before an executable
Plan is persisted.

### 4.4 Frozen resource, writer, and recovery ownership

Every FyAgent-managed mutation of a listed resource takes the shared
`ChangeMutationCoordinator` and increments a durable `provider_state_epoch`.
Plan baselines bind that epoch plus per-resource fingerprints. This includes
legacy direct add/update/switch during migration, official-seed insertion,
endpoint DAO commands, settings current writers, WebDAV/S3 import, and database
restore. A path that cannot participate is disabled while a Plan is ready or a
job is nonterminal. External processes editing Codex files cannot take this lock;
the contract promises effect-gate and post-effect detection, not an impossible
zero-width external race window.

The epoch SSOT is an app-scoped row in a device-local
`change_coordination(scope PRIMARY KEY, state_epoch, worker_instance_epoch,
updated_at)` table; Codex Provider uses scope `codex_provider`. The table is in
the same sync-skip/local-preserve/export/sanitized-backup class as the ledger.
Every managed mutation increments `state_epoch` inside the coordinator before
releasing its lock. Managed import/restore first preserves the local value and
writes `max(local_before, restored_value_or_zero) + 1`; it never accepts a
remote epoch. Restore is rejected while a Plan is ready or job is nonterminal.
Preview, admission, and effect gate all read this same row.

Operation/resource inclusion is closed:

| Operation | Required mutation resources | Required read-only predicates |
| --- | --- | --- |
| create-only | target Provider row, target endpoint set | provider ID absent, provider epoch, secret requirements |
| create-and-select | create-only resources, optional source backfill, DB current, device current, Codex catalog/auth/config, common config, managed MCP | source/target identity, proxy not takeover, provider epoch, all live/source fingerprints |
| edit non-current | target Provider row, target endpoint set | target identity/version, provider epoch, secret requirements |
| edit current | edit resources, Codex catalog/auth/config, common config, managed MCP | DB/device current remain target, proxy not takeover, provider epoch, all live fingerprints |
| switch custom | optional source backfill, DB current, device current, Codex catalog/auth/config, common config, managed MCP | source and frozen target definitions, provider epoch, proxy not takeover, secret requirements |

`optional source backfill` is declared during preparation when current live state
would update the source Provider; when absent it produces no hidden write. Current
markers that an edit does not change remain required CAS predicates rather than
mutation actions.

| Resource family | Authoritative reader / all managed writers | Baseline and effect-gate CAS | Ordered action | Readback | Recovery and sync disposition |
| --- | --- | --- | --- | --- | --- |
| Provider row / endpoint set | SQLite DAO / ProviderService prepared apply, legacy add/update, official seed, endpoint commands, import/restore | row absence or definition+endpoint digest, row version, provider epoch | 1 source backfill; 2 target row+endpoint transaction | required, exact normalized definition and endpoint set | pre-effect DB snapshot; `manual_required`; defer business sync |
| DB current | SQLite current DAO / ProviderService switch/select, import/restore | app-scoped current ID+version, provider epoch | 3 | required | snapshot; `manual_required`; defer business sync |
| device current | non-repairing settings reader / ProviderService and settings writers | exact file bytes + identity/version | 4 | required | staged replacement + old bytes; `manual_required`; no business sync |
| Codex catalog/auth/config | injected exact-byte readers / Codex committer and any live-config command | existence + SHA-256 bytes + source version for each file | 5 catalog, 6 auth, 7 config, all bytes staged before first rename | required per file; unavailable is never green | declared old-byte backup; partial write is `manual_required`; no automatic replay |
| common config | non-repairing file reader / common-config writer, import/restore | exact file bytes + version | 8 | required when declared | old-byte backup; `manual_required`; defer business sync only after safe terminal |
| managed MCP | pure projection reader / MCP projection writer, import/restore | normalized managed block digest + version | 9 | required when declared | old managed-block backup; `manual_required`; defer business sync only after safe terminal |
| proxy state | proxy status/backup reader / proxy service only | must prove normal mode and no takeover before Plan and effect | never mutated in first slice | required precondition | mismatch invalidates; no recovery action |

Steps 1–4 that are SQLite-backed use the narrowest available transaction;
cross-file steps 5–9 are not claimed atomic. All bytes are prepared and backups
durably recorded before the first rename. Readback classifies actual state after
any error. The first v2 slice has no automatic `inverse`, `compensate`, or backup
restore operation. Public recovery codes are only `none` and `manual_required`.
Backups are evidence/input for the displayed manual recovery hints, not a promise
that FyAgent restores them. After manual remediation,
`recheck_change_recovery(jobId, expectedRevision)` performs fenced readback only;
it never calls a writer. A safe classified state clears quarantine and enqueues
at most the one allowed final sync. Otherwise quarantine and evidence remain.

Business-table auto-sync notifications are suppressed and coalesced for the
entire effect/readback/recovery window. A fully read-backed safe terminal state
enqueues exactly one final business-state sync. No-effect failure enqueues none;
partial/unknown/recovery-required state quarantines sync until controlled
manual remediation plus a safe readback-only recheck, or explicit user
disposition. The local Plan/job/event tables are never part of that payload.

## 5. Persistence and compatibility

Do not claim schema v17. Continue additive, idempotent v16 initialization using
the existing `add_column_if_missing` pattern. Every new column is nullable with
no SQL default so an existing row is unambiguously v1. A v2 DAO write validates
all logically required fields before its transaction; `schema_version = 2` is
the row discriminator.

### `change_plans` additive v2 fields

- `schema_version INTEGER`, `canonicalization_version TEXT`,
  `operation_version TEXT`
- `public_projection_json TEXT`, `execution_envelope_json TEXT`
- `intent_digest TEXT`, `lifecycle_status TEXT`,
  `lifecycle_reasons_json TEXT`
- `plan_revision INTEGER`, `owning_job_id TEXT`, `actor_code TEXT`,
  `source_versions_json TEXT`
- `abandoned_at INTEGER`, `invalidated_at INTEGER`

The physical legacy `status` CHECK remains `ready|consumed`; v2 lifecycle uses
`lifecycle_status`. A v2 row always writes legacy `status='consumed'` and
`operation='v2_managed'`, plus inert redacted placeholders for other mandatory
legacy columns. Thus an old binary either rejects it as consumed or fails to
decode the unknown operation; it can never execute a v2 row. v1 rows have
`schema_version IS NULL`, derive lifecycle from legacy status, and contain no v2
envelope. No second Plan table is created.

### `change_jobs` additive v2 fields

- `schema_version INTEGER`, `operation TEXT`, `terminal_at INTEGER`,
  `observed_state TEXT`
- `effect_started_at INTEGER`, `cancel_state TEXT`,
  `recovery_envelope_json TEXT`
- `owner_instance_id TEXT`, `worker_epoch INTEGER`, `worker_phase TEXT`,
  `owner_heartbeat_at INTEGER`

### `change_coordination` device-local table

```text
scope TEXT PRIMARY KEY
state_epoch INTEGER NOT NULL CHECK(state_epoch >= 0)
worker_instance_epoch INTEGER NOT NULL CHECK(worker_instance_epoch >= 0)
updated_at INTEGER NOT NULL
```

The Codex Provider row is seeded monotonically for scope `codex_provider`.
Increment fails closed at signed 64-bit exhaustion. It is never synchronized,
exported, restored from remote state, or included in a sanitized backup.

Existing status has no SQL CHECK and can store cancelled/reconciling. Events stay
append-only and monotonic. Every job transition transaction CASes
`revision + status + effect_started_at + cancel_state + worker_epoch`, increments
revision and eventSeq, updates the snapshot, and appends exactly one event.

Compatibility is fail closed:

| Stored state | New v2 code | Old v1 code after downgrade |
| --- | --- | --- |
| v1 ready Plan | `unsupported_schema`; force re-preview | unchanged v1 behavior |
| v1 consumed + terminal job | compatibility read projection | readable as before |
| v1 consumed + nonterminal/orphaned job | claim only for v1 predicate readback; terminalize without writer replay | old reconciliation behavior |
| v2 Plan/job | full v2 behavior | Plan is consumed/unknown and cannot execute; history need not be readable |

Rollback therefore promises fail-closed safety, not old-version readability of
v2 history. Historical v1 payloads are never rewritten.

Recovery decoding dispatches on row `schema_version` before parsing the enum.
The v1 compatibility decoder accepts all three exact legacy values and projects
`not_needed -> none`, `succeeded -> none`, and
`recovery_required -> manual_required`, while retaining the original safe
legacy recovery/result code in the compatibility projection. The v2 persisted
and wire decoders accept only `none|manual_required`. Three shared v1 fixtures
cover those mappings; they are not passed through the strict v2 decoder.

The three ledger tables and `change_coordination` are added to WebDAV/S3 sync
skip and local-preserve lists. Remote import cannot delete, insert, or overwrite
them. Normal SQL export and diagnostic bundles exclude the tables entirely;
there is no raw private envelope export option. Automatic app-managed database
backups are produced as sanitized SQLite copies with the four tables removed
before atomic publication, never by publishing the raw copied file. Upgrade
maintenance removes or rewrites pre-feature app-managed backups that contain
those tables. User-created external filesystem copies are outside application
retention claims.

Retention uses an injected clock and does not depend on a later read changing
lifecycle. Ready rows with `expires_at <= now` become purgeable 24 hours from
`expires_at`; abandoned and invalidated rows use `abandoned_at` and
`invalidated_at` respectively. A terminal job and its Plan use only the owning
job's `terminal_at` as the 30-day anchor. Nonterminal or recovery-required jobs
and recovery envelopes are never timed-purged. User-confirmed clearance is a
separate typed operation and also scrubs any application-managed legacy backup
identified by the upgrade inventory.

## 6. Admission, effect gate, and race closure

Admission follows the product decision order and one SQLite transaction for
consume/job creation. Fresh inspection happens under the shared Change Plan /
Provider mutation coordination lock. The admission transaction creates an
unowned `planned` job; it never executes work inline.

The native process owns an exclusive `ChangeWorkerLease` (process lock plus a
durable monotonically increasing instance epoch). A freshly scheduled worker
CAS-claims the new job from `{planned, owner=null, effect=null}` to
`{running, ownerInstanceId, workerEpoch, phase=pre_effect}`. A query never
claims, advances, or reconciles a live job. Heartbeats are diagnostic; expiry
alone is not permission to steal an owner in the current live process.

Before first durable effect, the claimed worker re-decodes the private envelope,
re-computes all three digests, terminalizes an integrity failure as the typed
pre-effect/no-effect outcome, and otherwise calls:

```text
ProviderService::apply_prepared_change(
  prepared_exact_payload,
  expected_resource_fingerprints,
  secret_requirements,
  effect_gate
)

ProviderService::commit_prepared_change(       # private IO seam
  prepared_exact_payload,
  expected_resource_fingerprints,
  dependency_owned_secret_leases,
  effect_permit
)
```

Inside the same Provider-owned critical section:

1. re-check expected fingerprints, provider epoch, source versions,
   preconditions, and readability without repair/write;
2. on any CAS/resource/source/precondition/readability failure,
   CAS-terminalize the existing job as
   `failed + pre_effect_validation_failed + typed reasons + no_effect +
   recovery=none`; keep the Plan consumed, keep writer/backup/managed-write
   counters zero, and require a new preview;
3. call #35 `resolve_for_apply`, verify exact ref/version, capability, and minimum
   lease lifetime, and hold the returned lease only in attempt memory;
4. on resolver failure, CAS-terminalize the existing job as
   `failed + dependency_unavailable + no_effect + recovery=none` and stop;
5. atomically arbitrate cancellation vs effect start with one job CAS and obtain
   the unforgeable `effect_permit`;
6. mark `effect_started_at` and `worker_phase=effect` in that CAS;
7. call the private commit seam with exact payload/CAS, resolved leases, and
   effect permit; it creates the recovery envelope/backup as first effect and
   executes once under sync suppression;
8. zeroize/release all leases on every exit path.

Cancellation CASes the same pre-effect tuple. Exactly one transition from
pre-effect succeeds: `cancelled/no_effect` or `effect_started`. After effect
start, cancel returns `too_late`; it cannot change job ownership or writer state.
Every subsequent phase/snapshot/event write carries the claimed worker epoch and
current revision, so a stale worker cannot overwrite a newer terminal state.

The writer does not reload Provider semantics by ID. An ID may identify a row for
CAS, but exact payload and expected version come from the envelope. A test-only
race hook runs between admission and effect gate; target mutation terminalizes
the existing owning job with the typed pre-effect/no-effect result above, not an
admission rejection, and leaves Provider writer/effect counters zero.

`apply_change_plan` performs admission and schedules the worker, then returns the
planned snapshot. It no longer waits for terminal execution. Worker emits only
`{jobId,eventSeq}` after durable snapshot updates.

## 7. Readback and reconciliation

Each operation adapter owns expected target/baseline predicates for every
affected resource. Readback is independent of mutation result. Classifier
outputs the fixed terminal truth from the product state machine.

`get_change_job` and polling are pure snapshot reads. If the stored owner is the
current live instance, they return its progress without waiting for the writer
lock or triggering reconciliation.

Only startup after acquiring the exclusive process lease, or an internal worker
supervisor that proves its current-process task absent, may claim orphaned work
with a transition CAS:

- planned or running with `effect_started_at IS NULL` becomes terminal
  `interrupted_before_effect/no_effect`; it is not resumed or executed;
- effect-started work becomes owned `reconciling` at a new worker epoch and uses
  authoritative readback only;
- an owner that is still the current registered task is never stolen;
- an ownership/revision CAS miss reloads and stops instead of guessing.

Reconciliation never calls Provider writer. Required unavailable readback is
failed/unknown/recovery-required, never green. Its terminal write and event are
one revision/eventSeq CAS transaction.

## 8. IPC, TypeScript, query, and UI boundary

Commands:

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

`PlanCreationOutcome` is `ready(plan) | no_change(code) | rejected(code,
reasons[])`. `ChangePlanCapabilities` reports each closed operation as enabled or
disabled with a stable reason. Registered-but-disabled operations and legacy
plaintext secret state return `dependency_unavailable`; they do not use a
missing-command code. The legacy switch command can remain as a narrow wrapper
during source migration, then is removed after all consumers use the union.

TypeScript decodes `unknown` once in the API facade through Zod. Query keys are
centralized by Plan/job identity. The UI flow stores only IDs and local draft;
reload uses Plan/job discovery. Events with foreign ID, stale sequence, or
unknown shape are ignored and followed by authoritative query.

`operationScope` is a safe closed tuple `{app, operation, subjectId}`. For edit
and switch, subjectId is the Provider ID; create uses the preallocated Provider
ID already bound into intent. `find_latest_change_plan` is a pure read ordered by
`created_at DESC, plan_id DESC` over non-purged matching rows and returns the
authoritative current projection (including ready/expired/invalidated/abandoned/
consumed). It exposes no digest input or private subject data beyond that safe
scope.

`abandon_change_plan` uses the injected clock and transactionally CASes only
`{schema=2,lifecycle=ready,planRevision=expectedRevision,owningJobId=null,
expires_at>now}` to `abandoned`, sets `abandoned_at`, and increments Plan
revision. A physically ready row with `expires_at<=now` atomically becomes typed
`expired`, uses `expires_at` as its retention anchor, and never writes
`abandoned_at`. Missing, revision-conflict, or other non-ready state returns a
typed no-change outcome. It creates no job/event and calls no
writer/backup/cache/tray/sync/managed effect.
Reload discovery plus this command make both ready resume and explicit abandon
reachable without renderer-owned authority.

Frontend state is a view projection, not a second job state machine. All user
text uses the four locale files. Expiry has a timer and backend revalidation.
Unsupported host never falls back to direct create/edit/switch.

Credential dependency outcomes expose only a safe reason enum:
`secret_backend_unavailable | credential_migration_required |
credential_rebind_required`. The first two render wait/repair or #35 migration
guidance; `credential_rebind_required` projects to
`universal_credential_rebind_required/no_effect`, explains that import or a
sanitized restore committed safe fields but intentionally carried no credential,
and that the attempted Universal mutation saved nothing. It routes only into
#35 secure entry. On successful rebind the UI reloads the safe Universal view
before retrying; ordinary forms never collect the secret.

Copy/import uses a closed safe `UniversalCredentialTransferOutcomeV1 =
committed | committed_rebind_required | migration_required{artifactKind} |
rejected{code}`. `committed_rebind_required` renders imported safe fields
without child materialization and offers #35 rebind. `migration_required` maps
`artifactKind=sql_import|webdav_v6|s3_v6|app_backup` to one
`legacy_credential_artifact_blocked` projection: the artifact remains isolated,
ordinary import/restore/sync/export is unavailable, and only #35 secure staged
migration or an existing source-specific delete action may continue. UI never
previews the credential or silently deletes/mutates the artifact.

Quarantine identity is backend-owned and CASed:

```text
CredentialArtifactRecordV1 {
  artifactId,                         // random opaque local ID
  artifactKind,
  revision,
  lifecycle: CredentialArtifactLifecycleV1,
  safeDisplayCode,
  privateSourceLocator,
  privateContentBinding: bytesDigest | manifestHash | etag,
  sourceGeneration,
  candidateLineage:
    NeverPublished |
    Published {
      candidateId, candidateGeneration,
      publishAttemptId, privatePublishReceipt
    },
  pairIntegrity: Intact |
                 Inconsistent {
                   code: candidate_record_missing|candidate_identity_mismatch|
                         lineage_mismatch,
                   detectedAt
                 },
  effectSteps: [
    { stepId,
      kind: create_secret | publish_candidate | delete_source | cleanup,
      status: pending | effect_started | observed_succeeded |
              observed_failed | manual_required,
      effectStartedAt?, idempotencyKey?, privateReceipt? }
  ],
  detectedAt, terminalAt?,
}

CredentialArtifactLifecycleV1 =
  Detected
  | MigrationRequired { reason }
  | PreEffect { attemptId, action: migrate|delete,
                ownerId, ownerEpoch, ownerLeaseExpiresAt }
  | Reconciling { attemptId, action: migrate|delete,
                  ownerEpoch, currentStepId, effectStartedAt }
  | NeedsHelp { attemptId, action: migrate|delete,
                reason: observed_no_effect|ambiguous|
                        readback_unavailable, lastReadbackAt }
  | CandidateReady { candidateId, candidateRevision,
                     candidateGeneration, readyAt }
  | CandidateDeleted { candidateId, candidateRevision, wasApplied,
                       deletedAt }
  | Deleted { attemptId, deletedAt }
  | Rejected { code, rejectedAt }

CredentialArtifactSafeViewV1 {
  schemaVersion: 1, artifactId, artifactKind, revision,
  lifecycle, safeDisplayCode, allowedActions[], detectedAt, updatedAt,
  candidate?: { candidateId, candidateRevision, candidateGeneration },
  safeReasonCode?, pairIntegrityCode?
}

CredentialArtifactActionOutcomeV1 =
  Accepted { artifact, attemptId, action }
  | Rejected { code, artifact?, action }
  | Reconciling { artifact, action }
  | NeedsHelp { artifact, action, reason }
  | CandidateReady { sourceArtifact, candidateArtifact }
  | Deleted { artifactId, revision, deletedAt }
  | StoreUnavailable { code: artifact_store_unavailable, action }

list_credential_artifacts() -> CredentialArtifactSafeViewV1[]
get_credential_artifact(artifactId) -> CredentialArtifactSafeViewV1
migrate_credential_artifact(artifactId, expectedRevision)
  -> CredentialArtifactActionOutcomeV1
delete_credential_artifact(artifactId, expectedRevision)
  -> CredentialArtifactActionOutcomeV1
recheck_credential_artifact(artifactId, expectedRevision)
  -> CredentialArtifactActionOutcomeV1
```

All lifecycle/outcome types are schema-first, internally tagged, and
deny-unknown. Legal revision-CAS transitions are:

| From | Trigger | To |
| --- | --- | --- |
| Detected | private inspection | MigrationRequired or Rejected |
| MigrationRequired | accepted migrate/delete | PreEffect |
| PreEffect | pre-effect rejection/lease expiry | MigrationRequired |
| PreEffect | first persisted effect-start | Reconciling |
| Reconciling | readback | CandidateReady, Deleted, or NeedsHelp |
| NeedsHelp | fenced readback-only recheck | NeedsHelp, CandidateReady, or Deleted |
| CandidateReady + candidate Pinned/Applied | source-specific confirmed delete | PreEffect(delete), then Reconciling; candidate/ref/main unchanged |
| CandidateReady + candidate Applying/NeedsHelp | source-specific delete | rejected `candidate_action_in_progress`; zero source/candidate/main effect |
| CandidateReady | candidate apply | source remains CandidateReady; candidate-specific lifecycle changes |
| CandidateReady | successful candidate delete readback | CandidateDeleted |
| CandidateDeleted | source-specific confirmed delete | PreEffect(delete), then Reconciling |
| CandidateDeleted | migrate/apply candidate | rejected; same source record is never remigrated |
| Deleted / Rejected | any mutation | rejected terminal |

Every transition increments revision; `allowedActions` is backend-derived from
the exact lifecycle. `StoreUnavailable` is an action/query outcome, never a
fabricated lifecycle when the store cannot be read. Illegal lifecycle/step/
receipt combinations fail decoding and expose no action. A post-effect attempt
can never return to `MigrationRequired` or create a new attempt.

`candidateLineage` is private, tagged, and monotonic. Every new source starts
`NeverPublished`. The same sidecar transaction that first publishes
`CandidateReady` and creates its candidate record changes it once to
`Published{candidateId,candidateGeneration,publishAttemptId,privatePublishReceipt}`.
It can never return to NeverPublished and survives CandidateDeleted/Deleted and
source deletion. A publish-candidate effect-start/file/receipt with
NeverPublished is an interrupted boundary requiring readback, never proof for
artifact-only GC. Published plus a missing/mismatched candidate row is
corruption/needs-help, not authority to recreate, remigrate, or purge.

`pairIntegrity` is a separate deny-unknown safety overlay, not an artifact action
lifecycle. It starts Intact. `Published` with a missing/mismatched counterpart or
lineage/identity mismatch atomically marks every surviving record Inconsistent;
if only one survives, that record remains sufficient authority. Inconsistent is
sticky in this slice, pins every surviving record/file/ref/receipt indefinitely,
and takes projection/action precedence over CandidateReady/Deleted/Rejected or
candidate lifecycle. It permits only local help, exit, and safe-view reload—no
recreate, remigrate, apply, delete, GC, retry, or inferred repair. Public views
expose only the closed safe code, never lineage or private receipts.

The single `CredentialArtifactStoreV1` is a separate device-local SQLite sidecar
(`credential-artifacts-v1.sqlite3`, schema v1), not a table in the replaceable
main DB. It owns artifact records, attempts, steps, owner epochs, and CAS in one
transactional store. Its stable config-directory cross-process lock is
`CredentialArtifactIntegrityLockV1` at `credential-artifacts-v1.lock`; lock
identity never depends on source/candidate relationships. Every artifact or
candidate mutation, integrity scan, GC, and DB-replacement recovery holds this
exclusive outer lock from authoritative preflight through every external effect,
readback, and terminal/overlay publication. Per-ID locks may optimize only inside
the global lock and never grant authority. The universal lock order is integrity
lock → main-DB maintenance drain → `DbCompatibilityLockV1` exclusive → short
sidecar transactions/main publish; no path reverses it. Main-DB replacement
never replaces the lock/store. It is excluded from business sync, SQL/diagnostic export,
transfer, and app backups; a store open/integrity/version failure is
pre-effect `artifact_store_unavailable` and leaves every source/main DB untouched
only when detected before effect admission. After an effect marker/publish it is
`candidate_apply_authority_unavailable`, preserves observed uncertainty, and
permits no further effect. There is no fallback in-memory authority or
reconstruction that enables an action.

The private record and content/manifest/ETag binding are device-local. List/read
expose no path, URL, ETag, digest, ref, receipt, or credential.
Migrate/delete reacquire and compare the private binding plus revision before
any source or main-DB effect; mismatch is `artifact_changed` and requires reload.
An explicit action creates one `attemptId`, increments `ownerEpoch`, and records
ordered steps. A lease may be stolen after expiry only while phase is
`PreEffect` and no step has `effect_started`. Immediately before each external
effect, one CAS persists `effect_started`, timestamp, and idempotency key/receipt
slot. From then on, no owner may issue that effect again.

Post-effect interruption is claimed only by a reconcile owner and performs
readback: #35 resolves `attemptId+stepId` to a secret-creation receipt; candidate
manifests carry attempt/generation; delete readback compares source existence and
the pre-effect content binding. Missing source after a recorded delete start is
observed success, while missing source before start is `source_missing`.
Unchanged source after a started delete is observed failure/no-effect; changed or
unreadable source is `manual_required`. Both observed no-effect and ambiguous/
unavailable truth persist as `credential_artifact_needs_help` with reason
`observed_no_effect|ambiguous|readback_unavailable`. It offers only local help/
manual resolution or
`recheck_credential_artifact(artifactId, expectedRevision)`, which is fenced
readback-only and cannot create/publish/delete/cleanup or reset the attempt for
retry. Candidate/secret ambiguity is never replayed; cleanup is a separately
recorded idempotent temp-artifact step or remains manual. Only readback advances
to terminal.

`migrate_credential_artifact` may publish one independently named sanitized
candidate and a new candidate record; it never imports/restores the main DB,
overwrites the original artifact, or schedules `delete_source`. Applying the
candidate is a separate explicit sanitized-transfer action and reruns all
compatibility/binding/Codex-impact gates. `delete_source` exists only in a
source-specific confirmed `delete_credential_artifact` attempt. Temp cleanup
cannot target the original source.

The private candidate-to-credential handoff is also owned by the artifact store:

```text
CandidateCredentialBindingV1 {
  candidateId, candidateRevision, candidateGeneration,
  sourceArtifactId, sourceRevisionAtCreation,
  pairIntegrity: Intact |
                 Inconsistent {
                   code: source_record_missing|source_identity_mismatch|
                         lineage_mismatch,
                   detectedAt
                 },
  candidateContentBinding,
  bindings: sorted [
    { universalId, bindingKeyDigest,
      opaqueSecretRef, expectedVersion,
      creationAttemptId, privateCreationReceipt }
  ],
  cleanupSteps: sorted [
    { bindingKeyDigest, discardAttemptId,
      status: pending|effect_started|observed_succeeded|manual_required,
      privateDiscardReceipt? }
  ],
  lifecycle: Pinned |
             Applying{action: apply|delete_candidate,
                      attemptId,ownerEpoch,
                      priorMainDbGeneration?} |
             Applied{mainDbGeneration,appliedAt} |
             NeedsHelp{action: apply|delete_candidate,
                       attemptId,reason,lastReadbackAt,
                       priorMainDbGeneration?} |
             Deleted{wasApplied,mainDbGeneration?,deletedAt},
  createdAt, terminalAt?
}

CandidateActionAttemptV1 {
  attemptId, attemptRevision, candidateId,
  action: apply|delete_candidate,
  requestRevision, expectedCandidateDigest,
  state:
    PreEffect{ownerId,ownerEpoch,ownerLeaseExpiresAt} |
    EffectStarted{ownerEpoch,effectStartedAt} |
    NeedsHelp{reason,lastReadbackAt,dbCompletionAck?} |
    Terminal{
      resultRevision,
      outcome: Applied{mainDbGeneration,appliedAt} |
               Deleted{wasApplied,mainDbGeneration?,deletedAt},
      terminalSafeSnapshot,
      dbCompletionAck?
    },
  createdAt, updatedAt, terminalAt?
}

DbCompletionAckV1 {
  sidecarAttemptRevision,
  markerRevision, replacementId, attemptId,
  outcome: applied|observed_no_effect,
  observedDbGeneration
}

apply_sanitized_candidate(candidateId, expectedRevision)
  -> CredentialCandidateActionOutcomeV1
delete_sanitized_candidate(candidateId, expectedRevision)
  -> CredentialCandidateActionOutcomeV1
recheck_sanitized_candidate(candidateId, expectedRevision)
  -> CredentialCandidateActionOutcomeV1
list_credential_candidates() -> CredentialCandidateSafeViewV1[]
get_credential_candidate(candidateId) -> CredentialCandidateSafeViewV1
credential-artifact://authority-updated
  -> { authorityKind: source|candidate, authorityId, revision }

CredentialCandidateSafeViewV1 {
  schemaVersion: 1, candidateId, revision, generation,
  lifecycle, safeDisplayCode, allowedActions[], createdAt, updatedAt,
  activeAction?: apply|delete_candidate, safeReasonCode?, pairIntegrityCode?
}

CredentialCandidateActionOutcomeV1 =
  Accepted { candidate, attemptId, action }
  | Rejected { code, candidate?, action }
  | Applying { candidate, action }
  | NeedsHelp { candidate, action, reason }
  | Applied { candidate, mainDbGeneration }
  | Deleted { candidateId, revision, wasApplied, deletedAt }
  | StoreUnavailable { code: artifact_store_unavailable, action }

CandidateActionRejectedCodeV1 =
  candidate_not_found | candidate_revision_changed |
  candidate_action_conflict | candidate_action_in_progress |
  candidate_action_superseded |
  source_action_in_progress | candidate_binding_changed |
  pair_integrity_inconsistent |
  secret_ref_changed | secret_ref_missing | baseline_changed |
  database_maintenance_pending | database_compatibility_unknown |
  universal_codex_transfer_unavailable | permission_denied

CandidateActionNeedsHelpReasonV1 =
  observed_no_effect | ambiguous | readback_unavailable
```

Candidate lifecycle and action attempts are schema-first/deny-unknown. Pinned may CAS to Applying for
apply or delete; Applied may CAS only to Applying(delete_candidate). An apply
variant forbids `priorMainDbGeneration`; delete before apply omits it; delete
after apply must retain the exact applied generation through Applying/NeedsHelp
to Deleted. Pre-effect rejection returns the exact prior Pinned/Applied state;
effect-start is action-specific: apply resolves only Applied or NeedsHelp;
delete_candidate resolves only Deleted or NeedsHelp. Recheck is also closed by
action/reason/ack:

| Persisted attempt | Readback-only legal result |
| --- | --- |
| apply + ReplacementPending + no ack | remain NeedsHelp, or exact-prior -> NeedsHelp(observed_no_effect)+immutable no-effect ack, or exact-target -> Applied+immutable applied ack |
| apply + NeedsHelp(observed_no_effect) + cleared Ready receipt | self-loop only; do not compare current main rows to the old target and never become Applied/Deleted |
| apply + NeedsHelp(ambiguous|readback_unavailable) + ReplacementPending | remain NeedsHelp or resolve through the exact pending branch above |
| delete_candidate + NeedsHelp | remain NeedsHelp or resolve Deleted from its recorded cleanup/candidate readback; never Applied |

`DbCompletionAckV1` is write-once and byte-immutable. A later unrelated main DB
mutation that resembles the old candidate cannot be attributed to the spent
attempt. This slice offers no re-apply from acknowledged no-effect NeedsHelp;
later application requires a new explicitly authorized candidate action/marker
contract, never rewriting the old attempt.
Every Applying/NeedsHelp lifecycle references one immutable attempt row whose
`action`, original `requestRevision`, `attemptId`, and
`expectedCandidateDigest` never change across owner epochs or rechecks. Applied
duplicate apply returns the same snapshot. Deleted is terminal and
retains `wasApplied`/generation when needed for idempotency. Every transition
increments revision and safe allowed actions are backend-derived; illegal
variants fail closed. `Applying` and
`NeedsHelp` always carry the persisted action. Apply copy warns that main DB may
change or may already have changed; delete-candidate copy says only the candidate
and proven-unreferenced pins may change and the original source/main DB are not
deleted or rolled back.

Candidate list/get are backend-authoritative sidecar safety reads and enumerate
every retained candidate, including a standalone candidate whose source record
is missing. Before returning, `CredentialArtifactIntegrityScannerV1` enumerates
IDs only after acquiring the global integrity lock, rereads every observed source/candidate
identity (including both sides of a mismatch), and CAS-persists only a newly
detected sticky Inconsistent overlay/revision in one
sidecar transaction. It changes no source/candidate bytes, refs, action attempt,
main DB, or lifecycle. Startup runs the scanner before recovery views; each
list/get/action repeats the exact pair preflight so external/cross-process drift
fails closed. Persistence failure returns store-unavailable with zero actions.

After an overlay commit, the safe event is emitted; otherwise the query is a
pure read. The renderer query cache stores only safe views; the event carries
only safe authority ID/revision and merely invalidates/refetches. Snapshot wins
over events. Pair Inconsistent always yields zero lifecycle actions regardless
of stale cached state. List/get/event/cache/log/DOM/diagnostics retain the same
ref/version/receipt/binding/lineage/value sentinels as other public surfaces.

Attempt revision is positive, monotonic, and increments exactly by one.
`DbCompletionAckV1` is legal only for action=apply. Terminal Applied requires an
`applied` acknowledgement; NeedsHelp(apply,observed_no_effect) after Ready
requires `observed_no_effect`; delete and ambiguous/readback-unavailable variants
forbid it and remain ReplacementPending when DB compatibility is unresolved.
Once present, the acknowledgement is byte-immutable across every later attempt
revision; no recheck can change its outcome, marker, replacement, attempt, or
observed generation.
The sidecar transition writes the acknowledgement and resulting
`sidecarAttemptRevision` atomically. Receipt clearing acquires the same
integrity→DB lock order and CASes both the exact Ready marker revision/
replacement/attempt/outcome/generation and the exact sidecar attempt revision;
any mismatch or store failure leaves the receipt and ordinary admission closed.

The public candidate file/manifest contains only candidate ID/generation and
safe transfer data; all refs/versions/receipts stay in the private sidecar. The
#35 creation receipt pins each new ref to the candidate. Source and candidate
actions share the global `CredentialArtifactIntegrityLockV1`; under it they
re-read and CAS every observed sidecar record, so source-delete, candidate
apply/delete, GC, and mismatched-lineage scanning cannot cross. Candidate apply
uses the fixed lock order: integrity lock → main-DB maintenance drain → exclusive
`DbCompatibilityLockV1` → short artifact-store CAS → staged/main DB. No path
acquires these in reverse order.

Under exclusive DB lock, apply revalidates candidate ID/revision/generation/
content binding, every Universal binding digest, SecretRef version/creation
receipt, compatibility marker, local baseline, and Universal Codex impact. It
then resolves minimum-lifetime leases, atomically persists the immutable action
attempt as `EffectStarted` plus candidate Applying, and builds/checkpoints/closes
one staged DB with reference-native refs. It writes the exact
`ReplacementPending(CandidateApply)` receipt, atomically publishes the staged
main once, and publishes Ready with the matching completion receipt; leases are
zeroized. Matching sidecar Applied or exact-prior NeedsHelp writes
`DbCompletionAckV1` before the receipt can clear. Crash recovery uses only the pending/ready receipt,
attempt row, file identities/digests, and exact main projection readback; it never
rebuilds or reapplies. Same ID/revision after success returns `Applied`;
revision/binding drift or duplicate different identity rejects with zero main
write. Sidecar unavailable before ReplacementPending blocks apply with zero
effect; after pending/publish it maps to authority-unavailable and causes no
further effect or replay.

Original-source deletion does not affect the pinned or applied candidate/ref
handoff. A source-specific confirmed delete acquires the same global integrity
lock and atomically rechecks both records. Source `CandidateReady` may
enter `PreEffect(delete) -> Reconciling -> Deleted` only while the candidate is
`Pinned` or `Applied`; source `CandidateDeleted` may take the same path. Candidate
`Applying` or `NeedsHelp` rejects source delete as
`candidate_action_in_progress` with zero effect. A successful source delete
changes only the source record/source bytes: candidate lifecycle, candidate/ref
pins, and applied main DB state remain byte-for-byte unchanged and reloadable.
Conversely, candidate apply/delete may start only when the source is
`CandidateReady` or already `Deleted`; source `PreEffect`, `Reconciling`, or
`NeedsHelp` rejects it as `source_action_in_progress`. If source deletion already
completed, candidate operations never require source bytes/existence.
Unapplied candidate content, binding record, and pinned refs have no timed purge.
Explicit candidate delete removes only the sanitized candidate and uses a
recorded #35 idempotent discard receipt for refs proven unreferenced; ambiguous
cleanup becomes `NeedsHelp`. Each discard step writes its attempt ID and
`effect_started` before the #35 port call, then records opaque readback receipt;
reconciliation queries that attempt and never reissues it. Candidate terminal
state, terminal action-attempt receipt, and source `CandidateDeleted` are published in one
sidecar transaction after effect readback. If the source is already `Deleted`,
it remains Deleted; no source record is recreated. After applied, the main DB
owns refs. Applied candidate content may be explicitly deleted without deleting
main refs.

The artifact store has a unique `(candidateId, requestRevision)` action-attempt
ledger, not one overwritable `last` slot. Command handling consults it before
current-revision comparison. A repeated request for the same candidate, action,
and original `requestRevision` returns its stored active state or exact terminal
safe snapshot and never reruns main publish or cleanup while that terminal
`resultRevision` is still current. If a later valid action advanced the candidate,
the old request returns `candidate_action_superseded` plus the current safe view;
it never renders historical allowed actions or mutates. A retry with another
action at that request revision is
`candidate_action_conflict`; another revision is
`candidate_revision_changed`. In particular, response loss after successful
`delete_candidate` is safe: repeating the original delete returns the stored
`Deleted` result without calling #35 discard or candidate cleanup again.

On successful candidate deletion, the source record CASes from
`CandidateReady` to `CandidateDeleted`. That source remains isolated and may
only be kept or removed through a separate, source-specific confirmed delete;
the same source record can never be migrated again. `wasApplied=false` says the
main DB did not change; `wasApplied=true` says the already-applied main DB state
remains. Both say the original source was not deleted. Fixtures cover crash
before/after effect marker/main publish/ready marker, duplicate apply, binding
drift, source already deleted, sidecar unavailable, candidate delete response
loss/cleanup, and retention/pinning.

A source artifact has no timed source deletion; only an explicit source-specific
confirmed delete removes its bytes. Metadata uses one cross-record GC rule, not
independent 30-day timers. The source tombstone remains pinned while its candidate
is Pinned/Applying/Applied/NeedsHelp or while any candidate file/ref pin/action
receipt remains. Candidate binding, tombstone, and attempt ledger remain pinned
while source bytes exist, the source record references the candidate, candidate
content exists, a ref pin is candidate-owned, or any action/recovery is legal.
Thus both `source=Deleted + candidate=Pinned|Applied` and
`source=CandidateReady + candidate=Applied` retain all authority beyond 30 days.

`gc_credential_artifact_pair(sourceArtifactId, candidateId)` acquires the global
integrity lock and uses one sidecar transaction. It may purge both
records/attempt receipts together only when source is Deleted, candidate is
Deleted, source and candidate files are absent, candidate-owned refs are released
or proven main-owned, no pending/needs-help attempt or DB completion receipt
exists, and 30 days have elapsed since the later of both terminal times and the
last action receipt. It cannot purge only one record. Any missing counterpart or
illegal link is corruption/needs-help, never authorization to reconstruct or
continue. Artifact-only GC requires the persisted exact
`candidateLineage=NeverPublished`, source Deleted/absent, no candidate file/row/
ref/action/effect-start/receipt, the same global integrity lock, and 30 days from source
terminalAt. `Published` is permanent; a missing counterpart is always
corruption/needs-help and only intact-pair GC can eventually purge it. `rejectedCode` is closed to
`artifact_changed|source_missing|unsupported_generation|invalid_schema|
integrity_mismatch|inspection_failed|permission_denied|
secret_backend_unavailable|migration_failed|artifact_store_unavailable|
readback_unavailable|pair_integrity_inconsistent`; unknown codes fail closed. Double-owner, delete-after-
effect crash, SecretRef-after-effect crash, candidate-publish boundary,
replacement-survival, and corrupted-store fixtures must prove no effect replay.

The backend-to-UI projection is total over every closed public lifecycle and
action outcome. Internal-only variants never cross IPC:

| Backend variant | Public projection and exact user truth |
| --- | --- |
| Artifact `Detected` | internal-only while inspection completes; no public partial record |
| Artifact `MigrationRequired` | `legacy_credential_artifact_blocked`; source isolated, main unchanged; keep, migrate, or confirmed source delete as backend-authorized |
| Artifact `PreEffect(migrate|delete)` | `credential_artifact_preparing`; names the action and says no effect has started; close/wait only |
| Artifact `Reconciling(migrate|delete)` | `credential_artifact_reconciling`; action-specific readback, no replay |
| Artifact `NeedsHelp(migrate|delete, reason)` | `credential_artifact_needs_help`; action/reason-specific copy, manual help or fenced readback-only recheck |
| Artifact `CandidateReady` | `sanitized_candidate_ready`; candidate is separate and original/main are unchanged |
| Artifact `CandidateDeleted` | `credential_artifact_candidate_deleted`; candidate is gone, original remains isolated, no remigration; keep or confirmed source delete only |
| Artifact `Deleted` | `credential_artifact_source_deleted`; confirmed original source removal did not delete or roll back candidate/main DB state |
| Artifact `Rejected(code)` | `credential_artifact_rejected`; safe code only, source isolated, no effect/automatic retry |
| Artifact outcome `Accepted` | project the embedded `PreEffect` safe view as action-specific preparing |
| Artifact outcome `Rejected` | project the embedded lifecycle plus `credential_artifact_action_rejected(code)`; no inferred transition |
| Artifact outcomes `Reconciling|NeedsHelp|CandidateReady|Deleted` | project their identically named persisted lifecycle rows above |
| Artifact/candidate pair `Inconsistent(code)` | `credential_artifact_pair_inconsistent`: “Local quarantine records are inconsistent. FyAgent retained the remaining record and deleted or reconstructed nothing.” Local help/exit/safe reload only; lifecycle controls suppressed |
| Artifact outcome `StoreUnavailable` before effect | `credential_artifact_store_unavailable`; source/main unchanged, no action |
| Candidate `Pinned` | `sanitized_candidate_ready`; apply/delete candidate/leave pinned as allowed |
| Candidate `Applying(apply)` | `sanitized_candidate_applying`; main DB may change or may already have changed; no duplicate submit |
| Candidate `Applying(delete_candidate)` | `sanitized_candidate_deleting`; candidate/pins only, original and main DB unchanged; no duplicate submit |
| Candidate `NeedsHelp(apply, observed_no_effect)` | `sanitized_candidate_apply_needs_help`; “Recovery verified that the main database is still the exact prior version. This candidate was not applied, and this attempt will not be replayed.” Manual help or fenced recheck only |
| Candidate `NeedsHelp(apply, ambiguous|readback_unavailable)` | `sanitized_candidate_apply_needs_help`; main DB truth is uncertain/unavailable and normal DB stays closed when compatibility is unresolved; manual help or fenced recheck only |
| Candidate `NeedsHelp(delete_candidate, reason)` | `sanitized_candidate_delete_needs_help`; candidate cleanup truth requires readback; original/main unchanged; manual help or recheck only |
| Candidate `Applied` | `sanitized_candidate_applied`; main DB matches candidate, original source remains; allow explicit delete sanitized candidate and separately confirmed source delete |
| Candidate `Deleted(wasApplied=false)` | `sanitized_candidate_deleted`; candidate/pins removed, main DB and original source unchanged |
| Candidate `Deleted(wasApplied=true)` | `sanitized_candidate_deleted_after_apply`; candidate removed, applied main DB state remains, original source remains |
| Candidate outcome `Accepted|Applying|NeedsHelp|Applied|Deleted` | project the embedded persisted lifecycle/action row above |
| Candidate outcome `Rejected(candidate_action_superseded)` | `sanitized_candidate_action_superseded`: “A newer candidate action superseded this earlier request. Nothing was repeated. The current candidate state is shown.” Dismiss/review plus current-view actions only; never historical controls |
| Candidate outcome `Rejected(other code)` | project current lifecycle plus `sanitized_candidate_action_rejected(code)`; no inferred state change |
| Candidate outcome `StoreUnavailable` before effect | `credential_artifact_store_unavailable` with candidate context; no candidate/main/source effect |
| DB replacement recovery `AuthorityUnavailable` after pending/publish | `candidate_apply_authority_unavailable`; main may be prior or target, remains closed, no repeat; local help/repair or exit only |

`CredentialArtifactSafeViewV1` and `CredentialCandidateSafeViewV1` expose only
the safe discriminants required by this table. The `Deleted` projections retain
opaque ID, candidate revision, `wasApplied`, and safe timestamps for idempotent
readback, but never expose a SecretRef, SecretRef version, creation/discard
receipt, content binding, locator, ETag, digest, or credential value. Unknown
enum values, missing action discriminants, and illegal field combinations fail
closed before rendering.

Cutover is one source commit across renderer, native entry points, and the writer
authority boundary. A central `ProtectedCodexMutationGate` owns the per-operation
cutover flag and stable `change_plan_required` error. Its pure precedence is:
classify target/mode/risk first; return specific typed unsupported for proxy
takeover, official-target switch, or critical risk; otherwise return/route
`change_plan_required` only when a supported normal-mode request enters a legacy
write path. Both branches precede all effects. After cutover:

| Exact-source entry | Required Codex behavior before any managed write |
| --- | --- |
| Tauri `add_provider` and `add_provider_with_result` | both classify unsupported first; supported normal mode returns `change_plan_required`; zero Provider/hook effects |
| Tauri `update_provider` and `update_provider_with_result` | both classify unsupported first; supported normal mode returns `change_plan_required`; zero Provider/hook effects |
| Tauri `switch_provider` and `switch_provider_with_result` | both classify unsupported first; supported normal mode returns `change_plan_required`; zero Provider/hook effects |
| native tray Provider click | classify unsupported first; otherwise, before proxy-flag/menu/provider writes, emit safe Plan-UI request `{app:codex,operation:switch,targetProviderId}` and focus/open the exact switch flow; navigation failure returns typed required with zero writes |
| `ProfileService::apply` / `apply_profile` with a Codex Provider delta | return structured `profile_change_plan_required` before autosave, proxy disable, Provider switch, MCP toggle, current-profile write, or events; UI says the whole apply was unsaved and offers edit/remove-delta; #41 later supplies its UCP adapter |
| Codex provider deep link | route draft ID plus allowlisted safe fields into draft-to-Plan UI before `add_draft`, endpoint insert, or switch; exclude secrets; navigation failure has zero native persistence |
| existing UCP v1 executor | replaced by the v2 prepared/effect-permit seam; it cannot call public legacy switch |
| public Codex endpoint add/remove used by create/edit | return `change_plan_required`; form endpoints are draft-only until private commit |
| Universal Provider save/delete/sync | replace two IPCs with one revision/epoch-bound backend mutation; actual `universal-codex-{id}` child participates in impact/CAS; blocked Codex impact is whole-operation zero-write |

The public legacy ProviderService add/update/switch/add-draft/endpoint writer
methods themselves also consult the gate, so a missed native caller fails closed
even when no Plan/job is active. For a cut-over protected Codex operation they
cannot write under any ordinary call. Only module-private
`commit_prepared_change`, which requires the unforgeable `EffectPermit`, can
perform that protected mutation. Test-only permit construction is compiled only
under test hooks. A temporary wrapper may create a Plan only; it cannot call a
legacy writer.

Every Codex create/edit/switch subcase is protected. Proxy takeover,
official-target switch, and critical risk return their typed unsupported result
and never enter a legacy path. Prior routing is limited to non-Codex apps and
separately named Codex operation families that do not implement
create/edit/switch: delete, import-default, live-remove, official-seed, proxy
failover control, and sort/last-used metadata. Every
resource writer still joins the coordinator/epoch. Sort/last-used is classified
non-semantic; it cannot alter prepared endpoint content. Registration/callsite
static scans plus entry-specific spies cover all six commands, tray, profile,
deep link, old UCP, and endpoint writers. Positive tests assert exact tray target
navigation and safe deep-link draft preservation; failure tests assert zero
proxy/menu/provider/endpoint/writer/effect counters.

Universal mutation moves from renderer `upsert -> sync` to one backend command:

```text
UniversalMutationRequestV1 =
  Create {
    universalId, expectedAbsent=true, expectedProviderStateEpoch,
    proposed: UniversalProviderMutationDraftV1, syncAfterSave
  }
| Edit {
    universalId, expectedRevisionToken,
    proposed: UniversalProviderMutationDraftV1, syncAfterSave
  }
| Duplicate {
    sourceUniversalId, expectedSourceRevisionToken,
    newUniversalId, expectedNewAbsent=true, expectedProviderStateEpoch,
    proposed: UniversalProviderMutationDraftV1, syncAfterSave
  }
| Delete { universalId, expectedRevisionToken }
| Sync { universalId, expectedRevisionToken }

mutate_universal_provider(request: UniversalMutationRequestV1)
  -> UniversalMutationOutcome
```

The serde enum is internally tagged and `deny_unknown_fields`; variants make
every required/forbidden combination structural. `UniversalProviderMutationDraftV1`
contains exact non-secret fields plus #35-owned
`UniversalCredentialIntentV1`; it never
contains API key/plaintext. Invalid combinations reject before state reads or
writes.

Renderer revision authority comes only from safe backend reads:

```text
UniversalProviderMutationViewV1 {
  universalId,
  safeDraft,
  revisionToken,                 // opaque backend-authored token
  observedProviderStateEpoch,
  actualCodexChildStatus: absent | present_credential_free |
                          present_secret_ref | present_legacy_plaintext
}

get_universal_providers() -> Map<id, UniversalProviderMutationViewV1>
get_universal_provider(id) -> UniversalProviderMutationViewV1?
```

The token domain-binds the current redacted Universal fingerprint, Provider
epoch, actual child presence/epoch/redacted digest, and expected materialization.
TypeScript treats it as opaque and never recomputes it. The existing command
names change return type to the safe view in the same cutover commit; no legacy
plaintext read IPC remains registered. Raw DAO/Service readers return a
non-serializable `StoredUniversalProvider`, are module-private, and are reachable
only inside the coordinator-owned mutation adapter. A token/epoch/absence
mismatch returns typed `universal_revision_changed` with a fresh safe view and
zero writes. Registration/callsite and secret-sentinel scans cover IPC, TS
query/cache, event, DOM, log, and diagnostics.

The old `upsert_universal_provider`, `delete_universal_provider`, and
`sync_universal_provider` write commands and the identically purposed public
ProviderService writer methods return
`universal_mutation_v2_required` after cutover, including for non-Codex rows, so
UI cannot recreate a two-IPC TOCTOU. The single command acquires the mutation
coordinator and reads a backend-owned snapshot before its first write:

```text
UniversalCodexImpactSnapshotV1 {
  universalId,
  universalPresence,
  universalFingerprint,                 // redacted canonical row
  observedAtProviderStateEpoch,
  priorCodexMembership,
  proposedCodexMembership,
  actualChildId: "universal-codex-{id}",
  actualChildPresence,
  actualChildObservedAtEpoch,
  actualChildRedactedDefinitionDigest?, // non-secret fields + credential-state code
  expectedMaterialization: present | absent,
  action
}
```

Concrete Universal credentials require a #35 exact-SHA adapter and migration,
not only placeholder types:

```text
UniversalCredentialAdapter {
  inspect(bindingRequirement) -> presence/ref/version/migration status
  prepare_storage_projection(intent) -> reference-native stored projection
  resolve_for_mutation(requirements, minimumLifetime) -> UniversalSecretLeaseSet
}

UniversalCredentialIntentV1 =
  None
  | Clear
  | Preserve { opaqueBindingToken }
  | Replace { secretRef, expectedVersion }
```

`UniversalCredentialIntentV1` is a schema-v1, deny-unknown wire enum in the
safe Universal mutation draft. Its canonical domain is
`fyagent.universal-credential-intent.v1`; its exact variant tag/payload binds
the backend revision/prepared-payload digest. Only the #35 adapter may map it to
an internal prepared credential requirement after resolving the binding token;
it cannot deserialize or cast it as `ProviderCredentialIntentV1`. Cross-domain
tags/fields and unknown schema versions fail before state access. Shared fixtures
cover every variant and illegal mixed payload.

The exact handoff must define reference-native Universal and projected child
storage; neither Universal nor Claude/Gemini/Codex child rows persist plaintext
after migration. Under the same coordinator and after snapshot/CAS, outer
mutation inspects exact ref/version and resolves minimum-lifetime leases before
creating the Universal permit. Resolver/migration/expiry failure returns typed
`dependency_unavailable/no_effect` with zero Universal/child/event/cache/epoch/
other-app writes. On every exit, the owning outer seam zeroizes leases.

Before that handoff/migration, any legacy plaintext, Preserve/Replace intent, or
sync requiring a credential is production-disabled—even for non-Codex apps.
Only a proven credential-free non-Codex operation with no actual Codex child and
`UniversalCredentialIntentV1=None|Clear` may continue. Safe views expose only credential
status and an opaque binding token, never ref identity or value.

The #35 handoff also owns a closed, deny-unknown persisted discriminator:

```text
UniversalCredentialStorageV1 =
  None { schemaVersion: 1 }
  | SecretRef {
      schemaVersion: 1, opaqueRef, expectedVersion, bindingKeyDigest
    }
  | NeedsLocalRebind {
      schemaVersion: 1,
      requirementCode: credential_required,
      source: remote_import | sanitized_restore | legacy_staging,
      expectedBindingKeyDigest
    }
```

It is stored in new reference-native columns/rows, never encoded into the legacy
`api_key` string. `None` means proven credential-free;
`NeedsLocalRebind` means a credential is required but deliberately absent and
cannot be decoded as `None`. The safe view, Universal revision token, prepared
payload digest, Provider epoch/CAS, and import/restore fixtures all bind the
exact discriminator. Only #35 secure rebind may transition
`NeedsLocalRebind -> SecretRef`, with a fresh token/epoch.

Local-ref reuse is authorized only by this non-secret canonical key:

```text
UniversalCredentialBindingKeyV1 {
  schemaVersion: 1,
  universalId,
  credentialSlot: primary_api_key,
  providerType,                       // normalized closed provider type
  consumerDestinations: sorted [
    { app: claude|codex|gemini,
      authSchemeCode,
      normalizedEndpoint: { scheme, host, effectivePort, normalizedBasePath } }
  ]
}
bindingKeyDigest = SHA-256(
  "fyagent.universal-credential-binding.v1" || 0x00 || canonicalJsonBytes(key)
)
```

The closed constructor normalizes in this exact order:

1. parse the URL and reject userinfo, query, fragment, malformed percent escapes,
   and unknown fields;
2. lowercase scheme/closed provider/app/auth codes, convert host with IDNA2008 to
   lowercase A-label, and materialize the effective port;
3. NFC-normalize literal Unicode path code points and percent-encode their UTF-8
   bytes with uppercase hex;
4. uppercase all existing percent escapes, then percent-decode only RFC 3986
   unreserved bytes—including `%2E`; reserved `%2F`/`%5C` remain encoded;
5. apply RFC 3986 dot-segment removal to the decoded literal `.`/`..` segments;
   preserve repeated-slash empty segments and trailing slash, require a leading
   slash, and map empty path to `/`;
6. NFC-normalize Universal ID and build the closed struct.

It then uses the existing Change Plan v2 canonical UTF-8 JSON exactly:
UTF-8-key-sorted objects, declared array order, no insignificant whitespace,
and no implicit Unicode normalization by the encoder. `consumerDestinations` is
sorted uniquely by app before encoding. The
per-app destination comes from the same pure child projection used for impact,
so provider type, endpoint, auth scheme, app membership, or any other
credential-scope-affecting projection change changes the digest. Model/display
fields do not. Transfer recomputes and verifies the digest from safe fields;
local storage and CAS use the same value. Version/field/vector/digest mismatch
always becomes `NeedsLocalRebind`; ID or `required` alone never preserves a ref.
Rust fixed vectors and the shared TypeScript/transfer fixture freeze equal and
mismatch values; TypeScript treats the backend digest as opaque and never
authorizes reuse.

Normative path preprocessing vectors are:

| Input path | Normalized path |
| --- | --- |
| `/a/%2e/b/%2e%2e/c/` | `/a/c/` |
| `/a/%2f/b/%5c` | `/a/%2F/b/%5C` |
| `/a//b` | `/a//b` |
| `/cafe\u0301/` | `/caf%C3%A9/` |
| `/a/b/..` | `/a/` |
| empty | `/` |

Fixed vector A starts with decomposed Universal ID `cafe\u0301`, host
`bücher.example`, omitted HTTPS port, and path `/a/./b/%7euser/../`; preprocessing
produces the following exact UTF-8 bytes (the `é` is literal NFC):

```text
{"consumerDestinations":[{"app":"claude","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"xn--bcher-kva.example","normalizedBasePath":"/a/b/","scheme":"https"}},{"app":"codex","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"api.example.com","normalizedBasePath":"/v1","scheme":"https"}}],"credentialSlot":"primary_api_key","providerType":"custom","schemaVersion":1,"universalId":"café"}
sha256:9f537327bab07e3a8834832fe24d439222b1d91dbf170e000899c273d8452d51
```

Vector B changes only Codex effective port to `8443`; its exact canonical bytes
and digest are:

```text
{"consumerDestinations":[{"app":"claude","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":443,"host":"xn--bcher-kva.example","normalizedBasePath":"/a/b/","scheme":"https"}},{"app":"codex","authSchemeCode":"bearer","normalizedEndpoint":{"effectivePort":8443,"host":"api.example.com","normalizedBasePath":"/v1","scheme":"https"}}],"credentialSlot":"primary_api_key","providerType":"custom","schemaVersion":1,"universalId":"café"}
sha256:2142b5f03e1d35ffe5f7b8df800d9ab88e38469833267c8ff8312b79400162d3
```

Migration allocates a fresh database `user_version` greater than every binary
that understands only plaintext Universal rows. Existing source `ca552f4d`
uses default `Connection::open`; when version inspection succeeds it stops before
`create_tables`, but inspection error can fall through to normal initialization.
It is evidence only for the narrower successful-inspection pre-schema-write path,
not a read-only or error-fail-closed guard. #35 migration is
therefore disabled until a predecessor commit containing the revised
`dbUpgrade` UI, side-effect-free future-version preflight, and zero-activity
fixture is recorded
as immutable `MIGRATION_GUARD_BASELINE_SHA` and released as the minimum supported
pre-migration version. The exact SHA—not a branch name or version label—is an
enablement input and acceptance fixture. A ref/binding token can never be
interpreted as an API key.

The safe predecessor and every migration/replacement path share one cross-process
`DbCompatibilityLockV1`. A normal process acquires a shared lease before
inspection and holds it for its whole DB lifetime. Bootstrap, #35 migration,
SQL/sync/backup replacement, and marker update require the exclusive lease; #35
holds it from the pending-marker write through DB commit/checkpoint/close and
ready-marker publication. Thus inspection-to-SQLite-open and migration cannot
race. Non-participating external file replacement is detected when the stored
file identity no longer matches and is outside any safety claim.

The lock has a stable config-directory identity
`fyagent.db.compat.lock`; it never follows the replaceable DB inode. A running
replacement never upgrades a shared lock in place. It enters
`database_maintenance_pending`, stops admission of jobs/sync/backup/DAO work,
drains workers and readers, closes every SQLite handle/hook, then releases the
shared lease. It next acquires exclusive, re-runs the complete marker/header/
identity inspection, and aborts on any baseline change. After replacement it
checkpoints/closes, publishes ready, releases exclusive, reacquires shared,
reinspects, reopens DB, and only then resumes work. Drain/acquire/reinspect
failure reopens the unchanged baseline under shared or remains fail-closed; no
replacement effect occurs. This release/reacquire protocol plus full reinspection
closes the race without a cross-platform lock upgrade.

```text
DbCompatibilityMarkerV1 =
  BootstrapPending {
    schemaVersion: 1, markerRevision, bootstrapId,
    targetGeneration: 1,
    targetApplicationId: 0x46594147,       // ASCII FYAG
    targetUserVersion, minCompatibleUserVersion, checksum
  }
  | MigrationPending {
    schemaVersion: 1, markerRevision, migrationId,
    dbFileIdentity: posix{device,inode} | windows{volumeSerial,fileIndex},
    observedGeneration, targetGeneration,
    observedApplicationId: 0 | 0x46594147,
    targetApplicationId: 0x46594147,
    observedUserVersion, targetUserVersion,
    minCompatibleUserVersion, checksum
  }
  | ReplacementPending {
    schemaVersion: 1, markerRevision, replacementId,
    replacementKind: CandidateApply {
      sourceArtifactId, candidateId, candidateGeneration, attemptId,
      requestRevision, expectedCandidateDigest,
      expectedMainProjectionDigest
    },
    priorDbFileIdentity,
    priorDbGeneration, priorDbContentDigest,
    targetDbFileIdentity,
    targetDbGeneration, targetDbContentDigest,
    applicationId: 0x46594147,
    userVersion, minCompatibleUserVersion, startedAt, checksum
  }
  | Ready {
    schemaVersion: 1, markerRevision,
    dbFileIdentity: posix{device,inode} | windows{volumeSerial,fileIndex},
    dbGeneration,
    applicationId: 0x46594147,
    userVersion, minCompatibleUserVersion,
    completionReceipt:
      None |
      CandidateApply {
        replacementId, sourceArtifactId,
        candidateId, candidateGeneration, attemptId,
        requestRevision, expectedCandidateDigest,
        expectedMainProjectionDigest,
        outcome: applied|observed_no_effect,
        observedDbGeneration, completedAt
      },
    checksum
  }
```

The tagged, deny-unknown canonical JSON variants make nonapplicable fields
illegal: bootstrap has no observed DB fields or migration/replacement ID;
migration pending alone has schema-migration fields; replacement pending has one
closed replacement-kind receipt and exact prior/target identities/digests; ready
has neither target nor migration fields and can never have identity `none`.
`completionReceipt` is itself tagged `None|CandidateApply`; no optional field
combination is legal. User versions are nonnegative i32;
revisions/generations are positive i64, increment exactly by one, and never wrap;
pending requires target generation = observed + 1 and target user version >=
observed. Legacy `application_id=0` is accepted only by marker-absent legacy
fallback or as pending observed state; first bootstrap/migration writes
`FYAGENT_APPLICATION_ID=0x46594147`, and every ready marker/DB must match it.

The marker hashes every field except checksum
with `SHA-256("fyagent.db-compat-marker.v1" || 0x00 || bytes)`. It is written via
temp-file fsync + atomic rename + parent-directory fsync while the exclusive
lock is held. #35 publishes `migration_pending` before touching credential
storage, then checkpoints/closes SQLite and publishes `ready` after commit.
Candidate apply writes `ReplacementPending` only after its staged DB is
checkpointed, closed, content-digested, and its target file identity is fixed;
the marker is fsynced before atomic main replacement. It then publishes `Ready`
with the matching `CandidateApply` completion receipt. No later replacement may
start while a Ready completion receipt lacks its sidecar acknowledgement.

```text
DbReplacementRecoveryOutcomeV1 =
  ExactPriorResolved { candidateState: needs_help_observed_no_effect }
  | ExactTargetResolved { candidateState: applied }
  | Ambiguous { reason: ambiguous|readback_unavailable }
  | AuthorityUnavailable {
      reason: sidecar_unavailable|ack_persist_failed|ack_mismatch
    }
```

`AuthorityUnavailable` projects to
`candidate_apply_authority_unavailable`: “FyAgent cannot verify the local
candidate-apply authority. The main database may still be the prior version or
may already contain the candidate; it remains closed and this apply will not be
repeated.” Actions are local repair/help or exit only; apply/delete/retry/replay
and ordinary DB admission are absent. This post-effect projection is distinct
from pre-effect `StoreUnavailable`, whose no-effect copy remains valid.

`DbReplacementRecoveryV1` is the only code allowed to interpret
`ReplacementPending`. Before any business service, DAO, sync, job admission, or
normal SQLite open, it acquires global `CredentialArtifactIntegrityLockV1`, then
the exclusive compatibility lock, and only then freshly enumerates and
reads/revalidates the exact marker plus every observed source/candidate identity.
It performs no source/candidate ID peek and accepts no pre-lock enumeration.
Disputed or missing
relationships never select a lock. No reverse acquisition is allowed. It never creates a new
attempt, rebuilds a staged DB, invokes #35, or repeats replacement. It classifies:

| Exact observation | Recovery transition |
| --- | --- |
| Main identity/generation/content digest match every prior field and the target identity is not published | `observed_no_effect`; publish Ready for the unchanged prior DB with a CandidateApply completion receipt, then sidecar NeedsHelp for the immutable attempt |
| Main identity/generation/content digest match every target field and a hook-free/query-only read verifies `expectedMainProjectionDigest` plus exact candidate/attempt rows | `applied`; publish matching Ready completion receipt, then sidecar Applied |
| Ready already contains the matching completion receipt but the matching sidecar acknowledgement is absent | verify its recorded outcome and write matching Applied or NeedsHelp(apply,observed_no_effect) plus `DbCompletionAckV1` |
| Sidecar cannot be read, acknowledgement cannot persist, or marker/ack identity mismatches after pending/Ready | `AuthorityUnavailable`; preserve marker/remaining authority and keep normal DB closed; never claim no effect |
| Identity/digest/projection is mixed, missing, or unreadable | `ambiguous|readback_unavailable`; retain ReplacementPending, persist sidecar NeedsHelp, expose only the safe compatibility/candidate-help surface, and keep normal DB closed |

The narrow target verification uses a dedicated read-only/query-only connection
with no `Database::init`, migrations, hooks, caches, jobs, sync, backup, or
network. For an exact prior match it does not open SQLite. Publishing Ready and
the sidecar resolved state cannot be one transaction, so ordering is deliberate:
Ready first retains the full completion identity; matching Applied or
NeedsHelp(apply,observed_no_effect) records `DbCompletionAckV1`; a later exact
marker+sidecar CAS may replace `completionReceipt` with `None`. Crash at any
boundary repeats only this readback/finalization. A newer replacement is blocked until acknowledgement
and receipt clearing complete. Missing/corrupt sidecar preserves the pending or
Ready receipt, keeps normal DB/services closed, and reports
`candidate_apply_authority_unavailable`;
it never guesses a terminal state.

| Disk state under lock | Admission |
| --- | --- |
| DB, marker, `-wal`, `-shm`, and `-journal` all absent | `fresh_bootstrap_allowed`; after shared release/exclusive acquire and all-absent reinspection, owner writes `bootstrap_pending`, initializes once, publishes `ready`, releases exclusive, reacquires shared, reinspects, then opens normally |
| Valid `ready` marker with matching file identity/application ID/generation and supported min version | SQLite open allowed while shared lease remains held |
| Valid `ready` with unacknowledged CandidateApply completion receipt | run narrow receipt finalization before ordinary service/DB admission; do not overwrite the receipt |
| Valid `replacement_pending(CandidateApply)` supported by this binary | run only `DbReplacementRecoveryV1` under exclusive lock; ordinary SQLite/service admission remains closed |
| Valid pending/ready marker requiring a newer reader | `database_upgrade_required`; no SQLite open |
| Compatible `bootstrap_pending` or `migration_pending` with no live exclusive owner | `database_compatibility_unknown(interrupted_bootstrap|interrupted_migration)`; no automatic resume/open/init; this generic rule never consumes ReplacementPending |
| Marker absent, existing DB, no sidecars/hot journal | exact 100-byte main-header parser may admit only a valid supported legacy DB |
| Marker absent plus WAL/SHM/nonempty journal, invalid SQLite magic/page size/read-write versions/application ID, `changeCounter != versionValidFor`, truncated/permission error, marker checksum/identity/generation mismatch | `database_compatibility_unknown`; no SQLite open or `Database::init` |

Fallback uses ordinary read-only file IO, never SQLite, and parses big-endian
header fields including user version, application ID, schema cookie, change
counter, and version-valid-for. It never creates/touches DB/WAL/SHM/journal.
Fresh bootstrap is the only missing-marker/missing-DB success case. Lock
acquisition failure is `database_compatibility_unknown(lock_busy)` after a
bounded wait; it never resumes migration. Unknown reasons are the closed safe
set `interrupted_bootstrap|interrupted_migration|lock_busy|marker_invalid|
header_invalid|sidecar_present|permission_denied|identity_mismatch|
replacement_ambiguous|replacement_readback_unavailable`. Copy says a
prior startup/migration may be incomplete and only metadata was inspected;
actions are local help, compatible-build guidance, or exit. The exact guard fixtures include fresh
install, pending/ready, WAL-only newer user-version, hot rollback journal,
change-counter mismatch, and a concurrent migrator paused at every boundary.

Forward migration is #35-owned and fail closed: establish SecretBackend
capability, convert and verify every required legacy value, write the closed
storage discriminator and reference-native child projections, clear legacy
plaintext, and commit the new `user_version`/migration marker before enabling
new reads or writes. Partial external-secret work is cleaned or quarantined by
#35; failed migration leaves legacy rows disabled and cannot expose a mixed
writable state. The generic pre-migration database backup path may not create an
ordinary plaintext-bearing backup for this migration: #35 must supply a
device-local sealed migration artifact or a no-backup fail-closed procedure,
and the migration cannot proceed if that protection fails.

Downgrade after the marker is forbidden. A migrated database may be opened only
by a binary whose supported schema includes `UniversalCredentialStorageV1`;
normal rollback never restores plaintext or removes the version/adapter guard.
Business sync/export/diagnostics/sanitized backup emit only safe credential
requirements, never opaque ref/binding token/value. Remote import may commit
safe non-secret fields plus `NeedsLocalRebind`; it cannot lower the marker or
overwrite a local reference. That import is an import effect, not a Universal
mutation. A later blocked Universal mutation returns
`credential_rebind_required/no_effect`; `no_effect` refers to that attempted
mutation, while the already-committed safe import remains visible.

The old-binary projection is the stable `database_upgrade_required` state on
the safe predecessor's `dbUpgrade` surface. Copy says the data requires a newer
compatible FyAgent and this build inspected only compatibility metadata and did
not initialize, migrate, or modify business data; it read only the compatibility
marker or SQLite main-file header needed for the guard. Permitted actions
are local upgrade instructions, a verified already-local compatible installer
when available, or exit. Config-folder mutation, continue, downgrade, ordinary
rollback, and backup restore are absent. The surface owns initial heading focus,
semantic alert/description, labelled keyboard actions, and complete
`zh|zh-TW|en|ja` copy; it performs no DDL, business-data query, DAO/service
initialization, sync, writer, or network work. Binaries older than
`MIGRATION_GUARD_BASELINE_SHA` receive only the narrower existing guarantee of
pre-schema-write stop when their version inspection succeeds; they may continue
initialization on inspection error and retain legacy recovery copy/actions. They
are not accepted as the migration predecessor and are outside the safe-UX claim.

### 8.1 Credential copy and database-replacement matrix

Universal plaintext currently lives inside the `settings` blob, so skipping a
table is insufficient. Every copy path parses the exact Universal settings key
into `UniversalCredentialTransferV1`: safe non-secret fields plus
`credentialRequirement=none|required` and recomputable `bindingKeyDigest` when
required; malformed/unknown legacy data fails
closed. The transfer contains no plaintext, `opaqueRef`, binding token, lease,
or reversible credential fingerprint.

| Surface | Outbound/create rule | Inbound/restore rule | Compatibility/failure rule |
| --- | --- | --- | --- |
| SQL export/import | Export only `UniversalCredentialTransferV1`; never raw Universal settings | Import into a temporary DB, run #35 legacy migration/validation there, merge stable IDs with device-local bindings, then atomically replace; `required` without matching local binding becomes `NeedsLocalRebind` | Main DB and marker remain unchanged on failure; no raw safety backup before staging |
| WebDAV/S3 upload/download | #35 allocates `DB_COMPAT_VERSION > 6` and a new `db-vN` layout; upload only the safe transfer, never dual-write old layout | Download to staging; preserve matching local refs row-by-row; otherwise persist `NeedsLocalRebind`; candidate marker is `max(local, staged, required)` | New client never automatically consumes/writes `db-v6`; old client cannot see new layout. Legacy remote requires explicit #35 staging migration or typed rejection |
| App-managed backup create/list | Create a sanitized staged copy with safe transfer and compatibility metadata; list labels credential sanitation/generation | Restore only after staging validation/migration and local-ref merge; raw file never replaces main DB first | Backup failure is zero-write; no plaintext-bearing safety backup |
| Existing app-managed backup | Pre-enablement inventory classifies pre-safe/unknown backups as `legacy_credential_backup_blocked` and quarantines them from ordinary restore/sync/export | Only #35 secure staging migration may make a new sanitized backup; otherwise user may delete it | No one-click/raw restore; main DB and compatibility marker cannot be lowered |
| Diagnostics/ordinary logs | Emit status/reason codes only | Not importable | Sentinel rejects value/ref/token/path leakage |

Every staged transfer also enters the existing Universal Codex-impact boundary:

```text
UniversalTransferCodexImpactSnapshotV1 {
  universalId,
  local/staged Universal presence + redacted fingerprint,
  local/staged Codex membership,
  local actual child presence + Provider epoch + redacted definition digest,
  staged actual `universal-codex-{id}` child presence + redacted digest,
  staged-safe-fields projected Codex child digest,
  action: create | update | delete | unchanged
}
```

The stage scans both Universal membership and every Provider row whose ID or
versioned provenance identifies a Universal Codex child. Child presence counts
even when `apps.codex=false`. Before a Universal-to-UCP adapter exists, any
membership change, staged child, local child create/update/delete difference, or
staged safe-field change that alters the projected child digest returns
`universal_codex_transfer_unavailable`, records/quarantines the artifact, and
leaves main DB, local child, Provider epoch, marker, sync, cache, and events
unchanged. Staging files may be cleaned but are not published.

An allowed transfer contains no Codex impact. It structurally excludes every
staged Universal Codex child row and reinjects/preserves the exact local actual
child and local Codex membership; it cannot materialize, update, delete, or
resync Codex. This applies identically to SQL, WebDAV, S3, current backup, and
legacy-backup staging. Fixtures cover `apps.codex=false + staged child`, local
orphan child, staged/local membership changes, and child create/update/delete/
projected-digest changes, plus a no-impact control.

There is no automatic old/new remote dual-read, dual-write, raw main-DB swap, or
marker downgrade. A transfer row declared `none` remains `None` only when its
schema proves credential-free; any stripped/unknown credential requirement is
`NeedsLocalRebind`. Existing local `SecretRef` survives only when stable ID and
credential-requirement digest match; otherwise no guessed association occurs.
All transfer outcomes are persisted/read back before the renderer acts; reload
cannot turn `committed_rebind_required` or `migration_required` into `None`.

Neither fingerprint/digest constructs or hashes plaintext `apiKey`; legacy
plaintext contributes only a stable safe `legacy_plaintext_present` credential
state and therefore cannot leak. Snapshot/version CAS uses the same device-local
Provider state epoch; every Universal/child managed writer increments it.

`affectsCodex` is true when actual child presence, prior membership, proposed
membership, or a present-child redacted projection change is observed. Thus the
legacy state `apps.codex=false + actual child present` is always blocked. Under
the same lock, a matching allowed snapshot creates one unforgeable
`UniversalMutationPermit` and consumes it immediately before the first write.
Blocked operations return `universal_codex_change_plan_unavailable` with zero
universal/per-app/event/cache/epoch/other-app writes.

All actual writes live only in module-private
`commit_universal_mutation(preparedExactMutation, secretLeases, permit)`. The
prepared mutation contains reference-native storage payload plus exact
non-secret child projections; leases are attempt-memory only. The permit has
private fields, is neither `Clone` nor serde-serializable, and binds action,
source/target IDs, exact prepared-payload digest, snapshot revision token, and
Provider epoch. It is consumed by value; wrong action/ID/payload, reuse, or
production construction outside the owning module cannot reach IO. The allowed
non-Codex private commit structurally enumerates only non-Codex targets; it
cannot call Codex save or delete even as a no-op. It rechecks the universal
fingerprint, actual child presence/digest, and provider epoch against the
permit, and consumes/zeroizes leases on every exit. `AddProviderDialog`,
`UniversalProviderPanel`, and form modal use the
single command for create/edit/duplicate/save/save-and-sync/delete/manual-sync.
For blocked create/edit/duplicate/save, a separate app-specific Codex Plan may
be opened while cancelling the Universal operation. Delete/remove/resync/
manual-sync offers only return/cancel plus adapter-required guidance. A
credential-free non-Codex-only operation with no actual Codex child continues
before #35; secret-bearing variants wait for the adapter/migration. The renderer
and mutation path never forward raw Universal `apiKey`; only #35's explicit
migration may convert legacy plaintext into reference-native storage.

Codex deep-link routing is governed by the owning
`deeplink-import-security.md` contract. Its only output is closed
`CodexDeepLinkPlanDraftV1 {schemaVersion,draftId,app,operation,name,homepage,
primaryEndpoint,additionalEndpoints,iconCode,model,credentialStatus,source}`.
`enabled` selects create-only versus create-and-select intent but grants no
approval. API key, config/configUrl/configFormat, all usage fields/scripts/
tokens, notes, cross-resource fields, unknowns, and `activationApproved=true`
are rejected; homepage/endpoints reject URL userinfo, query, and fragment, and
no config URL is fetched. The shared full-input fixture must equal
the safe DTO field-for-field and every forbidden-field fixture must prove zero
add-draft/endpoint/switch/Plan/secret/network effect.

## 9. Operational and security-adjacent boundaries

- Plan insert/lifecycle is local sensitive metadata and excluded from external
  business sync, remote import, diagnostics, SQL exports, and sanitized
  app-managed backups; private sentinel scans cover every path.
- Actor is an opaque local code, not OS username/email.
- Logs contain stable diagnostic codes only.
- Provider list/edit redaction is a prerequisite for production create/edit;
  otherwise plaintext could enter renderer/query state before Change Plan.
- Endpoint model fetch/speed test remains explicit editing assistance and is not
  called by preview/apply. It cannot persist endpoint rows before apply.
- Native startup smoke must use isolated test state to avoid mutating the user's
  actual schema/default-pricing/backup state.

## 10. Compatibility and rollback

- v1 terminal job readback remains available.
- v1 unconsumed Plan returns unsupported schema/re-preview.
- v2 capability can enable only credential-free hardened switch before #35;
  secret-bearing switch/create/edit return typed `dependency_unavailable` until
  #35 exact SHA, credential migration, and redacted list/edit DTO gates pass.
- No supported Codex entry silently returns to direct mutation when its Change
  Plan capability is unavailable.
- Before any rollback, capability gates disable Universal and secret-bearing
  Plan entry. UI and workers may roll back only while safe view commands,
  Universal credential parser/adapter, migration marker, old-writer guards, and
  #35 resolver remain installed. Once a reference-native row or migration marker
  exists, downgrade below its minimum database version is forbidden; adapters,
  guards, storage columns, and the future-version startup preflight are never
  removed by ordinary commit rollback. The exact safe predecessor returns
  `db_version_too_new` or `database_compatibility_unknown` after marker/header
  inspection and before SQLite open/DDL/business read/write/sync/network. V2
  Plan rows
  separately retain their existing old-binary fail-close behavior; additive
  fields and v1 history remain harmless/readable.
- Sanitized import/restore never synthesizes or transports a credential. If a
  matching device-local binding exists it is retained; otherwise the safe view
  persists `NeedsLocalRebind`; the safe import/restore is reported as committed,
  while any subsequent Universal mutation reports
  `credential_rebind_required/no_effect`. Only #35 secure rebind can clear it.
- Main merge/deployment are outside this task.

## 11. Early #41 handoff

After architecture/detail freeze, send a docs-only
`DESIGN_CONTRACT_HANDOFF_SHA` explicitly marked non-compilable/non-consumable so
#41 can plan without inventing a schema. After the minimum #55 source-contract
slice and before downstream or broad feature integration, deliver a separate
`CONSUMABLE_CONTRACT_HANDOFF_SHA` containing:

- Rust DTO + TS decoder + shared fixture;
- canonical digest version/domains/vectors;
- resource/baseline model;
- Plan/job persistence/read APIs;
- lifecycle/admission invalid reasons;
- one-confirmation immediate-planned handshake;
- worker ownership/CAS, device-local coordination epoch, readback-only manual
  recovery recheck, and legacy IPC cutover guard;
- synchronized owning specs for Unified Change Plan, Codex Provider writer, and
  frontend routing, with an explicit v1 compatibility section;
- ownership note: #41 consumes core ledger/job and owns only its V2 workspace and
  domain-specific execution extensions. Its compile-time seam is a new closed
  operation enum variant plus domain adapter; it cannot create another Plan/job/
  event table, lifecycle, admission path, worker, or confirmation handshake.

Only the consumable handoff is called a frozen source contract, and only when
the same SHA contains all source artifacts above and static review is PASS. The
docs-only receipt never satisfies #41's integration gate and its delivery or
acknowledgement never blocks #55 source work. The consumable gate passes only
when #41 acknowledges the exact SHA/expected branch and base, producer and
consumer seam reviews both have `0 P0 / 0 P1 / 0 P2`, all required path hashes
and compatibility commands pass, `compatibilityStatus=pass`, and the seam
finding list is empty; every other result keeps #41 integration blocked.

## 12. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Additive v16 columns drift from a future migration | versioned DTO/fixtures; idempotent initializer; coordinate before main integration |
| Exact envelope stores sensitive metadata | backend-only access, no secret values, retention, export/log sentinel scans |
| Provider writer remains too monolithic | extract pure preparation/projection, retain single commit owner |
| #35 stays unavailable | fixture/port only; generic typed rejection remains registered, but no create/edit capability/UI routing; explicit dependency blocker |
| Cancellation races writer | one Provider-owned atomic effect gate; deterministic race test |
| External auto-sync sees private or partial state | ledger excluded everywhere; suppress/coalesce business sync; quarantine partial state; sentinel spies |
| UI reimplements state machine | backend snapshot authority; one decoder/query owner; exhaustive projection tests |

## 13. Architecture review gate

Review must close every P0/P1/P2 around single-ledger ownership, v16 additive
compatibility, canonical digest, exact writer payload, resource completeness,
secret dependency, cancellation/effect arbitration, query authority, retention,
and rollback before detailed design begins.

| Severity | Round 9 finding | Revision 10 closure |
| --- | --- | --- |
| P1 | The safe Universal mutation view was additive while the registered `get_universal_providers` / `get_universal_provider` IPCs could still serialize plaintext API keys | The existing command names now return only `UniversalProviderMutationViewV1`; no plaintext read IPC remains. Raw stored readers are module-private and non-serde, and secret sentinels cover IPC/query cache/events/DOM/logs/diagnostics. |
| P1 | `CredentialIntent` reached the Universal commit path without a #35-owned migration, lease lifetime, reference-native persistence, or failure protocol, contradicting unconditional non-Codex continuation | The exact #35 adapter/migration owns `None|Clear|Preserve|Replace`, reference-native Universal/child persistence, post-CAS/pre-permit minimum-lifetime lease resolution, attempt zeroization, and typed zero-write failure. Before handoff, only proven credential-free non-Codex `None|Clear` operations with no actual Codex child may continue. |

## Round 10

Product delta review is PASS (`0 P0 / 0 P1 / 0 P2`). Architecture review failed
with `0 P0 / 2 P1 / 0 P2`; its read-surface and Universal lease findings are
closed, but it found an enum collision and missing storage downgrade contract.

| Severity | Round 10 finding | Revision 11 closure |
| --- | --- | --- |
| P1 | Provider and Universal used incompatible definitions under the same `CredentialIntent` name | Split into closed versioned `ProviderCredentialIntentV1` and `UniversalCredentialIntentV1`, with separate canonical domains, schema-first/deny-unknown decoding, no implicit conversion, and per-variant/illegal-cross-domain fixtures. Only #35 maps Universal intent to an internal prepared requirement. |
| P1 | Reference-native Universal/child migration lacked an old-binary/downgrade/rollback contract | Added `UniversalCredentialStorageV1`, a fresh DB user-version marker, existing startup `db_version_too_new` pre-DDL fail-close, #35-owned all-or-disabled migration and sealed/no-plaintext backup gate, forbidden downgrade, rollback ordering that retains parser/adapter/guards, and safe sync/export/backup/import projections. |

## Round 11

Revision 11 product delta review failed with `0 P0 / 2 P1 / 0 P2`: safe
old-binary stop lacked a complete user recovery contract, and import/restore
rebind lacked a distinct user state. Revision 11b adds stable
`database_upgrade_required` and
`universal_credential_rebind_required/no_effect` projections, bounded actions,
four-locale accessibility, safe dependency reason codes, and exact acceptance
evidence. Product delta re-review passed (`0 P0 / 0 P1 / 0 P2`); architecture
re-review failed with `0 P0 / 3 P1 / 0 P2`.

| Severity | Round 11 finding | Revision 12 closure |
| --- | --- | --- |
| P1 | A new commit cannot make every historical plaintext-only binary present the new safe recovery UX, and exact source reads SQLite metadata | Migration now requires immutable `MIGRATION_GUARD_BASELINE_SHA` containing the safe UI/preflight and released as minimum predecessor. Acceptance runs that SHA. Earlier binaries get only pre-DDL/write fail-close, and copy says version metadata was read but business data was not initialized/migrated/modified. |
| P1 | `needs_local_rebind` was not persisted and contradicted import `no_effect` | Added `NeedsLocalRebind` to `UniversalCredentialStorageV1`, bound it into safe view/revision/digest/epoch/CAS, and made import commit safe fields plus the variant. Only the later blocked Universal mutation is `/no_effect`; #35 rebind transitions it to `SecretRef`. |
| P1 | Fresh DB user-version did not close raw SQL, remote sync, backup/restore, or existing-backup replacement paths | Added `UniversalCredentialTransferV1` and a closed copy matrix: exact settings parsing, temporary staging, new `DB_COMPAT_VERSION>6`/`db-vN`, no dual-write, row-level local-ref merge, monotonic marker, no raw safety backup, and legacy-backup quarantine. |

## Round 12

Product delta review is PASS (`0 P0 / 0 P1 / 0 P2`). Architecture review failed
with `0 P0 / 4 P1 / 0 P2`.

| Severity | Round 12 finding | Revision 13 closure |
| --- | --- | --- |
| P1 | `ca552f4d` used default SQLite open and continued initialization on inspection error, so predecessor guard was not side-effect-free/fail-closed | Safe predecessor now checks an atomic `DbCompatibilityMarkerV1` before SQLite; missing-marker fallback reads only the main-file header. It never touches DB/WAL/SHM, and all inspection errors return `database_compatibility_unknown` without `Database::init`. Current source is explicitly only successful-inspection evidence. |
| P1 | Local-ref retention used an undefined credential-requirement digest | Added canonical `UniversalCredentialBindingKeyV1` domain/fields/normalization/vectors over ID, slot, provider type, sorted app auth schemes and projected endpoints; storage, transfer and CAS share it, and any mismatch requires rebind. |
| P1 | Quarantined artifacts had kind only, no identity/revision/content CAS or owner/retention | Added device-local `CredentialArtifactRecordV1`, safe list/read, opaque `artifactId+expectedRevision` migrate/delete, private byte/manifest/ETag binding, owner lease, closed errors, explicit-only source deletion, interruption and retention rules. |
| P1 | Transfer sanitized Universal settings but could still create/update/delete Universal Codex child Provider rows | Added per-transfer Universal Codex impact snapshot and whole-transfer quarantine across staged/local membership, actual child and projected digest. Allowed no-impact paths structurally drop staged child rows and preserve exact local membership/child across every copy family. |

## Round 13

Revision 13 product delta review found one P2 for overclaiming completed migration
while the marker may still be pending. Revision 13b uses neutral, provable copy;
product delta re-review passed (`0 P0 / 0 P1 / 0 P2`); architecture re-review is
failed with `0 P0 / 3 P1 / 0 P2`.

| Severity | Round 13 finding | Revision 14 closure |
| --- | --- | --- |
| P1 | Compatibility marker lacked closed schema, fresh-install branch, WAL/hot-journal handling, and cross-process inspection/open ownership | Added `DbCompatibilityLockV1`, exact marker fields/checksum/atomic publication, full disk-state matrix, process-lifetime shared versus migration-exclusive lease, all-absent bootstrap, exact header validation, sidecar/journal fail-close, and concurrency fixtures. |
| P1 | Binding digest used undefined length-prefix encoding and contradicted canonical v2 JSON/hash separator | Constructor now performs explicit NFC/IDNA/port/RFC3986 preprocessing, then uses existing canonical UTF-8 JSON and `domain || 0x00 || bytes`. Exact Unicode/default-port/dot/percent/trailing-path bytes and two computed digests are frozen. |
| P1 | Artifact authority lacked one storage topology and post-effect receipt/reconcile/no-replay state machine | Added separate `CredentialArtifactStoreV1`, attempt/owner epoch/phase/ordered effect steps/receipts, pre-effect-only takeover, post-effect readback-only reconciliation, operation-specific delete/secret/candidate truth, store failure, retention and crash/dual-owner fixtures. |

## Round 14

Revision 14 product delta review failed with `0 P0 / 3 P1 / 1 P2`: compatible
pending markers, determined post-effect no-effect, source-preserving migration,
and explicit UI acceptance were incomplete. Revision 14b closes those contracts;
product delta re-review passed (`0 P0 / 0 P1 / 0 P2`); architecture re-review is
failed with `0 P0 / 3 P1 / 1 P2`.

| Severity | Round 14 finding | Revision 15 closure |
| --- | --- | --- |
| P1 | Shared process-lifetime DB lock had no executable runtime replacement transition; marker bag allowed impossible states/application IDs | Added stable lock-file maintenance drain/close/release/acquire/reinspect/reopen protocol with no in-place upgrade. Marker is a tagged BootstrapPending/MigrationPending/Ready union with field exclusions, ranges, monotonicity, `FYAG=0x46594147`, and legacy-0 boundary. |
| P1 | Artifact record/action result could not express reconciling, needs-help, candidate-ready, deleted or store-unavailable | Added deny-unknown `CredentialArtifactLifecycleV1`, `CredentialArtifactActionOutcomeV1`, exact safe view, revision transition matrix, illegal-combination rejection, and backend-derived actions. |
| P1 | Candidate file had no private CAS mapping to newly created SecretRefs or explicit apply protocol | Added private `CandidateCredentialBindingV1`, candidate lifecycle/apply APIs, fixed artifact→maintenance→DB lock order, ref receipt/binding/gate validation, one-time reference-native publish, crash/duplicate readback, pins and retention. |
| P2 | Path preprocessing did not order unreserved decode versus dot removal | Frozen exact six-step order and normative encoded-dot, encoded slash/backslash, repeated/trailing slash, Unicode and empty-path vectors. |

## Round 15

Revision 15 product delta review failed with `0 P0 / 3 P1 / 1 P2`. Revision 15b
closed those findings but re-review found `0 P0 / 1 P1 / 0 P2`: independent
source deletion had no transition while a candidate was pinned/applied. Revision
15c adds the Pinned/Applied/CandidateDeleted source-delete paths, blocks active or
needs-help candidate actions, preserves candidate/ref/main state, and carries
prior main generation through delete-candidate recovery. Product delta re-review
passed (`0 P0 / 0 P1 / 0 P2`); architecture re-review is pending.

## Round 16

Architecture round 15 failed with `0 P0 / 3 P1 / 0 P2`. Revision 16 closes:

| Round-15 finding | Revision-16 closure |
| --- | --- |
| Candidate main publish can strand a generic pending marker | Added tagged ReplacementPending(CandidateApply), prior/target identity/content/projection receipts, Ready completion receipt + sidecar acknowledgement, and exclusive pre-service exact-prior/exact-target/ambiguous readback-only recovery. |
| Original request revision is lost across needs-help/rechecks | Replaced overwritable last receipt with unique CandidateActionAttemptV1 ledger retaining action/requestRevision/attempt/digest and exact terminal safe snapshot; exact replay is scoped to its current result revision and a later action returns typed superseded plus current safe view. |
| Independent source/candidate timers can purge live authority | Replaced them with joint pinning and one shared-lock cross-record GC allowed only after both Deleted, all file/ref/action/recovery dependencies clear, and the later terminal/receipt anchor is 30 days old. |

Product delta review failed with `0 P0 / 2 P1 / 1 P2`. Revision 16b gives
observed-no-effect its determined copy, exposes delete-candidate from Applied,
and freezes superseded copy/current-view-only actions with exhaustive locale/a11y
acceptance. Product delta review passed (`0 P0 / 0 P1 / 0 P2`); architecture round-16
re-review is pending. No implementation/test/build/browser/server/runtime action
has run.

## Round 17

Architecture round 16 failed with `0 P0 / 2 P1 / 0 P2`. Revision 17 adds a
closed, variant-legal `DbCompletionAckV1` for exact-prior NeedsHelp and
exact-target Applied, plus exact marker+sidecar CAS clearing and mismatch/store
controls. It also adds monotonic source `candidateLineage` and makes
NeverPublished the only artifact-only GC authority; Published survives all
source lifecycles and missing counterpart is corruption. Product delta and
architecture round-17 re-review are pending. Product delta found
`0 P0 / 2 P1 / 0 P2`; revision 17b separates post-publish candidate authority
unavailable from pre-effect store failure and adds a closed pair-integrity safety
overlay with bounded UI/actions. Product re-review passed
(`0 P0 / 0 P1 / 0 P2`); architecture round-17 re-review is pending.

## Round 18

Architecture round 17 failed with `0 P0 / 2 P1 / 0 P2`. Revision 18 freezes the
action/reason/ack-specific recheck matrix: acknowledged exact-prior no-effect is
immutable and self-loops, unresolved apply alone may resolve exact target, and
delete recheck only resolves Deleted. It also adds backend-authoritative safe
candidate list/get plus invalidate/refetch events so a source-missing candidate
survivor remains discoverable after restart with pair-integrity actions forced to
zero. Product delta passed (`0 P0 / 0 P1 / 0 P2`); architecture round-18
re-review is pending.

## Round 19

Architecture round 18 failed with `0 P0 / 1 P1 / 0 P2`: a corrupt
source-A/candidate-C/source-B relationship could select two pair locks. Revision
19 elevates the stable sidecar lock to global
`CredentialArtifactIntegrityLockV1`; every action/scanner/GC/recovery holds it
exclusively across preflight, effects, readback, and publication before any DB
lock. Relationship IDs no longer grant lock authority. Split-brain concurrency
fixtures are frozen. Architecture round-19 re-review is pending; there is no
product semantic delta.

## Round 20

Architecture round 19 failed with `0 P0 / 1 P1 / 0 P2`: the Provider owning
spec retained a relationship-derived recovery lock and a pre-lock scanner
enumeration. Revision 20 removes both alternatives. Recovery and scanner now
acquire stable global `CredentialArtifactIntegrityLockV1` before any ID peek or
enumeration, perform a fresh complete enumeration/reread under that lock, and
retain it through readback, acknowledgement, and authority publication. Static
owning-spec assertions reject the removed sequences. Architecture round-20
re-review passed (`0 P0 / 0 P1 / 0 P2`); there is no product semantic delta.

## Round 21

Detailed-design review found `unified-change-plan.md` still used the ambiguous
phrase `source-artifact action lock`. Revision 21 replaces it with stable global
`CredentialArtifactIntegrityLockV1`, acquired before identity reads and retained
through effects/readback/acknowledgement/publication; per-ID locks are explicitly
non-authoritative and nested only inside it. Static stale-text assertions reject
the removed phrase. Architecture round-21 re-review is pending; there is no
product semantic delta.

## Round 22

Architecture round 21 passed (`0 P0 / 0 P1 / 0 P2`). Detailed-design review
found the handoff schedule conflated an early design receipt with the runnable
source contract. Revision 22 freezes two distinct receipts: a docs-only,
non-consumable `DESIGN_CONTRACT_HANDOFF_SHA`, followed after the minimum
contract/store/worker/decoder/fixture/guard slice by the only consumable
`CONSUMABLE_CONTRACT_HANDOFF_SHA`. #41 cannot integrate against the first. No
Plan/job/digest/product behavior changes. Architecture round-22 delta re-review
is pending.

## Round 23

Architecture round 22 failed with `0 P0 / 2 P1 / 0 P2`. Revision 23 removes the
docs-only notification from the source-unlock predicate: delayed #41 delivery or
acknowledgement never blocks #55. The consumable receipt now requires exact SHA,
expected consumer branch/base, required path hashes, producer and consumer seam
reviews at `0 P0 / 0 P1 / 0 P2`, zero-exit compatibility commands,
`compatibilityStatus=pass`, and an empty seam finding list. All other outcomes
keep #41 integration blocked. Architecture round-23 re-review passed
(`0 P0 / 0 P1 / 0 P2`).
