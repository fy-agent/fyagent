# Directory Structure

The production renderer is V2. `src/index.html` loads `src/v2/main.tsx`. Put
new product-shell and feature-page code under `src/v2/**` and follow
[V2 Shell](./v2-shell.md).

`src/App.tsx`, `src/components/**`, `src/hooks/**`, `src/lib/**`,
`src/i18n/**`, and `src/main.tsx` are leftover V1. They are not the product
shell. Change them only when a leftover test or leftover surface still
requires it.

## Production V2 Layout

```text
src/
|- index.html                 # production HTML; imports src/v2/main.tsx
`- v2/
   |- main.tsx                # production composition root
   |- app/                    # router, PersistentPrimaryOutlet, RootError, styles
   |- pages/<route>/          # agents, models, skills, mcp, prompts, memory
   |- widgets/app-shell/      # AppShell, TopBar, Brand, PrimaryNav,
   |                          # ToolCluster, ContentViewport
   |- shared/
   |  |- config/              # navigation source
   |  |- assets/              # agent and app icons
   |  |- ui/                  # primitives, catalog, split, SelectionLens, FeatureTabs/Search/List/Pagination, ExternalLinkButton
   |  |- features/            # ports, types, queries, FeatureProvider
   |  |- platform/            # tauri/browser adapters, runtime, lifecycle
   |  |- design-system/
   |  `- codex-desktop/       # V2 panel/hook over @/shared/codex-desktop
   `- dev/                    # DEV-only UI Lab
```

- One folder per first-level route under `pages/<route>/`.
  Agent directory scan-driven order lives in
  `pages/agents/agentDirectoryOrder.ts` plus `useAgentDirectoryScan.ts`.
  Domestic priority is `PRODUCT_DIRECTORY[].directoryPriority` in
  `shared/features/directory.ts`, not a page-local `Set`.
- Overlay chrome (macOS drag strip) lives in `widgets/app-shell` (`TopBar`
  inside `AppShell`). It is window chrome, not a feature route.
- Feature pages talk to native code only through `shared/features` ports and
  `shared/platform` adapters.
- Reuse is the default. Put chrome, helpers, and hooks that another route or
  later sibling module will use under `shared/ui` or `shared/features` on the
  first commit. Exclusive tracks, management search, feature lists, and
  numbered feature pagination use `FeatureTabs` / `FeatureSearch` /
  `FeatureList` / `FeaturePagination`. See
  [Frontend Reuse](./reuse.md).

## Leftover V1 Layout

```text
src/
|- main.tsx                 # leftover bootstrap; not the production entry
|- App.tsx                  # leftover shell and view selection
|- components/
|  |- ui/                   # leftover Radix/Tailwind primitives
|  |- theme-provider.tsx
|  |- topbar/               # leftover TopLevelHeader chrome
|  |- providers/
|  `- mcp/
|- hooks/
|- lib/
|  |- api/                  # leftover Tauri facades
|  |- layout/
|  |- query/
|  `- schemas/
|- config/
|- i18n/ and icons/
|- types/ and types.ts
`- utils/
```

That leftover tree does not use a route-per-folder layout. Do not add new
product pages there.

## Placement Rules

### V2

- Pages import `shared/**` and same-route modules. They must not import
  `widgets`, `app`, or `dev`. Current pages also use `shared/design-system`
  and the V2 `shared/codex-desktop` panel; do not treat `ui` / `features` /
  `assets` as an exclusive allowlist.
- Do not add a second tabs, search, or list recipe under `pages/<route>/`
  when [Frontend Reuse](./reuse.md) already names a shared owner.
- Widgets import `shared/**` and sibling widget modules. They must not import
  `pages`, `app`, or `dev`.
- Tauri `invoke` stays under `shared/platform/tauri/**`.
- Do not import leftover `src/components/**`, `src/hooks/**`, `src/lib/**`,
  `src/i18n/**`, or `src/index.css`.

### Leftover V1

These rules apply outside `src/v2/**`.

- Put reusable primitives in `src/components/ui/`.
- Put domain UI with its owning leftover feature, such as
  `src/components/providers/forms/`.
- Keep a hook in `src/hooks/` when it is reused outside one leftover feature.
- Feature-level leftover Tauri calls stay in `src/lib/api/`. Narrow
  direct-`invoke` exceptions remain `src/main.tsx`,
  `src/components/theme-provider.tsx`, and
  `src/components/DatabaseUpgrade.tsx`.

## Test Placement

V2 unit tests live under `tests/v2/`. V2 Playwright tests live under
`tests/v2-browser/`. `vitest.config.ts` excludes both from
`mise run test:unit`.

Leftover tests mirror their subject under `tests/components/`,
`tests/hooks/`, `tests/lib/`, and `tests/integration/`. A small number of
pure utilities have adjacent `*.test.ts` files under `src/utils/`.

`tests/desktop-acceptance/` owns mock-only desktop acceptance contracts.
`tests/e2e/visual-baselines/` owns candidate-only visual baselines; it is not
a locally runnable real-desktop E2E runner.

## Evidence

- [src/index.html](../../../src/index.html) loads the production V2 entry.
- [src/v2/main.tsx](../../../src/v2/main.tsx) is the production composition root.
- [src/v2/app/router.tsx](../../../src/v2/app/router.tsx) mounts one page
  folder per product route.
- [src/v2/widgets/app-shell/AppShell.tsx](../../../src/v2/widgets/app-shell/AppShell.tsx)
  composes Overlay chrome and the content viewport.
- [src/main.tsx](../../../src/main.tsx) still composes leftover V1 providers
  before rendering `App`, but is not the HTML entry.
