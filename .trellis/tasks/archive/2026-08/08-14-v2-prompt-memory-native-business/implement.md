# V2 Prompt and Memory native business integration — Implementation

## 1. Activate and protect scope

- [x] Confirm the pinned Prompt/Memory branch remains an ancestor and record no
      redundant merge commit.
- [x] Load the curated specs and start this Trellis task.
- [x] Preserve unrelated working-tree changes and exclude every other
      `codex/*` branch.

## 2. Add V2 native business ports

- [x] Add closed Prompt and Memory DTOs/unions and the two `FeaturePorts`
      interfaces.
- [x] Add query keys/hooks for per-app Prompt data, live Prompt files,
      long-term documents, Hermes limits, daily lists/search/content.
- [x] Implement Tauri input assertions, output parsers, exact document mapping,
      filename validation, and existing command invocations.
- [x] Implement native-only browser adapters with no seeded business data.
- [x] Extend adapter/ACL tests for success, malformed payload, invalid input,
      and exact command/payload behavior.

## 3. Replace the Prompts prototype

- [x] Rebuild Prompts with the shared feature header, toolbar, master/detail,
      panels, and primitives.
- [x] Implement per-app read/search/create/edit/import/enable/disable/delete and
      current-live-file display with a write lock and authoritative refresh.
- [x] Implement accessible dirty discard, delete confirmation, loading, empty,
      error, busy, disabled, native-only, and refresh-warning states.
- [x] Replace prototype tests with injected-port business and failure tests.

## 4. Replace the Memory prototype

- [x] Rebuild Memory with shared Long-term/Daily tabs, master/detail layout,
      panels, and primitives.
- [x] Implement the four fixed long-term resources, missing-file create-on-save,
      Hermes toggles/budgets, over-limit warning, and authoritative refresh.
- [x] Implement OpenClaw daily list/search/read/create/save/delete/open-directory
      behavior and every dirty-state transition.
- [x] Replace prototype tests with injected-port business and failure tests.

## 5. Remove stale prototype surface and update contracts

- [x] Remove prototype datasets, page-specific deep-blue styles, agent target
      configuration, and tests that only describe fake data/sync behavior.
- [x] Add only the minimal namespaced editor/responsive CSS needed beyond the
      shared V2 feature/control styles.
- [x] Update the V2 shell/index and add the durable Prompt/Memory native-business
      contract without changing unrelated specs.
- [x] Update browser and generated-preview tests to assert truthful native-only
      behavior and absence of private/static data.

## 6. Validate, smoke, and finish

- [x] Run focused adapter/page tests, then `mise run lint:v2`,
      `typecheck:v2`, `test:v2`, `test:v2:browser`, and `build:renderer`.
- [x] Regenerate the standalone twice and compare SHA-256 for determinism.
- [x] Run `mise run test:desktop:mock` and
      `mise run test:desktop:visual:preflight`.
- [x] Run a real `mise run dev` read-only Windows smoke without exposing or
      modifying user content; record any native limitation.
- [x] Run final `mise run check` on the resolved tree.
- [x] Dispatch the full-scope Trellis check, fix verified findings, update the
      durable spec if behavior changed, and rerun affected gates.
- [x] Review the final diff, commit as
      `feat(v2): connect prompts and memory management`, and do not push.

## Rollback Points

- Port/type changes must compile before either page replacement is accepted.
- Prompt and Memory page replacements remain independently reviewable, but the
  final commit includes both so the shell never lands with one stale prototype.
- No migration or automatic data write is permitted; a code revert is the
  complete software rollback.

## Completion Evidence

- Branch ancestry: `codex/prompt-memory-v2-main-pr` remains an ancestor of the
  resolved tree; no merge commit was added.
- V2 gates: lint and typecheck passed; `test:v2` passed 21 files with 163 tests
  passed and 1 skipped. The two Memory route-blocker tests retain Radix/jsdom
  asynchronous focus-cleanup `act(...)` warnings; their behavior assertions
  pass and the other 15 Memory tests do not emit that warning.
- Browser gate: 116/116 Playwright tests passed across 900x600, 1152x640,
  1232x700, and 1440x900 with zero page, console, or React errors.
- Standalone preview: two consecutive renderer builds produced SHA-256
  `8998BD7E5762188EB41D06730A4BC031FF9E2F5FB93E5E76603EC28D8FC0D34B`;
  browser coverage also proved that the file issues no external requests.
- Desktop gates: mock acceptance passed 7/7 and visual preflight reported
  `ready-for-candidate-capture` without writes.
- Native read-only smoke: `mise run dev` launched the Windows Tauri binary and
  a temporary boolean-only probe confirmed native authoritative reads on
  Prompts, Memory Long-term, and Memory Daily; seven applications, four fixed
  resources, shared feature/control layout, reachable editors/search, no
  horizontal overflow, and zero console/runtime errors. The probe was removed
  and `src/index.html` was verified unchanged afterward.
- Repository gate: `mise run check` passed after updating the sealed digest for
  the intentionally changed standalone generator test.
- Prearchive gate: `mise run check:prearchive -- --exclude-active-task
  .trellis/tasks/08-14-v2-prompt-memory-native-business` passed on the resolved
  tree. Rust tests were run with `RUST_TEST_THREADS=1` because an existing
  process-environment-isolation test was intermittent in the full suite; its
  focused rerun and the complete serialized Rust suite both passed without any
  production-code workaround.
- No real-profile Prompt/Memory write HIL was performed. Writes are covered by
  isolated Rust tests, exact invoke adapter tests, and injected stateful page
  ports so private configuration files remain untouched.
