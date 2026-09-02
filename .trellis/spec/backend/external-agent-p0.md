# External Agent P0 — Compatibility Router

> Compatibility path only. Do not add new behavior to this file.

The former P0 document combined several independent backend owners and became
too large for reliable task-scoped retrieval. Its stable contracts now live in
the following focused specifications:

| Change area | Read this Spec |
| --- | --- |
| Static product catalog, runtime observation, trusted launch and ACL | [External Agent Catalog and Runtime](./external-agent-catalog-runtime.md) |
| Readiness, inventory, opaque targets, install/update/launch jobs and source resolution | [External Agent Lifecycle](./external-agent-lifecycle.md) |
| Login/logout/provider sessions and Auth observation | [External Agent Auth](./external-agent-auth.md) |
| Qoder Hooks configuration | [QoderWork Hooks Configuration](./qoderwork-hooks.md) |
| TRAE model preflight/observation and OpenCode models | [External Agent Model Integration](./external-agent-models.md) |
| Skill persistence, archive safety and target synchronization | [Skill Management](./skill-management.md) |
| MCP persistence, validation and vendor live files | [MCP Management](./mcp-management.md) |

Related native/security owners remain separate:

- [Codex Desktop Installer](./codex-desktop-installer.md)
- [Windows Shell-user Runtime](./windows-runtime-security.md)
- [macOS Privileged System-Commit Helper](./macos-system-commit.md)
- [SecretRef Native Backend](./secretref-backend.md)

This path is retained so archived Trellis tasks, review notes, and change
classification fixtures continue to resolve. New task context and new links
must select the focused owner directly.
