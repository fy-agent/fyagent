# Issue #55 Change Plan — Implementation plan

Status: blocked on detailed-design review and `DESIGN_FREEZE=PASS`.

This plan executes `detailed-design.md`; it does not authorize implementation
before freeze. Every shell command runs through `rtk` from
`/Users/serendipity/.codex/worktrees/issue-55-change-plan-mainline/fyagent`.
The dirty root repository and duplicate `/worktrees/ucp` checkout remain
untouched.

## Gate 0 — Immutable inputs and design freeze

- [x] UCP owner completed and pushed handoff
  `6859e9ce04970008f4cf8b3d4883b4f70316291a`.
- [x] Source implementation frozen at
  `ca552f4d918cacc734f81f7efdef70619da139b8`.
- [x] Product review passed `0 P0 / 0 P1 / 0 P2`.
- [x] Architecture review passed `0 P0 / 0 P1 / 0 P2`.
- [ ] Detailed-design review passed `0 P0 / 0 P1 / 0 P2`.
- [ ] `design-freeze.md` records hashes of every contract/review/spec input and
  `DESIGN_FREEZE=PASS`.
- [ ] Freeze commit is created and immutable SHA is read back locally.
- [ ] #35 status is recorded; only an owner-declared exact handoff is consumed.

Allowed before Gate 0 closes: static read, document patch, review, `git diff
--check`, text/stale-reference scans, JSON syntax validation, SHA-256 of design
files. Forbidden: source/test edits, tests, builds, browser/server, renderer,
native runtime.

### Post-freeze #41 design notification — non-blocking

Immediately after Gate 0 closes, send
`research/handoffs/issue-41-design-contract.md` with the exact design-freeze SHA
and label it docs-only/non-consumable. Record delivery/readback when available,
but neither send success nor #41 acknowledgement is a predicate for Phase 1 or
source/test edits. A delayed/unavailable #41 thread cannot block #55. This
notification is not considered administratively closed until readback, and it
never satisfies #41's source integration gate.

### Gate 0.1 — Trellis activation after freeze

Before Phase 1 prototype or any source/test write, replace the `_example` row in
both context files. `implement.jsonl` contains exactly the backend/frontend
indexes, `unified-change-plan.md`, `codex-provider-configuration.md`,
`deeplink-import-security.md`, `development-environment.md`,
`task-runner-contract.md`, and the four task research audits. `check.jsonl`
contains those owning specs plus frontend type/state/quality guidelines and the
ownership/dependency audits. Every row uses `{ "file", "reason" }`; no code path
or seed row remains.

Exact `implement.jsonl` paths:

```text
.trellis/spec/backend/index.md
.trellis/spec/backend/unified-change-plan.md
.trellis/spec/backend/codex-provider-configuration.md
.trellis/spec/backend/deeplink-import-security.md
.trellis/spec/backend/development-environment.md
.trellis/spec/backend/task-runner-contract.md
.trellis/spec/frontend/index.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ownership-and-handoff-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ucp-contract-gap-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/dependency-contract-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/provider-create-edit-code-map.md
```

Exact `check.jsonl` paths:

```text
.trellis/spec/backend/unified-change-plan.md
.trellis/spec/backend/codex-provider-configuration.md
.trellis/spec/backend/deeplink-import-security.md
.trellis/spec/backend/development-environment.md
.trellis/spec/backend/task-runner-contract.md
.trellis/spec/frontend/index.md
.trellis/spec/frontend/component-guidelines.md
.trellis/spec/frontend/hook-guidelines.md
.trellis/spec/frontend/state-management.md
.trellis/spec/frontend/type-safety.md
.trellis/spec/frontend/quality-guidelines.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ownership-and-handoff-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/ucp-contract-gap-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/dependency-contract-audit.md
.trellis/tasks/08-14-issue-55-change-plan-mainline/research/provider-create-edit-code-map.md
```

Then run exactly:

```text
rtk proxy python3 ./.trellis/scripts/task.py validate 08-14-issue-55-change-plan-mainline
rtk proxy python3 ./.trellis/scripts/task.py start 08-14-issue-55-change-plan-mainline
rtk proxy python3 ./.trellis/scripts/task.py current --source
rtk jq -e '.status == "in_progress"' .trellis/tasks/08-14-issue-55-change-plan-mainline/task.json
```

The current-source output and task status are recorded in the task progress log.
Before a lane's first edit, that lane loads `trellis-before-dev` and its exact
layer specs: backend lanes load backend index plus UCP/Codex/deep-link as
applicable; frontend lanes load frontend index plus component/hook/state/type/
quality specs; DB/evidence lanes also load development-environment and task-
runner contracts. A dispatched lane without this readback may not claim or edit
files.

## Phase 1 — Visual reference, prototype, and usability review

Starts only after freeze.

- [ ] Load the image generation skill and produce one visual reference aligned
  to the current Prompt/Memory V2 full-screen interaction pattern.
- [ ] Store prompt, generated asset, timestamp, dimensions, and SHA-256 under
  `research/prototype/manifest.json`; label evidence `prototype`.
- [ ] Build a non-production high-fidelity prototype covering clean, warning,
  expired/drift, unsupported/secret dependency, running/recovery, and candidate
  safety.
- [ ] Run usability review for hierarchy, exact one-confirmation path,
  resources/backup/credential/privacy answers, focus/keyboard/screen-reader
  behavior, and state-specific actions.
- [ ] Close all usability P0/P1/P2 before product-source UI implementation.

Generated images and prototype screenshots are not runtime or native evidence.

## Phase 2 — Contract and DAO module

Owner: CP-core + CP-store. CP-store edits only
`database/dao/change_plan.rs`, publishes its release SHA, and does not touch
schema/backup/mod; DB-runtime claims those shared files only in Phase 3.

- [ ] Add strict v2 contract/projection/capability types and retain immutable v1
  compatibility decoder.
- [ ] Add canonical value implementation and four fixed vectors.
- [ ] Remove full Provider/live-auth/value-derived v1 digest authorization.
- [ ] Add schema-dispatched DAO, scope discovery, lifecycle CAS, atomic
  admission, event CAS, retention/purge.

Focused closure:

```text
rtk git diff --check
rtk mise run rust:test -- change_plan_contract_v2
rtk mise run rust:test -- change_plan_canonical_v2
rtk mise run rust:test -- change_plan_preview_side_effects
rtk mise run test:unit -- tests/lib/change-plan.test.ts tests/integration/change-plan-cross-layer.test.ts
```

Commit only after every process reaches a terminal zero exit.

## Phase 3 — Compatibility/runtime and artifact authority

Owner: DB-runtime/compat owns `database/mod.rs` plus the complete connection/copy
inventory in `detailed-design.md`; Artifact owns its sidecar files; main owns
only later `lib.rs` wiring after those commits.

- [ ] From CP-store's immutable release SHA, add nullable v2 columns and
  `change_coordination` idempotently while retaining schema 16.
- [ ] Filter ledger/coordination from hooks, sync, export, diagnostics, and
  application backups; add sanitized restore/live-authority guards.
- [ ] Add stable DB compatibility lock/marker/header inspector and durable atomic
  writer.
- [ ] Add closeable `DatabaseRuntime` and maintenance drain/reopen path.
- [ ] Replace every baseline main-DB holder/caller/copy participant under the
  closed `DbParticipant`/`DbMaintenanceReason` contract; classify external
  Codex/Hermes/OpenCode SQLite separately.
- [ ] Remove every production explicit/implicit borrowed `Database` facade
  boundary, including Claude Desktop config, Codex-history main DB, Provider/
  proxy, Skill/session/sync and all 19 `impl Database` blocks; keep exact
  syntax-aware test-range classifications until tests use the runtime harness.
- [ ] Make `legacyDatabasePathUses` equality cover every production/test
  `Database::` associated call, import/re-export, alias and trait form; move
  backup/bootstrap/external-Codex helpers to their frozen dispositions and make
  all expected legacy syntax sets empty.
- [ ] Add connection-free `DbActivityLease` across complete async/local
  operations, a closed background stop/join registry, generation-bound
  publication permits, and linear Result transition tokens with pre-close
  rollback versus post-close failed-closed states.
- [ ] Hold a linear remote-effect permit/activity from upload snapshot through
  every WebDAV/S3 object PUT, authoritative manifest readback, ack, and cleanup/
  quarantine; no old-generation fixed object or manifest may publish after
  maintenance admission closes.
- [ ] Persist `DbRemoteEffectReceiptV1` before the first PUT. Every linear token
  transition returns its token on error; Drop/panic marks recovery-required
  synchronously. Startup and runtime block maintenance/new upload until remote
  terminal/quarantine readback closes every durable attempt.
- [ ] Make the baseline/current inventory equality task pass; no subset-only
  or unclassified production match is accepted.
- [ ] Reorder startup so compatibility/replacement recovery precedes SQLite open
  and sync workers start last.
- [ ] Add sidecar schema v1, stable global integrity lock, strict open/version.
- [ ] Add source/candidate contracts, attempts, effect-start fence, scanner,
  sticky pair integrity, write-once completion ack, recovery and joint GC.
- [ ] Add safe IPC/events with backend-derived allowed actions.
- [ ] Add old-binary guard and `MIGRATION_GUARD_BASELINE_SHA` fixture; do not
  enable #35's later DB migration without its handoff.

Focused closure:

```text
rtk git diff --check
rtk mise run change-plan:db-inventory -- --baseline-sha ca552f4d918cacc734f81f7efdef70619da139b8 --manifest scripts/change-plan/database-runtime-inventory.json
rtk mise run rust:test -- change_plan_store_v2
rtk mise run rust:test -- change_plan_backup_sync
rtk mise run rust:test -- db_compatibility_v1
rtk mise run rust:test -- db_activity_v1
rtk mise run rust:test -- credential_artifact_v1
rtk mise run rust:test -- credential_artifact_concurrency
```

Fault cases include each worker stop/join timeout, activity drop/panic/drain,
manual/auto WebDAV/S3 after-snapshot, each-object-PUT, pre-manifest and
post-manifest/pre-ack pause, every PUT/readback error, cancellation/panic,
response loss, ack/cleanup fsync failure and restart, plus Skill/session/
Codex-history, every transition failure, old-generation hook/tray/cache/event
rejection, split identity, lock busy, store failure, every replacement boundary,
exact prior/target/ambiguous/authority-unavailable, and no effect replay.

## Phase 4 — Provider preparation, worker, and cutover

Owner: Provider-adapter + CP-worker; integration main owns shared entrypoints.

- [ ] Extract pure create/edit/switch preparation and live projection planning.
- [ ] Add coordinator/provider epoch and non-repairing readers.
- [ ] Add protected gate, private one-use `EffectPermit`, exact private commit,
  per-resource readback and sync suppression/quarantine.
- [ ] Add worker lease/claim, immediate planned admission response,
  cancellation/effect CAS, post-admission drift, orphan readback-only reconcile,
  recovery recheck.
- [ ] Land strict TS decoder/query/launcher primitives before any public cutover
  flag; they remain unreachable until an atomic operation commit.
- [ ] Cut over switch atomically: renderer hook/API route, native commands,
  service gate/private commit and tests in one commit.
- [ ] Cut over create/edit atomically: Add/Edit form routing, draft-only endpoint
  plumbing, API mutation removal, native commands/service gate/private commit
  and tests in one commit.
- [ ] Cut over tray/profile/deep-link and Universal in separately atomic commits,
  each pairing navigation/API with native/service guards and tests.
- [ ] Guard old UCP, config/proxy and every remaining bypass before effects.
- [ ] Keep non-Codex and separately named non-create/edit/switch behavior intact
  while joining managed writers to the epoch.

Focused closure:

```text
rtk git diff --check
rtk mise run rust:test -- change_plan_worker_v2
rtk mise run rust:test -- change_plan_provider_cutover
rtk mise run test:unit -- tests/hooks/useProviderActions.test.tsx tests/integration/change-plan-entry-cutover.test.tsx
```

After Provider guard plus ledger/worker/strict decoder/fixture/registration are
present at one statically reviewed commit (commit 6 in `detailed-design.md`),
create `research/handoffs/issue-41-consumable-contract.md`, send its exact local
SHA to #41, and verify
`ackSha/consumerBranch/consumerBaseSha/compatibilityStatus/seamFindings`. #41
integration is allowed only when `ackSha` exactly equals the handoff SHA,
consumer branch/base match the receipt, `compatibilityStatus=pass`, producer and
consumer reviews both report `0 P0 / 0 P1 / 0 P2` for the seam, every required
path hash matches, every named compatibility command exits zero, and
`seamFindings=[]`. Any other state is `blocked`; #55 may continue, but #41 may
not integrate. This is the first source handoff; Gate 0's design receipt is
never relabelled.

## Phase 5 — #35 and Universal dependency integration

Owner: main integration thread only for shared seams.

- [ ] Verify #35 owner-declared exact SHA/ref and record compatibility matrix.
- [ ] Integrate its native-only capability/confirmation/resolve/zeroization
  contract without copying material or adding a second store.
- [ ] Implement reference-native Universal storage and the single
  revision/epoch-bound mutation command only after the safe predecessor guard.
- [ ] Cover `None|Clear|Preserve|Replace`, `NeedsLocalRebind`, binding-key
  vectors, safe import/restore, and quarantined legacy artifacts.
- [ ] If handoff is still absent, keep all secret-bearing production paths
  typed-disabled and mark native acceptance for those cases blocked; do not
  weaken the contract.

Focused closure after handoff integration:

```text
rtk git diff --check
<exact #35 owner-provided focused commands at the verified SHA>
rtk mise run rust:test -- universal_mutation_v1
rtk mise run rust:test -- change_plan_secret_ref
```

## Phase 6 — Frontend platform and product UI

Owner: FE-product completes visual/a11y states; FE-platform/FE-cutover authority
routing already landed atomically with Phase 4; main owns App/shared indexes.

- [ ] Complete (and reverify) strict v1/v2 decoding, discovery, expiry,
  abandon/cancel/recheck, snapshot-wins events and unified launch paths already
  introduced by the atomic cutover commits.
- [ ] Full-screen Prompt/Memory V2-aligned Plan surface and root candidate safety
  banner/panel.
- [ ] Complete state/action projection, four locale parity, focus/keyboard/
  screen-reader semantics, no private sentinel.
- [ ] Replace raw Universal UI/API with safe view/one mutation and dependency
  states.

Focused closure:

```text
rtk git diff --check
rtk mise run test:unit -- tests/lib/change-plan.test.ts tests/components/change-plan tests/hooks/useChangePlanLauncher.test.tsx tests/hooks/useCredentialArtifacts.test.tsx tests/integration/change-plan-entry-cutover.test.tsx tests/integration/App.test.tsx
rtk mise run test:i18n
rtk mise run typecheck
rtk mise run build:renderer
```

## Phase 7 — Module integration and failure-path closure

- [ ] Integrate one immutable module commit at a time in dependency order:
  contract → store/runtime → Provider/worker → artifact → #35/Universal → UI.
- [ ] After each integration, rerun both producer and consumer focused gates.
- [ ] Run operation × mode × failure matrix for create-only, create-select, edit
  non-current/current, and switch custom.
- [ ] Prove preview side-effect counters, pre-admission rejection, post-admission
  no-effect terminal, cancellation, response loss, crash/restart, partial write,
  unreadable readback, retention and sanitized transfer.
- [ ] Verify #41 compatibility against the latest immutable #55 source SHA and
  provide any additive extension seam without changing one-confirmation or
  digest meaning.
- [ ] Add the debug-only pre-Tauri evidence path authority and headless native/
  failure dispatch, then the host-native same-SHA executable receipt.
- [ ] Add exact mise/task-runner/generated docs, Playwright 1.61.1, reviewed
  macOS Chromium lock preview/apply, repo-read-only prepare, and the
  repo-scoped `ActiveEvidenceSessionV1` lock/pointer/record/CAS protocol.
- [ ] Commit `research/evidence-inputs.v1.json` with design/UCP/#35 closed state
  and receipt hashes before source freeze; caller session/source/evidence env
  is never authority.
- [ ] Make renderer the only session creator and browser/native/failure strict
  joiners. From `native`, default failure captures the fourth mode and prepares
  a zero-repository-write preview; the separate `failure --apply` accepts only
  that `publish_prepared` authority and alone publishes.
- [ ] Model claim-owned publish-preparing, receipt-bound publish-prepared,
  destination-renamed/fsynced, cleanup, final-snapshot, terminal-receipt and
  published states. Use separate immutable
  active-pointer/final-snapshot/terminal-receipt schemas and a one-way hash
  graph; final record references the receipt, then active pointer is unlinked.
  Recovery admits only the exact bound untracked destination digest.
- [ ] Put session ID, binding digest and deterministic terminal-receipt locator
  in the destination manifest before root hashing; fix the final snapshot path
  and fail closed on zero/duplicate/foreign/mismatched post-unlink receipts.
- [ ] Implement the tracked deny-unknown prepared-publication-receipt schema,
  claim-qualified prepared and receipt paths, canonical no-replace write,
  file/parent fsync and readback. Cover partial-only, prepared-without-receipt,
  temp/torn/corrupt receipt, valid-before-CAS and stale/mismatched preimage with
  the exact resume, rebuild, quarantine/abort, or zero-write result.
- [ ] Contract-test direct apply from `native` as out-of-order/zero-write,
  preview-then-apply reachability, idempotent prepared preview, and receipt/tree/
  manifest/file-list/root mismatch before `publishing` leaving the
  `publish_prepared` record byte-identical.
- [ ] Keep shared owners serial: Evidence owns config/app-store/task runtime;
  Provider owns Codex/settings accessors; CP-core owns the module declaration;
  Integration owns main/lib/package/lock wiring.

Evidence-runtime focused closure before source freeze:

```text
rtk mise run rust:test -- change_plan_evidence_path_authority
rtk mise run rust:test -- change_plan_evidence_store_isolation
rtk mise run test:unit -- tests/miseTaskContract.test.ts tests/taskDocs.test.ts tests/localBuildBoundary.test.ts tests/changePlanEvidenceContract.test.ts
rtk mise run change-plan:chromium:lock
rtk mise run change-plan:chromium:lock --apply
rtk mise run tasks:docs:generate --apply
rtk mise run tasks:docs:check
rtk mise run tasks:validate
```

No formal screenshot/native evidence is generated until product source is
frozen.

## Phase 8 — Source freeze and fresh exact-runtime evidence

- [ ] Review/stage only Issue #55 files and create the source-freeze commit.
- [ ] Record `BASE_SHA`, `DESIGN_FREEZE_SHA`, dependency SHAs, the clean HEAD
  captured as `SOURCE_HEAD`, its tree, name-status and diff stat. Do not accept
  caller `SOURCE_FREEZE_SHA`; renderer derives and binds current HEAD.
- [ ] Run from a clean worktree with Node 24.19.0 and locked mise:

```text
rtk mise run env:check
rtk mise run change-plan:db-inventory -- --baseline-sha ca552f4d918cacc734f81f7efdef70619da139b8 --manifest scripts/change-plan/database-runtime-inventory.json
rtk mise run typecheck
rtk mise run format:check
rtk mise run test:unit
rtk mise run test:i18n
rtk mise run test:desktop:mock
rtk mise run test:desktop:visual:preflight
rtk mise run build:renderer
rtk mise run rust:fmt:check
rtk mise run rust:check
rtk mise run rust:clippy
rtk mise run rust:test
rtk mise run tasks:docs:check
rtk mise run tasks:validate
rtk mise run check:contracts
rtk mise run check
rtk mise run change-plan:chromium:prepare
rtk mise run build:debug
rtk mise run change-plan:evidence:renderer
rtk mise run change-plan:evidence:browser
rtk mise run change-plan:evidence:native
rtk mise run change-plan:evidence:failure
rtk mise run change-plan:evidence:failure --apply
rtk proxy python3 ./.trellis/scripts/task.py validate 08-14-issue-55-change-plan-mainline
rtk proxy python3 ./.trellis/scripts/task.py current --source
rtk git diff --check
rtk git status --short --branch
rtk git diff --name-status 4b4e17540ad8ddd564bb7ef7c5ca2a31b7c36287..HEAD
```

- [ ] Poll every yielded session to its terminal exit; never infer success from a
  live session ID.
- [ ] Before each evidence mode, require exact HEAD/tree/binding and a fully
  clean porcelain-v2 worktree. Renderer creates the sole repo-scoped session;
  browser/native/failure join it via locked CAS. The first three never write
  the repo or rely on caller session/source env.
- [ ] For `failure --apply` recovery after destination rename, allow only the
  exact stored porcelain path set/root digest; reject all unrelated dirt and
  retain active pointer authority through cleanup/snapshot/receipt/final-record
  fsync. After unlink, recover only from the deterministic terminal receipt.
- [ ] `change-plan:evidence:renderer` captures the frozen 1440x960 four-locale
  state matrix from deterministic fixtures and labels screenshots
  `runtime_screenshot`.
- [ ] `change-plan:evidence:browser` records keyboard/focus/reload/event/one-
  confirmation interactions separately from static renderer captures.
- [ ] `change-plan:evidence:native` launches the hash-verified exact
  `build:debug` binary whose ignored target receipt matches current HEAD/tree,
  host/target/argv and executable size/hash, through its debug-only runner with task-owned
  FyAgent/Codex/store roots, exercises real local
  credential-free create/edit/switch apply/readback/restart, proves user-state
  sentinels unchanged, and labels it `native_runtime`.
- [ ] `change-plan:evidence:failure` runs the frozen injected and real-OS failure
  matrix with counters/readback and the same receipt digest as native. The
  first, default invocation captures/prepares and prints the publish diff with
  zero repository write; a byte-identical retry is a no-op. The immediately
  following `failure --apply` revalidates its prepared receipt and bytes before
  the first `publishing` CAS and atomically publishes all four modes. It rejects
  `native` or any mismatch without a write. Label it `failure_path`, not native
  success.
- [ ] Record any user-performed acceptance separately as `UAT`; absence of UAT
  does not become an automated pass.
- [ ] Any source edit invalidates affected commands/captures and requires a new
  source-freeze SHA and fresh evidence.

## Phase 9 — Final review, publication, and readback

- [ ] Independent final review on the exact source/evidence SHA maps every Issue
  #55 and related contract AC to file/test/evidence and returns no unresolved
  P0/P1/P2.
- [ ] Run fresh `rtk git diff --check`, stale-reference scans, task validation,
  status/branch/upstream/diff and clean-worktree checks.
- [ ] Make only coherent small commits; push
  `codex/issue-55-change-plan-mainline` without force.
- [ ] Read back remote SHA and compare byte-for-byte local commit identity.
- [ ] Create the PR only after final review; read back title/body/base/head/check
  state. Do not merge or deploy.
- [ ] Update GitHub Issue #55 with exact SHA/PR/evidence boundaries and dependency
  limitations; read back the comment.
- [ ] Send #41 the final exact source SHA/paths and verify its handoff readback.
- [ ] Mark the Trellis card done/archive only when deliverables/evidence/remote
  readback are present. Then mark the goal complete.

## Stop conditions

Stop only for a genuine permission/user decision, safety boundary, repeated
unresolvable verification failure, or a dependency required for a claim that
cannot be honestly narrowed. #35 absence does not stop credential-free core,
fixtures, typed-disabled UI, or #41 contract handoff; it does block claiming
secret-bearing production/native acceptance. No blocker authorizes a second
credential system, duplicate writer, main merge, or deployment.
