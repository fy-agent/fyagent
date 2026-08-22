# Full-stack Modular Maintainability Refactor — Design

## 1. Objective

Reduce FyAgent's maintenance cognitive load by turning mixed-responsibility hotspots into domain-owned modules while preserving the current product, wire, persistence, platform and security contracts.

The target remains a **modular monolith**. The success metric is change locality and explicit ownership, not the number of files or average line count.

## 2. Design principles

### 2.1 Extract responsibilities, not arbitrary line ranges

A module is extracted only when at least one of these is true:

- it already has an independent public/port contract;
- it is a coherent domain capability with a focused test seam;
- it contains pure parsing/validation/transformation logic reusable by one owning capability;
- it is implementation hidden behind an existing facade/transaction coordinator;
- the current location violates an established layer responsibility (for example, thousands of lines of tooling policy inside Tauri command transport).

Large declarative data, schema contracts and tightly coupled rollback/state-machine code may remain large.

### 2.2 Keep stable composition APIs

Refactors preserve the APIs callers already depend on:

- V2 callers continue receiving one `FeaturePorts` object.
- `ProviderService`, `SkillService`, `ProxyService` remain stable service facades.
- `codex_config::*` external call sites continue through the parent module via explicit re-exports.
- Tauri command strings, arguments and return serialization stay unchanged.
- proxy router-facing handler names remain stable.

This lets physical/internal structure change without forcing unrelated consumers to change.

### 2.3 State/transaction owners stay singular

Do not split one transaction into several peer modules merely for file size.

Examples:

- Provider mutation guard and rollback orchestration remain owned by one mutation coordinator.
- Proxy takeover/hot-switch/restore I/O ordering remains coordinated by `ProxyService`.
- `RequestForwarder` remains the network request pipeline coordinator even if pure helpers move out.
- Skill archive extraction/copy/symlink/budget safety stays under one archive-safety owner.

### 2.4 V2 / leftover / neutral boundaries remain non-negotiable

```text
src/v2/**       production renderer
src/shared/**   renderer-generation-neutral domain/serialization core
src/**          leftover renderer outside v2/shared
```

No architectural cleanup may make V2 import leftover UI/runtime code. The current architecture test remains the authority and will be extended when new boundaries need executable enforcement.

## 3. Renderer target architecture

### 3.1 V2 Tauri feature-port adapters

Keep this external import stable:

```ts
import { createTauriFeaturePorts } from "./tauri/features";
```

Turn `features.ts` into a small composition facade. Capability-owned adapters/parsers live below a private-ish implementation directory, for example:

```text
src/v2/shared/platform/tauri/
├── features.ts                  # composes FeaturePorts only
└── feature-ports/
    ├── common.ts                # only genuinely cross-port guards
    ├── agents.ts                # catalog + external-agent validation
    ├── qoder-trae.ts            # external-agent config/probe contracts
    ├── codex-desktop.ts
    ├── models.ts                # provider/workbuddy/opencode model ports
    ├── skills.ts
    ├── mcp.ts
    ├── prompts.ts
    ├── memory.ts
    └── settings.ts
```

Exact grouping can be adjusted by dependency evidence, but the facade remains one `FeaturePorts` composition point.

The ACL contract must scan the entire Tauri adapter tree rather than assuming all command literals live in one file.

### 3.2 V2 route composition

Route roots retain route-level selection, query ownership and composition. Existing named panels/editors are moved to sibling modules:

```text
pages/models/
  Page.tsx
  WorkBuddyPanel.tsx
  ProviderPanel.tsx
  ... existing OpenCode/Qoder/Trae panels

pages/skills/
  Page.tsx
  InstalledSkillDetail.tsx
  Discovery.tsx
  SkillDialogs.tsx

pages/mcp/
  Page.tsx
  ServerDetail.tsx
  McpEditor.tsx

pages/memory/
  Page.tsx
  LongTermMemory.tsx
  DailyMemory.tsx

pages/prompts/
  Page.tsx
  PromptEditor.tsx
```

No new global store is introduced. State stays at the nearest common owner; extracted components receive explicit domain props/callbacks.

### 3.3 V2 feature contracts

After adapter/page extraction, re-evaluate `shared/features/types.ts` and `queries.ts`. Split them by existing product domains only if the remaining mixed ownership still forces unrelated changes together. Do not create barrels merely to make a directory look uniform.

### 3.4 Leftover renderer

Legacy remains compatibility code, not a second product architecture.

High-value selective changes:

- `ProviderForm`: extract cohesive submission/config-projection controllers or pure builders where this reduces app-specific branching without moving all form state into a monolithic hook.
- `providerConfigUtils`: separate generic JSON common-config behavior from Codex TOML editing if consumer analysis shows clean ownership.
- `App`, WebDAV and Session Manager: extract only named responsibilities with clear tests; do not perform path churn solely to reduce LOC.

No new product feature is added to leftover V1.

## 4. Rust/Tauri target architecture

### 4.1 Tooling: command transport vs application service

Current `commands/tooling.rs` is a transport-layer ownership violation. Target:

```text
src-tauri/src/commands/tooling.rs
  # four thin #[tauri::command] wrappers only, plus transport mapping if needed

src-tauri/src/services/tooling/
├── mod.rs            # ToolingService / stable crate-level API
├── versions.rs       # version probes + remote version lookup
├── lifecycle.rs      # install/update policy and anchored commands
├── discovery.rs      # installation/source/path probing
└── terminal.rs       # provider terminal launch policy/platform adapters
```

The exact internal split follows current helper call graph. `commands/hermes.rs` depends on the service/facade rather than command internals. Tauri command names and registration remain unchanged.

### 4.2 Skills

Target responsibilities:

```text
services/skill/
├── mod.rs            # SkillService facade + selected public DTO re-exports
├── types.rs          # stable skill/repository/backup DTOs where useful
├── discovery.rs      # existing
├── marketplace.rs    # skills.sh + SkillHub transport/mapping
├── repository.rs     # repo configuration and agents lock/source coordinates
├── assignment.rs     # target/app synchronization and toggles
├── backup.rs         # backup/restore ownership
├── migration.rs      # SSOT/storage migration
└── archive.rs        # ZIP/vendor-copy/symlink/budget security domain
```

Implementation may use a smaller set of files if call dependencies show stronger cohesion. Archive resource limits, traversal checks and materialization ordering remain in one security owner.

### 4.3 Codex configuration

Keep `codex_config` as the public facade:

```text
codex_config/
├── storage.rs        # existing
├── features.rs       # image/websocket capability analysis + patching
├── auth.rs           # auth/login/stale-residue/backfill policy
├── catalog.rs        # model catalog template/generation/readback/path validation
└── live.rs           # live provider/proxy/session/MCP projection and TOML mutations
```

Where a TOML helper belongs to one capability, keep it there; do not create a generic `utils.rs` dumping ground.

### 4.4 Proxy HTTP handlers

Keep router-facing `proxy::handlers::*` stable while splitting protocol ownership:

```text
proxy/handlers/
├── mod.rs
├── common.rs
├── claude.rs
├── codex.rs
└── gemini.rs
```

Health/status/models may remain in `mod.rs` or a small system module. Shared helpers move only when used by more than one protocol.

### 4.5 Provider service

Continue the existing facade approach:

```text
services/provider/
├── mod.rs
├── mutation.rs       # add/update/delete/switch + shared mutation guard
├── quick_setup.rs    # quick-setup validation/apply transaction
├── common_config.rs  # extraction/sensitive-key/scrub/migration rules
├── imports.rs        # startup/live import eligibility flows
├── live.rs           # existing host-config adapter; split per app only if proven
├── universal.rs
├── endpoints.rs
├── gemini_auth.rs
└── usage.rs
```

The mutation coordinator keeps rollback order. `common_config` owns credential scrubbing as one ordered safety flow even if helper functions are private within that module.

### 4.6 Proxy service and forwarder

These are high-risk coordinators. Do not pre-commit to a full physical split.

Permitted extractions:

- pure takeover/live-route decision functions;
- provider-specific projection builders;
- retry/failover decisions that do not own the loop or mutable request body;
- pure request/response transform helpers already covered by focused tests;
- status/config operations when independent from lifecycle locks.

Retain `ProxyService` and `RequestForwarder` as the single transaction/pipeline owners.

### 4.7 Usage analytics and `lib.rs`

Evaluate only after P0/P1 work:

- `usage_stats`: separate cost-backfill ownership from read analytics if tests/callers prove a one-way boundary.
- `lib.rs`: move lifecycle/window/deeplink implementation only when a named module reduces composition-root detail without hiding command/Builder registration.

`database/schema.rs`, protocol-specific transforms and existing Qoder/TRAE safety modules are intentionally not line-count targets.

## 5. Public/visibility rules

### Rust

- New implementation submodules are private `mod` by default.
- Use `pub(super)`/`pub(crate)` only for demonstrated sibling/crate consumers.
- Parent facades explicitly re-export stable types/functions; consumers should not deep-import implementation modules.
- Do not introduce traits unless there is an actual substitute implementation or test seam requiring one.

### TypeScript

- V2 page-local modules stay within their route folder.
- Tauri command strings stay within `shared/platform/tauri/**`.
- Shared multi-route UI continues to use existing `shared/ui` owners.
- Do not create giant `index.ts` barrels that hide dependency direction; use direct imports or deliberately small public entry points.

## 6. Compatibility strategy

This is a structural refactor. The following are frozen unless an existing bug is separately proven:

- Tauri command names/ACL entries and serialized fields;
- event names and unlisten/cleanup behavior;
- database/config file formats and paths;
- Provider mutation/rollback/guard ordering;
- Codex TOML losslessness, auth preservation, catalog projection and live-change semantics;
- Skill traversal/symlink/archive budgets and backup-before-delete behavior;
- Proxy takeover/restore/hot-switch and failover/streaming behavior;
- V2 route URLs, native-only states, dirty blockers and user-visible product behavior.

## 7. Architecture enforcement

Extend executable tests where source movement creates a stable rule:

- V2 Tauri adapters must remain under the platform boundary and `features.ts` should stay a composition facade rather than regrow capability parsers.
- `commands/tooling.rs` must not reacquire installation/terminal business implementation.
- new Rust implementation submodules remain private behind facades.
- existing V2/leftover/shared dependency rules remain unchanged.

Avoid brittle tests for arbitrary line counts or exact directory inventory unless the repository already treats the source as a sealed platform/security asset.

## 8. Rollback design

Each stage is committed independently after focused tests. A module move is reverted as one stage if behavior evidence regresses. High-risk Proxy/Forwarder work is optional within the task: if a clean seam cannot be proven without changing transaction ownership, document the retention and stop rather than forcing a risky split.

## 9. Delivery order

1. Architecture guards and V2 adapters.
2. V2 route decomposition.
3. Rust Tooling ownership.
4. Skill and Codex config.
5. Proxy handlers and Provider.
6. Selective leftover frontend and optional Proxy/Forwarder/Usage/lib cleanups where evidence remains strong.
7. Full Trellis check.
8. **Mandatory SPEC update before archive.**
9. Commit final task/spec evidence, finish and archive Trellis, record journal.
10. Push the resulting final archived/journal HEAD once and require green GitHub `CI / Required` for that exact SHA.

## 10. Deferred crate split decision

No new Cargo crate is planned. Reconsider only in a future task if an extracted module has:

1. stable one-way dependencies;
2. a narrow API independent of Tauri `AppState` internals;
3. meaningful independent compilation/test/reuse value;
4. no DTO/config duplication or workspace cycle.
