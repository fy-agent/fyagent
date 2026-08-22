# Full-stack modular maintainability refactor

## Goal

Perform a second, repository-wide architecture audit of the FyAgent renderer and Rust/Tauri host, then refactor the dependency-proven maintainability hotspots into cohesive modules with explicit ownership and stable boundaries. The outcome should reduce the amount of unrelated code a maintainer must understand for a local change without changing intended product behavior.

## Background

- The previous modularization task established initial architecture guardrails and extracted several low-risk boundaries (`provider/universal`, `skill/discovery`, `proxy/takeover`, `codex_config/storage`) while preserving V2 isolation.
- The repository still contains large frontend and backend modules. File length alone is not sufficient evidence for extraction: tightly coupled transactions/state machines may remain together when execution order is a correctness or security property.
- Production renderer is `src/v2/**`; leftover V1 remains in-tree for compatibility/tests. V2 must not gain imports from leftover implementation. Legitimate cross-generation logic belongs only in renderer-neutral `src/shared/**`.

## Requirements

1. Audit the complete current frontend and backend architecture before product-code edits. Use measured dependencies, responsibilities, public surfaces, test seams, cycles, duplication and state/transaction ownership rather than line-count thresholds.
2. Research current high-quality architecture guidance and relevant reference projects before finalizing the design. Prefer first-party/primary sources where possible; record which principles are adopted or rejected and why they fit FyAgent.
3. Keep FyAgent a modular monolith unless a package/crate boundary is independently justified by one-way dependencies, a narrow API and real compilation/reuse value. Do not create Cargo crates merely to reduce source-file size.
4. Rust refactors must preserve Tauri command names, serialized DTOs, persisted formats, validation order, rollback semantics, credential/security boundaries and platform fail-closed contracts unless a separately proven existing defect requires a regression-tested correction.
5. Renderer refactors must preserve V2 as the production architecture. Do not move V2 code into leftover V1 or introduce direct V2 dependencies on `src/components/**`, `src/hooks/**`, `src/lib/**` or `src/i18n/**`. Cross-generation reuse must go through renderer-neutral shared code or existing ports/facades.
6. Within V2 and leftover V1, split only when a cohesive responsibility, independent test seam, repeated domain abstraction or dependency boundary is demonstrable. Prefer feature/domain ownership and composition over technology-bucket growth.
7. Add or strengthen executable architecture rules for durable dependency boundaries introduced by this task. Do not rely on prose-only conventions when a low-false-positive compiler/test/lint rule is practical.
8. Use reviewable staged commits with focused validation after each risky architectural movement. Do not overwrite unrelated changes.
9. Before task archive, run Trellis spec-update review and update the owning frontend/backend code-specs with every durable module ownership/dependency rule learned or changed by this work. SPEC updates are mandatory when architecture contracts change.
10. Complete all local repository quality gates applicable to the final diff, archive the Trellis task, and only then push once. The final archived HEAD on `dev/laiyongjie` must be the pushed HEAD.
11. Monitor the GitHub `CI / Required` workflow for that exact final archived HEAD. If it fails, classify the failure and fix it (including adjacent unrelated CI blockers when necessary), then re-run the required Trellis finish/archive sequence as needed so the final delivered HEAD is green and archived.
12. Leave the checkout on `dev/laiyongjie` with a clean working tree and local/remote heads synchronized.

## Acceptance Criteria

- [ ] A documented current-state architecture audit covers Rust host, V2 renderer, leftover renderer and neutral shared boundaries, including measured hotspots and dependency evidence.
- [ ] The final design identifies which large modules are split, which are intentionally retained, and the evidence for each decision.
- [ ] High-value backend hotspots are decomposed into private responsibility-oriented modules behind stable facades where dependency/test evidence supports extraction.
- [ ] High-value frontend hotspots are decomposed or simplified without breaking V2/leftover/shared generation boundaries or creating speculative architecture layers.
- [ ] New/changed module boundaries reduce deep/cross-domain imports or mixed responsibilities and are protected by compiler visibility and/or executable architecture tests where appropriate.
- [ ] No intended Tauri wire contract, persisted data format, credential boundary, rollback order, proxy streaming/failover behavior or user-visible renderer behavior regresses.
- [ ] Existing affected integration/security/platform tests pass, plus new focused tests for extracted pure/domain behavior where a seam is introduced.
- [ ] `mise run check` passes on the final product-code state; V2-specific lint/typecheck/test gates pass when V2 is touched; other focused gates required by owning specs pass.
- [ ] Relevant `.trellis/spec/backend/**` and `.trellis/spec/frontend/**` files are updated before archive and validated.
- [ ] Changes are committed in reviewable stages; Trellis task is archived before remote delivery.
- [ ] Exactly the final archived branch state is pushed to `origin/dev/laiyongjie`; no deliberate pre-archive push is performed.
- [ ] GitHub CI for the final pushed SHA completes with `CI / Required = success`.
- [ ] Final working tree is clean and `dev/laiyongjie` matches `origin/dev/laiyongjie`.

## Out of Scope

- Product redesign, new end-user capabilities or deliberate wire/storage migrations unrelated to maintainability.
- A cosmetic repository-wide rename/move whose only benefit is directory uniformity.
- Cargo workspace/crate proliferation without a proven package boundary.
- Replacing V2 with leftover V1, or deleting leftover V1 solely for architectural aesthetics while it still has tested compatibility consumers.
- New state-management/framework dependencies unless repository evidence proves the existing tools cannot express the required boundary.

## Open Questions

No user-owned product/scope decision is currently unresolved. Technical extraction boundaries remain subject to repository audit and research during planning.
