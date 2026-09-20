# Integration checkpoint after 0.4.6 main

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
