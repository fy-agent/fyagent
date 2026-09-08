# Comprehensive Spec Audit

## Method

- Read all layer indexes and extracted every heading/line count from the 43 existing Markdown files.
- Verified relative Markdown links and index coverage programmatically.
- Checked explicit repository path references and representative command/task names against the checkout.
- Mapped major source owners under `src/v2`, `src-tauri/src/{agent_install,commands,services,database,mcp,proxy}` and representative tests.
- Used the current source and tests—not document age or line count—as the authority for disposition.

## Global findings

- Links and index coverage are healthy; this is an information-architecture and fact-alignment refresh, not a broken-link cleanup.
- Three files mix independently queried domains and should be split: `external-agent-p0.md`, `v2-agent-models.md`, `v2-skills-mcp.md`.
- Long CI/Release/native security/installer/task-runner documents remain cohesive, order-sensitive contracts; splitting them solely by size would weaken authority.
- Missing high-value owners: SQLite lifecycle/migrations, proxy runtime, and localization.
- Concrete drift exists in Rust modular-boundary module names and Agent command ownership.

## Existing backend disposition matrix

| Existing file | Disposition | Evidence and rationale |
| --- | --- | --- |
| `backend/index.md` | Update | Route directly to focused new owners; keep as thin navigation. |
| `application-brand-assets.md` | Retain | Focused canonical asset/packaging contract with byte-level tests. |
| `application-identity.md` | Retain | Focused identity/provenance boundary; no independent split owner. |
| `change-plan-executor.md` | Retain | One typed execution/compensation ledger with schema-v20 sequencing. |
| `codex-desktop-installer.md` | Retain + boundary link | One installer transaction; Agent lifecycle consumes reviewed primitives but owns product policy elsewhere. |
| `codex-provider-configuration.md` | Retain | Config/auth/catalog writes share one ordered live mutation and rollback owner. |
| `codex-session-usage.md` | Retain | Small focused JSONL import/retry/fingerprint contract. |
| `deeplink-import-security.md` | Retain | Focused untrusted-input and confirmation boundary. |
| `development-environment.md` | Retain | Toolchain/bootstrap/host evidence remain one environment authority. |
| `development-hooks.md` | Retain | Short optional Codex/Trellis hook ownership note. |
| `external-agent-p0.md` | Split + compatibility router | Mixes at least six independent source and test owners. |
| `fyagent-version-contract.md` | Retain | One canonical version/asset identity authority. |
| `github-ci-workflow.md` | Retain | Required aggregation, identity and toolchain semantics form one CI graph. |
| `github-merge-governance.md` | Retain | One merge-readiness/queue/task lifecycle authority. |
| `github-release-workflow.md` | Retain | Signing, notarization, assets, draft recovery and publication are one transaction. |
| `macos-dmg-layout.md` | Retain | Focused DMG layout/retry/byte-preservation contract. |
| `macos-system-commit.md` | Retain | Privileged helper ABI and enablement gate are one native boundary. |
| `main-window-layout.md` | Retain | Focused native geometry/chrome contract. |
| `modular-boundaries.md` | Correct | Update `auth_actions`/`auth_sessions`/`sources`, Agent command owners and nuanced Proxy visibility. |
| `reuse.md` | Retain | Cross-backend implementation-order contract; no feature behavior duplication. |
| `secretref-backend.md` | Retain | Focused secret material/reference/native evidence boundary. |
| `task-runner-contract.md` | Retain | Public `mise run` API and host guards are one repository interface despite size. |
| `upstream-sync.md` | Retain | One immutable-source/ancestry/conflict/provenance transaction. |
| `windows-installer.md` | Retain | One NSIS/signing/uninstall/cleanup contract. |
| `windows-runtime-security.md` | Retain | Explorer-user, registry, helper and process-launch constraints are mutually dependent. |
| `workbuddy-configuration.md` | Retain | One revisioned config/model write owner with native identity and readback. |

## Existing frontend disposition matrix

| Existing file | Disposition | Evidence and rationale |
| --- | --- | --- |
| `frontend/index.md` | Update | Route directly to new feature owners and localization. |
| `component-guidelines.md` | Retain | Focused shared component/accessibility guidance. |
| `directory-structure.md` | Retain | Current V2/leftover placement and test layout remain valid. |
| `hook-guidelines.md` | Retain | Short hook/effect/native-event ownership guidance. |
| `modular-boundaries.md` | Retain + link refresh | Renderer/host and V2/leftover boundaries remain valid; link focused owners. |
| `quality-guidelines.md` | Retain + deduplicate locale detail | Keep test/evidence rules; delegate locale schema to `localization.md`. |
| `reuse.md` | Retain | Shared owner/dependency/business-authority separation remains focused. |
| `state-management.md` | Retain | Server/URL/draft/secret/derived ownership remains one foundation contract. |
| `type-safety.md` | Retain | Dynamic boundary parsing/exhaustive union rules remain focused. |
| `user-facing-copy.md` | Retain + localization link | Own tone/evidence; delegate key parity/language mechanics. |
| `v2-agent-models.md` | Split + compatibility router | Source has separate agents and models pages, Auth hook/feature owner and distinct tests. |
| `v2-prompts-memory.md` | Retain | Compact shared native CRUD/delegation contract; current size does not justify fragmentation. |
| `v2-shell.md` | Retain | Route persistence, chrome, SelectionLens and motion coordinate one shell lifecycle. |
| `v2-skills-mcp.md` | Split + compatibility router | Skills, MCP and shared assignment have independent pages, ports and test suites. |

## Existing guide disposition matrix

| Existing file | Disposition | Evidence and rationale |
| --- | --- | --- |
| `guides/index.md` | Retain | Already a short router that distinguishes guides from code-specs. |
| `code-reuse-thinking-guide.md` | Retain | Short pre-design checklist; detailed rules live in layer specs. |
| `cross-layer-thinking-guide.md` | Retain | Short round-trip/owner checklist; does not duplicate DTOs. |

## New owner set

### Backend

- `external-agent-catalog-runtime.md`
- `external-agent-lifecycle.md`
- `external-agent-auth.md`
- `external-agent-configuration.md`
- `skill-management.md`
- `mcp-management.md`
- `persistence-and-migrations.md`
- `proxy-runtime.md`

### Frontend

- `v2-agent-directory.md`
- `v2-agent-auth.md`
- `v2-models.md`
- `v2-assignments.md`
- `v2-skills.md`
- `v2-mcp.md`
- `localization.md`

## Evidence anchors to preserve during rewriting

- Strict parser versions, exact catalog/target order, forbidden wire fields and opaque capability grammars.
- Fail-closed ordering before network/filesystem/process side effects.
- Transaction/rollback/readback semantics for installer, config, assignment, database and proxy live mutation.
- Secret lifetime/redaction requirements and HIL limits.
- Exact focused test names and assertion points.
- Historical paths remain valid only as compatibility routers; they must not become parallel behavior owners.

## Review rounds

### Round 1 — Structural and retrieval audit

- Audited all 43 pre-refresh Markdown files, layer indexes, relative links, headings and line distribution.
- Confirmed that the old tree had healthy links and index coverage; the material defect was cross-domain retrieval, not broken Markdown.
- Identified exactly three cross-owner monoliths and rejected a mechanical “split every long file” strategy.

### Round 2 — Source, architecture and security audit

- Mapped every proposed owner to current Rust/TypeScript modules and representative tests.
- Corrected retired Agent module/transport ownership in the Rust modular-boundary contract.
- Preserved opaque capabilities, trusted-path/native-identity admission, secret redaction, fail-closed ordering, compensation and authoritative reread semantics during compression.
- Kept CI, Release, native security, installer, task-runner and other single high-risk transactions intact where splitting would create parallel authority.

### Round 3 — Information-architecture and compatibility audit

- Added 15 focused owners: eight backend and seven frontend, including persistence, proxy and localization gaps.
- Replaced the three former monoliths with short compatibility routers so archived tasks and classifier fixtures still resolve.
- Rewrote backend/frontend indexes to inject focused owners directly; compatibility paths are explicitly excluded from the primary reading order.
- Checked all new relative links, index coverage, seven-section contract order, source/test anchors and router size.

### Round 4 — Repository contract verification

- Checked exact command/version/owner symbols used by the new contracts against the current checkout.
- Checked every documented `mise run` task against the repository task registry.
- Ran focused architecture, locale-parity and change-classification tests, V2 type/tests, the Rust test suite and Trellis contract gates before archive.
- Re-ran whitespace, modified-path scope, duplicate-authority and compatibility-routing audits after corrections.
