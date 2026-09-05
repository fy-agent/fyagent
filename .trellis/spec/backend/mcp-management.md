# MCP Management Contract

## 1. Scope / Trigger

Read this contract before changing MCP server CRUD, server-spec validation,
application assignment, import, SQLite persistence, vendor live-file
projection, or the QoderWork/TRAE external MCP preflight surface.

Primary owners are:

- `src-tauri/src/commands/mcp.rs` for unified and compatibility Tauri commands;
- `src-tauri/src/services/mcp.rs` for CRUD, assignment ordering, locking,
  import, and multi-target synchronization;
- `src-tauri/src/mcp/**` for target-specific parse/write/remove adapters and
  shared server-spec validation; private `mcp/json_document.rs` owns the common
  QoderWork/TRAE Work/WorkBuddy JSON document read/backup/write mechanics;
- `src-tauri/src/database/dao/mcp.rs` for the durable row and nine assignment
  flags;
- `src-tauri/src/commands/traework.rs` and
  `src-tauri/src/services/traework.rs` for non-executing Qoder/TRAE external MCP
  validation.

Renderer behavior is owned by [V2 MCP](../frontend/v2-mcp.md) and
[V2 Shared Assignment](../frontend/v2-assignments.md). SQLite lifecycle is
owned by [Database Persistence](./database-persistence.md).

## 2. Signatures

The native `McpTargetId` domain is closed and contains nine IDs:

```text
claude | codex | gemini | grokbuild | opencode | hermes |
qoderwork | trae-work | workbuddy
```

The V2 presentation subset is the seven catalog-aligned targets documented by
[V2 Shared Assignment](../frontend/v2-assignments.md). QoderWork, TRAE Work,
and WorkBuddy are direct target IDs rather than `AppType` conversions.

The current unified Tauri commands are:

```text
get_mcp_servers() -> IndexMap<String, McpServer>
upsert_mcp_server(server: McpServer) -> ()
delete_mcp_server(id) -> bool
toggle_mcp_app(server_id, app, enabled) -> ()
import_mcp_from_apps() -> usize
```

The shared native DTO is:

```text
McpServer {
  id, name,
  server: serde_json::Value,
  apps: McpApps,
  description?, homepage?, docs?, tags[]
}
```

The target-adapter validator in `src-tauri/src/mcp/validation.rs` accepts these
connection shapes when a live source is parsed or an enabled target is
projected:

```text
type omitted or "stdio" -> non-empty command
type "http"             -> non-empty url
type "sse"              -> non-empty url
```

External Qoder/TRAE preflight is a separate command and is not the CRUD write:

```text
validate_external_mcp_config(
  agent_id: qoderwork | trae-work,
  config: JSON value,
) -> TraeMcpValidationResult
```

`get_claude_mcp_status`, `read_claude_mcp_config`,
`upsert_claude_mcp_server`, `delete_claude_mcp_server`,
`validate_mcp_command`, `get_mcp_config`,
`upsert_mcp_server_in_config`, `delete_mcp_server_in_config`, and
`set_mcp_enabled` are compatibility surfaces. New V2 work uses `McpPort` and
the unified commands.

## 3. Contracts

### Validation and canonical data

- The unified Tauri DTO deserializes `McpServer.server` as
  `serde_json::Value`. `upsert_mcp_server` and `McpService::upsert_server` do
  not currently run one centralized `validate_server_spec` before the SQLite
  write.
- Live-source parsers and target adapters apply `validate_server_spec` before
  accepting or serializing their target representation. A non-object value,
  unknown connection type, missing stdio `command`, or missing HTTP/SSE `url`
  is therefore rejected at that adapter boundary, which may be after an upsert
  has already saved the durable row.
- A direct native upsert with every target disabled can currently persist a
  `server` value that no target adapter has validated. Adding a global
  pre-persistence validator is a product-contract change: it needs compatibility
  fixtures for existing rows and tests for the upsert ordering before this spec
  may claim fail-closed validation at the command boundary.
- A missing `type` is normalized semantically as stdio for validation and
  equivalence; it is not permission to discard unknown executable fields.
- Import equivalence removes only representation differences reviewed by
  `server_specs_are_equivalent` (`type=stdio`, empty args/env/headers/cwd, and
  matching `headers`/`http_headers`). Commands, arguments, environment, URLs,
  headers, and unknown fields otherwise remain exact.
- Source-side enablement is active unless the imported entry contains the
  explicit boolean `enabled: false`.
- SQLite is the durable FyAgent catalogue and stores all nine assignment flags.
  Vendor files remain independent execution surfaces and are never assumed to
  be transactionally coupled to SQLite.

### CRUD and assignment ordering

- `upsert_server` acquires the Codex/provider writer lock, removes every target
  disabled relative to the previous row, saves the new row, and then projects
  the server to every enabled target.
- Disabled-target removal therefore happens before the database says disabled.
  Enabled-target projection happens after the database save. A later projection
  validation or write failure can leave the row enabled while one live file is
  stale/missing; the command returns error and callers must present the result
  as unconfirmed rather than claiming rollback.
- `toggle_target(..., true)` acquires the target writer lock, updates the
  database flag, then writes the target live file. A live-write failure can
  leave the database flag true; this is a known non-atomic boundary, not an
  implicit rollback contract.
- `toggle_target(..., false)` removes the live entry first and updates the
  database flag only after removal succeeds. A failed removal must not make the
  UI/database claim the server is disabled.
- `delete_server` removes the server from every previously enabled live target
  before deleting the durable row. If any owned live removal fails, keep the
  row so the operation remains discoverable and retryable.
- Callers reread `get_mcp_servers` after mutation. A reread confirms FyAgent's
  durable assignment state only; the successful command result is also needed
  to claim that the requested live projection completed.

### Target adapters, import, and locks

- Target-specific syntax belongs only under `src-tauri/src/mcp/**`. Generic
  services pass the stored semantic server; each target adapter validates the
  shape it is about to parse/write. Pages and shared components do not
  serialize Claude JSON, Codex TOML, or vendor-specific files.
- Codex MCP and provider settings share `config.toml`; all read-modify-write
  paths use the same proxy/provider switch lock and preserve unrelated valid
  fields.
- QoderWork, TRAE Work, and WorkBuddy adapters resolve their native roots,
  preserve unrelated entries, create the adapter-owned backup where specified,
  and skip creating a brand-new vendor file when the product/root is absent.
  They delegate `mcpServers` JSON mechanics to private `json_document` rather
  than copying parsers or writers. WorkBuddy's hidden-file import fallback and
  QoderWork's `streamable-http` import normalization remain adapter policy.
- `json_document::read_servers(path)` preserves existing missing/non-object-map
  read behavior. `write_servers(path, backup, root_error, servers)` validates the
  root and every server object before backup or mutation, preserves unrelated
  root and executable fields, and removes only top-level FyAgent metadata:
  `enabled`, `source`, `id`, `name`, `description`, `tags`, `homepage`, `docs`.
  Nested fields with the same names are not metadata and must survive.
- The common writer uses the existing config reader, `serde_json` pretty
  serialization, exact-byte backup, and `config::atomic_write`. Preserve JSON
  key ordering; do not replace it with the generic recursively sorted writer.
  Invalid JSON/root/server or backup failure must not overwrite the original.
  This document owner adds no cross-file transaction or symlink guarantee and
  does not change the unified upsert validation boundary described above.
- Import parses each supported live source independently and commits a batch
  atomically per target. The same server ID with materially different
  executable specs is not silently merged across sources.
- `sync_all_enabled` is best-effort across independent target files and reports
  aggregated failures after attempting the remaining targets.

### External MCP preflight and sensitive fields

- `validate_external_mcp_config` is restricted to QoderWork and TRAE Work. It
  parses a bounded config, validates supported transport fields and resolves a
  stdio executable without running it.
- The validation result is a closed DTO of findings/reason codes. It must not
  return raw environment values, secret headers, full command output, or a
  caller-controlled path.
- Validation is advisory and non-mutating. A successful preflight does not
  write the unified MCP row or either vendor file.
- MCP `env` and header values may be sensitive. The unified management DTO
  currently returns their raw values because the existing-server editor needs
  to round-trip them; this boundary is not SecretRef-backed or write-only.
  Ordinary list/detail/search, errors, logs, analytics, copy/export, and the
  external preflight DTO must redact or omit those values. The exact editor
  exception and renderer lifetime are owned by
  [V2 MCP](../frontend/v2-mcp.md).

## 4. Validation & Error Matrix

| Condition                                                                                                                       | Required result                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A live-source parse or target projection sees a non-object server, unknown type, missing stdio command, or missing HTTP/SSE URL | Reject at the adapter boundary. If unified upsert already saved the row, return error and treat durable/live state as divergent; do not claim pre-save rejection.                        |
| Direct unified upsert has no enabled target                                                                                     | The row can currently be saved without adapter validation. Do not use that success as proof that the spec is executable; a future centralized validator must be introduced deliberately. |
| Unknown target ID                                                                                                               | Reject before lock or mutation.                                                                                                                                                          |
| Upsert cannot remove a newly disabled live entry                                                                                | Abort before saving the disabled row.                                                                                                                                                    |
| Upsert saves row but an enabled-target projection fails                                                                         | Return error; durable/live state may differ and requires repair/reread.                                                                                                                  |
| Enable toggle writes DB but live projection fails                                                                               | Return error; do not claim rollback or vendor activation.                                                                                                                                |
| Disable live removal fails                                                                                                      | Keep the database flag enabled and return error.                                                                                                                                         |
| Delete cannot remove one owned live entry                                                                                       | Keep the database row and return error for retry.                                                                                                                                        |
| Imported ID has a materially different executable spec                                                                          | Keep source conflict explicit; do not merge by ID alone.                                                                                                                                 |
| Shared JSON projection has an invalid root/server or cannot create the backup                                                   | Return error without overwriting the original file; validation failures also preserve the previous backup.                                                                               |
| One target sync fails during full reconciliation                                                                                | Attempt independent targets, aggregate failures, and avoid global success.                                                                                                               |
| External validation receives an unsupported Agent                                                                               | Reject; only qoderwork/trae-work are valid.                                                                                                                                              |
| Secret env/header value reaches ordinary UI, errors, logs, analytics, copy, export, or preflight result                         | Security regression. Raw values are permitted only in the explicit existing-server editor/query boundary documented by V2 MCP.                                                           |

## 5. Good / Base / Bad Cases

- **Good:** disable a Codex server by removing its live TOML entry under the
  shared writer lock, then update the row and reread the catalogue.
- **Good:** import the same semantically equivalent stdio server from two
  targets after representation-only normalization, preserving both flags.
- **Base:** WorkBuddy is absent and has no MCP file; its adapter skips creating
  a vendor tree while other target projections continue.
- **Base:** external TRAE validation resolves a command and returns findings
  without writing SQLite or the vendor file.
- **Base:** an advanced/direct upsert saves a row, then the first enabled target
  rejects its shape. Return an error, reread the durable catalogue, and present
  repair as required; do not describe the row as rolled back.
- **Bad:** merge conflicting commands because IDs match, mark an enable as
  rolled back after the live write failed, claim every upsert was validated
  before persistence, overwrite Codex `config.toml` without the shared lock,
  or display `env`/authorization header values outside the editor.

## 6. Tests Required

Run the focused backend/V2 gates named by the repository task runner. Required
assertion owners include:

- `src-tauri/src/mcp/validation.rs` and adapter tests: closed type set, required
  command/URL, representation-only equivalence, explicit-false source
  enablement, and validation before each live target parse/write;
- `src-tauri/src/services/mcp.rs`: upsert/toggle/delete ordering, target locks,
  the save-before-enabled-projection boundary, direct all-disabled upsert
  behavior, per-target atomic import, conflict handling, and aggregate
  synchronization failures;
- `src-tauri/src/mcp/**`: each adapter preserves unrelated entries, maps the
  supported transport correctly, removes only the owned ID, and keeps backup/
  absent-product behavior explicit;
- `src-tauri/src/mcp/json_document.rs`: exact original backup bytes, unknown and
  nested executable fields, all eight metadata keys, missing reads, invalid
  root/entry preservation, and backup failure before write. Architecture tests
  keep document mechanics private and prevent adapter-local copies;
- `src-tauri/src/database/dao/mcp.rs`: all nine flags round-trip, missing-row
  updates do not insert, and failed batch import rolls back that target batch;
- `src-tauri/src/services/traework.rs` and V2 platform tests: external MCP
  Agent/transport/reason enums are closed, executable resolution is
  non-executing, DTOs are redacted, and invoke payloads use `agentId/config`;
- `tests/v2/features/authoritativeAssignment.test.tsx` and MCP page/Port tests:
  one serialized mutation, reread authority, exact seven-target order, and no
  direct Tauri/vendor serialization in the page.

Portable tests prove parsing and projected files in fixtures; they do not prove
that a real vendor process reloaded a changed MCP server.

## 7. Wrong vs Correct

Wrong:

```rust
state.db.update_mcp_server_target_enabled(id, &target, false)?;
remove_server_from_target(id, &target)?;
// A failed removal now leaves a false database flag while the Agent still runs it.
```

Correct:

```rust
McpService::toggle_target(&state, id, target, false)?;
// The service removes the live entry before committing the disabled flag.
```

Wrong:

```ts
await invoke("toggle_mcp_app", { serverId, app, enabled }); // feature page
setAssigned(enabled);
```

Correct:

```ts
await ports.mcp.toggleApp(serverId, app, enabled);
await mcpServersQuery.refetch();
// Treat an error as unconfirmed even if the reread durable flag changed.
```
