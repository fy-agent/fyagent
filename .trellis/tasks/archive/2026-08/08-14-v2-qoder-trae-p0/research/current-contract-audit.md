# Current Contract Audit

## Baseline

- HEAD: `c4f7a9c48b86ed670a00f583e5f3bf76e49e7a60`
- Branch: `dev/laiyongjie`
- Initial worktree: clean
- Existing catalog contract: v2, ordered QoderWork CN, TRAE Work, WorkBuddy, Codex, Claude Code.

## UI Findings

- Agent rail currently uses `minmax(220px, .72fr) minmax(0, 1.7fr)`, 34px list icon and 58px row.
- Models rail currently uses `minmax(190px, .58fr) minmax(0, 1.82fr)`, 32px list icon, 52px row and an extra 900px ratio branch.
- The previous Agent catalog task already established exact official-link semantics, Codex V2 installer reuse, five-candidate order, keyboard selection, local assets and left-panel height independence; the new shared UI must preserve them.

## Backend Findings

- `src-tauri/src/commands/agent_catalog.rs` owns the deterministic static catalog.
- `src-tauri/src/app_config.rs` models Skill/MCP assignment through broad `AppType` booleans; adding vendor products directly would couple unrelated Provider/session domains.
- `src-tauri/src/services/skill.rs` already owns authoritative Skill storage, archive safety, target synchronization and reread behavior.
- `src-tauri/src/services/workbuddy/` contains reusable patterns for bounded network access, secret-safe errors, HMAC revisions, overwrite confirmation and Windows path identity.
- `src-tauri/src/proxy/http_client.rs` owns the application proxy selection contract; endpoint preflight should reuse its resolved mode while adding stronger SSRF/DNS-pin requirements.

## Environment

- Git and WebView2 prerequisite checks pass.
- Visual Studio 2022 Build Tools is installed under `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`.
- The ordinary PowerShell does not expose `cl.exe`; Rust/full gates must load `Common7\Tools\VsDevCmd.bat` first.

