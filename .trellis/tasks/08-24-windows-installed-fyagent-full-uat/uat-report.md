# Installed FyAgent 0.4.0 — Windows full-page UAT

## Verdict: NO-GO

This is an independent Windows-only verdict for the FyAgent 0.4.0 build
installed and running on AIMASTER. No macOS result, score, signing status, tool
inventory, or release decision was inherited.

The candidate is not ready for Windows release sign-off:

1. Daily Memory is a confirmed P1 whole-page failure and Retry fails again
   (WIN-UAT-001).
2. The installed executable, user helper, and uninstaller are all unsigned
   (WIN-UAT-002).
3. The formal installed build cannot be redirected to a disposable profile, so
   both mandatory isolated P1 retests remain NOT TESTED (WIN-UAT-003, AC7).
4. 125/150% DPI, stable minimize, and full keyboard focus evidence remain NOT
   TESTED after Windows Settings was not targetable and real user input was
   detected three times on the shared desktop (WIN-UAT-004, AC4/AC5).

The runtime traversal, private evidence, inventory, and issue documentation are
complete to the strongest level that was safe. Missing evidence is never
represented as PASS.

## Scope and evidence model

- System under test: installed/running FyAgent 0.4.0, machine scope.
- Windows: Windows 11 Pro 23H2 build 22635.5305, x64.
- Source baseline accepted for explanatory audit:
  d080a873850d0e073fac91add8c88df2a9f0a257.
- Execution branch: codex/windows-installed-fyagent-full-uat-20260824.
- GUI method: Computer Use via a persistent node_repl and @oai/sky; the
  resulting JPEG/JFIF captures stayed private/untracked.
- Evidence grades: runtime_screenshot, interaction_readback, UAT, and
  code_audit. No pixel_diff claim.
- C/R/P/A discipline: clicks are C; renderer changes remain
  interaction_readback; no request instrumentation means R is not claimed; no
  installed-profile writes means P is not claimed; Windows/Appx/filesystem
  authority is labeled A only where directly read.

The complete page scores, display matrix, C/R/P/A matrix, Windows failure paths,
and both isolated-hypothesis dispositions are in
[coverage-matrix.md](./coverage-matrix.md). Every evidence id resolves through
[evidence-index.md](./evidence-index.md).

## Baseline and provenance

The standalone machine receipt is
[machine-acceptance-receipt.md](./machine-acceptance-receipt.md).

| Boundary | Windows observation |
| --- | --- |
| Install source/scope | Machine-wide FyAgent uninstall record, install directory under %ProgramFiles%\FyAgent. No reinstall or upgrade was performed. |
| Launch state | Running/responding as PID 70360 at acceptance; one com.fyagent.desktop window was exposed to Computer Use. |
| Main executable | fyagent.exe version 0.4.0, 32,268,288 bytes, SHA-256 A2F61EB08DEB9CC7AD997E5750647FD202C8949DBECD4338DA48C94BCECC0F79, NotSigned. |
| User helper | fyagent-user-helper.exe, 202,752 bytes, SHA-256 70F1B2EAE181F61DB1A2F9D77906F1CE0F9DF024B6FDE4365DD39F564FFADC50, NotSigned. |
| Uninstaller | uninstall.exe version 0.4.0, 96,546 bytes, SHA-256 0579B9DB3E0FC4995792C6965FFEFAE3C21A6DE46B5DB5AA989DA11F2116BA8C, NotSigned. |
| Updater | Uninstall record has no Modify/Update entry; install directory has no separate updater. No update action was invoked. |
| Uninstall/rollback | Uninstaller exists but is unsigned; no versioned rollback package was discovered and uninstall was not invoked. Private UAT rollback/working copies are evidence-only, not a product rollback package. |
| Repository | Dedicated worktree/branch; initial machine receipt and claim milestones were separately committed and pushed before deep UAT. |

## Runtime surface result

All safe, visible first-level areas were traversed at Windows 100% scale:
Agent, Models, Skills, MCP, Prompts, and Memory. The run also covered seven
Agent entries, seven Models entries, Skills Installed/Discover, MCP
Installed/Discover, seven Prompt targets, four long-term Memory documents,
Daily Memory, Search/Settings/Account clicks, Skill detail/install target/path
dialogs, Add MCP, MCP config-install, Codex loading/settled states, normal,
maximized, and restored window states, and representative long/scrolling
content.

The highest-value runtime results were:

- Codex installed status settled correctly: local 26.818.5345.0, available
  26.818.5229.0, with the local version recognized as newer.
- Skills Installed showed 153 items with independent master/detail scrolling
  and a seven-target assignment panel.
- Skills Discover loaded the SkillHub catalog, official categories, pagination,
  detail, install-target, and path-preview states.
- MCP showed one installed item and the seven-target assignment panel; Add MCP
  and discovery configuration dialogs opened and canceled without persistence.
- Prompts and long-term Memory switched across every actual target without
  exposing private names or bodies and without writes.
- Daily Memory loaded, failed, retried, and failed again.
- Search, Settings, and Account clicks produced no secondary visible or
  accessible surface. They are recorded as C only, consistent with an inert
  tool result rather than an invented dialog.

Average-like rollups are intentionally avoided because a single average would
hide the Daily Memory P1 and untested DPI states. Concrete per-surface 10-point
scores are in the coverage matrix.

## Visual and Windows state result

| Requirement | Result |
| --- | --- |
| 100% normal | PASS for covered surfaces; display/window DPI 96, stable 1154×826 captures. |
| 100% maximized | PASS; stable main capture 1920×1032, authoritative native maximized readback, no observed horizontal clipping. |
| Restore | PASS; stable return to 1154×826. |
| Minimize | NOT TESTED; one transient result contradicted native state and a later attempt was interrupted by real user input. |
| 125% / 150% | NOT TESTED; Windows Settings was not targetable and no fake DPI mechanism was used. |
| Multi-monitor | NOT TESTED; only one monitor was present. |
| Full Tab/focus order | NOT TESTED after three real-user-input detections; all later GUI input stopped. |
| Scrolling / long Chinese text | PASS for covered 100% states; OpenCode Models, Skills, MCP, categories, validation text, and path preview remained readable. |
| Contrast | PARTIAL; no unreadable state was observed, but no formal WCAG measurement was performed. |

## Functional and failure-path result

Safe read/navigation/dialog-cancel paths are covered. No provider, model,
assignment, prompt, memory, install, update, or delete mutation was made against
the real profile.

Daily Memory Retry is the only repeat-click failure promoted to a product
finding. Untouched validation states were captured but submit behavior was not
invoked. Windows long paths, case collisions, CRLF/LF, junctions, ACL/UAC
denial, Defender, atomic rollback, deep links, native-bridge loss, updater,
uninstall, and tray remain NOT TESTED where inducing them would require a
redirectable installed profile, product/environment changes, security-app UI,
or further shared-desktop input.

A profile rollback copy and isolated working copy were created in a private
evidence location. Each contains 2,994 files, 397,675,522 bytes, zero reparse
points, and the same aggregate SHA-256:
EEB0BECD1494E44DFC993E1C17F71F6367D247A6BFF96C0FF125FB5EA9D3BBD6.
Two live source files were locked by the running app, so full real-source to
backup equality is not claimed. No installed-app write was attempted.

## Mandatory isolated P1 retests

### Disabled Prompt may clear the live prompt file

**Windows installed-runtime result: NOT TESTED.**

Current-source code audit shows the risky branch: a disabled prompt is saved,
then an existing live prompt file is cleared if no prompt remains enabled.
Formal Windows builds resolve the frozen interactive profile and ignore
FYAGENT_TEST_HOME unless compiled for tests/test-hooks. The installed 0.4.0
binary contains neither marker. Code audit is not installed-runtime proof, and
the real profile was not touched.

### Mixed Daily Memory filenames may fail the entire page

**Required isolated retest result: NOT TESTED.**

An independent read-only Windows runtime failure is confirmed on the current
profile: its sanitized directory shape contains two valid-date and six
non-date Markdown files; the page and Retry both fail. Source audit explains
that the backend returns all .md entries while the frontend strictly validates
the entire array. This correlation supports WIN-UAT-001, but it is not relabeled
as the synthetic isolated AC7 retest.

Minimum unblock for both cases: an already-enabled disposable Windows
Sandbox/VM, an installed 0.4.0 test-hooks build with vendor-supported profile
redirect, or another vendor-sanctioned HIL redirect. No Windows feature, VM,
account, credential, registry Shell Folder, or user profile was changed.

## Windows-local Agent and tool inventory

This inventory was independently discovered from Windows uninstall records,
Appx, PATH, running processes, executable metadata, and Authenticode. A fixed
FyAgent catalog label is not treated as proof that the corresponding local tool
was detected.

| Tool | Windows version/status | Source and provenance | FyAgent relationship / update boundary |
| --- | --- | --- | --- |
| FyAgent | 0.4.0; running/responding | Machine install; all three installed artifacts NotSigned | Host application. No separate updater or Modify entry; unsigned uninstaller exists. |
| Codex Desktop | Appx 26.818.5345.0 x64; Status Ok; running | Microsoft Store Appx OpenAI.Codex; Store signature kind | Only catalog entry with authoritative installed/version readback. Agent, Models, Skills, MCP, and Prompts target. |
| Grok Bot | 0.24.0; 9 running processes | Per-user registry publisher SpaceXAI; executable signature Valid from Anysphere, Inc. | Not an explicit FyAgent catalog label and not equated with Grok Build. No update entry observed. Provenance needs clarification. |
| Grok CLI | grok 1.0.5 (5115b46bc9); 2 running processes | %USERPROFILE%\.grok\bin\grok.exe; valid X.AI LLC signature | Closest discovered CLI to Grok Build, which appears in Agent/Models/Skills/MCP/Prompts. No authoritative UI install-detection state. |
| Qoder CN IDE | registry 1.106.3; executable 1.25.1; running | E:\Program Files\QoderCN\QoderCN.exe; valid Bright Zenith signature | QoderWork appears in Agent/Models/Skills/MCP; UI does not expose local detection. Registry/executable version drift. |
| WorkBuddy | 5.3.13; not running | E:\Program Files\WorkBuddy\WorkBuddy.exe; valid Tencent signature | WorkBuddy appears in Agent/Models/Skills/MCP; UI does not expose local detection. |
| Claude Desktop | 1.24012.9; not running; no claude PATH command | %LOCALAPPDATA%\AnthropicClaude\Claude.exe; valid Anthropic signature | Name-adjacent to Claude Code, which appears in Agent/Models/Skills/MCP/Prompts; no authoritative equivalence or UI detection claimed. |
| OpenCode legacy desktop | uninstall 1.2.26; executable 0.0.0; not running | %LOCALAPPDATA%\OpenCode\OpenCode.exe; NotSigned | Parallel OpenCode identity; no UI detection precedence exposed. |
| OpenCode AI desktop | 1.18.18; not running | %LOCALAPPDATA%\Programs\@opencode-aidesktop\OpenCode.exe; valid Anomaly Innovations signature | OpenCode appears in Agent/Models/Skills/MCP/Prompts; parallel with legacy desktop and CLI. |
| OpenCode CLI | 1.18.14; not running | WinGet registration plus %USERPROFILE%\bin\opencode.cmd | Same catalog label family, but UI does not state which install it would launch/update. |
| Gemini CLI | 0.54.4; no matching running process | User npm shim under %APPDATA%\npm | Prompts target only; no desktop registry entry or UI install detection. |
| OpenClaw CLI | 2026.8.1-beta.2 (8f382a2); no matching running process | User npm shim under %APPDATA%\npm | Prompts and Memory target; no UI install detection. Gateway/bootstrap work was not touched. |
| Cursor | 3.16.17, commit 6b2afae0257df2bb5e1835f15165dc2f0de056b0, x64; not running | F:\cursor\Cursor.exe; valid Anysphere signature | Not represented as a FyAgent catalog target. |
| CodeBuddy | registry 1.106.1; executable 4.10.0; not running | %LOCALAPPDATA%\Programs\CodeBuddy\CodeBuddy.exe; valid Tencent signature | Not represented as a FyAgent catalog target; version drift. |
| Antigravity | 2.6.0; not running | %LOCALAPPDATA%\Programs\antigravity\Antigravity.exe; NotSigned | Not represented as a FyAgent catalog target. |
| TRAE | Not detected in uninstall records, PATH, or processes | none discovered | Still present as a fixed FyAgent Agent/Models/Skills/MCP target; Models showed guidance-only state. |
| Hermes | Not detected in uninstall records, PATH, or processes | none discovered | Present in Prompts and Memory only; no local detection exposed. |

FyAgent's assignment catalog order observed in Skills/MCP is QoderWork, TRAE,
WorkBuddy, Grok Build, Codex, Claude Code, and OpenCode. Prompts targets Grok
Build, Codex, Claude Code, OpenCode, Gemini, OpenClaw, and Hermes. Memory targets
OpenClaw and Hermes. Cursor, CodeBuddy, Antigravity, and Grok Bot itself have no
explicit catalog relationship in the inspected runtime.

## Acceptance criteria disposition

| AC | Result | Rationale |
| --- | --- | --- |
| AC1 | PASS | Standalone receipt exists; task was claimed by aimaster-windows-codex with actor/time/worktree/progress metadata in a separate pushed milestone. |
| AC2 | PASS | Installed-app and repository baselines, hashes, scope, process, signature, updater/uninstall boundaries are independently recorded. |
| AC3 | PARTIAL | Every safe visible first-level page and actual entry/dialog reached is scored; mutation-dependent confirmation/update/error states are explicitly NOT TESTED. |
| AC4 | PARTIAL | 100% normal/maximize/restore/scroll/clipping covered; 125/150%, stable minimize, full focus order, and multi-monitor are NOT TESTED with exact reasons. |
| AC5 | PARTIAL | Strongest actual C/readback/A layers and safe Cancel/loading/error/Retry states are recorded; R/P and unsafe writes are not claimed. |
| AC6 | PASS | Fresh Windows-only inventory covers FyAgent, Grok Bot/Grok CLI, Codex, and every additional discovered Agent-adjacent tool, including absences and version/provenance drift. |
| AC7 | NOT MET | Both mandatory installed-runtime isolated retests are separately NOT TESTED because no safe supported profile redirect or pre-existing disposable Windows session exists. |
| AC8 | PASS | Findings are reproducible, evidence-linked, severity-ranked, owner-routed, and tied to release/retest decisions in the issue register. |
| AC9 | PASS | Sanitized artifacts passed the fresh gate and were committed/pushed at 1c8399005188ff475c5beff7b5eab770040f398c. |
| AC10 | PASS | Dedicated PR [#138](https://github.com/fy-agent/fyagent/pull/138) targets main, is open/unmerged, and has an active review request for python-rust (赖永杰). |

## Prioritized fixes and retest order

1. Fix Daily Memory backend/frontend filename contract and add mixed-name
   regression coverage; retest initial load and Retry on installed Windows.
2. Produce a signed Windows candidate for the executable, helper, installer,
   and uninstaller; verify chain/timestamp/publisher on a clean machine.
3. Provide a supported disposable-profile path for formal installed UAT and run
   both P1 hypotheses with pre/post/rollback fingerprints.
4. Repeat the entire visual matrix in an exclusive desktop session at 100%,
   125%, and 150%, including stable minimize and full keyboard focus order.
5. Restore Agent introductions, correct Codex installer copy, keep Skills
   header actions mounted on Discover, and defer validation until touch/submit.
6. Resolve Grok Bot publisher/signer provenance and define detection precedence
   for registry/executable/CLI/parallel installations.

Release may be reconsidered only after P1 fixes/gaps are closed and a fresh
signed installed candidate passes the blocked Windows retests. The current
verdict remains NO-GO even if lower-severity visual issues are deferred.

## Fresh verification gate

| Check | Result |
| --- | --- |
| Trellis task/context validation | PASS; implement.jsonl (4 entries) and check.jsonl (3 entries) validated. Two large-spec injection-size warnings are pre-existing context warnings, not task-artifact failures. |
| V2 TypeScript type-check | PASS via the local TypeScript compiler. |
| V2 ESLint scope | PASS for src/v2, tests/v2, and tests/v2-browser. |
| Targeted Agent/Models/Prompts/Memory/Skills tests | BLOCKED before collection: repository requires exact Node 24.19.0, while both available Windows Node paths are 24.18.0. Five suites correctly aborted with zero tests; no pass is claimed. Existing mise is also below the repository-required mise version and was not self-updated. |
| Evidence ledger integrity | PASS; 50/50 private JPEG/JFIF filenames, decoded dimensions, and SHA-256 values match the sanitized index. |
| Image content/type correction | PASS; all 50 files have JPEG/JFIF magic and decoder format, use the .jpg extension, and preserve their pre-correction bytes, hashes, dimensions, and timestamps. |
| Artifact JSON and Markdown links | PASS; task.json parses and all relative Markdown links resolve. |
| Secret/private-path scan | PASS; no username, raw private-evidence path, token, bearer, or private-key pattern found in task artifacts. |
| Git whitespace check | PASS. |
| Scope review | PASS before staging: only task.json plus four task-local deliverables are modified/untracked; the tool-generated workspace journal was removed and an automatically added pnpm build-approval stanza was reverted. |

The Node mismatch is a static-test environment blocker, not a runtime UAT
substitute and not a reason to weaken the NO-GO verdict. No Node/mise upgrade or
build-script approval was performed.

## Delivery and privacy

- Pull request: [#138](https://github.com/fy-agent/fyagent/pull/138), open
  against main, not draft, not merged.
- Review request: python-rust (赖永杰), resolved from the repository's
  dev/laiyongjie branch and confirmed as a repository administrator.
- Current corrected deliverables commit:
  623e13a8ca3ae387ad709a295a5bef3187b5ebd2, pushed to origin.
- CI snapshot after PR creation: Commit Convention, label, Classify Changes,
  and Desktop Acceptance Contract passed; Repository Contracts, Frontend
  Checks, Backend Checks (Windows/macOS), and Windows Native Contracts
  (X64/ARM64) were still in progress. The PR merge state was BLOCKED while
  required checks/review were pending.
- Report: [uat-report.md](./uat-report.md)
- Coverage and scores: [coverage-matrix.md](./coverage-matrix.md)
- Evidence ledger: [evidence-index.md](./evidence-index.md)
- Issues: [issue-register.md](./issue-register.md)
- Machine receipt:
  [machine-acceptance-receipt.md](./machine-acceptance-receipt.md)

No raw screenshots, prompt/memory bodies, credentials, application logs,
identifying user-profile paths, or private evidence-directory absolute paths
are tracked by Git.
