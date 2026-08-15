# Issue #55 architecture review

Evidence level: `design_review + exact-SHA code_audit@ca552f4d`.

No lint, typecheck, test, build, browser, server, or runtime command was run in
this phase.

## Round 1 — FAIL

Result: `ARCHITECTURE_REVIEW=FAIL` (`2 P0 / 6 P1 / 1 P2`). Option A (extend the
single UCP ledger in place) remains selected; no second state machine is needed.

| Severity | Finding | Revision 2 closure |
| --- | --- | --- |
| P0 | Private envelope can enter WebDAV/S3 sync, SQL/diagnostic export, remote import, and raw app backup | Design sections 4.4, 5, and 9 freeze ledger skip/preserve, export exclusion, sanitized backup, legacy-backup cleanup, effect-window sync suppression/coalescing, and sentinel tests. |
| P0 | Background worker, query reconciliation, cancel, and stale snapshots lack persisted ownership/CAS | Sections 5–7 freeze process lease, owner instance/worker epoch/phase, revision/effect/cancel CAS, query read-only behavior, orphan proof, pre-effect interruption terminalization, and readback-only reconciliation. |
| P1 | Resource/writer/CAS/action/recovery matrix incomplete | Section 4.4 freezes operation inclusion, all managed writers, provider epoch, action order, required readback, recovery modes, and sync disposition. |
| P1 | Additive v16 row discrimination and downgrade behavior undefined | Section 5 freezes nullable v1 columns, `schema_version=2`, inert legacy sentinel/status, compatibility matrix, and downgrade fail-closed claim. |
| P1 | Canonical digest inputs under-specified | Section 4.2 freezes three closed structs, null/array/integer/Unicode/dynamic JSON/TOML rules, private ref binding, typed reconstruction, and one vector per operation. |
| P1 | Public resource DTO exposes private fingerprints | Section 4.1 splits `AffectedResourcePublic` from backend-only `ResourceExpectation` and requires one-way projection/sentinel scans. |
| P1 | #35 gate conflicts with enabling hardened switch | Sections 3.3, 8, and 10 freeze credential-free pre-#35 capability, typed `dependency_unavailable`, generic registered command semantics, and no ID/plaintext fallback. |
| P1 | Retention misses invalidated/unobserved expiry and raw app backup | Section 5 defines anchors for ready-expired, abandoned, invalidated, terminal jobs, backup sanitization, legacy-backup cleanup, and user clearance. |
| P2 | Owning specs and #41 extension seam conflict with v2 design | Owning Trellis spec/indexes are revised with v1 compatibility and the handoff requires those specs plus a closed adapter/enum seam at the same SHA. |

## Round 2

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 4 P1 / 0 P2`). Both round-1 P0s
and all but two original P1s were closed; full scan found one additional IPC
authority P1.

| Severity | Finding | Revision 3 closure |
| --- | --- | --- |
| P1 | Secret lease missing from effect gate and post-admission failure contradicted no-job admission semantics | Design sections 3.3 and 6, product flow, and owning specs separate metadata rejection from terminal `dependency_unavailable/no_effect`, resolve exact minimum-lifetime lease before effect CAS, pass it explicitly, and forbid persisted lease/plaintext. Pre-#35 credential-free proof covers source/target/live/recovery. |
| P1 | `restore_from_backup` promised without an executable recovery protocol | First slice now has only `none|manual_required`; no automatic inverse/restore claim. Backups support manual hints and a fenced `recheck_change_recovery` performs readback only before quarantine can clear. |
| P1 | Durable Provider epoch lacked a monotonic local SSOT | Design and specs freeze device-local `change_coordination`, app scope, all-writer increments, sync/export/backup exclusion, restore/import `max(local, restored)+1`, and one preview/admission/effect SSOT. |
| P1 | Legacy Codex Tauri commands could bypass one confirmation after UI cutover | Design and specs freeze same-commit renderer/backend cutover, `change_plan_required` at direct IPC before writer/hooks, Plan-creation-only wrappers, and zero-writer direct IPC tests. |

## Round 3

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 4 P1 / 1 P2`). Device-local epoch,
legacy IPC cutover, and the manual-only recovery strategy were closed; the scan
found remaining state/API/compatibility reachability gaps.

| Severity | Finding | Revision 4 closure |
| --- | --- | --- |
| P1 | Post-admission digest/CAS/source/precondition/readability drift had no existing-job terminal truth | PRD AC-07, state machine, design effect gate, and spec freeze `failed + pre_effect_validation_failed + typed reasons + no_effect + recovery=none`, consumed Plan, zero writer/backup/managed writes, and new preview. |
| P1 | Lease was both pre-passed and resolved inside one ambiguous Provider API | Design/provider spec now freeze outer `apply_prepared_change(payload,CAS,requirements,effectGate)` and private `commit_prepared_change(payload,CAS,leases,effectPermit)` with one critical section, permit fencing, and lease zeroization tests. |
| P1 | Strict v2 recovery enum contradicted readable v1 legacy enum | Schema-first compatibility maps all three exact v1 values into the two v2 projections while preserving legacy result code; only v2 persisted/wire decoders reject other values; fixtures are required. |
| P1 | Ready Plan reload discovery and abandon lifecycle had no IPC authority | `find_latest_change_plan` and revision-CAS `abandon_change_plan` are now frozen with safe operation scope, pure-read/metadata-only behavior, typed no-change outcomes, and zero-effect tests. |
| P2 | PRD/AC retention omitted invalidated Plan | R-14 and AC-16 now name `invalidatedAt + 24h` and its clock-controlled purge case. |

## Round 4

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 3 P1 / 0 P2`). Round-3 state,
compatibility, discovery, and retention gaps closed; three ownership/race/error
promises remained.

| Severity | Finding | Revision 5 closure |
| --- | --- | --- |
| P1 | Design/UCP prose still assigned secret lease resolution to worker despite the chosen Provider-owned API | Worker now performs only pure stored-envelope/digest verification and passes requirements/effect gate. Outer ProviderService owns resource recheck, resolve, effect permit, private commit, and zeroization in one coordinator section; worker never sees a lease. |
| P1 | Physically ready but expired Plan could be abandoned and reset its retention anchor | Abandon uses injected clock and one CAS with `expires_at>now`; at/equal expiry persists typed expired, never abandonedAt. AC-19 covers equality, stale renderer, and expiry race. |
| P1 | Codex spec promised preserving prior live bytes for every mutation failure despite partial multi-file effects/manual-only recovery | Error matrix now separates pre-effect no-effect preservation from post-effect authoritative readback; mixed/partial remains observed actual state with `manual_required`, no auto restore/replay or success/restart signal, with focused tests. |

## Round 5

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). All round-4
findings closed; exact-source entry audit found two remaining bypasses.

| Severity | Finding | Revision 6 closure |
| --- | --- | --- |
| P1 | Cutover guarded only three `_with_result` IPCs while plain add/update/switch remained registered direct writers | Design/product/spec inventory now guards all six commands in the same cutover commit, before ProviderService/hooks, with six independent zero-writer/effect tests and registration/callsite scan. |
| P1 | Native tray and other direct native callers could bypass confirmation; tray writes proxy flags before switch | Inventory now covers tray, profile, deep link, old UCP, endpoint writers, and public ProviderService. Tray routes safe Plan UI before any write; profile/deep link fail before their first effect; public protected Codex writers always fail closed, and only private `EffectPermit` commit can write. |

## Round 6

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). The all-entry cutover,
positive navigation, return-code priority, and private permit closed; two owning
contracts still allowed definition materialization.

| Severity | Finding | Revision 7 closure |
| --- | --- | --- |
| P1 | Deep-link owning spec still mandated add-draft/switch and had no closed safe DTO | Owning spec now freezes exact `CodexDeepLinkPlanDraftV1`, field normalization/rejection table, #35 secure-entry boundary, zero-write UI routing, full-input shared fixture, and retains legacy behavior only for non-Codex. |
| P1 | Universal additive-app whitelist can directly upsert/sync plaintext Codex Provider definitions | Removed from whitelist. Universal commands/services/UI preflight old/new membership and projection before every whole-operation write; Codex-affecting operations are disabled until UCP/#35 adapter, with matrix/zero-effect tests and non-Codex control. |

## Round 7

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 1 P1 / 0 P2`). Deep-link contract
closed; Universal classification still lacked the actual materialized child and
a backend atomic/CAS boundary.

| Severity | Finding | Revision 8 closure |
| --- | --- | --- |
| P1 | Membership/projection preflight missed `apps.codex=false + existing child`, and renderer upsert→sync had TOCTOU/partial state | Replaced with one coordinator-owned revision/epoch-bound backend mutation. Snapshot binds redacted Universal row and actual child presence/epoch/redacted digest; child presence always blocks. Legacy write IPCs are guarded, one-use private permit precedes first write, and allowed non-Codex commit structurally omits Codex save/delete. Race/interruption/orphan-child/control tests are frozen. |

## Round 8

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). Actual-child/atomic
classification closed, but the new authority request and private writer seam
were not yet closed.

| Severity | Finding | Revision 9 closure |
| --- | --- | --- |
| P1 | Universal mutation request lacked IDs/action-specific required fields and no safe authoritative revision source existed | Closed deny-unknown Create/Edit/Duplicate/Delete/Sync enum freezes each required/forbidden field. Safe backend list/get supplies redacted draft, opaque revision token, Provider epoch, and child status; stale returns fresh view with zero writes; TS never computes authority. |
| P1 | Public ProviderService universal writers could still bypass the compound command/permit | All three return `universal_mutation_v2_required`; actual IO is module-private `commit_universal_mutation`. Non-Clone/non-serde by-value permit binds action, IDs, exact prepared-payload digest, snapshot token, and epoch. Direct bypass/forgery/reuse/misbinding/visibility/static-callsite tests are frozen. |

## Round 9

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). Closed action
variants and the private permit removed the write-side bypasses, but the read
surface and concrete Universal credential path were still open.

| Severity | Finding | Revision 10 closure |
| --- | --- | --- |
| P1 | Safe Universal mutation view was additive while registered legacy list/get commands could still serialize plaintext API keys | The existing `get_universal_providers` and `get_universal_provider` command names return only the safe mutation view. No plaintext read IPC remains; raw `StoredUniversalProvider` readers are module-private/non-serde, with IPC/query cache/event/DOM/log/diagnostic sentinels. |
| P1 | Universal `CredentialIntent` had no #35 storage migration, lease lifetime, reference-native persistence, or zero-write failure protocol; unconditional non-Codex continuation was unsafe | The #35 exact-SHA adapter/migration owns `None|Clear|Preserve|Replace`, reference-native Universal/child storage, post-CAS/pre-permit minimum-lifetime leases, attempt zeroization, and typed `dependency_unavailable/no_effect`. Before #35, only proven credential-free non-Codex `None|Clear` operations with no actual Codex child can continue. |

## Round 10

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). The round-9 read
surface and Universal lease findings are closed; the full scan found an enum
identity collision and an incomplete storage downgrade boundary.

| Severity | Finding | Revision 11 closure |
| --- | --- | --- |
| P1 | Generic Provider and Universal used incompatible wire shapes under one `CredentialIntent` name | Split into backend-private `ProviderCredentialIntentV1` and safe-wire `UniversalCredentialIntentV1`, each schema-v1/deny-unknown with a distinct canonical domain. Only #35 maps Universal intent to an internal requirement; implicit casts and mixed payloads fail, with variant/domain fixtures. |
| P1 | `UniversalCredentialStorageV1` migration had no old-binary, downgrade, backup, or rollback contract | #35 allocates a new DB user-version marker and closed persisted discriminator; old plaintext-only binaries hit existing `db_version_too_new` before DDL/read/write/network. Migration is all-or-disabled, cannot make an ordinary plaintext backup, and downgrade is forbidden; rollback retains parser/adapter/guards and safe sync/export/backup/import projections. |

## Round 11

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 3 P1 / 0 P2`). The enum collision is
closed; full scan found an impossible all-historical-binary UX claim, an
unpersisted rebind state, and uncovered DB replacement/remote generations.

| Severity | Finding | Revision 12 closure |
| --- | --- | --- |
| P1 | New source could not guarantee safe `dbUpgrade` copy/actions for every older binary; exact preflight reads SQLite metadata | #35 enablement now requires released immutable `MIGRATION_GUARD_BASELINE_SHA` containing safe UI/preflight. Acceptance runs that SHA; earlier binaries receive only pre-DDL/write fail-close. Copy accurately excludes initialization/migration/business-data mutation, not the read-only version inspection. |
| P1 | `needs_local_rebind` had no persisted discriminator and import writes contradicted `/no_effect` | `UniversalCredentialStorageV1` adds `NeedsLocalRebind`, bound by safe view/revision/digest/epoch/CAS. Import commits safe fields plus this state; only a later rejected Universal mutation is `/no_effect`, and only #35 rebind can create `SecretRef`. |
| P1 | Startup user-version did not protect SQL import/export, WebDAV/S3 generations, backup restore, or existing plaintext backups | `UniversalCredentialTransferV1` plus the copy matrix covers exact `settings` parsing, staging-before-replace, new remote compat/layout with no dual-write, local-ref merge, monotonic marker, sanitized backups, raw-backup prohibition, and legacy-backup quarantine. |

## Round 12

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 4 P1 / 0 P2`). Round-11 directions
were sound, but exact-source preflight, ref-binding identity, artifact authority,
and transfer-time Codex child fencing remained incomplete.

| Severity | Finding | Revision 13 closure |
| --- | --- | --- |
| P1 | Current predecessor check used writable/default SQLite open and continued on inspection error | Safe baseline uses atomic marker then read-only main-header fallback before SQLite open; DB/WAL/SHM stay untouched, all inspection errors are `database_compatibility_unknown`, and `Database::init` is unreachable. Current source claim is narrowed. |
| P1 | Local ref merge depended on an undefined requirement digest | Frozen `UniversalCredentialBindingKeyV1` canonical domain, exact scope fields, normalization, fixed vectors, shared storage/transfer/CAS, and mismatch-to-rebind behavior. |
| P1 | Artifact kind could not identify or CAS multiple/replaced quarantine sources | Added local artifact record, opaque ID/revision APIs, private bytes/manifest/ETag binding, owner lease, closed errors, interruption/reload, explicit deletion and retention/exclusion. |
| P1 | SQL/sync/backup transfer could bypass Universal Codex child gate via ordinary Provider rows | Added transfer impact snapshot and whole-transfer zero-write/quarantine; allowed paths drop staged child rows and preserve exact local membership/child across all four families. |

## Round 13

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 3 P1 / 0 P2`). Transfer child fencing
is closed; compatibility admission, binding canonical bytes, and artifact
post-effect ownership were incomplete.

| Severity | Finding | Revision 14 closure |
| --- | --- | --- |
| P1 | Marker was not a complete atomic SQLite admission protocol | Closed lock/marker schemas, process-lifetime shared and migration-exclusive ownership, all-absent bootstrap, header/WAL/journal matrix, atomic publication, inspection/open exclusion and error/concurrency fixtures. |
| P1 | Binding canonical encoding contradicted the v2 canonical contract | Explicit semantic preprocessing now feeds existing canonical UTF-8 JSON and `domain||0x00||bytes`; exact Unicode/default-port/dot/percent/trailing canonical bytes and two SHA-256 values are normative. |
| P1 | Artifact record lacked unique store and post-effect no-replay recovery | Separate local sidecar store plus attempt/epoch/phase/step receipts, pre-effect-only takeover, post-effect readback-only reconcile, delete/secret/candidate classifications, store-failure and dual-owner/crash fixtures. |

## Round 14

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 3 P1 / 1 P2`). Marker admission,
binding canonical bytes, and artifact no-replay directions were sound, but
runtime lock transition, closed artifact outcomes, candidate ref handoff, and
path ordering remained incomplete.

| Severity | Finding | Revision 15 closure |
| --- | --- | --- |
| P1 | Shared→exclusive runtime replacement and marker variant legality were undefined | Stable lock maintenance protocol plus tagged variants, application ID/legacy boundary, numeric/monotonic constraints, and contention fixtures. |
| P1 | Artifact lifecycle/outcome/safe view could not represent recovery states | Separate closed lifecycle/outcome/safe-view types and exact revision transition matrix; illegal post-effect retry is structurally impossible. |
| P1 | Sanitized candidate was disconnected from new SecretRefs | Private candidate binding, explicit apply/delete/recheck, fixed lock order, exact validation/lease/publish/readback/idempotency, pins and retention. |
| P2 | Percent-decode/dot-removal order was ambiguous | Ordered constructor and normative encoded-dot/slash/backslash/repeated/Unicode/empty vectors. |

## Round 15

Status: pending independent re-review.

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 3 P1 / 0 P2`). The Round-14
maintenance/tagged-marker, artifact lifecycle/outcome, candidate binding, and
ordered path-canonicalization findings are closed in isolation. Full integration
review found three remaining durability contradictions across candidate apply,
compatibility admission, action receipts, and cross-record retention.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | Candidate apply has no reachable crash-recovery path through the compatibility guard. All replacements hold the DB lock from a pending marker through ready (`prd.md:450-456`), while a compatible abandoned pending marker explicitly forbids resume, SQLite open, or `Database::init` (`design.md:1330-1331`). Candidate recovery nevertheless requires the ready marker and exact main rows (`design.md:881-888`). `DbCompatibilityMarkerV1` has no candidate/attempt receipt (`design.md:1283-1307`) despite the claim that the marker binds both, so a crash after main publish and before ready publication strands the DB at compatibility-unknown and cannot reach candidate readback. | Add a tagged `ReplacementPending`/candidate-apply receipt containing candidate ID/generation, attempt ID, prior/target DB identity and target digest, plus a narrowly authorized exclusive-lock recovery transition that can classify pre-publish, fully published, and ambiguous states without starting business services. Ready or an atomically published private main-DB receipt must retain that identity until the sidecar reaches `Applied|NeedsHelp`. Cover crashes after pending, main replace/fsync, ready publication, and sidecar terminal publication. |
| P1 | `lastActionReceipt` cannot be reconstructed after the recovery path it is meant to protect. The only persisted terminal receipt contains `{action,requestRevision,resultRevision}`, but `Applying` and `NeedsHelp` retain action/attempt only and omit the original request revision (`design.md:809-820`, `design.md:855-865`). After a crash followed by one or more readback-only rechecks, the terminal writer has no durable source for the original revision, so the promised exact retry result (`design.md:914-923`) is not implementable. | Persist immutable `{action,requestRevision,attemptId,expectedCandidateDigest}` in every Applying/NeedsHelp variant or a dedicated candidate-action-attempt row, and copy it transactionally into the terminal receipt. If exact replay must survive a later valid action, retain a receipt ledger rather than one `last` slot; otherwise explicitly scope the guarantee. Add crash -> NeedsHelp -> multiple rechecks -> terminal -> original-request retry fixtures for both apply and delete. |
| P1 | Source/candidate retention can purge the authority still required by the surviving counterpart. A deleted source row may purge 30 days after source readback (`design.md:935-937`) while a pinned candidate is intentionally unbounded and later operations are defined against source `CandidateReady|Deleted` (`design.md:892-905`). Conversely, Applied candidate metadata may purge after 30 days while its candidate file and source `CandidateReady` link may remain (`design.md:905-915`). An absent row is not a legal lifecycle and carries no deletion receipt, so later candidate apply/delete or independent source delete cannot distinguish authorized purge from corruption and can no longer satisfy the shared-lock/CAS contract. | Pin the source tombstone while any candidate is Pinned/Applying/Applied/NeedsHelp, and pin candidate authority while source or candidate content still references it. Alternatively copy an authenticated source-deletion tombstone into the candidate record and atomically move the source link before purge. Define one cross-record GC transaction under the shared action lock; metadata may purge only after both records/files/ref ownership are terminal and no allowed action depends on them. Add >30-day source-deleted + pinned/applied-candidate and applied-candidate + live-source fixtures. |

The ordered canonical-path vectors, stable maintenance lock transition, closed
artifact/candidate public projections, source-delete versus candidate-action
CAS, discard-attempt no-replay fencing, prior-main-generation preservation, one
logical Plan ledger, additive v16/v1 compatibility, Provider writer/effect gate,
and #35/#41 ownership boundaries have no additional P0/P1/P2 finding in this
static review.

### Revision 16 closure submitted for re-review

- Tagged `ReplacementPending(CandidateApply)` plus Ready completion receipt and
  sidecar acknowledgement make pending/main/ready/sidecar crash boundaries
  reachable by one exclusive pre-service readback-only recovery path.
- Unique `CandidateActionAttemptV1` rows retain immutable action/original
  revision/attempt/digest and exact terminal safe snapshots across needs-help,
  repeated rechecks; replay is exact while result revision is current, and a
  later valid action yields typed superseded plus current safe view.
- Cross-record pinning replaces independent timers; one shared-lock transaction
  can purge only after source+candidate are Deleted, all files/ref ownership/
  attempts/DB receipts clear, and the later terminal/receipt anchor reaches 30
  days.

`ARCHITECTURE_REVIEW_ROUND_16=PENDING`

## Round 16

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). The immutable
candidate-action ledger closes the Round-15 request-identity/replay finding, and
the replacement and pair-GC directions are materially improved. Two durability
contracts remain incomplete at the exact cross-store boundaries they are meant
to authorize.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The Ready-receipt acknowledgement is not representable for the exact-prior branch. `CandidateActionAttemptV1` has `NeedsHelp` and `Terminal` states but neither carries the claimed marker-revision acknowledgement (`design.md:823-838`). Exact-prior recovery deliberately writes Ready and moves the candidate attempt to `NeedsHelp(observed_no_effect)` (`design.md:1424`), while receipt clearing is authorized only after a **sidecar terminal** records the marker revision (`design.md:1432-1435`; owning specs repeat this at `unified-change-plan.md:350-351,488-489` and `codex-provider-configuration.md:238-240,351-354`). `NeedsHelp` is explicitly nonterminal, and the closed schema has no acknowledgement field even for `Applied`. Therefore exact-prior recovery either leaves an unacknowledged Ready receipt forever, which blocks ordinary DB/service admission (`design.md:1445`), or clears it without the durable cross-store CAS evidence promised by the design. | Add a closed persisted `DbCompletionAckV1` (or equivalent variant-specific fields) to the candidate attempt authority, binding at least `{markerRevision,replacementId,attemptId,outcome,observedDbGeneration}`. Permit acknowledgement only from matching `Applied` or matching `NeedsHelp(apply,observed_no_effect)` after Ready exists; ambiguous/unavailable remains ReplacementPending and cannot acknowledge. Clear only by exact marker+sidecar CAS. Add both exact-prior and exact-target crash fixtures at Ready-before-sidecar-ack and sidecar-ack-before-marker-clear, plus mismatch/store-unavailable controls; ordinary admission resumes only after the matching acknowledgement is durable. |
| P1 | The unpaired-source GC exception has no durable authorization discriminator. The closed source lifecycle collapses every source deletion to `Deleted{attemptId,deletedAt}` and drops candidate lineage (`design.md:683-697`). The GC text simultaneously treats a missing counterpart as corruption and permits a source “provably never” published a candidate to purge alone (`design.md:984-988`; `unified-change-plan.md:546-554`), but it defines neither a monotonic `never-published` fact nor an exact append-only publish-receipt predicate. The general `effectSteps` list is not specified as immutable lineage through later source-delete transitions. Acceptance then says only both-Deleted joint purge succeeds (`unified-change-plan.md:680-681`; `prd.md:766-770`), omitting and textually contradicting the exception. A Deleted source with no candidate row therefore cannot authoritatively distinguish a legitimate never-paired source from a lost/corrupt formerly paired counterpart. | Persist a monotonic private lineage discriminator such as `NeverPublished | Published{candidateId,candidateGeneration,publishReceipt}` (or normatively make the equivalent publish receipt append-only), set `Published` atomically with CandidateReady, and retain it through CandidateDeleted/Deleted. Artifact-only GC must require exact `NeverPublished`, source absence, no candidate file/row/ref/attempt/receipt, the shared action lock, and the 30-day anchor; `Published` plus a missing counterpart is always corruption/needs-help. Qualify the “only both-Deleted” acceptance rule to paired histories and add positive never-published GC, publish-boundary crash, missing-counterpart corruption, paired-control, and concurrent GC/action fixtures. |

The Round-15 request-revision/attempt/digest ledger and current-versus-superseded
replay semantics are closed. Joint pinning and both-Deleted transactional GC are
also sound for an intact pair. The full static scan found no additional P0/P1/P2
in the single logical Plan ledger, additive v16/v1 compatibility, canonical
digests, ProviderService/effect gate, cancel/admission/worker/readback, deep-link
and Universal cutovers, external-sync exclusion, rollback, or #35/#41 handoffs.

`ARCHITECTURE_REVIEW_ROUND_16=FAIL`

### Revision 17 closure submitted for re-review

- `DbCompletionAckV1` now binds sidecar attempt revision, marker revision,
  replacement/attempt, outcome, and observed DB generation. Matching Applied or
  NeedsHelp(apply,observed_no_effect) may acknowledge; ambiguous/unavailable/
  delete may not. Exact marker+sidecar CAS alone clears Ready receipt.
- Private monotonic `candidateLineage=NeverPublished|Published{...}` changes to
  Published atomically with CandidateReady and survives CandidateDeleted/Deleted.
  Artifact-only GC requires exact NeverPublished plus no candidate/effect trace;
  Published+missing counterpart is corruption.

`ARCHITECTURE_REVIEW_ROUND_17=PENDING`

## Round 17

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`). Revision 17b closes
the Round-16 missing acknowledgement representation, exact Ready/sidecar CAS,
pre-effect versus post-publish failure truth, monotonic lineage, NeverPublished
GC predicate, and sticky pair-integrity action gate. The full integration pass
found one unresolved acknowledgement transition and one unreachable orphan-safe
query surface.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The completion acknowledgement is not stable across the candidate recheck state machine. Exact-prior recovery persists `NeedsHelp(apply,observed_no_effect)` with an `observed_no_effect` ack and then clears the only matching Ready completion receipt (`design.md:950-958,1523-1527`). The same contract nevertheless permits any candidate NeedsHelp recheck to resolve to `Applied|Deleted` (`design.md:935-936`; `unified-change-plan.md:500-503`) and requires crash → NeedsHelp → repeated rechecks → terminal (`prd.md:782-784`; `unified-change-plan.md:722-724`). `Terminal Applied` requires an `applied` ack (`design.md:950-951`), but a readback-only recheck cannot mint a second matching Ready receipt after the first was cleared, and overwriting the old ack would destroy the durable no-effect fact. Once ordinary services resume, a later unrelated mutation can also make current rows resemble the old target without proving that old attempt applied them. The documented transition is therefore either unreachable or falsely attributes a later state to the spent attempt. | Make the recheck matrix action/reason/ack-specific and keep `DbCompletionAckV1` immutable. After an exact-prior ack and receipt clear, the old apply attempt may only self-loop in resolved no-effect NeedsHelp or enter an explicit `NotApplied/manual-cleared` terminal that retains that ack; it cannot become Applied/Deleted from current-state readback. An unresolved apply while ReplacementPending may resolve to exact-prior+no-effect ack or exact-target+applied ack; delete NeedsHelp may resolve only Deleted. If later application is required, define a new authorized attempt/marker receipt rather than rewriting the old ack. Add post-clear repeated-recheck, unrelated-main-mutation-to-old-target, and ack-immutability fixtures, and scope the generic terminal acceptance accordingly. |
| P1 | A candidate-only pair-integrity survivor has no reachable public read path after reload. The new candidate record explicitly represents `source_record_missing` (`design.md:837-845`) and promises that the sole survivor remains pinned and can be safely reloaded (`design.md:778-785`; `prd.md:382-387`). But the only discovery APIs list/get source artifacts (`design.md:733-734`); candidate APIs are mutation-only and require a caller-supplied candidate ID (`design.md:894-899`). If the source row is the missing counterpart, a fresh renderer cannot rediscover the candidate ID or obtain `CredentialCandidateSafeViewV1`, so the required `credential_artifact_pair_inconsistent` projection and per-code reload fixtures (`prd.md:796-800`; `unified-change-plan.md:694-700`) are not implementable. The authority is safely pinned but becomes invisible and operationally unreachable. | Add a backend-authoritative safe `list/get` surface for candidate records, or a closed source-or-candidate recovery-authority union query, that can enumerate a standalone candidate after restart without reconstructing a source. Route it through IPC/query cache/event invalidation with the existing privacy sentinels and force the Inconsistent overlay to zero lifecycle actions. Add source-record-missing candidate-only startup/reload/list/get fixtures plus stale-cache and mutation-attempt zero-effect controls. |

The Round-16 exact acknowledgement persistence/clearance crash boundaries and
NeverPublished versus Published GC authorization are otherwise closed. The
post-publish authority-unavailable projection, pair-integrity sticky overlay,
indefinite pinning, and concurrent GC/action fencing are internally sound once
their public discovery path exists. No additional P0/P1/P2 was found in the
single Plan ledger, additive v16/v1 compatibility, canonical digests,
ProviderService/effect permit, cancellation/admission/worker/readback,
deep-link/Universal cutovers, external-sync exclusion, rollback, or #35/#41
handoffs.

`ARCHITECTURE_REVIEW_ROUND_17=FAIL`

### Revision 18 closure submitted for re-review

- Recheck is now action/reason/ack-specific. After exact-prior ack + receipt
  clear, apply NeedsHelp self-loops and never consults later target-like rows;
  unacked ReplacementPending alone can resolve exact prior/target, delete can
  resolve only Deleted, and ack bytes never change.
- Backend-authoritative safe candidate list/get and safe invalidate/refetch event
  enumerate a candidate-only survivor after restart. Pair Inconsistent suppresses
  all actions; startup/reload/stale-cache/privacy/zero-effect fixtures are frozen.

`ARCHITECTURE_REVIEW_ROUND_18=PENDING`

## Round 18

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 1 P1 / 0 P2`). The Round-17
action/reason/ack-specific recheck matrix and candidate-only safe discovery are
closed. The full scan found one remaining concurrency hole in the new integrity
scanner exactly when the records disagree about the lock identity.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | `CredentialArtifactIntegrityScannerV1` cannot guarantee its advertised action/GC fencing for an identity-mismatched pair. Normal source/candidate operations serialize on one lock derived from `sourceArtifactId` (`design.md:1003-1007`), while the scanner says only to acquire the pair's “normal” lock before persisting Inconsistent (`design.md:969-977`; `unified-change-plan.md:511-515`). But the states it is specifically required to detect include `source_identity_mismatch`, `candidate_identity_mismatch`, and `lineage_mismatch` (`design.md:684-689,840-845`). For example, source A can point to candidate C while C claims source B: a source-A scan/action locks A, whereas a candidate-C action locks B. The short sidecar transaction serializes row CAS only; it does not fence the staged/main/file/ref effect window held under the other action lock. The scanner can therefore mark C Inconsistent under A while an apply/delete/GC already validated and proceeds under B, contradicting sticky-overlay precedence and the promised zero actions. | Introduce a lock identity that does not depend on the disputed relationship. One minimal contract is: every candidate-associated source/action/GC/scanner path always acquires a stable candidate-ID lock plus every observed source-ID lock in canonical byte order; alternatively use one artifact-integrity maintenance lock shared by all such paths. Under that complete lock set, reread the connected records, reject/mark Inconsistent, and retain the locks through any permitted effect boundary. Define restart-on-expanded-lock-set without in-place upgrade or reverse acquisition. Add split-brain fixtures (`source A -> candidate C`, `candidate C -> source B`) racing scanner/list/get against candidate apply/delete, source delete, and pair/artifact GC; prove no deadlock, no file/ref/attempt/main/lifecycle effect, one sticky overlay CAS, and post-commit invalidation for every changed survivor. |

The write-once acknowledgement, exact-prior post-clear self-loop, unresolved
exact-prior/exact-target classification, delete-only terminalization, narrowed
acceptance, candidate list/get reachability, snapshot-authoritative
invalidate/refetch, overlay-only CAS, event-after-commit, persistence-failure
fail-close, and public privacy/action suppression are otherwise complete. No
additional P0/P1/P2 was found in the single Plan ledger, additive v16/v1
compatibility, canonical digests, ProviderService/effect permit,
cancellation/admission/worker/readback, deep-link/Universal cutovers,
external-sync exclusion, retention, rollback, or #35/#41 handoffs.

`ARCHITECTURE_REVIEW_ROUND_18=FAIL`

### Revision 19 closure submitted for re-review

Stable config-dir `CredentialArtifactIntegrityLockV1` is now the exclusive outer
lock for every artifact/candidate action, integrity scan, GC, and replacement
recovery across preflight/effect/readback/publication. Relationship IDs never
select authority. The order is integrity → maintenance → DB compatibility →
sidecar/main; split source-A/candidate-C/source-B races are explicit fixtures.

`ARCHITECTURE_REVIEW_ROUND_19=PENDING`

## Round 19

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 1 P1 / 0 P2`). The design, process,
PRD acceptance, and unified-change-plan owner now consistently introduce one
stable config-directory `CredentialArtifactIntegrityLockV1`, hold it across the
artifact/candidate effect window, and make disputed relationship IDs data rather
than lock authority. The Codex Provider owning spec still contains two older,
more-specific sequences, so the Round-18 split-brain finding is not closed in
the complete normative set.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | `codex-provider-configuration.md` still says that `DbReplacementRecoveryV1` first peeks the source ID and acquires a “source/candidate lock” before the compatibility lock (`codex-provider-configuration.md:233-235`). That is the relationship-derived authority Revision 19 is intended to remove and directly contradicts both the global-lock rule in the same owning spec (`codex-provider-configuration.md:330-333`) and the recovery contract that acquires `CredentialArtifactIntegrityLockV1` before reading any disputed relationship (`design.md:1552-1557`; `unified-change-plan.md:341-346`). The same owning spec orders scanner discovery as “enumerates IDs, acquires the global integrity lock, rereads all observed identities” (`codex-provider-configuration.md:352-356`), while the authoritative scanner contract requires enumeration itself under the global lock (`design.md:979-985`; `unified-change-plan.md:514-520`). A mutation between the pre-lock enumeration and acquisition can therefore be omitted from the supposedly backend-authoritative scan/list snapshot. These two normative alternatives permit either the original source-A/candidate-C/source-B split-lock race or a stale observed-ID set, despite the new fixtures promising one globally fenced view. | Delete the relationship-derived recovery sequence. Require recovery to acquire the stable global integrity lock before peeking or reading any source/candidate ID, then acquire compatibility exclusive, enumerate and reread the marker plus every observed sidecar identity under that lock, and retain it through readback/acknowledgement/authority publication. Likewise require startup/list/get/action scanners to acquire the global lock first and perform a fresh complete enumeration plus reread inside it; no pre-lock ID set may authorize or bound the scan. Keep optional per-ID locks explicitly non-authoritative and add a stale-text/spec assertion for the removed `peek source ID` / pre-lock-enumeration sequences alongside the existing split-identity races. |

The stable lock identity, full effect-window retention, optional-per-ID
non-authority, integrity-to-maintenance-to-compatibility ordering, paired and
NeverPublished GC fencing, and split-identity race fixtures are otherwise
coherent. No additional P0/P1/P2 was found in the single Plan ledger, additive
v16/v1 compatibility, canonical digests, ProviderService/EffectPermit,
cancellation/admission/worker/readback, deep-link and Universal cutovers,
external-sync exclusion, retention, rollback, or #35/#41 handoffs. The exact
source baseline remains unchanged from
`ca552f4d918cacc734f81f7efdef70619da139b8` under `src/` and `src-tauri/`; no
test, build, browser, server, or runtime command was run.

`ARCHITECTURE_REVIEW_ROUND_19=FAIL`

### Revision 20 closure submitted for re-review

The Provider owning spec now acquires stable global
`CredentialArtifactIntegrityLockV1` before any recovery ID peek or scanner
enumeration, freshly enumerates and rereads all observed identities under that
lock, and retains it through readback, acknowledgement, and authority
publication. Static owning-spec assertions reject relationship-derived recovery
locks and enumerate-before-global-lock sequences.

`ARCHITECTURE_REVIEW_ROUND_20=PENDING`

## Round 20

Result: `ARCHITECTURE_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`).

The Round-19 owning-spec contradiction is closed. Replacement recovery now
acquires stable global `CredentialArtifactIntegrityLockV1` before reading or
deriving anything from a source/candidate ID, then acquires compatibility
exclusive and freshly enumerates/rereads the marker and every observed identity
under both locks (`codex-provider-configuration.md:233-240`; `design.md:1555-1563`).
The scanner likewise acquires the global lock before performing its fresh,
complete enumeration and reread; no pre-lock set can authorize or bound the
scan (`codex-provider-configuration.md:355-361`; `design.md:982-991`;
`unified-change-plan.md:514-520`). Static acceptance rejects all three stale
forms—source-ID-derived recovery authority, `peek source ID`, and
enumerate-before-global-lock (`prd.md:811-817`;
`unified-change-plan.md:750-755`; `codex-provider-configuration.md:641-647`).

The resulting contract has one stable lock identity for every artifact/
candidate mutation, scanner, GC, and replacement recovery. It retains the
global lock from authoritative preflight through effect/readback and authority
publication, permits per-ID locks only as non-authoritative inner
optimizations, and preserves the single order integrity → maintenance drain →
DB compatibility exclusive → sidecar/main (`design.md:795-806`). Recovery
cannot start services or replay an effect; post-effect sidecar/ack failure keeps
the marker and main admission closed. Scanner publication remains an
overlay-only CAS followed by invalidation, with zero actions on persistence
failure. The split source-A/candidate-C/source-B, source/candidate action, GC,
and replacement crash boundaries therefore share the same fencing authority;
no reverse-acquisition, deadlock, stale-enumeration, or recovery-replay contract
remains.

The full static pass found no additional P0/P1/P2 in the single Plan ledger,
additive v16/v1 compatibility, canonical digest and payload boundaries,
ProviderService/EffectPermit ownership, cancellation/admission/worker/readback,
deep-link and Universal cutovers, external-sync exclusion, retention, rollback,
or #35/#41 handoffs. The exact source baseline remains unchanged from
`ca552f4d918cacc734f81f7efdef70619da139b8` under `src/` and `src-tauri/`; no
test, build, browser, server, or runtime command was run.

`ARCHITECTURE_REVIEW_ROUND_20=PASS`

### Revision 21 closure submitted for re-review

Detailed-design review found one additional ambiguous owning-spec phrase:
`source-artifact action lock`. The unified spec now names stable global
`CredentialArtifactIntegrityLockV1`, acquired before any identity read and held
through effects, readback, acknowledgement, and authority publication; per-ID
locks are explicitly non-authoritative and nested only within it. Static
assertions reject the old phrase and equivalent relationship-selected authority.

`ARCHITECTURE_REVIEW_ROUND_21=PENDING`

## Round 21

Result: `ARCHITECTURE_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`).

The remaining Round-20 lock-identity ambiguity is closed. The unified owning
spec now requires source/candidate actions to acquire the stable config-directory
`CredentialArtifactIntegrityLockV1` before any identity read and retain it
exclusively through authoritative preflight, every external effect, readback,
acknowledgement, and authority publication. Any per-ID or source lock is an
optional non-authoritative optimization nested only inside that global lock
(`unified-change-plan.md:555-559`). This agrees with the store-wide rule covering
every artifact/candidate action, scanner, GC, and replacement recovery
(`unified-change-plan.md:455-461`; `design.md:795-810`) and the Provider-owned
action contract (`codex-provider-configuration.md:330-338,382-391`).

Recovery, scanning, mutation, and GC therefore retain one lock identity and one
order. `DbReplacementRecoveryV1` takes the global integrity lock before any ID
peek/enumeration and then compatibility exclusive while it freshly rereads the
marker and every observed identity (`design.md:1555-1566`;
`codex-provider-configuration.md:233-241`). The scanner also takes the global
lock before its complete enumeration/reread (`design.md:982-991`;
`codex-provider-configuration.md:355-363`). Candidate apply uses integrity →
maintenance drain → compatibility exclusive → sidecar/main, source/candidate
actions retain the same outer lock across their effect window, and GC uses that
lock for its joint transaction (`design.md:1019-1026,1102-1115`). No
relationship-derived lock selection, reverse acquisition, stale pre-lock ID set,
split-brain effect window, or new deadlock alternative remains.

The removed phrase `source-artifact action lock` now occurs only in revision
history or explicit negative static assertions. PRD and both owning specs reject
that phrase together with `peek source ID`, relationship-derived recovery locks,
and enumerate-before-global-lock sequences (`prd.md:811-817`;
`unified-change-plan.md:754-760`;
`codex-provider-configuration.md:642-649`). It no longer grants normative
authority anywhere in the current PRD, state machine, design, or owning specs.

The full static regression scan found no additional P0/P1/P2 in the single Plan
ledger, additive v16/v1 compatibility, canonical/private payload boundaries,
ProviderService/EffectPermit ownership, admission/cancellation/background
worker/readback, deep-link and Universal cutovers, external-sync exclusion,
retention/rollback, or #35/#41 handoffs. The exact source baseline remains
unchanged from `ca552f4d918cacc734f81f7efdef70619da139b8` under `src/` and
`src-tauri/`. No test, build, browser, server, or runtime command was run.

`ARCHITECTURE_REVIEW_ROUND_21=PASS`

### Revision 22 handoff-sequencing delta submitted for re-review

The #41 path now distinguishes docs-only/non-consumable
`DESIGN_CONTRACT_HANDOFF_SHA` from the later runnable
`CONSUMABLE_CONTRACT_HANDOFF_SHA` containing the minimum canonical source seam.
Only the second can satisfy #41 integration. No Plan/job/digest/lock/product
semantics changed.

`ARCHITECTURE_REVIEW_ROUND_22=PENDING`

## Round 22

Result: `ARCHITECTURE_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`).

The named receipts now correctly distinguish a docs-only planning artifact from
a runnable source contract, and §11 preserves #55 as the sole ledger/job/worker/
confirmation owner while limiting #41 to a closed operation adapter and its V2
workspace (`design.md:1801-1826`). However, two handoff-gate contradictions
remain:

| Severity | Finding | Minimal required fix |
| --- | --- | --- |
| P1 | The design receipt is simultaneously non-blocking and a blocking source gate. `detailed-design.md:50-56` says the acknowledged `DESIGN_CONTRACT_HANDOFF_SHA` “does not block #55 source work” and is planning-only. `implement.md:23-30` nevertheless keeps #41 receipt/acknowledgement inside Gate 0 and forbids every source/test edit until that gate closes. This retains the scheduling contradiction called out by detailed-design review: an unavailable or delayed #41 readback stops #55 even though the receipt grants no compile/integration authority. | Move send/readback of `issue-41-design-contract.md` to an explicitly non-blocking post-freeze notification step (it may remain required before calling the planning notification complete), and remove #41 acknowledgement from the predicate that unlocks source/test edits. Gate 0 should close from local design freeze/immutable-SHA evidence; only `CONSUMABLE_CONTRACT_HANDOFF_SHA` gates #41 integration. Synchronize `detailed-design.md` and `implement.md` on that one rule. |
| P1 | The consumable receipt names readback fields but defines no closed acceptance predicate, and its compatibility command does not cover several artifacts that make the SHA consumable. `detailed-design.md:58-76` never requires `ackSha == CONSUMABLE_CONTRACT_HANDOFF_SHA`, a passing closed `compatibilityStatus`, or no blocking `openFindings`. Its only name-status command omits the required additive schema, command re-export/`lib.rs` registration, Provider guard, canonical fixture, and synchronized specs, although those are explicitly required at the same SHA (`detailed-design.md:58-63,96-114,204-205`; `design.md:1807-1825`; `implement.md:207-212`). A receipt can therefore be “verified” for the right commit object while acknowledging the wrong SHA, a failed compatibility result, or an incomplete source seam. | Define a closed consumer decision such as `compatibilityStatus=pass|blocked`; integration requires exact `ackSha`, the expected consumer branch/base, `pass`, and zero P0/P1/P2 affecting the seam. Add the missing required paths to the exact-SHA manifest/diff verification—or add named registration/Provider-guard/schema/canonical static tests that prove them—and state that any missing/hash-mismatched path or nonzero compatibility command keeps #41 blocked. Record the producer static-review receipt against the same source SHA. |

Outside these handoff authority gaps, the delta does not alter Plan/job/digest,
Provider effect, lock, cancellation, readback, retention, rollback, #35, or
product semantics. The one-ledger and one-confirmation ownership remains
coherent, and no additional P0/P1/P2 was found. The exact source baseline remains
`ca552f4d918cacc734f81f7efdef70619da139b8`; no test, build, browser, server, or
runtime command was run.

`ARCHITECTURE_REVIEW_ROUND_22=FAIL`

### Revision 23 handoff-gate closure submitted for re-review

The docs-only notification no longer gates #55 source work. The consumable
handoff now has one closed PASS predicate: exact SHA plus expected consumer
branch/base, full required-path hash set, producer and consumer seam reviews at
0/0/0, zero-exit compatibility commands, `compatibilityStatus=pass`, and empty
findings. Any mismatch remains blocked for #41 integration.

`ARCHITECTURE_REVIEW_ROUND_23=PENDING`

## Round 23

Result: `ARCHITECTURE_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`).

Both Round-22 P1 findings are closed. The docs-only receipt has been removed
from the Gate-0 predicate that unlocks source/test work. Gate 0 now closes from
local review/freeze/immutable-input evidence; the #41 design notification runs
afterward and explicitly states that send failure, delayed delivery, or missing
acknowledgement cannot block Phase 1 or any #55 source/test edit
(`implement.md:11-40`; `detailed-design.md:47-56`). It remains planning-only,
non-compilable/non-consumable, and never satisfies #41 integration.

The consumable source receipt is now one closed fail-closed predicate. The exact
handoff SHA must contain and hash the Change Plan composition/module tree, DAO
and additive v16 schema, command module/export/`lib.rs` registration, Provider
guard/commit owner, TypeScript API/query, both DTO/canonical fixtures, and the
UCP/Provider/frontend owning specs (`detailed-design.md:58-90`). #41 verifies the
exact commit and full path set, then runs four separate single-filter Rust
compatibility commands plus the TypeScript cross-layer command
(`detailed-design.md:92-100`). Integration is permitted only when
`ackSha == CONSUMABLE_CONTRACT_HANDOFF_SHA`, the expected consumer branch/base
match, producer and consumer seam reviews are both `0 P0 / 0 P1 / 0 P2`, every
path hash and command passes, `compatibilityStatus=pass`, and
`seamFindings=[]`; every other state remains blocked for #41 alone
(`detailed-design.md:102-108`; `implement.md:215-226`). The producer static
review is explicitly bound to the same source SHA.

This sequencing does not transfer execution ownership. #55 remains the sole
owner of the Plan/job/event ledger, lifecycle, admission, worker, event and
one-confirmation handshake; #41 may consume only the reviewed core seam and add
a closed operation variant/domain adapter plus its V2 workspace, never a second
store or state machine (`design.md:1803-1835`; `unified-change-plan.md:10-13`).
The later final-source notification is an evidence update, not an alternate
integration authority.

The full static delta scan found no new P0/P1/P2 in schema/digest/payload,
ProviderService/EffectPermit, cancellation/worker/readback, artifact locking,
external sync, retention/rollback, or #35 boundaries. The exact source baseline
remains `ca552f4d918cacc734f81f7efdef70619da139b8`; no test, build, browser,
server, or runtime command was run.

`ARCHITECTURE_REVIEW_ROUND_23=PASS`
