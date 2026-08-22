# P0 Modular Architecture Completion — Implementation Plan

## Stage 0 — Guardrails and dependency proof

- Reconfirm exact current public surfaces/call graphs for Tooling, Skill, Codex and V2 feature contracts.
- Add only architecture checks that pass before movement and protect durable boundaries (no arbitrary LOC checks).
- Focused baseline: architecture tests + V2 tests + Rust compile.

## Stage 1 — Tooling private module tree

- Create `services/tooling/` with private `versions`, `lifecycle`, `discovery`, `terminal` owners as supported by call dependencies.
- Keep command surface and service callers stable.
- Move tests with their owner when practical; preserve Windows/platform scanners and sealed assets.
- Validate fmt/check/clippy, Tooling tests, Windows user-scope/security and architecture tests.
- Commit independently.

## Stage 2 — Skill domain owners

- Extract additional one-way Skill responsibilities, preferring repository metadata/lock, assignment/sync, backup/migration or install planning.
- Keep archive/symlink/traversal/resource/materialization ordering under one security/transaction owner.
- Preserve `SkillService` command-facing API.
- Validate Skill unit/integration, architecture and supported-platform gates.
- Commit independently.

## Stage 3 — Codex features and catalog

- Extract `codex_config/features.rs` and `codex_config/catalog.rs` behind parent facade.
- Preserve feature diagnostics/patch validation, model catalog template/cache/path confinement, auth/storage and live projection semantics.
- Tighten visibility; do not publish test-only helpers.
- Validate Codex focused tests, Provider/Codex integration, supported-platform, clippy/full Rust.
- Commit independently.

## Stage 4 — V2 feature contract domains

- Split mixed feature DTO/constants into product-domain owners while preserving FeaturePorts and current runtime behavior.
- Update direct imports deliberately; use at most a small compatibility facade, not a wildcard barrel.
- Add architecture enforcement for contract ownership/dependency direction if it can be low false-positive.
- Validate V2 lint/typecheck/test, full TypeScript typecheck, focused architecture tests.
- Commit independently.

## Stage 5 — Full quality / audit

- Run `mise run check`.
- Run `mise run lint:v2`, `mise run typecheck:v2`, `mise run test:v2`.
- Recompute relevant hotspots/import cycles/public surfaces and document the final outcome, including any intentionally retained cohesive code.
- Review staged history for accidental product behavior or P1/P2 scope creep.

## Stage 6 — Mandatory SPEC update and finish

- Update `.trellis/spec/backend/modular-boundaries.md` and `.trellis/spec/frontend/modular-boundaries.md` (and owning indexes/specs only if actually needed).
- Run SPEC-owned architecture checks.
- Commit SPEC/task outcome.
- Run `check:prearchive --exclude-active-task <task>` under authoritative Trellis session context.
- Archive task and record session journal.

## Stage 7 — Single planned push / CI

- Confirm clean `dev/laiyongjie`, archived task, journal commit and no remote divergence.
- Push once.
- Confirm `origin/dev/laiyongjie` equals exact local SHA.
- Monitor the exact SHA CI run until `CI / Required` succeeds; fix only if a real failure requires a corrective cycle.
- Reconfirm clean/synchronized checkout.

## Implementation outcome

- [x] Tooling has private `versions`, `lifecycle`, `discovery`, and `terminal`
  owners behind the unchanged four-command/facade surface.
- [x] Skill added repository, migration, and assignment owners without splitting
  the archive/filesystem safety transaction.
- [x] Codex feature and full catalog domains are private behind the existing
  facade; live/proxy/session ordering remains in the parent coordinator.
- [x] V2 feature contracts are product-domain owned and `types.ts` is an
  explicit compatibility facade protected by architecture tests.
- [x] Platform scanners/sealed assets and Windows security contracts follow the
  new sensitive owners without weakening fail-closed behavior.
- [x] `mise run check` passed on the final product state.
- [x] V2 lint/typecheck/test passed; 278/278 tests passed.
- [x] Concrete workstation user-home paths are rejected from tracked docs /
  Trellis artifacts by an executable repository contract while semantic
  placeholders remain allowed.
- [x] Mandatory SPEC/outcome commit completed.
- [ ] Prearchive gate completed.
- [ ] Task archived and session journal recorded.
- [ ] Final archived/journal HEAD pushed and exact-SHA `CI / Required` green.
