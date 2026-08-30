# #141 current-main frontend mapping

Issue: https://github.com/fy-agent/fyagent/issues/141

## Review identity

- Issue status checked 2026-08-30: open; last updated 2026-08-24. Its
  macOS 0.4.2 / Windows 0.4.0 findings are historical.
- Current Stage 5 implementation commits: `3110a5cc`, `0c87c12e`,
  `32f81e96`.
- Comparison base: `origin/main` at `1e52e416900426cdc86539ee5c359f486ed08bb3`.
- Automated host: macOS 26.6.2 arm64; repository Chromium matrix at 900×600,
  1152×640, 1232×700 and 1440×900.

`fixed (automated)` means executable component/browser evidence exists. It does
not imply installed macOS/Windows UAT.

| Finding | Current-main classification | Evidence / next owner |
| --- | --- | --- |
| A3 Search/Settings/Account inert | **fixed (automated)** | Focusable no-op controls were removed from `ToolCluster`; TopBar/browser keyboard tests verify Brand-only chrome. |
| Left selected state dims after right-side interaction | **fixed (automated); native confirmation pending** | Host CSS/ARIA remains visible without Lens; missing/delayed observer, fallback, reduced-motion and right-side interaction tests pass. |
| A4 Models sticky overlap | **not reproduced in browser matrix; native evidence gap** | Long Models flows and all viewports pass overflow/scroll checks. Installed WebView measurement remains. |
| A5 MCP last target / scroll affordance | **fixed (automated)** | Shared catalog/split panes and keyboard/pointer scenarios pass at minimum viewport. |
| A6 Prompts app rail discoverability | **fixed (automated)** | Rail selection, route unmount/return and dirty navigation pass component/browser coverage. Native wheel/trackpad feel remains unverified. |
| A7 duplicate search clear | **fixed (automated)** | `FeatureSearch` owns one clear action and Escape behavior; shared tests cover empty and non-empty states. |
| A8 corrected validation style | **current behavior covered; historical visual artifact not reproduced** | Models action/submission validation clears errors on corrected paths. Installed historical-pixel comparison was not run. |
| B4 Windows DPI/minimize/full keyboard | **evidence gap — still applies as acceptance work** | Chromium viewport/keyboard tests do not prove Windows 125/150% DPI, minimize/restore, multi-monitor or WebView2 focus. |
| B7 untouched model validation | **fixed (automated)** | Empty drafts do not surface submit errors on route mount; fetch/probe/save paths own validation and corrected paths clear it. |
| B8 Skills Discover header actions | **fixed (automated)** | Radix-backed Installed/Discover tabs retain reviewed chrome and keyboard semantics. |
| React `act(...)` warnings | **fixed** | Full V2 run: 58 files / 417 tests, zero warnings; an exact fail-fast guard rejects recurrence. |
| Prompt live-file and Daily Memory mixed-file blockers | **still outside Stage 5** | A1/A2/B1/A9 remain owned by focused data-safety remediation and installed-profile UAT. |

Issue #141 must remain open until its P1 data-safety work, Windows signing/HIL
owners, and new installed macOS/Windows candidate verdicts satisfy the umbrella
close conditions. Stage 5 does not substitute browser evidence for them.
