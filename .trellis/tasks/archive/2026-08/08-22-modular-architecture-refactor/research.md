# Modular Architecture Research

## Scope

This note records the repository evidence and external architecture research used to choose the refactor strategy. It is planning evidence, not a generic architecture template.

## Repository evidence

### Runtime boundaries

- `src/index.html` loads `src/v2/main.tsx`; V2 is the production renderer composition root.
- The old `src/main.tsx` / `src/App.tsx` renderer is retained as leftover/compatibility code and remains covered by the non-V2 Vitest suite.
- `tests/v2/app/architecture.test.ts` already enforces a useful V2 dependency direction:
  - root -> app
  - app -> app/pages/widgets/shared/dev
  - pages -> pages/shared
  - widgets -> widgets/shared
  - shared -> shared
  - dev -> dev/shared
- V2 direct Tauri imports are already restricted to `src/v2/shared/platform/tauri/**`.
- V2 repository imports outside `src/v2/**` are limited to the intentionally renderer-neutral `src/shared/codex-desktop/**` core.

### Frontend hotspots

TypeScript compiler-API dependency inspection found:

- `src/App.tsx`: 64 static imports and 1,958 lines. It combines shell composition, feature orchestration, queries, dialogs, Tauri calls, provider/proxy/environment coordination, and route-level UI.
- `src/components/providers/forms/ProviderForm.tsx`: 54 static imports and 2,808 lines.
- Other large leftover files include `WebdavSyncSection.tsx`, `SessionManagerPage.tsx`, `UsageScriptModal.tsx`, and `providerConfigUtils.ts`.
- The leftover provider area has especially high cross-area fan-out into `components/ui`, `lib`, `config`, `utils`, `hooks`, and shared types.
- The current static import graph has one strongly connected component: `ProviderForm.tsx` <-> `GrokBuildProviderForm.tsx`.
- There are 47 direct `@tauri-apps/*` imports across the whole renderer; most leftover command calls are already concentrated in `src/lib/api/**`, but direct invokes still escape that facade in a small set of bootstrap/UI files.

### Rust hotspots

Approximate crate-reference analysis and source inspection found:

- `services` and `commands` are the dominant dependency hubs.
- Large production/test files include:
  - `services/provider/mod.rs` ~6.7k lines
  - `services/skill.rs` ~6.9k lines
  - `services/proxy.rs` ~7.4k lines
  - `codex_config.rs` ~6.2k lines
  - `proxy/forwarder.rs` ~5.0k lines
  - `proxy/handlers.rs` ~3.3k lines
- Large files mix production logic and extensive unit tests. Extracting tests alone will improve navigability but is not considered sufficient architectural decomposition.
- `services/mod.rs` exposes 42 service submodules to the crate, enabling broad deep imports instead of a deliberately narrow service API.
- `commands/mod.rs` glob-reexports roughly 40 command modules.
- `commands/misc.rs` is a catch-all containing unrelated system, clipboard, initialization, tool lifecycle, provider terminal, and window-theme responsibilities.
- `src-tauri/src/lib.rs` contains the complete Tauri command registration list plus substantial startup/window/runtime logic, making the composition root harder to review.

### Baseline quality gate

Before product-code changes, `mise run check` was executed.

- Environment guard: pass.
- Typecheck: pass.
- Format check: pass.
- Unit suite: 164 files passed, 1 file failed; 1465 tests passed, 2 failed, 1 skipped.
- Both failures are in `tests/remainingPlatformSurface.test.ts` and predate this refactor. They report a structure-identity digest drift for `src-tauri/src/codex_config.rs`.
- This baseline failure must be repaired before final delivery, but must not be misattributed to the modularization work.

## External evidence

### Feature-first frontend structure and dependency direction

Feature-Sliced Design documents a strict layer import rule: a slice may depend on slices only on lower layers, and slices should expose a public API so internal structure remains refactorable. The project will borrow the dependency/public-boundary principles without adopting every FSD layer or directory name.

Sources:

- https://fsd.how/docs/reference/layers/
- https://fsd.how/zh/docs/reference/public-api/
- https://github.com/feature-sliced/steiger/tree/master/packages/steiger-plugin-fsd/src/public-api

Bulletproof React independently recommends feature-local colocation, avoiding cross-feature imports, composing features at the application level, and enforcing unidirectional dependency flow with lint rules. The project will use this as corroborating evidence, not as a template to copy literally.

Source:

- https://github.com/alan2207/bulletproof-react/blob/master/docs/project-structure.md

### Incremental legacy modernization

Martin Fowler's Strangler Fig material recommends gradual replacement around an existing system rather than a high-risk cut-over rewrite. The 2024 mobile-app case study is especially relevant to FyAgent's V2/leftover coexistence because it discusses modular modernization with an explicit bridge between old and new implementations.

Sources:

- https://martinfowler.com/bliki/StranglerFigApplication.html
- https://martinfowler.com/articles/strangler-fig-mobile-apps.html

### Tauri IPC boundary

Tauri's official guide treats commands as an explicit frontend-to-Rust invocation boundary and recommends grouping commands into separate modules when the number of commands grows rather than bloating `lib.rs`.

Source:

- https://v2.tauri.app/develop/calling-rust/

### Rust module privacy before crate proliferation

The Rust Reference and Rust Book describe modules, privacy, restricted visibility, and re-exports as the mechanisms for hiding implementation details and exposing a deliberate API. Cargo workspaces manage multiple packages together, but a workspace is not itself evidence that code should be split into crates.

Sources:

- https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html
- https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
- https://doc.rust-lang.org/stable/cargo/reference/workspaces.html

## Decision matrix

| Candidate | Benefit | Cost/Risk | Decision |
| --- | --- | --- | --- |
| Big-bang frontend rewrite | Visually clean tree | High regression risk; ignores active V2/leftover coexistence | Reject |
| Move every leftover file under a new `legacy/` directory immediately | Clear physical namespace | Huge path churn with little product value | Reject as first move |
| Preserve V2 layers and enforce a narrow neutral shared bridge | Protects production renderer and allows staged extraction | Requires explicit compatibility ownership | Adopt |
| Feature-first decomposition of high-coupling leftover domains | Improves local ownership without rewriting all leftover code | Temporary compatibility facades may exist | Adopt |
| Direct Tauri calls from feature/page components | Short code path | Couples UI to host protocol and complicates browser tests | Reduce/forbid by boundary |
| Split Rust into many crates immediately | Strong compiler boundaries | Expensive dependency graph and build churn before boundaries are proven | Defer |
| Single-crate modular monolith with private submodules/facades | Compiler-enforced internal boundaries with low operational cost | Requires deliberate visibility cleanup | Adopt |
| Mechanical line-count splitting only | Smaller files | Does not reduce coupling or responsibility | Reject |
| Extract large inline test modules while decomposing production responsibilities | Improves navigation and keeps private-unit testing | File churn | Adopt as supporting cleanup |

## Architectural conclusion

The target is an incremental modular monolith, not a directory-only cleanup:

1. Preserve V2's already-good production boundary and make neutral cross-generation code explicit under `src/shared/**`.
2. Refactor only high-value leftover hotspots into feature-owned orchestration/model/API modules; do not mass-move low-value leftover files solely for aesthetics.
3. Keep Tauri invocation behind renderer platform/API adapters.
4. Reshape Rust hotspots into private responsibility submodules with narrow facades while keeping public command names and serialized contracts stable.
5. Use compiler visibility and architecture tests to make dependency rules executable.
6. Consider extra Rust crates only after an independent package boundary is demonstrated by dependency direction, ownership, testability, and compile/runtime needs.
