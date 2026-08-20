# Implementation plan: V2 Agent catalog, Models, Y identity, and Codex integration

## 0. Planning approval and activation gate

**Depends on:** this final PRD/design/plan and context manifests have been
reviewed; the final planning summary has been presented; the user sends a
subsequent explicit approval.

1. Validate task artifacts and read them top to bottom.
2. Run `task.py start` only after the approval gate.
3. Load `trellis-before-dev` and the curated frontend/backend/cross-layer/brand
   specifications before product edits.
4. Reconfirm branch `dev/laiyongjie`, active task identity, and working-tree
   scope; preserve unrelated user changes if any appeared.
5. Commit planning artifacts as a small local administrative commit.

**Completion definition:** task status is `in_progress`, required specs are
loaded, and implementation begins from an attributable diff.

## 1. Native catalog vertical slice

**Ownership:** one Trellis implementation worker owns the new Rust catalog
module/command, command registration, and focused Rust tests. It must not edit
V2 pages or icon assets and must accommodate concurrent changes without
reverting them.

- Define the versioned catalog DTO and exact five-entry data.
- Add the payload-free read-only Tauri command and register it.
- Freeze version/order/URLs/status/action invariants and non-secret response in
  focused tests.
- Run Rust formatting and the narrowest discoverable test filter.

**Focused validation**

```powershell
mise run rust:fmt:check
mise run rust:test agent_catalog
mise run rust:check
```

**Completion definition:** the command compiles, exact contract tests pass,
and QoderWork/TRAE cannot accidentally expose native install/config support.

## 2. V2 feature ports and pages

**Depends on:** the catalog wire shape from phase 1 (the worker may coordinate
against the agreed design while phase 1 is being implemented).

**Ownership:** one Trellis implementation worker owns `src/v2/**` and
`tests/v2/**`; another may own `tests/v2-browser/**` after shared fixture
coordination. Neither edits Rust catalog or package icon pipeline files.

- Extend V2 types, query keys, feature ports, Tauri adapter, and browser adapter
  for catalog, Claude/Codex Provider operations, and WorkBuddy operations.
- Add tested pure Provider quick-setup builders with a stable reserved ID and
  exact Claude/Codex wire shapes.
- Implement Agent master/detail UI, lazy/bounded observations, official-link
  actions, and `/models?target=...` navigation.
- Implement Models target selection in the exact QoderWork, TRAE Work,
  WorkBuddy, Codex, Claude Code order; make QoderWork the missing/unknown-target
  default and render all five local icons.
- Implement WorkBuddy fetch/save/overwrite/revision flow, Claude/Codex atomic
  quick-setup apply/reread flow, warning/unconfirmed/failure
  states, and QoderWork/TRAE controlled guidance.
- On Windows, bind WorkBuddy reads and the complete save transaction to the
  frozen Explorer profile and one verified no-follow directory handle; reject
  parent/leaf reparse objects and never return to a full string path for rename.
- Add responsive namespaced styles and reuse current V2 primitives.
- Update browser fixtures and positive `open_external` recording. Preserve
  native-only rejection in normal browser adapter tests.
- Update router/shell tests so the first four pages are non-empty and only
  Prompts/Memory remain empty.

**Focused validation**

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

**Completion definition:** both new routes are usable at all four viewport
sizes, exact IPC calls/payloads and security lifecycle are covered, and existing
Skills/MCP/shell architecture tests remain green.

## 3. Official QoderWork/TRAE catalog assets

**Ownership:** one worker owns only V2 Agent assets, provenance checks, and
their raster/source inventory/test changes. It must not edit page logic except
an agreed asset mapping file.

- Retrieve the official Qoder SVG and TRAE PNG from research URLs.
- Verify original type/dimensions/SHA-256 before any processing.
- Inspect/sanitize the SVG without changing paths/colors and store both assets
  in the V2 shared asset boundary.
- Connect all five catalog entries to local assets, reusing reviewed existing
  WorkBuddy/OpenAI/Claude sources through V2-owned copies rather than legacy
  imports.
- Update the supported asset manifest and focused decode/resolution tests.

**Focused validation**

```powershell
mise run supported-platform:check
mise run test:v2
mise run build:renderer
git diff --check
```

**Completion definition:** five local catalog assets resolve, official bytes
match recorded provenance, no third-party art becomes FyAgent identity, and the
asset inventory is exact.

## 4. Y application and installer asset pipeline

**Ownership:** a dedicated worker owns the application-brand vector/source,
icon task/check implementation, generated Tauri/About/tray assets, raster
manifest, and icon/release contract tests. It must not edit Agent pages.

- Materialize the high-resolution Y app-icon vector geometry in the maintained
  brand source boundary.
- Deterministically render a 1024x1024 RGBA canonical `assets/fyagent.png` with
  transparent canvas/corners.
- Extend `assets:icons -- --apply` to synchronize Tauri, About, and template
  tray consumers; extend `assets:icons:check` to validate them.
- Run the generator once, inspect all binary diffs, regenerate the supported
  raster inventory through its canonical command, and require a second
  generation to be byte-stable.
- Update exact icon/frame/mask/About/NSIS/release tests and relevant specs.

**Focused validation**

```powershell
mise run assets:icons -- --source assets/fyagent.png --apply
mise run assets:icons:check
mise run supported-platform:check
mise run test:unit -- tests/windowsSetupIcon.test.ts tests/windowsNsisContract.test.ts tests/remainingPlatformSurface.test.ts tests/releaseWorkflow.test.ts
mise run release:check
```

**Completion definition:** all documented identity consumers share the Y
geometry, generated output is stable, tray and About contracts pass, and the
Windows canonical ICO remains release-verifiable.

## 5. Spec synchronization and cross-layer integration

**Depends on:** phases 1-4 interfaces are stable.

- Update `.trellis/spec/frontend/v2-shell.md` and the dedicated Agent/Models
  contract for the page state at this task's feature baseline. After remote
  integration, reconcile the shell again to the final six-non-empty-page state.
- Add or update a focused durable Agent catalog/model quick-setup spec if the
  behavior would otherwise overload the shell note; update spec indexes.
- Update application-brand asset spec only for durable generator/check behavior
  learned during implementation; do not record transient commands/results.
- Inspect full diffs for wire mismatch, credential leakage, stale static
  capability duplication, asset provenance loss, and unrelated changes.

**Focused validation**

```powershell
mise run tasks:validate
mise run check:contracts
mise run test:v2
mise run assets:icons:check
git diff --check
```

**Completion definition:** executable tests, code, and current Trellis specs
describe one consistent cross-layer contract.

## 6. Commit feature baseline and integrate remote Codex branches

**Depends on:** phases 1-5 are stable and their focused checks pass.

1. Review and commit the local Agent/Models/backend/security/Y-icon/spec write
   set in attributable commits so a failed merge can be diagnosed without
   destructive reset.
2. Use the immutable fetch snapshot in
   `research/remote-codex-merge-inventory.md`. For each ref, record the pinned
   tip, merge base, unique commits, ancestry, overlap, and conflict paths.
3. Merge non-contained tips in deterministic topology-aware order with full
   remote ref names and attributable merge commits. Do not rebase, squash,
   force-update, delete, rename, or push a remote branch.
4. Resolve overlapping V2 shell/spec/test content into one six-page contract:
   Agent/Models retain this task's native-backed behavior; Prompt/Memory retain
   the integrated branch's behavior; Skills/MCP remain regression-safe.
5. Preserve the latest credential and Provider-atomicity fixes, redact newly
   introduced absolute personal paths, and regenerate derived manifests from
   the resolved tree.
6. Require `git merge-base --is-ancestor <tip> HEAD` for every pinned tip.
   Later remote branch creation or movement is outside this user-frozen scope.

**Focused validation**

```powershell
Get-Content .trellis/tasks/08-13-v2-agent-catalog-model-setup/research/remote-codex-merge-inventory.md
git merge-base --is-ancestor <each-pinned-tip> HEAD
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
git diff --check
```

**Completion definition:** every fetched remote Codex tip is an ancestor of the
local branch, both feature sets survive conflict resolution, no personal path
or credential is newly exposed, no remote state changes, and the post-merge
tree is ready for one final full gate.

## 7. Native Windows HIL and bundle evidence

**Depends on:** focused static/unit/browser checks pass.

1. Launch the real current-host Tauri/WebView2 application through the
   canonical `mise run dev` path.
2. Verify the first four routes are reachable/non-empty; exercise Agent
   selection, real catalog read, QoderWork/TRAE official-link launch, WorkBuddy
   observation, and a controlled task-owned Provider quick-setup/reread path.
3. Avoid overwriting unrelated user config. Use a reserved Provider ID and
   remove/restore only task-owned HIL state; use test-home isolation where the
   existing command contract supports it.
4. In disposable directories, exercise normal WorkBuddy create/update/backup,
   parent-junction rejection, leaf-reparse rejection, and target-tree zero-write
   assertions. A genuine two-account Explorer/UAC run is separate evidence and
   remains explicitly unverified when this host cannot provide it.
5. Inspect window/taskbar/About identity visually on the current Windows host.
6. Build the real Windows bundle and locate the generated setup artifact.
7. Parse the setup PE icon resource and compare it to canonical `icon.ico`.

**Validation**

```powershell
mise run system:check -- --json
mise run dev
mise run build
node scripts/release/verify-windows-setup-icon.mjs <actual-setup.exe> src-tauri/icons/icon.ico
```

**Completion definition:** real Windows runtime actions and package icon are
observed and recorded; any action that cannot be safely exercised remains an
explicit unverified item rather than a mock-derived claim.

## 8. Full quality review and final local gate

1. Run `trellis-check` with code-quality, cross-layer, UI/accessibility,
   security/credential, testing/release, and docs/spec perspectives. Resolve
   every verified in-scope finding and rerun its focused checks.
2. Run `trellis-update-spec` for durable contracts after code stabilizes.
3. Execute the final local matrix:

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check

mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test

mise run assets:icons:check
mise run supported-platform:check
mise run release:check
mise run check
mise run build

mise run supported-platform:check
mise run check
git diff --check
```

4. Review `git diff --stat`, `git diff --name-status`, all binary/generated
   changes, commits, and `git status --short` for scope and user-owned edits.

**Completion definition:** every local gate passes or an acceptance criterion
is honestly marked unverified; no known critical/security finding remains.

## 9. Commits, Trellis archive, and clean-tree proof

1. Create reviewable local commits, preferably:
   - `chore(trellis): plan v2 agent and model pages`
   - `feat(agent): add the versioned agent catalog`
   - `feat(v2): add agent catalog and model quick setup`
   - `build(brand): adopt the For You Gate app icon`
   - a narrow review-fix commit only when it cannot safely be folded into its
     owning change without obscuring evidence.
2. Use `trellis-finish-work`, validate both manifests, record actual validation
   evidence in the journal, and archive through the canonical task script.
3. Commit the archive/journal administration locally. Do not push.
4. Rerun the post-archive task/contracts check as applicable, then require:

```powershell
git status --short
```

to produce no output.

**Completion definition:** implementation and evidence are committed, the task
is archived, all fetched `origin/codex/*` tips remain ancestors, no remote state
changed, and the local worktree is clean.

## Rollback and stopping conditions

- Each feature/brand commit is a rollback point; do not use destructive reset
  or discard user changes.
- Stop a configuration mutation immediately if the current revision/provider
  differs from the task-owned expected state. Reread and surface the conflict.
- Stop icon adoption if the generated source is not 1024 RGBA with transparent
  corners, any documented consumer is not synchronized, output is not
  deterministic, or canonical ICO/package verification fails.
- Do not downgrade or remove safety tests to make a deadline. If macOS HIL,
  vendor permission, trademark review, hosted CI, signing, or notarization is
  unavailable, report it as residual; it does not block the explicitly scoped
  local code/package delivery but does block broader approval/release claims.
