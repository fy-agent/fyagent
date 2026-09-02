# Implementation Plan

## Phase A — Audit and planning

- [x] Load Trellis workflow, update-spec, spec-bootstrap, meta and before-dev guidance.
- [x] Inventory all existing Spec files, line counts, indexes and Markdown links.
- [x] Map repository architecture, source owners and representative tests.
- [x] Record the disposition of all 43 pre-existing Spec files in `research.md`.
- [x] Converge PRD and target information architecture.

## Phase B — Backend Spec refresh

- [x] Add `persistence-and-migrations.md` with exact SQLite lifecycle and migration contracts.
- [x] Add `proxy-runtime.md` with exact listener/service/route/auth/failover/restore contracts.
- [x] Split `external-agent-p0.md` into catalog/runtime, lifecycle, Auth, configuration, Skills and MCP owners.
- [x] Replace `external-agent-p0.md` with a short compatibility router.
- [x] Correct `modular-boundaries.md` module names, transport owners and Proxy visibility wording.
- [x] Update `backend/index.md` and affected cross-links.

## Phase C — Frontend Spec refresh

- [x] Split `v2-agent-models.md` into directory, Auth and Models owners.
- [x] Split `v2-skills-mcp.md` into assignment, Skills and MCP owners.
- [x] Replace both old files with short compatibility routers.
- [x] Add `localization.md` and keep locale mechanics out of unrelated foundation contracts.
- [x] Update `frontend/index.md` and affected cross-links.

## Phase D — Verification and review

- [x] Run the structural Spec audit: links, index coverage, placeholders, empty headings, path references and duplicate authority checks.
- [x] Run focused contract tests for Rust modular boundaries, locale parity and change classification.
- [x] Run `mise run check:contracts`.
- [x] Run `git diff --check` and review the complete diff from user, architecture, security, testing and retrieval perspectives.
- [x] Correct any defect and repeat the affected checks.

## Phase E — Finish

- [x] Run `mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/09-02-comprehensive-spec-refresh`.
- [x] Commit the Spec/task changes with a focused public-repository message.
- [x] Archive the Trellis task, update the developer journal and commit lifecycle artifacts.
- [x] Confirm final status and report checks, commits and residual HIL limits.

## Risky files and rollback points

- `external-agent-p0.md`: highest risk of dropping native/security/vendor constraints. Do not replace until all six new owners cover its stable contracts.
- `v2-agent-models.md`: preserve strict catalog/readiness/model DTO parsing and Change Plan confirmation semantics.
- `v2-skills-mcp.md`: preserve secret lifetime, authoritative reread, assignment rollback and discovery-only boundaries.
- `backend/index.md` / `frontend/index.md`: broken routing can silently remove guidance from future sessions; verify exact file coverage.

At every risky file, compare the replacement owner set against the old headings, validation matrix and required tests before deleting detailed content.
