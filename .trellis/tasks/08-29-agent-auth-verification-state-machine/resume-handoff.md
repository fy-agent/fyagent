# Stage 4 completion evidence before archive

> This record captures the reviewed local state immediately before the final
> work commit and exact-head CI. It is evidence, not an archive or native-HIL
> claim.

- captured_at: `2026-08-30T17:18:52+0800`
- branch: `dev/laiyongjie`
- pre_commit_head: `4e914358b731227c7f8d77c144f855bc4e1e1917`
- pre_commit_upstream: `4e914358b731227c7f8d77c144f855bc4e1e1917`
- task: `08-29-agent-auth-verification-state-machine`
- local_state: ready for work commit, push, exact-head CI, and Trellis archive

## Completed behavior

- Auth owns a separate observation and session domain rather than returning an
  install-job success after opening a terminal or application.
- Claude uses bounded official `auth status` output and matching exit semantics
  before reporting verified login/logout.
- OpenCode exposes bounded Provider connections instead of a global login bool.
- Grok Build and desktop Agent flows stay explicitly handoff-only where no
  reviewed authoritative observer exists.
- Codex delegates to the existing FyAgent Auth Center and does not duplicate
  OAuth state.
- Stop-waiting ends FyAgent monitoring without claiming the external flow was
  cancelled.
- An active process-local session can be recovered after a renderer reload by
  canonical Agent ID. No path, URL, command, credential, or browser-storage
  persistence was introduced.

## Local verification

- `TRELLIS_CONTEXT_ID=chatgpt-stage4-20260830 mise run check:prearchive --exclude-active-task .trellis/tasks/08-29-agent-auth-verification-state-machine` passed.
- Rust library result: `2945 passed; 0 failed; 5 ignored`.
- V2 result: `55 files / 393 tests passed`.
- Browser result: `140 tests passed` across the four configured Chromium viewports.
- `mise run lint:v2`, `mise run typecheck:v2`, `mise run rust:fmt:check`,
  `mise run rust:clippy`, `mise run supported-platform:check`, release contracts,
  task contracts, desktop mock acceptance, and visual preflight passed.
- The supported-platform manifest covered `2306` current files.

## Existing non-blocking test diagnostics

The repository still emits known React `act(...)` and MSW missing-handler
warnings in unrelated test families. They do not fail Stage 4 and are owned by
Stage 5 frontend reliability work; this task does not suppress them.

## Explicit residual risk

Real-account/native macOS and Windows HIL remains unexecuted. Portable and
browser tests do not prove vendor account behavior, terminal/browser launch,
Windows Explorer-user identity, or a real external login. Any later HIL must use
disposable/test accounts where permitted and record only bounded state codes,
never account identifiers or credential material.

## Dependency state

- Stage 1 archived: `.trellis/tasks/archive/2026-08/08-29-agent-install-target-authority`
- Stage 2 archived: `.trellis/tasks/archive/2026-08/08-29-macos-agent-in-place-update`
- Stage 3 archived: `.trellis/tasks/archive/2026-08/08-29-windows-agent-discovery-install`
- Stage 4 remains active until the work commit is pushed, exact-head CI is
  successful, and `task.py archive` creates the archive commit.
