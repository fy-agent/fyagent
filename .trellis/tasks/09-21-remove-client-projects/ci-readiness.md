# PR #196 merge-readiness follow-up — 2026-09-22

## Scope and provenance

The user authorized normal CI repair and ordered merge preparation for #196, then #197. This follow-up starts at `e321081f4f2f87d6279abcbbdbbff4bbac5f4cd6` in isolated branch `codex/pr196-merge-readiness-20260922`; the PR remains [#196](https://github.com/fy-agent/fyagent/pull/196) on `codex/remove-client-projects-20260921`. The original implementation worktree and #197 are untouched. No product source, release version, CI gate, timeout, retry policy, or repository settings changed.

CI run `35629400204` had two failing jobs. The contract failure named five concrete workstation-path occurrences across `execution-result.md`, `research/module-scope.md`, and `task.json`. Those home prefixes now use `/Users/<username>/`; original branch names, commit evidence and historical conclusions remain. Original text is recoverable at the starting commit above.

The sole browser failure was WebKit `responsive-density.spec.ts`, MCP draft retention across resize and route return, at the Escape/zero-dialog assertion. The original case passed locally 3 serial runs, 12 runs with 4 workers, and all 30 cases in three WebKit whole-file rounds. Therefore the hosted failure itself was not reproduced and its exact engine timing remains unproven. The source/test review found a missing keyboard-readiness precondition: restored draft text alone was followed immediately by a global Escape. The test now additionally waits for the dialog's normal initial template-control focus. It does not call `focus()`, click to force focus, sleep, repeat Escape, or weaken cleanup/draft/zero-write assertions. This is a test synchronization correction; it does not claim a reproduced product defect.

## Final scoped validation

- Locked repository bootstrap: passed; dependency versions and lockfiles unchanged.
- Full `responsive-density.spec.ts` using the repository Playwright configuration: **50/50 passed**, four Chromium viewports plus WebKit, 2 workers, 46.5 seconds.
- `mise run typecheck`: passed.
- `mise run lint`: passed.
- `mise run check:contracts` after privacy correction: passed, **35 files / 652 tests passed / 1 existing skip**, plus **4 native-fetch tests**. This precedes the lifecycle update below; the postarchive gate is authoritative for final submitted docs.
- No Rust code changed in this follow-up. Existing hosted Rust results are independent CI evidence, not new local native verification.

Raw logs remain outside the repository in the coordinator workspace under `work/merge-047/pr196-fix/`: `bootstrap.log`, `browser-before.log`, `browser-stress-before.log`, `browser-file-before.log`, `browser-final.log`, `typecheck.log`, `lint.log`, and `contracts.log`. The full failed CI logs are one directory above. These local logs are diagnostic evidence, not distributable repository fixtures or another platform's runtime evidence.

## Lifecycle

The previously completed-but-unarchived task was reopened for this bounded repair. Direct session is `session:codex-pr196-merge-readiness-20260922`. Applicable context now includes the quality contract. The prearchive wrapper in this historical branch resolves `python` from PATH; running it inside the canonical `python:run` environment supplies the locked managed Python without changing the checker or machine environment.

Final prearchive/archive/postarchive results will be recorded after execution. This report supersedes only earlier task statements about having no commit or PR and needing no further merge-readiness work; it preserves the historical implementation and local acceptance evidence. The parent coordinator owns push, hosted exact-head CI, Merge Queue admission and final merge readback.
