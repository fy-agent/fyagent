# Renderer Skills and MCP Compatibility Router

This path preserves archived references to the former combined document. It is
a navigation surface only. New tasks must load the focused contract that owns
their change.

## Read by concern

| Concern                                                                                                 | Authoritative contract                                           |
| ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Skills installed/discovery views, SkillHub/search sources, pagination, install target and assignment UI | [Renderer Skills](./skills.md)                                   |
| MCP installed/discovery views, launch parsing, security projection, presets, install and assignments    | [Renderer MCP](./mcp.md)                                         |
| Shared routing, persistent hidden-page behavior and Agent return descriptor                             | [Renderer Navigation and Persistent Route](./navigation.md)      |
| Shared selection material, collapse motion and external-link interaction                                | [Renderer Window Shell and Interaction](./window-shell.md)       |
| Shared seven-target order, AssignmentPanel API, serialization and authoritative reread                  | [Renderer Shared Assignment](./assignments.md)                   |
| Native Skill persistence, discovery, archive safety and target synchronization                          | [Skill Management](../backend/skill-management.md)               |
| Native MCP persistence, validation, vendor live files and import                                        | [MCP Management](../backend/mcp-management.md)                   |
| Typed multi-owner mutation and compensation                                                             | [Change Plan Typed Executor](../backend/change-plan-executor.md) |
| Secret references and native redaction                                                                  | [SecretRef Native Backend](../backend/secretref-backend.md)      |

## Shared invariants

- Skills and MCP use their domain Ports and shared assignment owners; neither
  page maintains a second target matrix, native-path map, secret store, or
  persistence implementation.
- Discovery/catalog metadata is untrusted display input until the owning native
  installer/validator admits it. A homepage or repository URL is not an
  executable install capability.
- Ordinary MCP detail/search redact recognized sensitive arguments and URLs
  and exclude env/header values. The existing native DTO, installed Query and
  explicit editor still carry raw env/header fields; see [MCP](./mcp.md#current-sensitive-value-boundary).
  That narrow editing boundary is not permission to log, export, animate-copy
  or put secrets in route state or Change Plan prose.
- Assignments are read back from the authoritative target after mutation.
  Optimistic UI may show progress but cannot invent persisted success.
- Keep this router short; detailed validation, cases and tests belong in the
  two focused feature contracts and their native owner.
