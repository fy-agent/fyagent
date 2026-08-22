# Modular Architecture Refactor Design

## 1. Architectural objective

Convert FyAgent from a growing collection of horizontal utility/service buckets into a modular monolith with explicit ownership, stable public boundaries, and enforceable dependency direction while preserving current product behavior.

The design deliberately uses an incremental migration. The production V2 renderer already has useful boundaries and will not be rewritten merely to match a fashionable architecture. The Rust host will be reorganized aggressively where coupling evidence justifies it, but package/crate boundaries will only be introduced if module-level boundaries prove insufficient.

## 2. Non-negotiable invariants

### Product/runtime

- `src/v2/main.tsx` remains the production renderer entry.
- Existing Tauri command names and wire DTO behavior remain stable unless a test-backed compatibility change is explicitly required.
- Existing persisted database/config formats remain compatible.
- Secrets must not cross new renderer/query/storage boundaries beyond their existing approved lifetime.
- Native platform behavior and security contracts remain intact.

### Branch/delivery

- Work remains on `dev/laiyongjie`.
- Changes are committed in reviewable stages.
- Final local quality gates and remote CI must pass before archive.

## 3. Renderer target architecture

### 3.1 V2 remains the production architecture

Keep the current roles:

```text
src/v2/
  app/                 composition, router, top-level providers
  pages/               route-owned composition
  widgets/             reusable shell-level composition
  shared/features/     typed feature ports, DTOs, queries/helpers
  shared/platform/     browser/Tauri adapters
  shared/ui/           cross-route UI primitives/chrome
```

Dependency direction remains enforced by `tests/v2/app/architecture.test.ts`. No broad legacy import exception will be added.

### 3.2 Cross-generation neutral core

`src/shared/**` is the only intentional place for logic genuinely shared by V2 and leftover code.

Rules:

- A shared module must be renderer-generation-neutral: it may not import V2 pages/widgets or leftover UI/hooks.
- It exposes a small public entry point rather than encouraging deep imports.
- Existing `src/shared/codex-desktop/**` is the reference implementation.
- New neutral extraction is allowed only when both generations need the same domain logic or when moving logic there removes an otherwise-invalid dependency.

This is the explicit compatibility bridge. V2 must never import `src/components/**`, `src/hooks/**`, `src/lib/**`, or other leftover UI directly.

### 3.3 Leftover renderer

Do not perform a repository-wide path migration solely to rename the old frontend. Instead, modularize the high-coupling areas where maintenance cost is real.

Target responsibilities for a leftover feature are conceptually:

```text
feature/
  api/ or port/         host-facing operations
  model/                state, validation, transformations
  ui/                   feature UI
  index/public facade   stable exports to composition code
```

The first high-value targets are:

- application/shell orchestration currently concentrated in `src/App.tsx`;
- provider editing/creation forms and provider configuration utilities;
- settings/session orchestration where large components mix data and presentation.

Compatibility exports are acceptable during migration. The goal is reduced coupling and stable ownership, not path churn.

### 3.4 Renderer platform boundary

V2 keeps direct `@tauri-apps/*` imports under `src/v2/shared/platform/tauri/**` only.

For leftover code:

- consolidate direct `invoke`/event/window operations behind existing `src/lib/api/**` or a purpose-specific platform adapter;
- bootstrap-only Tauri lifecycle code may remain at the entry boundary when wrapping it would not improve testability;
- React feature components should not add new raw command strings.

Architecture tests/lint rules will enforce the high-value boundaries rather than relying on convention alone.

## 4. Rust host target architecture

### 4.1 Stay a single crate initially

Use Rust module privacy as the primary architectural boundary:

```text
public Tauri command adapter
        |
        v
module facade / application service
        |
        +--> domain/responsibility submodules
        +--> persistence/config/platform adapters
```

Do not create traits merely to imitate hexagonal architecture. Introduce a trait/port only when there are multiple implementations, a real substitution boundary, or a test seam that cannot be achieved cleanly otherwise.

### 4.2 Command layer

Tauri commands should be thin transport adapters:

- deserialize/validate transport-level input;
- obtain application state;
- call one module/application operation;
- map errors/DTOs;
- avoid owning persistence, filesystem, provider mutation, or proxy orchestration logic.

`commands/misc.rs` will be eliminated or reduced by moving unrelated responsibilities into named command modules while preserving command names.

`lib.rs` remains the composition root but startup/window/single-instance/command-registry concerns should be moved behind named modules when that reduces review surface without fighting Tauri's single `generate_handler!` registration requirement.

### 4.3 Service/domain hotspots

Existing public entry types remain stable where practical (`ProviderService`, `SkillService`, `ProxyService`), but their implementations move into responsibility submodules. Example target shapes:

```text
services/provider/
  mod.rs                facade + stable shared types
  mutation.rs           add/update/delete/switch transactions
  quick_setup.rs        quick-setup validation/transaction
  common_config.rs      shared config extraction/scrubbing
  imports.rs            live/import/startup migration flows
  universal.rs          universal-provider operations
  endpoints.rs          existing endpoint ownership
  usage.rs              existing usage ownership
  live.rs               existing live-config helpers
  tests.rs or tests/    private unit tests

services/skill/
  mod.rs                facade + stable types
  install.rs            install/uninstall/update
  assignment.rs         target synchronization/toggle
  backup.rs             backup/restore
  discovery.rs          repositories/discovery/SkillHub
  archive.rs            bounded download/extract/copy safety
  repository.rs         repo lock/config handling
  tests.rs or tests/

services/proxy/
  mod.rs                facade + ProxyService
  lifecycle.rs          start/stop/recovery
  takeover.rs           takeover/restore/backup
  switching.rs          provider hot-switch orchestration
  live_config.rs        provider-specific live projection
  settings.rs           proxy config/status operations
  tests.rs or tests/
```

Exact file names may change during implementation if source-level dependencies show a better cohesion boundary; the responsibility boundaries above are the contract.

### 4.4 Proxy pipeline

`proxy/forwarder.rs` and `proxy/handlers.rs` are decomposed by request-processing responsibility/protocol, not arbitrary line count.

Likely boundaries:

- retry/failover execution;
- request preparation and endpoint rewriting;
- auth/header/body override handling;
- response validation/error normalization;
- Claude/Codex/Gemini protocol handlers;
- stream aggregation/conversion;
- logging/diagnostic helpers.

The public router-facing API remains narrow. Protocol-specific implementation modules should not become general service dependencies.

### 4.5 Codex configuration

`codex_config.rs` is a platform-sensitive hotspot and already has a failing repository structure digest before this task. Decomposition must preserve behavior and update the repository's structure-identity contract intentionally.

Responsibilities to separate when source dependencies permit:

- file/path I/O and atomic persistence;
- feature/capability patching;
- auth extraction/projection;
- model catalog construction/cache;
- proxy route/session-bucket projection;
- tests.

## 5. Visibility and public APIs

### Rust

- Prefer private child modules.
- Re-export the small API consumers actually need.
- Replace deep `crate::services::<domain>::<internal>` dependencies with facade imports where practical.
- Use `pub(crate)`, `pub(super)`, or private visibility for internal contracts; plain `pub` is reserved for items that genuinely need the wider visible surface.

### TypeScript

- V2 slice/layer imports remain governed by the existing architecture test and ESLint restrictions.
- Neutral `src/shared/**` modules expose intentional entry points.
- Leftover feature modules should expose a stable feature entry rather than letting composition code reach through arbitrary internal files when the refactor touches that area.

## 6. Migration strategy

Use a Strangler-Fig-style sequence:

1. Freeze/encode the desired dependency boundaries.
2. Introduce facades/adapters while old implementation is still present.
3. Move one responsibility at a time behind the facade.
4. Run focused tests after each movement.
5. Remove compatibility paths only when all consumers have moved.

No stage should require an all-at-once renderer or host cutover.

## 7. Architecture enforcement

Prefer executable rules with low false-positive cost:

- extend V2 architecture tests rather than inventing a second checker;
- add a cross-generation shared-boundary test so `src/shared/**` cannot import V2 or leftover UI;
- add leftover direct-Tauri restrictions for feature/UI code where existing adapters cover the operation;
- add Rust source-contract tests only for stable rules that the compiler cannot express (for example no catch-all `commands/misc.rs` reintroduction or no forbidden deep dependency from command modules);
- rely on Rust privacy for the rest rather than regex-testing every directory shape.

## 8. Compatibility and rollback

- Each large decomposition is behavior-preserving and should be revertible as one stage commit.
- Public command strings/DTOs stay stable so renderer and Rust commits can be staged independently where possible.
- Compatibility re-exports are allowed temporarily; annotate only when the reason is not obvious from structure.
- If a module split reveals an undocumented behavior dependency, preserve the behavior first and record the boundary before further cleanup.

## 9. Crate-split decision rule

Do not split a Rust module into a separate crate during this task unless all of these are true:

1. its dependency direction is already one-way and stable;
2. it has a narrow public API independent of Tauri application state internals;
3. independent compilation/testing materially improves the project;
4. moving it does not create DTO/config duplication or a circular workspace dependency.

If those conditions are not met, a private module is the better boundary.

## 10. Expected architectural outcome

- V2 remains a well-guarded production renderer rather than being disrupted.
- Shared cross-generation logic is explicit and neutral.
- Leftover high-coupling features become locally understandable and the legacy composition root shrinks.
- Rust command files become transport adapters rather than application logic owners.
- Provider/Skill/Proxy/Codex/protocol hotspots become responsibility-oriented module trees with narrow facades.
- Architecture rules are executable so future changes cannot silently collapse the boundaries again.
