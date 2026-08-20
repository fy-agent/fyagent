# Implement

1. Move Skill/MCP view tabs into `fy-feature-header`; persist Skill actions on Discover.
2. Add `.fy-feature-header > .fy-feature-tabs { margin-bottom: 0; }` in `features.css`.
3. Narrow Skills page.css tab width rule.
4. Add `intros.ts` and render on Agent detail (skip empty Codex).
5. Update `v2-skills-mcp.md` and `v2-agent-models.md`.
6. Tests: `tests/v2/features/featurePages.test.tsx`, `tests/v2/pages/agents/Page.test.tsx`, skills page styles if needed.

```bash
mise run typecheck
pnpm exec vitest run tests/v2/features/featurePages.test.tsx tests/v2/pages/agents/Page.test.tsx tests/v2/pages/skills/page.styles.test.ts
```
