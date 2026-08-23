# FyAgent 0.4.2 full-page UAT report

## Decision

**NO-GO** for the installed FyAgent 0.4.2 macOS build.

Two release-blocking P1 defects remain:

1. A Prompts create/import operation can clear a real live prompt file when the application has no enabled prompt record.
2. Daily Memory becomes entirely unusable when its real directory contains any non-date Markdown file.

No P0 was found. The absence of a P0 does not offset the two demonstrated P1 risks. This task changed no product code, executed no install/update, performed no destructive prompt/memory write, and does not claim Windows acceptance or `pixel_diff` evidence.

## Scope and immutable baselines

| Dimension | Recorded baseline |
|---|---|
| Platform claim | `platform=macOS` only |
| Host | Apple Silicon, macOS 26.5.1 (25F80), `arm64` |
| Runtime UAT object | Installed `/Applications/FyAgent.app`, version/build 0.4.2, bundle id `com.fyagent.desktop` |
| Installed provenance | Universal `x86_64/arm64`; Developer ID Application William Wang (HY446996QX); Gatekeeper accepted as `Notarized Developer ID` |
| Runtime state | Installed executable launched and running throughout the page traversal |
| Installed source reference | v0.4.2 tag peeled source `d8ab2a2228389fe41ff5c815ddccb3b5823bcaab` |
| Repository context | `origin/main` `e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`; contextual product baseline, not the UAT object |
| Window sizes | Normal 1232×700; enforced minimum 1152×640 |
| Privacy | Private local profile backup verified before risk-bearing interactions; raw/restricted screenshots and private fingerprints excluded from Git |

The installed app and repository baseline are separate facts. `origin/main` is 62 commits after the installed v0.4.2 source reference; source parity was checked only for the specific defect paths cited below. Nothing in this report treats the latest source checkout as proof that the installed binary behaved differently.

Evidence index: [evidence-index.md](evidence-index.md).

## Evidence model

- `code_audit`: source-level explanation or contract evidence; never substitutes for runtime success.
- `runtime_screenshot`: stable running-app visual evidence.
- `interaction_readback`: the UI or safe local authority was reread after an interaction.
- `UAT`: end-user workflow exercised in the installed app with a stated result.
- `pixel_diff`: not performed; no 1:1 visual claim is made.

Functional depth is stated as `C` (control clicked), `R` (request observed), `P` (persistence observed), or `A` (authoritative readback). A click alone is not reported as a saved change.

## Runtime-derived surface inventory and coverage

Coverage status means only whether the stated UAT case was exercised: `COVERED`, `PARTIAL`, or `FAILED`. It is not a release verdict.

| Surface | Actual runtime entries and states | Coverage | Evidence |
|---|---|---:|---|
| Global shell | Six primary navigation items; Search, Settings, Account; normal/minimum window | COVERED | SHL-001/002, BSL-006 |
| Agent | QoderWork, TRAE, WorkBuddy, Grok, Codex, Claude, OpenCode; detected/undetected/version/update/unsupported states; direct links to Models, Skills, MCP | PARTIAL | AGT-001–004, RDB-004 |
| Models | The same seven Agent targets; official-unsupported, empty, configured, pending draft, invalid URL, populated model-ID, minimum-window scroll states | PARTIAL | MOD-001–004, RDB-005 |
| Skills / Installed | 116 visible installed records; search/no-result; detail; seven Agent assignment switches; update check; More menu | PARTIAL | SKL-001/003/005 |
| Skills / Discover | Catalog, category filter, search/no-result, detail, install confirmation, target/path preview, minimum window | COVERED to confirmation; write canceled | SKL-002–005, RDB-006 |
| Skills / More | Import local Skill, From ZIP, Backup Restore, Skill Settings; sync-mode and migration settings dialog | PARTIAL | UAT interaction readback; no persistent setting/import action |
| MCP / Installed | 4 visible records; search/no-result; details; seven Agent assignment switches; Add quick form; Add JSON editor; delete confirmation | COVERED to validation/confirmation; writes canceled | MCP-001/002/004, RDB-001/006 |
| MCP / Discover | Catalog; configured install dialog; API-key validation; seven targets; minimum-window target overflow | COVERED to validation; install canceled | MCP-003/005/006 |
| Prompts | Grok, Codex, Claude, OpenCode, Gemini, OpenClaw, Hermes; list/search/no-result/empty/editor/new/discard/guarded-delete; normal/minimum window | PARTIAL; risky create/import/enable write intentionally not executed | PRM-001–006, RDB-002, CDA-001 |
| Memory / Long-term | OpenClaw Memory, OpenClaw User, Hermes Memory, Hermes User; read-only selection, unsaved edit, cancel/confirm discard | COVERED without persistent write | RDB-003; restricted screenshots omitted |
| Memory / Daily | Daily tab, loading/error, Retry, minimum-window state | FAILED | MEM-001–003, RDB-007, CDA-002 |

Agent web links, real provider connections, application update, real Skill/MCP installs/imports, and persistent configuration writes were not treated as passed; their exact boundaries are listed later.

## Visual experience review

The content-page mean is **7.2/10**. Scores describe visual/interaction presentation only; they do not override functional failures.

| Page | Score | First impression and hierarchy | Layout, type, spacing, color, density, readability | Discoverability, states, resize, consistency |
|---|---:|---|---|---|
| Agent | **8.1** | The product purpose is immediately legible: Agent rail first, selected Agent status and actions second. Version/status information is well prioritized. | Calm neutral surface and restrained accent color work well; card spacing and line lengths are readable without feeling empty. Type hierarchy is consistent with the shell. | Detected, unsupported, installed, and update-available states are distinguishable. Related-page actions are easy to find. At 1152×640, labels wrap without hiding the seven targets. |
| Models | **7.0** | Target-first navigation is clear and provider state is quickly scannable, but large empty regions weaken information economy on unsupported/empty targets. | Form labels and grouped controls are readable, with consistent colors and spacing. Configured targets become dense, and the long scrolled form competes with the sticky heading. | Invalid URL messaging is specific and actions are discoverable. At minimum size, scrolled content visibly runs beneath the sticky header (FYG-UAT-004); a cleared invalid field can retain error styling until route refresh (FYG-UAT-008). |
| Skills | **7.2** | Installed/Discover separation is obvious and the three-pane inventory supports scanning. The catalog feels feature-rich but visually busier than Agent/Models. | Cards, chips, filters, and details use consistent primitives; mixed-language/third-party catalog text and dense detail copy reduce rhythm and readability. | Empty/no-result and confirmation states are clear. Minimum size remains operable, though filter/search controls become crowded and the search field exposes two competing clear controls (FYG-UAT-007). |
| MCP | **7.4** | Installed/Discover and the privacy/risk messaging give good first-pass orientation. Add/delete/config actions have clear intent. | List-detail balance is strong and destructive emphasis is appropriate. Quick/JSON forms are readable, but configuration dialogs become vertically dense. | Validation is concrete and cancel paths are obvious. At minimum size the final OpenCode target is initially clipped without a visible scroll cue; keyboard focus can reveal it (FYG-UAT-005). |
| Prompts | **7.1** | App rail, prompt list, and editor form a coherent three-pane mental model. Empty apps are easy to understand. | Typography and editor spacing are comfortable at normal size; dense real content is necessarily prominent and requires privacy-conscious capture. Search/list controls match Skills/MCP. | Unsaved-change confirmation is excellent and enabled-delete guard is explicit. At minimum size the last app is below the initial rail viewport with weak scroll affordance (FYG-UAT-006); duplicate clear controls recur (FYG-UAT-007). |
| Memory | **6.2** | Long-term sources are clearly named and the editor is understandable, but the Daily tab's hard failure dominates the experience. | Long-term content is readable with consistent editor styling. The Daily failure leaves a very large empty canvas and provides only generic error copy. | Discard behavior is clear and safe. Retry is discoverable but repeats the same failure; no usable empty/list state can be reached on the actual profile. Minimum-size rendering remains legible, but functionally empty. |
| Global shell | **7.0** | Primary navigation is clean and stable; the six-page information architecture is easy to learn. | Icon size, selected state, spacing, and color are consistent across all pages. | Search, Settings, and Account look actionable but produce no visible action because they are wired to `noop` (FYG-UAT-003), materially reducing trust and feature discoverability. |

### Page-specific visual notes

#### Agent — 8.1/10

- First glance answers “which Agent?” and “what state/version?” with little effort. Status badges and the primary action are visually separated from secondary navigation.
- Hierarchy remains coherent across all seven targets, including official-unsupported and undetected cases. Color is semantic without becoming decorative.
- Density is the best balanced of the six pages. Minimum width produces controlled wrapping instead of horizontal clipping, and the page remains consistent with the Models target rail.
- The score is below 9 because provider-specific action sets vary enough to make cross-target comparison slower, and Update was visible but not safely executed in this install-preserving UAT.

#### Models — 7.0/10

- Labels, connection grouping, and provider context are clear, but empty/unsupported targets leave oversized blank areas while populated OpenCode/Codex views are comparatively dense.
- The neutral palette and typography are readable; pending and invalid states are visible. The invalid message itself is precise.
- At the minimum window, scrolling a long provider panel places form labels beneath the translucent/sticky heading (MOD-004). This degrades readability and creates ambiguity about which section owns the fields.
- Cleared invalid input can retain error styling until route refresh. It is recoverable but visually suggests a still-invalid state.

#### Skills — 7.2/10

- Installed and Discover answer different user intents cleanly. The list-detail split supports selection and comparison, and empty search feedback is obvious.
- Catalog metadata, chips, descriptions, and mixed-language third-party text create uneven density. This is legible but less calm than the Agent page.
- Confirmation dialogs communicate target and destination before write. At minimum width they remain readable and cancellable.
- When a search value exists, the native search-clear affordance and the app's own X appear together. Competing controls add avoidable visual noise.

#### MCP — 7.4/10

- Privacy copy and explicit Add/Delete/Install semantics make risk-bearing actions more understandable than in many configuration tools.
- Quick form and JSON mode share a coherent structure; validation messages are specific. The installed list and detail panel maintain good alignment.
- The Discover configuration dialog does not fully expose the seventh target at minimum height. Only keyboard focus reveals OpenCode, and there is no persistent scrollbar or fade cue.
- Color and danger treatments are consistent with Prompts; the minimum-window overflow prevents a higher score.

#### Prompts — 7.1/10

- The three-pane model is immediately understandable. New, edit, empty, search, and enabled states remain visually distinct.
- Unsaved-change confirmation cleanly distinguishes Cancel from Discard. The enabled-delete guard explains the required state transition instead of silently ignoring the action.
- At 1152×640, Hermes is not initially visible in the app rail. Wheel scrolling reaches it, but the rail lacks a strong cue and its heading can scroll away.
- Real prompt text is intentionally visible in the editor; this is usable but makes screenshot/export privacy a product-operation concern. Restricted captures were therefore excluded.

#### Memory — 6.2/10

- Long-term source labels and editor controls are consistent with Prompts, and discard protection is understandable.
- Daily Memory never reaches a useful list/empty state on the real profile. The centered generic failure message is readable, but the surrounding empty canvas conveys little diagnosis or recovery context.
- Retry is visible, yet the repeated identical result with no actionable detail makes the page feel stalled rather than recoverable.
- The minimum window keeps the error legible, so the low score is driven by state quality and utility, not raw typography.

## Functional UAT matrix

| Surface/case | Expected | Highest observed layer | Result | Evidence |
|---|---|---:|---:|---|
| Shell: six primary routes | Route and selected state update | C | PASS | Runtime traversal + interaction readback |
| Shell: Search / Settings / Account | Open a visible search/settings/account surface or clearly show unavailable state | C | FAIL | SHL-001/002, CDA-003 |
| Agent: traverse seven targets | Correct target-specific state appears | C | PASS | AGT-001–003 + interaction readback |
| Agent: Refresh version state | Action returns and state is reread | C | PASS at UI layer; request not independently instrumented | AGT-002, RDB-004 |
| Agent: direct Models/Skills/MCP links | Destination route opens with correct context where applicable | C | PASS | UAT route readback |
| Agent: Update | Preserve current installed build in an install-excluded UAT | — | NOT TESTED by design | Visible available-version state only |
| Models: seven target states | Unsupported, empty, configured, and populated states are accurate/readable | C | PASS | MOD-001/002 + interaction readback |
| Models: blank/invalid URL | Prevent test/save path and explain constraint | C | PASS | MOD-003, RDB-005 |
| Models: unsaved draft across routes | Preserve draft according to current contract; allow safe clear | C | PASS | RDB-005 |
| Models: Save/Apply/real connection | Persist only with real provider authority and credential safety | — | NOT TESTED by design | No credential or config write used |
| Skills: search/category/detail/no result | Filter and selected state update | C | PASS | SKL-002/003 + interaction readback |
| Skills: install confirmation then Cancel | Show target/path before write; Cancel leaves no install | C | PASS at confirmation layer | SKL-004, RDB-006 |
| Skills: update check | Accept click without unsafe write | C | PARTIAL | No durable completion/readback was exposed |
| Skills: assignment/import/settings write | Avoid modifying real Agent tool state | — | NOT TESTED by design | Dialog/menu inventory only |
| MCP: installed search/no result | Filter and restore safely | C | PASS | MCP-001/002 + interaction readback |
| MCP: blank Quick add | Block missing id/name/command | C | PASS | Runtime validation |
| MCP: invalid JSON add | Block missing identity and malformed JSON | C | PASS | MCP-004 |
| MCP: Delete then Cancel | Confirmation appears; database remains unchanged | A | PASS | RDB-001/006 |
| MCP: Discover config with blank key | Block install and identify required API key | C | PASS | MCP-003/005 |
| MCP: any safe negative/cancel flow | Installed registry remains unchanged | A | PASS | RDB-001 |
| Prompts: seven app rails and empty states | Each app shows its own list/empty state | C | PASS | PRM-001, runtime traversal |
| Prompts: isolated unsaved draft, switch, Cancel | Cancel retains draft and current app | C | PASS | PRM-002/003 |
| Prompts: confirm Discard | Switch app; no isolated record persisted | A | PASS | PRM-004, RDB-002 |
| Prompts: delete enabled item | Block delete until disabled | C | PASS | Restricted runtime readback |
| Prompts: create/import disabled item | Preserve live file until explicit enable operation | — | FAIL by code contract / unsafe to execute on live profile | CDA-001, FYG-UAT-001 |
| Memory: four long-term sources | Each source opens without exposing content in deliverables | C | PASS | Restricted interaction readback; screenshots omitted |
| Memory: isolated unsaved edit, Cancel/Discard | Cancel retains draft; Discard restores source; file unchanged | A | PASS | RDB-003 |
| Memory: Daily initial load | List valid daily entries or tolerate unrelated Markdown | C | FAIL | MEM-001, CDA-002; UAT evidence |
| Memory: Daily Retry | Recover or provide new actionable state | C | FAIL | MEM-002, RDB-007; UAT evidence |
| All pages: minimum window | Remain readable, navigable, and expose all actions | C | PARTIAL | AGT-004, MOD-004, SKL-005, MCP-005/006, PRM-005/006, MEM-003; UAT evidence |

No positive `R` or `P` is claimed: transport calls were not independently instrumented, and all environment-mutating writes were stopped before persistence. `A` is used only where a safe database or file authority was reread after the negative/cancel workflow.

## Findings

### FYG-UAT-001 — P1 — Prompts create/import can clear the live prompt file

- **Page/entry:** Prompts → New Prompt Save; Prompts → Import from file.
- **Safe reproduction contract:** Use only a copied/isolated profile with a non-empty live prompt file and zero enabled database prompts. Save a new prompt (created disabled), or import the live file. Compare the live-file fingerprint before and after.
- **Expected:** Creating/importing a disabled library item changes the prompt library only. The live file must remain unchanged until an explicit enable/disable action.
- **Actual:** The UI creates new prompts with `enabled=false`. Backend upsert saves the record, sees no enabled prompt, and writes an empty live file. Import constructs a disabled prompt and invokes the same upsert path.
- **Runtime safety decision:** Not executed against the user's real profile because current state satisfies a destructive precondition. This is a `code_audit` finding, not a claimed live destructive UAT.
- **Evidence:** CDA-001; [`Page.tsx`](../../../src/v2/pages/prompts/Page.tsx#L367), [`prompt.rs`](../../../src-tauri/src/services/prompt.rs#L28), [`prompt.rs`](../../../src-tauri/src/services/prompt.rs#L146). The installed v0.4.2 source and current baseline share this implementation.
- **Impact:** Silent loss of real Agent instructions/configuration on an operation whose UI explicitly says it will not auto-enable.
- **Suggested owner:** Backend / Prompts domain, with frontend contract review.
- **Release blocking:** **Yes**.

### FYG-UAT-002 — P1 — One non-date Markdown makes Daily Memory entirely unusable

- **Page/entry:** Memory → Daily.
- **Reproduction:** In a safe test directory, place at least one valid `YYYY-MM-DD.md` file and one other `.md` file; open Daily Memory; click Retry.
- **Expected:** The backend filters non-daily Markdown, or the frontend tolerates/skips invalid entries while presenting valid daily items and a bounded warning.
- **Actual:** The installed app shows “无法加载每日记忆”; Retry returns the same failure. The live directory contained 94 Markdown files: 79 date-shaped and 15 non-date-shaped; no private filename/body was recorded. Backend returns all Markdown, while frontend rejects the complete array when any filename fails its date-only parser.
- **Evidence:** MEM-001/002, RDB-007, CDA-002; [`workspace.rs`](../../../src-tauri/src/commands/workspace.rs#L59), [`content.ts`](../../../src/v2/shared/platform/tauri/feature-ports/content.ts#L173).
- **Impact:** Complete loss of Daily Memory browsing/search utility for a realistic directory shape.
- **Suggested owner:** Backend / Workspace plus frontend Tauri adapter.
- **Release blocking:** **Yes**.

### FYG-UAT-003 — P2 — Visible Search, Settings, and Account buttons are inert

- **Page/entry:** Global shell top tool cluster.
- **Reproduction:** Click Search, Settings, and Account on any page; reread the window.
- **Expected:** Open the named surface, or expose an explicit disabled/coming-soon state that does not imply a working action.
- **Actual:** Each button accepts the click and produces no visible state. Source binds all three to `noop`.
- **Evidence:** SHL-001/002, CDA-003; [`ToolCluster.tsx`](../../../src/v2/widgets/app-shell/ToolCluster.tsx#L7).
- **Impact:** Users cannot distinguish broken controls from unavailable features; Settings is especially trust-sensitive.
- **Suggested owner:** Frontend / App shell + Product.
- **Release blocking:** No for this report, but should be resolved or visibly disabled before broad release.

### FYG-UAT-004 — P2 — Models scrolled content overlaps the sticky header at minimum size

- **Page/entry:** Models → OpenCode (also structurally applicable to long provider forms), 1152×640, scroll down.
- **Reproduction:** Resize to the enforced minimum; open a populated long provider; scroll the main form.
- **Expected:** Header remains opaque or content starts below it with no text collision.
- **Actual:** Field text/labels render beneath the translucent/sticky header and reduce section readability.
- **Evidence:** MOD-004.
- **Impact:** Ambiguous form context and reduced readability on supported small-window use.
- **Suggested owner:** Frontend / Models responsive layout.
- **Release blocking:** No.

### FYG-UAT-005 — P2 — MCP config-install dialog hides the last target at minimum size

- **Page/entry:** MCP → Discover → configured install dialog, 1152×640.
- **Reproduction:** Open the dialog at minimum size and inspect the full target list; then keyboard-tab to the last target.
- **Expected:** All seven targets are visible/reachable with an obvious scrollbar, fade, or bounded scrolling region.
- **Actual:** OpenCode is initially clipped and there is no persistent visual scroll cue; keyboard focus scrolls it into view.
- **Evidence:** MCP-005/006.
- **Impact:** Mouse/trackpad users can miss a valid installation target and assume only six are supported.
- **Suggested owner:** Frontend / MCP responsive dialogs.
- **Release blocking:** No.

### FYG-UAT-006 — P3 — Prompts app rail hides the last app with weak scroll affordance

- **Page/entry:** Prompts at 1152×640.
- **Reproduction:** Resize to minimum height and inspect the app rail; scroll within the rail.
- **Expected:** All apps are visible or the scrollable boundary is visually signaled while its heading remains stable.
- **Actual:** Hermes is below the initial viewport; wheel scrolling reveals it, while the heading/top border can scroll away and no persistent cue advertises more items.
- **Evidence:** PRM-005/006.
- **Impact:** Reduced discovery of the last integration.
- **Suggested owner:** Frontend / Prompts responsive layout.
- **Release blocking:** No.

### FYG-UAT-007 — P3 — Search fields expose duplicate clear controls

- **Page/entry:** Skills, MCP, Prompts search fields.
- **Reproduction:** Enter any non-empty query.
- **Expected:** One consistently placed clear action.
- **Actual:** The platform search affordance and a custom X are both visible.
- **Evidence:** SKL-003 and equivalent runtime states on MCP/Prompts.
- **Impact:** Small visual clutter and uncertain control ownership.
- **Suggested owner:** Frontend / shared search primitive.
- **Release blocking:** No.

### FYG-UAT-008 — P3 — Cleared Models validation can retain error styling

- **Page/entry:** Models → Grok connection URL.
- **Reproduction:** Enter an invalid URL, trigger Test Connection, clear the field, and remain on the route.
- **Expected:** Validation styling returns to neutral when the invalid value is removed, or the message clearly states the required empty state.
- **Actual:** Error styling can persist until a route refresh, even though the invalid text has been cleared.
- **Evidence:** MOD-003 plus interaction reread.
- **Impact:** The form appears invalid after the offending value is gone.
- **Suggested owner:** Frontend / Models form state.
- **Release blocking:** No.

### FYG-UAT-009 — P3 — Daily Memory hides the actionable parse cause

- **Page/entry:** Memory → Daily failure state.
- **Reproduction:** Trigger FYG-UAT-002.
- **Expected:** Present a privacy-safe actionable explanation or a bounded “skipped invalid files” warning.
- **Actual:** Only a generic load failure and Retry are shown; Retry repeats without new diagnostic information.
- **Evidence:** MEM-001/002, CDA-002.
- **Impact:** Users cannot self-correct a filename-shape conflict or report a precise failure.
- **Suggested owner:** Frontend / Memory error presentation.
- **Release blocking:** No independently; secondary to FYG-UAT-002.

## Issue summary

| Severity | Count | IDs | Release blocking |
|---:|---:|---|---:|
| P0 | 0 | — | — |
| P1 | 2 | FYG-UAT-001, FYG-UAT-002 | Yes, both |
| P2 | 3 | FYG-UAT-003, FYG-UAT-004, FYG-UAT-005 | No |
| P3 | 4 | FYG-UAT-006–009 | No |

## Untested and intentionally bounded areas

- **Update/install:** The installed build was neither reinstalled nor updated. Agent Update, real Skill install, real MCP install, import, migration, backup/restore, and assignment toggles could mutate the environment and were stopped at confirmation/validation.
- **Provider/network:** No real API key, token, paid provider, model connection, or external service request was used. Network success and provider-side persistence remain unverified.
- **External links:** Official website buttons were inventoried but not opened as functional acceptance.
- **Prompts/Memory writes:** No real prompt enable/disable/create/import or long-term Memory save was executed. Safe draft discard and authoritative no-change readback were covered. The Prompts P1 must be reproduced only in a copied profile.
- **Forced write failure:** File permissions/processes were not deliberately damaged on the user's real profile. Validation, cancel, guarded-delete, no-change readback, and naturally occurring Daily load failure were covered; low-level write-denied rollback remains untested.
- **Native unavailable:** The installed native bridge was available. Browser-only/native-unavailable behavior was not upgraded from repository test context to installed-app UAT.
- **Strict visual parity:** No design baseline or `pixel_diff` existed; all visual results are runtime review, not 1:1 acceptance.
- **Windows:** AIMaster was not contacted and no Windows behavior was tested in this task.

## Repair and macOS retest plan

1. **P1 Prompts — first gate:** Separate library persistence from live-file activation. Creating/importing a disabled record must not touch the live file. Add an isolated regression test covering non-empty live file + zero enabled DB records + create/import; assert the live fingerprint is unchanged and database transaction/rollback behavior is deterministic.
2. **P1 Daily Memory — second gate:** Filter to valid calendar filenames in the backend and/or tolerate invalid entries in the frontend without rejecting the full list. Add mixed valid/non-date/invalid-calendar fixture coverage and expose a bounded skipped-entry warning.
3. Build a new signed/notarized macOS candidate from the fixes. Repeat both P1 cases on a copied profile, then repeat all six pages and authoritative prompt/memory/MCP fingerprints.
4. Resolve or explicitly disable the three inert shell controls; re-run navigation and keyboard accessibility.
5. Fix minimum-window Models/MCP/Prompts overflow, then retest at 1152×640 and normal size with keyboard and pointer paths.
6. Normalize search clearing and Models validation reset; repeat empty/error/cancel states.

The macOS verdict can move from **NO-GO** only after steps 1–3 pass on an installed candidate. Repository tests alone are insufficient.

## Windows reuse handoff (AIMaster)

### What the Windows experience reviewer may reuse

- The six-domain coverage matrix shape: Agent, Models, Skills, MCP, Prompts, Memory, plus every actual shell/settings/update/dialog surface discovered at runtime.
- The seven-part visual review: first impression, hierarchy, layout/type/spacing/color/density/readability, action/state discoverability, failure/empty/disabled states, resizing, and cross-page consistency.
- The functional evidence ladder (`C/R/P/A`), evidence grades, privacy rules, issue schema, severity scale, NO-GO decision rule, and report sections.
- The macOS P1 reproduction contracts as mandatory Windows regression probes after they are safe to run on a copied Windows profile.
- The rule that actual visible Agent tools must be inventoried rather than copied from docs. GrokBot, Codex, and any Windows-only or missing tools must each receive their own runtime row.

### What Windows must independently verify

1. **Installation and provenance:** Actual installed version/build, package source and exact artifact hash, Authenticode signer/chain, SmartScreen/reputation result where available, architecture, install scope, executable location (redacted in shared evidence), launchability, running process, updater, and uninstall/rollback boundary.
2. **Actual pages and tools:** Re-enumerate all first-level pages, secondary entries, dialogs, settings, install/update/detail/confirmation states, and detected Agent-tool set. Do not assume the macOS count of seven or the same versions/capabilities. Explicitly cover GrokBot and Codex if present.
3. **Windows visuals:** Native font rendering, title bar/window controls, 100%/125%/150% DPI, multiple display scaling, minimum/maximized window behavior, scrollbars, keyboard focus, clipping, contrast, and long localized strings.
4. **Functional paths:** Windows detection/version/update for every Agent; Models configuration and native adapter behavior; Skill/MCP install locations and assignment readback; prompt/memory persistence and rollback; repeat-click/cancel/invalid/empty/loading/error/disabled paths.
5. **Windows filesystem/security paths:** Drive and profile boundaries, path separators, long paths, case-insensitive collisions, CRLF/LF handling, junction/symlink behavior, file locking, ACL/UAC denial, antivirus/Defender quarantine, atomic replacement, rollback, and native-unavailable behavior.
6. **Prompt/Memory safety:** Back up the Windows FyAgent profile and every target file before writes; use isolated/copied content; verify pre/post authoritative fingerprints; never publish private bodies, tokens, or local user paths.
7. **Failure evidence:** Independently induce safe write-denied/native-unavailable paths in an isolated profile, verify error propagation and no partial persistence, and distinguish UI click/request/persistence/readback exactly as in this report.

### Non-extrapolation rule

**No macOS pass, score, page/tool count, signature result, data path, defect absence, or release verdict establishes a Windows pass.** The Windows Codex CLI reviewer must issue a separate `platform=Windows` evidence index, issue register, and GO/CONDITIONAL GO/NO-GO conclusion. Mac findings are regression inputs only.

## Validation

- Focused V2 suite: **PASS**, 8 files / 133 tests under the repository-locked toolchain.
- Rust substring-filtered `prompt` run: **PASS**, exit 0; 29 library tests and one additional name match passed, with no selected failure. The selected set does not contain the missing FYG-UAT-001 regression.
- These checks support source interpretation and artifact consistency. They do not erase installed-app failures or prove Windows behavior.

## Delivery

- UAT branch: `codex/installed-fyagent-uat-20260824`.
- Report commit: `e68aaf67`.
- Pull request: [#131](https://github.com/fy-agent/fyagent/pull/131), opened against `main` and left unmerged.
- Required governance: **赖永杰 must review and merge**. This UAT task must not merge, release, or modify `main` directly.

## Final release condition

Current macOS 0.4.2 status remains **NO-GO**. A new installed macOS candidate must close both P1s with isolated write/readback evidence and full-page regression. The Windows AIMaster run is a separate acceptance stream and cannot inherit this task's result.
