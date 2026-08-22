# Full-stack Modular Maintainability — Audit and Research

## Scope

This note records the second-pass repository audit and external architecture evidence used to choose concrete refactor boundaries. It updates the previous archived modularization research against the current post-refactor tree at baseline commit `7249d28c`.

## Baseline evidence

- Branch: `dev/laiyongjie`.
- Baseline working tree was clean before this Trellis task was created.
- Baseline HEAD: `7249d28ce4e4f94af18e0d224b11fccdeb62da28`.
- GitHub CI run `32553501296` for that exact SHA completed successfully.
- Production renderer remains `src/v2/main.tsx`; leftover V1 remains compatibility/test code.
- Current TypeScript static import graph has **0 strongly connected components/cycles**. This confirms the prior ProviderForm/GrokBuild cycle removal held.

## External architecture evidence

### Rust modules are the first organizational boundary

The current Rust Book explicitly frames packages/crates/modules as tools for growing programs: related functionality should be grouped, distinct features separated, and implementation details hidden behind public interfaces to reduce the amount of detail a maintainer must keep in their head. It also explicitly recommends moving large modules to separate files while retaining the module tree.

Sources:

- https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html
- https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html

Adopted principle: **private responsibility submodules + narrow facade are the default refactor unit**. File-count or line-count reduction alone is not a sufficient reason.

### Cargo workspaces are package boundaries, not a substitute for modules

Cargo defines a workspace as a set of related packages managed together; the Rust Book describes it as useful when a growing package genuinely needs multiple library crates. That is stronger than merely having a large source file.

Sources:

- https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
- https://doc.rust-lang.org/cargo/reference/workspaces.html

Adopted principle: keep FyAgent a single-crate modular monolith during this task. A crate split is rejected unless one-way dependencies, a narrow Tauri-independent API, and independent compilation/reuse value are demonstrated.

### Tauri commands are an explicit transport boundary

Tauri's official guide recommends defining commands in separate modules when command groups grow rather than bloating `lib.rs`; frontend command names remain independent of the Rust module prefix.

Source:

- https://v2.tauri.app/develop/calling-rust/

Adopted principle: reorganizing command ownership must preserve command names/ACL registration, while substantial install/probe/filesystem/orchestration logic should live behind a service/domain owner instead of remaining in command transport.

### React: decompose when one component/module has multiple concerns

React documents both render trees and module dependency trees as useful architecture models. `Thinking in React` explicitly applies separation of concerns: a component should ideally concern itself with one thing, and growing components should be decomposed. React's import/export guide also notes that crowded modules should be split into files for scanability and reuse.

Sources:

- https://react.dev/learn/understanding-your-ui-as-a-tree
- https://react.dev/learn/thinking-in-react
- https://react.dev/learn/importing-and-exporting-components

Adopted principle: route composition should remain visible at the route root, while large independent panels/editors/dialogs/controller logic move to route-owned modules. State is not globalized merely to shorten a component.

### Feature boundaries and public APIs

Feature-Sliced Design describes a public API as a contract/gate around a module group and warns that uncontrolled cross-imports reduce refactorability. Its public-API documentation also warns that giant barrel files can create circular-import/tree-shaking/navigation problems.

Source:

- https://fsd.how/docs/reference/public-api/

Bulletproof React independently recommends feature-local colocation, no arbitrary cross-feature imports, composition at the application layer, and unidirectional shared → feature → app dependencies; it demonstrates enforcing those rules with ESLint.

Sources:

- https://github.com/alan2207/bulletproof-react/blob/master/docs/project-structure.md
- https://github.com/alan2207/bulletproof-react

Adopted principle: borrow **feature/domain ownership and dependency direction**, not the directory names wholesale. Existing FyAgent V2 layers remain authoritative.

## Current renderer audit

### Dependency graph

- Source files: V2 76, renderer-neutral shared 7, leftover renderer 347.
- Static import cycles: **0**.
- Highest fan-out modules include:
  - `src/App.tsx`: 54 source-module dependencies, 1709 lines.
  - `src/components/providers/forms/ProviderForm.tsx`: 48 source-module dependencies, 2699 lines.
  - `src/v2/pages/mcp/Page.tsx`: 19 source-module dependencies, 996 lines.
  - `src/v2/pages/models/Page.tsx`: 18 source-module dependencies, 1547 lines.
  - `src/v2/pages/skills/Page.tsx`: 16 source-module dependencies, 1273 lines.
- Highest fan-in modules are expected shared contracts/primitives (`src/types.ts`, UI primitives, `src/lib/api/index.ts`) rather than a new cyclic hotspot.

### V2 platform adapter hotspot

`src/v2/shared/platform/tauri/features.ts` is 1787 lines. It contains parsers/validators and native adapters for all of these independent port families:

- agent catalog and external-agent lifecycle;
- QoderWork hooks and external MCP validation;
- TRAE model validation/probe;
- Codex Desktop installer events/jobs;
- Provider quick setup/model probe;
- WorkBuddy/OpenCode model management;
- Skills;
- MCP;
- Prompts;
- Memory;
- Settings/external link.

The file has only five imports because the coupling is hidden behind one enormous type import and one `createTauriFeaturePorts()` object. This is **high cohesion at the platform layer but low cohesion by product capability**. The existing `FeaturePorts` object is a useful composition API and should remain; the per-capability adapter/parser implementations should not all remain in one file.

### V2 shared feature contract hotspot

`src/v2/shared/features/types.ts` is 813 lines and mixes runtime constants plus DTOs for Skills, MCP, Agents, Qoder/TRAE, Provider quick setup, WorkBuddy/OpenCode, Prompts, and Memory. `queries.ts` similarly owns query hooks for every route. These files are not currently cyclic, but their mixed ownership means a local feature contract change frequently opens a repository-wide contract file.

Decision signal: split only into existing product-domain owners while retaining a small composition surface; do not create speculative FSD layers or generic barrel hierarchies.

### V2 route hotspots

- `pages/models/Page.tsx` — 1547 lines; `WorkBuddyPanel` alone occupies most of the first ~900 lines, followed by `ProviderPanel` and route target composition. Qoder/TRAE/OpenCode are already separate panels, proving the missing WorkBuddy/Provider split is structural debt rather than a new pattern.
- `pages/skills/Page.tsx` — 1273 lines; contains installed detail, discovery cards/list, install/update flow and auxiliary dialogs in separately named functions. These are already clear route-local responsibility seams.
- `pages/mcp/Page.tsx` — 996 lines; contains `ServerDetail`, route orchestration and `McpEditor`.
- `pages/memory/Page.tsx` — 965 lines; contains long-term memory view/editor and daily-memory view/editor with independent source semantics.
- `pages/prompts/Page.tsx` — 843 lines; contains route orchestration plus prompt editor/identity form.

Decision signal: move named route-local responsibilities into sibling modules while leaving route selection/query coordination at `Page.tsx`. Do not invent a global state store.

### Leftover renderer

- `App.tsx` was reduced from 1958 to 1709 lines in the prior task and no longer owns the extracted runtime event coordinator, but it still has a broad view-selection/composition switch. It is not the production renderer.
- `ProviderForm.tsx` remains the dominant leftover hotspot: 2699 lines, 55 imports and many app-specific branches across Claude/Codex/Gemini/OpenCode/OpenClaw/Hermes. It already delegates field rendering to specialized components and state helpers, but the remaining form orchestrator owns preset selection, multiple auth systems, config projection, soft-validation/submit, pricing, request overrides and per-app save preparation.
- `WebdavSyncSection.tsx` (1867), `SessionManagerPage.tsx` (1757), and `providerConfigUtils.ts` (1600) are large. They are leftover-only; `providerConfigUtils.ts` is mostly cohesive Codex/TOML transformation logic, while the components mix several UI responsibilities.

Decision signal: prioritize production V2. Only refactor leftover modules where a proven dependency/logic boundary materially reduces compatibility maintenance; avoid large path churn in non-production surfaces.

### Large data/config files

Large preset modules (`openclawProviderPresets.ts`, `opencodeProviderPresets.ts`, `codexProviderPresets.ts`, etc.) are primarily declarative data. They are **not** modularization targets merely because they exceed 1000–2500 lines.

## Current Rust/Tauri audit

### Dependency hubs

Approximate crate-reference analysis shows the expected infrastructure hubs (`error`, `config`, `settings`, `store`) and these high domain fan-out modules:

- `lib`: ~26 top-level/module dependencies.
- `services::provider`: ~24.
- `commands::provider`: ~20.
- `services::proxy`: ~18.
- `commands::tooling`: ~8, despite containing thousands of lines of implementation.

This corroborates that Provider/Proxy are orchestration hubs while Tooling hides substantial implementation behind a small command surface.

### `commands/tooling.rs` — transport-layer ownership violation

- 5481 total lines; roughly 3557 production lines before the main test module.
- Only four Tauri commands: `get_tool_versions`, `run_tool_lifecycle_action`, `probe_tool_installations`, `open_provider_terminal`.
- The implementation includes version fetches, install/update policy, package-manager/source classification, environment/path probing, anchored upgrade command generation, command deadlines, installation discovery, AppleScript/terminal launch logic and Windows elevated/Explorer launch behavior.

Conclusion: the file is not merely a large command module; it is a hidden tooling application service. The command signatures should stay stable while lifecycle/discovery/terminal policy moves behind a dedicated private service/module facade.

### `services/skill.rs`

- 6780 total lines; approximately 4914 production lines.
- Responsibilities include DTO/store types, skills.sh, SkillHub marketplace, repository config, `.agents` lock parsing, installation/update/uninstall, assignment/sync, backup/restore, migration, ZIP extraction, vendor copy, symlink validation and archive resource budgets.
- `skill/discovery.rs` already proves one safe extraction seam.

Conclusion: strong candidate for further decomposition. Security-sensitive archive/copy/symlink operations must remain under one archive-safety owner rather than being scattered across unrelated helper modules.

### `codex_config.rs`

- 6078 total; ~3049 production lines plus extensive tests.
- Natural responsibility bands are visible in source order: native capability/features; auth/login/backfill; model catalog generation/readback; live/provider/proxy/session projection; TOML field mutation.
- `codex_config/storage.rs` already proves the parent facade can remain stable while implementation moves private.

Conclusion: strong candidate for `features`, `auth`, `catalog`, and `live/projection` private modules, with TOML helpers placed according to ownership rather than a catch-all `utils` file.

### `proxy/handlers.rs`

- 3351 total; ~2661 production lines.
- Source itself has explicit Claude, Codex and Gemini handler sections.
- Public router-facing handlers are already protocol-specific.

Conclusion: high-value, relatively low-risk physical/module split by protocol with a stable `handlers` facade. Shared response helpers remain in the smallest common owner.

### `services/provider/mod.rs`

- 6569 total lines with interleaved private-unit tests and substantial production logic.
- Existing child modules: endpoints, gemini_auth, live, universal, usage.
- Remaining parent responsibilities still include mutation/switch/quick setup, common-config extraction and credential scrubbing, startup/import/migration and validation.
- `live.rs` itself is 2618 total/~1975 production lines and spans multiple applications.

Conclusion: continue only along independently testable responsibility boundaries (`common_config`, mutation/quick-setup/import), preserving one mutation guard/transaction owner. Avoid splitting a switch transaction across arbitrary helper modules.

### `services/proxy.rs`

- 7355 total/~3190 production lines.
- Owns lifecycle, takeover/restore/backup, provider switching, live projection and recovery state transitions.
- Prior `proxy/takeover.rs` extraction successfully removed pure takeover matching only.

Conclusion: keep `ProxyService` as transaction coordinator. Extract pure decision/projection/lifecycle planning only where state/I/O ordering does not cross the boundary. Do not distribute one rollback transaction across multiple owners just to shorten the file.

### `proxy/forwarder.rs`

- 5023 total/~3586 production lines.
- Owns request preparation, retries/failover, endpoint/header/body transforms, streaming response handling, error normalization, diagnostics and part of the Codex→Anthropic bridge.

Conclusion: architectural debt exists, but this is the network data-plane coordinator. Split only pure request/response/protocol helpers that have dedicated tests while preserving a single `RequestForwarder` pipeline coordinator. This is a later/high-risk stage.

### Other large Rust files

- `services/usage_stats.rs` (~2364 production lines): combines analytics query construction with pricing-cost backfill. A query/backfill separation is plausible and should be evaluated during implementation.
- `lib.rs` (~3166 production lines): still broad, but as the Tauri composition root some fan-out is legitimate. Lifecycle/window/deeplink helpers should move only when a named owner reduces review surface without obscuring Builder registration.
- `database/schema.rs` (~3183 production lines): predominantly schema/migration contract; large but highly cohesive. Not a line-count target.
- protocol transform files (`transform_codex_chat`, `transform_codex_anthropic`, etc.) are long but already protocol-owned and heavily tested. Request/response separation is optional, not a priority unless implementation reveals a clean one-way seam.
- `services/codex_desktop/mod.rs` is already surrounded by cancellation/download/jobs/platform/runtime/source/temp/types/verify modules; its remaining service orchestration is not a priority despite total file length.
- Qoder/TRAE modules are security-boundary domains with explicit DTO/state/storage/probe invariants. Do not split them without a specific ownership gain.

## Adopt/reject matrix

| Approach | Decision | Reason |
| --- | --- | --- |
| Mechanical maximum file size | Reject | line count does not distinguish data, cohesive protocol transforms or safety state machines |
| V2 per-capability Tauri adapter modules composed into `FeaturePorts` | Adopt | clear existing port boundaries; no product behavior change |
| Route-local V2 panel/editor extraction | Adopt | named components already prove the seams; React composition becomes scannable |
| Feature/domain split for V2 contract/query owners | Adopt selectively | mixed ownership is real, but avoid a new speculative layer taxonomy |
| Mass rewrite/move of leftover V1 | Reject | non-production compatibility tree; high churn/low value |
| Further ProviderForm decomposition | Selective | useful only for controller/submission boundaries, not another giant hook |
| Rust private submodules with parent facade | Adopt | compiler privacy + current Trellis contract + Rust guidance |
| New `services/tooling` owner behind four stable commands | Adopt | removes thousands of lines of business/platform policy from transport layer |
| Skill/Codex/handler decomposition | Adopt | independently named/testable responsibilities already exist |
| Split Proxy/Forwarder transaction coordinators by arbitrary phase | Reject | would distribute rollback/stream/failover state and increase cognitive coupling |
| Extract pure Proxy/Forwarder decisions/transforms | Adopt when proven | reduces coordinator detail without fragmenting transaction ownership |
| New Cargo crates/workspace members | Reject for this task | no demonstrated independent package boundary |

## Planning conclusion

The second pass should optimize **change locality and ownership**, not source-file aesthetics. The preferred order is:

1. strengthen architecture checks around the new target boundaries;
2. split V2 platform adapters and obvious route-local modules;
3. move Tooling implementation out of Tauri transport;
4. decompose Skill and Codex config along existing responsibility/test seams;
5. split proxy handlers by protocol and Provider common/mutation responsibilities;
6. only then touch Proxy/Forwarder/usage/lib if a clean seam remains after the safer refactors;
7. update SPEC with the resulting executable ownership rules before archive;
8. archive, make the archive/session commits, then perform one final push and require green CI for that exact archived HEAD.

## Final implementation outcome

The implementation followed the audit's **change-locality over line-count**
rule, but deliberately stopped several originally plausible moves after a
second call-graph/test-seam review showed that the physical split would add
state or protocol coupling without creating a stronger owner.

### Extracted boundaries

| Baseline hotspot | Final ownership | Result |
| --- | --- | --- |
| `src/v2/shared/platform/tauri/features.ts` (1787 lines) | 18-line composition facade + capability-owned `feature-ports/**` adapters | validators, parsers and invoke calls now move with the owning FeaturePort while the `FeaturePorts` API stays stable |
| `commands/tooling.rs` (~5481 lines) | 32-line command facade + `services/tooling.rs` (5476) | Tauri transport no longer owns CLI discovery/lifecycle/version/terminal policy; platform fail-closed contracts moved with the service implementation |
| `services/skill.rs` (6780) | parent facade/transaction owner + `skill/marketplace.rs` (539) | skills.sh / SkillHub transport and mapping are isolated without duplicating archive extraction or filesystem safety |
| `codex_config.rs` (6078) | parent facade + `codex_config/auth.rs` (145) | auth/login/stale-residue/backfill policy has one owner; catalog/live/features remain together because their test/TOML coupling did not justify a navigation-only move |
| `services/provider/mod.rs` (6569) | parent transaction facade + `provider/common_config.rs` (238) | pure sensitive-key/common-config extraction is isolated; mutation guards and ordered Gemini scrub remain singular |
| `providerConfigUtils.ts` (~1600) | 37-line compatibility facade + JSON (241), Codex TOML (1207), structural-safety (74) owners | all 28 previous exports are preserved while JSON and TOML configuration languages no longer share one implementation file |

Executable architecture checks now protect the Tooling transport boundary,
private Rust subdomains, the V2/leftover/shared renderer direction, the V2
Tauri adapter tree, and the leftover provider-config compatibility facade.

### Intentionally retained large coordinators

- **V2 route roots** (`Models` 1547, `Skills` 1273, `MCP` 996, `Memory` 965,
  `Prompts` 843) were not physically split. AST/call-site review showed the
  remaining length is predominantly route-local panels bound to common
  selection/query/dirty-blocker state. Moving those named functions would
  mostly relocate JSX and expand props/state plumbing without reducing a
  cross-domain dependency.
- **`proxy/handlers.rs` (3351)** remains one protocol data-plane owner. Its
  Claude/Codex/Gemini sections share streaming/error/usage helpers; no clean
  one-way module boundary was proven that justified changing this hot path.
- **`proxy/forwarder.rs` (5023)** remains the single retry/failover/streaming
  request-pipeline coordinator. Splitting phases into peer modules would make
  mutable request body, failover state and error normalization cross module
  boundaries.
- **Skill archive/materialization/backup** remains under the existing parent
  safety owner because traversal, symlink, byte/entry budgets and
  backup-before-delete ordering are coupled invariants.
- **Provider mutation/Gemini scrub** remains in the parent service because its
  lock/rollback/write order is a correctness and credential-safety property.

These are stop-rule decisions, not unfinished implementation: the task rejects
mechanical line-count targets when the proposed extraction worsens ownership.

### Final dependency and quality evidence

- Current renderer scan: **440 TS/TSX files**, **0 multi-file static-import SCC
  cycles**. The highest fan-out remains expected composition/leftover hotspots
  (`App.tsx` 54, `ProviderForm.tsx` 48); the modularization did not introduce a
  new cycle.
- `mise run check` passed on the final product-code state, including TypeScript
  type/format checks, **168 frontend test files / 1476 passed / 1 skipped**,
  Rust fmt/check/clippy, **2807 Rust library tests passed / 5 ignored**, Codex
  Desktop 109/109, Provider/Skill integrations, supported-platform sealing and
  release/contracts.
- V2-specific `lint:v2`, `typecheck:v2`, and `test:v2` passed; V2 tests are
  **277/277**.
- Modular-boundary/platform focused tests passed **32/32** after the SPEC
  update.

No Tauri command name/serialized contract, persisted format, Provider rollback
order, Skill archive budget, proxy streaming/failover path, or intended
renderer behavior was deliberately changed by this task.
