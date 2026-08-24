# Design

## Owned paths

- new `src-tauri/src/agent_install/**`
- one command implementation file dedicated to readiness, but no shared command module registration
- new `src/v2/shared/features/agent-install-readiness.ts`
- new strict adapter implementation under `src/v2/shared/platform/tauri/feature-ports/**`, but no composition-root edit
- existing `/agents` detail-owned component/styles and focused tests

Do not change canonical Agent catalog data/order, `src-tauri/src/lib.rs`, shared ACL manifests, `src/v2/shared/platform/ports.ts`, Tauri/browser composition roots or Git state.

## DTO

`AgentInstallReadinessDto` includes contractVersion 1, canonical agentId, reviewedAt, automation state/reason, source state/install mode/license scope/distribution state/checkedAt, integrity state/summaryCode/checkedAt, preflight state/sanitized checks/checkedAt, and plan state with null snapshot fields plus `plan_not_created`.

## Truth policy

- No executor means no ready automation and no snapshot.
- Unrefreshed source/license claims remain `unknown`/`unconfirmed`.
- Local hash alone does not establish integrity or eligibility.
- A fail dominates a layer; all-unknown remains unknown.
- Generic products use official-guide/executor-not-implemented reasons; Codex redirects conceptually to the existing managed installer, without adding another action.

## UI

Render status/reason as read-only explanatory copy, fail closed on query errors, reuse existing official links, and never introduce product-looking green state from unavailable/unknown data.
