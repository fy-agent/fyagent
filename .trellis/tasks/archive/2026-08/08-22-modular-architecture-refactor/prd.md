# Modular architecture refactor

## Goal

Refactor FyAgent into a maintainable, professionally engineered modular architecture without changing the product intent. The work should reduce oversized and mixed-responsibility modules, make dependency directions explicit, and create durable boundaries that future feature work can follow.

The refactor may substantially change the Rust backend. The frontend must preserve the current coexistence model where V2 and the legacy renderer can both exist and V2 may still depend on selected legacy functionality during migration.

## Confirmed Facts

- The repository is a Tauri 2 desktop application with a Rust host and React/TypeScript renderer.
- `src/index.html` loads `src/v2/main.tsx`; V2 is the production renderer while the old renderer remains as tested leftover/compatibility code.
- V2 already enforces an explicit layer/platform boundary and its only tree-external production dependency is the renderer-neutral `src/shared/codex-desktop/**` core.
- The backend already has top-level areas such as `commands`, `services`, `proxy`, `database`, `mcp`, `deeplink`, and platform/runtime modules, but several modules have grown into multi-thousand-line responsibility clusters.
- The current branch is `dev/laiyongjie`; the working tree was clean when planning started.
- The user authorizes broad refactoring, staged commits, fixes needed to make CI pass even when adjacent failures are outside the primary refactor scope, final push, and completion/archival on `dev/laiyongjie`.
- Architecture choices must be backed by repository evidence and external research rather than intuition alone.
- The pre-change `mise run check` baseline passes environment/type/format checks but has two existing unit failures in `tests/remainingPlatformSurface.test.ts` because the recorded structure identity for `src-tauri/src/codex_config.rs` has drifted.

## Requirements

- Perform an evidence-backed architecture audit before implementation, including dependency structure, high-coupling hotspots, oversized mixed-responsibility modules, IPC boundaries, and legacy/V2 interactions.
- Use authoritative or high-quality external references and representative mature projects to validate architecture choices.
- Prefer high-cohesion modules with explicit public boundaries and controlled dependency directions over mechanical file splitting.
- Use an incremental modular-monolith migration rather than a big-bang rewrite; compatibility facades are allowed while consumers move.
- Backend refactoring may be broad, including command/service/domain/infrastructure boundaries, module visibility, DTO placement, proxy/provider decomposition, and crate boundaries where justified by evidence.
- Frontend refactoring may redesign leftover and V2 internals independently, but V2's existing production boundaries must remain intact and any cross-generation reuse must live behind an intentional renderer-neutral shared API.
- Direct platform access from feature/UI code should be reduced behind typed ports/facades where appropriate, especially for Tauri IPC.
- Avoid a flag-day migration. Preserve behavior through staged, verifiable changes and compatibility shims when needed.
- Add or strengthen architecture guardrails where useful so dependency boundaries do not immediately regress after the refactor.
- Preserve user-visible behavior unless a behavior change is necessary to correct a defect or is explicitly documented as part of the refactor.
- Use staged commits with validation at meaningful boundaries.
- Before final archive, run the repository's relevant quality gates, push `dev/laiyongjie`, and verify CI. If CI failures are caused by adjacent repository issues that block the branch, fix them as authorized and re-run validation.

## Acceptance Criteria

- [ ] A documented current-state module/dependency assessment identifies the major coupling and responsibility hotspots that drive the refactor.
- [ ] The final architecture has explicit module ownership and dependency directions for both renderer and Rust host.
- [ ] Legacy frontend and V2 remain functional during coexistence; any V2-to-legacy dependency is isolated behind an intentional compatibility/public API boundary rather than arbitrary deep imports.
- [ ] High-value oversized/mixed-responsibility frontend and backend hotspots are decomposed by responsibility, not just by line count.
- [ ] Tauri IPC usage follows an explicit boundary so React feature code does not need uncontrolled direct knowledge of host command implementation details.
- [ ] Backend command entry points are thin relative to application/domain logic, and catch-all modules are reduced or given clear ownership.
- [ ] Provider/proxy/skill and other large backend areas have clearer submodule APIs and restricted visibility where practical.
- [ ] Automated checks or lint/tests enforce important new architecture boundaries where practical.
- [ ] Existing relevant unit, typecheck, lint, renderer/browser, Rust, and desktop acceptance checks pass, with any intentionally excluded checks documented.
- [ ] The pre-existing `codex_config.rs` structure-identity baseline failure is intentionally repaired and no longer blocks the full local/remote quality gate.
- [ ] Work is committed in reviewable stages, final branch remains `dev/laiyongjie`, changes are pushed, and remote CI is green before task archival.

## Out of Scope

- Rewriting the product into microservices or introducing distributed runtime boundaries solely for architectural aesthetics.
- A mandatory split into many Rust crates before module boundaries are proven useful inside the existing application.
- Intentional UX redesign unrelated to preserving or enabling the modular refactor.
- Removing the legacy frontend before V2 has replacements for capabilities it still depends on.
- Mass-moving every leftover frontend file solely to produce a cosmetically uniform directory tree when the move does not reduce coupling or improve ownership.
