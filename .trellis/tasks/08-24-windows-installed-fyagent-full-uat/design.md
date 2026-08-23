# Windows UAT Design

## Ownership and Handoff

AIMASTER Windows Codex is the sole executor for runtime actions and Windows claims. Mac seeded only the branch and task contract. Coordination is asynchronous through Git commits and the final PR; remote desktop is not the execution channel after task submission.

## Evidence Flow

1. Claim the task and record a sanitized machine receipt.
2. Freeze installed-app and repository baselines separately.
3. Enumerate source-declared and runtime-visible surfaces independently.
4. Create a private raw-evidence directory and an isolated copy of the FyAgent profile.
5. Capture stable runtime screenshots and interaction/readback evidence at each DPI/state.
6. Record sanitized evidence ids, timestamps, SHA-256 hashes, methods, and observations in Git.
7. Derive page scores, functional layers, findings, verdict, and retest plan only from Windows evidence.

## Safety Contracts

- No real private configuration or credential reaches Git or chat.
- No persistent write happens without known target, pre-state fingerprint, backup, rollback, and post-state readback.
- A click never upgrades to persistence or authoritative readback without direct evidence.
- Unsafe-to-induce cases are `NOT TESTED` with reasons, not PASS.
- Source changes after runtime capture do not retroactively validate screenshots.
- The old remote control-plane path is out of scope and must remain untouched.

## Evidence Grades

Use only `code_audit`, `runtime_screenshot`, `interaction_readback`, `UAT`, and `pixel_diff`. Pixel parity is not claimed without a canonical target and actual diff.

## Rollback

The Git branch/worktree is isolated from other work. UAT data writes are limited to the copied profile and must have verified rollback. The installed app and user profile are not upgraded, repaired, or overwritten by this task.
