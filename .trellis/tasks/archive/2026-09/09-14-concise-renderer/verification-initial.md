# Initial verification — superseded by verification.md

This records the first implementation and its blocked continuation. The user
subsequently required all reported blockers to be resolved. The current results,
expanded repair boundary and final acceptance are in `verification.md`; the
baseline exception and pending-confirmation state below are historical only.

## Checkout and scope

Base: `b2cdc098b6a70b9429e3dfcbf6a94e23c81aefc0`, branch `dev/laiyongjie`.
The checkout was clean before this task. Source changes are limited to renderer
presentation and its tests; there are no native, port, domain, dependency,
configuration-format, route, CI or persisted-data changes.

## Completed focused checks

| Check | Result |
| --- | --- |
| `mise run typecheck` | Pass |
| `mise run lint` | Pass |
| `mise run format:check` | Pass |
| `mise run test:unit tests/renderer src/domain` | 110 files, 758 tests pass |
| `mise run test:desktop:mock` | 7 tests pass; mock contract verified, not native evidence |
| `mise run test:desktop:visual:preflight` | Pass; read-only, no approved baseline updated |
| `mise run build:renderer` | Pass; all eight route chunks verified |
| Production boot tests in the existing performance configuration | 2 pass; startup only, not a full performance profile |
| Concise-secondary browser tests, all configured projects | 10 pass; four Chromium sizes plus WebKit, both themes, eight populated routes |
| Full functional browser matrix | 586 pass in 6.5 minutes; final log completed, no failed/skipped tests reported |
| Trellis context validation | Pass; implement/check context entries resolve |
| Task validation and task documentation inside prearchive aggregate | Pass |
| Python lock / version contracts | Pass |
| `git diff --check` | Pass |

Final `mise run check:frontend`: typecheck, lint and format pass; the aggregate
unit run has 184 passing files and 2 failing files, with **1,640 passing tests,
3 baseline failures and 1 skipped test**. The three failures are listed below;
the full gate is not reported as passing.

The renderer build, desktop mock/preflight and production-boot checks were rerun
separately after that aggregate stopped. The final full-browser run completed
with **586 passing tests**; the command was:

```sh
mise exec -- pnpm exec playwright test --config config/playwright.config.ts --workers=4
```

No production source was changed while that final run was executing. Its local
log is `node_modules/.cache/concise-renderer/browser-full-final.log`; the final
summary is on line 635. It is functional browser evidence, not a performance
profile or native-platform acceptance.

## Continuation review

The continuation reused the existing task and reviewed all 41 tracked dirty
paths against its audit and design, rather than resetting or replacing the
previous work. No additional production source or test changes were needed.

The following commands were rerun against the current working tree:

```sh
mise run typecheck
mise run lint
mise run format:check
mise run test:unit tests/renderer src/domain
```

All commands exited 0; the unit result remains 110 files / 758 tests passed.
The local combined log is
`node_modules/.cache/concise-renderer/resume-renderer-checks.log`.
The unit run did emit React `act(...)` warnings from Agent directory tests;
the earlier aggregate log contains the same warning categories. This is not
claimed as a warning-free run, and warnings were not filtered or suppressed.
Unlike the three native assertion failures below, these warnings were not
independently reproduced in an unchanged HEAD worktree during this continuation.

The source/test diff identity at this review is:

```text
git diff --binary -- src tests/renderer tests/browser | shasum -a 256
ba47fce85a2e289115c8bf3cba7b7d20f1c039b3525c38da34479262a853759d
```

`git diff --exit-code -- src-tauri src/domain src/platform package.json
pnpm-lock.yaml config` exited 0. The existing narrow MCP and Memory captures
were inspected again: metadata stays within the detail pane and the Memory
toolbar does not collapse into unnecessary one-button rows. This spot check
does not expand the earlier manual-review claim to every capture.

SPEC changes are present before archival. The task remains `in_progress` until
the local work-commit plan is confirmed; no commit, archive or push has been
performed in this continuation. See `commit-plan.md` for the exact file scope.

After the closeout documents were updated, `git diff --check` and task context
validation passed again. The prearchive aggregate was also rerun: all 80 mise
tasks, task/document contracts and lockfile checks passed; the aggregate still
exited 1 on the same `src-tauri/src/services/tooling.rs` supported-platform
identity drift. The new log is
`node_modules/.cache/concise-renderer/resume-prearchive-contracts.log`.

## Independently reproduced baseline failures

`mise run check:frontend` includes repository native contracts, not only renderer
tests. The following failures were reproduced with the same command/filter in an
unmodified detached worktree at the base commit:

1. `tests/codexWindowsUserScopeContract.test.ts`: the test named
   `does not consume elevated-process user path environment on Windows` expects
   a source-text fragment containing a macOS `cfg`/`extend_from_cli_path_env` sequence that is not
   present in the checked-in native file.
2. `tests/remainingPlatformSurface.test.ts`: current repository scanner fails
   with `Supported-platform structure identity drifted: src-tauri/src/services/tooling.rs`.
3. The same file's source-seal test fails on that same native inventory drift.

Detached baseline command:

```sh
mise run test:unit tests/codexWindowsUserScopeContract.test.ts tests/remainingPlatformSurface.test.ts
```

Result: 3 failures, 32 passes. Both the checked-out `tooling.rs` and its HEAD blob
were `f23ef78992b4eb3c4b848ab527f6133b5ab352b2`. The temporary worktree was removed
after confirming it had no tracked changes; its dependency symlink alone was
unlinked. Other existing worktrees were not touched.

Prearchive command (the context ID is required):

```sh
TRELLIS_CONTEXT_ID=fyagent-concise-renderer-20260914 mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/09-14-concise-renderer
```

Its task/doc checks pass before the same supported-platform check fails.
`mise run release:check` also reports the existing supported-platform failure and
the same Codex Windows source-text contract, not an additional renderer failure.
No native inventory was resealed and no assertion was suppressed to report green.

## Browser and visual review

Existing functional tests retain assignment changes, secret/redaction boundaries,
copy actions, confirmation, focus/origin lifetimes, dirty editors, native-only
limits, row heights and scroll reachability. New regressions verify single-owner
metadata, actual switch state, optional descriptions, distinct account targets,
normal versus non-ready health copy, and compact memory toolbars.

Screenshots are generated for eight populated routes × two themes × five
configured projects. Chromium uses 900×600, 1152×640, 1232×700 and 1440×900;
WebKit uses 1232×700. Existing responsive tests also resize the content panes and
apply long strings and enlarged text. Representative captures were visually
reviewed across all eight route families, including the narrow Memory correction;
automated assertions cover the whole configured matrix. This does not claim
manual inspection of every pixel in every capture.

Local generated evidence lives under
`node_modules/.cache/concise-renderer/` (ignored): logs, `screenshots/` and
downsampled WebKit copies in `review/`. Browser `screenshot`/trace-on-failure
artifacts use the repository's normal temporary Playwright artifact directory.
These are review captures, not approved desktop baselines.

## Limits

No live account, network inference, native install, credential replacement,
Windows/macOS bundle acceptance, full performance profile or release candidate
was exercised. No benchmark budget, accessibility threshold or approved visual
baseline was relaxed. The repository-wide aggregate remains blocked as described
above even when the renderer-specific checks pass.
