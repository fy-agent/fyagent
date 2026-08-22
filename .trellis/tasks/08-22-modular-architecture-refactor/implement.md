# Modular Architecture Refactor Implementation Plan

## Execution policy

- Work only on `dev/laiyongjie`.
- Preserve existing user-visible behavior and wire/storage contracts unless an existing defect requires a tested correction.
- Use small, reviewable stage commits. A stage is complete only after its focused validation passes.
- Do not overwrite unrelated user changes. Re-check `git status` before every stage.
- Treat the pre-existing `remainingPlatformSurface` digest failure as baseline debt to repair explicitly, not as a refactor regression.

## Stage 0 — Baseline and architecture guardrails

### Work

- Record the current dependency/hotspot evidence in task artifacts.
- Add/extend architecture tests for:
  - V2 isolation remaining unchanged;
  - renderer-neutral `src/shared/**` not depending on V2 or leftover UI;
  - no new raw Tauri command access from V2 outside its adapter;
  - targeted leftover UI/feature code using existing API/platform facades instead of adding new direct invoke calls.
- Repair the already-drifting `codex_config.rs` supported-platform structure identity contract using the repository's intended update mechanism, after verifying the current file is legitimate.

### Validation

```bash
mise run typecheck
mise run lint:v2
mise run test:v2
mise run test:unit -- tests/v2/app/architecture.test.ts tests/remainingPlatformSurface.test.ts
```

### Commit gate

Commit architecture guardrails/baseline repair separately before structural moves.

## Stage 1 — Renderer compatibility boundary

### Work

- Keep `src/v2/**` layer structure intact.
- Harden `src/shared/**` as the renderer-neutral bridge with explicit public entry points.
- Consolidate the remaining high-value direct leftover `invoke` calls behind existing `src/lib/api/**`/platform adapters when they are not bootstrap-only lifecycle operations.
- Ensure V2 continues to depend only on V2 internals plus explicitly approved neutral shared modules.

### Validation

```bash
mise run typecheck
mise run lint:v2
mise run test:v2
mise run test:unit -- tests/shared tests/v2/app/architecture.test.ts
```

### Rollback point

Renderer adapter/facade changes remain one commit independent from legacy component decomposition.

## Stage 2 — Leftover frontend high-coupling decomposition

### Work

- Shrink `src/App.tsx` by extracting feature orchestration/composition responsibilities while preserving the old entry behavior used by tests.
- Break the `ProviderForm.tsx` / `GrokBuildProviderForm.tsx` cycle and move provider-specific state/validation/transformation ownership out of the giant form component.
- Split provider configuration logic into cohesive model/config units rather than a single catch-all utility.
- Refactor additional large leftover settings/session components only where the same coupling pattern is confirmed; avoid churn on low-value dead/isolated code.
- Add feature-level public entry/facade points for newly modularized leftover areas.

### Validation

```bash
mise run typecheck
mise run format:check
mise run test:unit -- tests/integration/App.test.tsx tests/components tests/hooks src/utils/providerConfigUtils.test.ts
```

Run the full unit suite before the stage commit if provider/shared APIs changed broadly.

### Commit gate

Prefer separate commits for App composition and Provider domain decomposition if both diffs are substantial.

## Stage 3 — Rust composition and command boundary

### Work

- Split `commands/misc.rs` by named responsibility while preserving Tauri command names.
- Reduce command-module business logic where commands directly own filesystem/config/service orchestration that belongs to an application/service module.
- Extract review-heavy startup/window/single-instance/command-registration helpers from `lib.rs` when doing so leaves `lib.rs` as a clearer composition root without fighting Tauri's handler model.
- Replace avoidable broad/deep service imports from command modules with stable service facade imports.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/desktopSecurityBoundary.test.ts tests/singleInstanceActivationContract.test.ts
```

### Commit gate

Commit command/composition refactor before service-internal decompositions.

## Stage 4 — Provider and Codex configuration modules

### Work

- Turn `services/provider/mod.rs` into a real facade and move mutation, quick setup, common-config, import/migration, universal-provider, and test responsibilities into child modules.
- Keep existing endpoint/usage/live submodules where ownership is already good; tighten visibility/imports.
- Decompose `codex_config.rs` by capability/auth/catalog/proxy-route/persistence responsibilities where dependency analysis supports it.
- Move large inline tests into child test modules so production modules remain reviewable without weakening private-unit coverage.
- Update supported-platform structure contracts intentionally for moved platform-sensitive code.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/remainingPlatformSurface.test.ts tests/codexDesktopDtoContract.test.ts tests/lib/providersApi.codexFeatures.test.ts
```

### Rollback point

Provider and Codex-config decomposition should be separate commits if either produces a large diff.

## Stage 5 — Skill service modules

### Work

- Split `services/skill.rs` into install/update, assignment/sync, backup/restore, discovery/repository, archive/security, and tests according to actual call dependencies.
- Keep `SkillService`/public DTO behavior stable.
- Restrict filesystem/archive helpers to the narrowest module visibility that still supports tested workflows.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:v2
mise run test:unit -- tests/components/UnifiedSkillsPanel.test.tsx tests/components/SkillsPageInstall.test.tsx tests/v2/features/featurePages.test.tsx
```

## Stage 6 — Proxy service and protocol pipeline

### Work

- Split `services/proxy.rs` into lifecycle, takeover/restore, switching, live-config projection, and settings/status ownership.
- Split `proxy/forwarder.rs` into retry/failover, request preparation, endpoint/header/body transformation, response validation/error handling, and diagnostics where call graph supports it.
- Split `proxy/handlers.rs` into protocol-specific handlers and stream aggregation/conversion helpers.
- Keep router-facing and command-facing APIs narrow and stable.

### Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit
```

Proxy changes are high risk: run the full Rust suite at each independently meaningful commit, not just at the end.

## Stage 7 — Visibility cleanup, documentation, and architecture contracts

### Work

- Reduce unnecessary module/item visibility exposed across the Rust crate.
- Replace remaining deep imports in touched modules with facade imports.
- Remove compatibility re-exports that have no remaining consumers.
- Update `.trellis/spec/` with durable module-boundary rules learned during the refactor.
- Update maintained developer documentation only where source paths/contracts materially changed.
- Ensure no placeholder/generic architecture prose is added to project docs; rules must point to real code/tests.

### Validation

```bash
mise run check
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:desktop:mock
mise run test:desktop:visual:preflight
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

Run browser/Playwright checks if V2 shell/page behavior changed rather than only types/adapters.

## Stage 8 — Final integration, push, and CI convergence

### Work

- Review staged commit history and full diff for accidental behavior changes or unrelated churn.
- Run the complete repository-supported local quality gate.
- Push `dev/laiyongjie`.
- Inspect remote CI results.
- If CI fails:
  - first classify whether the failure is introduced by the refactor, pre-existing, platform-specific, or unrelated repository drift;
  - fix failures blocking this branch as authorized by the user, even when adjacent to the primary refactor scope;
  - validate locally where reproducible;
  - commit and push the fix;
  - repeat until required CI is green.
- Keep the final checkout on `dev/laiyongjie`.
- Run Trellis finish/spec-update flow and archive only after CI evidence is green.

## Final acceptance checklist

- [ ] V2 production architecture remains isolated and functional.
- [ ] Cross-generation shared code has an explicit neutral boundary.
- [ ] Major leftover frontend coupling hotspots are reduced without a pointless mass path migration.
- [ ] `commands/misc.rs` no longer acts as a catch-all ownership bucket.
- [ ] Rust Provider/Skill/Proxy/Codex/protocol hotspots are responsibility-oriented module trees with narrow facades.
- [ ] Important dependency rules are compiler- or test-enforced.
- [ ] Relevant frontend, Rust, desktop, and architecture checks pass.
- [ ] Stage commits are reviewable and behavior-preserving.
- [ ] `dev/laiyongjie` is pushed and required CI is green.
- [ ] Trellis task is archived with final evidence.
