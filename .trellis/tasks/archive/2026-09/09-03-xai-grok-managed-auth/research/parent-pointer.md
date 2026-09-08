# Pointer: parent research (do not duplicate)

- **Query**: locate frozen parent evidence for this child without editing parent files
- **Scope**: internal
- **Date**: 2026-09-03

## Findings

This child (`09-03-xai-grok-managed-auth`) inherits product/protocol/license baseline from:

| Parent file | Role |
|---|---|
| `.trellis/tasks/09-03-unified-agent-auth-management/design.md` | §10.2 Grok helper vs native fallback |
| `.trellis/tasks/09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md` | 2026-09-03 upstream commit/license matrix |
| `.trellis/tasks/09-03-unified-agent-auth-management/research/current-fyagent-integration-seams.md` | 2026-09-03 seam inventory |
| `.trellis/spec/backend/managed-auth.md` | `refresh_owner=grok_native` fail-closed resolver |
| `.trellis/spec/backend/external-agent-auth.md` | Grok `handoff_only` contract |
| `.trellis/spec/backend/external-agent-lifecycle.md` | Grok npm/Mainland install owner (Auth must not own) |

Refreshed child evidence for this slice lives in sibling files under this directory. Parent files were not modified.

## Caveats / Not Found

Parent grok-build pin was `72a61251fcffb464bcc687aeb5a998e5a98ec0c9` (2026-09-01). This refresh found the same SHA still at `xai-org/grok-build` `main`.
