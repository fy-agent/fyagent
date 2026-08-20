# 实施计划：Agent 目录交互、官方链接与 Codex 安装接入

## 0. Planning gate and activation

**Depends on:** final `prd.md`, `design.md`, this plan, research notes and both context manifests are complete and reviewed; the final planning summary has been presented; the user sends a subsequent explicit implementation approval required by the Trellis phase gate.

1. Run task/context validation and read all artifacts top to bottom.
2. Confirm branch `dev/laiyongjie`, current task and a clean/non-conflicting worktree.
3. Run `python ./.trellis/scripts/task.py start .trellis/tasks/08-14-agent-catalog-interactions` only after the gate.
4. Load Phase 2.1 context. Dispatch prompts must begin with `Active task: .trellis/tasks/08-14-agent-catalog-interactions`.
5. Context validation warns that `.trellis/spec/backend/codex-desktop-installer.md` exceeds the automatic injection byte limit. Every installer implement/check worker must therefore read that file completely from disk before acting; truncated injected text is not sufficient.
6. Preserve unrelated active tasks and user changes; no destructive Git operation.

**Completion definition:** task status is `in_progress`, curated context is available to workers, and no product code was edited while status was `planning`.

## 1. Baseline and change inventory

**Owner:** coordinating session; read-only.

- Record `git status --short`, relevant existing tests and current hashes of reused application assets.
- Run the narrow pre-change V2 Agent/Models, Skills/MCP, installer hook/card and Rust catalog tests to establish whether any failure predates this task.
- Confirm exact official URLs against the research note immediately before editing.

```powershell
mise run test:v2
mise run test:unit -- tests/hooks/useCodexDesktopInstaller.test.tsx tests/components/CodexDesktopInstallerCard.test.tsx
mise run rust:test agent_catalog
```

**Stop condition:** an overlapping user change or unrelated failing baseline that prevents attribution is reported and isolated before writes continue.

## 2. Parallel foundation work

Run three Trellis implementation workers with non-overlapping ownership. Every worker is told it is not alone, must accommodate concurrent edits, must not revert other changes, and must run focused checks for its write set.

### 2A. Neutral Codex installer pure core

**Owner:** Worker A owns only `src/shared/codex-desktop/**`, compatibility re-exports/import adjustments in `src/types/codexDesktop.ts`, `src/components/codex/versionState.ts`, `src/hooks/useCodexDesktopInstaller.ts`, and the existing legacy installer hook/state tests. It must not edit V2 files or Rust.

- Move the DTOs and pure installer derivations identified in `design.md` into a Tauri/UI-free neutral shared boundary.
- Preserve old import paths with compatibility re-exports where practical.
- Make the existing legacy Hook consume the shared pure functions without changing its public view model or behavior.
- Preserve every existing installer test; add focused pure-core tests for snapshot order and download-speed edge cases if current tests cannot import the new owner directly.

```powershell
mise run typecheck
mise run test:unit -- tests/hooks/useCodexDesktopInstaller.test.tsx tests/components/CodexDesktopInstallerCard.test.tsx
mise run format:check
```

**Rollback point:** if extracting a rule changes legacy behavior, restore the old caller behavior and narrow the extraction; do not continue with two diverging implementations.

### 2B. Rust Agent catalog v2

**Owner:** Worker B owns `src-tauri/src/commands/agent_catalog.rs` and its focused Rust tests only. It must not edit renderer files.

- Introduce the exact structured link DTO, contract version 2 and managed-install status.
- Populate the exact five-entry matrix, verified URLs, labels, capabilities and evidence.
- Freeze exact keys/order/IDs, HTTPS URLs, unique link IDs, Claude two links, Codex zero links, Codex browse/install states and secret-free serialization.

```powershell
mise run rust:fmt:check
mise run rust:test agent_catalog
mise run rust:check
```

**Rollback point:** v1 remains the last known contract; no compatibility shim or dual ambiguous payload is added if v2 exact tests fail.

### 2C. Assignment icons and Agent layout styles

**Owner:** Worker C owns `src/v2/shared/assets/apps/**`, the necessary V2-owned copied asset files, `src/v2/shared/ui/AssignmentPanel.tsx`, assignment/asset focused tests, `src/v2/app/styles/features.css`, and `src/v2/pages/agents/Page.css`. It must not edit `AgentsPage`, FeaturePorts, catalog types or Rust.

- Add the exhaustive six-app typed icon map using reviewed local assets.
- Render decorative icons beside existing assignment text while preserving exactly six switch names and one panel.
- Remove Agent image white tiles and add cross-axis start alignment without fixed heights or nested scrolling.
- Update exact asset inventory/structure tests only through their current owner pattern.

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run build:renderer
```

**Rollback point:** retain original third-party asset bytes; stop if an asset requires unreviewed remote download, recoloring or provenance loss.

## 3. Integrate catalog v2 into V2 Agent/Models

**Depends on:** 2B complete; 2C asset helper stable.

**Owner:** one Trellis implementation worker owns `src/v2/shared/features/types.ts`, catalog runtime parsing/query/fixtures, `src/v2/pages/agents/Page.tsx`, `src/v2/pages/models/Page.tsx`, and their focused V2/unit/browser tests. It must not edit Rust or the installer modules from 2A/4.

- Replace `officialUrl` consumption with parsed `officialLinks` and contractVersion 2.
- Render one official action for QoderWork/TRAE/WorkBuddy, two independent Claude actions, and none for Codex.
- Keep per-link pending state and one external-open lock; retain fixed safe error copy.
- Make Models select the explicit `product` link for QoderWork/TRAE instead of relying on array order.
- Update fake-Tauri catalog fixtures and exact IPC/browser assertions for all external actions and negative Codex behavior.
- Confirm Agent/Models still use the exact same five IDs/order.

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

**Completion definition:** no V2 consumer reads `officialUrl`; runtime-invalid/v1 payloads fail controlled; exact buttons and URLs are covered.

## 4. Add the V2 Codex installer port, controller and view

**Depends on:** 2A shared core complete; Agent detail integration shape from 3 stable.

**Owner:** one Trellis implementation worker owns the CodexDesktopPort additions, Tauri/browser adapters, V2 installer controller/query/view modules, fake-Tauri installer fixture, Codex detail mounting and focused tests. Coordinate before touching any file also owned in phase 3; phase 3 must finish first.

- Add the exact port methods and Tauri event subscription under `src/v2/shared/platform/tauri/**`; use only the seven existing fixed commands and event name.
- Add explicit native-only browser behavior and injected rich test fixtures.
- Reuse neutral DTO/view/snapshot/speed rules; do not import old Hook/component/lib or duplicate backend validation.
- Build a V2-native installer panel using V2 primitives and tokens, including all applicable state/action/error/progress branches.
- Mount it only for selected Codex. Suppress external-link UI and preserve Provider observation/model-configuration action as a separate concern.
- Add StrictMode listener cleanup, stale/out-of-order event, repeat-click lock, METADATA_CHANGED, progress-unit and redacted-error tests.

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run test:unit -- tests/hooks/useCodexDesktopInstaller.test.tsx tests/components/CodexDesktopInstallerCard.test.tsx
mise run build:renderer
```

**Completion definition:** legacy and V2 installer views consume the same pure contract, V2 exact IPC/events pass, browser preview never reports authoritative success, and no new renderer-controlled installer input exists.

## 5. Cross-layer integration and native external-launch diagnosis

**Owner:** coordinating session plus a follow-up implementation worker only if a verified native failure requires code changes.

1. Review the combined diff for catalog wire agreement, secret exposure, duplicate state rules, V2 boundary violations, missing assets and unrelated changes.
2. Run the real current-host application through `mise run dev` and exercise:
   - Agent selection and layout;
   - at least one QoderWork/TRAE/WorkBuddy system-browser action;
   - both Claude buttons;
   - Codex installer status/refresh view without starting a destructive install;
   - Skills and MCP assignment icons.
3. If `open_external` fails, capture the public error and relevant redacted application log, trace `open_http_url_as_user`, implement only the smallest fix that preserves HTTP(S) validation and interactive-user launch authority, then rerun focused Rust tests and native observation.
4. Do not start a real Codex installation when it would overwrite user state, add system packages or lacks a reversible test condition. Record that HIL separately.

```powershell
mise run system:check -- --json
mise run dev
mise run rust:test process_launch
```

**Completion definition:** real browser handoff is observed or an exact blocker/root cause is recorded; native installer status renders; no mock claim substitutes for HIL.

## 6. Full quality check and review loop

Dispatch `trellis-check` with the exact active task and curated `check.jsonl`. It reviews code quality, contract/spec consistency, UI/accessibility/responsiveness, installer/security boundaries, test evidence and regression risk; it self-fixes only verified in-scope findings and reruns focused checks.

Required final local matrix:

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer

mise run typecheck
mise run format:check
mise run test:unit

mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test

mise run supported-platform:check
mise run check:contracts:prearchive -- --exclude-active-task .trellis/tasks/08-14-agent-catalog-interactions
git diff --check
```

Review `git diff --stat`, `git diff --name-status`, binary asset hashes, `git status --short`, exact task scope and every check result. A failing full gate is diagnosed; unrelated pre-existing failures are distinguished with baseline evidence, not silently ignored.

## 7. Spec synchronization, commits and Trellis finish

**Depends on:** phase 6 green or every remaining exception explicitly evidenced.

- Use `trellis-update-spec` to update durable contracts in V2 Agent/Models, V2 Skills/MCP, V2 shell neutral-shared allowlist and Codex Desktop Installer notes. Do not record transient run output as a code contract.
- Validate task context/manifests and run prearchive contract checks.
- Create reviewable local commits focused on the change itself, for example:
  - `refactor(codex): share desktop installer state`
  - `feat(v2): complete agent catalog actions`
  - `fix(v2): polish agent and assignment presentation`
  - a narrow verified review-fix commit only if needed.
- Record actual validation and residual native risk in the developer journal, finish/archive through Trellis, and commit the archive/journal administration locally. Do not push without explicit authority.

```powershell
python ./.trellis/scripts/task.py validate .trellis/tasks/08-14-agent-catalog-interactions
mise run check:prearchive -- --exclude-active-task .trellis/tasks/08-14-agent-catalog-interactions
git status --short
```

## Final stopping conditions

- No implementation begins before the planning/start gate.
- No broken contract version, v1/v2 silent fallback, direct V2 legacy import, renderer installer URL/path input, duplicate assignment panel, missing app icon, secret-bearing error or unsafe process-launch fallback is accepted.
- A real system-browser observation is required for AC4. A real Codex installation remains explicitly unverified unless it can be executed without risking user-owned installation/configuration state.
- Completion requires attributable diffs, checks matching every claim, recorded unverified items, local commits, Trellis archive and no remote push.
