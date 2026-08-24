# Implementation plan

1. Reuse `AgentCatalogId` and existing directory metadata; do not duplicate IDs or links.
2. Implement pure readiness contracts and exact serialization tests.
3. Implement one read-only command function and an unregistered strict V2 adapter seam.
4. Add the read-only Agent detail region and focused tests, preserving Codex installer lifecycle.
5. Report shared registration/port composition needs to integration.

## Progress log

- 2026-08-24: claimed by `gpt-5.6-sol-medium`; baseline and ownership frozen by root.
- 2026-08-24: read-only readiness was integrated with the canonical catalog; focused V2/Rust and full local gates passed.

## Deliverables

- [Readiness domain](../../../src-tauri/src/agent_install/mod.rs)
- [Agent detail readiness section](../../../src/v2/pages/agents/AgentInstallReadinessSection.tsx)

## Acceptance evidence

- Exact seven-ID, exact-key DTO, sensitive-field rejection and single-command ACL tests passed.
- Fresh V2 unit `314/314` and browser `116/116` passed; the existing Codex Desktop installer remained reachable.
