# Windows UAT evidence index

All evidence in this index was produced on Windows AIMASTER against the
installed/running FyAgent 0.4.0; no macOS result was imported. Raw JPEG/JFIF files
remain private and untracked under the alias %PRIVATE_EVIDENCE%\screenshots.
Only sanitized filenames, timestamps, dimensions, hashes, and observations are
committed. Screenshots marked **private-body** may contain local prompt or
memory text and must not be attached to the PR.

Evidence grades follow the task contract: runtime_screenshot,
interaction_readback, UAT, and code_audit. No pixel_diff evidence exists.

## Runtime screenshot ledger

| Id | Private filename | Captured (+08:00) | Pixels | SHA-256 | Sanitized use |
| --- | --- | --- | ---: | --- | --- |
| EV-001 | 001-baseline-100pct-normal-initial-z0.jpg | 2026-08-24 06:00:52 | 1154×826 | 7E05F86423A5227E322306F3D9F173E835B1CBCD23DD76FDAA99BD1FE4A81BB9 | 100% normal baseline; Agent / TRAE. |
| EV-002 | 002-agents-qoderwork-100pct-normal-z0.jpg | 2026-08-24 06:01:17 | 1154×826 | 1031A55A1CD05CD232C7EF5B57BF1A5AE96920CE82EA7662BC4EE17344BEEABC | Unsettled QoderWork transition; retained but excluded from stable conclusions. |
| EV-003 | 003-agents-qoderwork-100pct-normal-stable-z0.jpg | 2026-08-24 06:01:26 | 1154×826 | 43C251AE8B797D9A226A1FD003EEE51683C302C7028433B6D246EEF1A4C4175E | Agent / QoderWork stable. |
| EV-004 | 004-agents-workbuddy-100pct-normal-stable-z0.jpg | 2026-08-24 06:01:41 | 1154×826 | 69E6CC1511DFE12ECC68BD7B6B278E8C96D7DAFDED10ED45765395C451FA62BF | Agent / WorkBuddy stable. |
| EV-005 | 005-agents-grokbuild-100pct-normal-stable-z0.jpg | 2026-08-24 06:01:52 | 1154×826 | 5E018278DEC61EF90F3DBF4205418174E3D3EBB2BED6FECEDD9A553376830926 | Agent / Grok Build stable. |
| EV-006 | 006-agents-codex-100pct-normal-stable-z0.jpg | 2026-08-24 06:02:03 | 1154×826 | F87EBF1D3B3841827A3BA49889F55041D3E556BFB4E554EBD34A04742FE12CE7 | Agent / Codex installer loading. |
| EV-007 | 007-agents-codex-100pct-status-settled-z0.jpg | 2026-08-24 06:02:20 | 1154×826 | 1B4CB750190FC332B69CE66A8E046F758E1978D8833C267404B2E15B36F7D8B9 | Agent / Codex installer settled. |
| EV-008 | 008-agents-claude-code-100pct-normal-stable-z0.jpg | 2026-08-24 06:02:37 | 1154×826 | 042A1EBAA3DA73F4D0270E8DE05C9F8F035E1751B8669663DBBE0434B4E397D4 | Agent / Claude Code stable. |
| EV-009 | 009-agents-opencode-100pct-normal-stable-z0.jpg | 2026-08-24 06:02:51 | 1154×826 | 4DA79CB9592D53858EF5D8557EF45F91238337D2B8A458C9B229F8F1C99FDA75 | Agent / OpenCode stable. |
| EV-010 | 010-models-workbuddy-100pct-normal-stable-z0.jpg | 2026-08-24 06:03:06 | 1154×826 | 640BF77B985590CC4A52A4D66CC33D58BD1FB8F13B1868C2735A6A12C8A999F1 | Models / WorkBuddy, untouched validation state. |
| EV-011 | 011-models-qoderwork-100pct-normal-stable-z0.jpg | 2026-08-24 06:03:26 | 1154×826 | 4C37A612ECA4D635D488163EC75CBFCA5E5240CC429CE6D42E0BA21BAFCE7E9F | Models / QoderWork unsupported state. |
| EV-012 | 012-models-trae-work-100pct-normal-stable-z0.jpg | 2026-08-24 06:03:39 | 1154×826 | 20C0A48AC9353BF9026DB5795A4ECD7BEA101EDEB1EA219F5300277D20585FA2 | Models / TRAE guidance state. |
| EV-013 | 013-models-grokbuild-100pct-normal-stable-z0.jpg | 2026-08-24 06:03:52 | 1154×826 | 177936FF0A06363DD6BD2E8CB3ABC8E8C69B42224B0900EAD2AC69827E39EFAB | Models / Grok Build untouched errors. |
| EV-014 | 014-models-codex-100pct-normal-stable-z0.jpg | 2026-08-24 06:04:09 | 1154×826 | 2954521FE057CAA7153B868930B8883EFA1F43EEE9DDEF771F0A164FFB898AA2 | Models / Codex configured state. |
| EV-015 | 015-models-claude-code-100pct-normal-stable-z0.jpg | 2026-08-24 06:04:21 | 1154×826 | 3618D68DB2B54FA0C864373E937B123AFAD004EE33424FFAA5512F1D39064235 | Models / Claude Code. |
| EV-016 | 016-models-opencode-100pct-normal-stable-z0.jpg | 2026-08-24 06:04:33 | 1154×826 | EF90814A524949926410BC4597372F1A517A186440C53073DFF794B83BC70764 | Models / OpenCode normal. |
| EV-017 | 017-models-opencode-100pct-detail-scrolled-z0.jpg | 2026-08-24 06:04:52 | 1154×826 | B506625058E0742CA22A451948D8F728E12E46BAACB90DDABACF0B9ACDB2E9EE | Models / OpenCode scrolled. |
| EV-018 | 018-skills-installed-100pct-normal-stable-z0.jpg | 2026-08-24 06:05:08 | 1154×826 | 4E99C7A228D1E28EB92B130E575F48D6C75A6C7CE3EC0F1899175264B5AAEF2B | Skills / Installed master-detail. |
| EV-019 | 019-skills-discover-100pct-normal-settled-z0.jpg | 2026-08-24 06:05:25 | 1154×826 | 514226C62FD52AE59263F3E6E7CC5015809BADEA918903070DDCADBE662B9F42 | Skills / Discover settled. |
| EV-020 | 020-skills-discover-detail-dialog-100pct-z0.jpg | 2026-08-24 06:05:46 | 1154×826 | 9600C4E2441EDA0E01C0373DE2758A03438EAE5EC95EEC5AA50434E9BABCA56F | Skill detail dialog. |
| EV-021 | 021-skills-install-target-dialog-step1-100pct-z0.jpg | 2026-08-24 06:06:17 | 1154×826 | 5DDF2AB8955A2570905C745B5EB49E00F56C179C3042C6D71A8F33AFCDB71343 | Skill install target step. |
| EV-022 | 022-skills-install-target-dialog-path-preview-100pct-z0.jpg | 2026-08-24 06:06:29 | 1154×826 | 707AB187900234CC9A4AAE3650CEB849B7C007365301E501D6705A1BA7BC3046 | Skill install path preview; canceled before confirm. |
| EV-023 | 023-mcp-discover-100pct-normal-stable-z0.jpg | 2026-08-24 06:07:22 | 1154×826 | 49A10EF70FE8E783CFB0C06BDEE564BB266F6805544B6DA657BD3E5B1F4FDE77 | MCP / Discover initial stable state. |
| EV-024 | 024-mcp-installed-100pct-normal-stable-z0.jpg | 2026-08-24 06:07:49 | 1154×826 | 8B3B1D719AB65D1E0D1C736B8DFB0D7D1288456641641B386DC568ADFD9B6087 | MCP / Installed selected detail. |
| EV-025 | 025-mcp-add-dialog-100pct-z0.jpg | 2026-08-24 06:08:03 | 1154×826 | 5C7C9E9AF59B1FDDC0B744D5DB517D06B5E5DEEA722DD2AE5B2C7033CB8311FD | Add MCP dialog; canceled. |
| EV-026 | 026-mcp-discover-100pct-normal-stable-z0.jpg | 2026-08-24 06:08:29 | 1154×826 | 06D3BDC49D7ACC60B4AF8C7AFBB048A3C3318849993A3F36985C17BDCC48D121 | MCP / Discover settled after tab switch. |
| EV-027 | 027-mcp-config-install-dialog-100pct-z0.jpg | 2026-08-24 06:09:07 | 1154×826 | 7DC327F02B9F8E74E5AC73BFF790C65FDE71888DAE4BBADE26001B978CFF8BA4 | MCP config-install target dialog; canceled. |
| EV-028 | 028-prompts-current-app-100pct-private-z0.jpg | 2026-08-24 06:09:35 | 1154×826 | E163240D9AB38584B8B9AC127EAB77BC293A77EFA4B9C3DE16C1DACDB520FD81 | Prompts / Grok Build; **private-body policy**. |
| EV-029 | 029-prompts-codex-100pct-private-z0.jpg | 2026-08-24 06:09:55 | 1154×826 | 9FE08194C2457F7E5DECA77D2CA306D210E005D67D6FC6EE68948EA1279BE1F2 | Prompts / Codex; **private-body**. |
| EV-030 | 030-prompts-claude-code-100pct-private-z0.jpg | 2026-08-24 06:10:13 | 1154×826 | 1D07E48095835F52158490D5F382FD4627DD93C2C5E8A35AC3DE2382ED8C8911 | Prompts / Claude Code; **private-body policy**. |
| EV-031 | 031-prompts-opencode-100pct-private-z0.jpg | 2026-08-24 06:11:09 | 1154×826 | 76426ADA910FBF4DED5B2F411A4DABEB3D33D671628E0CD86745220C9A8A4A0D | Prompts / OpenCode; **private-body**. |
| EV-032 | 032-prompts-gemini-100pct-private-z0.jpg | 2026-08-24 06:11:22 | 1154×826 | D2BE553209BAA9C1D9254AE34C72D7540BB191D0647D70D355AA2B0E8EB5DD5F | Prompts / Gemini; **private-body policy**. |
| EV-033 | 033-prompts-openclaw-100pct-private-z0.jpg | 2026-08-24 06:11:38 | 1154×826 | BFEC2079EB60C0ADE21EFA06D3301EF9F4A5717EE8370D0287787242FAE2ADFB | Prompts / OpenClaw; **private-body policy**. |
| EV-034 | 034-prompts-hermes-100pct-private-z0.jpg | 2026-08-24 06:11:55 | 1154×826 | 52DFDA8D228EAFF80AF7BEFD3D46B0473A9EB57525ECF6462766B31A48F12E87 | Prompts / Hermes; **private-body policy**. |
| EV-035 | 035-memory-current-document-100pct-private-z0.jpg | 2026-08-24 06:12:24 | 1154×826 | 0836EAF3F67F25170EC648A7C814085FAE2DE4DDD17AA8FAB74DD7352674A6FC | Memory current document; **private-body policy**. |
| EV-036 | 036-memory-empty-openclaw-100pct-visual-z0.jpg | 2026-08-24 06:12:39 | 1154×826 | 0836EAF3F67F25170EC648A7C814085FAE2DE4DDD17AA8FAB74DD7352674A6FC | OpenClaw MEMORY empty state; duplicate pixels of EV-035. |
| EV-037 | 037-memory-openclaw-user-100pct-private-z0.jpg | 2026-08-24 06:13:08 | 1154×826 | B97060AB32003460DF1095F35031518F934D57FF759A7AA1DEF432CDBF550E54 | OpenClaw USER; **private-body**. |
| EV-038 | 038-memory-hermes-memory-100pct-private-z0.jpg | 2026-08-24 06:13:49 | 1154×826 | BF8A45FD3EA3740FFDE48499B51B629F4BF4A1629AB49DD482DCCAFEF424BF3B | Hermes MEMORY; **private-body**. |
| EV-039 | 039-memory-hermes-user-100pct-private-z0.jpg | 2026-08-24 06:14:18 | 1154×826 | E6C1CFC168CA20FFA79A9337F7447E8A31DDFDB890A9B0C189D9A841800BE84D | Hermes USER; **private-body**. |
| EV-040 | 040-memory-daily-list-100pct-private-z0.jpg | 2026-08-24 06:14:46 | 1154×826 | BD1C1CAABF89C8BFD65E52872AE4B7EAB68EADDE31CCC643D8E2E4BEC7F36FC2 | Daily Memory initial loading; **private-body policy**. |
| EV-041 | 041-memory-daily-settled-100pct-private-z0.jpg | 2026-08-24 06:15:02 | 1154×826 | 562C32C409FEB696A76440E095F6946C72B0FF328B8B4777B0BCE994B654EE2D | Daily Memory settled whole-page failure. |
| EV-042 | 042-memory-daily-retry-loading-100pct-private-z0.jpg | 2026-08-24 06:15:20 | 1154×826 | E3B098AD9AA1868BA00C7C458210E6F0F85BAC6B716CB773B20E9BB8A0481B14 | Retry clicked; loading state. |
| EV-043 | 043-memory-daily-retry-failed-again-100pct-private-z0.jpg | 2026-08-24 06:15:37 | 1154×826 | 4ADBDFFCA24CABDF7DE537B90989FEF3AC2E686855B66638C79B3A17FA7FE45D | Retry settled to the same failure. |
| EV-044 | 044-global-search-surface-100pct-z0.jpg | 2026-08-24 06:15:50 | 1154×826 | 305A917C1F664B75BC4FF4C7CE06C7C91AE0EBA773A98A088846771646F020FE | Search tool clicked; no secondary surface. |
| EV-045 | 045-global-settings-surface-100pct-z0.jpg | 2026-08-24 06:16:07 | 1154×826 | FDCE5187D888CCD4CC165189015214E9873B0EDDC475CB5E055BE5B16BC3666F | Settings tool clicked; no secondary surface. |
| EV-046 | 046-global-account-surface-100pct-z0.jpg | 2026-08-24 06:16:21 | 1154×826 | 5CECFCF40A2F6753DBB4D911EDF3850A890E8BDD49B486E282824C15D1A73850 | Account tool clicked; no secondary surface. |
| EV-047 | 047-agents-opencode-100pct-maximized-stable-z0.jpg | 2026-08-24 06:16:52 | 16×16 | DB9E3B1E7C2F51D608B76E958BE86FE9A30156ABCFC4390E7DE71A6C54E30D1C | Auxiliary native capture fragment; excluded. |
| EV-048 | 048-agents-opencode-100pct-maximized-stable-z1.jpg | 2026-08-24 06:16:52 | 16×16 | DB9E3B1E7C2F51D608B76E958BE86FE9A30156ABCFC4390E7DE71A6C54E30D1C | Auxiliary native capture fragment; excluded. |
| EV-049 | 049-agents-opencode-100pct-maximized-stable-z2.jpg | 2026-08-24 06:16:52 | 1920×1032 | 6479C66E1B60BEAC2EACB790CF4970FDF9184F4D478D0502F207BA31FA1BFDFA | Main maximized 100% capture. |
| EV-050 | 050-agents-opencode-100pct-restored-stable-z0.jpg | 2026-08-24 06:17:59 | 1154×826 | FE2DED75A7C6776048A4874EB660208FB81C613EB12A845DEF3A08C1E89A3566 | Stable restore after maximize. |

## Sanitized native and interaction evidence

| Id | Grade | Method | Sanitized observation |
| --- | --- | --- | --- |
| SYS-001 | UAT | Windows version, process, uninstall registry, file metadata | Windows 11 Pro 23H2 build 22635.5305, x64; FyAgent 0.4.0 is machine-installed and was running/responding as PID 70360 at acceptance. See [machine-acceptance-receipt.md](./machine-acceptance-receipt.md). |
| SYS-002 | UAT | Get-FileHash + Get-AuthenticodeSignature | %ProgramFiles%\FyAgent\fyagent.exe: 32,268,288 bytes, SHA-256 A2F61EB08DEB9CC7AD997E5750647FD202C8949DBECD4338DA48C94BCECC0F79, NotSigned; fyagent-user-helper.exe: 202,752 bytes, SHA-256 70F1B2EAE181F61DB1A2F9D77906F1CE0F9DF024B6FDE4365DD39F564FFADC50, NotSigned; uninstall.exe: 96,546 bytes, SHA-256 0579B9DB3E0FC4995792C6965FFEFAE3C21A6DE46B5DB5AA989DA11F2116BA8C, NotSigned. |
| SYS-003 | UAT | Native DPI/monitor/window APIs | One 1920×1080 monitor, 1920×1032 work area; system and FyAgent DPI 96 (100%). Maximize readback: maximized, outer window 1936×1048; restore returned 1154×826. |
| SYS-004 | UAT | Native copy + full private-copy tree fingerprints | Rollback and isolated working copies each have 2,994 files, 397,675,522 bytes, zero reparse points, and aggregate SHA-256 EEB0BECD1494E44DFC993E1C17F71F6367D247A6BFF96C0FF125FB5EA9D3BBD6. Both copy operations reported success-with-copies. Two live source files could not be hashed because the running app held them open; source equality is therefore explicitly unclaimed. |
| SYS-005 | UAT | Optional-feature, VM, profile, binary-string discovery | Windows Sandbox is disabled, its executable/start entry is absent; Hyper-V is enabled but has zero existing VMs. One other non-special user profile exists but is unloaded. Installed fyagent.exe has no FYAGENT_TEST_HOME, test-hooks, or profile-redirect marker. No feature/account/VM/Shell Folder was changed. |
| SYS-006 | interaction_readback | Computer Use safety stop | Real user input was detected on three GUI attempts. No later GUI input was sent; stable minimize, 125/150% DPI, and full keyboard focus order remain NOT TESTED. |
| SYS-007 | UAT | Registry/Appx/PATH/process/file-version/signature inventory | Fresh Windows-only Agent tool inventory is recorded in [uat-report.md](./uat-report.md). It includes FyAgent, Grok Bot, Grok CLI, Codex, Qoder CN, WorkBuddy, Claude Desktop, OpenCode variants/CLI, Gemini CLI, OpenClaw CLI, Cursor, CodeBuddy, and Antigravity; TRAE and Hermes were not detected. |
| SYS-008 | UAT | Read-only filesystem shape count, bodies/names omitted | Current OpenClaw daily-memory directory exists with 12 entries: 10 files, 2 directories, 8 Markdown files, of which 2 match YYYY-MM-DD.md and 6 do not; all 8 Markdown files were readable and there were zero reparse points. |
| SYS-009 | interaction_readback | Runtime click and stable readback | Skill detail/install, MCP add/config-install, and their Cancel/Escape paths were read back without persistence. Daily Memory Retry produced loading and the same terminal failure. |
| SYS-010 | UAT | JPEG/JFIF magic, image decoder, and pre/post manifest | All 50 private captures begin with JPEG/JFIF magic FFD8FFE000104A464946 and decode with RawFormat GUID b96b3cae-0728-11d3-9d7b-0000f81ef32e. Their ledger/private filenames were normalized to the .jpg extension. A manifest over basename, bytes, timestamp, SHA-256, decoded dimensions, raw format, and magic retained SHA-256 6CEA52FA660FC20319B136BE5D38C47F95F6B165EB8A41B2977A8AF27A3B4BF2 before and after rename. |

## Source and contract evidence

Source inspection is explanatory code_audit; it never substitutes for the
installed runtime.

| Id | Grade | Relative source | Observation |
| --- | --- | --- | --- |
| SRC-001 | code_audit | src-tauri/src/config.rs:21 | Formal Windows get_home_dir() resolves the frozen interactive user's profile; FYAGENT_TEST_HOME is compiled only for macOS, tests, or test-hooks. |
| SRC-002 | code_audit | src-tauri/src/commands/workspace.rs:59; src/v2/shared/platform/tauri/feature-ports/content.ts:173 | Backend list includes every .md file, while the frontend parser maps the whole array through strict real-date YYYY-MM-DD.md validation. One non-date Markdown entry can reject the full list. |
| SRC-003 | code_audit | src-tauri/src/services/prompt.rs:28 | A disabled prompt is saved; if no prompt remains enabled, an existing live prompt target is atomically overwritten with an empty string. This makes the unrun isolated hypothesis materially plausible but is not installed-runtime proof. |
| SPEC-001 | code_audit | .trellis/spec/frontend/v2-agent-models.md:31 | Non-Codex Agent details require page-local 产品介绍; installer/intro copy must not name FyAgent. |
| SPEC-002 | code_audit | .trellis/spec/frontend/v2-skills-mcp.md:553 | Skills header remains mounted on Discover so 检查更新 and 更多 remain visible. |

## Privacy and integrity notes

- No credential, token, prompt body, memory body, identifying user-profile path,
  raw screenshot, or raw application log is committed.
- Screenshot hashes were freshly recalculated from the private JPEG/JFIF files after
  capture. EV-035 and EV-036 intentionally hash identically because the
  rendered pixels are identical.
- An independent content/type audit found an earlier extension-label mismatch.
  All 50 names are now normalized to .jpg; bytes, SHA-256 values, decoded
  dimensions, and timestamps are unchanged.
- EV-002 is an unsettled transition, and EV-047/EV-048 are tiny auxiliary
  native-capture fragments; none is used to support a pass/fail claim.
- R is not claimed merely because a renderer changed. No request-sniffer was
  attached. P is not claimed because no installed-profile mutation was
  authorized or safely isolated.
