# Windows runtime coverage matrix

This matrix covers only the FyAgent `0.4.0` instance installed and running on
`AIMASTER`. `COVERED` means the named runtime surface was visibly inspected; it
does not mean every control passed. `NOT TESTED` is intentional wherever the
shared-desktop, profile-safety, permission, or environment boundary prevented a
valid runtime claim.

Evidence ids resolve through [evidence-index.md](./evidence-index.md). Scores are
visual-experience scores out of 10 for the state actually rendered at Windows
100% scale. They are not pixel-diff scores.

## Runtime surfaces and visual scores

| Area | Runtime entry/state | Coverage | Evidence | Score | Visual assessment |
| --- | --- | --- | --- | ---: | --- |
| Shell | Sidebar, native title bar, content frame | COVERED | EV-001, EV-049, EV-050 | 7.4 | Navigation and hierarchy are immediately legible; content width scales cleanly from the normal window to maximized. |
| Shell tools | Search | COVERED (C only) | EV-044 | 5.8 | Click produced no visible or accessible secondary surface; discoverability is low even if the current contract intentionally keeps this tool inert. |
| Shell tools | Settings | COVERED (C only) | EV-045 | 5.8 | Same inert result as Search; no settings surface was available to score separately. |
| Shell tools | Account | COVERED (C only) | EV-046 | 5.8 | Same inert result as Search; no account surface was available to score separately. |
| Agent | TRAE | COVERED | EV-001 | 5.9 | Identity is readable, but the main body is unusually empty and no visible 产品介绍 section is rendered. |
| Agent | QoderWork | COVERED | EV-003 | 5.9 | Clear selection state; sparse detail body and missing visible 产品介绍 reduce information value. |
| Agent | WorkBuddy | COVERED | EV-004 | 5.9 | Consistent with the directory, but the empty lower region dominates the page. |
| Agent | Grok Build | COVERED | EV-005 | 5.9 | Clean but under-informative; no visible product introduction. |
| Agent | Codex | COVERED, loading + settled | EV-006, EV-007 | 7.3 | The installer state, local/available versions, refresh, and launch affordances have a useful hierarchy; host-branded explanatory copy conflicts with the current contract. |
| Agent | Claude Code | COVERED | EV-008 | 5.9 | Legible but sparse; no visible product introduction. |
| Agent | OpenCode | COVERED | EV-009 | 5.9 | Legible but sparse; no visible product introduction. |
| Models | QoderWork unsupported state | COVERED | EV-011 | 6.8 | The unsupported third-party configuration state and MCP direction are explicit. |
| Models | TRAE guidance-only state | COVERED | EV-012 | 6.9 | Guidance and the observed zero custom model IDs are understandable without pretending to be writable configuration. |
| Models | WorkBuddy configured list/form | COVERED, read-only | EV-010 | 6.2 | Existing model count is useful, but an untouched empty draft immediately shows a validation error. |
| Models | Grok Build form | COVERED, read-only | EV-013 | 5.8 | Multiple errors appear before interaction and compete with the form hierarchy; no secret was entered. |
| Models | Codex configured state | COVERED, read-only | EV-014 | 7.0 | Current configured status and transport fields are structured clearly; secret values remain blank/masked. |
| Models | Claude Code form | COVERED, read-only | EV-015 | 6.8 | Fetch-model affordance is visible and the density remains manageable. |
| Models | OpenCode provider/form | COVERED, scrolled | EV-016, EV-017 | 6.1 | Provider context is visible, but an untouched blank service URL is already marked invalid; long content scrolls without horizontal clipping. |
| Skills | Installed, selected detail, 153-item state | COVERED | EV-018 | 7.6 | Strong master-detail hierarchy, independent scrolling, and clear seven-target assignment panel. |
| Skills | Discover / Skill 市场 | COVERED | EV-019 | 7.1 | Catalog, categories, pagination, install, detail, and external links are findable; page actions disappear even though the current contract keeps them mounted. |
| MCP | Discover | COVERED | EV-023, EV-026 | 7.2 | Curated cards are readable and the discovery state is distinct from installed management. |
| MCP | Installed, one configured item | COVERED | EV-024 | 7.3 | Installed detail and seven-target assignments are compact and legible. |
| Prompts | Grok Build empty state | COVERED, read-only | EV-028 | 7.0 | Empty state is direct and avoids implying persistence. |
| Prompts | Codex editor state | COVERED, private body not recorded | EV-029 | 7.1 | Selection/editor hierarchy is clear; evidence intentionally omits names and body text. |
| Prompts | Claude Code empty state | COVERED, read-only | EV-030 | 7.0 | Consistent empty-state treatment. |
| Prompts | OpenCode editor state | COVERED, private body not recorded | EV-031 | 7.1 | Consistent editor layout; no save/toggle action was used. |
| Prompts | Gemini empty state | COVERED, read-only | EV-032 | 7.0 | Consistent empty-state treatment. |
| Prompts | OpenClaw empty state | COVERED, read-only | EV-033 | 7.0 | Consistent empty-state treatment. |
| Prompts | Hermes empty state | COVERED, read-only | EV-034 | 7.0 | Consistent empty-state treatment. |
| Memory | OpenClaw MEMORY empty/missing | COVERED, read-only | EV-035, EV-036 | 7.0 | The empty document state is understandable; save remains unavailable until dirty. |
| Memory | OpenClaw USER | COVERED, private body not recorded | EV-037 | 7.1 | Document selector, editor, and character context are legible without exposing content. |
| Memory | Hermes MEMORY | COVERED, private body not recorded | EV-038 | 7.0 | Budget/state information is concise and readable. |
| Memory | Hermes USER | COVERED, private body not recorded | EV-039 | 7.0 | Consistent with Hermes MEMORY and remains safely read-only. |
| Memory | Daily Memory loading/error/retry | COVERED, failed | EV-040–EV-043 | 3.0 | The loading and error states are clear, but the entire page remains unavailable after Retry even though valid date-named files exist. |
| Dialog | Skill detail | COVERED, Cancel | EV-020 | 7.5 | Full description is separated from the clamped card preview and the modal hierarchy is clear. |
| Dialog | Skill install target, step 1 | COVERED, Cancel | EV-021 | 7.6 | Seven catalog-ordered radio targets are understandable. |
| Dialog | Skill install path preview | COVERED, Cancel before confirm | EV-022 | 7.4 | Destination preview is useful; no installation or profile write occurred. |
| Dialog | Add MCP | COVERED, Cancel | EV-025 | 6.4 | Fields are understandable, but the tall internal-scroll layout is dense and blank Save looks enabled; invocation was deliberately not attempted. |
| Dialog | MCP discovery config install | COVERED, Cancel | EV-027 | 7.5 | Target selection is clear and consistent with Skills; no persistence occurred. |

No pixel parity claim is made: there was no canonical Windows target image and
no `pixel_diff` run.

## Display, window, focus, scrolling, and clipping

| Check | Result | Evidence / reason |
| --- | --- | --- |
| Windows 100% scale, normal window | PASS | Native display and window DPI both read `96`; stable `1154×826` captures cover every runtime area above. |
| Windows 100% scale, maximized | PASS | EV-049 is a stable `1920×1032` main capture; native window readback reported maximized and no horizontal scrollbar/clipping was visible. EV-047/EV-048 are 16×16 auxiliary capture fragments and are excluded from conclusions. |
| Restore after maximize | PASS | EV-050 returned to stable `1154×826`. |
| Stable minimized state | NOT TESTED | One capture observed only a transient minimized error, but native state immediately read not minimized; a later attempt was interrupted by real user input. The transient is not promoted to PASS. |
| Windows 125% scale | NOT TESTED | Windows Settings could not be located as a targetable Computer Use window. Forcing registry, PowerShell, or WebView zoom would not be valid Windows DPI evidence and was not used. |
| Windows 150% scale | NOT TESTED | Same blocker as 125%. |
| Multi-monitor scale transition | NOT TESTED | Only one `1920×1080` monitor was present. |
| Keyboard Tab/focus order | NOT TESTED | Computer Use detected real user input on three attempts; all further GUI input was stopped to protect the shared desktop. |
| Vertical scrolling | PASS at 100% | OpenCode Models (EV-016/EV-017), Skills Installed/Discover, and MCP content visibly scrolled without horizontal drift. |
| Horizontal clipping/scroll | PASS for covered 100% states | None observed in normal or maximized stable captures. This does not extrapolate to untested DPI values. |
| Native title bar and maximize/restore controls | PASS at 100% | Native maximize and restore both had authoritative OS window-state readback. |
| Long Chinese labels/readability | PASS for covered 100% states | Representative long labels, validation text, category names, and path preview remained readable. |
| Contrast | PARTIAL | Runtime visual review found no unreadable covered state, but no formal WCAG contrast measurement was run. |

## Functional evidence layers

`C` = control clicked, `R` = request observed, `P` = persistence observed,
`A` = authoritative readback. A visible renderer transition is recorded as
`interaction_readback`; it is not silently upgraded to `R`, `P`, or native
configuration authority.

| Capability | Status | Strongest observed layer | Evidence / boundary |
| --- | --- | --- | --- |
| First-level navigation across Agent, Models, Skills, MCP, Prompts, Memory | COVERED | C + interaction_readback | Stable route-specific content was read back for every first-level area. |
| Seven Agent selections | COVERED | C + interaction_readback | EV-001, EV-003–EV-009. |
| Codex Desktop installed/version status | PASS (read-only) | A | EV-006/EV-007 plus Windows Appx authority: local `26.818.5345.0`, available `26.818.5229.0`, local newer. No update/launch action was invoked. |
| Seven Models selections | COVERED (read-only) | C + interaction_readback | EV-010–EV-017. No provider/model mutation, fetch, save, delete, or secret entry. |
| Skills Installed/Discover switch and list/detail reads | COVERED | C + interaction_readback | EV-018–EV-020. |
| Skill install target/path flow | COVERED through pre-confirmation | C + interaction_readback | EV-021/EV-022; canceled before persistence. |
| MCP Installed/Discover switch and detail reads | COVERED | C + interaction_readback | EV-023/EV-024/EV-026. |
| Add MCP and discovery config dialogs | COVERED through pre-confirmation | C + interaction_readback | EV-025/EV-027; canceled before persistence. |
| Prompt app switching and read-only states | COVERED | C + interaction_readback | EV-028–EV-034; private names/bodies omitted. |
| Prompt create/import/toggle/save/delete | NOT TESTED | none | Installed formal build cannot be redirected from the real Windows profile; an isolated write would violate the task safety contract. |
| Long-term Memory switching/read-only states | COVERED | C + interaction_readback | EV-035–EV-039; private bodies omitted. |
| Long-term Memory save/toggle | NOT TESTED | none | Same profile-isolation blocker; no real-user write. |
| Daily Memory initial load | FAIL | interaction_readback | EV-040/EV-041: loading settled to whole-page failure. No network/native request instrumentation was used, so `R` is not claimed. |
| Daily Memory Retry | FAIL | C + interaction_readback | EV-042/EV-043: Retry returned to loading and then the same failure. |
| Search / Settings / Account | COVERED as inert tools | C only | EV-044–EV-046; no transition, dialog, request, persistence, or authoritative configuration readback. |
| Maximize / restore | PASS | C + A | Native OS window state and stable screenshots agree. |
| Minimize / keyboard-only navigation | NOT TESTED | none reliable | Shared-desktop input safety stop; see display matrix. |

## Windows failure-path matrix

| Failure/path family | Result | Evidence / reason |
| --- | --- | --- |
| Repeat click / Retry | FAIL on Daily Memory | EV-041–EV-043; retry did not recover. |
| Cancel before mutation | PASS for covered dialogs | Skill detail, Skill install, Add MCP, and MCP config-install dialogs all closed without persistence. |
| Empty state | COVERED | Agent sparse states, unsupported Qoder Models, Prompt empty states, OpenClaw/Hermes memory empty states. |
| Invalid untouched model draft | OBSERVED | Grok Build, WorkBuddy, and OpenCode display validation errors before user interaction; submit behavior was not invoked. |
| Loading and terminal error | COVERED | Codex status loading→settled and Daily Memory loading→error→retry→error. |
| Disabled/clean editor state | COVERED | Prompt/Memory save remained unavailable until dirty; no write was induced. |
| Drive/profile boundary | PARTIAL | Machine install is on `%ProgramFiles%`; discovered tools span system, user-local, `E:` and `F:` locations. Mutation across those boundaries was not induced. |
| Profile copy and rollback boundary | VERIFIED FOR PRIVATE COPIES | Backup and working copies each contain 2,994 files / 397,675,522 bytes, zero reparse points, and identical aggregate SHA-256 `EEB0BECD1494E44DFC993E1C17F71F6367D247A6BFF96C0FF125FB5EA9D3BBD6`. Two live source files were locked, so full source-to-backup equality is not claimed. |
| Windows separators / long path / case collision / CRLF-LF | NOT TESTED | Requires controlled writes in a redirectable installed profile; no safe supported redirect exists. |
| Junction/symlink | NOT TESTED | Private copies were verified to contain zero reparse points; no junction was created against real-user paths. |
| File lock | OBSERVED, product recovery NOT TESTED | Read-only hashing encountered two live profile files held open by FyAgent. The copy succeeded, but no lock was induced against a product save. |
| ACL/UAC write denial | NOT TESTED | Deliberately inducing denial in the machine install or real user profile would cross the safety boundary. |
| Defender boundary | NOT TESTED | No exclusion, security-app automation, quarantine, or synthetic detection was attempted. |
| Atomic replace / rollback after failed write | NOT TESTED | Installed-profile redirect blocker prevents a safe authoritative write test. |
| Native bridge unavailable | NOT TESTED | No sanctioned way to disable the bridge without altering the running product environment. |
| Deep link | NOT TESTED | Import/update deep links can mutate real configuration and were not invoked. |
| Updater | NOT TESTED | FyAgent uninstall record has no update/modify entry and install directory has no separate updater; no upgrade was authorized. |
| Uninstall / rollback package | NOT TESTED | `%ProgramFiles%\FyAgent\uninstall.exe` exists but is unsigned; uninstall was not invoked and no versioned rollback package was discovered. |
| Tray | NOT TESTED | Shared-desktop user-input detection stopped further GUI interaction before a reliable tray exercise. |

## Isolated P1 hypotheses

| Hypothesis | Windows result | Evidence | Minimum safe unblock |
| --- | --- | --- | --- |
| Creating/importing a disabled prompt when no DB prompt is enabled may clear the live prompt file | NOT TESTED | Current-source `code_audit` shows the clearing branch; installed `fyagent.exe` contains neither `FYAGENT_TEST_HOME` nor `test-hooks`, and formal Windows home resolution ignores the environment override. No real-user write occurred. | Provide an already-enabled disposable Windows VM/Sandbox, or an installed `0.4.0` test-hooks build with a vendor-supported profile redirect. |
| A Daily Memory directory containing a valid date Markdown file plus a non-date Markdown file may fail the whole page and Retry | NOT TESTED as the required isolated retest; independently CONFIRMED as a read-only runtime failure on the current profile | Current private directory shape is 2 valid-date plus 6 non-date Markdown files; EV-040–EV-043 show load and Retry failure. This correlation is not mislabeled as the AC7 isolated write retest. | Same disposable Windows/test-hooks condition; seed only synthetic files, then verify list, Retry, and authoritative filesystem readback. |

Windows Sandbox is disabled and has no executable/start entry; Hyper-V is enabled
but contains zero existing VMs. One other non-special Windows profile exists but
is not loaded and no credentials/authorization were available. No feature was
enabled, no account or VM was created, and no Shell Folder or registry redirect
was changed.
