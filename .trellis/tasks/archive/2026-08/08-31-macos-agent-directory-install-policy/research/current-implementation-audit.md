# Current Implementation Audit

Date: 2026-08-31

Scope: read-only planning evidence. The working tree already contains another in-progress macOS Agent lifecycle change set. This audit did not modify those files and must be refreshed when this task enters implementation.

## 1. Current ownership map

| Capability | Current owner | Current behavior relevant to this task |
| --- | --- | --- |
| Product order and local UI metadata | `src/v2/shared/features/directory.ts` | `PRODUCT_DIRECTORY` freezes seven entries in Qoder/TRAE/WorkBuddy/Grok/Codex/Claude/OpenCode order. |
| Native static Agent Catalog | `src-tauri/src/commands/agent_catalog.rs` | Same seven-item order; Claude exposes CLI + Desktop links, OpenCode exposes Product + CLI links. |
| Directory rendering | `src/v2/pages/agents/AgentDirectory.tsx` | Renders `entries.map(...)`; there is no runtime sort owner. |
| Progressive scan state | `src/v2/pages/agents/useAgentDirectoryScan.ts` | Parallel readiness refetch, request IDs, per-row settle/failure, retained prior results, completed scan timestamp. |
| Scan row truth | `src/v2/pages/agents/agentDirectoryScanProjection.ts` | Distinguishes installed, not-installed, unknown, unavailable and technical error; installed-not-runnable still proves existence. |
| Frontend lifecycle surfaces | `src/v2/shared/features/agent-install-readiness.ts` | OpenCode is CLI+Desktop; Claude/Grok are CLI; Chinese three and Codex are Desktop. |
| Backend lifecycle surfaces | `src-tauri/src/agent_install/types.rs` | Mirrors the frontend surface matrix and selects a default surface. |
| Readiness/actions | `src-tauri/src/agent_install/mod.rs` | OpenCode has dual-surface aggregation; Claude routes to CLI Tooling; managed desktop products currently allow update when evidence/version permits. |
| Target authority | `src-tauri/src/agent_install/inventory.rs` | Owns opaque inventory/target/revision, target selection and action eligibility projection. |
| Desktop source registry | `src-tauri/src/agent_install/sources/` | QoderWork, TRAE Work, WorkBuddy and OpenCode source adapters exist. Claude managed-desktop source does not. |
| Download/job/progress | Codex Desktop and shared Agent install owners | Streamed artifact, retry/cancel, transfer telemetry and terminal jobs are already being shared. |
| macOS DMG transaction | `src-tauri/src/codex_desktop/platform/macos/dmg.rs` | Read-only mount, single direct app, generated staging/backup, replace/rollback and managed-product adapter. |
| System `/Applications` commit | separate helper task | Still gated; no product-specific elevation may be added here. |

## 2. Directory ordering facts

### Current render path

`AgentsPage` obtains the backend catalog and passes `entries` to `AgentDirectory`. `AgentDirectory` maps the array directly. Scan results only affect each card’s badge/actions; they do not alter order.

### Current scan semantics worth preserving

- Initial idle and unresolved in-flight rows are `pending`, not “not installed”.
- `installed` and `installed_not_runnable` both make the row configurable.
- A technical failure is distinct from vendor-reported `unknown` or `unavailable`.
- A later failed scan retains an earlier successful readiness object and keeps configuration available while also setting `readFailed=true`.
- Old request IDs are ignored.
- `applyReadiness` can patch one row after a lifecycle action without resetting the scan identity.

Implication: ordering must use current scan failure as “unresolved” while allowing the existing stale result to remain visible/configurable. Reusing only `readiness.installState` would incorrectly keep a failed stale installation in the “currently confirmed installed” bucket.

### Existing tests

`tests/v2/pages/agents/useAgentDirectoryScan.test.ts` already covers:

- progressive rows;
- installed-not-runnable;
- technical failure;
- retained prior results;
- stale configurability;
- request ID ordering;
- one-row authoritative patches.

The new sort tests should extend this owner rather than create a page-local mock state model.

## 3. Current product surface/action facts

### OpenCode

- Rust `legal_surfaces(OpenCode)` returns CLI and Desktop, defaulting to CLI.
- `opencode_readiness` separately probes CLI and Desktop and puts both into `surfaces`.
- CLI install/update dispatches through Tooling.
- Desktop install/update dispatches through managed desktop.
- Desktop source already uses the official fixed stable endpoints:
  - `https://opencode.ai/download/stable/darwin-aarch64-dmg`
  - `https://opencode.ai/download/stable/darwin-x64-dmg`
- The current source descriptor is versionless (`display_version=None`), so it cannot reliably distinguish up-to-date from update-available without an additional reused metadata owner.

### Claude

- Rust and TypeScript currently declare only CLI surface.
- Readiness and install/update actions use Tooling `claude`.
- Native catalog already has a Claude Desktop official link, but no managed desktop inventory/source/policy.
- The stable product ID and many non-installer domains use `claude-code`; creating an eighth product would cause unnecessary cross-layer migration.

### QoderWork, TRAE Work and WorkBuddy

- They are managed desktop products with official source adapters and existing inventory evidence.
- Generic desktop readiness currently computes remote version/update state and can add `update` to `allowedActions` for one eligible installed target.
- Action dispatch accepts both Install and Update for all managed desktop products.
- Tests contain fixtures where QoderWork candidates have `updateEligible=true`.

Implication: removing only the button would leave direct action requests and target picker eligibility active. The policy must be applied in readiness, inventory and dispatcher.

## 4. Current source details that must not be deleted

### QoderWork

- official metadata: `latest.yml` / `latest-mac.yml`;
- fixed installer aliases for Windows x64, macOS arm64 and macOS x64;
- metadata version is required to construct the release descriptor.

### TRAE Work CN

- official latest API;
- exact `data.solo` + `region=cn` selection;
- exact stable download host/path/file policy.

### WorkBuddy

- vendor endpoint path is `/v2/update`;
- response supplies current release metadata;
- macOS `.zip` URL is rewritten to the corresponding fixed `.dmg` form under the same reviewed host/path contract.

The word `update` in an upstream API path does not mean FyAgent must expose an update action. All three source adapters remain necessary for fresh install.

## 5. Existing reuse opportunities

### Source and HTTP

- Codex `source.rs` already demonstrates fixed endpoint enums, private manifest DTOs, bounded streamed metadata, retry/cancel/cache and opaque release descriptors.
- Tooling already has a fixed GitHub latest-version capability used for OpenCode.
- Agent managed sources already centralize host allowlists and opaque release IDs.

### Artifact and installation

- managed Agent downloads already delegate toward the Codex-tested streamed artifact owner;
- managed DMG install delegates to the Codex transaction through product policy;
- inventory owns target selection and revision;
- progress format/job state already exists and should be reused;
- application launch uses the shared backend launch boundary.

### Frontend

- scan row projection already owns current/stale truth;
- shared product directory already owns local icon/product metadata;
- lifecycle hook already derives install/update only from `allowedActions`.

## 6. Current gaps

1. No single backend product lifecycle policy; product/surface/action matches are distributed across types, inventory and dispatcher.
2. No dynamic Agent Directory order owner.
3. No single domestic-priority metadata field.
4. Qoder/TRAE/WorkBuddy update is still enabled across readiness, inventory and action dispatch.
5. Claude has no desktop product/source policy.
6. OpenCode latest stable version is not bound to its desktop release descriptor.
7. Catalog official links still advertise removed CLI installation surfaces.

## 7. Implementation caveat

This audit observes an active refactor. When this task starts, Phase 0 must re-run the call-graph and test search. The required behavior is stable; exact file placement may change. Execution must adapt to the final owner rather than restoring paths or patterns from this snapshot mechanically.

