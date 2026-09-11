# Renderer Directory Structure

## Scope and ownership

The product has one renderer. `src/index.html` statically loads `src/main.tsx`;
there is no alternative application, generation-prefixed root or offline HTML
generator. Vite serves development/automated fixtures over loopback HTTP and
builds the ordinary multi-chunk application into `dist`.

```text
src/
  index.html, main.tsx        application entry and composition
  app/                       router, persistent outlet, errors and global styles
  pages/<route>/             eight product routes and route-local panels/state
  widgets/app-shell/         top bar, navigation and window chrome
  shared/
    assets/, config/, design-system/
    ui/                      pure visual controls, dialog and resize adapters
    features/                product contracts, queries and feature-aware controls
    platform/                browser/native adapters; Tauri imports only in tauri/
    codex-desktop/           current installer panel and hook
  domain/                    React/Tauri-independent parsing and serialization
  dev/                       development-only component lab
tests/
  renderer/                  production page, port, component and contract tests
  browser/                   Chromium/WebKit interaction and production profiling
  domain/                    retained pure configuration/security behavior
  architecture/, shared/    ownership and portable domain contracts
  desktop-acceptance/        explicitly mock-only native acceptance fixtures
```

`domain/codex-desktop` owns portable installer DTOs/parsers/version logic, not
the UI in `shared/codex-desktop`. Configuration presets and serialization rules
retained for native interoperability live under `domain/configuration`; these
are not a second user interface. Keep their existing protocol and persisted IDs.

## Placement contracts

Pages depend on their route and shared/domain modules, not application roots or
widgets. Widgets depend on shared and sibling widgets. Shared modules do not
import pages/widgets/app/dev. Domain modules never import React, Tauri, UI or
notification runtimes. Exact import checks live in
[Modular Boundaries](./modular-boundaries.md).

`shared/ui` owns visual controls and geometry. A control consuming FeaturePorts,
catalogues or notifications belongs to `shared/features/controls`, without a
reverse UI re-export. Split panes adapt the maintained resize library through
`shared/ui/split/vendor.ts`; pages supply constraints, never another drag engine.

Do not retain a `legacy` application as a cosmetic replacement for a versioned
tree. Move only still-needed pure contracts/tests, then remove retired runtime,
assets and dependencies. Update effective imports, assets, scripts, generated
tool documents and SPEC links. Archive prose/commit history records historical
facts; executable JSONL context references must point to the current contract.

## Verification and failure behavior

Root `tsconfig.json` and `eslint.config.mjs`, plus `config/vitest.config.ts`, serve the current
renderer. Vitest projects separate renderer, portable contracts and local
host-integration setup, not product generations. Browser/performance configs are separate by purpose;
performance is serial and never concurrent with the full compile gate.

Explicit build/test/graph/PostCSS configurations live in `config/`. Public
commands keep one entry and use built-in `--config`; the root does not contain
forwarding copies. [Repository Layout](../backend/repository-layout.md) owns
root exceptions, configuration-relative paths and the executable inventory.

Run `mise run typecheck`, `mise run lint`, `mise run test:unit`,
`mise run test:browser` and `mise run build:renderer`; the standard full gate is
`mise run check`. Root `test:unit` includes the product renderer. A move that
leaves unresolvable imports, an unscanned source role, stale job filters or an
invalid context reference fails the migration, even when one page still boots.

Good: preserve portable Codex parsing in `domain`, keep Tauri decoding in the
platform port, and retain its tests. Bad: rename the old shell to `legacy` or
delete its security assertions without an explicit retirement/replacement map.
