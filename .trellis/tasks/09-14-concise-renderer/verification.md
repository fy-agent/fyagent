# Verification

## Scope and current result

Base: `b2cdc098b6a70b9429e3dfcbf6a94e23c81aefc0`, branch `dev/laiyongjie`.
The checkout was clean before the task. All eight renderer route families and
their secondary surfaces were audited; see `audit.md` for keep/remove decisions.

The user's final completion request supersedes the earlier proposal to archive
with known baseline failures. The three baseline assertions, React warning
guard/lifecycle issue and four additional Clippy errors have been resolved.
The full current-host prearchive check, functional browser repeat and corrected
canonical production performance suite pass. The final aggregate repeat after
the timing-harness and native test-port repairs also exits 0. Postarchive
verification is recorded below after the task move.

Production native changes are limited to two mechanical repairs: conditional
compilation of Windows-only npm string adapters (with host unit tests retained),
and removal of a redundant return binding. One existing native test additionally
uses an OS-assigned port and explicit listener teardown instead of assuming the
product default port is free. The reviewed platform identity manifest and source
contract tests are synchronized. No native operation, API/DTO, dependency,
configuration format, route, credential or stored user data was changed.

## Final checks

| Check | Result |
| --- | --- |
| Full `check:prearchive` with direct session identity | Pass; frontend, backend, environment and contract aggregates all exit 0 |
| Typecheck, ESLint, Prettier | Pass |
| Full unit suite | 187 files; 1,649 pass, 1 existing Windows-only skip, 0 failures |
| Renderer with restored act-warning guard | 104 files; 680 pass; no unexpected React warnings |
| Desktop mock | 7 pass; mock-only evidence |
| Visual preflight | Pass; no approved baseline changed |
| Rust formatting, check and all-target Clippy | Pass; 0 compiler warnings/errors, no allow/warning suppression added |
| Current-host Rust tests | 3,565 pass, 6 pre-existing explicit ignores, 0 failures across 19 result blocks |
| Task/docs/platform/Python lock/version contracts | Pass |
| Release contract aggregate | Pass; includes 619 passing contract tests, 1 host skip and 4 native-fetch tests |
| Renderer build / full functional browser repeat | Pass; all eight route chunks, 2 production boot cases and 586 functional browser cases |
| Serial production performance suite | 35 pass in 2.7m, with the corrected dedicated timing configuration; no CLI override |
| Trellis context | Both manifests resolve all 7 entries within injection limits |
| Canonical postarchive `mise run check` | Pending archive |

The one unit skip is the existing `it.runIf(process.platform === "win32")`
host test. Rust's six explicit ignores are two backup performance diagnostics,
two live S3 tests, a real Codex corpus replay, and matching-host OS credential-store
HIL. None was newly skipped or counted as passing. These require their own inputs
or authority and are not reasons to enable real external writes during UI work.

## Reproduction and logs

Use the repository toolchain, not the ambient system Node:

```sh
TRELLIS_CONTEXT_ID=fyagent-concise-renderer-20260914 \
  mise run check:prearchive --exclude-active-task .trellis/tasks/09-14-concise-renderer
mise run test:browser
mise run test:performance
```

The last two run sequentially after the first, so compilation and parallel
functional tests do not contaminate the serial production performance sample.
Final commands use existing thresholds, zero performance retries and the normal
project/browser configuration. No product source changes occur during these runs.

The first browser repeat was externally terminated mid-run (107 test completions,
no failed assertion or final suite summary). Its termination at 13:49:28 coincides
with the new DevSpace service process start time and loss of the original tool
session; the exact stop trigger was not established. Preserve that log as
`browser-interrupted.log`, not passing evidence. Only its identified orphan Vite
test-server process was stopped after verifying its PID, project path and port.
The entire canonical browser command was restarted with no source or threshold
changes and completed with 2 production boot cases and 586 functional cases
passing (6.7 minutes for the functional matrix). Its process exited before the
canonical serial performance command was started; the suites did not overlap.

Local generated evidence is under `node_modules/.cache/concise-renderer/`:
`prearchive-complete.log`, `browser-complete.log`, `performance-accepted.gRDHIN`.
The final aggregate repeat after the native test-port repair exited 0 and is
recorded as `prearchive-port-isolated.3aiKDV`: 1,649 unit passes and 3,565 Rust
passes. The earlier timing-config repeat is `prearchive-final.VdiXBx`.
The complete prearchive log records 0 React act warnings and 0 Rust compiler
warnings/errors. Generated logs/screenshots are ignored, not committed binaries.

## Repaired failures and prevention

The initial 3 native-source contract failures were independently reproduced at
unmodified HEAD (3 failed, 32 passed). They came from the two previous tooling
commits, not the UI patch. The five stale native identities were reviewed before
their individual hashes changed. The newly platform-scoped npm adapter and the
modified Windows contract test are themselves sealed as well. No scanner or
negative inventory assertion was relaxed.

The old Windows assertion assumed cfg/call adjacency. The replacement checks
the complete macOS PATH block, both login/process sources and absence of those
reads in other branches. Three mutation cases reject moving or copying the PATH
read outside, or admitting Windows through a widened guard.

Two new warning-guard tests failed before repair (`guard-red.log`): Vitest's
per-test mock restore removed the beforeAll guard. Installing it in beforeEach
restores the intended failure behavior. The Agent install-target test now awaits
readiness/inventory readback before teardown, instead of ending at deferred job
resolution. Its actual disabled configuration state is still asserted. The full
renderer passes with that guard enabled (`renderer-guard-first.log`).

The full native gate additionally exposed three Windows-only functions unused in
the normal macOS library and a redundant let-return. The exact platform/test cfg
and direct return repairs keep native semantics and all existing pure tests.
Detailed source review, primary references and SPEC owners are in
`research/validation-repairs.md`. The initial blocked review is retained in
`verification-initial.md` as history, not current acceptance.

Another complete repeat (`prearchive-final.log`, distinct from the successful
`prearchive-final.VdiXBx`) exposed a fixed-port fixture collision in the Claude
Desktop provider takeover test: its proxy could not bind the product default
port. The test now reuses the existing `listen_port: 0` support, asserts a nonzero
bound port in the generated profile URL and explicitly stops its own listener.
No unrelated local process was stopped. The focused native test and subsequent
complete gate both pass. Details and
the owning SPEC are in `research/validation-repairs.md`.

### Performance measurement repair

The traced performance configuration reproduced content-resize p95 above the
existing 33.4ms budget (33.5ms, then 50ms in an exclusive complete repeat).
`performance-final.log` is that failed instrumented run, not final acceptance.
Two unchanged-test repetitions with trace recording off passed; an independent
1x/4x diagnostic also passed. The actual configuration now separates untraced
timing from traced diagnosis, with a regression test importing both real configs.
All five configuration-ownership tests pass after the new test failed before
the repair. Functional browser failure tracing remains enabled.

The canonical 35-test performance suite then passed with no CLI override. At
1232×700, using the serial production Chromium runner:

| Metric | 1x CPU cost | 4x CPU cost |
| --- | --- | --- |
| Navigation p95 | 31.5ms | 55.3ms |
| Presentation frame p95 | 33.4ms | 33.4ms |
| Content-resize frame p95 | 33.4ms | 33.4ms |
| Maximum of theme-segment p95 values | 16.8ms | 16.7ms |

Normal navigation budget remains 100ms and normal frame budget 33.4ms. The
4x values are additional pressure measurements, not expanded normal budgets.
CPU profiles, real clocks, sample counts, geometry and teardown assertions remain
enabled. No product motion or benchmark assertion was changed. See
`research/performance-instrumentation.md` for the failed evidence, official
recording semantics, controlled comparison and logging incident.

The normal navigation/presentation/resize samples record no long tasks. At 4x,
navigation records one 53ms long task and presentation one 50ms long task; these
pressure observations are retained. Navigation uses 48 return samples per run;
content resize uses 20 warm cycles and 511/503 frame intervals at 1x/4x. The
accepted performance log has no NUL bytes and belongs to one exclusive run.

## Browser and visual evidence

Existing regressions retain assignment changes, copy/redaction boundaries,
confirmation, focus/origin lifetimes, dirty editors, native-only limits, intrinsic
row heights and scroll reachability. New assertions cover single-owner metadata,
actual switches, optional descriptions, distinct account targets, normal versus
non-ready Health copy and compact Memory toolbars. User-authored content remains.

The final complete functional run passed 586 tests. Captures cover eight
populated routes × two themes × five projects (80 images): Chromium 900×600,
1152×640, 1232×700 and 1440×900; WebKit 1232×700. Existing pressure tests resize
panes continuously and use long strings/enlarged text. Representative captures
were manually reviewed across all eight families, including the corrected narrow
Memory toolbar; automated assertions cover the complete matrix. This does not
claim pixel-by-pixel manual review of every capture.

These are renderer review captures in `screenshots/` and `review/`, not newly
approved desktop baselines. Playwright trace/attachment evidence uses the normal
temporary artifact directories.

## Delivery boundary

SPEC updates are prepared before the work commit and archive. The user's local
commit/archive authorization is recorded in `commit-plan.md`; no remote push,
release or publishing action is performed. Archive completion and the canonical
postarchive result will be recorded after execution.

Current-host native tests do not prove Windows runtime, installer/signing,
real-account login, credential replacement, live model inference or packaged
WebView behavior. No such user-data operation was performed. Performance numbers
describe this host's production Chromium run, not universal native latency.
