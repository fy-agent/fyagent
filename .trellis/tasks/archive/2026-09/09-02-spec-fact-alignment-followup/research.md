# Fact-alignment review

## Why this follow-up exists

The comprehensive refresh correctly split large retrieval domains and archived
its task, but an independent post-archive source pass found that several new
documents had preserved design-intent names instead of the interfaces actually
compiled in the checkout. Because code-spec is executable institutional memory,
these were correctness defects rather than editorial nits.

## Source-backed findings

### Native Skill and MCP ordering is not one atomic transaction

- Skill target toggling performs the live target effect before updating SQLite.
  Target failure leaves the flag unchanged; a later database failure can leave
  live and durable state divergent.
- A historical Skill row whose stored directory is itself unsafe has one
  recovery-only uninstall path: no filesystem/backup/target path is resolved;
  only the poisoned SQLite row is removed.
- MCP enable updates SQLite before the live file, while disable removes the live
  file before updating SQLite. Upsert and delete have their own explicit order.
- Unified MCP upsert stores `serde_json::Value` before enabled target adapters
  validate/project it. An all-disabled row can therefore persist without
  adapter validation, and an enabled target can fail after the row was saved.
- Specs must expose those asymmetries so callers do not invent rollback or
  treat a reread durable flag as proof that a vendor file/reload succeeded.

### V2 assignment and Auth have concrete shared owners

- `AgentAuthPort` is singular and exposes `getObservation`,
  `getActiveSession`, `startSession`, `getSession`, and `stopWaiting`.
- Observation and active-session calls bind returned `agentId` to the request;
  the current `startSession` adapter strictly parses the snapshot but does not
  add a second response/request Agent equality check.
- `useAgentAuthSession` owns recovery and polling. Terminal callbacks can be
  reached through several lifecycle paths, so consumers deduplicate by
  `sessionId`.
- `useAuthoritativeAssignmentMutation` serializes one pending item, confirms
  only after exact reread, and performs one best-effort failure reread. It does
  not maintain optimistic assignment state or promise rollback.

### Management pages and Agent assignment pages have different confirmation

- Agent Skill/MCP sections use the authoritative assignment helper.
- Skills and MCP management pages instead serialize writes at page scope and
  invalidate/refetch their queries after each terminal operation. MCP toggle
  returns `void`; Skill toggle returns a boolean. Neither page receives an
  authoritative resource snapshot from the command itself.
- The current Skills management page ignores a resolved `false` from
  `toggleApp`; native currently returns `true` on success and throws on failure.
  A meaningful non-throwing false result therefore requires a coordinated
  page/Port/test change.
- The simple Skill/MCP Tauri adapter is compile-time typed only. It does not
  runtime-parse/version those DTOs today.

### Sensitive renderer state must be described honestly

- MCP `env` and `headers` are present in query/editor state so an existing
  server can be edited. Ordinary detail/search redacts or excludes them, but
  the renderer boundary is not currently `SecretRef`-based.
- Skill detail intentionally exposes a user-copyable local path; the target
  destination string is only a UI preview and never native path authority.
- Model API keys live in mounted draft state/ref, survive fetch/probe for later
  save, and clear at the owning terminal/current-revision boundary or unmount.

### Models is a family of protocols, not `ModelPorts`

- `FeaturePorts` owns `providers`, `workbuddy`, `opencodeModels`, `traeWork`,
  and `changePlans`; no aggregate `ModelPorts` exists.
- Claude and Grok Build use direct provider apply plus summary reread.
- Codex and normal WorkBuddy saves use typed Change Plans.
- WorkBuddy delete is a direct revisioned save; OpenCode writes are direct
  revision/overwrite-token transactions.
- QoderWork is unsupported in the current route. TRAE is read-only there even
  though shared/native validation and probe capabilities exist elsewhere.

## Structural review result

- 64 current Spec files, 15,414 lines after fact alignment.
- Zero broken relative links, unreachable Specs, missing cited repository paths,
  or substantial cross-file duplicate paragraphs.
- Every newly split infra/cross-layer contract has the seven mandatory sections.
- Compatibility files are 28–37-line routers rather than second authorities.
- Six documents remain over 600 lines only where ordered installer, provider,
  CI/release, task-runner, or Windows security semantics remain one cohesive
  failure domain. Splitting them by line count would increase retrieval risk.

## Validation evidence

- Final `mise run check:contracts`: passed, including 596 passed / 1 skipped
  contract tests and 4 native-fetch contract tests.
- Final `mise run test:v2`: passed, 65 files and 474 tests.
- Structural scanner: 0 broken links, 0 unreachable files, 0 missing paths,
  0 duplicate blocks, 0 seven-section failures for the focused contract set.
- `git diff --check`: passed.
- Final exact prearchive with
  `--exclude-active-task .trellis/tasks/09-02-spec-fact-alignment-followup`:
  passed under the explicit session context ID.

Lifecycle commits are recorded in `implement.md` and the archived task record.
