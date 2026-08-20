# Implementation Plan

## Batch 1: system-owned window contract

- Remove frame props/preparation from AppShell and TopBar; preserve the ready
  bridge and make its error message lifecycle-specific.
- Delete the custom WindowControls component, V2 window-frame platform
  modules, `WindowFramePort` exports/types, dedicated tests, and their CSS.
- Search V2 source/tests for remaining window-frame, drag-region,
  `setDecorations(false)`, or fake-control references.

Validation: focused V2 typecheck/unit tests and static negative searches.

## Batch 2: bounded selected lens and material system

- Add exact `@samasante/liquid-glass@0.1.1` through the pinned Mise/pnpm
  environment and update only package manifest/lock outputs.
- Add the internal `LiquidGlassLens`, integrate it only with active NavLink,
  and reuse it in the UI Lab for inspectable coverage.
- Replace V2 tokens, ambient background, structural/interactive glass,
  selected/tool/popover/content styles, and remove the selected underline.
- Preserve existing geometry and use CSS-only constrained-width adaptation.

Validation: V2 lint/typecheck, renderer build, focused browser Shell/UI Lab.

## Batch 3: regression coverage

- Update Router Shell unit tests for no frame injection, absence of custom
  chrome, Router/ARIA agreement, unique lens, ready idempotence, and nine-stop
  tab order.
- Update Playwright Shell tests for Brand/Nav/Tools geometry, negative custom
  chrome assertions, all nine controls, unique lens, empty content, route/ARIA
  agreement, and no errors at all four viewports.
- Update UI Lab browser tests for non-opaque surfaces, backdrop/fallback,
  edge/highlight/shadow, no selected underline, portal visibility, focus, and
  reduced-motion state independence.

Validation: `mise run test:v2` and `mise run test:v2:browser`.

## Integration and spec

- Update `.trellis/spec/frontend/v2-shell.md` to the executable system-owned
  chrome and bounded-lens contract.
- Run full-scope Trellis check and independent code/spec reviewers; fix only
  task-caused or acceptance-blocking findings.
- Run, in order: `mise run env:check`, `mise run lint:v2`,
  `mise run typecheck:v2`, `mise run test:v2`,
  `mise run test:v2:browser`, `mise run build:renderer`,
  `mise run format:check`, `git diff --check`, then `mise run check`.

## Acceptance-blocking baseline closure

- If the final full-project gate exposes a pre-existing failure, reproduce it
  with the narrowest canonical command and use Git history plus executable
  specs to separate baseline drift from this task's product diff.
- Repair only the directly blocking classifier/checker/test-harness contract,
  retain fail-closed negative cases, and synchronize an existing long-lived
  spec when the executable contract changes.
- Validate each repair with focused tests before rerunning `release:check` or
  the complete ordered gate. Do not skip, suppress, or weaken a failing gate.

## Commit and archive

- Commit the coherent implementation/spec/task artifacts as
  `feat(frontend): restore native chrome and liquid glass shell`.
- Archive the task through Trellis, producing its administrative commit.
- Record the session with the work commit hash, producing the journal commit.
- Verify no active task, no uncommitted/untracked paths, and do not push.

## Rollback points

- Do not change Rust/native config to compensate for a renderer failure.
- Do not substitute another package or expand the lens surface if the selected
  dependency fails required gates; keep the task active and report evidence.
- Never remove or include unrelated dirty paths in this task's commits.
