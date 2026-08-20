# 实施计划：QoderWork / TRAE Work P0

## 0. Planning and Activation

- [x] Confirm HEAD/status and protect non-task changes.
- [x] Curate repo-owned research and implement/check manifests.
- [x] Validate task artifacts and ensure no blocked questions remain.
- [x] Start the reviewed task with scope `v2-qoder-trae-p0-automated`.

## 1. Wave 0 — Baseline Contracts

- [x] Freeze catalog v2/current failure behavior, Agent/Models geometry, six existing Skill/MCP targets, Qoder/TRAE placeholder behavior and secret-negative assertions.
- [x] Run focused V2/browser/Rust tests before implementation; record any pre-existing failure rather than weakening tests.
- [x] Preserve the prior Agent catalog official-link, Codex installer, selection, focus and left-panel-height behavior.

## 2. Wave 1 — Parallel Foundations

### Worker A: Shared Catalog UI

Ownership: shared catalog primitives/style, Agent/Models adoption, typed Agent asset metadata, focused component/style/browser tests. Do not edit Rust, Skills services or network modules.

- [x] Add the shared component family and single token set.
- [x] Migrate both pages without changing their detail capability flows.
- [x] Remove duplicate geometry, brand-specific CSS and the model-only rail breakpoint.
- [x] Add geometry, scrollbar, responsive, keyboard and reduced-motion assertions.

### Worker B: Catalog v3 and Runtime Boundary

Ownership: Rust catalog/runtime types and focused tests; TypeScript catalog/runtime parser/fixtures may be integrated by the main session after Rust shape stabilizes. Do not edit Skills or vendor configuration modules.

- [x] Upgrade static catalog to exact v3 capabilities/evidence.
- [x] Add runtime status and closed launch DTOs/negative adapters.
- [x] Register narrow commands and fail-closed tests.

### Worker C: SkillTargetId and Target Adapters

Ownership: Skill target types, additive `SkillApps` compatibility, Qoder/TRAE destination adapters, target constants and focused Rust/frontend tests. Do not edit catalog UI or vendor Hooks/network modules.

- [x] Add eight-value SkillTargetId with explicit six-AppType adapter.
- [x] Preserve old serialized fields and defaults.
- [x] Reuse SkillService copy/conflict/hash/reread paths for fixed targets.
- [x] Split Skill and MCP target collections in renderer.

Wave 1 focused gates:

```powershell
mise run typecheck:v2
mise run lint:v2
mise run test:v2
mise run test:v2:browser
mise run rust:fmt:check
mise run rust:test agent_catalog
mise run rust:test skill
```

## 3. Wave 2 — Parallel Vendor Capabilities

### Worker A: QoderWork Hooks

Ownership: safe document/Qoder adapter, commands, DTOs and Rust tests. Do not edit TRAE network or V2 pages.

- [x] Implement bounded projection and HMAC revision.
- [x] Implement preservation merge, concurrency, token, backup, atomic replace and reread.
- [x] Cover invalid JSON/shape, unknown fields, links/reparse/TOCTOU and failure ordering.

### Worker B: TRAE Endpoint and MCP Validators

Ownership: endpoint URL/address/proxy/cancellation and MCP validation modules plus focused tests. Do not edit Qoder or UI pages.

- [x] Implement pure model validation and short-lived secret wrapper.
- [x] Implement DNS/address policy, pinned connection, proxy safety, limits and cancellation.
- [x] Implement MCP tagged-union validation and no-execute command resolution.
- [x] Add local resolver/server fixtures without contacting real vendor services.

### Worker C: Runtime and Permission Integration

Ownership: external-agent module registry, command registration, narrow permissions and exact fake/native negative tests. Do not introduce positive executable candidates without evidence.

- [x] Wire observe/launch/write/probe commands.
- [x] Ensure remote/webview and generic fs/shell paths remain unavailable.
- [x] Add secret-free errors and controlled unverified behavior.

## 4. Wave 3 — Renderer Integration

One integration owner handles feature ports, strict Tauri/browser adapters, Qoder/Trae panels, Models preflight, fake-Tauri fixtures, i18n and focused tests after Waves 1–2 stabilize.

- [x] Parse exact v3/runtime DTOs and reject unknown wire.
- [x] Add Qoder status/Skills/Hooks/MCP flows and restart-required copy.
- [x] Replace Qoder/Trae placeholder notes; add TRAE stepwise model preflight and cancellation.
- [x] Add redacted MCP template preview/copy and vendor-UI completion guidance.
- [x] Prove target/unmount/terminal secret clearing and query/storage/DOM negatives.
- [x] Preserve browser preview as non-authoritative.

## 5. Wave 4 — Full-Scope Check and Fix Loop

- [x] Dispatch a Trellis check agent over the entire task diff and artifacts.
- [x] Verify spec/PRD/design compliance, code reuse, cross-layer DTO equality, i18n, permissions and secret boundary.
- [x] Fix every verified finding directly and rerun focused tests until green.
- [x] Run final full-scope gates in a Visual Studio Build Tools Developer environment:

```powershell
mise run check:prearchive --exclude-active-task .trellis/tasks/08-14-v2-qoder-trae-p0
mise run test:v2:browser
mise run build:renderer
git diff --check
```

- [x] Run a repository scan for secret sentinels and forbidden local-reference names/paths.
- [x] Confirm no HIL was run and all HIL-dependent capabilities remain unverified.

## 6. Spec, Commit and Archive

- [x] Run `trellis-update-spec`; update catalog/Skills/vendor/security contracts where knowledge is durable.
- [x] Commit task-owned changes in coherent batches: tests/UI; catalog/runtime/Skills; Qoder; TRAE/network/MCP; spec/security closure.
- [x] Exclude all unrecognized dirty files; do not amend, push, create a PR or publish.
- [x] Run the Trellis finish workflow, archive the task, record the session journal and verify a clean worktree/current-task state.

## Rollback Points

- Shared UI batch reverts independently and restores existing per-page styles.
- Catalog v3 must revert Rust/TypeScript fixtures together.
- Skill fields are additive; reverting code leaves unknown fields ignorable by old readers.
- Qoder failure uses backup/unknown status; never delete user settings as rollback.
- TRAE validator rollback removes only FyAgent code because no vendor storage is written.

