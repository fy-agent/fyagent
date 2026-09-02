# V2 MCP UI and Secret Draft Contract

## 1. Scope / Trigger

Read this contract before changing the V2 MCP page, server editor, stdio/HTTP
transport selection, external validation, presets/import, target assignment,
secret draft handling, or MCP error/rendering behavior.

Primary owners:

- `src/v2/pages/mcp/**`
- `src/v2/shared/features/mcp.ts` and related MCP feature helpers
- `src/v2/shared/security/mcpSecurity.ts`
- desktop `McpPorts` adapter
- shared [V2 Assignment](./v2-assignments.md)

Native authority is [MCP Management](../backend/mcp-management.md).

## 2. Signatures

The page uses `McpPorts` for typed:

```text
list/upsert/delete MCP servers
validate a closed external MCP config
import from supported targets
read target status/probe metadata where exposed
toggle one closed target assignment and return authoritative MCP state
```

The editor produces exactly one transport shape:

```text
stdio = { command, args?, env? }
http  = { url, headers? }

config = { mcpServers: Record<serverId, stdio | http> }
```

Stdio and HTTP fields are mutually exclusive. Dynamic results pass strict
parsers before they enter page state.

The renderer never supplies a vendor config path, database row, executable
path, shell string, DNS result or “run/test server” instruction.

## 3. Contracts

### Editor and transport model

- One server ID identifies the draft and persisted item. Display labels do not
  become identity.
- Selecting stdio clears HTTP-only fields; selecting HTTP clears stdio-only
  fields and their secret drafts. Hidden incompatible values must not survive
  and be submitted later.
- Command/args/env and URL/headers are structured fields. Do not concatenate a
  shell command or parse one with whitespace splitting.
- Reject/flag prototype-pollution keys, control characters, duplicate server
  IDs, invalid names, unsupported fields and configured count/depth/byte limits
  before mutation.
- URL validation and executable availability remain native observations. The
  component does not fetch, resolve DNS, spawn, `which`, or inspect the local
  filesystem.

### Secret draft lifetime

- Environment/header values that may be secrets live only in the current
  editor draft. They never enter query data, URL/local storage, analytics,
  error logs, React keys or reusable clipboard templates.
- Server list/readback renders secret presence/redacted placeholders, not raw
  values. The UI does not attempt to reconstruct a stored secret from
  `<redacted>`.
- Clear secret drafts on cancel, successful submit, failed submit after the
  result is handled, transport change, selected server/target change, delete,
  route unmount and test cleanup.
- Copy/export/template actions pass through `mcpSecurity.ts` and redact every
  env/header value. “Show raw JSON” must not bypass the same policy.

### Validation is non-executing

- Validation returns closed per-server findings: transport, reason,
  `executableAvailable: true | false | null`, and `hasSecrets`.
- `false` means the native observer found the executable unavailable; `null`
  means it could not authoritatively observe. The UI must not collapse null to
  false.
- HTTP validation is grammar/literal-address policy only; stdio validation is
  grammar/availability only. A successful validation is not server execution,
  network reachability, authentication or vendor acceptance.
- Raw absolute executable path, environment/header values, full URL
  diagnostics, command output and response body never render.

### CRUD, import and assignment

- Query cache owns persisted MCP servers; the editor owns only the current
  draft. Upsert/delete success commits from native returned/readback data and
  invalidates the relevant list/detail/assignment keys.
- Import is explicit and presents per-target conflict/outcome. It never runs an
  imported command or probes an imported URL.
- Presets are reviewed structured templates and still pass the same local and
  native validation. A preset is not a trust bypass or proof its executable is
  installed.
- Target switches use the shared `AssignmentPanel` and MCP-specific port. The
  page does not duplicate target order, vendor file paths or rollback logic.
- A successful target write means FyAgent wrote/reread its representation. It
  does not prove the vendor app reloaded the MCP server.

### Errors and accessibility

- Field grammar errors stay adjacent to fields; native validation/write errors
  render localized closed reasons. Focus moves to the first invalid field or
  alert without revealing raw backend text.
- Deleting or changing transport requires explicit confirmation when it drops
  secret-bearing/assigned state.
- Transport controls, dynamic env/header rows, validation findings and target
  switches remain keyboard operable with semantic labels and non-color status.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| MCP DTO/parser/version failure | Fail closed; no partial editor/assignment. |
| Draft contains both `command` and `url` families | Block submit and focus transport conflict. |
| Transport changes | Clear incompatible fields and all associated secret drafts. |
| Validation returns executable `null` | Render unknown/unobserved, not missing. |
| Validation succeeds | Describe structural/availability validation only; do not say connected/running. |
| Import returns conflict/unsupported target | Render per-target outcome; no implicit overwrite. |
| Upsert/delete/assignment fails | Restore/invalidate authoritative state and clear sensitive draft. |
| Preset contains placeholder/secret field | Keep placeholder local, require review and redact copies. |
| User copies/exports config | Route through redaction helper; never copy raw env/header secrets. |
| Route/server/target changes or unmounts | Clear secret and stale validation state. |
| Raw path/command output/header/env appears in DOM/log/snapshot | Security regression. |

## 5. Good / Base / Bad Cases

- **Good:** choose stdio, enter structured command/args/env, validate without
  execution, save through `McpPorts`, clear the draft and render authoritative
  redacted state.
- **Good:** executable availability is unknown on the current host; the finding
  remains unknown and assignment can follow only native target policy.
- **Base:** a preset validates structurally but the executable is missing; show
  the exact finding and do not claim it runs.
- **Base:** target file write succeeds; explain FyAgent configuration and that
  vendor reload may still be needed.
- **Bad:** concatenate shell text, `fetch` the HTTP URL, run the stdio command,
  store headers in query/local storage, copy raw JSON, or keep an optimistic
  target switch after native rollback.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- strict MCP/server/finding/target parsing, exact mutually exclusive transport
  shapes and unknown-field rejection;
- transport switch clears incompatible fields/secrets and submit cannot revive
  hidden values;
- every terminal/cancel/error/change/unmount path clears env/header secret
  drafts;
- validation invokes only the typed native validator and preserves
  `true | false | null`; no renderer process/network/filesystem path;
- preset/import/CRUD success and failure use authoritative cache updates and no
  automatic non-idempotent retry;
- shared assignment order/rollback/readback and vendor-reload wording;
- copy/export/template redacts every env/header value;
- accessibility for dynamic rows, errors, findings, confirmations and switches;
- browser fixtures do not count as native executable, network or vendor reload
  evidence.

## 7. Wrong vs Correct

Wrong:

```tsx
const test = async () => {
  if (draft.url) await fetch(draft.url);
  else await invoke("run_mcp_command", { command: `${draft.command} ${draft.args}` });
};

localStorage.setItem("mcp-draft", JSON.stringify(draft));
```

Correct:

```tsx
try {
  const config = buildExclusiveMcpConfig(draft);
  const findings = await ports.mcp.validate(agentId, config);
  setFindings(findings);
} finally {
  // Submission or route lifecycle clears secret-bearing draft state according
  // to the owning interaction path.
}
```
