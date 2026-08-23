# Sanitized UAT Evidence Index

## Evidence policy

- `platform=macOS`; the system under test is the installed FyAgent 0.4.2 app, not the source checkout.
- Raw screenshots and the pre-UAT user-data archive stay in a private, untracked local evidence directory. No PNG, private body, credential, token, or user-specific Agent path is committed.
- SHA-256 values below identify only privacy-reviewed screenshots. Restricted captures are omitted rather than redacted into the repository.
- Evidence grades used: `code_audit`, `runtime_screenshot`, `interaction_readback`, `UAT`. `pixel_diff` was not performed and is not claimed.
- Functional layers: `C=control_clicked`, `R=request_observed`, `P=persistence_observed`, `A=authoritative_readback`. A row never claims a stronger layer than was directly observed.

## Baseline evidence

| ID | Grade | Sanitized observation |
|---|---|---|
| BSL-001 | UAT | macOS 26.5.1 (25F80), Apple Silicon `arm64`; app bundle reports version/build 0.4.2 and bundle id `com.fyagent.desktop`. |
| BSL-002 | UAT | Installed bundle is universal `x86_64/arm64`, signed by Developer ID Application William Wang (HY446996QX), and accepted by Gatekeeper as `Notarized Developer ID`. |
| BSL-003 | interaction_readback | Installed executable was running from the application bundle during UAT. |
| BSL-004 | code_audit | Repository context baseline is `origin/main` at `e94307cd810d7c5157b3791da2a8d7ef6a01b8a7`; installed v0.4.2 source tag peels to `d8ab2a2228389fe41ff5c815ddccb3b5823bcaab`. These baselines are deliberately separate. |
| BSL-005 | interaction_readback | A private local pre-UAT profile archive was created with mode 0600 and passed archive-integrity verification. Its location and digest are intentionally not published. |
| BSL-006 | UAT | Runtime window was exercised at 1232×700 and the enforced minimum 1152×640. No canonical design image was available for pixel comparison. |

## Privacy-reviewed runtime screenshots

| ID | Page/state | SHA-256 |
|---|---|---|
| AGT-001 | Agent / QoderWork overview | `fa916cdb5ef784f0e77deb5486e8ffddb0af2c70abb119675b203e547da3f6a8` |
| AGT-002 | Agent / Codex after Refresh | `4fabeb794cfcb43f6866230140447de5d43048eb330e352f161b3e5133ca28fc` |
| AGT-003 | Agent / OpenCode overview | `755d0e6ab3fb2f8d0eb4f1743234972b9c2dcc4206882a23bbb6d561e67fb6dc` |
| AGT-004 | Agent / minimum window | `a3af3199f7e7d994daceff0e7681ea1a2fd8ec7ed36a915d9fcf68c52b53a15c` |
| MOD-001 | Models / Qoder unsupported state | `b5bbc2754a469261bcbe491bf7eb82d2431cd8a1083f8cb7f95899727dc84e3f` |
| MOD-002 | Models / OpenCode populated state | `6707a0fedfeb941d4aa8f8ec5d269a8b562e856eafbc6927b545202f2e189431` |
| MOD-003 | Models / Grok invalid URL | `70452b8c6d6494d6383e5942edddb8f3a4e02a87c2d446813f34bc2b7dc7f6f0` |
| MOD-004 | Models / minimum-window scrolled overlap | `8a49a9f1d12bbc44743df1df8b9fa1e9df8a793a92a3d85ca49e982da98ce58d` |
| SKL-001 | Skills / Installed | `abe0d12ea1964ee20a682a3ff41639aa6e75bcd21c8de2da7d67566796944dc4` |
| SKL-002 | Skills / Discover | `5d553b155370ab729b544c484833259aef35e9f674c387d23847695887ecd4ec` |
| SKL-003 | Skills / no results | `7a86cff22e875affa6bf4d13b7a153b42d7e5caa9e74d1b9505e3e2e40f61713` |
| SKL-004 | Skills / install confirmation at minimum size | `7df9fb06833a7465c0dff90f2e0518246a21317ee568f5d0d2487914a82e2017` |
| SKL-005 | Skills / minimum window | `b1d005bc1aabc7f00cce168aaadab8ef5910a6391b5f328d94e7cc8805eb599e` |
| MCP-001 | MCP / Installed | `68a7f3e8546d1e3838d38f5432f42d87f9d8d72db6daf7863863e27b4c173202` |
| MCP-002 | MCP / Installed no results | `d2aadacddea7a630bb8d73fec403413cbfad1db083c51515bcc9f27f6369bbcf` |
| MCP-003 | MCP / Discover | `2d1ceca34dd8dfd75fca0b116763d3d62baa7a640a983e0bf8e3906ad3288b84` |
| MCP-004 | MCP / invalid JSON add flow | `f619c1130147ea6347f206b4824818f49d8d604a6b11c06baf6121783ca0cb80` |
| MCP-005 | MCP / config-install dialog at minimum size | `786016b2999d232d078f87993748a258085d0394583e0df2e8e24a03a1808462` |
| MCP-006 | MCP / clipped OpenCode target revealed by keyboard focus | `fd904f21982e29278293e427e039165db833ca69a5a6f348953ae2d018772c7e` |
| PRM-001 | Prompts / Grok empty state | `61d0f0b12e9d00c17af4e2c6f44657f95040b6e7878df9d4246c5e98d1d14d0c` |
| PRM-002 | Prompts / isolated unsaved draft | `7f9951515fa3137d6031b9c61f75ce7cf10e965d4b08b4496e7074d8a6451045` |
| PRM-003 | Prompts / discard confirmation | `f677666f80186c5f9c4472418068f44a7cad39e1100fdc1e9914f2e589fea780` |
| PRM-004 | Prompts / post-discard reread | `8114f87649271a2b935a77325206e9eb36fbc90acfc946bca94a178b696136de` |
| PRM-005 | Prompts / minimum window | `9526865b328901984b442e7e75c2162e1b978acb81bd5dd13018940fe06a5d43` |
| PRM-006 | Prompts / rail after minimum-window scroll | `930a23df6bddd679132f20f2281cb46a33b981934116695194e283962d21031b` |
| MEM-001 | Memory / Daily load failure | `d17564c70588ee9bbb86910a744ec77ac4348a933807c4a1765ef639cc904e08` |
| MEM-002 | Memory / Daily retry failure | `f343b81892405ffe4efc8cf18903dd89bab5465c199067ba5271fe3c1fb023df` |
| MEM-003 | Memory / Daily failure at minimum window | `820ee42ea8242e3d5bbcae133c96d8aae5c097e34dfe2ea82694faf45ab98e32` |
| SHL-001 | Shell / Settings click with no visible result | `ef5600af98508b41dd9267d8bb92e0ce8953c4abbba2ea6cb92715596803ef2a` |
| SHL-002 | Shell / Search and Account clicks with no visible result | `d42a34d0ab64ff54351b0375172b63c76bf2500e3cdc05ea7a94b5c9b98d32bf` |

## Interaction and authoritative readback

| ID | Highest functional layer | Case | Result |
|---|---|---|---|
| RDB-001 | A | MCP safe negative/cancel flows | Authoritative database reread remained at 4 records and the pre/post fingerprint matched. |
| RDB-002 | A | Prompt isolated draft cancel/discard | Database reread remained at 2 pre-existing records; no isolated test record was persisted and pre-existing content fingerprints matched. |
| RDB-003 | A | Long-term Memory isolated unsaved edit/discard | The selected live file fingerprint matched its pre-UAT baseline after confirmed discard. |
| RDB-004 | C | Agent Refresh | Refresh was clicked and the same installed/available version state was reread; the transport request was not independently instrumented and no update action was executed. |
| RDB-005 | C | Models invalid and unsaved states | Invalid URL was rejected; route changes preserved the unsaved draft as designed; the draft was then cleared without Save/Apply. |
| RDB-006 | C | Skills/MCP install and delete confirmations | Confirmation and validation states appeared; every persistent action was canceled before write. |
| RDB-007 | C | Memory Daily retry | Initial load and explicit Retry both returned the same visible failure state. |

## Code-audit evidence linked to runtime behavior

| ID | Grade | Evidence |
|---|---|---|
| CDA-001 | code_audit | Prompts UI creates a new item disabled and promises it will not auto-enable: [`Page.tsx`](../../../../../src/v2/pages/prompts/Page.tsx#L367). Backend disabled upsert clears the live prompt file when no prompt is enabled, and import calls that same path: [`prompt.rs`](../../../../../src-tauri/src/services/prompt.rs#L28), [`prompt.rs`](../../../../../src-tauri/src/services/prompt.rs#L146). The installed v0.4.2 source and `origin/main` have the same affected implementation. |
| CDA-002 | code_audit | Daily Memory backend returns every Markdown file: [`workspace.rs`](../../../../../src-tauri/src/commands/workspace.rs#L59). The frontend strictly maps the whole result through a date-only filename parser, so one non-date Markdown rejects the page: [`content.ts`](../../../../../src/v2/shared/platform/tauri/feature-ports/content.ts#L173). Runtime inventory found both date-form and non-date Markdown files without reading or publishing their private bodies/names. |
| CDA-003 | code_audit | Search, Settings, and Account shell buttons are wired to `noop`: [`ToolCluster.tsx`](../../../../../src/v2/widgets/app-shell/ToolCluster.tsx#L7). Installed v0.4.2 source and `origin/main` match for this component. |

## Repository validation evidence

| ID | Result | Scope |
|---|---|---|
| TST-001 | PASS — 8 files, 133 tests | Focused V2 shell/Agent/Models/Skills/MCP/Prompts/Memory tests under the repository-locked toolchain. |
| TST-002 | PASS — command exit 0 | Rust substring-filtered `prompt` run: 29 library tests and one additional name match passed; no selected test failed. The selected set does not contain a regression for CDA-001. |
| TST-003 | PASS — command exit 0 | Trellis prearchive validation and the canonical post-archive `check:contracts` task both completed successfully. |

These test results support contract/code interpretation only. They do not upgrade an installed-app interaction to persistence, Windows acceptance, or production readiness.
