# Build the V2 Agent catalog, model quick setup, and integrated Codex branches

## Goal

Turn the production V2 renderer's empty `Agent 目录` and `模型` routes into a
small, honest, command-backed first delivery. Users can browse QoderWork,
TRAE Work, WorkBuddy, Codex, and Claude Code, follow official assisted paths
for products without a stable integration contract, and perform bounded model
configuration for the products already supported by FyAgent's native backend.
The same delivery adopts the existing For You Gate Y mark across the
application and installer asset chain and keeps the first four V2 routes
(`Agent 目录`, `模型`, `Skills`, and `MCP`) regression-tested as reachable,
non-empty, and connected to a real native action or an explicit controlled
degradation. As a final integration requirement, every current remote
`origin/codex/*` branch is merged into the local `dev/laiyongjie` branch. The
combined result preserves the Agent/Models contracts from this task and the
Prompt/Memory functionality brought in by the remote frontend branch, then
passes one post-merge validation and archival gate without pushing any remote
state.

## Background and confirmed facts

- `src/index.html` loads `src/v2/main.tsx`; V2 is the current production
  renderer, not a prototype-only entrypoint.
- At the pre-implementation baseline observed on 2026-08-13, `/agents` and
  `/models` existed in the router but returned `null`; `/skills` and `/mcp`
  already had real Tauri-backed pages.
- V2 is deliberately isolated from the legacy renderer. V2 code cannot import
  legacy components, hooks, API facades, translations, or styles; native calls
  remain below `src/v2/shared/platform/tauri/**`.
- The existing backend already owns safe WorkBuddy status/model discovery and
  persistence commands, Provider CRUD/switch commands for Claude and Codex,
  Codex live-change/warning results, and the validated `open_external` command.
- WorkBuddy is an independent configuration domain, not a Provider `AppType`.
  Its revision, overwrite capability, bounded model fetch, atomic persistence,
  and credential-isolation contract must remain intact.
- Remote Issue #101 is the current group-level product decision source. It
  requires one versioned Agent catalog, honest capability states, real backend
  actions or controlled degradation, and no promotion of a candidate into a
  supported integration without evidence.
- QoderWork and TRAE Work have official product and icon sources, but no stable
  public local configuration schema, BYOK contract, subscription-token relay,
  or redistribution permission was found. They therefore remain
  `pending_verification` entries with official assisted paths only.
- Official icon inputs and remote evidence are recorded in
  `research/brand-assets-and-issues.md`. The Qoder family mark is the official
  square mark available for QoderWork; the TRAE CN and international sites
  expose byte-identical current favicon art. Neither vendor publishes a clear
  brand-use license, so the marks are used unmodified, at small size, only to
  identify their own catalog entries and never as FyAgent identity.
- The canonical package source path is `assets/fyagent.png`; Tauri/NSIS, About,
  and macOS tray consumers form one application-brand asset contract. At the
  pre-implementation baseline the V2 header already used the Y mark while the
  package consumers still used the prior identity. The implementation must
  regenerate all of those consumers from the audited Y vector rather than
  treating the historical baseline as current acceptance evidence.
- The For You Gate Y mark is still a `concept_candidate` in remote Issue #93.
  This task implements the user's explicit code/package switch, but does not
  claim that trademark, similarity, or cross-platform native visual approval
  has been completed.
- The user expanded the approved scope on 2026-08-13 to include every remote
  `origin/codex/*` branch, then explicitly froze "current" to the fetch snapshot
  recorded in `research/remote-codex-merge-inventory.md`. Completion is proved
  by ancestry of every pinned tip, not by chasing later ref movement or by
  creating empty merge commits for tips already contained in `HEAD`.

## Requirements

### R1. Versioned Agent catalog backend contract

- Add a small, non-secret native Agent catalog command whose response has an
  explicit contract version and exactly these entries in this order:
  QoderWork, TRAE Work, WorkBuddy, Codex, Claude Code.
- Each entry exposes a stable ID, display name, short description, official
  HTTP(S) URL, reviewed date/evidence label, overall status, and per-action
  states for browse, observe, install, and configure.
- QoderWork and TRAE Work are `pending_verification`; they expose browse and
  assisted/manual actions only. Their presence and icons must not be counted or
  worded as full FyAgent support.
- WorkBuddy, Codex, and Claude Code expose only actions backed by current
  repository behavior. The catalog itself contains no credential, local path,
  process ID, installation claim, model-availability claim, or login state.
- Register and test the command. V2 consumes the command through a typed port;
  it does not maintain a second hard-coded capability matrix.

### R2. Agent directory page

- Render a responsive two-column master/detail page: Agent selector on the
  left and the selected Agent's detailed status and actions on the right.
- Show the processed QoderWork, TRAE Work, WorkBuddy, Codex, and Claude Code
  icons without recoloring or implying endorsement. Preserve provenance in
  the task research and raster inventory.
- All five entries show their catalog status and explicit capability boundary.
  QoderWork/TRAE actions open only their official sites through the validated
  Tauri external-link command; no installer is cached, mirrored, or launched.
- WorkBuddy displays native configuration observation from
  `get_workbuddy_status` and can navigate to its model target. Codex and Claude
  Code display a bounded Provider summary/current selection and can navigate to
  their model targets. Failed/unavailable observations remain distinct from
  `not_observed` and never become a false "not installed" or "verified" state.
- Selection, keyboard focus, `aria-current`, loading/error/empty/disabled
  states, and responsive behavior are testable at the four maintained V2
  viewport sizes.

### R3. Models quick-setup page

- Present five clearly separated targets in this exact order: QoderWork CN,
  TRAE Work, WorkBuddy, Codex, and Claude Code. Missing or unknown `target`
  defaults to QoderWork CN, and all five selectors render the reviewed local
  Agent icon. Opening `/models` performs only lightweight reads; every write
  requires an explicit user action.
- WorkBuddy reuses the existing dedicated commands for status, model IDs,
  bounded `/models` fetch, save, revision drift, and one-time overwrite
  confirmation. API keys remain only in component memory and one Tauri request,
  clear on target change/unmount and after terminal success/failure, and never
  enter query keys/cache, URL state, renderer/shared storage, logs, toasts, or
  error text. The explicitly requested protected WorkBuddy configuration file
  is the intended credential destination.
- On formal Windows builds, every WorkBuddy status/read/save/backup/temp/replace
  operation is anchored to the frozen Explorer user's profile and an opened
  `.workbuddy` directory handle. Parent junctions and leaf reparse points fail
  closed before data is returned or any file is created; the implementation
  must not fall back to a checked string path for the final replace.
- Codex and Claude Code offer one bounded quick-configuration form for name,
  Base URL, API key, and model ID. One quick-setup-specific backend apply
  operation owns the per-app critical section, authoritative reread, reserved
  Provider persistence, current selection, live configuration, and failure
  recovery. The renderer must not compose separate save/switch IPC calls.
  Repeat clicks are locked. A sanitized reread may confirm only that the fixed
  Quick Setup Provider ID is currently active; it does not prove that the
  current bytes still belong to this request after another serialized writer.
  An unconfirmed/failed outcome is not collapsed into success.
- Codex surfaces `liveConfigChanged` and stable warning codes. Saving a Provider
  and restarting/using Codex remain separate outcomes; this task does not port
  the full legacy restart or Codex Desktop installer coordinator.
- QoderWork and TRAE Work may show simple, transient non-secret model/endpoint
  note fields and official setup guidance. FyAgent does not persist or submit
  those values and the action is named "打开官方设置" (or equivalent), never
  "一键配置成功".
- Normal browser adapters report native-only unavailability for authoritative
  reads and reject writes. Deterministic success fixtures exist only in focused
  tests, and browser/mock success is never described as a native configuration
  result.

### R4. V2 shell, visual, and architecture integration

- Preserve the six-route order, Hash Router ownership, single selected lens,
  native chrome boundary, and V2 downward dependency rules.
- Update the V2 shell contract and executable tests so Agents, Models, Skills,
  and MCP remain non-empty command-backed/controlled-degradation pages. After
  integrating the remote Prompt/Memory frontend branch, Prompts and Memory also
  remain reachable and non-empty under their own frozen prototype contracts.
- Extend the existing V2 feature types, ports, queries, Tauri adapter, browser
  adapter, fixtures, primitives, and namespaced styles rather than importing
  legacy code or adding a parallel service/store abstraction.
- Preserve the existing Skills and MCP behavior; only update their shared
  fixtures/tests when required by the expanded port contract.

### R5. Official Agent icon assets

- Retrieve QoderWork and TRAE art only from the official URLs recorded in
  research. Validate response type, dimensions, and known SHA-256 before use.
- Prefer the official Qoder SVG and the official TRAE PNG. Sanitize/normalize
  only what is necessary for safe local rendering; do not redraw, recolor, add
  effects, or synthesize a different vendor mark.
- Store the final small catalog assets under the V2 shared asset boundary,
  include them in the supported raster/source inventory as applicable, and
  test that every catalog entry resolves to a local asset.

### R6. FyAgent Y application and installer identity

- Adopt the existing For You Gate Y geometry as the application icon while
  retaining `assets/fyagent.png` as the canonical 1024x1024 RGBA source path.
  Do not upscale the 128px header raster or use the historical RGB file with a
  white corner background.
- Generate Tauri PNG/ICO/ICNS assets through the canonical repository path and
  synchronize the About icon plus the macOS 1x/2x/3x black RGBA tray templates
  from the same approved geometry.
- Close the current generator/check gap: the `--apply` path must update all
  documented consumers, and the check must validate source metadata,
  About/32px byte equality, tray sizes/mask constraints, and required outputs.
- Update the supported-platform raster inventory deliberately. Preserve the
  existing Windows installer/uninstaller canonical ICO and release verification
  contract, then prove a real locally built setup executable embeds the new
  canonical ICO.
- Record the unresolved Issue #93 similarity/trademark and non-Windows native
  visual gates as residual risk. A successful asset build is not represented
  as legal approval, notarization, or macOS HIL.

### R7. Verification, commits, archive, and clean tree

- Add focused Rust, V2 unit, V2 browser, port/fixture, security, icon, raster,
  and existing-route regression tests that match the claims above.
- Run the native Windows application, not only Vite/browser preview, and verify
  the two new pages plus at least one real backend read and one reversible
  configuration flow using controlled test/provider data. Do not overwrite an
  unrelated user Provider or WorkBuddy entry during HIL.
- Run the full validation matrix in `implement.md`; clearly separate mock,
  static/build, real Windows runtime, package-resource, and unexecuted macOS
  evidence.
- Commit all product, test, spec, generated asset, task, journal, and archive
  changes locally in reviewable commits. Do not push or update remote Issues.
- Archive this Trellis task through the canonical workflow and require
  `git status --short` to be empty at completion.

### R8. Integrate every remote Codex branch

- Use the approved `origin` fetch snapshot recorded in
  `research/remote-codex-merge-inventory.md`. Record each full ref, pinned tip,
  merge base, unique commits, overlap, and likely conflict paths before
  mutation; later ref creation/movement is outside this fixed scope.
- Start from a committed, verified Agent/Models/Y-icon baseline. Do not merge
  into the dirty implementation worktree and do not discard or overwrite
  either side to resolve conflicts.
- Merge every fetched tip into local `dev/laiyongjie` in a deterministic,
  topology-aware order. Preserve both branches' user-visible functionality;
  resolve shared V2 shell/spec/test files against the final six-page product
  contract, and regenerate derived manifests from the resolved file set rather
  than choosing stale manifest bytes from either side.
- Treat a remote tip as integrated only when
  `git merge-base --is-ancestor <tip> HEAD` succeeds. A tip already contained
  in `HEAD` needs no artificial empty merge commit; every non-contained branch
  receives an attributable merge commit.
- Re-run code, security, docs, V2/browser, Rust, application-brand, release,
  native Windows, and fresh bundle checks after the final merge. Pre-merge
  results are diagnostic evidence only and cannot establish final acceptance.
- Do not push, delete, rename, rebase, force-update, or otherwise mutate any
  remote branch.

## Acceptance criteria

- [x] The native catalog command returns contract version 1 and exactly the
  five requested Agents in the required order with tested, honest action
  states; QoderWork and TRAE remain `pending_verification`.
- [x] `#/agents` renders a keyboard-accessible left selector and right detail
  view for all five Agents, uses the requested official icons, and opens only
  official QoderWork/TRAE HTTP(S) destinations through native `open_external`.
- [x] Agent details show real WorkBuddy and Claude/Codex backend observations
  or an explicit controlled degradation; no UI copy upgrades absence of
  evidence to installation, authentication, model, or support success.
- [x] `#/models` performs real WorkBuddy model discovery/save and real
  Claude/Codex atomic quick-setup apply operations with pending, success,
  warning, failure, concurrency, overwrite, rollback, and
  sanitized activation-reread states; QoderWork CN is the default/top selector
  and all five selectors use local icons.
- [x] Formal Windows WorkBuddy reads and writes remain bound to the frozen
  Explorer user's verified no-follow directory handle. Parent junctions, leaf
  reparse points, and identity drift fail closed with no primary, backup, temp,
  or redirected-target change.
- [x] QoderWork/TRAE model fields are transient guidance only and cannot emit a
  configuration-success state or persist secrets/configuration.
- [x] API keys enter only the explicitly requested protected Provider/WorkBuddy
  DB or live credential/config files and the one native request. They do not
  enter URLs, renderer/shared/non-secret storage, query caches/keys, logs,
  snapshots, public summaries, model IDs/names, or user-visible errors, and are
  cleared at all required renderer lifecycle boundaries.
- [x] Agents, Models, Skills, MCP, Prompts, and Memory are reachable and
  non-empty in final post-merge V2 shell unit/browser tests; existing
  Skills/MCP behavior, the merged Prompt/Memory contracts, and V2 architecture
  gates continue to pass.
- [x] The QoderWork/Trae asset bytes match their recorded official-source
  identities before processing, local assets resolve correctly, and the
  conservative nominative-use/provenance boundary remains documented.
- [x] `assets/fyagent.png`, all Tauri bundle icons, Windows NSIS icon, About
  icon, and macOS tray templates are synchronized to the Y identity and pass
  source, decode, frame, digest, equality, and mask checks.
- [x] A real Windows native app run validates the new routes/actions and visible
  application identity; a real Windows bundle build plus PE inspection proves
  its setup icon matches `src-tauri/icons/icon.ico`.
- [x] Every commit pinned in `research/remote-codex-merge-inventory.md` is an
  ancestor of local `dev/laiyongjie`; conflicts preserve both intended feature
  sets, all merge commits are local and attributable, and no remote ref is
  modified.
- [x] The full local check matrix passes, any unexecuted macOS/legal/remote
  evidence is reported as residual rather than inferred, the task is archived,
  all work is committed locally, no remote state is changed, and the working
  tree is clean.

## Out of scope

- A first-run goal interview, recommendation engine, ranking telemetry, or a
  complete implementation of every future behavior in Issues #22/#34/#101.
- Automatic QoderWork/TRAE installation, package mirroring, login-state or
  token capture, subscription relay, private schema reverse engineering, or
  claims that their model configuration is natively supported.
- Porting the complete legacy Provider manager, usage/proxy/failover UI, Codex
  Desktop installer/job coordinator, trusted restart flow, or Claude/Codex
  tool lifecycle manager into V2.
- A generalized cross-Agent apply-job framework, hardware secret backend, or
  cloud/team secret product. Existing safe backend commands remain the bounded
  MVP write path and their evidence limits are visible.
- Changing application identifier, data directories, deep links, product
  name, third-party Provider art outside the five catalog entries, or unrelated
  renderer/backend refactors.
- Claiming trademark clearance, vendor endorsement, macOS native visual HIL,
  signing/notarization, hosted-runner success, or release readiness without
  separately executed evidence.
- Rebasing, squashing, deleting, renaming, or pushing the remote `codex/*`
  branches. Their committed content is integrated; their remote lifecycle and
  any unrelated product expansion beyond that content are not changed here.
