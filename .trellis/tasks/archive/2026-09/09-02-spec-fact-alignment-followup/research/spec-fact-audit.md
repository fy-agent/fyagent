# Focused Spec Fact Audit — 2026-09-02

## 1. Scope and method

This follow-up first rechecks seven focused contracts changed after the
archived comprehensive refresh. A subsequent full-library symbol pass also
found the V2 Models contract using fictional aggregate Port names, so that
eighth cross-layer contract was added to the fact-alignment scope. Evidence is
the current checkout at `af1f4835` plus the uncommitted documentation work.
Product source has not changed since the refresh commits, so command, service,
Port and test behavior remains the authority.

The review compares each durable statement with the exact implementation owner
and records both implemented guarantees and current limitations. A limitation
is documented when removing it would require product code or test changes;
SPEC text must not silently promote the desired behavior into current fact.

## 2. Backend findings

### Skill management

Evidence:

- `src-tauri/src/commands/skill.rs`
- `src-tauri/src/services/skill.rs`
- `src-tauri/src/services/skill/assignment.rs`
- `src-tauri/src/app_config.rs`
- `src-tauri/src/database/dao/skills.rs`

Findings:

- Native `SkillTargetId` has nine values; the V2 UI intentionally presents the
  seven catalog-aligned targets.
- `toggle_skill_app` returns `Ok(true)` after `SkillService::toggle_target`
  succeeds. The service performs the live target effect before the SQLite flag
  update, so a late database failure can leave divergent state.
- Uninstall has a recovery exception for a stored row whose `directory` is
  invalid: it skips every managed/source/target filesystem operation, creates
  no backup, deletes only the database row, and returns success. This is not
  the same contract as accepting an invalid directory from a new external
  request.
- `backupPath` is optional because no safe source may exist; the renderer must
  not promise recovery merely because uninstall succeeded.

Disposition: keep the focused contract and correct the dirty-row cleanup
matrix and tests.

### MCP management

Evidence:

- `src-tauri/src/commands/mcp.rs`
- `src-tauri/src/services/mcp.rs`
- `src-tauri/src/mcp/validation.rs`
- `src-tauri/src/mcp/**`
- `src-tauri/src/database/dao/mcp.rs`
- `src-tauri/src/commands/traework.rs`

Findings:

- The unified external preflight command is
  `validate_external_mcp_config(agentId, config)` and is separate from CRUD.
- `McpServer.server` is deserialized as `serde_json::Value`; the Tauri command
  and `McpService::upsert_server` do not perform centralized server-spec
  validation before saving SQLite.
- Upsert removes newly disabled live entries, saves the row, and then projects
  enabled targets. Target adapters call `validate_server_spec` while parsing or
  writing. A projection validation/write failure can therefore occur after the
  durable row was saved. With no enabled targets, a direct native caller can
  persist a value that has not passed an adapter validation.
- Enable toggle updates SQLite before target projection; disable removes the
  live entry before updating SQLite. Delete removes all live entries before
  deleting the row. These are intentionally different non-atomic boundaries.
- The ordinary management DTO contains raw `env` and header values because the
  existing server editor needs them. External preflight returns closed findings
  without those values. Ordinary list/detail, errors, logs and analytics must
  redact them, but the editing query/draft is not currently SecretRef-backed or
  write-only.

Disposition: remove the false “always validate before persistence” guarantee,
document the actual ordering, and distinguish editing from ordinary exposure.

## 3. Frontend findings

### Shared assignment

Evidence:

- `src/v2/shared/features/directory.ts`
- `src/v2/shared/features/assignments.ts`
- `src/v2/shared/features/authoritative-assignment.ts`
- `src/v2/shared/platform/tauri/feature-ports/simple.ts`

Findings:

- The shared seven-target order is one TypeScript closed tuple used by Skills
  and MCP.
- `useAuthoritativeAssignmentMutation` serializes calls with a ref, treats an
  explicit `false` as rejection, and confirms only an exact authoritative
  reread.
- `createSimpleFeaturePorts` is a thin compile-time-typed adapter. It does not
  runtime-parse Skill/MCP target IDs before IPC. Normal typed call sites cannot
  construct an unknown ID; a value forced through the type boundary is sent to
  native code, whose enum parser rejects it.

Disposition: keep the strict helper contract but correct claims that the simple
Port itself has a runtime target parser.

### Skills page

Evidence:

- `src/v2/pages/skills/Page.tsx`
- `src/v2/shared/features/ports.ts`
- `src/v2/shared/platform/tauri/feature-ports/simple.ts`

Findings:

- The management page uses one write lock and query invalidation; it does not
  use the authoritative assignment helper.
- The page awaits `toggleApp` but ignores its boolean value. Current native
  `toggle_skill_app` returns `true` on success and throws on failure, so this is
  observable only if the Port contract changes or a test double returns false.
- Sequential bulk treats only thrown operations as failures; a resolved false
  currently counts as success. Agent-bound assignment remains stricter.
- Uninstall backup evidence is optional and installed paths intentionally enter
  the renderer for explicit `CopyablePath` display.

Disposition: document the exact current success channel and add a change rule
for any future meaningful false result.

### MCP page

Evidence:

- `src/v2/pages/mcp/Page.tsx`
- `src/v2/shared/features/helpers.ts`
- `src/v2/shared/features/mcpSecurity.ts`
- `src/v2/shared/platform/tauri/feature-ports/simple.ts`

Findings:

- The editor intentionally receives raw env/header values. Ordinary detail and
  search redact/exclude them; a failed save keeps the editing draft mounted.
- Quick mode validates its known fields locally. Advanced mode guarantees only
  one JSON object and no top-level `mcpServers` wrapper; unknown and incorrectly
  typed known fields can reach native upsert.
- New servers default all seven target flags on. Native upsert saves before
  target projection validation, so an adapter failure may leave a persisted row
  while the page reports an error and refetches. No current contract may claim
  centralized pre-save validation or rollback.

Disposition: preserve open advanced fields, but state the real native timing
and failure/readback result.

### Agent Auth

Evidence:

- `src/v2/shared/features/agent-auth.ts`
- `src/v2/shared/platform/tauri/feature-ports/agentAuth.ts`
- `src/v2/pages/agents/useAgentAuthSession.ts`
- `src/v2/pages/agents/AgentAuthStatusPanel.tsx`

Findings:

- All five responses are strict exact-key/version/closed-enum parses.
- Observation and active-session reads also verify the returned `agentId`
  against the requested Agent. `startSession` validates the request ID and
  strictly parses the response, but currently does not perform the same second
  equality check. `getSession`/`stopWaiting` are session-ID addressed.
- A non-terminal poll failure preserves the last snapshot, surfaces an error,
  and schedules another poll. Terminal callbacks are deduplicated by session
  ID in the panel.

Disposition: keep strict parsing guarantees but do not claim an implemented
`startSession` response-binding check.

### Agent Directory

Evidence:

- `src/v2/shared/features/agent-install-readiness.ts`
- `src/v2/pages/agents/useAgentLifecycleAction.ts`
- `src/v2/shared/platform/tauri/feature-ports/agentInstallReadiness.ts`

Finding: `AgentInstallationTarget` owns `expectedTargetRevision`; it has no
generic `revision` field. The SPEC example must forward
`selectedTarget.expectedTargetRevision`.

### Models

Evidence:

- `src/v2/pages/models/**`
- `src/v2/shared/features/ports.ts`
- `src/v2/shared/platform/tauri/feature-ports/models.ts`
- `src/v2/shared/platform/tauri/feature-ports/changePlans.ts`
- `src/v2/shared/platform/tauri/feature-ports/qoderTrae.ts`

Findings:

- No aggregate `ModelPorts` or `ChangePlanPorts` exists. `FeaturePorts` owns
  `providers`, `workbuddy`, `opencodeModels`, `traeWork`, and `changePlans`.
- Claude/Grok Build use direct provider apply plus summary reread; Codex and
  normal WorkBuddy save use Change Plans; OpenCode and WorkBuddy delete use
  their own revision/overwrite protocols.
- QoderWork is explicitly unsupported on the current route. TRAE is read-only
  there even though shared/native validation and probe capabilities exist for
  other flows.
- Runtime response parsing is mixed and must be documented per focused Port,
  rather than promoted into a false route-wide guarantee.

## 4. Unchanged conclusions

- The compatibility routers and focused information architecture from the
  comprehensive refresh remain valid; no new split or merge is needed.
- `frontend/v2-agent-auth.md` and `frontend/v2-agent-directory.md` remain the
  correct semantic owners after the narrow factual fixes.
- The archived comprehensive task checklist update only marks commit/archive/
  reporting steps that were already completed; it should remain with this
  follow-up rather than rewriting the archived task history elsewhere.

## 5. Validation targets

- exact seven-section structure for every focused cross-layer contract,
  including the later Models correction;
- no broken relative link or unindexed focused spec;
- every cited repository path exists;
- focused Port/helper/page tests for Agent Auth, Assignments, Skills and MCP;
- `git diff --check`, `mise run check:contracts`, and exact prearchive gate.
