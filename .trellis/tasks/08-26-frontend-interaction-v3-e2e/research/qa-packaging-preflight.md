# QA and packaging preflight

## Evidence boundary

- Initial source: read-only Codex audit `01a039b6-f80f-7922-b364-57df6c9b1e91`, `gpt-5.6-sol/max`.
- Current branch refresh: `codex/frontend-interaction-v3-20260825` after planning and Memory implementation commits.
- This document distinguishes configuration/probe evidence from browser, packaged-app, Windows-native, and release evidence.

## Current facts

- FyAgent is a Tauri 2 desktop application. It is not Electron.
- macOS bundles are app + DMG. Windows uses native NSIS configuration and WebView2 provisioning.
- V2 quality gates are explicit and are not substituted by the legacy aggregate test:
  - `mise run lint:v2`
  - `mise run typecheck:v2`
  - `mise run test:v2`
  - `mise run test:v2:browser`
- Full repository closure additionally requires `mise run check` and a host-native `mise run build` after the UI surface freezes.
- Browser tests use fake Tauri IPC. They prove renderer interaction only, not WebView, filesystem, Keychain, installer, UAC, or Windows-native behavior.

## Resolved preflight findings

- `mise run bootstrap` now passes in the implementation worktree and installed the locked Node/Python/Rust dependencies.
- `mise run system:check --json` now reports `ok: true` for macOS host prerequisites.
- The Trellis task directory exists, is active, and has validated planning/spec artifacts.
- Playwright Chromium 151, headless shell, and FFmpeg are already present at the exact cache paths requested by the current Playwright package. No browser download is currently required.

## Runtime order after integration freeze

1. Run focused page tests while surfaces are mutable.
2. Freeze the integrated renderer surface.
3. Run V2 lint, typecheck, unit, and browser gates.
4. Run the full repository check.
5. Build the macOS host-native app/DMG.
6. Launch the newly built bundle with an isolated `FYAGENT_TEST_HOME`; do not use the installed 0.4.2 app as v3 evidence.
7. Capture representative 1232x700 and 900x600 runtime screenshots and compare only stable representative states.
8. Perform Windows candidate build/install/UAT last, against the exact candidate SHA.

## Windows acceptance boundary

- Tailscale or remote-desktop reachability is not Windows UAT.
- CI packaging is not installed-app UAT.
- Windows acceptance must record OS build, architecture, candidate commit, installer SHA256, signature state, DPI states, minimum/maximized window behavior, core interaction success/failure paths, and GO/NO-GO.
- The destructive install/uninstall lifecycle script belongs only on a clean disposable administrator VM because it refuses existing FyAgent registrations and may touch ProgramData. It must not be run casually on the daily AIMaster machine.
- A push, PR/main merge, tag, Release workflow, or production rollout remains a separate authorization/evidence boundary.

## Focused coverage

- Shell: three navigation groups, expansion semantics, active route, keyboard focus, keep-alive.
- Agents: first/re-scan, settled progress, complete/empty/error/unknown, stale response guard, four tabs, return and management handoff.
- Resources: Skills/MCP writer + invalidate + reread, model/prompt capability honesty, prompt unsupported states.
- Memory: current long-term/daily draft copied as text; save/open/delete and dirty-navigation contracts remain intact.
- One browser path: expand management, scan, enter Agent, switch section, enter a global management page, then return without losing the kept-alive state.

Do not add an 11-page x 4-viewport x 3-DPI Cartesian suite, fake scan cancellation tests, duplicate editor tests, or browser claims for native delivery.
