# V2 Models 配置安全与连通性修复 — Implementation Plan

## Stage 0 — Baseline and focused reproduction

- [x] Load Trellis before-dev context for backend + V2 frontend.
- [x] Run focused existing V2 Models, model-probe, Provider Quick Setup, WorkBuddy/OpenCode storage and architecture tests as baseline.
- [x] Reproduce `SecretInput` geometry in the real Playwright browser fixture. Record wrapper/input/toggle bounding boxes before fill, after a long key, and after reveal/hide. Do not change CSS until the failing geometry is measured.
- [x] Record the planning-time protocol conclusion: the current Codex probe contains one confirmed incompatible output-limit field. Do not retain endpoint/model/credential details in task artifacts.
- [x] Use local wire-level fixtures as the implementation proof. Do not perform another real external API probe unless the user explicitly requests it.

## Stage 1 — Targeted Provider Quick Setup live patches

- [x] Add syntax/data-preserving Quick Setup patch functions for Claude, Codex, and Grok Build using current live preimages.
- [x] Codex: patch only the owned top-level/provider fields, preserve an existing `disable_response_storage`, preserve unrelated `http_headers`, comments/order and every unrelated table.
- [x] Grok Build: patch `[models].default` and the selected model's owned fields only; preserve other model blocks and config sections.
- [x] Claude: deep-merge only the three Quick Setup env fields into existing settings; preserve all other settings/env entries.
- [x] Ensure later writes/switches of the fixed V2 Quick Setup reserved Providers reuse the same patch projection instead of reverting to a minimal full snapshot.
- [x] Add fixtures based on large realistic configs with unrelated keys/comments/MCP/features and assert the permitted diff only.

## Stage 2 — One rolling backup + authoritative disclosure

- [x] Introduce deterministic Provider Quick Setup backup paths adjacent to each physical target; no timestamp generations.
- [x] Back up every existing physical file that will actually be modified before the first primary mutation; backup failure must abort the transaction.
- [x] Preserve restrictive credential-file permissions/user-scope authority, including Windows storage rules.
- [x] Expose secret-free authoritative write-plan metadata to V2 for Provider Quick Setup, WorkBuddy and OpenCode without letting React construct filesystem paths.
- [x] Reuse WorkBuddy/OpenCode's existing single backup files rather than creating duplicates.
- [x] Add a shared Models disclosure block that visibly states target path, backup path and single-backup replacement behavior. Disable save when required write-plan metadata cannot be read.
- [x] Cover first-create semantics explicitly: no nonexistent preimage is fabricated, and UI says backup begins once a preimage exists.

## Stage 3 — Protocol-correct connectivity probes

- [x] Remove the confirmed incompatible output-limit field from the Codex Responses probe. Keep the remaining request shape bounded and aligned with current native Codex semantics without attributing the failure to unrelated input-shape differences.
- [x] Pass bounded Codex image-extension intent so the probe reproduces the generated actor header when needed; never accept arbitrary renderer headers.
- [x] Change Grok Build probe from Chat Completions to Responses to match `DEFAULT_API_BACKEND` / Quick Setup.
- [x] Keep Claude Messages and WorkBuddy/OpenCode Chat Completions.
- [x] Add wire-level mock tests asserting exact endpoint, key headers, body fields, bounded errors and secret-negative results for all five probe targets.
- [x] Do not add live external endpoint checks to the automated or Trellis acceptance path; exact local protocol tests are authoritative for this fix.

## Stage 4 — Renderer state and shared SecretInput

- [x] Add one Models-shared draft commit-revision mechanism and use it in ProviderPanel, WorkBuddy and OpenCode.
- [x] Mark successful submitted revisions clean; keep edits made during an in-flight save dirty; failed/rolled-back writes remain dirty.
- [x] Add a reset version to shared `ModelConnectivityTest`; invalidate old results on connection draft changes and successful save.
- [x] Fix the measured reveal-button geometry at `SecretInput` / shared controls CSS. Do not add Codex-only positioning.
- [x] Add component/browser tests for long keys, password/text toggling, no overflow and stable toggle bounding box.
- [x] Add browser tests proving successful save clears `待保存` and stale connectivity feedback for ProviderPanel, WorkBuddy and OpenCode; a subsequent edit restores `待保存`.

## Stage 5 — Adjacent target audit

- [x] Re-audit every writable Models target: Claude, Codex, Grok Build, WorkBuddy, OpenCode. Qoder/TRAE remain non-writing guidance/observation targets.
- [x] Confirm WorkBuddy preserves unknown JSON fields and uses its existing immediate-previous fixed backup.
- [x] Confirm OpenCode preserves unrelated provider/root JSON fields and uses exactly one fixed backup; make only narrowly required safety adjustments if a failing test proves a gap.
- [x] Confirm no new V2 import from leftover UI/business trees and no duplicate invoke wrapper/component was introduced.

## Stage 6 — Quality and regression gates

- [x] `mise run format:check`
- [x] `mise run lint:v2`
- [x] `mise run typecheck:v2`
- [x] focused V2 Models unit tests
- [x] focused V2 Models Playwright browser tests
- [x] focused Rust model-probe / Provider / Codex / Grok / WorkBuddy / OpenCode tests
- [x] `mise run rust:fmt:check`
- [x] `mise run rust:check`
- [x] `mise run rust:clippy`
- [x] `mise run rust:test`
- [x] architecture / secret-negative / supported-platform contract tests
- [x] final `mise run check`

## Stage 7 — Trellis finish

- [x] Update `.trellis/spec/frontend/v2-agent-models.md` with the executable write-plan, dirty-state, probe-reset and protocol contracts.
- [x] Update `.trellis/spec/backend/codex-provider-configuration.md` and the owning Grok/Provider boundary spec as required by the implementation, including the 7-section cross-layer/infra contract format.
- [x] Update any WorkBuddy/OpenCode spec only if behavior actually changed beyond disclosure/state wiring.
- [x] Run spec-owned focused checks.
- [x] Commit implementation/spec in scoped local commits and pass the Trellis prearchive gate. Archive and journal are completed by the final Trellis wrap-up; remote push / exact-SHA CI are not part of this user-requested local archive flow.

## Rollback points

- Stage 1 can be reverted independently if targeted patch fixtures reveal an ownership ambiguity; do not fall back to full-file reconstruction.
- Stage 3 is independent of persistence and can be reverted without changing saved configs.
- Stage 4 frontend state/layout changes are independent of backend file semantics.
- Any backup/path change that cannot preserve the platform user-scope security boundary blocks the task rather than shipping a reduced-safety fallback.
