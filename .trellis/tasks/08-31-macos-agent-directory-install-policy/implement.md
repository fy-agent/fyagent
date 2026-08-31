# Implementation Plan

## 0. Execution contract

- This task now runs in parallel with `08-31-macos-privileged-application-commit-helper`.
- Obey `research/parallel-with-helper.md` file ownership. Do not edit helper-owned paths.
- macOS is the implementation/HIL platform. Product-level action/surface removals stay consistent across shared contracts, but no Windows desktop installer is added.
- Production `/Applications` stays `authorization_required` until the helper task’s signed HIL flag is on. Do not implement a second helper.

## Phase 0 — Re-establish the implementation baseline

### Work

1. Confirm the predecessor task is complete or has handed off stable interfaces.
2. Record the current source SHA and inspect only the final current versions of:
   - Agent lifecycle policy/surface types;
   - inventory and action dispatcher;
   - managed desktop sources;
   - shared downloader/job/DMG transaction;
   - Agent directory scan/projection;
   - catalog and strict wire parsers.
3. Compare the final code against:
   - `research/current-implementation-audit.md`;
   - `research/execution-context.md`.
4. Update this task’s research/implementation notes if file ownership changed, without changing the user-visible requirements.
5. Verify the helper task’s current status. System `/Applications` acceptance remains gated if helper implementation/HIL is incomplete.
6. Re-fetch current Claude fixed mirror manifest and current OpenCode official latest release metadata; treat recorded versions as stale evidence, not constants.

### Exit criteria

- No assumptions depend on another task’s uncommitted intermediate shape.
- One concrete backend policy owner and one frontend order owner are identified.
- No duplicate downloader/updater/helper plan is introduced.

## Phase 1 — Add one backend lifecycle policy owner

### Work

1. Create or consolidate a crate-private policy table/function for product + surface + allowed actions + source kind.
2. Encode the final matrix:
   - QoderWork/TRAE Work/WorkBuddy: desktop install+launch, no update;
   - Claude/OpenCode: desktop install+update+launch, no Agent CLI surface;
   - Grok/Codex: preserve existing owner behavior.
3. Make Rust `legal_surfaces` and default surface consume the policy.
4. Add `action_not_supported` to the closed reason enum and TypeScript parser/copy, if no exact semantic code exists at implementation time.
5. Validate action policy before target lookup, network or side effects.
6. Add exhaustive matrix tests and zero-side-effect rejection tests.

### Exit criteria

- One backend owner answers every lifecycle policy question.
- A direct update request for any of the three install-only products is rejected before transport/filesystem/helper calls.
- Removed Claude/OpenCode CLI requests fail with `surface_not_supported`.

### Rollback point

This slice should be independently reversible before catalog/source/UI work. If the policy abstraction weakens existing safety ordering or creates cycles, retain a private facade around the cohesive existing owner rather than splitting it further.

## Phase 2 — Apply install-only policy to readiness and inventory

### Work

1. Reorder desktop readiness so inventory/policy decide whether a source request is necessary.
2. For installed QoderWork/TRAE Work/WorkBuddy:
   - do not resolve remote metadata for update comparison;
   - project `updateState=unavailable`;
   - omit update release/action;
   - retain launch/configuration state.
3. For confirmed not-installed instances:
   - retain source resolution and install target projection;
   - retain current source validation and fixed endpoints.
4. Apply policy to candidate/destination action eligibility without deleting platform evidence.
5. Ensure unknown/ambiguous discovery does not expose install/update simply because policy permits install.
6. Add fake-source call-count tests and inventory/readiness contract fixtures.

### Exit criteria

- Installed domestic products generate zero update-source calls.
- Fresh install remains functional.
- Target picker and renderer cannot discover an eligible update target for these products.

## Phase 3 — Converge Claude/OpenCode Agent surfaces and catalog links

### Work

1. Change Rust and TypeScript legal surface maps:
   - Claude -> desktop only;
   - OpenCode -> desktop only.
2. Remove OpenCode dual-surface readiness aggregation and use the normal desktop projection.
3. Route Claude readiness/inventory/action to managed desktop instead of CLI Tooling.
4. Remove Claude/OpenCode Agent CLI action routing; retain underlying Tooling APIs where other domains still use them.
5. Update official links:
   - Claude Desktop official download;
   - OpenCode product + desktop download;
   - no CLI link for either product on Agent surfaces.
6. Bump the owning catalog/readiness/action contract versions once as required.
7. Update strict TypeScript wire parsers and exact fixtures.
8. Verify Provider, Skills, MCP, models and session IDs/assignments remain unchanged.

### Exit criteria

- Agent lifecycle has exactly one desktop component for Claude and OpenCode.
- Old CLI-bearing requests and fixtures are rejected.
- No stable non-lifecycle product domain was accidentally renamed or deleted.

## Phase 4 — Implement the stable directory-order projection

### Work

1. Add the closed domestic/standard priority field to the existing shared product directory metadata.
2. Add one pure ordering helper beside the existing Agent directory scan projection.
3. Implement bucket classification:
   - installed domestic;
   - installed other;
   - unresolved;
   - confirmed not installed.
4. Treat `installed_not_runnable` as installed.
5. Treat current scan failure as unresolved for ordering even when stale readiness remains displayed/configurable.
6. Keep canonical index as the only tie-breaker.
7. Integrate a committed-order lifecycle:
   - canonical during first scan;
   - update once after completion;
   - freeze during rescan;
   - update after authoritative action reread when no scan is active.
8. Keep `entry.id` keys and verify keyboard focus survives a reorder.
9. Add pure unit tests plus AgentDirectory integration tests.

### Exit criteria

- The list does not jump as individual scan requests settle.
- Completed order exactly matches the PRD matrix.
- The input catalog array/query data is not mutated.

## Phase 5 — Add the Claude Desktop fixed source adapter

### Work

1. Reuse or minimally extract the Codex fixed-manifest metadata transport/retry/cache/cancellation owner.
2. Add Claude-specific private schema parsing for fixed mirror manifest v2:
   - bounded body;
   - exact universal macOS branch;
   - exact official redirect identity;
   - bounded consistent version;
   - optional size hint.
3. Add code-owned endpoint kinds for:
   - manifest;
   - macOS universal DMG.
4. Ignore remote URL/hash/filename/publication fields as executable admission or download capability.
5. Add Claude managed desktop product policy:
   - Bundle ID `com.anthropic.claudefordesktop`;
   - Info.plist version;
   - exact version equivalence;
   - canonical `Claude.app` target policy where the target owner needs a basename.
6. Route download through the existing streamed artifact/job owner.
7. Route install/update through the existing managed DMG transaction and target authority.
8. Add fixtures based on current sanitized manifest and mounted bundle metadata; do not pin the observed version in production code.
9. Add source failure -> official page fallback and privacy-safe diagnostics.

### Exit criteria

- Claude source code adds no second HTTP stack or downloader.
- Renderer cannot submit a URL or mirror.
- Fresh install/update reaches the existing DMG transaction with a frozen release descriptor.
- Mounted bundle identity/version mismatch fails before success.

### Stop condition

If the reviewed mirror changes ownership, license, fixed endpoints, schema provenance or no longer serves unchanged official installer bytes, disable the managed source and keep only official-page fallback. Do not substitute an unknown public proxy.

## Phase 6 — Refine OpenCode Desktop update availability

### Work

1. Reuse the existing fixed-repository GitHub latest-version owner or extract one narrow backend shared owner.
2. Bind it only to `anomalyco/opencode` latest stable tag/version.
3. Keep current code-owned architecture-specific stable DMG endpoints as artifact transport.
4. Include frozen version in the opaque release descriptor.
5. Compare installed Info.plist version and remote stable version for `updateState`.
6. Force-refresh metadata immediately before action.
7. Require the mounted app version to match the frozen release; map drift to refresh/retry.
8. Prove no OpenCode Electron updater executable/API is invoked.
9. Add arm64/x64 selection, version drift and source error tests.

### Exit criteria

- A stale OpenCode installation exposes one-click update.
- Up-to-date state is honest when version metadata is available.
- All execution still flows through the shared FyAgent job/DMG/helper owners.

## Phase 7 — Frontend lifecycle and copy integration

### Work

1. Remove Claude/OpenCode CLI component rows and stale CLI labels from Agent lifecycle UI.
2. Label the Claude physical component `Claude Desktop` while retaining the stable product/configuration identity.
3. Verify the generic primary-action projection uses backend `allowedActions` without product-specific hide logic.
4. Verify installed QoderWork/TRAE Work/WorkBuddy show no update status/action.
5. Integrate Claude/OpenCode install/update/launch controls.
6. Reuse the shared progress/speed/terminal-state projection delivered by the predecessor task.
7. Add concise source/region-neutral errors and official-page fallback.
8. Update accessibility labels, keyboard tests and screenshots/visual assertions only where the repository currently owns them.

### Exit criteria

- Visible actions exactly match backend policy.
- No CLI install wording remains for Claude/OpenCode Agent lifecycle.
- No raw float, path, internal URL or diagnostic prose leaks into primary UI.

## Phase 8 — `/Applications` integration and macOS HIL

### Preconditions

- The separate privileged helper task has implemented and passed its own unit/integration gates.
- A signed/notarized build pipeline can include the helper.

### Work

1. Connect Claude/OpenCode existing-target/fresh-system transactions through the single `MacSystemCommitPort`.
2. Verify no product-specific elevation branch exists.
3. Run signed macOS HIL:
   - Claude fresh `/Applications` install;
   - Claude in-place update;
   - OpenCode in-place update;
   - existing `~/Applications` update remains in place;
   - authorization cancel;
   - app running;
   - post-commit verification failure and rollback;
   - helper unavailable/mismatch;
   - launch after explicit click only.
4. Record actual versions, source categories and sanitized terminal results in task evidence.

### Exit criteria

- System install/update is not marked complete from mocks or unsigned builds.
- No silent user-directory fallback occurs.
- Old app remains runnable after failed update or reports `recovery_required` with precise recovery evidence.

## Phase 9 — Full review, specs and closeout

### Review round A — product/UX

- Sorting matches user intent and is stable.
- Domestic products are install-only.
- Claude/OpenCode are desktop-only lifecycle surfaces.
- Region/source copy is honest.

### Review round B — architecture/reuse

- One backend policy owner.
- One frontend sort metadata/projection owner.
- Existing source/download/job/DMG/helper owners reused.
- No duplicated ID/source/action tables beyond required cross-language wire contracts.

### Review round C — security/supply chain

- No arbitrary URL/path/command/bypass.
- Fixed Claude mirror endpoints and exact schema branch.
- OpenCode fixed repository/artifact endpoints.
- Removed actions rejected before side effects.
- Logs/IPC remain redacted and bounded.

### Review round D — compatibility/release

- Shared contract bumps are atomic.
- Non-lifecycle Claude/OpenCode domains remain intact.
- Windows adds no new desktop installer and passes existing gates except intentional shared policy changes.
- Signed helper/app packaging evidence is complete where claimed.

### Spec updates

Update the owning backend/frontend specs only after behavior is proven. Keep one-time versions, current release IDs and research commit hashes in this task.

### Closeout

1. Run focused tests after each phase.
2. Run the repository’s current full check tasks.
3. Run task prearchive validation with the exact active task exclusion required by current Trellis workflow.
4. Resolve all review findings.
5. Commit/PR/archive only under explicit user instructions and repository governance.

## Suggested validation commands

The live `mise` task definitions are authoritative. Expected gates include:

```bash
python ./.trellis/scripts/task.py validate .trellis/tasks/08-31-macos-agent-directory-install-policy
mise run check:frontend
mise run check:backend
mise run check:contracts
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run test:unit -- tests/remainingPlatformSurface.test.ts
```

Focused tests should cover:

```text
agent_install policy/readiness/inventory/action
managed desktop source parsers
Codex shared downloader and macOS DMG regression
Agent catalog strict contract
Agent directory scan/order projection
Agent lifecycle actions and ports
```

Native HIL commands and artifact paths must be recorded when execution begins; do not hard-code one developer machine path into this task or specs.

## Final implementation checklist

### Product policy

- [ ] QoderWork/TRAE Work/WorkBuddy install-only is backend enforced.
- [ ] Claude/OpenCode Agent lifecycle is desktop-only.
- [ ] Non-lifecycle product identities remain intact.

### Ordering

- [ ] Initial scan stays canonical.
- [ ] Completed order uses four stable buckets.
- [ ] Rescan freezes order.
- [ ] Stale failure remains unresolved for ordering.

### Sources and execution

- [ ] Claude fixed mirror adapter is bounded and URL-free on IPC.
- [ ] OpenCode update metadata reuses a fixed GitHub owner.
- [ ] Shared downloader/job/DMG/helper owners are reused.
- [ ] No upstream updater or mirror shell script is embedded.

### Safety and evidence

- [ ] Unsupported actions have zero side effects.
- [ ] Install/update success requires authoritative readback.
- [ ] `/Applications` claims have signed HIL.
- [ ] Mainland download wording makes no service-availability claim.
- [ ] Windows desktop work remains deferred.
