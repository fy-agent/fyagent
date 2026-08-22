# Full-stack Modular Maintainability Refactor — Implementation Plan

## Execution policy

- Work only on `dev/laiyongjie`.
- Product behavior is frozen; this task is structural unless a separately proven defect requires a regression-tested correction.
- Use `dev.apply_patch` for every modification and check `git status` before each stage.
- Use reviewable stage commits; a stage is committed only after its focused validation passes.
- Do not push during implementation. SPEC review/update, Trellis archive and session journal happen before the planned single remote push.
- Do not weaken a sealed platform/security test to accommodate a move; update ownership/path/digest evidence only after preserving the underlying invariant.

## Stage 0 — Baseline and architecture guardrails

### Work

- Persist measured audit/research evidence in the task.
- Extend architecture tests to support multiple V2 Tauri feature-adapter files instead of assuming a single `features.ts` source.
- Add durable rules that keep the root V2 Tauri `features.ts` as composition and prevent command/tooling implementation from drifting back into Tauri transport.
- Run a focused baseline against current product code before structural moves.

### Validation

```bash
mise run typecheck
mise run test:v2
mise run test:unit -- tests/architecture/rendererBoundaries.test.ts tests/architecture/rustModuleBoundaries.test.ts tests/v2/platform/tauriAclContract.test.ts
```

### Commit gate

Architecture-test changes are their own commit.

## Stage 1 — V2 Tauri FeaturePorts adapters

### Work

- Split capability parsers/validators/adapters out of `src/v2/shared/platform/tauri/features.ts`.
- Retain `createTauriFeaturePorts()` and the existing production import path as the stable composition facade.
- Keep only genuinely cross-port guards in a shared adapter helper; do not create a generic dumping ground.
- Update feature-port tests/ACL scanner for the adapter tree while preserving exact command set and parser behavior.

### Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:unit -- tests/architecture/rendererBoundaries.test.ts
```

### Rollback point

One commit containing only V2 adapter movement/tests.

## Stage 2 — V2 route-local decomposition

### Work

- Models: extract WorkBuddy and Provider panels from `Page.tsx`.
- Skills: extract installed detail/discovery/dialog ownership.
- MCP: extract server detail/editor.
- Memory: extract long-term/daily modules.
- Prompts: extract editor pane/identity form where it improves route scanability.
- Keep route-level query/selection/dirty-blocker state at the nearest common owner; no new global state dependency.
- Reuse existing V2 shared UI and FeaturePorts; no copy of shared chrome.

### Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
```

Run focused route tests after each route movement before the aggregate V2 gate.

### Commit gate

Prefer one or two reviewable commits (Models first if substantial; remaining routes second).

## Stage 3 — Rust Tooling service ownership

### Work

- Create private `services/tooling` ownership for version/lifecycle/discovery/terminal behavior.
- Reduce `commands/tooling.rs` to four thin Tauri wrappers plus transport-only definitions where necessary.
- Move `commands/hermes.rs` terminal dependency to the service/facade.
- Keep command names, arguments, serialized responses, platform launch/security behavior and command registration unchanged.
- Update Windows fail-closed/platform scanners and sealed source ownership intentionally if moved code is protected.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/remainingPlatformSurface.test.ts tests/codexWindowsUserScopeContract.test.ts tests/desktopSecurityBoundary.test.ts
```

### Commit gate

Independent backend commit before other service decompositions.

## Stage 4 — Skill service decomposition

### Work

- Preserve `SkillService` and command-facing DTO behavior.
- Move marketplace/repository/assignment/backup/migration responsibilities into private child modules where call graph permits.
- Consolidate ZIP/vendor-copy/symlink/resource-budget behavior under one archive-safety owner; preserve ordering and limits.
- Move large private test families alongside their owning modules where this improves locality without reducing test coverage.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:v2
mise run test:unit -- tests/components/UnifiedSkillsPanel.test.tsx tests/components/SkillsPageInstall.test.tsx
```

### Rollback point

Archive-safety movement is not combined with unrelated Provider/Proxy work.

## Stage 5 — Codex config decomposition

### Work

- Extract native capability/features, auth, catalog and live/projection responsibilities behind the existing `codex_config` facade.
- Preserve lossless TOML edits, `auth.json` ownership/rollback, model-catalog path confinement, unified-session/MCP projection and proxy-route cleanup.
- Keep platform-sensitive scanner/digest evidence in sync only after focused behavior tests pass.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/remainingPlatformSurface.test.ts tests/codexDesktopDtoContract.test.ts tests/lib/providersApi.codexFeatures.test.ts
```

### Commit gate

Codex config is its own commit due to platform sensitivity.

## Stage 6 — Proxy protocol handlers and Provider responsibilities

### Work

- Split `proxy/handlers.rs` by Claude/Codex/Gemini protocol behind stable handler re-exports.
- Extract Provider `common_config` and other independently proven quick-setup/import/mutation responsibilities while preserving one mutation/rollback coordinator.
- Reassess `provider/live.rs`; split by app only if the call graph/test seam is clearly one-way.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/remainingPlatformSurface.test.ts
```

Run Provider integration tests and full Rust suite before each commit.

### Commit gate

Handlers and Provider may be separate commits if either diff is substantial.

## Stage 7 — Selective leftover renderer and remaining Rust hotspots

### Work

- Re-audit `ProviderForm`, `providerConfigUtils`, `App`, WebDAV and Session Manager after production V2 work. Extract only independently testable controller/pure logic; avoid mass legacy migration.
- Re-audit `services/proxy.rs`, `proxy/forwarder.rs`, `usage_stats.rs` and `lib.rs` after earlier extractions.
- Extract only pure decision/query/backfill/lifecycle boundaries that do not fragment transaction/pipeline ownership.
- Explicitly document retained large cohesive modules when no safe seam exists.

### Validation

Use focused tests for each touched owner, then:

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

### Stop rule

If the only remaining benefit is fewer lines/files while coupling or transaction ownership would worsen, do not extract it.

## Stage 8 — Full quality check and architecture audit

### Work

- Run `trellis-check` against the complete diff.
- Recompute major frontend import graph and Rust hotspot/public-surface measurements to confirm actual locality/ownership improvement.
- Review all stage commits/diff for accidental product behavior, unrelated churn, new deep imports, barrels or warning suppression.
- Run the complete repository local gate plus V2-specific gate.

### Validation

```bash
mise run check
mise run lint:v2
mise run typecheck:v2
mise run test:v2
```

Add any focused native/security tests identified by changed SPECs.

## Stage 9 — Mandatory SPEC update, final commits and archive

### Work

- Run the Trellis spec-update review **before archive**.
- Update `.trellis/spec/backend/modular-boundaries.md` with concrete new Rust ownership/facade/test rules.
- Update `.trellis/spec/frontend/modular-boundaries.md`, V2 shell/reuse or another owning frontend SPEC when adapter/route ownership changes.
- Update indexes only if new spec files are introduced.
- Re-run the SPEC-owned architecture/quality tests.
- Commit final product/SPEC/task-outcome changes.
- Use Trellis finish/archive flow and record the session journal. Archive/journal commits must be local before remote delivery.

### Archive gate

Do not archive while product/SPEC code is dirty or any applicable local gate is failing.

## Stage 10 — Single planned push and CI convergence

### Work

- Confirm local branch is `dev/laiyongjie`, clean, and final HEAD includes task archive/session journal.
- Push `dev/laiyongjie` once.
- Confirm `origin/dev/laiyongjie` resolves to that exact SHA.
- Watch the `CI` run for the exact SHA until `CI / Required` completes.
- If a remote-only failure requires a fix, classify it first. Any corrective code change must itself follow Trellis planning/check/spec/commit/archive discipline before another unavoidable corrective push; do not amend an already-delivered archived history silently.

### Final acceptance

- final pushed SHA is the archived/journal HEAD;
- `CI / Required = success` for that exact SHA;
- branch remains `dev/laiyongjie`;
- local/remote heads match;
- working tree clean.
