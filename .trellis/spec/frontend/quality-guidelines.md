# Quality Guidelines

`mise run typecheck`, `format:check`, and `test:unit` cover leftover renderer
and shared non-V2 tests. `vitest.config.ts` excludes `tests/v2/**` and
`tests/v2-browser/**`; V2 changes must use the gates in
[V2 Shell](./v2-shell.md). V2 copy is hardcoded Chinese and is not part of
the four-locale `t(...)` contract below. New UI must follow
[Frontend Reuse](./reuse.md): reuse existing shared owners; if a new
component will be used by another module, put it in `shared/` on the first
commit.

## Reproducible Core Frontend Checks

For an ordinary renderer change, start with the repository task API:

```bash
mise run typecheck
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

Vitest runs in `jsdom` and loads `tests/setupGlobals.ts` plus
`tests/setupTests.ts`. The shared setup installs Testing Library matchers,
initializes a minimal i18n instance, starts MSW, cleans up rendered trees, and
resets handlers/mocks after each test.

Component tests use React Testing Library (`render`, `screen`, events, and
role-based queries). Hook tests use `renderHook` and `act`. Tests that need
TanStack Query create a client with retries disabled so failures are immediate.

```tsx
// tests/utils/testQueryClient.ts
export const createTestQueryClient = () =>
  new QueryClient({ defaultOptions: { queries: { retry: false } } });
```

Test organization is primarily mirrored under `tests/components/`,
`tests/hooks/`, `tests/lib/`, `tests/config/`, and `tests/integration/`.
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

All ordinary Vitest, locale, and desktop contract package scripts launch Node
with the portable `--throw-deprecation` flag. The focused command adds the
pending gate:

```bash
mise run test:unit
mise run release:check
```

The focused pending probe is deliberately supplemental. Node 24.19.0 may not
surface a pending deprecation originating below every `node_modules` path, so
dependency proof is owned by `scripts/tasks/dep0040-check.mjs`: it parses the
manifest, active module specifiers, the versioned pnpm lock, and argv-based
`pnpm why --json` reverse paths. The obsolete chain is
`cross-fetch → node-fetch@2 → whatwg-url@5 → tr46@0.0.3`. Userland
`punycode@2.3.1` is permitted from only two reviewed reverse origins: the
existing jsdom chain through `whatwg-url@14` and `tr46@5`, or the exact
contiguous suffix `eslint@10.8.1 → ajv@6.15.0 → uri-js@4.4.1`. Wrappers
may precede ESLint, but no version drift or intermediate package is allowed,
and the why graph must explain that exact ancestor suffix. Lock/why
reconciliation separately proves the same versions for the watched URL and
punycode packages. Adding or upgrading either origin requires a new
reverse-path review; this is not a general allowance for every `punycode@2`
path.

The report fails closed on malformed active modules, non-canonical watched
lock entries, package/snapshot disagreement, unexplained aliases, and watched
reverse paths outside those two reviewed ancestries. Its suppression scan owns
the runnable package, workflow, mise, and script surfaces; statically composed
JavaScript arguments and shell/PowerShell script files are not escape hatches.
Negative detector fixtures belong in the contract test input, not in a scanned
execution script.

Never use `NODE_NO_WARNINGS`, `--no-warnings`, `--no-deprecation`,
`--disable-warning=DEP0040`, or stderr filtering to make these gates pass.

## Leftover UI Text and Accessible Primitives

When a leftover renderer change adds or changes user-visible text, use `t(...)`
and update the four locales registered by `src/i18n/index.ts`:

```text
src/i18n/locales/en.json
src/i18n/locales/ja.json
src/i18n/locales/zh.json
src/i18n/locales/zh-TW.json
```

Shared primitives already carry focus-visible styling and form ARIA linkage.
Preserve those properties when editing them, and test interactive behavior
through accessible roles where the nearby tests do so.

### Locale Schema Parity

`tests/config/localeKeyParity.test.ts` treats `zh.json` as the key-schema
baseline and requires `en.json`, `ja.json`, and `zh-TW.json` to have the exact
same leaf-key set. When adding, renaming, or deleting user-visible text:

- change all four locale files in the same patch;
- keep nested keys as objects and translation leaves as strings; and
- run `mise run test:i18n` (or the focused
  `mise run test:unit -- tests/config/localeKeyParity.test.ts`) before
  relying on fallback text.

Do not add a locale-specific key merely to silence a rendering issue. Fix the
shared key shape so a missing translation cannot become a production fallback.

## Evidence

- [package.json](../../../package.json) defines the runnable type-check,
  formatting, unit-test, locale, and desktop-acceptance scripts.
- [vitest.config.ts](../../../vitest.config.ts) configures the `jsdom`
  environment and shared setup files.
- [tests/setupTests.ts](../../../tests/setupTests.ts) manages Testing Library,
  i18n, MSW, cleanup, and mock reset lifecycle.
- [tests/config/localeKeyParity.test.ts](../../../tests/config/localeKeyParity.test.ts)
  enforces the registered locale key schema.
- [tests/msw/nativeFetchTauriMock.test.ts](../../../tests/msw/nativeFetchTauriMock.test.ts)
  exercises native Fetch, MSW, Tauri mock parsing, and cross-realm headers.
- [scripts/tasks/dep0040-check.mjs](../../../scripts/tasks/dep0040-check.mjs)
  owns the dependency graph and warning-suppression report.
- [Development Environment](../backend/development-environment.md) owns local
  runtime versions and command execution.
- [tests/e2e/visual-baselines/README.md](../../../tests/e2e/visual-baselines/README.md)
  records the candidate-only visual-baseline review boundary.
