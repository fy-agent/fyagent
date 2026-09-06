# Round-six integration review

## Baseline, boundaries and child delivery

Baseline `6117a7d9` includes the backend merge and native startup correction.
The final round diff contains no `src-tauri` change: native DTOs, permissions,
Claude CLI installation, reversible configuration and activation semantics
remain at that baseline. Browser fixtures use synthetic identities, not the
account details in the user's screenshots. No push, release, deployment or
real-account operation is included.

| Requirements | Delivered owner           | Work commit               | Integration evidence                                                                                                                             |
| ------------ | ------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1, R2       | renderer-consolidation    | e7227e8a                  | One runtime entry; retired-only UI/deps and offline HTML removed; preserved domain/security tests; role-based tooling, SPEC and import checks.   |
| R3, R4       | frontend-layout-integrity | bcceba7e                  | Real labels paint above glass in both themes; empty/populated Prompts, minimum usable widths, resize/keyboard constraints and editor continuity. |
| R5, R6       | frontend-blue-themes      | 081d5d49                  | Two complete blue palettes; pre-content preference, right-hand non-drag control, browser-owned radial transition and bounded interruption.       |
| R7, R8       | frontend-state-motion     | 070479bf                  | Same-session size and content handoff, upstream press-isolation correction, real short presses, stable revisits and constrained shared collapse. |
| R9           | All four children         | See above                 | Root-cause regressions, no native authority changes, explicit retained limitations and no lowered gates.                                         |
| R10          | This parent               | Recorded in task metadata | Combined checks, archive references/ancestry, journal and final clean checkout verification.                                                     |

All four children are completed and archived. Their focused review documents
retain design tradeoffs, corrected failures and exact evidence. The child
work commits include corresponding SPEC updates; the parent does not replace
those contracts with another implementation.

## Engineering and compatibility audit

The migration retirement/continuity map is
`docs/fyagent/development/renderer-migration.md`. The ordinary application
entry is `src/index.html`; no standalone testing HTML remains at the root.
Retired directories in the checkout contained only untracked Finder metadata
and empty folders; those inspected leftovers were removed. Genuine
`tests/fixtures/changePlanDtoContract.v2.json` remains a native protocol
fixture, not a frontend version fork. Historical commits/task prose and
persisted provider IDs are not rewritten.

Current source, scripts, mise commands and SPEC have no renderer-generation
imports or old collapsed-lens API. Raw button hosts remaining in production
either define the shared semantic wrapper or already use its explicit visual
press hook. Radix remains the sole modal/focus owner. Motion owns presses and
collapses; the existing platform boundary owns theme IPC. No second solver,
drag implementation, screenshot engine or credential copy was added.

Effective context audit examined 276 JSONL manifests: zero newly broken and
zero active broken references. There are 168 pre-existing unresolved historical
references outside this round; this work does not claim to repair those.
All 166 frontend SPEC relative links resolve. This round's five task manifests
are separately validated before and after final relocation. The consolidation
manifest now injects the focused renderer structure/quality owners rather than
silently truncating a 49KB general backend task-runner spec at the 32KB cap;
the complete general contract remains unchanged and discoverable.

Final aggregate inspection also exposed jsdom's unimplemented `window.scrollTo`
when Motion restores scroll after auto-height measurement. The renderer unit
setup now supplies an explicit call-recording test double and restores the
original at teardown. It does not emulate layout, alter product code, disable
animations or filter console diagnostics. Real scroll/focus/anchor behavior
remains in the browser suites. jsdom documents this no-layout boundary in its
official README: https://github.com/jsdom/jsdom#unimplemented-parts-of-the-web-platform.

## Combined verification

After the last source correction, complete state-task prearchive returned 0:
174 unit files, 1,550 passed and 1 existing skip; Rust fmt/check/Clippy and
3,495 tests passed, 0 failed, 6 existing ignored; desktop fake IPC 7 passed;
release contracts 611 passed and 1 existing skip; native-fetch 4 passed.
Unified type/lint/format, architecture, task, asset and release checks passed.

The latest full browser command separately passed production boot/timing
2 tests and all 351 behavioral cases, using four Chromium sizes and selected
WebKit key paths. Both themes, actual glyph paint, narrow containers, dirty
drafts, focus/cancel, resize/reversal and unsupported-API fallbacks are covered.
The three consecutive final production-performance runs each passed 10 tests.
No compiler or parallel browser gate ran during those serial measurements.

| Final performance, milliseconds                          | Observed across three runs |
| -------------------------------------------------------- | -------------------------- |
| Navigation revisit p95, ordinary CPU                     | 27.9-28.2                  |
| Navigation revisit p95, 4x CPU cost                      | 48.4-50.4                  |
| Modal open/close warm frame p95, ordinary / 4x           | 33.4 / 33.4                |
| Same-session step-size warm frame p95, ordinary / 4x     | 33.4 / 49.9-50.0           |
| Theme preparation p95, ordinary / 4x                     | 17.3-34.8 / 33.4-34.9      |
| Theme reveal frame p95 for each third, both CPU settings | 16.7-16.8                  |

Each navigation condition measures 42 revisits; each motion condition measures
one cold and 20 warm cycles at 1232x700. Normal 100ms navigation and 33.4ms
warm-frame targets are unchanged. Normal step-size maxima reach 50.1ms and
4x step-size p95 remains near 50ms; real unscaled text layout still costs work.
Cold 4x navigation includes 95-101ms long tasks and modal samples include
51-52ms long tasks. These are not all-frame 60fps or universal native-GPU claims.

Final logs are `/tmp/fyagent-round6-state-final-prearchive.log`,
`/tmp/fyagent-round6-reviewed-browser.log`, and
`/tmp/fyagent-round6-reviewed-performance-{1,2,3}.log`. The definitive parent
prearchive and post-archive contract results are recorded at closure below.

## Residual limits and rollback

Minimum-version Windows/macOS native WebViews, physical GPU composition,
real credential flows, signing and native release acceptance were not executed.
Browser fixtures and finite contrast samples cannot substitute for them.
Existing Rust ignored tests and the one unit skip retain their prior reasons.
No claim of whole-repository debt/security/accessibility certification is made.

The four local work commits separate layout, migration, themes and motion.
Any rollback must be reviewed in reverse dependency order and keep a single
renderer, native authority and persisted identities consistent; do not restore
only a versioned directory or mix old configs with new runtime paths.

## Closure

The final complete parent prearchive returned exit 0 after the test-environment
boundary correction (`/tmp/fyagent-round6-parent-final-prearchive.log`). The
affected model/sidebar unit cases also passed 42 tests without the jsdom
scroll diagnostic. Product source and lock are unchanged from the final
351-case browser and three production-performance runs. All five focused
task context validations now pass without the oversized-injection warning.

Parent complete prearchive and final archival bookkeeping are required before
delivery. Work commit metadata, task parent/child reciprocity, effective JSONL
references, the no-exclusion contract gate, journal and clean worktree are
verified after relocation; none is inferred from the preceding child checks.
