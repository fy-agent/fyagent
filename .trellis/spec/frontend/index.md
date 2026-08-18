# Frontend Development Guidelines

These guidelines describe the renderer patterns observed in this checkout of
FyAgent. They are evidence-based reference material for changes under
`src/` and related renderer tests, not a proposed frontend redesign.

## Pre-Development Checklist

Before changing renderer code:

1. For any `src/v2/**` change, read the
   [V2 Shell, Prompt and Memory Contract](./v2-shell.md) first. It is the only exception
   to the legacy route-placement, styling, primitive, and translation rules;
   the guidelines below remain authoritative outside V2.
2. Read the nearest relevant guideline and inspect the existing feature,
   primitive, and executable tests.
3. Locate the existing Tauri API facade, query hook, type, schema, and test
   family before creating another one.
4. Classify state as local UI state, Context state, or backend/resource state.
5. For user-visible text, locate the matching keys in all four registered
   locale files before adding a literal string.
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

## Guidelines

| Guide                                                                      | Use it for                                                                                               |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [V2 Shell, Prompt and Memory Contract](./v2-shell.md)                      | Isolated V2 routes, local-Agent Prompt/Memory prototypes, standalone preview, platform boundaries, and gates. |
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

## Quality Check

For frontend code changes, run the checks applicable to the affected behavior:

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
- [src/main.tsx](../../../src/main.tsx) shows the renderer provider boundary.
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) records the maintained
  contribution expectations, including strict TypeScript and translated UI
  text.
- [Development Environment](../backend/development-environment.md) summarizes
  the local mise command boundary enforced by repository tasks and tests.
