# MCP Management, Live Files, and Validation Contract

## 1. Scope / Trigger

Read this contract before changing MCP database records, target assignment,
vendor live-file projection/import, external MCP validation, presets, or the
MCP Tauri command surface.

Primary owners:

- `src-tauri/src/commands/mcp.rs` — transport;
- `src-tauri/src/services/mcp.rs` — database orchestration and application
  assignment;
- `src-tauri/src/mcp/**` — per-target live config readers/writers and shared
  validation;
- `src-tauri/src/database/dao/mcp.rs` plus schema migrations — persisted MCP
  rows and target flags.

Frontend behavior is in [V2 MCP](../frontend/v2-mcp.md) and
[V2 Assignment](../frontend/v2-assignments.md).

## 2. Signatures

The core command family remains:

```text
get_mcp_servers() -> McpServer[]
upsert_mcp_server(request) -> McpServer
delete_mcp_server(id) -> ()
toggle_mcp_app(id, targetId, enabled) -> authoritative McpServer
import_from_apps_mcp(targetIds?) -> ImportResult
validate_external_mcp_config({ agentId, config })
  -> ExternalMcpValidationResult
```

Target-specific status/probe commands remain closed helpers in
`commands/mcp.rs`; they do not accept arbitrary file paths or execute server
commands.

Native `McpTargetId` contains the supported legacy applications plus
`qoderwork`, `trae-work`, and `workbuddy`. V2 order is:

```text
qoderwork | trae-work | workbuddy | grokbuild |
codex | claude | opencode
```

The validated config envelope is exactly:

```text
{ mcpServers: Record<serverId, StdioServer | HttpServer> }

StdioServer = { command, args?, env? }
HttpServer  = { url, headers? }
```

Mixed transport fields and unknown fields are rejected.

## 3. Contracts

### Persistence and assignment

- SQLite stores one canonical MCP row with serialized server config, metadata,
  tags and explicit target-enable flags. Schema/migration mechanics are owned
  by [SQLite Persistence](./persistence-and-migrations.md).
- DAO reads and writes every supported target flag. A new target requires
  schema migration, DAO, enum, parser, live adapter and round-trip tests.
- `toggle_mcp_app` delegates to the target adapter, then returns authoritative
  persisted/readback state. A failed live write must not leave the database
  claiming that the target is enabled.
- QoderWork, TRAE Work and WorkBuddy remain MCP target IDs rather than
  `AppType` values.

### Vendor live files

Direct live files include:

```text
WorkBuddy     -> ~/.workbuddy/mcp.json
QoderWork CN  -> ~/.qoderworkcn/mcp.json
TRAE Work CN  -> TRAE SOLO CN User/mcp.json
```

- Each live document is a Claude-style `mcpServers` map and uses the shared
  bounded parse, backup, atomic-write and authoritative-reread discipline.
- When both the vendor home/User directory and target file are absent, skip the
  write. Do not create a vendor installation or user-data root merely to claim
  assignment success.
- WorkBuddy may import legacy hidden `.mcp.json` only when the official
  `mcp.json` is absent; the first official write may seed from that hidden
  document. New writes target only `mcp.json`.
- Qoder import may normalize reviewed `type: "streamable-http"` to the
  supported HTTP representation before validation.
- Do not write Qoder builtin `userData/mcp.json` or TRAE `state.vscdb` for MCP.
- Preserve unknown top-level/server fields only when the target adapter can
  round-trip them safely. Unsupported shapes block destructive replacement.

### Validation and secret handling

- Validate server IDs, object depth/count/byte limits and prototype-pollution
  keys before any projection.
- A server is exactly stdio or HTTP. Reject mixed `command`+`url`, unknown
  transport members, control characters and malformed values.
- Stdio validation checks only bounded command/argument/environment grammar and
  executable availability. It never starts a process, shell, installer or MCP
  server.
- HTTP validation checks URL grammar and literal-address policy only. It does
  not perform DNS or a network request.
- Findings expose server ID, closed transport/reason codes,
  `executableAvailable: boolean | null` and `hasSecrets`. They do not expose
  absolute executable paths, full environment/header values or resolved
  credentials.
- Templates replace every environment/header value with `<redacted>` before
  IPC, logs, DOM, clipboard or test snapshots.
- Secret drafts remain renderer-local for the current edit and are cleared on
  cancel, success, failure, target switch and unmount. Native persistence uses
  the reviewed target format; do not place plaintext secrets in query keys or
  diagnostics.

### Import and authority

- Import reads only reviewed target files through their adapter, validates the
  result and merges by canonical server identity under explicit conflict
  policy. It does not execute any imported command or probe any URL.
- A successful database upsert is not proof that a vendor app accepted/reloaded
  the config. Report only FyAgent write/readback; restart/vendor recognition is
  separate HIL evidence.
- Catalog `mcp.write` controls whether Agents navigation may offer “Open MCP.”
  The page does not infer writability from local file existence.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown MCP/target ID or extra field | Reject; no DB/live-file mutation. |
| Config lacks exactly one `mcpServers` object | Reject; no import/write. |
| Server mixes stdio/HTTP or contains unknown transport fields | Reject that config; execute nothing. |
| Prototype key, control character, unsafe URL/address or configured bound exceeded | Reject before projection. |
| Stdio executable is missing/unobservable | Return `false`/`null` availability; never execute it. |
| Env/header values contain secrets | Mark/redact; never echo raw values. |
| Vendor home/User and live file are both absent | Skip/unavailable; do not create vendor root. |
| Live document is linked/raced/unsupported or revision changes | Fail closed; preserve bytes and prior assignment. |
| Backup/write/replace/reread fails | Return failure/uncertain state; do not claim enabled. |
| WorkBuddy new write targets hidden `.mcp.json` | Contract regression; official file is `mcp.json`. |
| Qoder write targets builtin `userData` or TRAE write targets SQLite | Contract regression. |
| Vendor file write succeeds | Report FyAgent readback only, not vendor reload/acceptance. |

## 5. Good / Base / Bad Cases

- **Good:** validate a stdio definition without running it, persist the
  canonical row, back up the target file, atomically project `mcpServers`,
  reread, then return authoritative assignment state.
- **Good:** import WorkBuddy legacy hidden config only when the official file
  is absent, then write future changes to `mcp.json`.
- **Base:** executable availability is `null` on a host that cannot inspect it;
  validation remains non-executing and non-green.
- **Base:** the vendor directory is absent; assignment is skipped/unavailable
  instead of creating an apparent installation.
- **Bad:** test an HTTP server during validation, spawn the stdio command,
  serialize headers/env, write TRAE SQLite, or set enabled before an unverified
  live-file update.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
```

Required assertion points:

- `src-tauri/tests/mcp_commands.rs` covers command DTOs, CRUD, target toggle,
  import and error propagation;
- current schema/DAO round-trips every target flag and preserves legacy flags
  through migrations;
- exact WorkBuddy/QoderWork/Trae live paths, absent-root skip, hidden WorkBuddy
  import/seed rule, Qoder normalization, backup/atomic/reread and path race
  rejection;
- validation rejects mixed/unknown fields, prototype keys, unsafe URLs,
  control characters and all count/size/depth limits;
- stdio validation never starts a process and HTTP validation performs no
  DNS/network request;
- all env/header values are redacted in DTOs, errors, logs, templates,
  clipboard projections and frontend query state;
- assignment failure restores prior database/UI state and vendor success is not
  described as reload proof.

## 7. Wrong vs Correct

Wrong:

```rust
if config.command.is_some() {
    Command::new(config.command.unwrap()).status()?;
}
reqwest::get(config.url.unwrap()).await?;
```

Correct:

```rust
let findings = validate_external_mcp_config(agent_id, config)?;
// Grammar/path availability only: no process and no network.
```

Wrong:

```rust
let path = home.join(".workbuddy").join(".mcp.json");
```

Correct:

```rust
let path = home.join(".workbuddy").join("mcp.json");
// Hidden `.mcp.json` is import/seed compatibility only.
```
