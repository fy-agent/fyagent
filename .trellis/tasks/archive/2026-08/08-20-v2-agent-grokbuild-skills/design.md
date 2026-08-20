# Design

## Skills disk recognition

`SkillService::get_all_installed` keeps SQLite as the management registry, then unions `scan_unmanaged` results that pass `require_valid_directory`. Observed rows use the same `local:{directory}` / lock-derived IDs as `import_from_apps`, set `apps` from `SkillApps::from_labels(found_in)`, and expose the live path. GET stays read-only.

`toggle_target` / `uninstall` adopt missing IDs through `import_from_apps` before the existing SSOT mutation. This avoids deleting `~/.codex/skills/*` just because the user opened the Installed tab.

Dot directories stay skipped (Codex `.system`).

## Shared product directory

`src/v2/shared/features/directory.ts` is the single ordered catalog:

`qoderwork`, `trae-work`, `workbuddy`, `grokbuild`, `codex`, `claude-code`, `opencode`

Each row maps `agentId`, assignment id (`claude` vs `claude-code`), display name, prompt app (nullable), and models target. Skills/MCP/Models/Agent read this list. Prompts render catalog members with `promptAppId`, then Gemini / OpenClaw / Hermes.

Catalog contract version becomes 4.

## Agent detail

Shared `CatalogOfficialLinks` always uses `fy-control-button-primary`. Capability grid filters `direct` only. Observation, hooks, and MCP panels are removed from the Agent route; Qoder Models jumps to `/mcp`.

## Grok Build models

Extend Provider quick setup allowlist to `grokbuild` with reserved id `fyagent-v2-quick-setup-grokbuild` and a `settings_config.config` TOML document (`[models]` + `[model.<id>]`). Live snapshot is `~/.grok/config.toml`. Reuse `ProviderPanel`.

## Discovery scroll

On the Skills discovery tab, the feature page itself scrolls (`overflow: auto`). Inner `.fy-feature-discovery-scroll` becomes in-flow content (`overflow: visible; flex: none`). MCP discovery keeps the existing inner scroller.
