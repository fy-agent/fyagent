# Issue #35 execution plan

## Closure checklist

- [x] read-only root/worktree/remote/task-card writer audit; dedicated branch/worktree.
- [x] latest Issue #35 body/comments authority digest and current source audit.
- [x] PRD/state flows, direct OS backend research, device-local journal, Codex call graph and draft exact contract.
- [ ] product review P0/P1/P2=0 on one immutable design commit.
- [ ] architecture review P0/P1/P2=0 on that same commit.
- [ ] detailed design review P0/P1/P2=0 on that same commit.
- [ ] `DESIGN_FREEZE` receipt + immutable contract SHA.
- [ ] #55/#41 handoff sent/read back; their compatibility feedback adjudicated without waiting for implementation.
- [ ] generated visual reference + V2 high-fidelity prototype + usability review.
- [ ] secret module, platform/capture, Codex integration, V2 and scanner focused module tests pass.
- [ ] integrated migrate/apply/proxy/usage/rotate/delete/import/failure flow pass.
- [ ] source-freeze SHA and fresh exact-runtime gates.
- [ ] macOS + Windows x64 native/UAT; Windows required failure paths.
- [ ] final independent review, small commits, dedicated-branch push/PR/GitHub readback; no main merge/deploy.

## Phase 0 — design convergence

Allowed: static repository/doc/source reading and task-directory design edits.

Forbidden: dependency resolution, test, build, browser, renderer, server, native runtime, screenshot and product code.

1. unify `prd.md`, `secret-contract-v1.md`, `device-local-secret-store.md`, call graph, native evidence and both design overviews; V6 and V7 `REQUEST_CHANGES` receipts remain immutable inputs, and before any commit a later three-lane working-tree audit must independently return P0/P1/P2=0 on one stable hash set;
2. remove all v17/SQLite-secret/store-crate/by-ref resolve/repository-global/Agent-supported stale wording;
3. create a design-only candidate commit `D1`; reviewers never approve an uncommitted working tree;
4. product, architecture and detailed reviewers independently read exact `D1` and write new `*-rereview.md` records that name the SHA; initial `*-review.md` files remain immutable superseded snapshots;
5. any P0/P1/P2 correction creates `D2` (or later), invalidates all three prior rereviews, and all reviewers re-read the new exact candidate;
6. when all three latest rereviews name the same candidate `D` and show P0=0/P1=0/P2=0, that candidate becomes design authority;
7. create a separate receipt commit `R` whose `research/design-freeze-receipt.md` names `D`, exact file list/digests and latest review files; `reviews/index.md` marks the initial snapshots superseded and identifies the authoritative rereviews;
8. send `D` + receipt path/current receipt SHA to #55/#41, then read back each thread message.

#35 design freeze does not wait for #55/#41 compatible implementation SHAs. Their present baselines are recorded as incompatibility evidence; compatible successors become later integration/source-freeze gates.

## Phase 1 — visual reference and prototype

Prerequisite: frozen `D` and receipt.

1. read `~/.codex/DESIGN.md` and image-generation Skill;
2. inventory V2 Prompt/Memory tokens/components and freeze visual brief/archetype/four viewports;
3. generate one secret-free bitmap reference, save metadata and label only `visual_reference`;
4. implement browser-only credentials DTO/decoder/fixture/panel against frozen contract;
5. run focused V2 tests/browser interaction after implementation is allowed;
6. independent usability review covers hierarchy, state/action clarity, native capture expectation, staged candidate, hardware confirmation and destructive impact;
7. close usability P0/P1/P2 before native adapter composition.

Generated imagery never closes renderer/native/runtime evidence.

## Phase 2 — production modules with exclusive owners

Prerequisite: frozen authority SHA distributed to every worker. Canonical changed-path owner labels remain exactly `#35 module | #55 | #41 | main integration`. A–D below are temporary internal subworkers under canonical owner `#35 module`, not new manifest owner literals; each receives a disjoint exact path set from `detailed-design-overview.md`/call graph.

### A. secret contract/device store

Canonical owner: `#35 module`; subworker A. Implement strict types/material/error, local store/atomic/journal/reconcile, candidate/lifecycle/service and commands. Its core trait/type/backend module must compile without importing unpublished #55/#41/main-integration callback types. It does not edit platform/capture, V2 or scanner paths. Focused Rust core tests only.

### B. platform backend/native capture

Canonical owner: `#35 module`; subworker B. Starts after A's backend/material interfaces stabilize. It owns only `secret/platform/**` and `secret/capture/**`; no overlap with A. Implement direct macOS/Windows store and capture. Focused unit tests and current-host gated smoke only; formal native evidence waits for source freeze.

### C. V2 credentials

Canonical owner: `#35 module`; subworker C. Implement the exact V2 data/decoder/browser/approved Tauri adapter/panel/spec paths. No shared Page/router/platform index edit.

### D. scanner

Canonical owner: `#35 module`; subworker D. Implement the exact scanner script/test/baseline paths, four levels and positive/negative self-tests.

### E. main Codex integration

Canonical owner: `main integration`; executor: `root/MainIntegrationOwner`, serially after A/B public APIs stabilize. It alone edits the exact shared existing paths frozen in the call-graph owner map: Cargo/lock; AppState/startup/static registration; Provider/public projection/delete; legacy/live/import/backup/sync and `sync_protocol` cutover; Codex history/template migration; proxy/usage/model-fetch/coding-plan/terminal/deeplink/universal; Codex environment detection/removal, common-config JSON/SQLite/localStorage/live merge, shared public Provider types/schema/query/list/sort/MSW, request-override/raw transport rejection, stream/proxy stable diagnostics, and Codex MCP Level-3 inventory; Add/Edit dialogs, Provider list/card, Codex feature hook/forms/sections/editor/templates, usage API and deep-link preview/dialog; every exact existing fixture/test named by §9.4; V2 composition and CI. Issue #35 authority/baseline/scanner files retain their explicit `#35 module` owner and are never absorbed by a generic shared-files label.

Composition is deliberately two-stage: first #35 core APIs and focused tests become stable with no downstream source dependency; then #55/#41/main owners publish their immutable adapter types. Only after exact-SHA compatibility readback may `main integration` add the sole adapter/composition module and static Tauri registration. No #35 worker creates another lane's missing module/type, and no full-crate Rust PASS is claimed before this composition. Registration proof covers exactly 15 #35 commands plus the separately owned `resume_staged_import_cutover` handler; that handler is not a sixteenth `SecretCommandName`.

The integration manifest must include every current path discovered by the V6/V7 source scans, including `AddProviderDialog.tsx`, `EditProviderDialog.tsx`, `ProviderList.tsx`, `ProviderCard.tsx`, `useCodexProviderFeatures.ts`, `CodexFormFields.tsx`, `CodexConfigSections.tsx`, `CodexConfigEditor.tsx`, shared `src/types.ts`/provider schema/query/sort/MSW state, `src/lib/api/usage.ts`, `src/config/codexTemplates.ts`, Codex env checker/manager/command/API/type/banner, common-config app/DB/command/API/hook/modal, request override/Provider/proxy/hyper transport, stream-check/health/error/query, Codex MCP env/header DB/live chain, `deepLinkConfigPreview.ts`, `DeepLinkImportDialog.tsx`, `src-tauri/src/services/sync_protocol.rs`, `src-tauri/src/codex_history_migration.rs` and their callsites. Its exact fixture set comes only from call graph §9.4 and includes the V7 env/common-config/public-list/sort/override/diagnostic/MCP additions. The generator enumerates every exact path, replaces checked-in value/backfill expectations with runtime canaries and token-free/early-reject assertions, and keeps Codex MCP material as named Level-3 debt rather than Provider-primary PASS; no test-only waiver is permitted.

#55/#41 remain external owners of their own files. Do not edit their worktrees. Integrate only immutable compatible successors, with exact blob/readback and no wholesale branch merge.

If ownership changes, stop the affected writer, update the plan, and reassign exactly one owner before edits.

## Phase 3 — focused module verification

Commands below are authorized only after design freeze and after the matching implementation exists. Exact filters are committed task/test identities, not ad-hoc broad runs.

```bash
rtk mise run env:check --json
rtk mise run system:check --json

rtk rustup run 1.85.0 cargo check --workspace --all-targets --features fyagent/test-hooks --locked --manifest-path src-tauri/Cargo.toml

rtk mise run rust:test -- secret_
rtk mise run test:unit -- tests/scripts/secret-surface-scan.test.ts
rtk mise run test:v2
rtk mise run lint:v2
rtk mise run typecheck:v2
```

Module order:

1. strict types/material/envelopes, including durable `DeviceInstanceId` versus process-local store instance and the exact staged-resume result decoder;
2. device store/permissions/atomic/eight operation journals/four recovery-CAS arms/crash phases;
3. backend/capture seams, stateful broker-owned capture/capability/pending registries, exact dual-identity instance handle and scope-bound revocation receipt;
4. capture-intent registry; candidate/activation/prepare/consume/lifecycle; private capability claim/discard; explicit-discard/expiry independent delete+Validate-missing slots, three-field durable checkpoint, pending disposition and fresh-readiness actions;
5. legacy source matrix plus non-forgeable no-value coverage receipt, including exact 11-domain inventory-revision/completeness proof, sibling visibility, unique main-integration mint bridge, env/common-config, Provider public chain and legacy-blocked Provider-delete typed capture flow;
6. proxy/usage/model-fetch/primary-coding-plan/request-override/stream diagnostic/import/restore/sync/sync-protocol/staged admission→prepare and operationId+five-phase revision-digest resume integrations/fixtures;
7. V2 decoder/panel/browser fixture;
8. scanner self-tests.

A module fix invalidates that module's evidence; rerun it before main integration. No formal screenshot/native evidence while source is still moving.

## Phase 4 — main-thread integration

1. wire the sole startup sequence: opened store → no-backup DB preflight → same AppState/SecretService journal + full current/supplemental source coverage reconcile → `app.manage`/static registration of 15 #35 commands + the separate staged-resume handler → Clean sanitized backup → publish gate → workers; Blocked starts no backup/worker/consumer;
2. verify dependency/lock/MSRV/license/advisory facts, including the lock-compatible direct `security-framework-sys`/Core Foundation create-only path and pinned Rust 1.85.0 on macOS/Windows; host 1.97 is not substitute evidence;
3. reconcile #55 compatible structural plan/digest and #41 compatible coordinator/backup/readback SHAs;
4. audit changed paths against one-owner map and conflict budget;
5. rerun all focused modules from integrated tree;
6. run cross-layer sequence:

```text
legacy discovery -> candidate -> #55 activation plan
-> #41 activation prepare/confirm/lease/baseline -> #35 compare/CAS/scrub -> release
-> bound-owner readiness -> separate #55 apply plan
-> #41 apply prepare/confirm/new lease/backup -> #35 one-shot write/readback
-> proxy -> usage/balance/model-fetch/primary-coding-plan
-> rotate -> independently authorized old-delete/missing-readback cleanup -> lock/delete/explicit scope-bound revoke observation
-> provider-delete legacy block OR no-legacy detach -> optional separate secret delete
-> SQL import -> restore -> WebDAV/S3 temp token/projection -> #55 admission
-> authority match -> #35 prepare/confirm -> cutover -> revision-digest crash resume with fresh identity/admission
```

7. run failure matrix: cancel, missing, policy/backend lock, denied, unavailable, write/read/verify, cross-store or wrong registered backend handle, ordinary-read/probe revocation persistence rejection and revoke-receipt transplant, DB/provider scrub, dependency drift, private capability claim/discard replay/expiry, exact fresh-action routing with no generic retry, capture-intent stale/replay, explicit-discard/expiry delete-checkpoint/missing/restart, each delete→durable `{disposition,completedAt,CAS}` checkpoint→missing-readback failure, activation crash timestamp reconstruction, all eight journal/four recovery crash phases, Provider-delete legacy typed-flow block, hardware no-projection, staged admission/prepare-discard/old-admission/fresh-identity plus all five operationId-bound revision-digest resume phases, Codex env/common-config/public Provider/request-override/diagnostic canary rejection, MCP Level-3 no-regression, renderer/deeplink-before-preview rejection, history-backup gate and historical artifact scan/report (v1 performs no historical rewrite/delete).

Failure returns to the sole owning module; rerun module then integration.

## Phase 5 — source freeze and fresh local gates

Create source-freeze commit `F`; `FREEZE_SHA=$(git rev-parse F)`. Any source, harness, fixture, authority or task-contract change creates a new freeze SHA and invalidates every later artifact. After `F`, evidence-only `E` may add only sanitized files under `.trellis/tasks/08-14-issue-35-secret-backend/evidence/**` and `research/evidence-index.json`. After reviewing `F+E`, review-only `V` may add `reviews/final-review.md`. A later governance-only `G` may change `task.json` status/evidence-pointer/PR fields, append status rows to `implement.md`/`implement.jsonl`, update `reviews/index.md` pointers, and add `research/github-readback.md`; schema/diff verification rejects any contract, command or acceptance-rule edit in these files. These allowlisted additions do not change `FREEZE_SHA`; evidence verifier/readback reruns as applicable. Any other path, or any edit to source/task command, design authority, harness or fixture, creates a new freeze.

Required existing and newly registered tasks:

```bash
rtk mise run env:check --json
rtk mise run system:check --json
rtk mise run tasks:validate
rtk mise run tasks:docs:check

rtk mise run secret:scan:contract
rtk mise run secret:scan:inventory

rtk mise run format:check
rtk mise run lint:v2
rtk mise run typecheck:v2
rtk mise run rust:fmt:check
rtk mise run rust:check
rtk mise run rust:clippy
rtk rustup run 1.85.0 cargo check --workspace --all-targets --features fyagent/test-hooks --locked --manifest-path src-tauri/Cargo.toml

rtk mise run test:unit
rtk mise run test:v2
rtk mise run rust:test
rtk mise run build:renderer
rtk mise run test:v2:browser
rtk mise run check
```

Exact source/ownership checks:

```bash
rtk git rev-parse HEAD
rtk git merge-base afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab HEAD
rtk git diff --name-status afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab...HEAD
rtk git diff --check afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab...HEAD
```

`source-freeze-manifest.json` records base/freeze/dependency SHAs, sorted paths, owner and blob SHA. It also records the exact coverage schema/revision, fixed 11-domain set and omitted-domain/stale/empty-without-proof negative results, while containing no source value/path/locator/digest. The gate manifest records the exact 1.85.0 toolchain and proves the 15+1 handler registration separately. Local gates must prove ordinary tests removed/deny native-secret env and never run ignored real store cases.

## Phase 6 — pre-evidence push, CI and native evidence

Windows cannot fetch an unpushed SHA. The required order from `research/native-evidence-plan.md` is:

1. push only exact `FREEZE_SHA` to `refs/heads/codex/issue-35-secret-backend` with explicit refspec; no force/main/merge/deploy;
2. `git ls-remote` readback equals `FREEZE_SHA`;
3. `workflow_dispatch` `ci.yml` on that ref; choose the unique new run and require `headSha == FREEZE_SHA` plus required success;
4. manual macOS/Windows hosts detached-checkout exact clean SHA;
5. run the pinned Rust 1.85.0 locked all-target check on both native hosts, then native CRUD/failure/UAT/artifact scan/evidence verify;
6. any source fix restarts from Phase 5 with a new SHA.

Canonical tasks to register before `F`:

```bash
rtk mise run secret:native:macos:crud
rtk mise run secret:native:macos:uat
rtk mise run secret:native:windows:crud
rtk mise run secret:native:windows:failure
rtk mise run secret:native:windows:uat
rtk mise run secret:scan:codex -- <runtime-artifact-manifest>
rtk mise run secret:artifact:scan
rtk mise run secret:evidence:verify
```

`secret:scan:codex` is the low-level scanner and requires an explicit runtime artifact/allowed-sink manifest. `secret:artifact:scan` is the evidence-host guard/enumerator that invokes it and emits the manifest item; neither aliases the other silently.

Mandatory evidence:

- macOS real non-sync Keychain CRUD/missing with `AccessibleWhenUnlockedThisDeviceOnly` assertion = `native_runtime`; NSAlert/NSSecureTextField accept/cancel = `uat`.
- Windows x64 real CredMan CRUD/replace/delete/missing with LOCAL_MACHINE = `native_runtime`; CredUI accept/cancel = `uat`.
- Windows failure paths form a fixed all-pass set with separate `result=pass` items: real missing; injected locked; injected denied; injected unavailable; injected verify failure; injected post-switch old-delete failure; and real interactive capture cancel as both its own failure item and a separate UAT item. The manifest also records distinct failure count `>=3`, but that count never permits skipping a fixed case.
- `failure_path` items name `real_os` or `fault_injection`; CI/ARM64 compile/unit never substitutes.
- every real entry has cleanup readback missing; every manifest has exact SHA/host/task/times/exit/assertions/artifact scan and one evidence class per item.

Without Windows x64 native runtime, required failures and interactive UAT, task status remains non-DONE.

## Phase 7 — final review and delivery

1. copy only sanitized manifests plus `research/evidence-index.json` into evidence-only commit `E`; source remains `FREEZE_SHA`, and `git diff --name-only FREEZE_SHA..E` must be a subset of the two evidence-only paths/patterns above.
2. independent final reviewer audits `F+E`: source freeze, dependency SHAs, changed-path ownership, test results, native manifests, failure origins, cleanup and evidence labels; its sole file lands in review-only `V`.
3. any source fix invalidates evidence; governance-only correction reruns evidence verifier/readback as appropriate.
4. only after final P0/P1/P2=0 push the reviewed dedicated-branch tip, create the PR, and immediately read back PR/Issue state; no main merge/deploy.
5. create governance-only `G` with the verified PR/readback receipt plus Trellis status/deliverables/evidence, pass the narrow-field/append-only verifier, push `G`, then perform final remote-ref + PR/Issue readback. No further repo write follows that final readback.
6. final report separately lists `source_report`, `code_audit`, `runtime_screenshot`, `native_runtime`, `failure_path`, `uat`, and explicitly says `repository_runtime_global=NOT_CLAIMED`.

## Commit order

1. design authority `D`.
2. freeze receipt/handoff `R`.
3. visual reference + browser prototype.
4. secret contract/device store.
5. platform backend/native capture.
6. main Codex integration.
7. V2 native adapter/UI composition.
8. scanner/tasks/tests.
9. integration/source freeze `F`.
10. evidence-only sanitized manifests/index `E`.
11. independent final review only `V`.
12. narrow governance/PR/readback only `G`.

Use explicit path lists; never `git add -A`. No commit may absorb the dirty root or another worktree's files.
