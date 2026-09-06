# Quality Guidelines

One `config/vitest.config.ts` runs renderer and domain/contract tests through separate
environment projects, not product generations. `mise run typecheck`, `lint`,
`format:check` and `test:unit` cover the current renderer and retained contracts.
Use the [Frontend Quality Check](./index.md#quality-check). Product copy is
Simplified Chinese; manuals in other languages are not a locale runtime. New UI must
follow [Frontend Reuse](./reuse.md): reuse existing shared owners; if a new
component will be used by another module, put it in `shared/` on the first
commit.

## Reproducible Core Frontend Checks

For an ordinary renderer change, start with the repository task API:

```bash
mise run typecheck
mise run lint
mise run format:check
mise run test:unit
```

Run local checks through the repository's
[mise environment](../backend/development-environment.md). Do not report a
frontend command as a successful project check unless `package.json` declares
it.

### Desktop Shell and Acceptance Contract

For desktop-shell, responsive-header, window-layout, or desktop-acceptance
changes, also run:

```bash
mise run test:desktop:mock
mise run test:desktop:visual:preflight
```

`test:desktop:mock` is mock-only and must not be reported as a real desktop,
installer, or platform run. Visual-baseline capture/update is candidate-only,
requires reviewed evidence, and does not replace ordinary local checks;
`test:desktop:visual:update` is not an unattended baseline-writing command.
Windows maximize overflow is a host `set_min_size` invariant; mock, Playwright,
and macOS `rust:test` cannot close that acceptance gap. See
[Main Window Layout](../backend/main-window-layout.md).

## Test Setup and Patterns

Vitest projects explicitly define setup rather than inheriting one another.
The contracts project loads `tests/setupGlobals.ts` and `tests/setupTests.ts`;
MSW/native-fetch fixtures retain their transport setup and cleanup. The renderer
project loads `tests/renderer/app/setup.ts`, preserving its jsdom/native signal
bridge and cleanup. Removed i18n setup must not reappear as a phantom dependency.

Component tests use React Testing Library (`render`, `screen`, events, and
role-based queries). Hook tests use `renderHook` and `act`. Tests that need
TanStack Query create a client with retries disabled so failures are immediate.

Responsive density verification includes continuous large→small→large viewport
sequences and real pane dragging, not only fresh loads at preset sizes.
`responsive-density.spec.ts` checks intrinsic row heights, grouped actions,
flexible-detail growth, local card/metadata widths and real draft-node identity
in Chromium/WebKit. Its failing baseline and final production runs belong in
the task review; an empty overflow/error report alone is not layout evidence.

Reusable motion/observer tests must also cover lifecycle isolation: dispose one
control while another still updates, and re-register a selected host after its
decorative overlay already exists. Passing initial-mount geometry alone does
not prove independent cleanup or that an observer excludes its own output.

### Renderer Warning and Lifecycle Evidence

`tests/renderer/app/setup.ts` keeps Node's native Request/fetch and translates only
DOM `addEventListener` signal options when the adopted jsdom environment and
Node use different AbortSignal realms. A WeakMap reuses one real DOM controller
per native signal; cancellation and the original reason propagate, including
already-aborted signals. Both EventTarget prototype listeners and Vitest's
bound window listener are covered and restored after the suite. Never drop
`options.signal`, replace native fetch/Request, or skip press tests to hide a
realm error. This narrow upstream-compatible bridge is removable when the
adopted test environment provides it. `tests/renderer/app/abortSignalRealm.test.ts`
must prove window/element cancellation, multiple targets and native Request
abort reasons before it is changed.

The jsdom renderer setup explicitly records `window.scrollTo` as a test double
and restores the original at suite teardown. Motion's auto-height measurement
may restore viewport scroll, but jsdom has no real layout/scroll implementation.
Do not interpret this double as scrolling evidence or suppress console errors
to hide unsupported APIs. Browser focus, anchored controls and overflow tests
continue to use the real browser implementation.

Targeted Renderer interaction suites must fail on unexpected React warnings rather
than filtering stderr or globally mocking `console.error`. Async state changes
are awaited through Testing Library async helpers, `act`, or controlled fake
timers. A dependency warning may be allowlisted only by one exact message and
reviewed version, with an upstream reference and removal condition; broad
regular-expression suppression is prohibited.

Route/lifecycle tests prove both sides of lazy ownership: prefetch may request
an unvisited module, but its page is not mounted and creates no queries/observers; a visited
primary route stays mounted behind `PersistentSurface` with queries disabled
while hidden; returning to it must not flash 「正在加载页面」. Browser tests
also exercise semantic selected state with the decorative Lens disabled,
missing/delayed `ResizeObserver`, reduced motion, and right-side interaction.
Primary-nav lens tests during Agent directory scan require
`backdrop-filter: none` and `lens.right <= host.right + 0.5`.

Production builds must emit separately identifiable primary-route chunks. A
build contract inspects the generated manifest/chunk graph and an app-owned
initial-chunk budget; do not raise Vite's warning threshold to hide a
monolithic entry. Vendor budgets must name their source and remain separate
from the app route budget.

The browser gate also boots the production bundle and visits all seven routes
through `config/playwright.performance.config.ts` (the `production boots` case).
Passing Vite dev-server tests or producing a manifest does not prove bundled
module initialization. `config/vite.config.ts` uses Rollup's dependency-aware named
entry groups, not a catch-all node_modules path partition that can split React
initialization from its helpers and produce cross-chunk cycles.

For navigation profiling run `mise exec -- pnpm exec playwright test --config
config/playwright.performance.config.ts`. It uses a serial production server,
1232×700 viewport, 42 revisits at 1× and 4× CPU cost, CPU profiles and long-task
records. The normal-speed local target is p95 ≤100ms from semantic link
activation to the frame after visible destination DOM; it excludes OS input
dispatch, data freshness and animation settling. Report those limits, not a
claim about all native WebViews. Do not raise the existing build budgets.

The same production configuration also runs `presentation-performance.spec.ts`.
It also selects `dialog-origins.spec.ts` and `mcp-followup-origins.spec.ts`:
production CSS/chunks must preserve asynchronous entry timing, transient-source
handoff and complete teardown. A source data attribute alone is not evidence
of a visible entrance. Keep physical wheel/keyboard reachability under
`scroll-ownership.spec.ts`; ownership rules live in
[Surfaces](./surfaces-responsive.md) and [Motion](./motion-system.md).
When relocating configuration, compare exact project/file/test-name collection
before and after, not just totals; follow
[Repository Layout](../backend/repository-layout.md) for discovery boundaries.
`state-performance.spec.ts` independently exercises one cold and twenty warm
next/back pairs in the same login dialog at 1x/4x CPU cost, requiring real
intermediate heights and cleanup. Normal frame p95 remains 33.4ms. Layout
measurements are reported, not conflated with route activation latency.
`theme-performance.spec.ts` separately records capture preparation and each
third of the radial reveal. Functional browser configuration excludes all
`*-performance.spec.ts` files: a parallel development-server run cannot serve
as production performance evidence. See [Appearance](./appearance.md).
The `production boots` timing case must see actual 420ms entry and 360ms exit after
CSS optimization; `.42s` and `420ms` are equivalent units, not different timings.
Supplemental presentation sampling separates a cold cycle from 20 warm cycles
at 1x/4x CPU cost, reports frame intervals/JS/layout/long tasks and verifies cleanup.
The 1x warm-frame p95 target is 33.4ms. Normalize only sub-nanosecond floating-point
subtraction noise; never increase the frame budget or replace real motion with
test-only no-animation code. Background machine load is reported, not hidden.

Static geometry/contrast assertions wait for actual settled state. Paused native
keyframes verify source/80ms press lead/252–420ms content handoff and reverse
tracks, alongside real-time mouse/keyboard/touch, interruption and resource checks.
Event dispatch/focus completion does not imply Router's state commit completed;
await the exact selected-state assertion rather than arbitrary sleeps.
Startup module delay/abort fixtures match exact URL pathnames independently
of Vite cache-busting queries; still assert that interception actually occurred.
Keep production-bundle startup tests separate from those dev-module fixtures.

Test organization is mirrored under `tests/renderer/`, `tests/browser/` and
`tests/domain/`; native/tooling contracts remain in their established suites.
Use the closest existing test as the fixture/mocking model for the behavior
being changed; this repository has no documented universal coverage threshold.

### Native Fetch, MSW, and Deprecation Boundary

The Node test runtime is exactly the version in `.node-version`. Before MSW or
any Tauri mock is installed, `tests/setupGlobals.ts` requires native `fetch`,
`Headers`, `Request`, and `Response` functions and rejects a `fetch.polyfill`
marker. Tests must fail when that baseline is absent; they must not install
`cross-fetch`, `node-fetch`, `undici`, or another compatibility layer.

`tests/msw/nativeFetchTauriMock.test.ts` owns the focused transport behavior
contract. It must exercise the real path from the mocked Tauri `invoke` call,
through Node native Fetch and MSW, back through response parsing. Keep all four
cases: JSON success plus invocation recording, a non-2xx text error, a 204
empty response mapped to `undefined`, and `Headers` created in a separate
jsdom realm. A global-existence assertion or `instanceof` check alone is not a
replacement for these requests.

All ordinary Vitest and desktop contract package scripts launch Node
with the portable `--throw-deprecation` flag. The focused transport command adds
the pending-deprecation gate:

```bash
mise run test:unit
mise run test:native-fetch
```

The focused pending probe is deliberately supplemental. The Node runtime
selected by `.node-version` may not surface a pending deprecation originating
below every `node_modules` path, so dependency proof is owned by
`scripts/tasks/dep0040-check.mjs` and its contract tests. They parse the
manifest, active module specifiers, the versioned pnpm lock, and argv-based
`pnpm why --json` reverse paths; reject the obsolete `cross-fetch` chain; and
admit only the explicit, versioned remaining origins encoded in that executable
allowlist. This spec does not duplicate the package versions or ancestor
suffixes. Adding or upgrading a watched origin requires a new reverse-path
review and matching checker/test change; it is not a general allowance for
every userland `punycode` path.

The report fails closed on malformed active modules, non-canonical watched
lock entries, package/snapshot disagreement, unexplained aliases, and watched
reverse paths outside those two reviewed ancestries. Its suppression scan owns
the runnable package, workflow, mise, and script surfaces; statically composed
JavaScript arguments and shell/PowerShell script files are not escape hatches.
Negative detector fixtures belong in the contract test input, not in a scanned
execution script.

Never use `NODE_NO_WARNINGS`, `--no-warnings`, `--no-deprecation`,
`--disable-warning=DEP0040`, or stderr filtering to make these gates pass.

## UI Text and Accessible Primitives

Follow [Localization](./localization.md) and
[User-Facing Copy](./user-facing-copy.md) for the current Chinese product UI.
Retired locale assertions are recorded as retired scope, not relabelled as
passing current coverage. Browser fixtures never claim native locale evidence.

Shared primitives already carry focus-visible styling and form ARIA linkage.
Preserve those properties when editing them, and test interactive behavior
through accessible roles where the nearby tests do so.

### Canonical task invocation

Use `mise run test:unit` rather than the package-manager executable shim for
host-integration gates. The shim can inject `NODE_PATH`; native tasks correctly
reject loader overrides before processing Cargo arguments. Do not weaken the
native environment guard to make a noncanonical test invocation pass.

## Evidence

- [package.json](../../../package.json) defines the runnable type-check,
  formatting, unit-test, browser and desktop-acceptance scripts.
- [vitest.config.ts](../../../config/vitest.config.ts) configures the `jsdom`
  environment and shared setup files.
- [tests/setupTests.ts](../../../tests/setupTests.ts) manages Testing Library,
  MSW, cleanup, and mock reset lifecycle.
- [tests/msw/nativeFetchTauriMock.test.ts](../../../tests/msw/nativeFetchTauriMock.test.ts)
  exercises native Fetch, MSW, Tauri mock parsing, and cross-realm headers.
- [scripts/tasks/dep0040-check.mjs](../../../scripts/tasks/dep0040-check.mjs)
  owns the dependency graph and warning-suppression report.
- [Development Environment](../backend/development-environment.md) owns local
  runtime versions and command execution.
- [tests/e2e/visual-baselines/README.md](../../../tests/e2e/visual-baselines/README.md)
  records the candidate-only visual-baseline review boundary.
