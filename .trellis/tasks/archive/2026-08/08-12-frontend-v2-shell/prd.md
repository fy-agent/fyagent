# FyAgent frontend V2 shell

## Goal

Establish the Phase 1 FyAgent V2 renderer path as a Windows-first, light,
Apple-inspired application shell that gives future frontend work an isolated,
testable foundation without migrating legacy business UI or changing Rust/Tauri
business behavior.

The first visible screen must contain only the FyAgent brand, six primary
routes, Search, Settings, an avatar placeholder, Windows window controls, and an
empty content surface. This phase is a development and visual framework, not a
release-ready replacement for the legacy bootstrap.

## Confirmed Facts

- The starting revision is `f424ceff8f085673d00b8fd191045cb965987408`
  (`v0.3.4`) on `dev/laiyongjie`.
- The starting worktree is clean and `src/v2` does not exist.
- React 18, Tailwind 3.4, Vite, Vitest 2, Radix primitives, and the required
  Tauri window permissions already exist.
- The existing `pythonrust` task for Windows runtime/Trellis decoupling is
  unrelated and must not be modified or archived by this task.

## Requirements

### Runtime and architecture

- Add a new runtime rooted at `src/v2/main.tsx` and switch `src/index.html` to
  that entry while preserving all legacy source files in place.
- Keep production V2 code within `app`, `pages`, `widgets`, and `shared`, plus a
  development-only UI Lab under `dev`.
- Do not import legacy `src/App.tsx`, `src/main.tsx`, `src/components/**`,
  `src/hooks/**`, `src/lib/**`, `src/i18n/**`, or `src/index.css` from V2.
- Use React Router 7 Data Mode with `createHashRouter`. The six primary paths
  are `/agents`, `/models`, `/skills`, `/mcp`, `/prompts`, and `/memory`;
  `/models` is the default and fallback. Selected navigation state comes only
  from router location.
- Register `#/__dev/ui-lab` only in development builds.

### Visible shell

- Use a light-only, Apple/Liquid-Glass-inspired visual system with independent
  `--fy-*` semantic tokens and safe, namespaced global styles.
- Render a transparent Y mark and `FyAgent`, the fixed navigation labels
  `Agent 目录`, `模型`, `Skills`, `MCP`, `提示词`, `记忆`, three inert but
  accessible tool controls, Windows minimize/maximize/close controls, and a
  completely empty content viewport.
- Keep all six navigation labels, the brand, tools, and window controls visible
  from 900x600 through 1440x900. Adapt with CSS Grid/Flex, media queries, and
  custom properties; do not drive ordinary layout with JavaScript width state.
- Provide default, hover, pressed, selected, and focus-visible states; use
  restrained 70-180ms motion and honor `prefers-reduced-motion`.
- Use the supplied transparent Y asset only for V2 header branding. Do not
  change packaged, About, tray, installer, or platform application icons.

### Dependencies and component boundary

- Preserve React 18, Tailwind 3.4, and Vitest 2.
- Add compatible React Router 7, Phosphor Icons, ESLint TypeScript support, and
  Playwright Chromium dependencies with the lockfile as executable authority.
- Do not retain `glasscn-ui`; its compatibility research triggered the approved
  stop-loss. Build V2-owned thin primitives from already installed Radix
  packages and FyAgent tokens without importing legacy wrappers or source-copying
  third-party code.
- Use Phosphor only in V2 and leave the legacy Lucide/shadcn configuration
  unchanged.

### Platform and lifecycle

- Define an internal `WindowFramePort` with browser and Tauri implementations.
  Direct `@tauri-apps/**` imports are allowed only below
  `src/v2/shared/platform/tauri/**`.
- Browser methods are safe no-ops. Windows Tauri prepares the frame with
  `setDecorations(false)` and delegates minimize, toggle-maximize, and close to
  the existing window API.
- Limit drag behavior to explicit empty header regions; interactive controls
  must not initiate window dragging.
- Emit the existing payload-free `frontend-deeplink-ready` event at most once
  per renderer lifetime in Tauri and no-op in the browser.
- Do not change Rust commands, events, DTOs, capabilities, configuration,
  persistence, database schemas, or backend business logic.

### Tooling and scope

- Add V2-only package scripts and canonical mise wrappers for lint, typecheck,
  unit tests, watch tests, and browser tests. Do not alter CI, Release,
  installer, or the existing full-check aggregation.
- Add executable import-boundary tests in addition to ESLint restrictions.
- Keep screenshots, traces, reports, and temporary test programs outside the
  tracked worktree or in ignored locations.

## Acceptance Criteria

- [x] `mise run env:check` passes.
- [x] `mise run lint:v2` passes and rejects legacy/Tauri boundary violations.
- [x] `mise run typecheck:v2` passes under strict TypeScript rules.
- [x] `mise run test:v2` passes router, shell accessibility, platform adapter,
      lifecycle idempotency, and architecture-boundary tests.
- [x] `mise run test:v2:browser` passes at 900x600, 1152x640, 1232x700, and
      1440x900 with no overflow, overlap, missing primary control, routing
      mismatch, inaccessible tab path, page error, or relevant console error.
- [x] UI Lab verifies Tooltip, Popover/Portal, focus ring, long multilingual
      labels, icon treatment, and glass fallback without horizontal scrolling.
- [x] `mise run build:renderer` succeeds and the production build omits the
      development-only UI Lab route.
- [x] Changed files pass the repository formatting check and `git diff --check`.
- [ ] A real Windows Tauri/WebView2 run verifies one title bar, drag/no-drag,
      minimize, maximize/restore, close, 1232-to-900 resize, backdrop blur, focus
      treatment, and brand rendering at the current host scale.
- [x] Visual evidence outside the repository covers 900x600, 1232x700, and
      1440x900 and is compared against the approved structural and design rules.
- [x] The final diff contains no Rust/Tauri backend business change, legacy
      deletion or relocation, packaged icon change, external reference-pack
      link/dependency, debug output, or temporary QA artifact.
- [ ] All task changes are committed, the Trellis task is archived, the session
      is recorded, and `git status --short` is empty.

## Out of Scope and Residual Risk

- No Provider, Models, Skills, MCP, Prompt, Memory, search, settings, or account
  business behavior is migrated.
- No dark theme, React 19, Tailwind 4, Storybook, CI/CD, installer, cross-platform
  release, or Release preflight work is included.
- The V2 entry intentionally does not restore legacy deep-link consumption,
  database recovery UI, models.dev startup synchronization, or other complete
  bootstrap semantics and therefore is not Release-ready.
- Native Windows 125% and 150% scaling remain explicit human acceptance because
  the implementation must not mutate the user's system display scaling.

## Validation Record (2026-08-12)

- Final non-interactive code gates passed: environment, V2 lint, strict V2
  typecheck, 8 Vitest files / 27 tests, renderer production build, formatting,
  task contracts, and whitespace checks.
- The four-project Chromium suite passed 16/16 before the final accessibility
  and error-boundary polish. The user explicitly stopped further interactive
  testing, so it was not rerun after those final code-only fixes.
- Browser visual evidence was captured outside the repository at 900x600,
  1232x700, and 1440x900. It showed the fixed information architecture, empty
  content surface, light palette, transparent Y mark, complete controls, and no
  horizontal overflow.
- A native process was started before interactive testing was stopped. It was
  not carried through the complete maximize/restore, close, 900 resize, or 150%
  DPI checklist, so the native acceptance item above remains deliberately open.
- The V2 entry remains a Phase 1 development/visual shell. Legacy deep-link
  consumption, database recovery UI, models synchronization, full startup
  semantics, native DPI coverage, and Release preflight remain follow-up gates.
