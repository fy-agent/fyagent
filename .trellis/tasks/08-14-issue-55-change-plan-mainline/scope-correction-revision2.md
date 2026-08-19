# Issue #55 execution scope correction — Revision 2

Status: `PROPOSED_PENDING_USER_APPROVAL`

Evidence class: `code_audit` only
Prepared: 2026-08-15 (Asia/Shanghai)

This addendum preserves the original Issue #55 outcome and the reviewed Change
Plan product contract. It corrects the execution scope and order after the first
Phase-2 attempt demonstrated that unrelated platform work had been pulled into
the critical path. It does not delete or rewrite Revision-1 commits, reviews,
prototype evidence, or the interrupted implementation draft.

## 1. Problem restated from first principles

A person preparing a Codex Provider change needs to know exactly what will
change, see that preview itself did not make the change, and be certain that one
confirmation can authorize only that saved Plan while its baseline remains
valid.

The first downstream consumer, Issue #41, needs one reviewable source contract
for Plan identity, canonical digest, baseline/resources, persistence/read,
invalid reasons, and the confirmation handshake. It is not helped by waiting
for unrelated global database, credential-artifact, Universal, or evidence-
publication subsystems.

## 2. Confirmed facts

- UCP terminal handoff is
  `6859e9ce04970008f4cf8b3d4883b4f70316291a`; its source implementation is
  `ca552f4d918cacc734f81f7efdef70619da139b8`.
- The existing switch slice already proves a useful narrow shape: Plan preview,
  one identity-bound apply call, stale/replay zero writer calls, and local
  readback. It remains additive on SQLite schema 16 and is not merged to main.
- Revision-1 product/design/prototype history remains immutable at
  `d158b27690d897e8e9f2ece7d8887da6423b899c`, `c859c62a`, and `3021c7f7`.
- Issue #35 still has no consumable immutable SecretRef handoff. Production
  secret-bearing create/edit/switch paths must remain typed-disabled.
- Issue #41 must consume #55's ledger/contract and must not create a second
  Plan/job store, digest definition, confirmation state machine, or command
  namespace.
- The interrupted parallel Phase-2 draft reached 8,453 inserted lines in one
  checkpoint before it had a terminal module gate. It is preserved, not
  accepted, at local branch `codex/issue-55-phase2-exploration`, commit
  `2478b7724d07a917fc1ff7a71507bcb225f7ea9a`; the original stash is retained.
- The Revision-2 proposal is immutable at
  `507b09913ce5f4e43eb61bc46a93492edbc4bebd`;
  `3021c7f7e98f15672e261843ecc75a7099787379` is its pre-addendum predecessor.
  Approval authority will be the later exact reviewed/fixed SHA recorded in
  `task.json`, not a moving branch tip.

## 3. Decision scorecard

Weights follow the project decision contract: user value 25, evidence 20,
cost/speed 20, risk/boundary 20, reversibility/reuse 15.

| Option | User | Evidence | Cost | Risk | Reversible | Total | Decision |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Continue the Revision-1 monolithic implementation unchanged | 14 | 8 | 4 | 7 | 6 | 39 | Reject: delays the person and #41 while coupling unrelated systems |
| Preserve the full outcome but close one stable surface at a time | 24 | 18 | 17 | 18 | 14 | 91 | **Select** |
| Delete the draft and restart as a narrow #55-only rewrite | 17 | 15 | 16 | 9 | 5 | 62 | Reject: discards others' work and risks silently shrinking the approved goal |

The selected option keeps the requested end state. It changes only execution
order, ownership, and the rule for when complexity may enter the critical path.

## 4. Revised task contract

### Outcome

FyAgent can generate a side-effect-free, identity-bound Codex Provider Change
Plan for create, edit, and switch; the saved Plan exposes the exact safe public
projection, invalidates on relevant drift/expiry/dependency change, and can be
consumed once without recomputing its meaning. Issue #41 receives an immutable,
reviewed contract and later the reachable execution seam. Final delivery still
includes product UI, failure paths, native evidence, review, push, PR, and
GitHub readback without merging main or deploying.

### In scope

1. Strict v2 Rust request/private/public DTOs and v1 read compatibility.
   The public snapshot retains stable `planId`, `planDigest`, `schemaVersion`,
   created/expires timestamps, intent, baseline fingerprint, affected
   resources, ordered actions, risk, warnings, redacted SecretRef status,
   preconditions, and recovery hints. Baseline/target/source/SecretRef/
   precondition drift or expiry requires re-preview.
2. One canonical JSON implementation and domain-separated intent, baseline,
   credential-requirement, and Plan digests.
3. Exact affected-resource and ordered-action matrices for Codex Provider
   create-only, create-and-select, edit-current, edit-non-current, and switch.
4. Side-effect-free preview: no business-state/current/live-file/tray/cache/job/
   backup mutation and no Provider/model outbound request.
5. Additive schema-16 persistence/read/discovery/lifecycle invalidation for the
   saved v2 Plan; no shadow ledger.
6. Strict TypeScript schema dispatch using Rust-authored fixtures.
7. Exact `planId + planDigest` one-confirmation admission contract; apply never
   accepts a draft or recomputes form intent.
8. Protected Provider preparation, atomic one-confirmation admission creating
   one durable owning job, registered worker/supervisor, private one-use effect
   permission, cancellation before effect, exact commit/readback, and
   readback-only interruption reconciliation for the first create/edit/switch
   slice; #35-dependent operations remain typed-disabled.
9. Prompt/Memory-V2-aligned four-locale Plan UI and required clean/warning/
   expired/drift/unsupported/secret-missing/running/recovery states.
10. Proportionate focused, integration, renderer, native, and failure-path
    evidence after the corresponding source surface is stable.

### Explicitly not on the Issue #55 critical path

The following may be separate follow-on tasks or later integration dependencies.
They do not block the first #41 contract handoff or Issue #55's first real
Codex create/edit/switch slice unless a concrete source dependency proves
otherwise:

- global replacement of every production `Database` holder with a new
  `DatabaseRuntime`;
- WebDAV/S3 remote-effect token and quarantine protocols;
- credential-artifact sidecar/scanner/GC and legacy candidate migration;
- Universal Provider mutation redesign;
- a new cross-process Chromium/evidence publication transaction when existing
  repository runners can produce the requested fresh evidence safely.

This is a scope correction, not permission to weaken preview purity, digest
identity, stale/replay zero-write behavior, one confirmation, readback, #35
typed-disable, #41 no-shadow-ledger, native/failure evidence, or final review.

## 5. Observable closure conditions

Evidence is labelled only as `source_report`, `code_audit`,
`runtime_screenshot`, `native_runtime`, `failure_path`, or `UAT`. Automated
evidence never implies UAT, and one class never substitutes for another.

| # | Result | Authority and evidence |
| --- | --- | --- |
| 1 | Same intent + same baseline yields the same semantic Plan digest; any relevant change yields a different/invalid Plan | Rust canonical vectors and TypeScript fixture decode (`source_report`); contract inspection (`code_audit`) |
| 2 | Preview for all five operation variants performs zero forbidden effects and zero outbound Provider/model calls | narrow side-effect spy and fault checks (`failure_path`) |
| 3 | v2 Plan persists and reads back without private/secret/path leakage; expiry/drift/secretRef/target/source/precondition changes invalidate it | focused DAO results (`source_report`) and invalidation matrix (`failure_path`) |
| 4 | #41 can compile/decode the schema from one immutable SHA without creating another ledger; full execution integration waits for the later guard/worker SHA | immutable source manifest and producer/consumer contract inspection (`code_audit`); exact command receipts (`source_report`) |
| 5 | Codex create/edit/switch routes through saved Plan → one admission → protected writer → independent readback; stale/replay/no-op paths call no writer | backend/frontend integration (`source_report`) and stale/replay/interruption matrix (`failure_path`) |
| 6 | The final UI answers what changes, resources, backup/recovery limits, credential/privacy boundary, invalid reason, and allowed next action in four locales | renderer/browser/native evidence, clearly separated from prototype (`runtime_screenshot`/`native_runtime`) |
| 7 | Final source SHA passes exact repository gates, review, push/PR/GitHub readback; no main merge or deployment occurs | terminal exits and immutable Git/GitHub receipts (`source_report`); final static review (`code_audit`) |

## 6. Shortest execution loop

### Slice A — CP-core, one writer

Owned files: `src-tauri/src/change_plan.rs`, narrowly sized
`change_plan/{contract,canonical,projection,capabilities}.rs`, and three shared
fixtures. Inspection ports remain traits/data only; no Provider IO or DAO.

Gate after files are stable: `git diff --check`, contract test, canonical test.
One terminal rerun per gate after the last relevant edit.

### Slice B — persistence/read, one writer after Slice A commit

Owned files: `database/dao/change_plan.rs` plus the smallest additive schema-16
delta needed by the v2 rows. Implement persistence, read, latest-scope
discovery, lifecycle invalidation, and retention required by Issue #55. Worker,
global DB-runtime migration, remote sync, and artifact state do not enter this
slice.

Gate: one focused store test plus legacy v1 read compatibility.

### Slice C — strict renderer contract and early #41 compile handoff

Replace permissive TypeScript fallbacks with strict v1/v2 dispatch, consume the
Rust-authored fixtures, and publish an immutable
`contract_surface_compilable_non_executable` receipt. This lets #41 review and
compile against the schema without pretending the Provider execution seam is
ready. The receipt is non-consumable: it authorizes schema review/compilation
only, never persistence ownership, command registration, execution integration,
or a second ledger. Only `source_contract_consumable` authorizes #41
integration.

### Slice D — Provider preview and execution seam

Add pure create/edit/switch preparation, non-repairing readers, capability
gates, atomic one-confirmation admission that creates one durable owning job,
registered worker/supervisor, private one-use effect permission, cancellation
before effect, the existing writer call, exact readback, and readback-only
interruption reconciliation. Only credential-free cases execute until #35
hands off an exact compatible SHA. After registration and focused review,
publish the existing `source_contract_consumable` #41 receipt.

### Slice E — UI and entry cutover

Implement the reviewed full-screen projection and cut over switch, create/edit,
then tray/profile/deep-link only in atomic backend+frontend commits. Preserve
non-Codex behavior.

### Slice F — integration and evidence

Run module tests first. After source freeze, run the exact runtime ladder once
per final SHA: lint/typecheck/unit/integration/renderer/browser/native/failure
path/Trellis/git diff. Add new evidence infrastructure only if an existing
runner cannot prove a named requirement; document that concrete gap first.

## 7. Ownership and test discipline

- One source writer at a time until the previous slice has an immutable commit.
- No concurrent Cargo processes against a shared dirty compile surface.
- No test while the owning file is still changing. Read the first failure once,
  fix the cause, then run one fresh terminal rerun.
- Do not use broad `mise run check`, browser, renderer, native, or server during
  contract/DAO construction. Broad gates occur only after module closure and at
  final source freeze.
- A line-count increase is not itself a defect, but any slice that exceeds the
  existing module by several times must identify the user/downstream invariant
  that requires each added subsystem before it can be accepted.
- Checkpoint `2478b772...` is salvage input only. Do not cherry-pick it whole;
  select and independently review the smallest coherent pieces.

## 8. Dependency and handoff truth

- #35: `PENDING`; no production adapter, plaintext fallback, environment map,
  or second credential store. Secret-bearing capability remains typed-disabled.
- #41 design receipt: created, delivery/readback still pending because the
  thread transport timed out. Do not claim receipt.
- #41 compile receipt: created only after Slices A–C pass at one immutable SHA.
- #41 consumable execution receipt: created only after Slice D is reachable,
  registered, focused-green, and statically reviewed at one immutable SHA.

## 9. Approval gate

No product source resumes until the user explicitly approves this Revision-2
planning summary. One explicit approval authorizes uninterrupted execution of
Slices A–F through final review, push/PR, and GitHub readback. Slice transitions
require fresh immutable predecessors and their focused gates, but no additional
user approval unless a material product-contract change, new authority boundary,
or real blocker appears.
