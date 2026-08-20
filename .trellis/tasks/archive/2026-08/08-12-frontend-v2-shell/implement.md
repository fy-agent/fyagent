# FyAgent frontend V2 shell - Implementation Plan

## 1. Planning and context

- [x] Validate PRD/design/implementation artifacts and curated manifests.
- [x] Set task branch/base/scope metadata and run the planning convergence gate.
- [x] Activate the task only after all required artifacts validate.

## 2. Shared foundation (integration owner only)

- [x] Add compatible React Router 7 and Phosphor runtime dependencies plus
      ESLint/TypeScript and Playwright development dependencies.
- [x] Add V2 package scripts, canonical mise wrappers, V2 tsconfig, ESLint flat
      configuration, and Playwright configuration without changing CI or the
      full-check DAG.
- [x] Switch `src/index.html` title and module entry while preserving legacy
      sources.
- [x] Copy the transparent Y header asset into the V2 asset boundary without
      changing application icon consumers.

## 3. Parallel implementation after contracts are frozen

- [x] Router owner: implement `main`, router, root error, six empty pages, and
      dev-only UI Lab registration.
- [x] Shell owner: implement tokens, safe globals, motion, V2 primitives,
      AppShell/TopBar/Brand/PrimaryNav/ToolCluster/WindowControls/
      ContentViewport, UI Lab content, and responsive behavior.
- [x] Platform owner: implement `WindowFramePort`, runtime detection, browser
      no-op, Tauri window adapter, lifecycle idempotency, and focused unit tests.
- [x] Test owner: implement router/shell/architecture tests and four-viewport
      Playwright geometry/UI Lab smoke tests against the frozen contract.
- [x] Integration owner: reconcile imports/selectors/scripts and reject any
      change outside the assigned write sets.

## 4. Automated verification

- [x] `mise run env:check`
- [x] `mise run lint:v2`
- [x] `mise run typecheck:v2`
- [x] `mise run test:v2`
- [x] `mise run test:v2:browser`
- [x] `mise run build:renderer`
- [x] Repository formatting check for changed files and `git diff --check`
- [x] Diff assertions: no `src-tauri` business change, no legacy deletion, no
      packaged icon change, no external pack reference, no QA artifact.

## 5. Rendered and native verification

- [x] Use a local renderer preview for models-route identity, primary navigation
      interaction, console health, UI Lab Tooltip/Popover/focus, and screenshots.
- [x] Capture temporary 900x600, 1232x700, and 1440x900 images outside the repo;
      inspect reference and render side by side and close every unapproved drift.
- [ ] In Visual Studio Developer PowerShell run `mise run system:check` and
      `mise run dev`; verify the real Tauri window, one title bar, drag/no-drag,
      controls, 1232-to-900 resize, WebView2 blur, focus, and brand rendering.
- [x] Record native 125%/150% scale as pending human acceptance; do not mutate
      host display settings or substitute browser scaling as native evidence.

## 6. Review, spec sync, and finish

- [x] Dispatch parallel Trellis/code-quality and UI/accessibility review; verify
      each finding against the actual diff and fix all blocking issues.
- [x] Re-run the non-interactive code gates after the last fix; retain the prior
      browser result without further interactive testing as explicitly requested.
- [x] Update frontend Trellis specs with durable V2 exceptions while preserving
      legacy rules.
- [ ] Create the work commit in Phase 3.4.
- [ ] Archive only this task, record the `pythonrust` session journal, and verify
      `git status --short` is empty. Do not push or create a pull request.
