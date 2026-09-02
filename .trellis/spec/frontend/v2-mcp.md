# V2 MCP Management UI Contract

## 1. Scope / Trigger

Read this contract before changing the V2 MCP installed/discovery views,
quick/advanced editor, presets, import, target assignment, bulk assignment,
ordinary-detail redaction, or MCP mutation/error handling.

Primary owners are:

- `src/v2/pages/mcp/Page.tsx`, `Discovery.tsx`, `InstallDialog.tsx`, and
  `catalog.ts` for current route behavior;
- `src/v2/shared/features/mcp.ts` for renderer MCP types;
- `src/v2/shared/features/mcpSecurity.ts` and the MCP helpers in
  `src/v2/shared/features/helpers.ts` for display/search redaction and editor
  serialization;
- `src/v2/shared/features/ports.ts` and
  `src/v2/shared/platform/tauri/feature-ports/simple.ts` for `McpPort` IPC;
- `src/v2/shared/features/queries.ts` for the installed-server query.

Shared target presentation is owned by
[V2 Shared Assignment](./v2-assignments.md). Native validation, persistence,
live-file ordering, import, and conflict semantics are owned by
[MCP Management](../backend/mcp-management.md).

QoderWork/TRAE external MCP preflight is not part of this management page; it
uses `ExternalMcpPort` in the relevant Agent configuration flow.

## 2. Signatures

The current management Port is exactly:

```ts
interface McpPort {
  getAll(): Promise<Record<string, McpServer>>;
  upsert(server: McpServer): Promise<void>;
  delete(id: string): Promise<boolean>;
  toggleApp(
    serverId: string,
    app: McpTargetId,
    enabled: boolean,
  ): Promise<void>;
  importFromApps(): Promise<number>;
}
```

The renderer model is intentionally open to native/vendor fields:

```ts
interface McpServerSpec extends Record<string, unknown> {
  type?: "stdio" | "http" | "sse";
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  headers?: Record<string, string>;
}

interface McpServer extends Record<string, unknown> {
  id: string;
  name: string;
  server: McpServerSpec;
  apps: McpAssignments;
  description?: string;
  tags?: string[];
  homepage?: string;
  docs?: string;
  source?: string;
}
```

`createSimpleFeaturePorts().mcp` is currently a thin, compile-time-typed Tauri
adapter over these exact commands:

```text
get_mcp_servers
upsert_mcp_server        { server }
delete_mcp_server        { id }
toggle_mcp_app           { serverId, app, enabled }
import_mcp_from_apps
```

It does not currently perform runtime DTO/version parsing. Do not document a
strict renderer parser as already present. A future change that adds an
untrusted or versioned response must add parsing at this adapter boundary.

## 3. Contracts

### Query and mutation ownership

- `useMcpServers()` owns the installed map under `featureKeys.mcp`. List,
  detail, search, and assignment render from this query result.
- `McpPage.write` owns a page-wide `writeLock`, busy state, success/error toast,
  and `featureKeys.mcp` invalidation in `finally`. A concurrent management-page
  write is ignored before native invocation.
- Upsert, delete, one-target toggle, import, and sequential bulk assignment all
  go through `McpPort`; the page never serializes a vendor live file or calls a
  compatibility Tauri command directly.
- Unified upsert can return an adapter validation/write error after native code
  has already saved the SQLite row and before every enabled live target was
  projected. The page sanitizes the error and invalidates/refetches the MCP
  query in `finally`; the reread may therefore show a durable row after a
  failed toast. Do not claim native rollback or remove that row optimistically.
- The management page does not use `useAuthoritativeAssignmentMutation`.
  `toggleApp` returns `void`; convergence happens by invalidating/refetching the
  installed query after the command settles. Do not claim the toggle command
  directly returns authoritative MCP state or atomically rolls back.
- Bulk assignment calls `toggleApp` sequentially, records each failure, then
  reports successful/failed counts. Earlier successful items remain applied
  when a later item fails.
- A native command error is sanitized through
  `sanitizeMcpConfigurationError`; the raw backend string is not rendered.

### Quick and advanced editor

- New IDs are trimmed, required, and rejected when already present in the
  current installed map. An existing ID is fixed while editing; `name` is also
  required.
- A new editor draft starts from `DEFAULT_NEW_APPS`, which enables every ID in
  the closed seven-target `MCP_TARGET_IDS` tuple. Editing an existing server
  starts from its stored flags. Because native unified upsert saves SQLite
  before enabled target adapters validate/project, the default fan-out makes a
  post-save partial failure possible even on the first create.
- Quick mode supports three transports:
  - stdio: non-empty `command`, newline-delimited `args`, optional `cwd`, and
    `KEY=VALUE` env rows;
  - http/sse: a URL accepted by `new URL(...)` plus optional `Name: Value` or
    `Name=Value` header rows.
- `parseKeyValueLines` reports malformed rows but allows later duplicate keys
  to replace earlier values in the local map. Native validation remains the
  final authority.
- Advanced mode accepts one JSON object and rejects a top-level `mcpServers`
  wrapper. It intentionally preserves unknown fields because
  `McpServerSpec` extends `Record<string, unknown>`. This local parser does not
  validate the types/requirements of every known transport field. The unified
  native upsert also has no centralized pre-SQLite validator; an enabled target
  adapter can reject the shape only during post-save projection, while an
  all-disabled row can be saved without adapter validation.
- Moving quick -> advanced overlays the seven known transport fields onto the
  previous draft while preserving unknown fields. Moving advanced -> quick
  parses the JSON object and projects known fields into the form.
- Switching the selected quick transport does not erase the opposite family
  from React state immediately. `quickSpec()` emits only the active family, and
  `overlayKnownMcpFields` removes all old known transport fields before
  applying it, so hidden quick-form values are not persisted in that save.
- Presets are local structured `mcpPresets`; applying one clears env/header
  form text and still passes through the same editor/native save path.

### Current sensitive-value boundary

- The current native DTO returns raw `env` and `headers` so an existing server
  can be edited. Those values therefore exist in the installed query data and
  in the editor's textarea/advanced JSON while the dialog is open. FyAgent does
  not currently replace them with `SecretRef` or a write-only placeholder at
  this renderer boundary.
- Ordinary detail avoids rendering env/header values and shows only their
  counts. It redacts recognized secret-bearing URL query parameters and command
  arguments with `redactMcpUrl` / `redactMcpArgs`.
- Search excludes env/header maps, strips URL query/fragment values from its
  URL token, and uses redacted arguments. Command, cwd, ID, name, tags, source,
  homepage, and docs remain searchable/displayable according to current UI.
- The editor dialog is the only current UI intended to expose raw env/header
  values. Cancel or successful save unmounts it; a failed save deliberately
  leaves the dialog and draft open for correction. Do not claim every failure
  clears the draft.
- No current copy/export action serializes the full server object. Any future
  copy/log/analytics path must explicitly redact env/header values and
  secret-bearing URL/argument positions before release.

### Discovery, import, assignment, and trust copy

- Discovery uses the local reviewed catalog and builds an `McpServer` for
  `McpPort.upsert`; it is not a runtime/network test of the recipe.
- Import delegates to `importFromApps()` and reports the returned imported
  count. Native code owns per-target parsing and conflict behavior.
- The seven target rows and order come from `MCP_TARGETS`. Assignment pages do
  not construct vendor paths or choose the native write format.
- Enabling/installing WorkBuddy opens the product's trust disclosure after the
  command succeeds; it does not prove WorkBuddy reloaded the configuration.
- Detail may show a derived install directory from `cwd` or an absolute command
  parent through `CopyablePath`. This is explicit user-facing local-path UI,
  not a claim that paths never enter the renderer.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| Installed query fails before any data | Render load failure and retry; do not fabricate an empty map. |
| Refresh fails with cached data | Keep the last successful map and show the refresh warning. |
| New ID is empty/duplicate or name is empty | Block submit locally. |
| New server draft is created | Initialize all seven V2 target flags enabled; do not describe it as unassigned-by-default. |
| Quick stdio command is empty | Block submit with the local command error. |
| Quick HTTP/SSE URL fails `new URL` | Block submit locally. |
| Env/header row lacks a usable separator/key | Block submit and list the affected row. |
| Advanced JSON is invalid, not an object, or contains top-level `mcpServers` | Block mode switch/save. |
| Advanced object contains unknown fields | Preserve them for native validation; do not silently discard. |
| Native upsert/delete/toggle/import fails | Show sanitized error, keep/refresh current query authority, and do not claim rollback. |
| Advanced upsert is saved, then an enabled target rejects its transport shape | Keep the failure toast/editor for correction, refetch the durable map, and allow the saved row to remain visible; do not claim pre-save validation or automatic deletion. |
| Advanced/direct row has every target disabled | Native can currently persist it without target-adapter validation; do not describe successful save as proof that the server is executable. |
| One bulk item fails | Continue remaining items, report partial counts, and refetch the map. |
| WorkBuddy write succeeds | Show trust disclosure; do not claim vendor reload/execution. |
| Ordinary detail/search sees env/header or sensitive URL/arg value | Redact/exclude as defined above. |
| Editor opens an existing secret-bearing server | Raw values may appear only in the editing controls; do not log/copy them elsewhere. |

## 5. Good / Base / Bad Cases

- **Good:** edit a stdio server, preserve an unknown native field in advanced
  mode, save through `McpPort.upsert`, then render the invalidated query result.
- **Good:** toggle several servers sequentially for one target; report partial
  failure counts and retain successful earlier writes.
- **Base:** a failed upsert leaves the editor open with its draft so the user
  can correct it. The invalidated query may show a row already saved by native
  before target projection failed; that durable state is not hidden or called
  rolled back.
- **Base:** an HTTP URL contains a token query parameter; the editor can show it
  for editing, while detail/search redact or omit the value.
- **Bad:** call a nonexistent `McpPorts.validate`, claim runtime DTO parsing in
  `simple.ts`, claim native upsert always validates before SQLite, treat
  `toggleApp(): void` as returned authoritative state, clear unknown advanced
  fields, or display env/header values in normal detail.

## 6. Tests Required

Run the focused V2 checks through the repository task runner. Required
assertion owners include:

- `tests/v2/platform/featurePorts.test.ts`: exact five `McpPort` command names,
  camelCase payload keys, and return mapping;
- `tests/v2/features/featurePages.test.tsx`: installed loading/error/cached-data
  states, add/edit/delete/import, assignment, sequential partial bulk behavior,
  invalidation, and sanitized errors;
- `tests/v2/features/helpers.test.ts`: quick key/value parsing, advanced-object
  rules, known-field overlay, selection/search, and error sanitization;
- `tests/v2/features/mcpSecurity.test.ts` and
  `tests/v2/features/mcpCatalog.test.ts`: URL/argument redaction, search tokens,
  recipe identity, and reviewed catalog projection;
- `tests/v2/shared/AssignmentPanel.test.tsx`: semantic seven-target controls;
- `tests/v2/app/architecture.test.ts`: route/shared component ownership and no
  page-level Tauri/vendor-file imports.

Browser fixtures prove renderer interaction only. Native file projection,
save-before-projection failure, conflict handling, and vendor reload require
the backend/native evidence named by
[MCP Management](../backend/mcp-management.md).

## 7. Wrong vs Correct

Wrong:

```ts
const result = await ports.mcp.toggleApp(serverId, target, enabled);
setServer(result); // toggleApp returns void; no authoritative server is returned.
```

Correct:

```ts
try {
  await ports.mcp.toggleApp(serverId, target, enabled);
} finally {
  await queryClient.invalidateQueries({ queryKey: featureKeys.mcp });
}
// Render the query result; a native error is not described as atomic rollback.
```

Wrong:

```ts
logger.info(server.server.env);
const searchable = JSON.stringify(server.server);
```

Correct:

```ts
const args = redactMcpArgs(server.server.args ?? []);
const url = server.server.url
  ? redactMcpUrl(server.server.url)
  : undefined;
// Ordinary UI excludes env/header values; raw values stay in the editor only.
```
