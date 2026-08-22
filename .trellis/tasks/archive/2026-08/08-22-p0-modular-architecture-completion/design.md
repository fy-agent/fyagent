# P0 Modular Architecture Completion — Design

## Design objective

Finish the remaining P0 skeleton of the modular monolith. Optimize for ownership and future extension points, not average file size.

## 1. Tooling

Keep the public service entry points and four command wrappers stable. `services/tooling` becomes a private module tree. Target responsibilities:

```text
services/tooling/
├── mod.rs        # stable crate-level service facade / shared DTOs
├── versions.rs   # local/remote version resolution and semver policy
├── lifecycle.rs  # install/update planning/execution
├── discovery.rs  # executable search paths, installations and source inference
└── terminal.rs   # provider terminal/shell/platform launch policy
```

Exact helper placement follows the call graph. Cross-cutting Windows safety primitives stay with the operation they protect; do not duplicate policy.

## 2. Skill

`SkillService` remains the external facade/transaction coordinator. Add private domain owners only where one-way dependencies are provable. Preferred seams are repository metadata/lock handling, assignment/synchronization, backup/migration and installation planning.

The archive/materialization security boundary is singular:

```text
download/zip input
  -> traversal/symlink/resource validation
  -> materialization/vendor copy
  -> backup/delete/replace ordering
```

Do not spread this sequence across peer modules merely to reduce line count.

## 3. Codex Config

Keep `codex_config::*` as compatibility facade. Existing `storage.rs` and `auth.rs` stay private. Extract:

```text
codex_config/
├── features.rs   # image/websocket/native capability analysis, patch, validation, warnings
└── catalog.rs    # model template/loading/generation/readback/path-confinement helpers
```

Live/provider/proxy/session projection remains in the parent unless extraction is necessary for the feature/catalog modules. Public re-exports are explicit and limited to current callers.

## 4. V2 Feature Contract

The Tauri adapter is already capability-owned. Make the contract side symmetrical without introducing a broad barrel. Split the current mixed `types.ts` into product-domain files under `src/v2/shared/features/` (exact names follow existing domains), while retaining a deliberately small compatibility surface where needed.

Dependency direction stays:

```text
route/page -> feature contract/query -> FeaturePorts -> platform adapter
```

No V2 dependency on legacy implementation and no runtime code in neutral `src/shared/**`.

## Architecture stop rules

- Large state machines/pipelines are not P0 targets.
- Do not add traits/interfaces without a real alternate implementation/test seam.
- Do not replace direct imports with a giant `index.ts` that hides ownership.
- Do not weaken sealed platform/security tests for source movement.
- If a planned sub-split creates bidirectional dependencies or pushes transaction state through several peer modules, retain the cohesive owner and document the decision.

## Delivery

Each P0 area gets an independent commit after focused validation. Final full gates run before SPEC update/finish. SPEC changes and task outcome are committed before archive. Archive and journal commits precede the only planned push.
