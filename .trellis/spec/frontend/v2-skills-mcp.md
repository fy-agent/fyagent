# V2 Skills and MCP Compatibility Router

This path preserves archived references to the former combined document. It is
a navigation surface only. New tasks must load the focused contract that owns
their change.

## Read by concern

| Concern | Authoritative contract |
| --- | --- |
| Skills installed/discovery views, SkillHub/search sources, pagination, install target and assignment UI | [V2 Skills](./v2-skills.md) |
| MCP installed/discovery views, launch parsing, security projection, presets, install and assignments | [V2 MCP](./v2-mcp.md) |
| Shared routing, persistent hidden-page behavior and Agent return descriptor | [V2 Navigation and Persistent Route](./v2-navigation.md) |
| Shared selection material, collapse motion and external-link interaction | [V2 Window Shell and Interaction](./v2-window-shell.md) |
| Shared seven-target order, AssignmentPanel API, serialization and authoritative reread | [V2 Shared Assignment](./v2-assignments.md) |
| Native Skill persistence, discovery, archive safety and target synchronization | [Skill Management](../backend/skill-management.md) |
| Native MCP persistence, validation, vendor live files and import | [MCP Management](../backend/mcp-management.md) |
| Typed multi-owner mutation and compensation | [Change Plan Typed Executor](../backend/change-plan-executor.md) |
| Secret references and native redaction | [SecretRef Native Backend](../backend/secretref-backend.md) |

## Shared invariants

- Skills and MCP use their domain Ports and shared assignment owners; neither
  page maintains a second target matrix, native-path map, secret store, or
  persistence implementation.
- Discovery/catalog metadata is untrusted display input until the owning native
  installer/validator admits it. A homepage or repository URL is not an
  executable install capability.
- Commands, environment fields and headers are parsed and redacted before
  display. Secrets never round-trip as page state, logs, query parameters, or
  change-plan prose.
- Assignments are read back from the authoritative target after mutation.
  Optimistic UI may show progress but cannot invent persisted success.
- Keep this router short; detailed validation, cases and tests belong in the
  two focused feature contracts and their native owner.
