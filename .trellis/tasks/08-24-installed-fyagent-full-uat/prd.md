# Installed FyAgent 0.4.2 full-page UAT (macOS)

## Goal

Independently evaluate the currently installed FyAgent desktop application as a user, produce reproducible runtime evidence for every real V2 page and critical entry, and give a release decision without treating installation, code inspection, or a clickable control as functional acceptance.

This task is explicitly `platform=macOS`. A later Windows UAT on the AIMaster node may reuse this task's method and matrices, but must establish its own Windows evidence and verdict.

## Confirmed Baselines

- UAT object: `/Applications/FyAgent.app`, observed version `0.4.2`, bundle id `com.fyagent.desktop`.
- Installed application: Developer ID-signed, Gatekeeper accepted as `Notarized Developer ID`, universal `x86_64/arm64`, and running at preflight.
- Product/code baseline: `origin/main` commit `e94307cd810d7c5157b3791da2a8d7ef6a01b8a7` (2026-08-23). It is contextual authority only; it is not assumed to be byte-identical to the installed app.
- Work branch: `codex/installed-fyagent-uat-20260824`, created from the exact `origin/main` baseline in a separate worktree.
- Platform: `macOS` on Apple Silicon. No Windows runtime, installer, signature, page, or functional claim is made here.
- The source request is the locked UAT contract: no unresolved product, scope, UX, compatibility, or risk decisions remain.

## Requirements

### R1. Surface inventory

- Enumerate actual first-level pages, second-level entries, dialogs, settings, install/update/detail/confirmation flows, and state variants from the running app.
- At minimum cover Agent, Models, Skills, MCP, Prompts, Memory, and every additional visible V2 entry.
- Use current code only to explain or reproduce runtime behavior; do not infer runtime coverage from old documents.

### R2. Visual experience review

- For every page assess first-impression cognition, information hierarchy, layout, typography, spacing, color, density, readability, action/state discoverability, empty/loading/error/disabled states, resizing, minimum-size behavior, and cross-page consistency.
- Give a 10-point score with concrete rationale. Do not claim pixel parity without a pixel diff.

### R3. Functional UAT

- Operate each applicable critical control and cover happy path, empty/no-data state, cancel, repeated action, invalid input, write failure, and native-unavailable behavior.
- Distinguish `control_clicked`, `request_observed`, `persistence_observed`, and `authoritative_readback`.
- Never repair product code or hide defects in this task.

### R4. Prompt and memory safety

- Back up user-controlled FyAgent data before any potentially persistent interaction.
- Prefer isolated test labels and reversible targets. Never expose private prompt/memory bodies, tokens, secrets, or sensitive local paths in reports, commits, screenshots, or logs.
- If safe isolation or authoritative rollback cannot be established, test only read-only, cancel, validation, simulated, or failure behavior and record the exact blocker.

### R5. Evidence and findings

- Use evidence grades only from: `code_audit`, `runtime_screenshot`, `interaction_readback`, `UAT`, `pixel_diff`.
- Every finding records page/entry, reproduction, expected, actual, severity P0-P3, evidence id, impact, suggested owner, and release-blocking status.
- Retain sensitive raw screenshots only in a local untracked evidence directory; commit only safe derived metadata and evidence hashes.

### R6. Delivery and governance

- Deliver a page coverage matrix, per-page visual scores, functional pass/fail matrix, issue register, blockers, untested boundaries, overall `GO` / `CONDITIONAL GO` / `NO-GO`, and prioritized repair/retest plan.
- Run fresh repository validation, commit only UAT artifacts, push the UAT branch, and create a new PR to `main`.
- The PR must state installed version, environment, coverage, evidence levels, gaps, risks, and retest conditions. Do not merge, release, or modify `main`; explicitly require 赖永杰 to review and merge.

### R7. Windows reuse handoff

- Add a compact handoff for a later Windows Codex CLI experience reviewer on the AIMaster node.
- Identify which coverage/evidence structures are reusable and which Windows installation, signing, page, function, Agent-tool, storage, and failure-path checks require independent evidence.
- Explicitly prohibit extrapolating any macOS pass, score, count, or release verdict to Windows.

## Out of Scope

- Reinstalling or upgrading FyAgent without a fresh installation blocker.
- Repairing product code, historical PR governance, merging, releasing, or modifying `main`.
- Exposing, committing, or transmitting real private Agent configuration, memory content, prompt content, credentials, or secrets.
- Remotely operating AIMaster or performing Windows UAT in this task.
- Claiming Windows-native behavior, strict 1:1 visual parity, or production readiness without corresponding evidence.

## Acceptance Criteria

- [x] AC1: Installed-app and `origin/main` baselines are separately recorded with fresh evidence.
- [x] AC2: Runtime-derived inventory covers every visible first-level page, all applicable second-level entries/dialogs, and key state variants.
- [x] AC3: Every inventoried page has a concrete visual assessment and 10-point score.
- [x] AC4: Every applicable critical control has a result at the strongest actually observed layer, including negative/cancel/repeat paths where safe.
- [x] AC5: Prompts/Memory tests either use verified isolation and rollback/readback or document an exact safety blocker; no private body is exposed.
- [x] AC6: Findings are reproducible and evidence-linked with severity, impact, owner, and release decision.
- [x] AC7: Final report includes all required matrices, untested boundaries, verdict, prioritized fixes, and retest conditions.
- [ ] AC8: Task metadata and committed artifacts validate; branch is pushed and a new unmerged PR is created for 赖永杰 review.
- [x] AC9: Report declares `platform=macOS` and includes a Windows reuse handoff with explicit independent-verification boundaries.
