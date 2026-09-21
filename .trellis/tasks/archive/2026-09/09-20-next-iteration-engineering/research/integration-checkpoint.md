# Integration checkpoint after 0.4.6 main

The earlier sections below are chronological checkpoints. The latest status is
recorded at the end; later source findings and accepted fixes supersede earlier
pending work or provisional acceptance.

Coordinator: GPT-6 in the current task. Integration branch:
`codex/next-iteration-engineering-20260920`. Checkpoint `9fd852cd` contains the
reviewed work packages; merge `35848bef` includes verified main `da91427e`.
The original dirty checkout is preserved. No Issue has been closed by this task.

Later integration commits: `a0a170b2` (Antigravity browser repairs), `2ad28ef8`
(reviewed integration fixture/ACL/classifier repairs), and `3df9c8df` (opened
official-source contrast coverage). Current native repair remains uncommitted.

## Verified and remaining checks

- Before main merge, typecheck, lint, Rust formatting and renderer production
  build passed. Build: 9 routes, 663885 initial JS bytes, unchanged 665600 limit.
- After main merge, platform scan found 119 candidates with no changed/removed
  identity. CLI integration will require a fresh scoped review.
- Full frontend gate: 2066 passed, 2 failed, 1 skipped. Root corrected demo
  classifier ownership and the cold lazy About import test. Both affected test
  files then passed, 54 tests. Full final gate remains pending.
- Full browser baseline: 615 passed, 30 failed / 645 in 10.2 minutes. Distinct
  failures: regional URL assertion, actual-vs-saved model heading, old duplicate
  save confirmation, model action geometry, source-link contrast, recommendation
  keyboard order, About keyboard inclusion, subscription selector ambiguity,
  and one press-feedback geometry shift. Antigravity owns the bounded repair and
  affected viewport matrix in an isolated branch; baseline artifacts are copied.
- Antigravity subsequently passed all 196 affected browser tests. Root reviewed
  the source and the actual passed runtime artifact. An extra acceptance gap
  was closed with direct light/dark **opened** source-disclosure coverage: 40
  blue-theme matrix checks passed and root visually inspected the actual images.
  Root then isolated screenshot output by browser project to avoid overwrite
  between parallel tests and reran the ten opened-state checks; all passed.
- Cursor's main integration retains Kimi route/old card overrides and universal
  child metadata/order/unknown fields/current selection while asserting public
  SecretRef redaction and native credential resolution. Focused tests: 1 + 2
  passed. No production sync change. Writer stopped before next assignment.
- Cursor repaired the two Clippy argument-count errors using a typed usage
  input. The next full backend gate passed fmt/check/Clippy and exposed seven
  library failures plus one provider integration failure (3538 + 35 tests
  passed in those two targets). Root repaired the test-only ACL inventory to
  include the already-enabled config-pack permission file; runtime permissions
  did not change. Cursor owns five credential/ChangePlan failures.
- Two recovery failures were stale tests expecting unproven foreign-file
  overwrite. Cursor changed only those tests to assert conflict and exact byte
  preservation, then passed both filters and 21 receipt-backed recovery tests.
  GPT-6 reviewed the delta and accepted the test contract. That writer stopped.
  Final combined backend evidence remains pending; root is not running native
  checks concurrently with Cursor's active credential repair.
- Cursor repaired the five remaining filters and passed them, the rotation
  filter and all 35 credential tests. Root rejected the newly introduced broad
  post-snapshot common-table copy after source review. Grok independently
  confirmed its overwrite/read-error/two-publication defects and identified a
  failed-create upsert compensation regression. Cursor now owns the bounded
  shared-projection/single-write repair plus that compensation case. Its prior
  test success is not acceptance of the superseded implementation.

## Active delivery boundaries

- Antigravity CLI commits `ce5c1f89` and `be21f653` are **not accepted/integrated yet**. Its first
  report acknowledges npm may resolve a newer ordinary dependency than the
  preview budget. Root additionally found Windows space probes use AppData
  parents rather than actual npm prefix/cache and macOS cache-read failure
  falls back to a guessed path. Grok's completed read-only review demonstrated
  a narrow exact-dependency argv direction in an empty-prefix npm fixture.
  Jev advised the guarded direction; GPT-6 selected it with additional required
  existing-install/global-conflict fixtures. Antigravity is repairing the
  dependency carry, helper protocol, actual volumes and unknown-budget handling.
  Root also found the visible Grok owner panel bypassed preview/job through the
  legacy lifecycle port; its narrow fix is included in the same worker scope.
  In be21f653 root found an always-empty version slice, malformed/read-failure
  admission, generic Windows preflight volumes/target despite real checks later
  at execution, Claude skipping the tool-specific plan guard, and an unbound
  dependency registry scope. The original Antigravity conversation now owns
  those concrete repairs under `ANTIGRAVITY_CLI_FINAL_REPAIR_TASK.md` in its tree.
- Antigravity UI conversation `fc6ad4d4-b8b7-492a-bc7b-e702798c35c2`, observed
  Gemini 3.8 Flash High, owns `codex/next-ui-regressions` and the released
  browser port/output window during its tests; it has now stopped and released
  that window. Original CLI conversation remains separate and active.
- Cursor Debug conversation retains its assigned Grok 4.6 / High / Fast
  configuration; current package is `cursor-common-projection-repair-task.md`.
- Terminal Grok, model `grok-4.6`, completed exec session 57721 with exit 0.
  Reviewed final findings are in `grok-cli-budget-review.md`; raw output stays
  ignored under artifacts. Its empty-prefix fixture does not prove updates or
  preservation of unrelated global dependencies.

Formal demo capture, final cross-package checks, PR/queue delivery and remote
Issue readback remain pending. #67 needs actual release/certificate ownership;
#68 needs real signed Windows artifacts and clean Windows acceptance. Neither
can be inferred from source or local fixture results.

## Latest handoff

Antigravity CLI stopped after an Insufficient AI Credits error before changing tracked files in its last attempt. Observed Gemini 3.8 Flash High quota refresh is 2026-09-21 10:33:46 local; no overage or credit purchase was enabled. The already-authorized Cursor backend role is now implementing the four remaining exact defects in the isolated CLI tree, preserving `ce5c1f89` and `be21f653`. Its durable contract is `CURSOR_CLI_REPAIR_TASK.md`.

Cursor common-projection repair completed and stopped. Root source review accepted the shared single-write projection, strict current-file read and TargetNotFound-only failed-create compensation fallback, removed the obsolete unused wrapper, and started the complete backend gate. UI results are preserved in `research/antigravity-ui-regressions-result.md`; root screenshot provenance adjustment is in `tests/browser/blue-themes.spec.ts`.

## Backend gate follow-up

The complete backend gate passed formatting, Cargo check and warning-denying Clippy. Its first aggregate test run exposed three newly added common-projection tests that changed process-wide fixture homes without joining the existing `serial_test::serial` lock; one prompt test was affected as a consequence. Root added the same lock used by adjacent fixture-home tests, without changing assertions or production behavior. The full canonical `mise run rust:test` then exited 0; exact counts are in `artifacts/integration-native-serial.log`. The contract gate passed platform/source, task, lock, version and most release checks but detected concrete workstation paths in the imported UI report and new workstream entry. Root normalized those report paths to semantic `~/` references and moved the UI report into this task; no code/runtime path changed. Final combined gate remains pending the CLI package.

## Accepted backend and demo checkpoint

`fac051ea` contains the accepted common-projection repair, typed usage input,
three fixture-home locks and the reviewed source-identity updates. The complete
native run at this checkpoint passed 3883 tests, with 6 intentional ignored
tests across 19 targets. This is local macOS evidence, not a signed release or
Windows execution claim.

`3bc8c2e8` adopts the current-flow demo captured from clean source `fac051ea`:
7 screenshots, 2 captioned WebM clips and raw originals, with source and asset
hashes under `docs/fyagent/development/demos/0.4.6-fac051ea/`. Automated Chromium
full playback/cue checks and full media decode passed; root visually reviewed
the screenshots and all seven cue midpoint frames. The separate in-app browser
preview crashed on playback and is explicitly not accepted as playback evidence.

The subsequent full prearchive run passed frontend type/lint/format checks,
2068 frontend tests (1 skipped), desktop mock/visual preflight, Rust formatting,
Cargo check and Clippy. One native OpenCode confirmation test exposed a separate
process-wide fixture-home race. Cursor Debug reproduced it and changed only that
test's home isolation and existing serial lock, preserving the production target
guard. Root accepted this in `11a154b5`; both focused OpenCode filters passed.
The final aggregate run will include this fix and the still-active CLI package.

## CLI review feedback during implementation

The active Cursor repair initially introduced a handwritten JSON reader that
accepted trailing garbage and malformed nested/number values, and bounded the
file only after a full allocation. Root returned exact counterexamples and the
required bounded read/strict parser correction to the same writer. This
intermediate code is not accepted. The writer also retains responsibility for
actual npm destination binding and tool-specific execution admission; root will
review the completed package before integration.

Root provisionally integrated the two existing Antigravity commits as `955a8bde`
and `b58163d6` while the independent Cursor writer continues. They are not yet
accepted: the final repair remains necessary. No seed commit was cherry-picked.
Renderer production build passed after these UI changes: 9 routes, 664049 initial
JS bytes and 46775 CSS bytes. The live GitHub readback still shows main
`da91427e`, required `CI / Required`, loose classic status checks and the active
`MERGE` queue; no policy setting was changed.

Root then returned a second concrete gap before worker validation: the first
repair discarded npm identity and retained only existing-directory ancestors
for cache/temp. This could admit different nonexistent sibling destinations,
and the helper merely reobserved a new execution target without comparison.
Required correction: retain exact native prefix/cache/temp/npm identity, exclude
volatile available bytes from equality, and carry/recheck that authenticated
bounded binding immediately before mutation for both npm tools. The worker owns
the narrow plan/codec/dispatch changes and corresponding drift tests.

After the Antigravity base integration, the root renderer checks passed: the
Grok port test (1), installation readiness/confirmation tests (16), and the
complete serial performance suite (36). Production navigation p95 was 43.5 ms
at normal CPU and 88.3 ms at the 4x CPU-cost setting; these are browser fixture
measurements under the repository definition, not native launch or OS latency.
Logs: `artifacts/cli-renderer-final.log`,
`artifacts/cli-renderer-components-final.log`, and
`artifacts/integration-performance-final.log`.

The adopted demo's 367 source identities were unchanged before this integration.
Afterward exactly four renderer files differ: the installation-confirmation npm
budget copy, the Grok-only owner panel, the corresponding budget parser enum,
and the blocked legacy Grok install port. Root reviewed those deltas; none
changes the seven recorded WorkBuddy/first-use/config-save states. The demo
continues to name its original clean capture commit and does not claim to show
the new CLI installer or its native behavior. No screenshot/video was silently
relabeled as a different source version.

Cursor's next coherent patch retained the four exact target strings in native
prepared state and the bounded helper control frame, and compared them before
execution. Root accepted that direction but found the actual invocation still
resolved bare `npm` on a fresh macOS install and let npm reread ambient
prefix/cache after comparison. The same writer now owns the final binding step:
execute the observed absolute program and pin the admitted native prefix/cache
with correct argument quoting, preserving the existing temp and tool/registry
checks. Focused invocation tests must prove the confirmed target cannot be
redirected by later PATH/config changes. Intermediate test results are retained,
but acceptance waits for this actual invocation path.

## Final bounded CLI metadata review

Terminal Grok completed a narrow immutable-source review of the dependency budget graph at `b58163d6`. GPT-6 accepted three concrete admission gaps: extra root optionals, dependency maps on platform packages, and ignored peer/optional child maps. The suggestion to trust arbitrary package-name prefixes was rejected. Root checked the current public vendor package shapes and sent the existing Cursor writer a closed recognized-package repair, malformed/extra-map regression tests, and an explicit report/commit/stop completion requirement. This review does not assert the illustrative extra dependencies exist in published packages.

The root runtime fixture used npm 11.17.0 with two loopback registries, synthetic packages, scripts disabled, and temporary user/prefix/cache directories. Without the explicit `@iarna` scope pin, the same root/global command fetched the dependency from the ambient scoped registry; with the pin, all requests used the closed registry and the installed package marker/version matched. Prefix/cache paths containing spaces and `.config` were used successfully. This proves npm scope resolution/argument behavior only, not product UI or Windows runtime. The earlier executor's draft fixture used synchronous subprocesses against an in-process HTTP server and was not treated as evidence. Safe result: `research/cli-npm-scope-runtime.json`.

## Final integration corrections

The CLI worker returned `1811093d`; root read back a clean tracked tree and the stopped Cursor UI before integrating it as `f324079a`. Root made the macOS permission fixture explicit, restricted its shared import on Windows production builds, applied Rust formatting, and moved the result into this task with semantic home paths. The three first aggregate frontend failures were a stale 160-command count after removal of the direct renderer invocation, a fixed-length source slice that truncated the now longer helper dispatch, and the staged old report path. Root updated the count to 159 with an explicit legacy-invoke absence assertion, bounded the security assertion by its actual owning function boundary, and staged the report move. All 12 focused tests passed afterward.

Source review additionally closed the legacy native IPC itself: keeping the renderer from invoking it did not by itself prevent a direct native-install bypass. The compatibility command now always directs the user through software-detail preflight; the confirmed `start_agent_action` path remains the execution authority. A focused native regression covers both tools and all four legacy actions.

The adopted WebM clips exposed an unsupported binary type in the repository scanner. Root extended the existing exact path/mode/SHA media inventory to the four reviewed adopted/raw clips, added finite EBML/WebM header/segment bounds and bounded non-frame metadata inspection, and retained full media decode/visual evidence as separate acceptance. New negative fixtures cover truncation, trailing bytes, wrong document type and unknown size. The 27-test scanner run had only a second stale inventory-count assertion, now corrected to 133. Final aggregate verification is still required.

Jev returned all three narrowly stated scanner/evidence claims as verified, with low confidence for two and no concrete counterexample. This is advisory rather than acceptance; GPT-6 requires the actual scanner regressions and aggregate gate and retains the explicitly unverified Windows native boundary.

The fourth aggregate reached 3908 native passes and one failure: the project read-only test compared timestamped SQL exports across a one-second boundary. Root decoded both assertion values; the generation-time header was the only difference. The test now excludes only that exact second header line and additionally asserts SQLite total_changes is unchanged. Database schema/data equality and the production snapshot remain intact. No retry or timing sleep masks the failure. The third aggregate helper failure was an outdated assertion that the maximum Error frame equals the newly larger absolute protocol bound; it now checks its exact 265-byte size and the bound separately, with a new maximum 1075-byte ToolResult round-trip and four individual overlength-field rejections.
