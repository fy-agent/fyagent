# Issue #55 detailed-design review

Review scope: independent static review of `prd.md`,
`process-state-machine.md`, `design.md`, `detailed-design.md`, `implement.md`,
the product/architecture reviews, the three owning backend specs, task/Trellis
state, repository task-runner contracts, and the source map frozen at
`ca552f4d918cacc734f81f7efdef70619da139b8`.

Evidence boundary: `code_audit` only. `git diff --check` was clean before this
review write, and `git diff ca552f4d -- src src-tauri` was empty. No test, build,
browser, server, renderer, or native-runtime command was run.

## Round 1

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 8 P1 / 1 P2`).

The product and architecture contracts are unusually complete, but the current
detailed plan is not yet executable without either violating a frozen contract
or improvising file ownership, registration, and evidence commands during
implementation. The following findings block `DESIGN_FREEZE=PASS`.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The complete normative set still contains a relationship-scoped lock alternative. `unified-change-plan.md:554-555` says source and candidate actions share a “source-artifact action lock,” while the frozen design requires the single stable config-dir `CredentialArtifactIntegrityLockV1` for every action/scanner/GC/recovery before any relationship read (`detailed-design.md:554-578`, `codex-provider-configuration.md:330-361`). The phrase is not declared as an alias and can be implemented as the exact per-source authority that architecture rounds 18-20 removed. The existing stale-text assertion does not match this phrase. | Replace the sentence with the exact named global lock and its full preflight-to-publication lifetime; state that any per-ID/source lock is non-authoritative and nested only inside it. Extend the owning-spec stale scan to reject `source-artifact action lock` and equivalent relationship-selected authority. Because this reopens the prior concurrency finding in an owning spec, rerun the static architecture review after the text is synchronized. |
| P1 | The `DatabaseRuntime`/maintenance/copy source map is not closed against `ca552f4d`. `AppState` still owns `Arc<Database>` (`src-tauri/src/store.rs:7-23`), and 29 source files directly lock `db.conn`, including usage collectors, proxy logging, model pricing, response processing, DAO modules, migrations, backup, and tests. Remote replacement also enters through `services/sync_protocol.rs:318-336`, called by `services/{webdav_sync,s3_sync}.rs`; local import/backup/restore enter through `commands/import_export.rs`; DB hooks directly notify both auto-sync modules. `detailed-design.md:63-68,107-109,865-876` assigns only the new runtime/compatibility files and selected shared files, with conflicting ownership for `database/mod.rs`, and names none of these handle/copy participants. Consequently “drain every reader, close every handle, replace, reopen” and the SQL/WebDAV/S3/backup matrix cannot be implemented or reviewed from the frozen plan. | Add an exact migration inventory with one owner for `store.rs`, every production direct-connection caller, `sync_protocol.rs`, WebDAV/S3 command/service/auto-sync files, `commands/import_export.rs`, and the backup/restore call chain. Freeze the `DatabaseRuntime` guard/maintenance-participant API, how `AppState` and `ProxyService` receive it, how in-flight/background tasks drain, and the compile/static rule that forbids a raw connection surviving a maintenance boundary. Add named fault tests for every handle class and every local/remote replacement entry, including no post-download sync before readback. Reconcile `database/mod.rs` ownership in the source map, lane table, and implementation phase. |
| P1 | The protected create/edit cutover is neither source-complete nor commit-atomic. The code-map already records that the endpoint editor must become draft-only; current `EndpointSpeedTest.tsx:393-443` calls `vscodeApi.addCustomEndpoint/removeCustomEndpoint` and persists rows while editing. Neither that component, `src/lib/api/vscode.ts`, nor the Codex form plumbing is in the source/owner map. More broadly, the owning contract requires renderer and backend guards to switch in the same commit (`unified-change-plan.md:170`, `codex-provider-configuration.md:77`), but commit 9 guards backend/native entries and commit 10 later changes renderer/API/launcher (`detailed-design.md:917-920`). | Add exact owners and changes for `EndpointSpeedTest.tsx`, `vscode.ts`, and any `CodexFormFields`/form-state file needed to keep endpoint sets local until the private Provider transaction. Redraw commit boundaries so, per operation, renderer launch routing, API/query mutation removal, native entry guards, service gate, and private commit availability land together; visual-only components may remain later. Add a static/callsite fixture that proves no Codex form or speed-test path invokes endpoint add/remove before Plan apply, plus the existing positive probe-without-persist control. |
| P1 | The new Universal write path has no exact IPC registration contract. `detailed-design.md:526-538` requires `mutate_universal_provider`, but the “registered exactly once” list at `detailed-design.md:666-692` omits it. Current `lib.rs:2128-2133` registers only legacy get/upsert/delete/sync commands, and the source map assigns `universal_mutation.rs` but no precise public command function/registration/API call. An implementer could finish the domain module while leaving the one-command path unreachable. | Add `mutate_universal_provider` with exact request/outcome signature to the registered command list; name its implementation owner (`commands/provider.rs` or a dedicated command module), the `commands/mod.rs` export, `lib.rs` registration, TypeScript API/query call, and the retained legacy write-command behavior (`universal_mutation_v2_required`). Extend registration/fixture tests to prove one reachable new command, strict decode, safe reads, and zero renderer `upsert -> sync`. |
| P1 | Every focused Rust command in the detailed ladder is syntactically invalid for this repository task runner. The plan passes two to four filters in one `mise run rust:test` invocation (`detailed-design.md:821-826`; `implement.md:69-70,96,123,147`), while `scripts/tasks/host-native.mjs:744-767` rejects `filters.length > 1` with “rust:test accepts at most one test-name filter.” These commands cannot produce the promised terminal module evidence. | Use exactly one filter per `rust:test` invocation, or define one real shared prefix per intended batch and pass only that prefix. Mirror the corrected commands in both files, keep each family independently filterable, and retain the unfiltered final `rtk mise run rust:test`. Verify command existence/argument shape statically after the edit; do not run the tests until freeze. |
| P1 | AC-13's browser/renderer/native/failure-path evidence has no executable harness. `detailed-design.md:845-848` defers commands and capture paths until later, while `implement.md:219-225` only says “capture,” “launch,” and “run.” No owned script/task, fixture state, viewport, artifact/manifest path, startup readiness signal, terminal/cleanup command, or isolated native home/app-store contract is frozen. The existing `test:desktop:visual:preflight` only validates the existing manifest; it cannot generate Issue #55 state evidence. A raw debug-app launch can also honor the user's persisted `app_config_dir_override` before `FYAGENT_TEST_HOME`, risking mutation of real state. | Freeze exact owned script/task paths and commands for: deterministic renderer capture; browser interaction; built-Tauri isolated native launch/readback/restart; and deterministic failure injection. Define temp FyAgent/Codex homes plus isolated Tauri store/override handling, seed/readback/cleanup checks, readiness/timeout/exit rules, and evidence manifest schema. The runtime matrix must separately capture clean, warning, expired, drift, unsupported, secret-missing, running/recovery, and candidate safety; native credential-free create/edit/switch must prove actual local read/write/readback without touching user state. Add exact Trellis/Git/static/source-freeze commands to the same ladder and keep evidence classes distinct. |
| P1 | The #41 handoff sequence is impossible as written. Gate 0 requires #41 to receive and acknowledge the design-freeze SHA before any source/test edit (`implement.md:20-30`), but the authoritative handoff contract says the same consumable SHA must already contain Rust DTO, TS decoder, fixture, persistence/read APIs, worker/CAS, and cutover guard (`design.md:1795-1816`). The Gate-0 docs-only commit cannot contain those source artifacts, so either the gate never closes or a non-consumable design SHA is mislabeled as the required handoff. | Split the receipts explicitly: an early `DESIGN_CONTRACT_HANDOFF_SHA` may contain reviewed docs/specs and must be labelled non-compilable/non-consumable; the `CONSUMABLE_CONTRACT_HANDOFF_SHA` must occur after the minimum contract/store/worker/decoder/fixture/registration slice is committed and statically reviewed. Name the handoff artifact path, exact included paths, owner, #41 thread/readback field, and compatibility command. Move the blocking Gate-0 item to the correct phase while still sending the design contract early. |
| P1 | The Trellis execution gate is missing. The task is still `status=planning`; both `implement.jsonl` and `check.jsonl` contain only the seed `_example` row; and neither detailed artifact mentions context curation, `task.py validate`, `task.py start`, or `trellis-before-dev`. This plan explicitly uses subagent/module lanes, so beginning Phase 1/2 writes would violate the repository Phase-1 ready gate and dispatch workers without their owning specs/research. | Before any prototype/source/test write, add a Gate-0 checklist and exact commands to curate real implement/check context entries (UCP, Codex Provider, deep-link, backend/frontend indexes and the relevant research), remove/ignore the seed, run `rtk proxy python3 ./.trellis/scripts/task.py validate 08-14-issue-55-change-plan-mainline`, run `rtk proxy python3 ./.trellis/scripts/task.py start 08-14-issue-55-change-plan-mainline`, and read back `rtk proxy python3 ./.trellis/scripts/task.py current --source` plus `task.json.status=in_progress`. Require each implementation lane to load `trellis-before-dev` and its exact layer specs before editing. |
| P2 | The renderer file plan names two incompatible component sets. The source table owns `PlanPreview.tsx`, `PlanStatus.tsx`, and `CredentialArtifactRecovery.tsx` (`detailed-design.md:126-129`), while the frozen component split later names `ChangePlanPreview.tsx`, `ChangePlanLifecycleNotice.tsx`, `ChangePlanJobProgress.tsx`, `ChangePlanResourceResults.tsx`, `ChangePlanSafetyBanner.tsx`, `CredentialArtifactPanel.tsx`, and `CredentialCandidateCard.tsx` (`detailed-design.md:716-726`). The latter files have no source-map row, and the former names have no stated role in the split. Prototype/usability artifacts likewise have a manifest directory but no exact prototype/review filename or owner. | Select one exact component/file decomposition, update the source map, test map, lane ownership, and small-commit contents to match, and state which old files are replaced versus retained. Add exact prototype artifact, generated-reference, and usability-review paths/owner so later UI work cannot invent another structure. |

## Closure checklist for re-review

1. Synchronize the single global artifact-integrity lock wording and obtain a
   fresh architecture/static PASS.
2. Make the source/owner map exhaustive for DB runtime/copy participants and
   endpoint/form cutover callsites.
3. Freeze all IPC registrations, including the reachable Universal mutation.
4. Correct every Rust filter command and add exact isolated runtime/evidence
   harness commands and artifact paths.
5. Separate design-only and consumable #41 handoffs.
6. Add the mandatory Trellis context/activation gate.
7. Reconcile the renderer/prototype file map.
8. Re-run only static `git diff --check`, stale-text/callsite scans, and this
   independent detailed-design review. Tests/builds/runtime remain forbidden
   until the re-review is `0 P0 / 0 P1 / 0 P2` and `DESIGN_FREEZE=PASS` exists.

`DETAILED_DESIGN_REVIEW_ROUND_1=FAIL`

## Revision 2 closure submitted for re-review

All Round-1 findings have explicit closures in the synchronized design,
detailed design, implementation plan, owning specs, and source/owner map.
Architecture revision 23 is PASS. Tests/build/browser/server/renderer/native
runtime remain forbidden until this re-review passes and DESIGN_FREEZE is
signed.

`DETAILED_DESIGN_REVIEW_ROUND_2=PENDING`

## Round 2

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 3 P1 / 1 P2`).

This was a full static re-review rather than a Round-1 checklist replay. It
re-read the PRD, process/state machine, architecture and detailed design,
implementation plan, latest product/architecture verdicts, owning backend and
frontend indexes/specs, Trellis state, task-runner contract, and the source map
at `ca552f4d918cacc734f81f7efdef70619da139b8`. Architecture revision 23 remains
PASS. The global artifact-integrity wording, endpoint draft-only cutover,
reachable Universal command registration, one-filter Rust commands, split #41
receipts, Trellis activation gate, and single renderer/prototype file map are
closed. The remaining findings are detailed-design/source-map gaps and do not
reopen the accepted product or architecture semantics.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The source map still calls its DB-runtime list “complete” (`detailed-design.md:187-207`), but it omits production owners that retain `Arc<Database>` across request, background, or post-import paths: `commands/sync_support.rs:10-12`, `proxy/failover_switch.rs:19-29`, `proxy/provider_router.rs:16-29`, `proxy/server.rs:34-71`, and the many `services/skill.rs` APIs beginning at `:591`. These are not test-only matches. They conflict with the frozen rule that `AppState`/background services retain only `Arc<DatabaseRuntime>` and reacquire short guards (`detailed-design.md:522-535`), and none is represented by the named drain faults at `:537-546`. A close/replace/reopen implementation would therefore either edit unowned files or leave live holders outside the maintenance proof. | Regenerate the inventory from the frozen source and include every production `Arc<Database>` holder/direct-connection/copy participant, including post-import sync, proxy server/router/failover, and SkillService call chains. Freeze a closed participant type/tag for `DatabaseRuntime::read/write`, assign one non-overlapping owner for each file, and add named barrier faults for the omitted holder classes. Add a static inventory equality check that fails when a production raw holder/`.conn`/`lock_conn!` caller is unclassified; synchronize §2.3, the lane table, Phase 3, and commit ownership. |
| P1 | The isolated evidence harness is not source-complete and its four commands cannot currently run sequentially under their own rules. The design makes new evidence roots override persisted paths (`detailed-design.md:1013-1022`), but the mapped Evidence files at `:239-243` omit the current authorities that must implement that precedence: `config.rs::get_app_config_dir` currently prefers `app_store` (`config.rs:254-260`), `codex_config.rs::get_codex_config_dir` prefers settings (`codex_config.rs:253-260`), and the persisted store is initialized in `app_store.rs`. It also says the native task resolves a bundle through the existing host-native planner (`detailed-design.md:1034-1037`), although `scripts/tasks/host-native.mjs` exposes command planning, not a debug-bundle output receipt/resolver, and that file has no owner. Separately, every evidence task “refus[es] a dirty tree” (`:1001-1004`), but the first mode writes final artifacts to the unignored task directory (`:1055-1063`); `.gitignore` contains neither the claimed ignored `evidence/tmp/` nor an evidence-output policy, so the second mode must reject the first mode's output. The ladder also puts renderer and browser between `build:debug` and native even though native requires the “immediately preceding” build receipt. | Add the exact debug-only path/startup files and serialized owners (`config.rs`, `app_store.rs`, Codex path authority, `lib.rs`, and any settings seam), plus the host-native output-receipt/resolver file and tests, to the map/commit plan. Freeze precedence and release-build invisibility at those callsites. Define one coherent source-clean rule: ignore only validated temp roots and either whitelist/hash prior same-`SOURCE_FREEZE_SHA` evidence outputs or run all modes into an out-of-tree staging root before one atomic publish. Add the required ignore file/path owner. Bind native/failure to a same-SHA build receipt and either move `build:debug` immediately before native or replace “immediately preceding” with an exact, executable receipt predicate. |
| P1 | The proposed public mise/Playwright task API is not closed against the repository task contract. Adding `.mise/tasks/change-plan.toml` makes the owning spec's exact include list stale (`task-runner-contract.md:14-24`), and `tasks:docs:check` byte-generates every task into `docs/fyagent/development/mise-tasks.md` (`task-runner-contract.md:167-172`; `scripts/tasks/task-docs.mjs:94-125,151-174`). Neither the spec nor generated document appears in the Evidence source map/owner/commit, and no `tasks:docs:generate --apply` step exists, so the named final `check:contracts`/`check` gate will fail even if all four tasks exist. The design also does not freeze each new task's required `description`/`FYAGENT_TASK_EFFECT` metadata, and pinning `@playwright/test` alone does not define or preflight the Chromium executable used by the exact browser commands. | Add `task-runner-contract.md` and generated `docs/fyagent/development/mise-tasks.md` to the owned file/commit map; specify each task's description, effect class, composition, and noninteractive behavior; run the canonical docs generator during that implementation commit and byte-check it in the focused gate. Freeze a locked/offline-safe browser-executable contract (or an explicit dependency-environment preparation task) with version/path/hash/readiness preflight; evidence tasks must not silently download a browser. Include its package/workspace/lock files and failure tests in the same owner map. |
| P2 | Governance metadata is stale: `task.json:48` records `architectureReview=pass_round_20_0_p0_0_p1_0_p2`, while the normative design and review now require and report Round 23 PASS. Gate 0 checks architecture PASS, but the task card would freeze a different review generation. | Before creating `design-freeze.md`, update the task metadata to the exact Round-23 verdict and include the architecture-review file/hash in the freeze receipt; run the planned stale-reference/task validation checks. |

No additional P0/P1/P2 was found in the Plan schema/canonical digest,
baseline/affected-resources contract, one-confirmation admission, #35 narrow
port/typed-disable rule, #41 consumable predicate, Provider endpoint cutover,
Universal IPC reachability, artifact/candidate lock/ack/GC state machine,
four-locale/a11y projection, or the one-filter Rust command syntax.

Evidence boundary remains `code_audit`. Before this append, `git diff --check`
was clean and the source/test/tooling diff from `ca552f4d` was empty. No test,
build, browser, server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_2=FAIL`

## Revision 3 closure submitted for re-review

Round-2 findings are addressed by an equality-grade DB holder/caller/copy
inventory with closed participant/reason types and serialized owners; a
source-complete pre-Tauri debug evidence authority; repo-sibling evidence
transactions with one final atomic publish; a same-SHA host-native build
receipt; and exact task-runner/generated-doc/Chromium preparation and offline
preflight contracts. Task metadata now records architecture Round 23.

Tests, builds, browser, server, renderer, and native runtime remain forbidden
until this re-review passes and `DESIGN_FREEZE=PASS` is signed.

`DETAILED_DESIGN_REVIEW_ROUND_3=PENDING`

## Round 3

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 4 P1 / 0 P2`).

This was a complete static re-review of the PRD, process/state machine,
technical design, detailed design, implementation plan, Round-1/Round-2
findings, product/architecture reviews, task metadata, owning backend specs,
task-runner/development-environment contracts, and the source map at
`ca552f4d918cacc734f81f7efdef70619da139b8`. The declared 25-file production
`.conn`/`lock_conn!` set itself matches the frozen source after test-only ranges
are excluded, the architecture metadata now records Round 23, and the Round-2
config/store/pre-Tauri/build-receipt file map is present. Four executable
closure gaps remain before DESIGN_FREEZE.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The DB equality manifest does not cover the full legacy-`Database` migration surface it claims to close. Its discoverable classes are `Arc<Database>`, direct connection access, raw owned connections, external SQLite and copy/replacement participants (`detailed-design.md:241-260`), but the frozen source also has production operation entrypoints typed as `&Database` that are neither classified nor assigned a serialized file owner. In particular, `claude_desktop_config.rs:128,133,278,920,937,975` reads/writes the FyAgent main DB around config-file work, and `codex_history_migration.rs:114,169,658,705` reads/writes the main DB while the same file separately owns external Codex SQLite. The latter is listed only as an external authority plus a `lib.rs` capture, and the former is absent from the source map and `DbParticipant`. The explicit test-only inventory is also incomplete: `services/proxy.rs:6750` directly locks `db.conn` inside the `#[cfg(test)]` module beginning at `:3225`, but §2.3 names only `change_plan.rs`, `proxy/response_processor.rs`, `services/provider/mod.rs`, and later `database/tests.rs`/support as test classifications (`detailed-design.md:209-217,255-260`). The checker can therefore report equality while an implementer either leaves legacy typed operations behind, invents an undocumented `Database` alias/facade, or edits unowned files. | Add an exact `borrowedDatabaseCallers`/legacy-facade disposition to `database-runtime-inventory.json` and its checker, with separate baseline and expected sets. Either remove every production `&Database` boundary or freeze one explicit safe alias/facade and prove it cannot expose/retain a connection. Assign serialized ownership and participant scope for `claude_desktop_config.rs` and the main-DB portions of `codex_history_migration.rs` (plus the already mapped Provider/settings sites), and add `services/proxy.rs` to the test-range set. The equality test must fail on an unclassified borrowed boundary as well as on raw holders/direct opens. Add a Claude Desktop config/token barrier fault and keep the existing Codex-history fault bound to both its main-DB and external-DB phases. |
| P1 | The frozen `DatabaseRuntime` API cannot implement the promised drain semantics for asynchronous/background operations. `read`/`write` expose a non-async, non-Send closure guard, while maintenance waits for an active-participant count to reach zero (`detailed-design.md:578-619`). The same design requires WebDAV/S3 uploads and auto-sync, periodic session sync/backup, Codex-history migration, SkillService filesystem work, and post-import work to drain, and explicitly forbids a connection guard across filesystem/network awaits (`detailed-design.md:178-182,630-657`). Once a short DB closure returns, there is no typed operation/activity lease, cooperative stop-and-join receipt, or generation fence keeping that still-running file/network operation in the drain count; maintenance can observe zero between “read old generation” and “publish external effect.” The supposedly exact failure surface is also not representable: `begin_maintenance`, `close_and_take`, and `install_verified` return bare success types even though timeout, close, verification, install, and reopen faults are normative (`detailed-design.md:583-586,648-657`). | Freeze a separate connection-free `DbActivityLease`/registration protocol that can safely cover the whole async/background operation, or an equally exact stop/cancel/join plus generation-CAS protocol. Specify admission closure, acquisition order, whether the lease may cross `await`, how old-generation upload/sync results are quarantined, and how maintenance proves every registered task joined before close. Give `begin_maintenance`, `close_and_take`, `install_verified`, and `reopen` exact `Result` signatures with a closed error enum and the post-error admission/generation state. Extend named faults at the gaps after snapshot/read and before external publish for manual upload, auto-sync, SkillService, session sync, and Codex-history migration; a paused operation must prevent replacement or lose publication authority, not merely release a connection. |
| P1 | The four fixed-argv evidence commands still have no executable cross-process transaction authority. The transaction is keyed by repository hash, source SHA, **and session ID**, but no command accepts a session, no environment/pointer/lock is defined, and no mode is named as the creator versus a strict joiner (`detailed-design.md:1161-1169`). `SOURCE_FREEZE_SHA` is recorded after the source-freeze commit (`implement.md:323-325`) but no immutable receipt/path tells the first process what value to bind; later modes only promise to compare it. Stale and concurrent sessions are mentioned only as final publish failures (`detailed-design.md:1238-1245`). Consequently renderer, browser, native, and failure processes cannot deterministically select the same transaction, reject a stale partial transaction, or prove that failure did not assemble modes from different attempts. | Freeze the exact repo-sibling root and one authority protocol. For example, renderer exclusively creates an `ActiveEvidenceSessionV1` under a repository-scoped lock, binds current clean HEAD/tree/design/dependency/build-receipt hashes and a random session ID, and atomically publishes a pointer; browser/native/failure may only join that exact record and advance a closed `renderer -> browser -> native -> failure -> publishing -> published` CAS state. Define out-of-order/retry/crash/stale-owner/concurrent-session behavior, lock lifetime, ownership proof, cleanup, and how failure clears the pointer only after the destination rename and directory fsync. Alternatively expose the session/source receipt through formal validated task usage. Add contract tests for two concurrent renderers, a stale partial session, wrong HEAD/tree/receipt, crash after every mode rename, and a mixed-session assembly attempt. |
| P1 | The claimed exact Playwright/mise closure still requires unreviewed choices and contradicts the current mutation policy. No literal `@playwright/test` version appears in any frozen artifact, `chromium-lock.v1.json` has no unambiguous repository path or frozen schema/host-entry values, and no command creates/reviews its expected archive/executable/payload-tree hashes. `change-plan:chromium:prepare` is classified `dependency-environment` and source-clean, so it cannot generate or update a tracked lock; if it only reads the lock, there is no authoritative lock-generation/bootstrap path (`detailed-design.md:1113-1159`). Separately, `change-plan:evidence:failure` is `source-modifying` but deliberately has no usage or confirmation (`detailed-design.md:1120-1146`), while the current task-runner contract rejects a mutation task with neither preview nor confirmation and names only formatting as a no-prompt source-modifying exception (`task-runner-contract.md:146,216`). Merely scheduling a later spec/checker edit does not freeze which safety rule wins, so `tasks:validate` and the public task contract are not predetermined. | Freeze the literal Playwright version; the exact lock path/schema; current-host or closed multi-host entries; trusted download URL/archive hash, browser revision, executable path/hash and payload-tree hash; and the exact `pnpm-workspace.yaml` install-script policy. Add a reviewed lock generation/update command that is preview-by-default with `--apply` before source freeze, or commit lock bytes from another explicitly named reproducible authority; keep `chromium:prepare` repository-read-only and limited to the verified out-of-tree cache. For final publish, either require a default-no confirmation/formal `--apply`, or explicitly add one narrowly constrained evidence-publish exception to `task-runner-contract.md`, `task-contract-check.mjs`, `PARAMETERIZED_TASKS`/`REQUIRED_TASKS`, tests and generated docs. The selected rule must be stated now, including exact task metadata/argv and negative tests proving prepare cannot mutate source and evidence modes cannot download/fallback. |

No additional P0/P1/P2 was found in the Plan schema/canonical digests,
baseline/affected-resource contract, one-confirmation admission, #35 narrow port
and typed-disable behavior, #41 design/consumable handoffs, protected Codex
create/edit/switch cutover, Universal IPC reachability, artifact/candidate
global-lock/acknowledgement/GC state machine, four-locale/a11y projection,
prototype/component map, serialized ownership epochs, or small-commit/conflict
budget. `task.json` correctly records
`architectureReview=pass_round_23_0_p0_0_p1_0_p2` and
`phase=detailed_design_revision_3`.

Evidence boundary remains `code_audit`. Immediately before this append,
`git diff --check` was clean, `HEAD` was
`6859e9ce04970008f4cf8b3d4883b4f70316291a`, and the source/test/tooling diff
from `ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build,
browser, server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_3=FAIL`

## Revision 4 closure submitted for re-review

Round-3 findings are addressed by exact borrowed-function/impl/task-capture and
test-range inventories with no surviving production `Database` facade; a
connection-free async activity lease, closed stop/join registry, generation
publication fencing, and linear Result transition tokens; a repo-scoped locked
`ActiveEvidenceSessionV1` creator/joiner/CAS/crash protocol; and literal
Playwright 1.61.1 with reviewed Chromium lock preview/apply, repo-read-only
prepare, and failure publish under the existing preview/apply mutation policy.

Tests, builds, browser, server, renderer, and native runtime remain forbidden
until this re-review passes and `DESIGN_FREEZE=PASS` is signed.

`DETAILED_DESIGN_REVIEW_ROUND_4=PENDING`

## Round 4

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 3 P1 / 0 P2`).

This was a full static re-review of the PRD, process/state machine, technical
design, detailed design, implementation plan, all three earlier detailed-design
rounds, product/architecture reviews, task metadata, owning specs, current mise/
task-runner source, and the source map frozen at
`ca552f4d918cacc734f81f7efdef70619da139b8`. Revision 4 closes the prior
Playwright/task-runner finding: the literal 1.61.1 package set, closed macOS
Chromium entries and URLs, preview/apply lock bootstrap, repository-read-only
prepare, fixed task argv/metadata/sets/generated docs, and implementation steps
are now predetermined. The DB owner epochs, 25 direct callers, 19 implementation
blocks, external SQLite split, worker registry, result/error surface, evidence
root and creator/joiner sequence are substantially more precise. Three
executable authority gaps still block DESIGN_FREEZE.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The promised equality-grade removal of the legacy `Database` facade still has an uncovered syntax class. Section 2.3 lists files for borrowed boundaries and says the future manifest will contain exact function identities, but neither the declared manifest classes nor the checker's rejection list includes production associated-path calls such as `Database::…` or equivalent trait/alias forms (`detailed-design.md:223-293`). They exist at the frozen source in `commands/import_export.rs:147,169,175` (`list_backups`/`rename_backup`/`delete_backup`), `lib.rs:1047,1074` (version guard/`init`), and `codex_history_migration.rs:524-525,606-607,1129-1130` (`table_exists`/`has_column` on external Codex SQLite). The file-only “explicit `&Database`” list even includes `commands/import_export.rs`, whose production legacy surface is these associated calls rather than an explicit borrowed parameter. Because `explicitBorrowedFunctions`, `implDatabaseBlocks`, and task captures are different sets, the named inventory command cannot by its frozen contract prove that every facade use/import was dispositioned or that associated test uses were separated by exact `cfg` ranges. The compile gate may catch some unresolved calls, but it is not the claimed baseline/current equality evidence and does not freeze the helper relocation or external-authority disposition. | Add a closed `legacyDatabasePathUses`/associated-call-and-import class (including trait/alias equivalents) to the baseline/current manifest and checker, enumerate exact production symbol identities plus syntax-aware test ranges, and freeze each disposition: main-DB runtime closure, external `ExternalSqliteAuthority`, pure free helper, or test harness. Expected production facade/type uses must be empty and the checker must fail on an unclassified `Database::`, import/alias, trait implementation, or test range—not only `&Database`, `Arc<Database>`, and inherent `impl Database`. Synchronize Phase 3's inventory step and focused fixture with this class. |
| P1 | The activity lease still does not fence the irreversible WebDAV/S3 upload effect. The design first says an operation holds its lease through its last commit/publication fence, but then explicitly says uploads build a snapshot under a lease, **drop it, then send** the bytes (`detailed-design.md:670-680`). Current upload authority writes fixed remote DB/skills objects and then the manifest (`services/{webdav_sync,s3_sync}.rs`); if maintenance replaces generation `g` after the lease drops, an old-`g` manual or auto upload can still overwrite those remote objects/manifest. A later `fence_publication(g, participant)` rejection can suppress local tray/cache/status publication, but cannot undo the already completed remote PUTs. The frozen `DbPublicationPermit` signature also does not state that it is acquired before the first remote effect, remains counted by maintenance, or is consumed by the remote manifest publication. Thus the named “pause before external success publication” tests can pass while stale remote side effects have already occurred. | Hold a connection-free `DbActivityLease` through every remote object write, manifest publication, acknowledgement, and local result fence, **or** freeze an equivalent linear remote-effect permit acquired before the first PUT that closes maintenance admission/counts as active until remote terminal readback. Bind it to participant+generation+snapshot digest, define cancellation/partial-object cleanup or quarantine, and make old generation unable to publish the authoritative remote manifest. Add separate manual and auto WebDAV/S3 faults after snapshot, during each artifact PUT, before manifest, and after manifest/before ack; maintenance must either remain blocked or prove that no stale remote authority was published, not merely reject the later local notification. |
| P1 | `ActiveEvidenceSessionV1` crash recovery contradicts its own source-clean admission and pointer lifetime. Every evidence mode requires a fully clean porcelain-v2 worktree (`implement.md:379-384`), and `failure --apply` revalidates cleanliness before publication (`detailed-design.md:1437-1445`). A crash after the complete directory is renamed into tracked `evidence/<SOURCE_HEAD>/` but before record CAS necessarily leaves that new destination as worktree dirt; the promised retry that verifies the identical destination and completes CAS can therefore never pass its own preflight. The same dead-end exists after CAS and before pointer clear. Separately, the protocol clears the sole active pointer before cleaning the out-of-tree session, while every mode exits nonzero on cleanup failure; a cleanup failure after pointer unlink has already published but leaves no strict-joiner authority for the required retry, contradicting “cleanup failure never … publishes.” The listed crash tests do not resolve either state-machine contradiction. | Freeze a recovery-only clean-tree rule that admits exactly the expected destination file set/root digest for the bound session while rejecting every other dirty path, and represent destination-renamed, record-published, cleanup-pending, and pointer-cleared as explicit CAS states. Retain an immutable pointer or separate tombstone until cleanup is terminal; only then clear it, or define a bounded authenticated stale-session GC that cannot create a new session first. Enumerate the pointer/record fields needed to bind the destination receipt and cleanup state. Add contract cases that inject crashes after destination rename, after published CAS, after pointer operation, and during cleanup, assert the actual porcelain-v2 state, and prove an identical retry reaches one terminal published receipt while unequal/unrelated dirt remains zero-write. |

No additional P0/P1/P2 was found in the Plan schema/canonical digests,
baseline/affected-resource contract, one-confirmation admission, preview effect
spy, #35 narrow port/typed-disable behavior, #41 exact-SHA handoff, Codex
create/edit/switch cutover, Universal IPC, artifact/candidate lock/ack/GC,
four-locale/a11y projection, Chromium preparation/offline preflight, exact task
descriptions/effects/usage, serialized ownership epochs, or small-commit/conflict
budget. `task.json` correctly records
`architectureReview=pass_round_23_0_p0_0_p1_0_p2`,
`phase=detailed_design_revision_4`, and `designFreeze=pending`.

Evidence boundary remains `code_audit`. Immediately before this append, HEAD was
`6859e9ce04970008f4cf8b3d4883b4f70316291a`, `git diff --check` was clean, and
the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build, browser,
server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_4=FAIL`

## Revision 5 closure submitted for re-review

Round-4 findings are addressed by syntax-aware equality for every legacy
associated/import/re-export/alias/trait use; remote-effect permits held from
snapshot through immutable object PUTs, manifest readback, ack and cleanup; and
explicit destination/cleanup publication states with an exact recovery-only
dirty-set rule and pointer authority retained through terminal cleanup.

Tests, builds, browser, server, renderer, and native runtime remain forbidden
until this re-review passes and `DESIGN_FREEZE=PASS` is signed.

`DETAILED_DESIGN_REVIEW_ROUND_5=PENDING`

## Round 5

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`).

This was another full static review of the PRD, process/state machine,
architecture and detailed design, implementation plan, all prior findings,
product/architecture verdicts, task metadata, owning backend/frontend specs,
current task-runner source, and the immutable
`ca552f4d918cacc734f81f7efdef70619da139b8` source map. Revision 5 closes the
legacy-Database finding: the production associated-call list matches the frozen
source, `legacyDatabasePathUses` now covers imports/re-exports/qualified aliases/
trait forms and syntax-aware test scopes, every baseline identity has a closed
disposition, expected production/test sets are empty, and Phase 3 owns the same
equality command. It also closes the successful-path remote-generation race and
the destination-dirty retry rule. Two failure-state contracts remain
unimplementable as written.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | `DbRemoteEffectPermit` closes the successful upload path but has no representable error/interruption ownership protocol. The only frozen transitions are `begin_remote_effect(...) -> Result<DbRemoteEffectPermit, DbRuntimeError>` and `publish_manifest_and_ack(...) -> Result<DbRemoteTerminalReceipt, DbRuntimeError>` (`detailed-design.md:648-659`). The design requires failed/cancelled pre-manifest PUTs to delete or quarantine and read back the attempt **before** lease release, and post-manifest/pre-ack failures to persist a durable reconciliation receipt (`:713-730`), yet it defines no permit receiver/ownership on error, no staged-object PUT/abort/quarantine transition, no `TransitionFailure<DbRemoteEffectPermit>`, no durable attempt/receipt schema or store owner, and no startup recovery/admission gate. A normal `?`, cancellation, unwind, or process interruption can therefore drop the permit/lease and decrement the active count while asynchronous remote cleanup or acknowledgement is unfinished; `Drop` cannot await that work. The named tests only pause successful phases and assert eventual ack (`:804-812`; `implement.md:203-208`); they do not inject PUT/readback/cleanup failure, panic, response loss, or restart. Thus the Round-4 invariant has moved from the success path to the error path: maintenance can again observe no in-memory activity without the promised terminal remote authority. | Freeze a closed linear remote state machine, for example `RemoteStaging -> ManifestPublished -> AckPending -> Terminal|Quarantined`, with `!Clone` tokens that own the activity lease. Every PUT/manifest/readback/ack/abort operation must take or borrow the token explicitly and return the next token or a `TransitionFailure<state>`; an error may not silently drop authority. Name the durable effect-start/attempt/manifest/ack receipt schema, persistence path and owner, and run its recovery before maintenance admission after restart—or narrow the promise to immutable unreferenced objects and prove no durable recovery is required. Define cleanup-failure and process-death behavior. Add manual/auto WebDAV/S3 faults for each PUT error, cancellation/panic, manifest response loss, ack persistence failure, cleanup/quarantine failure, and restart; each must prove either terminal/quarantined readback before activity release or a durable recovery gate that still blocks replacement. Synchronize the exact API and Phase-3 checklist. |
| P1 | The terminal evidence-pointer protocol is byte- and hash-impossible. Renderer creates an **immutable** `active-session.v1.json` before any mode runs (`detailed-design.md:1388-1392`), but after terminal cleanup a later renderer must atomically rename that same pointer to `terminal-pointers/<sessionId>.v1.json` (`:1524-1532`). The terminal file is nevertheless required to contain destination/root digests, final record revision/hash, cleanup-complete receipt, and `archivedAt` (`:1421-1427`)—facts the immutable initial pointer cannot contain, and rename cannot change bytes. The record also stores the terminal-pointer hash while the terminal pointer stores the final-record hash, creating an undefined circular hash dependency. In addition, the declared CAS sequence contains `publish_prepared -> publishing`, but failure preview is specified to CAS directly from the fourth mode to `publishing`; no transition ever creates `publish_prepared` (`:1392-1409`, `:1505-1512`). The dirty-set and retained-pointer ideas are sound, but this exact authority cannot be serialized or crash-tested without inventing a different protocol during implementation. | Define separate closed schemas for `ActiveEvidencePointerV1` and `TerminalEvidenceReceiptV1` and a one-directional hash graph. For example, finalize/fsync the session record, create/fsync a terminal receipt that hashes that fixed record, permit both active and terminal files briefly, then unlink/fsync the active pointer; the record may store the terminal path/schema but not a hash that recursively depends on its own final hash. Specify how creator/joiners resolve the legal both-files crash state and reject mismatched pairs. Alternatively archive the original active-pointer bytes and move all terminal truth to a separately named receipt, without claiming the archive contains it. Make `publish_prepared` a real transition with exact creator/preimage or remove it consistently. Extend crash fixtures to compare exact pointer/record/receipt bytes and hashes before/after every create/fsync/unlink boundary and prove there is always at least one non-circular recovery authority. |

No additional P0/P1/P2 was found in the public/private Plan contract, canonical
digests, affected-resource baseline, one-confirmation admission, preview effect
spy, #35 narrow port and typed-disable behavior, #41 exact-SHA handoff, Codex
create/edit/switch cutover, Universal command, artifact/candidate lock/ack/GC,
DB direct/borrowed/impl/capture inventories, maintenance transition errors,
four-locale/a11y UI, Playwright/Chromium/task contracts, source ownership, or
small-commit/conflict budget. `task.json` correctly records architecture Round
23, `phase=detailed_design_revision_5`, and `designFreeze=pending`.

Evidence boundary remains `code_audit`. Immediately before this append, HEAD was
`6859e9ce04970008f4cf8b3d4883b4f70316291a`, `git diff --check` was clean, and
the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build, browser,
server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_5=FAIL`

## Revision 6 closure submitted for re-review

Round-5 findings are addressed by `#[must_use]` linear remote state tokens that
return authority on error, durable pre-effect receipts plus Drop/startup
recovery gates, and explicit terminal/quarantine restart faults. Evidence now
uses separate active-pointer, final-snapshot and terminal-receipt schemas with a
one-way hash graph; `publish_prepared` is reachable, the final record references
the receipt, and active-pointer unlink happens only after every prior fsync.

Tests, builds, browser, server, renderer, and native runtime remain forbidden
until this re-review passes and `DESIGN_FREEZE=PASS` is signed.

`DETAILED_DESIGN_REVIEW_ROUND_6=PENDING`

## Round 6

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 1 P1 / 0 P2`).

This round re-read the complete PRD, process state machine, architecture and
detailed design, implementation plan, Round 1-5 history, product/architecture
reviews, task metadata, owning specs, current task-runner implementation, and
the frozen `ca552f4d918cacc734f81f7efdef70619da139b8` source map. The Round-5
remote-effect finding is closed: the source map now owns `database/remote_effect.rs`;
the receipt path/schema/owner and startup state are frozen; every `#[must_use]`
token owns the activity lease; borrowed PUT errors preserve `RemoteStaging`;
consuming transitions return `TransitionFailure<StateToken>`; and pre-effect
receipt, Drop/panic, process-death, terminal/quarantine, offline recovery and
maintenance gates have named faults synchronized into Phase 3. The evidence
authority also now has three separate schemas and a non-circular hash DAG. One
durable-publication ordering/locator gap remains.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | `publish_prepared` is now reachable, but it is persisted before the bytes it purports to authorize exist. Failure without `--apply` first CASes `failure -> publish_prepared` and only afterward verifies receipts, assembles/fsyncs the out-of-tree publish directory, and records its file-list/root digest (`detailed-design.md:1578-1585`). A crash immediately after the CAS leaves `publish_prepared` with no complete directory or bound digest; the next `failure --apply` is specified to claim `publish_prepared -> publishing`, not to recover/clean/rebuild that incomplete preparation under an owner/preimage CAS (`:1586-1590`). The crash matrix likewise begins at destination rename/finalization and does not name a publish-assembly crash (`:1623-1633`). Separately, after the active pointer is unlinked, recovery is required to obtain the session ID from the destination `manifest.json` and derive `terminal-receipts/<SOURCE_HEAD>.<sessionId>.v1.json` (`:1596-1604`), but the frozen manifest schema lists no `sessionId`, binding digest, or terminal-receipt locator (`:1614-1619`), while the destination directory itself is keyed only by `SOURCE_HEAD`. The final snapshot is also only “at the session root,” without a literal deterministic path. With no active pointer, multiple retained same-HEAD/mismatched receipts cannot be selected by the stated exact authority. Thus the hash graph is acyclic, but the transition into it and its no-active recovery root are still underdefined. | Either assemble/fsync and hash a claim-owned candidate first, then CAS its exact preimage/receipt to `publish_prepared`, or add a distinct `publish_preparing{claimId,partialPath}` state with deterministic dead-claim cleanup/rebuild and only advance after the prepared directory receipt is durable. `failure --apply` must verify that receipt before `publishing`. Add `sessionId`, binding digest, and deterministic terminal-receipt relative path to the tracked evidence manifest/schema before its root digest is frozen; freeze the literal final-snapshot filename/path. Define no-active selection when zero, one, or multiple same-HEAD receipts exist and require exact destination/manifest/receipt agreement. Add faults during every publish-directory file/write/fsync and between prepared-receipt/CAS, plus post-unlink recovery with missing, duplicate, foreign, and mismatched terminal receipts. Synchronize the Phase-7/8 checklist and contract tests. |

No additional P0/P1/P2 was found in Plan schema/canonical identity,
baseline/affected resources, one-confirmation admission, preview spies, #35/#41
handoffs, Codex cutover, Universal/artifact/candidate authority, DB equality and
maintenance, the remote upload success/error/restart state machine, evidence
dirty-set recovery and one-way hashes, four-locale/a11y UI, Playwright/Chromium/
mise contracts, file ownership, or the small-commit/conflict budget. `task.json`
correctly records architecture Round 23, `phase=detailed_design_revision_6`, and
`designFreeze=pending`.

Evidence boundary remains `code_audit`. Immediately before this append, HEAD was
`6859e9ce04970008f4cf8b3d4883b4f70316291a`, `git diff --check` was clean, and
the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build, browser,
server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_6=FAIL`

## Revision 7 closure submitted for re-review

Round-6 finding is addressed by a claim-owned `publish_preparing` state whose
directory, fsyncs, manifest/root digest and prepared receipt precede the
`publish_prepared` CAS; destination manifests now bind session/binding/terminal
locator, the final snapshot path is literal, and no-active recovery has closed
zero/one/multiple/mismatch selection plus assembly/crash faults.

Tests, builds, browser, server, renderer, and native runtime remain forbidden
until this re-review passes and `DESIGN_FREEZE=PASS` is signed.

`DETAILED_DESIGN_REVIEW_ROUND_7=PENDING`

## Round 7

Result: `DETAILED_DESIGN_REVIEW=FAIL` (`0 P0 / 2 P1 / 0 P2`).

This was a full static re-review of the PRD, process state machine, technical
and detailed design, implementation plan, all Round-1..6 findings, product and
architecture verdicts, task metadata, owning backend/frontend specs, current
task-runner contract/source, and the immutable
`ca552f4d918cacc734f81f7efdef70619da139b8` source map. Revision 7 closes the
destination-manifest locator and no-active lookup portions of Round 6: the
manifest binds session ID, binding digest, and deterministic terminal-receipt
path before root hashing; the final snapshot path is literal; zero/one/
duplicate/foreign/mismatched post-unlink cases are named; and Phase 7/8 plus
the contract-fault list reference those cases. Two publication-authority gaps
still make the exact evidence ladder and its crash recovery unimplementable
without inventing semantics during implementation.

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | `PreparedPublicationReceiptV1` is still not a closed durable authority. The source/schema map enumerates manifest, active pointer/session, final snapshot, and terminal receipt schemas, but no prepared-publication schema or exact type/field contract (`detailed-design.md:362-374`). The new sequence renames the claim directory to the fixed `<SOURCE_HEAD>.prepared/`, fsyncs `publish/`, then merely “writes/fsyncs” `prepared-publication-receipt.v1.json` before the CAS (`:1469-1478`): it does not freeze create-new versus atomic-replace semantics, canonical/deny-unknown fields, the claim/session/binding/record-preimage/manifest/file-list/root binding, receipt readback, or the required fsync of the receipt's parent directory. More importantly, the recovery sentence says a crash **after prepared rename** verifies the immutable receipt and completes CAS (`:1479-1481`), although at that boundary the receipt may not yet exist (or may be torn before its fsync); the active claim still names the now-absent partial path, and the only cleanup rule deletes/rebuilds that partial. A fixed prepared directory or partial/corrupt receipt can therefore strand the claim or be replayed without a specified owner/preimage decision. Listing prepared-rename/receipt-fsync faults at `:1651-1664` does not define their expected bytes, state, or recovery result. | Add the prepared receipt schema/type to the exact source map and freeze its canonical fields, path, write primitive, file readback, and parent-directory fsync. Under the repo lock, define deterministic recovery for every `publish_preparing` filesystem state: partial only; prepared directory with no receipt; receipt temp/torn/corrupt; valid prepared+receipt before CAS; and stale/mismatched claim/preimage. State exactly when the owning claim may finish the receipt and CAS, when its own bytes may be removed/rebuilt, and when the session is quarantined/aborted; a new claim must never collide with or adopt unverifiable fixed-path bytes. Add byte/state/result oracles for crashes after every directory/file write, rename, file fsync, parent fsync, receipt readback, and before/after the prepared CAS, and mirror the named schema/recovery work in Phase 7/8. |
| P1 | The frozen command ladder cannot reach the state that `failure --apply` requires, and its explicit apply order still moves state before verification. Renderer, browser, and native leave the record at `native`, after which both exact ladders invoke only `change-plan:evidence:failure --apply` (`detailed-design.md:1721-1724`; `implement.md:390-393`). But the design assigns fourth-mode completion and `failure -> publish_preparing -> publish_prepared` only to failure **without** `--apply` (`detailed-design.md:1595-1601`), says `failure --apply` accepts only `publish_prepared` (`:1482-1483`), and explicitly has apply claim `publish_prepared -> publishing` *before* revalidating the publish digests (`:1603-1605`). Starting the documented final command from `native` is therefore out of order; starting it from a pre-existing prepared state also violates Round 6's required receipt verification-before-`publishing` order. Retry/crash semantics do not decide which behavior the single public argv owns. | Choose and freeze one executable contract. Either put a non-apply failure preview immediately before `failure --apply` in every exact ladder, or specify that `failure --apply` composes fourth-mode capture and claim-owned preparation from `native` through `publish_prepared` before applying. In both forms, load/schema-validate the prepared receipt, recompute and compare its prepared-tree/manifest/file-list/root and claim/record preimage under the lock, and only then CAS to `publishing`; mismatch must leave `publish_prepared` unchanged. Synchronize task help, Phase 7/8, and contract tests for direct final-ladder execution, preview-then-apply, idempotent prepared retry, and receipt mismatch before the publishing CAS. |

No additional P0/P1/P2 was found in the Plan schema/canonical identity,
baseline/affected resources, one-confirmation admission, preview effect spies,
#35 narrow port/typed-disable boundary, #41 exact-SHA handoffs, Codex create/
edit/switch cutover, Universal/artifact/candidate authority, DB equality and
maintenance, linear remote-upload recovery, terminal evidence hash graph,
post-unlink selection, four-locale/a11y UI, Playwright/Chromium/mise contracts,
serialized owners, or the small-commit/conflict budget. `task.json` correctly
records architecture Round 23, `phase=detailed_design_revision_7`, and
`designFreeze=pending`.

Evidence boundary remains `code_audit`. Immediately before this append, HEAD
was `6859e9ce04970008f4cf8b3d4883b4f70316291a`, `git diff --check` was clean,
and the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build, browser,
server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_7=FAIL`

## Round 8

Revision 8 claims closure of both Round-7 P1 findings by adding the exact
`PreparedPublicationReceiptV1` schema and claim-qualified durable/recovery
protocol, and by splitting the final ladder into default failure preparation
followed by `failure --apply` with verification before the `publishing` CAS.
Independent full static re-review is required; this note is not a verdict.

`DETAILED_DESIGN_REVIEW_ROUND_8=PENDING`

### Independent Round 8 verdict

Result: `DETAILED_DESIGN_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`).

This was a full static re-review of the complete PRD, process state machine,
technical design, detailed design, implementation plan, Round-1..7 findings,
product and architecture reviews, task metadata, owning backend/frontend specs,
current task-runner contract/source, and the source map frozen at
`ca552f4d918cacc734f81f7efdef70619da139b8`. It was not limited to the Revision
8 delta.

Both Round-7 P1 findings are closed:

- `PreparedPublicationReceiptV1` now has an exact tracked deny-unknown schema,
  canonical fields, claim-qualified prepared/receipt paths, frozen
  `preparedAt`, recovery epoch and record-preimage binding. Its no-replace hard-
  link publication has file fsync, both required parent-directory fsyncs, final
  no-follow readback, byte/hash comparison, and a receipt/tree/manifest/list/
  root equality gate before `publish_prepared`. The `publish_preparing` matrix
  covers partial-only, prepared-without-receipt, temp/torn/corrupt/leftover
  receipt, valid-before-CAS, stale/foreign/mismatched ownership, and claim-path
  collision. Recovery ownership is CAS-bound; quarantine first persists a
  path+inventory intent and then defines recovery after intent, every rename,
  parent fsync, and before the terminal abort CAS. Byte/state/result fault
  oracles and the Phase-7 implementation checklist cover the same boundaries.
- Both exact evidence ladders now invoke default failure preview followed by
  the separate `failure --apply`. Task help and the state contract agree that
  default failure alone advances `native -> failure -> publish_preparing ->
  publish_prepared`, repeated identical preview is a no-op, and apply accepts
  only `publish_prepared`. Before any `publishing` state write, apply holds the
  repo lock and validates schema/canonical receipt bytes, claim/recovery epoch,
  prepared record preimage, device/inode, tree, manifest, sorted entries,
  file-list/root, HEAD/tree, binding, four mode receipts, and build receipt.
  Any mismatch leaves the record byte-identical; direct apply from `native` is
  an explicit zero-write out-of-order case. Task help, Phase 7/8, and named
  contract tests are synchronized.

The full rescan found no unresolved P0/P1/P2 in Plan/public-private schema,
canonical digests, baseline and affected resources, one-confirmation admission,
preview side-effect spies, #35 narrow port and typed-disable behavior, #41
exact-SHA handoffs, Codex create/edit/switch all-entry cutover, Universal IPC,
artifact/candidate lock/ack/GC, the equality-grade DB inventory and maintenance
state machine, linear remote-effect recovery, terminal evidence hash graph and
post-unlink lookup, four-locale/a11y UI, Playwright/Chromium/mise contracts,
serialized ownership, command syntax, or the small-commit/conflict budget.
`task.json` correctly records architecture Round 23,
`phase=detailed_design_revision_8`, and `designFreeze=pending`; creating the
freeze receipt remains a subsequent main-thread action, not evidence from this
review.

Evidence boundary remains `code_audit`. Immediately before this append, HEAD
was `6859e9ce04970008f4cf8b3d4883b4f70316291a`, `git diff --check` was clean,
and the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` was empty. No test, build, browser,
server, renderer, or native-runtime command was run.

`DETAILED_DESIGN_REVIEW_ROUND_8=PASS`

### Freeze-prep byte attestation

`FREEZE_PREP_ATTESTATION=PASS` (`0 P0 / 0 P1 / 0 P2`).

Round-8 review recorded `detailed-design.md` SHA-256
`c179ddbf4228124a89f2da9b2ca1346e3790e67cabb0ecfec6dc5ea2251123a4`.
The current SHA-256 is
`7284e5a734ed9c9317fba2be9168c4be9518a1d6cb37ef8d996a646b5a198c83`.
Statically reversing only the current top status block and final Revision-8
status sentence to their reviewed pending text reproduces the recorded Round-8
SHA byte-for-byte. The two post-review deltas therefore only record Round-8
PASS while retaining the `DESIGN_FREEZE=PASS` gate; no contract byte changed
and no P0/P1/P2 is reopened.

`git diff --check` remains clean and the source/test/tooling diff from
`ca552f4d918cacc734f81f7efdef70619da139b8` remains empty. This attestation is
`code_audit` only; no test, build, browser, server, renderer, or native-runtime
command was run.
