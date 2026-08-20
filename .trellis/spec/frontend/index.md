# Frontend Development Guidelines

These guidelines describe the renderer patterns observed in this checkout of
FyAgent. They are evidence-based reference material for changes under
`src/` and related renderer tests, not a proposed frontend redesign.

The production desktop and browser entry is `src/index.html`, which loads
`src/v2/main.tsx`. Leftover V1 bootstrap (`src/main.tsx`, `src/App.tsx`,
`src/components/**`, `src/hooks/**`, `src/lib/**`, `src/i18n/**`) remains in
the tree and is still covered by the non-V2 Vitest suite; the guidelines
below stay authoritative for that leftover renderer. They are not the
production shell contract.

Reuse is the default frontend preference for both V2 and leftover work. Search
shared owners first. If a new component, helper, hook, or CSS recipe will be
used by another current or later module, put it in the shared layer on the
first commit. Do not wait for a third copy. The executable contract is
[Frontend Reuse](./reuse.md).

## Pre-Development Checklist

Before changing renderer code:

1. For any `src/v2/**` change, read the
   [V2 Shell Contract](./v2-shell.md) first. It is the only exception
   to the leftover route-placement, styling, primitive, and translation rules;
   the guidelines below remain authoritative outside V2.
2. Read the nearest relevant guideline and inspect the existing feature,
   primitive, and executable tests.
3. Locate the existing Tauri API facade, query hook, type, schema, shared UI,
   and test family before creating another one. Read the
   [Frontend Reuse Contract](./reuse.md). Reuse is the default: prefer
   existing shared chrome; if a new component will be used by another module,
   add it under `shared/` on the first commit. Do not wait until a third
   page copies it.
4. Classify state as local UI state, Context state, or backend/resource state.
5. For leftover renderer text, locate the matching keys in all four registered
   locale files before adding a literal string. V2 pages use hardcoded Chinese
   copy and must not import `src/i18n/**`.
6. For a backend payload change, inspect both the TypeScript facade and the
   matching `src-tauri/` serialization/command code.
7. For an application-brand icon change, read the shared
   [Application Brand Asset Contract](../backend/application-brand-assets.md)
   before regenerating Tauri or About assets.
8. For product names, storage keys, serialized markers, deep links, or public
   source/install links, read the shared
   [Application Identity Contract](../backend/application-identity.md).
9. Run local tooling through the shared
   [Development Environment Contract](../backend/development-environment.md).
10. For Codex Provider capability/restart UI or WorkBuddy navigation, consult
    the dedicated cross-layer note below and confirm it against current code and
    tests; do not infer behavior from an archived feature label.
11. For V2 title-bar drag, Overlay chrome, or a report that the window jumps
    off-screen after Windows maximize, read the
    [V2 Shell Contract](./v2-shell.md) and the
    [Main Window Layout Contract](../backend/main-window-layout.md) together.
    Gate the macOS drag strip with `shouldShowMacOverlayDragStrip()`, not
    userAgent. Do not shrink React layout to paper over native overflow.

## Guidelines

| Guide                                                                      | Use it for                                                                                               |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [Frontend Reuse Contract](./reuse.md)                                  | Default frontend preference: reuse existing owners; place chrome that other modules will use in `shared/` on first commit; do not wait for a third copy; port pre-V2 behavior without importing leftover UI. |
| [V2 Shell Contract](./v2-shell.md)                                         | Isolated V2 routes, styles, layer/platform boundaries, Overlay drag strip, lifecycle, and V2-only gates. |
| [Main Window Layout](../backend/main-window-layout.md)                     | Host maximize/min-size invariants; Windows overflow is not a renderer chrome bug.                        |
| [V2 Agent and Models Contract](./v2-agent-models.md)                       | Catalog v4 seven-entry order (Grok Build after WorkBuddy), shared `PRODUCT_DIRECTORY` / `CatalogOfficialLinks`, Agent detail only `direct` capabilities, Qoder unsupported third-party models (MCP jump), TRAE GET observation, OpenCode native persist, Codex installer, WorkBuddy, Provider quick setup including Grok Build, vendor icons, and secrets. |
| [V2 Skills and MCP Feature Contract](./v2-skills-mcp.md)                   | Seven catalog-ordered V2 Skill and MCP targets, disk-observed installed Skills, Skill 市场 discovery (`searchSkillHub` via official `/api/skills` page/category, 21 per page, no GitHub 管理仓库 / skills.sh tab), Skills page-level discovery scroll, leftover Gemini/Hermes round-trip, Qoder/TRAE/WorkBuddy live MCP files, secret handling, and responsive UI. |
| [V2 Prompts and Memory Native Business Contract](./v2-prompts-memory.md)    | Seven Prompt applications in Agent-catalog order plus Gemini / OpenClaw / Hermes, four fixed long-term memory resources, OpenClaw daily memory, native-only browser behavior, and data safety. |
| [External Agent P0 Safety](../backend/external-agent-p0.md)                | Cross-layer QoderWork/TRAE Work native command, persistence, network, permission, and evidence boundary.  |
| [Directory Structure](./directory-structure.md)                            | Selecting the existing frontend layer and test location.                                                 |
| [Component Guidelines](./component-guidelines.md)                          | UI primitives, props, styling, translation, and form composition.                                        |
| [Hook Guidelines](./hook-guidelines.md)                                    | Naming, placement, effects, cleanup, and stateful hook APIs.                                             |
| [State Management](./state-management.md)                                  | React state, Context, TanStack Query keys, mutations, and persistence.                                   |
| [Type Safety](./type-safety.md)                                            | Strict TypeScript, domain types, Zod schemas, and Tauri wire contracts.                                  |
| [Quality Guidelines](./quality-guidelines.md)                              | Core and desktop-contract checks, Vitest/MSW setup, translations, and accessible primitives.             |
| [Application Brand Assets](../backend/application-brand-assets.md)         | Cross-platform generated icons and renderer About reuse.                                                 |
| [Application Identity](../backend/application-identity.md)                 | FyAgent-owned runtime identity and factual repository/provenance boundaries.                             |
| [Codex Provider Configuration](../backend/codex-provider-configuration.md) | Codex native-capability controls, warnings, live-change result, and trusted restart handoff.             |
| [Codex Desktop Installer](../backend/codex-desktop-installer.md)           | Installer/restart facade DTOs, job snapshots, progress presentation, and trusted launch outcomes.        |
| [WorkBuddy Configuration](../backend/workbuddy-configuration.md)           | Top-level navigation, query isolation, model selection, overwrite confirmation, and credential lifetime. |

The integrated V2 shell has six non-empty product routes: Agents, Models,
Skills, MCP, Prompts, and Memory. All six use bounded feature ports. Prompts
manages the seven existing native prompt applications; Memory manages four
fixed OpenClaw/Hermes long-term resources plus OpenClaw daily memory. Browser
preview is explicitly native-only and contains no seeded business data. A merge
commit does not replace post-merge shell/browser validation.

## Quality Check

For frontend code changes, run the checks applicable to the affected behavior.
Confirm new UI follows [Frontend Reuse](./reuse.md) before merge.

```bash
mise run typecheck
mise run format:check
mise run test:unit
```

For desktop-shell, responsive-header, window-layout, or acceptance changes,
also run the mock and visual-preflight checks in
[Quality Guidelines](./quality-guidelines.md); they do not replace real native
desktop or installer evidence.

`package.json` remains the source of package-level frontend scripts. The
repository-level entrypoint is the generated `mise run` task API; Node.js and
pnpm versions come from `.node-version` and `package.json#packageManager`, not
from duplicate declarations in `mise.toml`.

## Evidence

- [package.json](../../../package.json) defines the renderer tooling and
  runnable scripts.
- [src/index.html](../../../src/index.html) selects the production renderer
  (`src/v2/main.tsx`).
- [src/v2/main.tsx](../../../src/v2/main.tsx) is the production composition
  root.
- [src/main.tsx](../../../src/main.tsx) remains the leftover V1 bootstrap and
  is not loaded by the production HTML entry.
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) records the maintained
  contribution expectations, including strict TypeScript and leftover-renderer
  translated UI text.
- [Development Environment](../backend/development-environment.md) summarizes
  the local mise command boundary enforced by repository tasks and tests.
