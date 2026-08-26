# A-to-A implementation audits

> SUPERSEDED_DO_NOT_EXECUTE（2026-08-26）：V3.1 任务已接管执行。本文仅保留历史证据，旧候选、Windows 等待与对外发送流程已停止。

## Evidence boundary

- Evidence level: `code_audit`.
- Both reviews were read-only. Neither reviewer changed files, ran tests, or produced runtime screenshots.
- Gemini route: `gemini-3.7-flash-high`, effort `high`, conversation `113aa0c5-fc60-4e3e-b29a-c38ea6bdc549`.
- Grok requested route: `grok-4.7/max`; this route is unavailable locally. The audit therefore used the newest verified local route `vibekey/grok-4.6/high`, session `ses_fc62a3b5affeei8KgosqmupGM6`. This is a disclosed fallback, not a claim that Grok 4.7 ran.

## Shared conclusions

1. The 11 approved images represent six existing routes and 11 states, not 11 pages or nested routes.
2. Keep `/agents`, `/models`, `/skills`, `/mcp`, `/prompts`, `/memory` and `PersistentPrimaryOutlet` keep-alive behavior.
3. Build the left shell first, then `/agents` scan/directory, then the four Agent sections, then align the five management/memory surfaces.
4. Aggregate the seven existing Agent Install Readiness queries. Settled queries define progress; `unknown` must remain distinct from not installed.
5. Do not add a scan-cancellation protocol, background daemon, global selected-Agent store, generic model-assignment port, second assignment store, nested Agent routes, or a new UI kit/theme.
6. Skills and MCP must use the existing assignment writers followed by invalidation and authoritative reread.
7. Models are capability-aware projections into their existing management owners. Prompts are writable only when a `PromptAppId` exists.
8. Memory `复制` copies the current memory text, not a file path.

## Identity mapping acceptance gate

| Agent | scan `agentId` | Skills/MCP `assignmentId` | model `target` | `promptAppId` |
| --- | --- | --- | --- | --- |
| QoderWork CN | `qoderwork` | `qoderwork` | `qoderwork` | unsupported |
| TRAE Work CN | `trae-work` | `trae-work` | `trae` | unsupported |
| WorkBuddy | `workbuddy` | `workbuddy` | `workbuddy` | unsupported |
| Grok Build | `grokbuild` | `grokbuild` | `grokbuild` | `grokbuild` |
| Codex | `codex` | `codex` | `codex` | `codex` |
| Claude Code | `claude-code` | `claude` | `claude` | `claude` |
| OpenCode | `opencode` | `opencode` | `opencode` | `opencode` |

## Gemini visual and interaction gates

- Side navigation uses three groups, retains readable labels at the 900x600 minimum viewport, exposes `aria-expanded` and `aria-current`, and preserves visible keyboard focus.
- The 1232x700 reference viewport uses a roughly 240px navigation column. At narrower widths, labels remain visible; do not collapse to an icon-only rail.
- Scanning exposes progress and disabled duplicate actions but no fake cancel success.
- Agent configuration uses `?target=<agentId>&section=models|skills|mcp|prompts`, a visible back-to-directory action, Agent identity, four tabs, and a truthful management-page handoff.
- Respect `prefers-reduced-motion` for scanning and ambient effects.
- Do not expose or log unmasked API secrets while aligning model management.

## Grok complexity challenges

- Use page-local state plus a request-generation guard; do not introduce a shared scan reducer/framework.
- Do not embed the full Models editor, Quick Setup, or Change Plan forms in an Agent tab.
- Do not introduce optimistic success for Skills/MCP; preserve write, invalidate, reread.
- Replace obsolete shell/Agent expectations instead of adding a parallel test suite.
- Keep the focused contract set to roughly five tests plus one cross-page browser path.

## Codex adjudication

The frozen implementation plan stands. Where the reviewers differ in wording, the simpler existing-contract path wins: page-local scan state without a new reducer abstraction, management-owner handoff instead of duplicated editors, and existing design tokens rather than new utility-framework conventions.
