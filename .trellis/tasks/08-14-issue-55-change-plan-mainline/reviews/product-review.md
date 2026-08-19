# Product review

Evidence level: `source_report + code_audit`. No lint, typecheck, tests, builds,
browser/server, screenshots, or runtime commands ran in this stage.

## Round 1

`PRODUCT_REVIEW_ROUND_1=FAIL`

The reviewer found 7 P0, 8 P1, and 2 P2 items. Direction retained unchanged:
user problem/value, no Provider/model request, exact Plan identity, no force
apply, and local-readback vs real-use separation.

## Closure table for round 2

| Finding | Resolution | Status |
| --- | --- | --- |
| P0-01 scope | Integrated delivery card; related Issues stay independently governed | pending re-review |
| P0-02 #35 | Secret-bearing production admission is dependency-unavailable until exact SHA | pending re-review |
| P0-03 payload/privacy | Safe public projection and backend-only private envelope | pending re-review |
| P0-04 operation semantics | Explicit create-only/create-and-select/edit/switch and no-op outcomes | pending re-review |
| P0-05 proxy effects | Proxy takeover unsupported in first slice | pending re-review |
| P0-06 risk confirmation | Info/warning one confirm; critical unsupported | pending re-review |
| P0-07 lifecycle | Full preview/admission/job/query states and effect-boundary cancellation | pending re-review |
| P1-01 fields | Versions/status/revision/sources/action/readback/recovery and visibility split | pending re-review |
| P1-02 side effects | Only Plan payload/lifecycle metadata may persist | pending re-review |
| P1-03 invalidation | Lifecycle separate; sorted multi-reason admission drift | pending re-review |
| P1-04 idempotency | Same identity returns owning job; different digest rejects | pending re-review |
| P1-05 recovery | Typed modes and fixed readback-unavailable truth | pending re-review |
| P1-06 testability | Separate public/private fixtures and deterministic race hook | pending re-review |
| P1-07 privacy lifecycle | 24h/30d retention, opaque actor, private export exclusion | pending re-review |
| P1-08 compatibility | v1 unconsumed re-preview; terminal v1 jobs read-only | pending re-review |
| P2-01 wording | Conditional backup wording and three scenarios | pending re-review |
| P2-02 evidence/governance | Add prototype/local_readback; parent is router | pending re-review |

Technical and detailed design remain blocked until independent round 2 confirms
all P0/P1/P2 closed.

## Round 2

`PRODUCT_REVIEW_ROUND_2=FAIL`

Round 2 left 2 P0, 4 P1, and 1 P2. Fixes applied for round 3:

- task scope and #41 dependency direction now match the integrated-card PRD;
- official-target switch is unsupported until auth.json is modeled;
- admission ordering and consumed idempotency are explicit;
- no-op planning is an independent success projection;
- backup begins the durable effect boundary and cancellation races through one
  atomic gate;
- retention/export/v1 compatibility gained explicit acceptance criteria;
- task phase and related review metadata were updated.

`PRODUCT_REVIEW_ROUND_3=PENDING`

## Round 3

`PRODUCT_REVIEW_ROUND_3=FAIL`

One P0 and two P1 findings remained. Round-4 fixes:

- 30-day retention starts only at terminalAt; nonterminal/recovery-required
  evidence cannot be timed-purged;
- abandoned, expired, and invalidated persisted lifecycle have typed outcomes;
- the flow places the atomic effect gate before optional backup, with backup as
  first durable effect when present and apply as first effect only otherwise.

`PRODUCT_REVIEW_ROUND_4=PENDING`

## Round 4

`PRODUCT_REVIEW=PASS`

All prior P0/P1/P2 findings are closed. Final review confirmed:

- only terminal jobs use terminalAt-based 30-day retention; recovery evidence is
  never timed-purged while nonterminal;
- abandoned/expired/invalidated/consumed admission behavior is typed and ordered;
- atomic effect gate precedes backup/apply and supports deterministic cancel
  races;
- no product scope conflict or user-owned decision remains.

#35 remains a recorded technical dependency, not an unresolved product choice.

## Architecture revision 3 product delta review

`PRODUCT_DELTA_REVIEW_ROUND_1=FAIL` (`0 P0 / 2 P1 / 0 P2`).

- Recovery terminology was not yet one stable enum across Plan, digest, job, UI,
  and acceptance criteria.
- Pre-admission secret metadata/capability rejection and post-admission lease
  acquisition/lifetime failure were not fully distinguished in terminal truth,
  UI, and acceptance evidence.

Round-2 revision unifies recovery as `none|manual_required`; backup supplies
manual hints only and readback-only recheck never restores. It freezes the
post-admission fault as one consumed Plan plus one readable owning job with
`failed + dependency_unavailable + no_effect + recovery=none`, zero
writer/backup/managed writes, and a required new preview/confirmation after the
dependency is repaired.

`PRODUCT_DELTA_REVIEW_ROUND_2=PENDING`

### Delta round 2

`PRODUCT_DELTA_REVIEW_ROUND_2=FAIL` (`0 P0 / 1 P1 / 1 P2`). D-01/D-02 core
semantics were closed; remaining work was explicit executable acceptance for
the recovery enum/recheck boundary and wording that still implied an automatic
recovery attempt.

Round-3 revision adds AC-18: all Rust/persistence/IPC/TypeScript decoders reject
recovery outside `none|manual_required`; backup provides hints only; recovery
recheck has zero writer/restore/compensate/backup/managed-write counters and may
change only fenced observation/snapshot/event state. Product copy now says
`manual recovery required` rather than `recovery failure`.

`PRODUCT_DELTA_REVIEW_ROUND_3=PENDING`

### Delta round 3

`PRODUCT_DELTA_REVIEW=PASS`

All delta P0/P1/P2 are closed. Recovery is one stable
`none|manual_required` enum with decoder and zero-effect recheck acceptance;
pre-admission metadata failure and post-admission lease failure are distinct;
backup/manual-help language makes no automatic restore or compensation promise.

## Architecture revision 4 product delta review

`PRODUCT_DELTA_REVIEW=PASS`

The added product-facing contracts introduce no new P0/P1/P2 or user decision:
post-admission pre-effect validation failure belongs to the existing no-effect
owning job; Plan discovery is a safe scoped read; abandon is revision-CAS
lifecycle metadata only; v1 recovery maps only after schema dispatch; and
invalidated Plans use `invalidatedAt + 24h` retention.

## Architecture revision 5 product delta review

`PRODUCT_DELTA_REVIEW=PASS`

Abandon is limited to unexpired ready Plans; equality/race persists typed
expired without resetting retention. Post-effect partial/third state remains
authoritative-readback `failed + manual_required`, with manual help/readback-only
recheck and no automatic restore, replay, success, or restart implication.

## Architecture revision 6 product delta review

`PRODUCT_DELTA_REVIEW=FAIL` (`1 P0 / 2 P1 / 1 P2`). The all-entry negative gate
was present, but the generic out-of-scope fallback overlapped unsupported Codex
switch subcases; tray/deep-link positive navigation was not accepted; Profile
had no whole-apply-unsaved projection; and ProviderService wording included
read methods.

Revision 6b narrows legacy routing to named non-create/edit/switch operation
families, keeps every proxy/official/critical subcase protected and typed
fail-closed, adds exact-target tray and safe-field deep-link acceptance, defines
translated accessible `profile_change_plan_required`, and names only legacy
ProviderService mutation methods.

`PRODUCT_DELTA_REVIEW_REVISION_6B=PENDING`

### Revision 6b re-review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 1 P1 / 0 P2`). Prior findings closed, but
unsupported subcases and the generic legacy guard both claimed first-return
priority.

Revision 6c freezes one pure order at every entry: classify target/mode/risk
first and return specific typed unsupported for proxy/official/critical; only a
supported normal-mode request entering a legacy write path returns/routes
`change_plan_required`. Both outcomes precede all managed effects.

`PRODUCT_DELTA_REVIEW_REVISION_6C=PENDING`

### Revision 6c re-review

`PRODUCT_DELTA_REVIEW=PASS`

All product P0/P1/P2 are closed. Pure unsupported classification precedes the
legacy guard; supported normal-mode bypass alone returns
`change_plan_required`; both are zero-effect and AC-20 covers their distinct
results.

## Architecture revision 7 product delta review

Revision 7 replaces Codex deep-link direct import with a closed safe
draft-to-Plan DTO and removes Universal Codex membership/materialization from the
legacy whitelist. Codex-affecting Universal operations are whole-operation
unavailable until a UCP/#35 adapter; non-Codex-only operations remain.

`PRODUCT_DELTA_REVIEW_REVISION_7=PENDING`

### Revision 7 re-review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 2 P1 / 1 P2`). Safe URLs still allowed
query/fragment credential smuggling; Universal guidance did not distinguish
create/edit from delete/resync; and out-of-scope “import” overlapped the new
zero-write Codex deep-link route.

Revision 7b rejects URL userinfo/query/fragment and precisely limits raw/rejected
URL exposure; splits Universal next actions by operation; and narrows out-of-
scope import to legacy import-default/bulk import while keeping Codex deep-link
safe draft routing in-scope.

`PRODUCT_DELTA_REVIEW_REVISION_7B=PENDING`

### Revision 7b re-review

`PRODUCT_DELTA_REVIEW=PASS`

All product P0/P1/P2 are closed. Deep-link safe URLs cannot carry query/fragment
credentials; Universal next steps are operation-specific and never imply an
unsupported delete/resync Plan; legacy import scope no longer overlaps the
in-scope zero-persistence Codex route.

## Architecture revision 8 product delta review

`PRODUCT_DELTA_REVIEW=PASS`

The single backend Universal mutation and actual-child/epoch snapshot tighten
the same whole-operation-unavailable promise. Operation-specific next actions
remain unchanged, and allowed non-Codex-only paths structurally skip Codex.

## Architecture revision 9 product delta review

`PRODUCT_DELTA_REVIEW=PASS`

Closed action variants, backend-issued safe revision authority, and the private
one-use commit permit preserve the same Universal whole-operation user contract;
stale/invalid/bypass paths are zero-write and non-Codex control remains usable.

## Architecture revision 10 product delta review

`PRODUCT_DELTA_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`)

The delta narrows pre-#35 continuation to proven credential-free non-Codex
`None|Clear` operations with no actual Codex child, makes the existing
Universal list/get command names safe-only, and adds the user-visible
`dependency_unavailable/no_effect` outcome for secret-bearing Universal work.
The whole-operation-unsaved message and repair/wait/reload-safe-view/retry next
action remain explicit. No new product ambiguity or user decision is open.

## Architecture revision 11 product delta review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`)

The technical delta separates Provider and Universal credential intent schemas
and makes reference-native migration an explicit forward-only compatibility
boundary. After migration, an old binary must show the existing upgrade-required
recovery screen before touching state; no ordinary rollback restores plaintext.

| Severity | Finding | Revision 11b closure |
| --- | --- | --- |
| P1 | Safe `db_version_too_new` stop had no frozen user copy, action set, localization, or accessibility acceptance | Added stable `database_upgrade_required` on `dbUpgrade`: says no data was opened/changed, allows only local upgrade guidance, an already-local verified compatible installer when available, or exit, and forbids continue/config mutation/downgrade/rollback/restore. Four-locale keyboard/screen-reader acceptance and zero activity are explicit. |
| P1 | Safe import/export privacy rules had no distinct user-facing local-rebind state | Added safe dependency reason enum and `universal_credential_rebind_required/no_effect`: explains credentials are intentionally absent, saves nothing, routes only through #35 secure rebind, reloads the safe view, and verifies artifacts omit ref/token/value. |

## Architecture revision 11b product delta re-review

`PRODUCT_DELTA_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`)

Both product P1s are closed. The old-build surface has bounded safe actions and
four-locale accessibility, and local rebind is a distinct no-effect state that
routes only through #35 secure entry before a safe-view reload. No user decision
remains open.

## Architecture revision 12 product delta review

`PRODUCT_DELTA_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`)

The delta narrows safe-upgrade UX to an immutable minimum predecessor, persists
`NeedsLocalRebind`, distinguishes committed safe import from a later no-effect
mutation, and quarantines unsafe legacy backup/remote generations behind staged
#35 migration.

Safe-upgrade UX is correctly scoped to the immutable minimum predecessor;
`NeedsLocalRebind` keeps committed import distinct from later mutation
`/no_effect`; and staged transfer/legacy-artifact actions are bounded,
accessible, reload-stable, and credential-free. No product decision remains
open.

## Architecture revision 13 product delta review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 0 P1 / 1 P2`)

The delta makes compatibility inspection truly fail-closed, binds local refs to
their exact safe destination, gives quarantined artifacts reload-safe identity/
CAS, and prevents SQL/sync/backup transfer from changing Universal Codex child
rows before an adapter exists.

| Severity | Finding | Revision 13b closure |
| --- | --- | --- |
| P2 | “A newer compatible FyAgent upgraded this data” was not provable for `migration_pending` or header-only detection | Four-locale baseline copy is now neutral: the data requires a newer compatible FyAgent; this build inspected only compatibility metadata and did not initialize, migrate, or modify business data. Actions are unchanged. |

## Architecture revision 13b product delta re-review

`PRODUCT_DELTA_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`)

The neutral copy is consistent across product, process, design, and owning
specs. It makes no completed-migration claim and preserves the bounded action
set. No product issue remains open.

## Architecture revision 14 product delta review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 3 P1 / 1 P2`)

The delta adds a complete DB compatibility admission matrix and user-visible
inspection failure, makes quarantine post-effect recovery readback-only, and
leaves binding mismatch behavior as secure rebind.

| Severity | Finding | Revision 14b closure |
| --- | --- | --- |
| P1 | Compatible interrupted pending marker had no user admission/outcome | Maps to neutral `database_compatibility_unknown(interrupted_bootstrap|interrupted_migration)`, no auto resume/init, with local help/compatible build/exit only. |
| P1 | Determined post-effect no-effect had no durable user terminal | Broadened needs-help covers observed no-effect/ambiguous/unavailable and permits only manual help or fenced readback-only recheck, never retry/migrate/delete. |
| P1 | Secure migrate might implicitly overwrite/delete original source | Migration publishes a separately named sanitized candidate only; apply is separate and delete-source exists only in confirmed delete. Source record cannot purge while source exists. |
| P2 | New recovery states lacked explicit locale/a11y/reload/no-secret/no-retry acceptance | Added per-state four-locale accessibility/reload assertions, post-effect control exclusion, public-surface sentinels, and original-source retention fixtures. |

## Architecture revision 14b product delta re-review

`PRODUCT_DELTA_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`)

All revision-14 recovery semantics are closed: interrupted pending is neutral
and fail-closed, post-effect no-effect is readback-only needs-help, migration
preserves original source, and every new state has explicit accessible,
reload-stable, no-secret/no-retry acceptance.

## Architecture revision 15 product delta review

`PRODUCT_DELTA_REVIEW=FAIL` (`0 P0 / 3 P1 / 1 P2`)

The delta adds bounded maintenance UI and explicit sanitized-candidate ready/
apply/readback states while preserving separate confirmed source deletion and
secure rebind semantics.

| Severity | Finding | Revision 15b closure |
| --- | --- | --- |
| P1 | Persisted candidate apply/delete recovery lacked an action discriminator | Candidate `Applying`, `NeedsHelp`, safe view, and outcomes now carry `apply|delete_candidate`; copy distinguishes possible main-DB apply from candidate/pin-only delete. |
| P1 | Artifact/candidate closed enums lacked a total UI mapping, including both deletion truths | Added an exhaustive backend-enum projection table, internal-only `Detected`, action-specific states, candidate/source deletion copy, `wasApplied` semantics, no-remigration source decision, and explicit safe fields. |
| P1 | Duplicate candidate delete after response loss could repeat cleanup | Private terminal `lastActionReceipt{action,requestRevision,resultRevision}` returns the exact persisted result for the same request and rejects action/revision conflicts without cleanup replay. |
| P2 | Acceptance covered only a subset of public lifecycle/outcome variants | AC-21 and owning specs now require every public artifact/candidate variant across four locales, a11y, reload, allowed actions, and secret/private sentinels, including deletion and action-specific needs-help. |

`PRODUCT_DELTA_REVIEW_REVISION_15B=FAIL` (`0 P0 / 1 P1 / 0 P2`)

The prior `3 P1 / 1 P2` findings are directly closed: candidate recovery now
retains its action, public enum projection is total, response-loss candidate
delete is idempotent, and AC-21/owning specs cover every public variant.

| Severity | New revision-15b finding | Required closure |
| --- | --- | --- |
| P1 | Original-source deletion is promised to be independent of a pinned/applied candidate (`design.md:868`, `process-state-machine.md:305`), but the legal transition table keeps source `CandidateReady` after apply and permits source-specific confirmed delete only from `CandidateDeleted` (`design.md:734-736`). Therefore the advertised “applied, then separately delete original source” action has no legal backend transition unless the user first deletes the candidate. | Preserve the approved independent-delete meaning: allow source `CandidateReady` with candidate `Pinned` or `Applied` to enter `PreEffect(delete) -> Reconciling -> Deleted` after separate source confirmation, without changing candidate/ref/main-DB state. Block it only while the candidate has an active/needs-help action, and add pinned/applied/candidate-deleted source-delete transition and reload fixtures. If delete-candidate-first is intended instead, product copy/actions and the independence claim must all be changed explicitly. |

Revision 15c preserves independent source deletion: source `CandidateReady` with
candidate `Pinned|Applied`, or source `CandidateDeleted`, may enter the confirmed
delete state machine; candidate `Applying|NeedsHelp` blocks it with zero effect.
Candidate/ref/main state is unchanged and reload fixtures cover all five cases.
It also makes candidate delete from Applied carry the exact prior main generation
through readback.

`PRODUCT_DELTA_REVIEW_REVISION_15C=PASS` (`0 P0 / 0 P1 / 0 P2`)

The revision-15b P1 is closed. Source `CandidateReady` with candidate
`Pinned|Applied`, and source `CandidateDeleted`, now enter the separately
confirmed source-delete state machine. Candidate `Applying|NeedsHelp` rejects
with `candidate_action_in_progress` and zero effect. Successful source deletion
preserves candidate/ref/main state across reload, while candidate deletion from
Applied retains `priorMainDbGeneration` through readback. No new product issue
or user-owned decision remains.

## Architecture revision 16 product delta review

Revision 16 adds a pre-service, readback-only recovery receipt for interrupted
candidate apply; a durable action-attempt receipt ledger for exact retries; and
joint source/candidate retention so no still-actionable authority disappears.
It introduces no new user action: exact prior/target observations project to the
existing candidate needs-help/applied states, while ambiguous DB truth keeps the
existing compatibility-unknown + candidate-needs-help surface.

`PRODUCT_DELTA_REVIEW_REVISION_16=PENDING`

### Independent revision 16 product verdict

This final verdict supersedes the pending marker above.

`PRODUCT_DELTA_REVIEW_REVISION_16=FAIL` (`0 P0 / 2 P1 / 1 P2`)

| Severity | Finding | Minimum product closure |
| --- | --- | --- |
| P1 | Exact-prior startup recovery is a determined `observed_no_effect` result (`prd.md:385-390`, `design.md:1418`), but the only candidate-apply needs-help copy says main-DB readback is “ambiguous or unavailable” (`process-state-machine.md:321`, `design.md:1013`). The UI therefore converts known no-effect truth into uncertainty and does not tell the user that this attempt will not replay. | Make `sanitized_candidate_apply_needs_help` reason-specific. For `observed_no_effect`, use: “Recovery verified that the main database is still the exact prior version. This candidate was not applied, and this attempt will not be replayed.” Keep only manual help or fenced readback-only recheck. Reserve the current uncertainty copy and compatibility-unknown pairing for `ambiguous|readback_unavailable`, and add four-locale/a11y/reload/allowed-action fixtures for both branches. |
| P1 | Candidate `Applied` permits `delete_candidate` in the backend (`unified-change-plan.md:466-469`), and joint GC cannot begin until candidate and source are both explicitly Deleted (`prd.md:362-366`, `design.md:966-978`), but the Applied UI offers only close and optional original-source delete (`process-state-machine.md:323`, `design.md:1015`). The documented UI has no action that lets an applied candidate reach Deleted, so users cannot intentionally enter the only purge-eligible lifecycle. | Add an explicit “delete sanitized candidate” action to the Applied surface, separate from source deletion, and state that it removes candidate recovery material without rolling back the applied main DB or deleting the original source. Explain that authority stays pinned until both records are explicitly Deleted, then joint GC waits 30 days; cover Applied -> delete-candidate -> Deleted and subsequent joint-GC eligibility across reload/locales/a11y. |
| P2 | `candidate_action_superseded` is a new public rejection returned with the current safe view (`prd.md:751-752`, `design.md:945-947`), but it has only a generic rejected-alert projection (`design.md:1019`) and no exact user copy or action-specific four-locale acceptance. | Specify: “A newer candidate action superseded this earlier request. Nothing was repeated. The current candidate state is shown.” Permit only dismiss/review plus actions derived from the current view, never historical controls; add four-locale/a11y/reload/no-secret/current-view fixtures. |

The replacement receipt, immutable attempt ledger, and joint-retention mechanism
are otherwise product-safe. These findings are requirement/copy gaps, not
technical unknowns, and require no new user-owned product choice.

### Revision 16b closure submitted for re-review

- `observed_no_effect` now says recovery verified the exact prior DB, candidate
  was not applied, and the attempt will not replay; ambiguous/unavailable retains
  uncertainty copy.
- Applied exposes explicit delete-sanitized-candidate, states no main rollback or
  source deletion, and covers the path to both-Deleted joint GC.
- `candidate_action_superseded` has exact no-repeat/current-state copy,
  dismiss/review plus current-view actions only, and complete locale/a11y/reload/
  privacy acceptance.

`PRODUCT_DELTA_REVIEW_REVISION_16B=PENDING`

### Independent revision 16b product verdict

`PRODUCT_DELTA_REVIEW_REVISION_16B=PASS` (`0 P0 / 0 P1 / 0 P2`)

All three revision-16 findings are closed. Candidate apply needs-help now gives
determined `observed_no_effect` exact-prior/not-applied/no-replay copy separately
from `ambiguous|readback_unavailable`, with help or fenced readback-only recheck
as the only actions. Applied exposes explicit candidate deletion and states that
it neither rolls back main DB nor deletes the source; acceptance covers the
Applied-to-Deleted path and joint GC only after the source is also Deleted.
`candidate_action_superseded` has exact no-repeat/current-state copy,
dismiss/review plus current-safe-view actions only, no historical controls, and
complete `zh|zh-TW|en|ja` accessibility/reload/privacy acceptance. The current
product, process, design, and owning-spec mappings agree. No new product issue,
technical unknown, or user-owned decision remains.

## Architecture revision 17 product delta review

Revision 17 makes Ready acknowledgement durable for both Applied and determined
no-effect NeedsHelp, and distinguishes a persisted NeverPublished source from a
corrupt missing counterpart before GC. Existing UI states/actions/copy remain;
the retention clarification is that source-only GC is legal solely when no
candidate was ever published.

`PRODUCT_DELTA_REVIEW_REVISION_17=PENDING`

### Independent revision 17 product verdict

`PRODUCT_DELTA_REVIEW_REVISION_17=FAIL` (`0 P0 / 2 P1 / 0 P2`)

| Severity | Finding | Minimum product closure |
| --- | --- | --- |
| P1 | Candidate-apply recovery now reports `store-unavailable` when the private sidecar cannot supply or persist the completion acknowledgement, including after the main DB may already match the target (`design.md:1464-1478`, `unified-change-plan.md:350-355`). The reused public copy instead promises that “sources and main DB remain untouched” (`process-state-machine.md:319`), and the total mapping says no candidate/main/source effect (`design.md:1053,1066`). That is false for a post-publish/Ready-before-ack failure and can make the user believe the candidate was not applied. | Split the projection by context or make the recovery copy neutral and exact: “FyAgent cannot verify the local candidate-apply authority. The main database may still be the prior version or may already contain the candidate; it remains closed and this apply will not be repeated.” Permit only local repair/help or exit—no apply/delete/retry/replay. Preserve the existing no-effect copy only for proven pre-effect store failure, and add four-locale/a11y/reload/private-sentinel fixtures at Ready-before-ack and ack-before-clear. |
| P1 | Published lineage plus a missing/mismatched candidate is declared `corruption/needs-help` (`design.md:761-768,1025-1028`; `process-state-machine.md:382-385`), but no existing public state can truthfully render it. Artifact NeedsHelp requires an active migrate/delete attempt and action-readback copy (`design.md:692-704`; `process-state-machine.md:315`), while Deleted is terminal and its UI says candidate actions remain independently governed (`design.md:752`; `process-state-machine.md:317`). AC-21 only asserts a corruption fixture (`prd.md:776-778`) and does not specify the safe result, copy, allowed actions, or reload behavior. | Define a closed safe pair-integrity outcome/projection (or a reason-specific, legally reachable existing outcome): “Local quarantine records are inconsistent. FyAgent retained the remaining record and deleted or reconstructed nothing.” Offer only local help/exit and reload of the safe view; forbid recreate/remigrate/delete/GC/retry. Keep the surviving authority pinned indefinitely and add four-locale/a11y/reload/privacy/no-effect fixtures. If the condition is intentionally internal-only, say so normatively and ensure no public Deleted view falsely advertises candidate actions. |

The `DbCompletionAckV1` success path itself needs no new user acknowledgement:
matching Applied and determined no-effect NeedsHelp safely resume ordinary
admission while retaining their approved UI actions. The monotonic lineage and
NeverPublished-only source GC rule are also clear and testable. The two findings
above are missing/incorrect product projections, not user-owned decisions.

### Revision 17b closure submitted for re-review

- Post-publish sidecar/ack failure now maps to neutral
  `candidate_apply_authority_unavailable`; only pre-effect StoreUnavailable says
  main/source are unchanged.
- Closed pair-integrity overlay maps Published missing/mismatch to
  `credential_artifact_pair_inconsistent`, pins the survivor, states nothing was
  deleted/reconstructed, and offers only help/exit/reload.
- Both states have complete four-locale/a11y/reload/privacy/failure-counter
  acceptance.

`PRODUCT_DELTA_REVIEW_REVISION_17B=PENDING`

### Independent revision 17b product verdict

`PRODUCT_DELTA_REVIEW_REVISION_17B=PASS` (`0 P0 / 0 P1 / 0 P2`)

Both revision-17 P1 findings are closed. Post-publish sidecar/ack failure has a
distinct `candidate_apply_authority_unavailable` projection with neutral
prior-or-target truth, closed-main/no-repeat behavior, and help/repair/exit only;
only proven pre-effect StoreUnavailable retains no-effect copy. Closed
`pairIntegrity=Inconsistent` overrides every underlying lifecycle, projects
exact retained/no-delete-or-reconstruct truth, suppresses all mutation and GC,
and pins every survivor indefinitely while allowing only help/exit/safe reload.
Both projections have complete four-locale accessibility, reload, privacy, and
zero-forbidden-effect acceptance. Product, process, design, and owning specs are
consistent; no new product issue, technical unknown, or user-owned decision
remains.

## Architecture revision 18 product delta review

Revision 18 makes acknowledged exact-prior no-effect recheck self-loop instead
of later claiming apply, and adds safe candidate list/get discovery for a
source-missing pair-integrity survivor. Existing copy and action limits remain;
candidate discovery only makes the already-approved safe state reachable after
reload.

`PRODUCT_DELTA_REVIEW_REVISION_18=PENDING`

### Independent revision 18 product verdict

`PRODUCT_DELTA_REVIEW_REVISION_18=PASS` (`0 P0 / 0 P1 / 0 P2`)

The acknowledged exact-prior branch remains truthful and reachable: its
write-once no-effect acknowledgement makes every later recheck a readback-only
self-loop, so unrelated main-DB rows cannot relabel the old attempt Applied or
Deleted. Existing “not applied / will not replay” copy and help/manual-resolution
or fenced-recheck actions remain consistent; unresolved apply and delete retain
their separately bounded outcomes. Backend-authoritative candidate list/get,
safe invalidate/refetch events, snapshot-wins cache behavior, and startup/reload
fixtures make a source-missing candidate-only survivor reach the existing
pair-inconsistent overlay without a pre-known ID. The overlay remains sticky,
zero-action, privacy-safe, and reload-stable, and stale-cache mutations are
zero-effect. No new product issue, technical unknown, or user-owned decision
remains.
