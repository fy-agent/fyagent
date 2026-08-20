# V2 Prompt and Memory native business integration

## Goal

Replace the V2 Prompts and Memory local-only prototypes with truthful native
management surfaces that reuse the visual and interaction language already used
by Agents, Models, Skills, and MCP. The result must operate on the repository's
existing Prompt, OpenClaw memory, and Hermes memory commands without adding a
second persistence model or claiming unsupported cross-tool synchronization.

## Confirmed Baseline

- The working branch is `dev/laiyongjie` and was clean when planning began.
- `codex/prompt-memory-v2-main-pr` is pinned at
  `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab` and is already an ancestor of the
  working branch through merge `33216297`; no redundant merge commit is needed.
- The other `codex/*` branches are outside this task.
- The pre-change V2 baseline passed `mise run typecheck:v2` and
  `mise run test:v2` (22 files, 168 passed, 1 skipped).

## Requirements

### Shared product and safety requirements

- Prompts and Memory must use the shared V2 feature layout, controls, semantic
  tokens, loading/empty/error states, and responsive behavior rather than their
  page-specific deep-blue prototype theme.
- Native effects must remain behind V2 feature ports. V2 pages must not import
  legacy renderer hooks/APIs or Tauri directly.
- Browser preview must expose an explicit desktop-only/unavailable state and
  must not seed simulated Prompt or Memory records.
- Opening either page must be read-only. Importing, enabling, saving, creating,
  and deleting occur only after an explicit user action.
- No dependency, database schema, persisted-file-format, Tauri command, or
  permission expansion is allowed.
- User content must not be printed to logs, embedded in the standalone preview,
  or captured in review screenshots.

### Prompts requirements

- Manage Prompts independently for Claude, Codex, Gemini, Grok Build, OpenCode,
  OpenClaw, and Hermes; Claude Desktop remains unsupported.
- Default to Claude without persisting a new app-selection setting.
- Support authoritative list/search, create, edit, delete, enable, disable,
  import-from-live-file, and current-live-file reads through existing commands.
- Preserve the backend's single-enabled-Prompt-per-application behavior. Do not
  reproduce the prototype's one-rule-to-many-app assignment model.
- Block duplicate writes and protect dirty editor state across app switches and
  route navigation with the shared confirmation UI.
- Distinguish loading, empty, no-results, native-unavailable, read error, write
  error, enabled-item deletion rejection, and refresh-after-write failure.

### Memory requirements

- Manage exactly four long-term resources: OpenClaw `MEMORY.md`, OpenClaw
  `USER.md`, Hermes `MEMORY.md`, and Hermes `USER.md`.
- Missing OpenClaw resources remain missing until the user explicitly saves.
- Hermes resources expose their real enable flags and character budgets;
  over-budget content is warned about but remains saveable, matching Hermes.
- Manage OpenClaw daily memory files through authoritative list, debounced
  search, read, create-today-on-save, edit, delete, and open-directory actions.
- Protect dirty content across document, tab, daily-file, and route changes.
- Remove prototype-only cross-tool scans, fake session counts, sync targets,
  promoted drafts, revisions, and pending task simulations.

## Acceptance Criteria

- [x] `git merge-base --is-ancestor codex/prompt-memory-v2-main-pr HEAD`
      succeeds and no redundant merge commit is introduced.
- [x] Both pages use shared `fy-feature-*`/`fy-control-*` components and tokens;
      their prototype-specific datasets and deep-blue CSS are removed.
- [x] Prompts performs all approved operations against the correct application
      port and reflects authoritative rereads without cross-app data leakage.
- [x] Memory performs all approved long-term and daily operations against the
      exact allowed resources; it offers no arbitrary path or filename input.
- [x] No prototype success wording, fake counts, cross-tool session/sync UI, or
      seeded browser data remains.
- [x] Loading, empty, error, busy, disabled, dirty, native-only, missing-file,
      and Hermes over-budget states are accessible and test-covered.
- [x] Focused adapter tests prove command names, payloads, runtime parsing, and
      rejection of invalid application/resource/file identifiers and payloads.
- [x] Unit tests with injected stateful ports cover Prompt and Memory business
      flows, failure recovery, authoritative refresh, and dirty-state guards.
- [x] Browser tests pass at 900x600, 1152x640, 1232x700, and 1440x900 with no
      overflow, overlap, console error, hidden critical control, or fake data.
- [x] The generated standalone preview is deterministic and contains no native
      user data or external entry request.
- [x] `mise run lint:v2`, `typecheck:v2`, `test:v2`, `test:v2:browser`,
      `build:renderer`, `test:desktop:mock`, `test:desktop:visual:preflight`, and
      the final `mise run check` pass on the resolved tree.
- [x] A real Windows Tauri read-only smoke verifies both routes and their shared
      visual language, or the exact native limitation is documented.

## Out of Scope

- Cross-tool memory/session discovery or synchronization for Codex, Claude,
  Gemini, or OpenCode.
- Multi-application Prompt replication, batch rollback, or shared Prompt IDs.
- New backend commands, storage migrations, dependencies, permissions, remote
  pushes, pull requests, or changes from other `codex/*` branches.
