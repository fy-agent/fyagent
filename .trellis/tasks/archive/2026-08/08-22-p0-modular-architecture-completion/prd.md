# P0 modular architecture completion

## Goal

Complete the four P0 architecture improvements identified by the post-phase-2 audit so FyAgent keeps a clear modular-monolith growth path without expanding into P1/P2 refactors. The work is structural: improve ownership and change locality while preserving product behavior, wire formats, state/transaction ordering, platform security contracts, and renderer generation boundaries.

## Requirements

1. Keep work on `dev/laiyongjie`; do not create new Cargo crates or new frontend frameworks/state managers.
2. Split `src-tauri/src/services/tooling.rs` into private responsibility modules behind the existing Tooling service/four Tauri command surface. Preserve command names, lifecycle behavior, platform discovery, Windows fail-closed policy, terminal launch behavior, and tests.
3. Continue `SkillService` decomposition along independently testable domain seams. At minimum isolate repository/assignment-or-sync/backup-or-migration/install planning where dependency evidence supports it; archive extraction, symlink/traversal/resource budgets, materialization order, and backup-before-delete safety must remain under one transaction/security owner.
4. Continue `codex_config` decomposition by extracting the native feature/capability policy and model catalog responsibility behind the existing `codex_config` facade. Preserve lossless TOML behavior, auth/storage ownership, catalog path confinement, proxy/live/session/MCP semantics, and existing callers.
5. Split `src/v2/shared/features/types.ts` by existing product domains while preserving the stable `FeaturePorts` composition API and all current imports/behavior. Do not create a giant barrel that hides dependency direction.
6. Add/adjust executable architecture rules for durable ownership boundaries introduced by this task. Avoid arbitrary line-count assertions.
7. Do not extend scope into ProxyService/Forwarder large-scale physical splitting, legacy mass migration, V2 route-page physical decomposition, UsageStats, `lib.rs`, or database schema restructuring unless required to keep a P0 boundary compiling.
8. Use focused staged commits with tests after each P0 area. No product behavior change is intended.
9. Before archive, update the owning backend/frontend Trellis SPECs with the resulting long-term module ownership, public/private visibility, facade and dependency rules.
10. Run the complete repository quality gates plus V2-specific gates on the final product state and prearchive gate before archive.
11. Archive the Trellis task and record the session journal before the planned remote delivery.
12. Perform one planned push after archive/journal. The exact final archived HEAD must be pushed to `origin/dev/laiyongjie`, and GitHub `CI / Required` for that exact SHA must complete successfully.
13. Leave `dev/laiyongjie` clean and synchronized with `origin/dev/laiyongjie`.

## Acceptance Criteria

- [ ] Tooling transport remains four commands and Tooling implementation is organized into private responsibility modules with a stable facade.
- [ ] Skill has additional private domain owners and retains a single archive/filesystem safety transaction owner.
- [ ] Codex feature/capability and catalog responsibilities are private modules behind `codex_config` with no caller-facing wire/persistence change.
- [ ] V2 feature contracts are product-domain owned; `FeaturePorts` callers remain stable and V2/leftover/shared dependency rules remain intact.
- [ ] New module boundaries are protected by Rust visibility and/or executable architecture tests.
- [ ] No intended Tauri command, serialized DTO, event, database/config format, credential boundary, rollback order, proxy behavior, or user-visible renderer behavior changes.
- [ ] Focused affected tests pass after each stage.
- [ ] `mise run check`, `mise run lint:v2`, `mise run typecheck:v2`, and `mise run test:v2` pass on final product state.
- [ ] Relevant frontend/backend SPEC updates are committed before archive and their architecture checks pass.
- [ ] Trellis task is archived and journal recorded before the planned push.
- [ ] Final archived/journal HEAD is pushed and `CI / Required = success` for that exact SHA.
- [ ] Working tree is clean; local and remote branch heads match.

## Out of Scope

- P1/P2 architecture items from the audit.
- Product feature work, wire/storage migrations, or UI redesign.
- Cargo workspace/crate proliferation solely for source-file size.
- Mechanical source splitting that increases cross-module state/transaction coupling.
