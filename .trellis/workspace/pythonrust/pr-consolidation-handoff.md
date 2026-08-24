# FyAgent PR Consolidation Handoff

Last verified: 2026-08-24 (Asia/Shanghai)

This is the durable cross-PR handoff for the current mainline consolidation.
It is intentionally broader than any one Trellis task. Before acting on a PR,
re-fetch GitHub state and compare against the current `main`; do not assume the
dynamic status below is still current.

## 1. Overall objective

The goal is **not** merely to review the remaining Draft PRs. Every relevant PR
must reach a clear terminal state:

- merge valid product capability into `main` through a current-main PR; or
- migrate the still-valid capability into a replacement PR and close the old
  stacked PR as superseded; or
- for non-code historical UAT PRs, migrate the findings to Issues and close the
  documentation PR without merging it.

Do not preserve a PR number at the cost of reintroducing obsolete architecture.
The latest verified `main` contracts take precedence over stale stacked branch
internals.

## 2. Current mainline baseline

Verified remote state at this handoff:

```text
origin/main           = bda0ffe74901dee53bacefb73a93484d428c44c3
origin/dev/laiyongjie = bda0ffe74901dee53bacefb73a93484d428c44c3
ahead/behind          = 0 / 0
```

`bda0ffe7` is the Merge Queue result of replacement PR #145 (SecretRef).

After every future main merge:

1. fetch the final `main` merge SHA;
2. confirm `dev/laiyongjie` has no independent commits;
3. only then fast-forward `dev/laiyongjie` to final `main`;
4. if dev has independent commits, do **not** force-move it; merge those commits
   through a separate PR first.

## 3. Repository merge and CI governance

The current repository policy was corrected and then validated through #142
and the P0 CI fix #144.

### Merge topology

Repository policy:

```text
Auto-merge         ON
Merge commit       ON
Squash merge       OFF
Rebase merge       OFF
Merge Queue        ON
Queue merge method MERGE
grouping           ALLGREEN
max build entries  2
max merge entries  1
min merge entries  1
wait               0
check timeout      30 minutes
```

Reason: FyAgent has an explicit upstream-sync contract that requires preserving
real two-parent ancestry. Global squash-only would destroy upstream provenance.
Use `git log --first-parent main` for a clean one-PR-per-mainline-node view while
retaining the full DAG for engineering investigation.

Commit hygiene and merge topology are separate concerns. Clean feature-branch
commits before merge-ready; do not use squash as the only mechanism for hiding
bad commit history.

### Required CI authority

After #144:

```text
Developer branch push
  -> Commit Convention / Push only

Pull Request
  -> Required CI (affected domains)
  -> CI / Required

Merge Queue
  -> merge_group
  -> Required CI (affected domains)
  -> CI / Required

Manual diagnostic
  -> workflow_dispatch
  -> Full CI
```

Ordinary `push` must not create `CI / Required`; `gh-readonly-queue/**` must not
run a second push authority. The P0 fix exists because #142 was ejected from the
queue multiple times by a `push`/`merge_group` concurrency race.

### Merge-ready boundary

Auto-merge / Merge Queue is only the final executor. A PR may be handed to
`Merge when ready` only after:

1. implementation and focused tests are complete;
2. maintained SPECs match final code semantics;
3. Trellis task evidence is current;
4. direct-session prearchive passes;
5. task is completed and archived;
6. post-archive contracts pass;
7. final diff/commit hygiene is clean;
8. exact-head PR is pushed.

Then use exact-head guarded auto-merge and let Merge Queue validate the latest
`main + earlier queued changes + current PR` candidate.

## 4. Completed PR dispositions

### PR #131 — macOS installed-app UAT

```text
state: CLOSED, not merged
```

Documentation-only UAT for installed FyAgent 0.4.2. Its findings were migrated
into Issue #141 because PRs should not be used as the long-lived tracker for
historical UAT findings.

Important findings preserved in #141 include:

- Prompts create/import potential live-prompt clearing risk;
- Daily Memory mixed/non-date Markdown failure;
- lower-priority V2 layout/control issues.

Do not reopen #131 for product fixes.

### PR #138 — Windows installed-app UAT

```text
state: CLOSED, not merged
```

Documentation-only UAT for installed FyAgent 0.4.0. Findings migrated to Issue
#141. Important Windows findings include Daily Memory failure, Authenticode
gap, disposable-profile/HIL limitations, DPI/focus evidence gaps, and lower
priority product/UX observations.

Windows signing implementation remains tracked independently by Issue #68.

### Issue #141 — unified historical UAT tracker

```text
state: OPEN
```

Contains the detailed #131/#138 history, evidence boundaries, P1/P2/P3
findings, untested boundaries, and current-main revalidation rules. Historical
0.4.0/0.4.2 failures must not be presented as current-main facts without a fresh
reproduction.

### PR #140 — V2 Models configuration safety

```text
state: MERGED
merge commit: 67f50b8ffdf4105b1e478f87fe60eca0af7dc9c2
```

Established important current-main invariants that later Change Plan work must
not regress:

- targeted Quick Setup patching instead of whole-file replacement;
- rolling adjacent backups and truthful write-target disclosure;
- Codex auth preservation semantics;
- protocol-correct connectivity probes;
- revision-based dirty state and stale probe invalidation;
- shared SecretInput layout safety.

### PR #135 — canonical Change Plan base

```text
state: MERGED
merge commit: 442ed9c91e180a8f9b8bff24c9385c7d731c98b9
```

This is the canonical Change Plan architecture. Future work must extend it, not
recreate a parallel implementation.

Canonical invariants:

- schema v20 is the only schema-v20 definition;
- `change_plans`, `change_jobs`, `change_job_events` are local-only and skipped
  / locally preserved by WebDAV sync;
- Provider mutation has one writer owner;
- plan baseline/digest/TTL/secret capability are enforced;
- apply uses real readback and does not equate writer return with success;
- crash recovery is readback-only and never replays the writer;
- Quick Setup target projection uses the same targeted-patch projection as the
  real writer;
- IPC accepts narrow IDs/digests rather than arbitrary configs/paths.

Never import the old #130 process-epoch/HMAC/private-proof design or a second
schema-v20 definition back into `main`.

### PR #142 — merge-governance SPEC

```text
state: MERGED
merge commit: d2340abc48094a766ce23615d95195d7bae12e45
```

Established durable Merge Queue + MERGE governance and corrected the earlier
squash-only policy.

### PR #144 — Merge Queue CI P0

```text
state: MERGED
merge commit: b296ed9e8a851c871805a69d0dfc50ee8964cd95
```

Separated Required CI from ordinary push policy and removed the
`push`/`merge_group` queue race. This topology must remain intact for all later
replacement PRs.

### PR #132 / #143 / #145 — SecretRef recovery chain

```text
#132 CLOSED Draft source
#143 CLOSED superseded replacement
#145 MERGED final replacement
#145 merge commit: bda0ffe74901dee53bacefb73a93484d428c44c3
Issue #35: CLOSED
```

Final #145 preserved the original #132 contributor ancestry while replacing
bad integration commits and hardening the native-store boundary.

SecretRef facts now available on main:

- strict SecretRef/material/error contracts;
- memory backend;
- macOS Data Protection Keychain + non-synchronizable semantics;
- Windows Credential Manager backend;
- process-global native backend serialization;
- secret material is not placed in ordinary DTO/debug output;
- no unproven destructive compensation on verification failure.

Important activation boundary for later #137 work: SecretRef core was kept
production-unregistered until the first real consumer can satisfy the signed
macOS Data Protection Keychain identity/entitlement boundary and a recoverable
create-admission story. Do not simply expose the dormant core from
`services/mod.rs` without re-checking the maintained SecretRef SPEC and current
production HIL evidence.

## 5. Remaining old stacked PRs

The remaining open chain is:

```text
#134 typed executor
  -> #136 V2 Change Plan UI
    -> #137 Codex Provider vertical
      -> #139 WorkBuddy adapter
```

These old PRs are useful implementation/evidence sources, **not** merge-ready
branches. Their stacked ancestry contains obsolete #130/#134-era ownership and
must not be merged directly into current main.

### PR #134 — typed executor source

Remote state at handoff:

```text
state: OPEN Draft
base: codex/issue-55-codex-switch-recovery
head: codex/issue-58-60-executor-recovery
head SHA: ec15bef545145528f03778003c586ebc1b64971e
issues: #58 #59 #60 (all OPEN)
```

Old PR value to salvage:

- typed adapter descriptor/registry;
- plan-level idempotency;
- pre-write cancellation;
- durable five-phase progress;
- structured partial truth;
- deterministic fault boundaries;
- snapshot-before-event observer contract;
- readback-only crash recovery.

Do **not** salvage:

- old #130 process epoch / HMAC / private proof;
- incompatible schema-v20 definition;
- second writer or second Change Plan lock;
- database status values that violate current v20 CHECK constraints.

#### Current replacement work

```text
branch: dev/change-plan-typed-executor-final
task: .trellis/tasks/08-24-change-plan-typed-executor-final
status: in_progress until final prearchive/archive closeout
```

Key commits already created:

```text
faac9b93 feat(change-plan): add canonical typed executor
60bd8548 chore(change-plan): sync final main baseline
4186fb92 test(rust): isolate integration fixture homes
35ea4634 fix(change-plan): correct adapter contract metadata
```

Current replacement design decisions:

- keep schema v20 unchanged;
- new phases:
  `precheck -> snapshot -> managed_write -> readback -> finalize`;
- legacy `apply/reconcile` remain decode-only and normalize on read;
- v20 has no `cancelled` DB status, so cancelled-before-write persists as
  coarse `failed + result_code=cancelled_before_write`, while the public DTO
  derives `status=cancelled`;
- `plan_id` remains the durable idempotency key;
- same plan + same digest returns the existing execution with writer +0;
- cancel is process-local and only valid before the managed-write commit point;
- event hints contain only `{jobId,eventSeq}` and are emitted after the durable
  snapshot/event commit;
- fault recovery performs readback only and never retries the writer;
- partial result is derived from durable steps/resources, not persisted as a
  second source of truth;
- wire contract is v2, while the first Codex adapter implementation correctly
  uses `adapterVersion=1` (independent version axes).

Validation already observed on the replacement work:

- Change Plan focused Rust: 24/24;
- focused V2: 28/28;
- V2 browser: 120/120;
- aggregate local gate has passed on the canonical main baseline;
- a cross-process Cargo integration-fixture race was discovered and fixed by
  PID-isolating test HOME in `4186fb92`.

**Current handoff point:** the final code/spec metadata correction was made in
`35ea4634`; re-run the final direct-session prearchive on that exact code, then
complete/archive the Trellis task, run post-archive contracts, create a
replacement PR, close old #134 as superseded, and hand the replacement to
Merge Queue.

Issue closure guidance:

- #58 is not necessarily fully closed by the first Codex adapter if the Issue
  requires proof from a second real adapter; WorkBuddy/#139 is that proof.
- #59/#60 should only close when their exact runtime/recovery acceptance is
  actually satisfied by the landed replacement chain.

### PR #136 — V2 Change Plan UI source

Remote state at handoff:

```text
state: OPEN Draft
base: codex/issue-58-60-executor-recovery
head: codex/issue-41-v2-change-plan-recovery
head SHA: 3e35859b03d4bf295a3749a376db110ef55bfa0c
issues: #41 OPEN, #56 OPEN
```

Old PR scope worth salvaging:

- strict closed TypeScript Change Plan wire contract;
- V2 Models existing-Provider switch surface;
- side-effect-free four-section preview;
- exactly one confirmation;
- authoritative job snapshots + event hints with polling fallback;
- five execution phases;
- pre-write cancellation UX semantics;
- terminal/partial/recovery/restart truth;
- native-required browser fallback;
- no fake coordinator.

Historical evidence in #136 includes a native macOS isolated-home UAT where one
plan was consumed once, five persisted phases succeeded, event sequence 1..7
was observed, DB/device/definition/live readbacks matched, and usage remained
`not_observed`.

Do not transplant #136's old backend or old singular `change-plan` ownership
blindly. Current main/replacement #134 already has the canonical plural
`change-plans` V2 port/parser and typed executor pieces. After the #134
replacement lands, perform a net-diff salvage review and only add the product UI
capability still missing from current main.

Target terminal state:

1. create a new current-main Trellis task/branch;
2. migrate only missing V2 preview/confirmation/progress/recovery UX;
3. preserve strict DTO parsing and backend-owned state;
4. no fake progress/timers, no second state machine, no proactive model request;
5. update frontend SPEC without reverting #140/#135/#134 contracts;
6. full prearchive/archive/post-archive;
7. replacement PR -> Merge Queue -> main;
8. close old #136 as superseded;
9. close #41/#56 only if all current Issue acceptance is truly met.

### PR #137 — Codex Provider create/edit/switch vertical source

Remote state at handoff:

```text
state: OPEN Draft
base: codex/ucp-integration-35-41
head: codex/issue-63-codex-provider-vertical
head SHA: 9b9e40b3d7d2e2a75ccd7b68fa74e4394d05de7f
issue: #63 OPEN
```

Historical dependencies listed by the old PR were #130/#55, #132/#35,
#134/#58-60, #136/#41/#56. In current main:

- #130 must not return;
- #135 replaces its canonical Change Plan role;
- #145 supplies the final SecretRef core;
- the #134 replacement will supply the canonical typed executor;
- the #136 replacement will supply the current V2 Apply surface.

Product capability to salvage:

- Codex Provider create;
- Codex Provider edit;
- set/switch current Provider;
- all through one immutable Change Plan preview and one confirmation;
- persist safe SecretRef metadata only;
- raw secret material process-private until apply;
- existing Provider writer invoked exactly once;
- five-phase authoritative snapshot/readback;
- stale/expiry/duplicate/failure/recovery behavior;
- no network validation during apply;
- truthful `usageEvidence=not_observed`.

Do not import from old #137:

- duplicate old SecretRef implementation now superseded by #145;
- old Change Plan schema/DAO/adapter ownership;
- any second writer;
- any old proof/epoch design;
- any spec that conflicts with #140 targeted patch/backups or #135/#134 current
  contracts.

Historical #137 evidence includes macOS native/Keychain and isolated Tauri UAT,
but it predates the final #145 activation boundary. The replacement must
re-evaluate how SecretRef becomes a real production consumer and satisfy the
current signed-app DPK/activation contract rather than merely copying old
service registration.

Target terminal state:

1. start from the main that already contains the #134 and #136 replacements;
2. integrate #145 SecretRef through a reviewed production activation boundary;
3. route create/edit/switch through the one canonical executor/writer;
4. add rollback/readback/stale/secret tests and matching-host evidence required
   by the maintained specs;
5. update backend/frontend Provider/SecretRef/Change Plan SPECs;
6. Trellis closeout and Merge Queue;
7. close old #137 as superseded;
8. close Issue #63 only after the entire first trusted Codex Provider vertical
   is actually landed and evidenced.

### PR #139 — WorkBuddy second adapter source

Remote state at handoff:

```text
state: OPEN Draft
base: codex/issue-63-codex-provider-vertical
head: codex/issue-66-workbuddy-adapter-recovery
head SHA: 61e5e0d3bd991f18f0e34b94217ee7148aeed586
issue: #66 OPEN
```

The old #139 Trellis task is already archived, but the PR itself is still
stacked on the obsolete #137 chain. Archived task status does **not** make the
old PR mergeable.

Capability worth salvaging:

- WorkBuddy as the second real typed Change Plan adapter;
- V2 WorkBuddy save/overwrite/delete through one preview + one confirmation;
- existing WorkBuddy revision protection;
- backup-first behavior;
- atomic replacement;
- real file readback;
- internally consumed destructive-overwrite token (not renderer-exposed);
- five persisted phases;
- recovery never replays writer;
- restart/usage truth;
- no Provider-domain coupling;
- no schema v21;
- no generic undo engine.

Historical #139 verification included local full gates, 116 browser tests,
focused Change Plan/WorkBuddy tests, and macOS isolated-home UAT. Windows
matching-host WorkBuddy HIL remained explicitly required before Issue #66 could
close.

Target terminal state:

1. wait until #137 replacement/Codex vertical is landed;
2. compare current WorkBuddy service/revision/backup contracts against #139;
3. implement a second adapter using the canonical typed executor, not old UCP
   internals;
4. preserve existing WorkBuddy atomic writer and backup authority;
5. perform matching-host Windows WorkBuddy HIL;
6. update WorkBuddy + Change Plan + V2 SPECs;
7. Trellis closeout and Merge Queue;
8. close old #139 as superseded;
9. close #66 when final Windows and mainline evidence exists;
10. if Issue #58 requires a second real adapter proof, close that acceptance
    only here, after WorkBuddy is landed.

## 6. Recommended execution order

Do not work on all old stacks simultaneously. Preserve one canonical base and
land each layer before beginning the next replacement:

```text
1. Finish #134 replacement typed executor
   -> merge to main
   -> sync dev/laiyongjie

2. Salvage #136 V2 Apply UI
   -> replacement PR
   -> merge to main
   -> sync dev/laiyongjie

3. Salvage #137 Codex Provider vertical
   -> activate final SecretRef correctly
   -> replacement PR
   -> merge to main
   -> sync dev/laiyongjie

4. Salvage #139 WorkBuddy second adapter
   -> Windows matching-host HIL
   -> replacement PR
   -> merge to main
   -> sync dev/laiyongjie
```

Parallel release work such as Windows Authenticode Issue #68 may proceed
independently, but it must not alter the canonical Change Plan ownership above.

## 7. SPEC discipline for every replacement

Before merge, compare final implementation against all maintained overlapping
SPECs. Do not resolve conflicts by blindly choosing `ours` or `theirs`.

High-risk SPEC areas for this chain:

- backend Change Plan executor/canonical Provider configuration;
- backend SecretRef activation boundary;
- backend WorkBuddy configuration;
- frontend V2 Models/Apply contracts;
- GitHub CI/merge governance when adding checks or workflow changes;
- supported-platform structure manifest after any monitored source change.

SPEC must describe the **final current-main behavior**, not preserve obsolete PR
wording for provenance.

## 8. Trellis lifecycle rules for the remaining chain

For each replacement:

- create a fresh task rather than reviving an obsolete stacked task unless the
  old task is truly the canonical active lifecycle;
- old archived tasks remain historical evidence;
- do not put machine-specific worktree paths into maintained task content;
- direct-session prearchive is a merge blocker;
- complete/archive the task **before** enabling auto-merge;
- post-archive contracts must pass on the real archived tree;
- PR URL/final evidence may be written into the archive only if contracts are
  re-run afterward;
- keep the cross-PR status in this handoff updated as each old PR reaches its
  terminal state.

## 9. Current Issue states

Verified at this handoff:

```text
#35  CLOSED  SecretRef core
#41  OPEN    visible/readback/recoverable apply
#56  OPEN    semantic/risk/precondition/recovery preview
#58  OPEN    typed change adapter contract
#59  OPEN    idempotent/cancellable/partial-result executor
#60  OPEN    true-target verification + crash recovery
#63  OPEN    Codex Provider create/edit/switch vertical
#66  OPEN    WorkBuddy Change Plan adapter/recovery
#68  OPEN    Windows Authenticode x64/arm64 release
#141 OPEN    consolidated historical macOS/Windows UAT findings
```

Issue closure must follow actual current-main acceptance, not merely old PR task
completion.

## 10. Immediate next action

At the moment this handoff was written, the immediate task is still the #134
replacement. The final adapter metadata correction is committed as `35ea4634`.
Run the final direct-session prearchive against that exact code/spec state,
finish/archive the task, run post-archive contracts, create the replacement PR,
close old #134 as superseded, and hand the replacement to Merge Queue. Then
continue directly to #136 using this document as the cross-PR source of truth.
