# Skills / MCP catalog sync evidence (2026-08-18)

## Current V2 targets

- Skills (8): `claude`, `codex`, `gemini`, `grokbuild`, `opencode`, `hermes`, `qoderwork`, `trae-work`
- MCP (6): `claude`, `codex`, `gemini`, `grokbuild`, `opencode`, `hermes`
- Agent catalog (6): `qoderwork`, `trae-work`, `workbuddy`, `codex`, `claude-code`, `opencode`

Gemini / Grok Build / Hermes are leftover AppType targets. They are not in the V2 Agent catalog.

## WorkBuddy local contracts (this machine)

- Skills directory: `~/.workbuddy/skills` (exists; copy-only like Qoder/TRAE is valid)
- MCP documents: `~/.workbuddy/.mcp.json` (newer) and `~/.workbuddy/mcp.json` (older). Both use `mcpServers`. Canonical write target for this task: `~/.workbuddy/.mcp.json`. Import may read either file when equivalent.
- WorkBuddy is **not** an `AppType`. Do not add it to Provider/session domains.

## Decision

V2 assignment collections follow the Agent catalog, not leftover AppType:

| Page | V2 visible targets | Backend |
|---|---|---|
| Skills | claude, codex, opencode, qoderwork, trae-work, workbuddy | Add `SkillTargetId::WorkBuddy` + `enabled_workbuddy` (schema 18, default false). Copy-only to `~/.workbuddy/skills`. Keep gemini/grokbuild/hermes flags for leftover; V2 UI does not show them. |
| MCP | claude, codex, opencode, workbuddy | Add `McpApps.workbuddy` without creating `AppType::WorkBuddy`. Adapter writes `~/.workbuddy/.mcp.json` in Claude-like `mcpServers` form. QoderWork / TRAE Work stay out of direct MCP assignment (validate/prepare only, jump from Agents). Gemini/Grok/Hermes remain in leftover MCP JSON but are hidden on the V2 page. |

QoderWork / TRAE Work MCP live vendor files are still undocumented. Do not invent a connectors write. Agent directory can jump to the existing validate/prepare UI.

Skill copy path for WorkBuddy is trusted-home `.workbuddy/skills` only (plus `FYAGENT_TEST_HOME` in tests).
