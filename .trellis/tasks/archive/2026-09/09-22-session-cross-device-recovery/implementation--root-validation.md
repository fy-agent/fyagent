> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Root integration validation

2026-09-22, codex/session-cross-device-recovery-20260922.

- After frontend root guards (active target provider; verified-stage opening; unknown empty result wording): mise run typecheck, mise run lint passed.
- After compact-height navigation spacing fix: mise run test:browser -- tests/browser/shell.spec.ts tests/browser/session-migration.spec.ts passed; production boot 3/3 and four-viewport browser cases 36/36.
- Full browser suite started after that successful targeted rerun; result pending.
- Identity module: native agent canonical rust:test identity reported 84 passing selections; includes 19 module tests duplicated through old QA path-include harness. This does not mean 84 independent identity cases.
- Strict extractor production crate: native agent canonical rust:test session_manager::migrate::extract reported 45 passed.
- Native writer product+isolated bridge tests underway; backend orchestration round two underway.
- Windows real-machine UAT waived by user; no claimed Windows execution.

## Integration pass (2026-09-22)
- `mise run format:check`: passed.
- `mise run test:browser`: production boot 3/3 and 666/667 browser passed. One unrelated model-disclosure animation initial-height failure under concurrent unit load, needs isolated rerun (do not weaken).
- `mise run test`: 2098 passed, 19 failed, 1 skipped. New route/IPC inventory assertions and session CSS tokens need fixes; platform freeze inventory needs reviewed updates. Remaining failures are test watchdogs under concurrent build/browser load; rerun without competing suites.
- `mise run check:contracts:prearchive ...`: unavailable bare python in task launcher, delegated environment fix to contract worker; no global Python modification.
- Grok4.7 identity review: native ID missing/conflict must stop publication; batch selection mismatch rejected before DB; origin conflict keyed on originId; unsupported source mapping fails; unused digest index removed. Identity install-directory rename/race fixes delegated to original owner.

### Targeted resolution
- Previous timeout files reran without the competing full browser suite: 8 files passed; one newly added Sessions route marker was missing in router test map and then fixed.
- Session presentation + route/ACL/copy/design-token focused check after semantic UI fixes: 11 files, 74 tests passed.
- Typecheck, lint and frontend format check passed after pending/readback/reply presentation fix.
- Browser review: deterministic async data readiness counterexample explains old motion failure; corrected test waits for actual model ID body and keeps stronger actual-height/intermediate/final-zero assertions; 25 motion cases and production boot passed.
- Final fixture screenshots at900x600 and1440x900 passed, reviewed actualimages; this is renderer evidence only. NativeWritten is pending, nativeReadback is notmodelreply.
- Production Rust QA harness now imports real fyagent_lib testhooks; initial 10/11passed after enabling isolatedhooks. Remaining assertion incorrectly treated installedCLI absence as unknownsourceheader; adjusted to mutate sourceheader and preserve currentCLI/sourceversion separation, retest pending.

## Final UI follow-up

- Canonical `mise run test:unit tests/session-migration`: 7 files / 47 tests passed after scoping the status-heading assertion to its actual banner.
- Canonical browser Session + shell: production boot 3/3 and focused 36/36 passed after the final UI copy change.
- Native Hermes/Gemini receipt-write-failure assertions corrected to proven-no-side-effect before native publication; final Rust rerun pending.

## Final integration gates (current source)

- `mise run typecheck`, `mise run lint`, `mise run format:check`: all exit 0 after final UI text and heading-test fix.
- Final Grok 4.7 High review exited 0; P1 mismatch-stage and P2 empty-selection findings accepted and fixed, with production counted-writer regressions; see grok-final-resolution.md.
- Full frontend aggregate and Rust suite in progress; do not infer success from their launch.

### Final frontend and repository contracts

- `mise run test`: exit 0, 223 unit files / **2,134 passed, 1 pre-existing skipped**. Desktop mock 7/7 and visual manifest preflight also passed. Raw transcript: local work/executor-logs/final-frontend/test-final.log.
- `check:contracts:prearchive`: exit 0, 664 passed / 1 existing skipped; native-fetch 4/4, platform inventory 3,001 files. Exact CI ownership for nine research files resolved the prior sole aggregate failure.
- Current fetched origin/main remains c0b2ec21, same as implementation baseline.

### Final Rust closure

- Canonical final rust:fmt:check and rust:clippy exit 0; rust:check exit 0 on unchanged product source before the final assertion-only correction. Full final rust:test exit 0: 3,754 library + 273 integration + 82 helper = **4,109 passed, 0 failed, 7 ignored**. Production migration integration 11/11. Grok regressions both passed. Root read actual exit receipt and full summary directly.
- Final code is ready for commit/PR; remote CI is not yet claimed.
