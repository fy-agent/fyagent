# Implement

1. `SkillService::get_all_installed` unions SQLite with `scan_unmanaged`. GET stays read-only. `toggle_target` / `uninstall` adopt via `import_from_apps` first. Skip dot directories. Tests: Codex disk skill listed with `apps.codex`; `.system` skipped; GET does not write DB.
2. Shared `src/v2/shared/features/directory.ts` as the ordered product catalog. Skills/MCP/Models/Agent/Prompts read it. Prompt-only Gemini / OpenClaw / Hermes follow catalog members.
3. Catalog contract v4 + Grok Build. Official URL `https://x.ai/grok`. Skills/MCP/models.write direct; hooks unsupported; detect/launch unverified.
4. Agent detail: shared primary official links; only `direct` capabilities; remove observation, unsupported, counts, usage, Hooks/MCP panels. Qoder Models jumps to `/mcp`.
5. Provider quick setup allowlist adds `grokbuild` (`fyagent-v2-quick-setup-grokbuild`, live `~/.grok/config.toml`).
6. Skills discovery: whole `.fy-skills-page` scrolls; inner discovery scroller is in-flow. MCP discovery unchanged.
7. Update V2 / Rust tests and the three V2 feature specs.
