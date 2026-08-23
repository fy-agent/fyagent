# Installed FyAgent full-page UAT (Windows AIMASTER)

## Goal

Run an independent, end-to-end UAT on the FyAgent build actually installed and running on the AIMASTER Windows computer. Produce Windows-native visual, functional, failure-path, privacy-safe evidence and an explicit release verdict. Execution happens locally in Windows Codex; coordination with Mac happens only through this Git branch, task artifacts, commits, and a dedicated PR.

## Locked Boundaries

- Platform claim is `Windows` only. Never copy or extrapolate a macOS pass, score, page count, signing result, or release verdict.
- The installed/running Windows FyAgent is the system under test. Source inspection may explain behavior but cannot replace runtime evidence.
- Do not repair product code in this task. Findings become reproducible issues and retest conditions.
- Do not merge `main`, publish a release, reinstall, or upgrade unless a fresh installation blocker makes that unavoidable and the user explicitly approves it.
- Do not print, copy, transmit, or commit credentials, tokens, private prompt/memory bodies, or identifying paths.
- Stop and ignore earlier OpenClaw Gateway/bootstrap/triad work. Do not modify Gateway, Tailscale Grants, firewall, SSH/WinRM, pairing, or EverOS `durable_mode`.

## Machine Acceptance Receipt

Before deep execution, reply in the Windows Codex task and commit a sanitized receipt containing:

1. hostname, Windows version/build, and architecture;
2. real repository absolute path, current SHA, branch, and worktree state;
3. installed FyAgent version, executable path, launch state, and PID;
4. this task card path and task id;
5. actual model and reasoning tier;
6. GUI evidence capture method;
7. any missing item as an exact blocker, while continuing safe discovery.

Then update `task.json` to `status=in_progress`, set `ClaimedBy`, `ClaimedAt`, and append a claim entry to `progress_log` before UAT execution.

## Requirements

### R1. Windows runtime baseline and provenance

Record Windows build/architecture, installed FyAgent version and source, package hash when verifiable, Authenticode chain, install scope, executable path, process, updater, uninstall, and rollback boundaries. Preserve any dirty worktree and other agents' changes.

### R2. Complete runtime surface inventory

Independently enumerate every visible first-level page, secondary entry, Settings/Search/Account surface, detail/install/update/confirmation/error dialog, and state family. At minimum cover Agent, Models, Skills, MCP, Prompts, and Memory; add every additional actual surface.

### R3. Visual experience review

For every page assess first impression, hierarchy, layout, typography, spacing, color, density, readability, action/state discoverability, empty/loading/error/disabled states, and cross-page consistency. Test 100%, 125%, and 150% DPI plus normal/minimized/maximized windows, native title bar and controls, scrollbars, keyboard focus, clipping, long Chinese text, contrast, and multi-monitor scaling when available. Give each page a concrete 10-point score. Mark unavailable multi-monitor evidence as `NOT TESTED`, never inferred.

### R4. Functional and failure-path UAT

Separate evidence layers: `C=control clicked`, `R=request observed`, `P=persistence observed`, `A=authoritative readback`. Cover happy path plus repeat-click, Cancel, invalid, empty, loading, error, disabled, native-unavailable, and write-denied behavior. Add Windows-specific cases for drive/profile boundaries, separators, long paths, case collisions, CRLF/LF, junction/symlink, file locks, ACL/UAC denial, safe Defender boundary, atomic replace/rollback, tray, deep-link, updater, and native bridge unavailable.

### R5. Agent tool inventory

Inspect actual Windows-local Agent integrations and tools, including FyAgent, GrokBot, Codex, and every additional discovered tool. Record real version, status, executable/source where safe, detection behavior, update entry, and relationships to Models/Skills/MCP. Do not reuse the macOS tool list.

### R6. Isolated Prompt and Memory safety checks

Use a copied/isolated Windows FyAgent profile. Before any write, record privacy-safe pre-state fingerprints and a verified backup/rollback path; after each write, record authoritative post-state and rollback fingerprints. Safely retest two macOS-origin hypotheses without treating them as Windows facts: (a) with no enabled DB prompt, creating/importing a disabled prompt may clear the live prompt file; (b) Daily Memory containing both a valid date Markdown file and a non-date Markdown file may fail the whole page and Retry. Real user data must remain unchanged.

### R7. Evidence and delivery

Keep raw screenshots in a private untracked Windows evidence directory. Git receives only sanitized observations, capture metadata, and SHA-256 hashes. Deliver `uat-report.md`, `evidence-index.md`, `issue-register.md`, page coverage/score and functional matrices, tested/untested boundaries, GO/CONDITIONAL GO/NO-GO, suggested owners, prioritized fixes, and retest conditions. Run fresh task/contract checks, commit, push this branch, create a dedicated PR to `main`, and report branch, exact commit, report paths, PR URL, CI status, and remaining blockers. The PR must request 赖永杰 review and merge; do not merge it yourself.

## Acceptance Criteria

- [ ] AC1: Machine receipt exists with all required fields or exact blockers; task is claimed by AIMASTER Windows Codex.
- [ ] AC2: Installed-app and repository baselines are independently recorded with fresh Windows evidence.
- [ ] AC3: Every actual runtime page/entry/dialog and key state has coverage status, visual assessment, and 10-point score.
- [ ] AC4: 100/125/150% DPI and required window/keyboard/scroll/clipping checks have runtime evidence or explicit `NOT TESTED` reasons.
- [ ] AC5: Applicable controls have the strongest actually observed C/R/P/A layer and safe negative-path coverage.
- [ ] AC6: FyAgent, GrokBot, Codex, and all additional detected Windows Agent tools have independent inventory evidence.
- [ ] AC7: Both macOS-origin P1 hypotheses are safely retested in an isolated copied profile without touching real user data.
- [ ] AC8: Findings are reproducible, evidence-linked, severity-ranked, owner-suggested, and tied to release/retest decisions.
- [ ] AC9: Sanitized report, evidence index, issue register, verdict, gaps, and verification results are committed and pushed.
- [ ] AC10: A dedicated unmerged PR exists for 赖永杰 review; no Windows conclusion is inherited from macOS.
