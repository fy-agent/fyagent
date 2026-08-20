# Design: V2 Agent catalog, model quick setup, Y identity, and Codex integration

## 1. Design objective

Deliver the smallest complete vertical slices that turn the two empty V2
routes into truthful product surfaces without reopening the legacy renderer or
inventing unsupported third-party integrations.

```text
Rust catalog and existing native services
  -> registered Tauri commands
  -> V2 typed feature ports
  -> React Query authoritative reads / explicit mutations
  -> Agent master-detail and Models quick-setup pages
```

Application identity is a parallel deterministic asset pipeline:

```text
For You Gate vector geometry
  -> assets/fyagent.png (1024 RGBA canonical source)
  -> Tauri PNG / ICO / ICNS
  -> About icon + macOS template tray masks
  -> raster inventory + package/resource verification
```

## 2. Scope boundaries and decisions

### One task, parallel implementation streams

The catalog/backend ports, pages, fixtures, tests, and brand inventory share
cross-layer contracts and one final native acceptance gate, so they remain one
Trellis task. Phase 2 may use parallel workers with disjoint file ownership:

1. native catalog contract and Rust tests;
2. V2 ports/pages/unit/browser fixtures and tests;
3. official Agent assets plus FyAgent package-asset generation/checks.

The main session owns spec updates, integration, conflict resolution, HIL,
commits, remote-branch merge orchestration, and archive.

### Reuse backend behavior; do not import legacy frontend

The V2 renderer duplicates only the stable TypeScript wire DTOs needed at its
own boundary. It does not import legacy `src/lib`, components, hooks, i18n, or
CSS. Existing Rust commands remain the behavioral authority for WorkBuddy and
Provider configuration. A new Rust command is justified only for the versioned
Agent catalog because remote product contracts require one catalog SSOT.

### Honest capability tiers

- `pending_verification`: visible catalog candidate; browse/assisted actions
  only; no support-count or configuration-success claim.
- `manual_install`: the product is externally installed; FyAgent may observe or
  configure only the specifically declared existing contract.
- Per-action states independently express `available`, `assisted`,
  `not_supported`, or `pending_verification` with a reason. The frontend never
  derives capability from name, icon, URL, or a locally observed file.

### Bounded writes instead of a new generic apply framework

This task does not build the future generalized Issue #41 apply-job system.
It calls existing commands with their current safety contracts:

- WorkBuddy: revision-checked atomic write, backup, opaque overwrite token,
  credential-safe fetch and error DTOs.
- Claude/Codex: one quick-setup-specific backend apply envelope. Its per-app
  critical section covers authoritative reread, validated Provider persistence,
  current selection, live configuration, and failure recovery. The renderer
  must not emulate this transaction with separate save and switch IPC calls.

The UI labels these as bounded quick configuration and never claims rollback
or end-to-end model availability beyond the command's actual result.

## 3. Native Agent catalog contract

Add a focused module/command close to other read-only product metadata. The
serialized shape is conceptually:

```text
AgentCatalogResult {
  contractVersion: 1,
  reviewedAt: "2026-08-13",
  agents: AgentCatalogEntry[5]
}

AgentCatalogEntry {
  id, displayName, description, officialUrl,
  status,
  actions: { browse, observe, install, configure },
  evidenceLabel
}

AgentActionCapability { state, reason }
```

The command is payload-free, deterministic, non-networking, and non-secret.
Tests freeze the exact IDs/order/version, require HTTPS official URLs, assert
QoderWork/TRAE cannot configure/install automatically, and reject accidental
support promotion through snapshot/exact assertions. `generate_handler!`
registration is covered by compile/static tests.

## 4. V2 feature boundary

Extend the existing `FeaturePorts` rather than add a service layer:

- `catalog.get()` -> versioned native catalog;
- `providers.getAll/getCurrent/applyQuickSetupWithResult` for only
  `claude | codex`; the apply request is the smallest quick-setup wire and the
  Rust side derives or verifies the reserved Provider identity;
- `workbuddy.getStatus/getModelIds/fetchModels/saveModels`;
- existing `settings.openExternal` for all official links.

All direct `invoke` calls remain in the Tauri adapter. The normal browser
adapter reports native-only unavailability for authoritative Agent, Provider,
and WorkBuddy reads and rejects every write; deterministic success fixtures
exist only in the dedicated browser test harness.
Feature query keys add catalog, per-app Provider summaries, and WorkBuddy status
and model IDs. Credentials are mutation arguments only; they are never query
data or keys.

## 5. Agent directory data flow and UX

1. Fetch the catalog from the native port.
2. Select the first entry initially; selection is page-local UI state, not a
   second route/navigation state.
3. Render the left list from catalog order with accessible button semantics and
   `aria-current`.
4. Render right-side identity, status badge, capability reasons, and actions.
5. For QoderWork/TRAE, call only `openExternal(officialUrl)`.
6. For WorkBuddy, lazily query status when selected (or share a bounded cached
   query) and navigate to `/models?target=workbuddy`.
7. For Codex/Claude, query Provider count/current selection and navigate to the
   corresponding target. A read failure is `unknown/unavailable`, not absent.

The two-column grid collapses to one column at the existing mobile breakpoint.
Icons have fixed dimensions, local sources, useful alt text in detail identity,
and decorative treatment in duplicate list positions.

## 6. Models quick-setup data flow

### Target selection

The target order is exactly QoderWork CN, TRAE Work, WorkBuddy, Codex, and
Claude Code. The URL query may select a known non-secret target ID for deep
navigation from the Agent page; a missing or unknown value falls back to
QoderWork CN. All five selectors use the local Agent asset mapping. API keys and form content
never enter the URL. Each target owns separate component state and changing
target clears sensitive state.

### WorkBuddy

1. Query status and model IDs; keep only the non-secret response in cache.
2. Keep Base URL, API key, allow-no-key, selected/manual model IDs, and any
   frozen overwrite request in component memory.
3. Fetch through the existing bounded native command and display truncation.
4. Save with `expectedRevision`.
5. If confirmation is required, freeze the exact original request and retry it
   once only with the backend token after explicit confirmation.
6. Concurrent modification, expired/mismatched token, success, or failure
   clears sensitive/frozen state as appropriate and rereads authoritative
   status/model IDs.
7. On formal Windows builds, open the frozen Explorer profile and `.workbuddy`
   component with no-follow, relative-handle semantics. Keep the directory
   handle pinned through primary/backup/temp work and perform the final rename
   from an opened source handle. A parent junction, leaf reparse point, shell
   identity drift, or object-identity drift returns a generic failure with no
   target-tree write. Non-Windows persistence retains its existing contract.

### Claude Code and Codex

1. Read the native sanitized `{providers: {id,name}, currentId}` snapshot; the
   renderer never receives the generic Provider map.
2. Validate trimmed name, absolute HTTP(S) Base URL, nonempty key, and model ID
   in the renderer before mutation; backend remains authoritative.
3. Construct only the dedicated `{name, baseUrl, apiKey, modelId}` quick-setup
   request in a tested pure helper. Rust derives the stable reserved ID and all
   generic Provider fields; the renderer never submits a generic Provider.
4. Send one backend quick-setup apply request. Under one per-app critical
   section it rereads authoritative state, persists exactly this request's
   reserved Provider, selects it, synchronizes live configuration, and rolls
   back task-owned DB/current/live changes if a later step fails.
5. Compute this request's warning result before releasing the per-app guard.
   Reread the sanitized Provider state and only claim that the fixed reserved
   ID is active when its current ID matches. Do not claim that the reread proves
   this request's exact configuration bytes; a later serialized writer may have
   legitimately won. A failed or mismatched reread remains unconfirmed.
6. Surface Codex live-change and request-attributed warning codes. Clear the key after every
   terminal outcome and prevent concurrent submits.

No model availability request, login probe, token reuse, or automatic process
restart occurs.

### QoderWork and TRAE Work

Render short explanatory fields for model/endpoint notes in local transient
state, a clear "FyAgent 不会写入这些值" notice, and an official-settings action.
Do not render an API-key persistence or success workflow. Clearing/leaving the
page discards the fields.

## 7. Agent asset handling

- Download only the exact official URLs and verify the recorded SHA-256 before
  processing.
- Sanitize the Qoder SVG by verifying the already-recorded passive element
  inventory; preserve its paths/colors/viewBox. Store it as a local V2 asset.
- Store the exact TRAE 48px PNG or a single deterministic lossless catalog
  derivative. Do not auto-trace it to SVG or recolor it.
- Keep third-party assets separate from FyAgent application-brand sources.
  Research/provenance remains in the task that will be archived in Git.
- Update the raster/source inventory and exact-count tests only after reviewing
  the final file set and hashes.

## 8. FyAgent application asset pipeline

Use the existing high-resolution vector geometry (graphite tile plus blue/cyan
Y gate), not the 128px header PNG and not the historical RGB/white-corner
raster, to render the canonical 1024 RGBA PNG. Keep the approved path
`assets/fyagent.png` to avoid changing consumers.

Expand the canonical icon task so `--apply`:

1. validates the source dimensions/mode/transparency;
2. runs the Tauri icon generator for bundle PNG/ICO/ICNS outputs;
3. synchronizes `src/assets/icons/app-icon.png` byte-for-byte with the generated
   32px icon;
4. creates 24/48/72px black RGBA tray template masks from the Y silhouette with
   correct centering/alpha;
5. leaves third-party art untouched.

The check path validates the same inventory without writing. Supported raster
hashes are regenerated only through the repository-owned manifest workflow.
Windows NSIS continues to point to `icons/icon.ico`; package acceptance parses
the actual setup PE resource and compares frames with the canonical ICO.

## 9. Compatibility, rollback, and evidence boundaries

- No persisted schema migration is introduced. The quick-setup Provider uses
  existing Provider storage and WorkBuddy uses its existing document contract.
- Existing user Providers and unrelated WorkBuddy entries are preserved. HIL
  uses a reserved Provider ID or isolated test home and removes/restores only
  task-owned test state.
- Logical commits provide rollback points for planning, native/catalog,
  V2/pages, brand assets, review fixes, and Trellis archive. No destructive Git
  operation is needed.
- Browser tests prove responsive UI and IPC wiring only. Rust tests prove
  service/command contracts. A Windows native run proves this checkout on this
  machine only. A Windows bundle/PE check proves embedded resources, not install
  lifecycle or signing. macOS visual behavior, hosted CI, vendor permission,
  trademark similarity, signing, notarization, and release readiness remain
  unproven unless separately executed.

## 10. Remote Codex branch integration

The remote integration is an ordered merge, not a rebase or content copy:

```text
pinned origin fetch snapshot
  -> enumerate refs/remotes/origin/codex/* and record immutable tip IDs
  -> commit the verified local Agent/Models/Y baseline
  -> merge topology-related brand/history tips before overlapping V2 work
  -> resolve Prompt/Memory + Agent/Models into one six-page shell contract
  -> regenerate derived inventories
  -> require every pinned tip to be an ancestor of final HEAD
  -> rerun the complete post-merge gate
```

- Use full remote refs, including non-ASCII names. Already-contained tips are
  recorded as integrated without manufacturing empty commits; each other tip
  is merged with an attributable merge commit.
- Conflict resolution preserves both user-facing feature sets. Shared V2 shell,
  navigation, styles, specs, and tests describe the final six non-empty pages;
  security fixes and the atomic Provider apply contract are not replaced by an
  older branch snapshot.
- Generated raster/structure manifests are recomputed from the resolved tree.
  They are never resolved by taking an arbitrary side's digest list.
- Before committing a merge, scan newly introduced journals, task metadata,
  fixtures, and generated previews for credentials and absolute personal paths;
  redact local-user/worktree fingerprints without changing product evidence.
- Final validation and review run only after all pinned tips are ancestors.
  Pre-merge checks remain useful diagnostics but are not final acceptance.
