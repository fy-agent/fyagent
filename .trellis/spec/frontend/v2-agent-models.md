# V2 Agent Directory and Models Quick-Setup Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Agent directory, Models quick setup,
their local Agent assets, the versioned native catalog (including OpenCode and
Grok Build), the shared `PRODUCT_DIRECTORY`, WorkBuddy model ports,
Claude/Codex/Grok Build Provider quick setup, or the sanitized Provider
summary boundary.
The common shell, native-chrome, router, and layer rules remain in
[V2 Shell](./v2-shell.md). Skills/MCP and Prompt/Memory have separate feature
contracts and must not be folded into the Agent capability catalog. Reuse is
the default: Agents and Models share `CatalogMasterDetail` / `SplitPanes`;
TRAE and OpenCode share `modelsShared` / `modelChips` / `ModelConnectivityTest`
rather than forking panel chrome. New chrome both panels will need goes in
that shared module
on the first commit. See [Frontend Reuse](./reuse.md).

The product boundary is deliberately asymmetric. Agents and Models share one
`CatalogMasterDetail` geometry and local brand metadata, but each detail keeps
its own capability workflow:

- QoderWork CN, TRAE Work CN, WorkBuddy, and Grok Build each expose one
  catalog-owned product link; Claude Code exposes separate CLI and Desktop
  links; OpenCode exposes `product` then `cli`. Agent directory details render
  only `mode === "direct"` capability jumps to `/models`, `/skills`, or
  `/mcp`. They do not render the capability-item grid, catalog `description`,
  or a page-level title. They do not mount application status, configuration overviews,
  unsupported-capability lists, support counts, usage notes, Qoder Hooks
  editors, or MCP validation panels. Non-Codex details mount a page-local
  「产品介绍」 section from `src/v2/pages/agents/intros.ts` (hardcoded Chinese,
  not the catalog `description` and not 「使用说明」). That copy describes the
  third-party product only and must not mention FyAgent. Codex keeps the desktop
  installer as its substantial body and does not require that intro. The
  installer heading explains install/update/launch of Codex Desktop and must
  not mention FyAgent.
- WorkBuddy and OpenCode each use a dedicated revision-checked
  model-configuration domain. WorkBuddy additionally exposes direct Skills
  copy and MCP `mcp.json` assignment. Grok Build Models uses the same Provider
  quick-setup boundary as Claude/Codex (`fyagent-v2-quick-setup-grokbuild`,
  live `~/.grok/config.toml`).
- Codex exposes no catalog link. Its detail owns the FyAgent-managed desktop
  installer while Codex, Claude Code, and Grok Build retain bounded Provider
  quick setup.
- OpenCode model write is `direct` + `dedicated_native_contract`. The Models
  page mounts the dedicated `opencodeModels` port, never Provider quick setup
  or the Codex installer.
- QoderWork CN `models.write` is `unsupported`: the Models page must state
  官方不支持第三方模型配置 and must not mount a third-party model editor. It must
  not render 「打开官方设置」 or 「管理 MCP」.
  TRAE Models must not render 「打开 TRAE 官方模型设置」. TRAE `models.write`
  is `assisted` + `vendor_ui_required`: the Models page states that custom
  models must be added in TRAE Work CN and must not save into TRAE sqlite.
- Browser preview never impersonates authoritative desktop state or installer
  success.

## 2. Signatures

The payload-free Rust catalog command serializes this exact versioned shape:

```ts
type AgentCatalogId =
  | "qoderwork"
  | "trae-work"
  | "workbuddy"
  | "grokbuild"
  | "codex"
  | "claude-code"
  | "opencode";

type AgentOfficialLinkId = "product" | "cli" | "desktop";

type AgentOfficialLink = {
  id: AgentOfficialLinkId;
  label: string;
  url: string;
};

type AgentCatalogResult = {
  contractVersion: 4;
  reviewedAt: string;
  agents: Array<{
    id: AgentCatalogId;
    variantId:
      | "qoderwork-cn"
      | "trae-work-cn"
      | "workbuddy"
      | "grokbuild"
      | "codex"
      | "claude-code"
      | "opencode";
    displayName: string;
    description: string;
    officialLinks: AgentOfficialLink[];
    capabilities: Array<{
      id:
        | "product.open" | "app.detect" | "app.launch"
        | "skills.read" | "skills.write"
        | "hooks.read" | "hooks.write"
        | "models.validate" | "models.write"
        | "mcp.validate" | "mcp.write";
      mode: "direct" | "assisted" | "unsupported" | "unverified";
      reasonCode: string;
      evidenceIds: string[];
    }>;
  }>;
};

get_agent_catalog() -> AgentCatalogResult

get_external_agent_status({ agentId }) -> {
  agentId: AgentCatalogId;
  detected: boolean | null;
  running: boolean | null;
  version: string | null;
  installSource:
    | "managed_installer" | "official_installer" | "system_package"
    | "user_installation" | null;
  capabilities: Array<{
    id: AgentCatalogResult["agents"][number]["capabilities"][number]["id"];
    state:
      | "available" | "assisted" | "unavailable" | "unverified"
      | "blocked_by_version" | "probe_failed";
    reasonCode: string;
  }>;
}

launch_external_agent({
  agentId,
  destination: "home" | "skills" | "hooks" | "models" | "mcp",
}) -> { agentId, destination, state, reasonCode }
```

V2 reads a non-secret Provider projection in one native snapshot:

```ts
type ProviderAppId = "claude" | "codex" | "grokbuild";
type ProviderSummary = { id: string; name: string };
type ProviderSummaryQueryData = {
  providers: Record<string, ProviderSummary>;
  currentId: string;
};

get_provider_summary({ app }) -> ProviderSummaryQueryData
```

Quick setup accepts a dedicated minimum request, never the generic Provider
wire. Rust derives the reserved ID, category, notes, and app-specific stored
shape:

```ts
type ProviderQuickSetupRequest = {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  codexFeatures?: {
    imageExtension?: boolean;
    websockets?: boolean;
  };
};

apply_provider_quick_setup_with_result({ request, app })
  -> ProviderMutationResult<{
       warnings: string[];
     }>
  | { code: "APPLY_FAILED_ROLLED_BACK" }
  | { code: "ROLLBACK_PARTIAL_STATE_UNKNOWN" };
```

The success envelope contains only non-secret stable fields:

```ts
type ProviderMutationResult<T> = {
  value: T;
  liveConfigChanged: boolean;
  app: ProviderAppId;
  warningCodes?: Array<
    "CODEX_WEBSOCKET_NON_GPT_MODEL" | "CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED"
  >;
};
```

WorkBuddy signatures and revision/overwrite semantics remain authoritative in
[WorkBuddy Configuration](../backend/workbuddy-configuration.md). TRAE Work CN
and OpenCode reuse that revision / one-time overwrite-token envelope through
dedicated ports. API keys are mutation arguments only and never query keys or
query data.

```ts
get_traework_model_ids() -> {
  modelIds: string[];
  revision: string | null;
  truncated: boolean;
}

get_opencode_model_snapshot() -> {
  providers: Array<{ id: string; name: string; modelIds: string[] }>;
  revision: string | null;
}

fetch_opencode_provider_models({ request: OpenCodeFetchModelsRequest })
  -> { models: Array<{ id: string; ownedBy?: string | null }>; truncated: boolean }

save_opencode_models({ request: OpenCodeSaveModelsRequest })
  -> WorkBuddySaveModelsResult

stream_check_url({ baseUrl: string })
  -> {
       success: boolean;
       status: "operational" | "degraded" | "failed";
       message: string;
       responseTimeMs: number | null;
       httpStatus: number | null;
     }

stream_check_model({
  app: "claude" | "codex" | "grokbuild" | "workbuddy" | "opencode",
  baseUrl: string,
  apiKey: string,
  modelId: string,
})
  -> {
       success: boolean;
       status: "operational" | "degraded" | "failed";
       message: string;
       responseTimeMs: number | null;
       httpStatus: number | null;
       modelUsed: string;
       errorCategory: string | null;
     }
```

`checkReachability(baseUrl)` remains on `providers`, `workbuddy`, and
`opencodeModels` as the URL-only GET probe (`stream_check_url`). Models UI
must not use it for 「测试连通」.

`checkModel({ app, baseUrl, apiKey, modelId })` is available on the same three
ports and invokes `stream_check_model`. It sends one authenticated streaming
request for the selected model (first SSE chunk = success) and never looks up
a saved Provider or touches the failover circuit breaker. Protocol is
Anthropic Messages for Claude, OpenAI Responses for Codex, and OpenAI Chat
Completions for Grok Build / WorkBuddy / OpenCode. Empty `apiKey` omits the
auth header. Failure `message` is `HTTP {status}: {truncated body}` or the
transport error, with the API key redacted. Browser adapters reject both
methods as native-only. Qoder and TRAE Models ports must not expose
`checkModel`.

`OpenCodeSaveModelsRequest` may carry `apiKey`
only as a mutation field. GET snapshots contain sanitized model/provider IDs
and a revision, never `ak` / `sk` / `apiKey`.

## 3. Contracts

### Catalog and local assets

- `get_agent_catalog` is deterministic, non-networking, non-secret, and ordered
  exactly: QoderWork CN, TRAE Work CN, WorkBuddy, Grok Build, Codex,
  Claude Code, OpenCode. Grok Build's product URL is exactly `https://x.ai/grok`.
  TRAE `displayName` is `TRAE Work CN`; its product URL is exactly
  `https://www.trae.cn/sem-work`.   Catalog descriptions use 支持 / 不支持
  wording and must not contain `可在 FyAgent` or `可通过 FyAgent`.
  QoderWork CN catalog `description` and Agent intro copy must not mention
  Hooks.
  QoderWork CN and TRAE Work CN describe MCP as 直接分配; their `mcp.write`
  mode is `direct` with `dedicated_native_contract`.
- The v4 link matrix is exact: QoderWork CN, TRAE Work CN, WorkBuddy, and
  Grok Build each own one `product` link; Claude Code owns `cli` then
  `desktop`; OpenCode owns `product` then `cli`; Codex owns an empty list and
  keeps its dedicated managed installer outside generic launch. Link IDs are
  unique per entry, labels are nonempty, and URLs are absolute HTTPS values
  owned by Rust. Official buttons on the Agent directory use shared
  `CatalogOfficialLinks` with `fy-control-button-primary`. Labels that already
  contain `官方` stay as catalog text.
- V1 `officialUrl`, catalog v2, future catalog versions, and unknown capability,
  mode, evidence, variant, or runtime values fail closed in the
  Tauri adapter. The renderer never guesses a legacy shape or carries a second
  URL table.
- The UI renders catalog capability mode/reason/evidence and the separate
  runtime capability state; it does not derive
  capability from the display name, icon, URL, installed files, or a duplicate
  frontend matrix. Agent details render only `mode === "direct"` capabilities
  and the matching jumps (`/models?target=` when `models.write` is `direct`,
  `/skills` when Skills read/write is `direct`, `/mcp` when `mcp.write` is
  `direct`). They omit the capability-item grid, catalog `description`,
  application status, configuration overviews,
  unsupported lists, support counts, usage notes, Qoder Hooks editors, and
  MCP validation panels. Official catalog links render through
  `CatalogOfficialLinks`.
- Every entry resolves through `src/v2/shared/assets/agents`. QoderWork CN uses
  the reviewed official 256x256 PNG extracted from QoderWork CN.app; TRAE uses
  the reviewed official 48x48 PNG without recoloring or runtime upscaling
  beyond its native detail size. List icons are decorative; the detail identity
  owns the useful accessible name.
- Third-party marks identify their own products only. Their presence is not
  vendor endorsement, redistribution permission, or FyAgent application
  identity.

### Agent directory

- Render a keyboard-accessible left selector and right detail. The selected
  button owns `aria-current`; initial selection follows native catalog order.
- Both pages use the shared `CatalogMasterDetail` geometry, backed by the
  shared `SplitPanes` chassis: default rail
  `clamp(220px, 24vw, 268px)`, 14px separator gutter, 56px rows, 36px list
  frames, 64px detail frames, stable scrollbar gutter, the 760px
  master/detail stack (list becomes two columns; the separator is hidden),
  and the 520px list collapse to one column. Page CSS must not redefine
  catalog columns, brand-ID sizing, or another responsive rail.
  `CatalogMasterDetail` keeps the catalog brand list and the separator name
  `调整目录与详情的宽度`. Other product pages reuse `SplitPanes` without
  catalog rail/list/brand classes.
- Above 760px the two panes fill the remaining feature-page height and
  scroll independently. Split-pane children fill that pane (`height: 100%`,
  `overflow: auto`), matching the catalog rail. The detail panel is at least
  the pane height and
  grows with its content so its chrome does not clip overflowing cards.
  Both catalog pages share the feature-page inset: 20px page padding. Agent
  and Models have no page-level `.fy-feature-header`. `.fy-catalog-page`
  sets `gap: 0`. Page CSS must not add another `gap` or `padding-top` on
  `.fy-agents-page` / `.fy-models-page`.
  A keyboard-accessible vertical separator resizes the
  rail between 220px and min(420px, remaining width minus a 360px detail
  floor). Width is session-local component state and never enters the URL or
  storage. Double-click restores the default clamp.
- QoderWork/TRAE/WorkBuddy/Grok Build/OpenCode and Claude link actions render
  `CatalogOfficialLinks` → `ExternalLinkButton` with the catalog HTTPS URL.
  That control is the only jump: `useOpenExternal` holds one FeatureProvider
  lock and calls `settings.openExternal(link.url)`. Official product/cli/desktop
  links live on the Agent directory, not the Models page. Models must not clone
  those catalog links as 「打开官方设置」 or 「打开 TRAE 官方模型设置」.
  These actions do not inspect login state, download packages, read
  private config, persist notes, accept an API key, or emit configuration
  success.
- Official catalog links render in the Agent detail identity, top-right, as
  `CatalogOfficialLinks` primary buttons. Display copy for labels that already
  contain `官方` stays as catalog text; `cli`/`desktop` labels become
  `打开 {catalog label} 官网`. The renderer does not rewrite Rust labels or
  URLs.
- FeatureProvider keeps one external-open lock and one pending URL. Agent
  detail does not keep a second lock. A failure toasts fixed text without
  echoing the URL. Codex renders no official link region and mounts the
  managed installer panel only while Codex is selected, immediately below
  the identity heading; leaving Codex releases its event subscription.
- Agent directory does not lazy-read WorkBuddy status or Provider summaries.
  Those observations belong on the Models page. External runtime status still
  preserves `null` as unknown when a future launch control is added.
- Qoder Hooks and Qoder/TRAE MCP validation remain native commands. The Agent
  directory is not their host; Qoder Models states 官方不支持第三方模型配置
  and does not jump to `/mcp`.
- Configuration actions navigate only with a known non-secret `target` query.

### Models target selection

- The exact selector order is QoderWork CN, TRAE Work CN, WorkBuddy, Grok
  Build, Codex, Claude Code, OpenCode. Missing, empty, or unknown `target`
  resolves to QoderWork CN. Side-rail items show the catalog label only; do
  not add a subtitle under the name. Never 测试模型连接 or
  在 OpenCode 中完成模型设置.
- All seven selectors use the same reviewed local Agent asset map. No selector
  image is loaded from a remote URL.
- Target state is component-local. API keys and form content never enter the
  hash, URL query, local/session storage, or cross-target state. The Models
  page stays mounted after its first visit: leaving for another primary route
  hides it (`hidden`/`inert`) instead of unmounting, so in-session form
  content including API keys remains until a write's terminal outcome or the
  persistent page actually unmounts. The other five primary routes keep the same
  in-session page. Target panels that have been opened stay
  mounted and hidden the same way. Process reload still starts empty.
- TRAE Work CN custom models are owned by TRAE cloud `model` / `model_list`.
  Catalog label TRAE Work CN is not a second Application Support folder; after
  v0.1.18 it is the renamed TRAE SOLO desktop app whose store is still TRAE
  SOLO CN. FyAgent `get_traework_model_ids` may read the TRAE SOLO CN
  `state.vscdb` colon key `{userId}:AI.agent.model.model_list_map` as a
  secret-free observation of currently cached custom IDs. FyAgent must not
  fetch-and-save into that sqlite document: TRAE launch refreshes it from
  cloud `model_list` and drops local-only rows. The Models panel states that
  custom models must be added in TRAE Work CN and never claims sqlite writes
  will appear in the Work CN UI. Never 请回 TRAE 保存. Switching Models targets
  or hiding the page for another primary route does not clear other targets'
  in-session forms; those values still never enter query cache, URL, or
  storage. Actual unmount of the persistent Models page still clears keys and
  cancels an in-flight probe.

### Design Decision: TRAE Work CN 自定义模型不写本地库

**Context**: Writing custom rows into TRAE SOLO CN `state.vscdb` can succeed on
disk, but Work CN listing is owned by cloud `model` / `model_list`. Launch
overwrites local-only rows that were never registered with `add_custom_model`.

**Options Considered**:
1. Keep sqlite SAVE and change copy to “请回 TRAE 保存”
2. Call TRAE cloud `add_custom_model`
3. GET observation plus vendor-UI guidance

**Decision**: Option 3. Catalog `models.validate` / `models.write` are
`assisted` + `vendor_ui_required`. There is no `fetch_traework_models` or
`save_traework_models`. The Models page does not collect a TRAE API key.

**Example**:
```ts
await ports.traeWork.getModelIds();
```

**Extensibility**: A future cloud-register command would be a new native
contract, not a sqlite upsert.

### Common Mistake: writing TRAE custom models into `state.vscdb`

**Symptom**: FyAgent reports SAVE success, but TRAE Work CN still does not list
the model after launch.

**Cause**: Work CN listing is owned by cloud `model` / `model_list`. Launch
overwrites local-only rows that lack a server `custom_model_id`.

**Fix**: Do not add `save_traework_models` or `fetch_traework_models`. Observe
cached IDs with `ports.traeWork.getModelIds()` and tell the user to add the
model in TRAE Work CN.

**Prevention**: Catalog `models.validate` / `models.write` stay `assisted` +
`vendor_ui_required`. The Models page must not collect a TRAE API key or mount
fetch/save controls.

- Every displayed model ID renders a ~14px decorative local vendor icon from
  `src/v2/shared/assets/models` via `resolveModelVendorIcon(modelId, ownedBy?)`.
  Unknown IDs use the bundled `unknown.svg`. Remote icon URLs are forbidden.
- OpenCode uses `opencodeModels.getSnapshot` / `fetchProviderModels` /
  `saveModels` / `checkModel` with the same chip/fetch/save UX as
  WorkBuddy. Snapshot IDs are sanitized; `get_opencode_models` (CLI runtime
  list) is not the write path.

### WorkBuddy

- Cache only sanitized status and model-ID DTOs. The API key lives in component
  memory and native discovery/save requests. A successful or failed fetch keeps
  the key so the user can review the draft and save without re-entering it.
  Save terminal outcomes still clear the key. Switching Models targets, hiding
  the page behind another primary route, and in-session keep-alive do not.
  Actual unmount of the persistent Models page still clears it. A visibility
  toggle may reveal the value in the input only; it never enters query cache,
  URL, storage, notices, or logs.
- Existing third-party model IDs are grouped by model family and start
  collapsed. Clicking a chip remove asks for confirmation that the model
  configuration will be deleted and cannot be recovered; confirming writes
  immediately via `removedModelIds` and does not wait for 「保存并应用」. The
  renderer may auto-replay one backend overwrite token after that UI
  confirmation so the user is not asked twice. Fetch and manual entry share one
  draft list: pull merges remote IDs, fill adds typed IDs, and save splits the
  draft back into selected versus manual IDs. Both the existing list and the
  draft list can be filtered by model ID. The panel does not display backup,
  configuration-file status, or the persisted-key-clear checkbox.
- Discovery, revision, overwrite capability, atomic persistence, concurrent
  modification, and authoritative reread follow the backend WorkBuddy
  contract. The UI freezes one exact overwrite request and replays it only with
  its opaque one-time token.
- A remote response or local document in which a model ID contains a complete credential
  fails closed before DTO/query/DOM construction. The frontend repeats the
  collision rejection before save as defense in depth.
- Saving is disabled while authoritative status/model IDs are unavailable.
  Reread copy says "confirmed" only after both queries succeed. The save
  control lives in the sticky detail heading with the panel title, not in a
  trailing section below the form. Unsaved draft IDs or connection input show
  a `待保存` badge.
- WorkBuddy, Claude, Codex, Grok Build, and OpenCode expose 「测试连通」 only
  after the panel has at least one selectable model ID (pulled or typed).
  The control is `ModelConnectivityTest`: it opens a picker with model-ID
  search and group filters, then calls `checkModel`. Failure copy must show
  the backend `message` (upstream HTTP body), not a generic network hint.
  Qoder and TRAE Models must not render that button.

### Claude Code, Codex, and Grok Build

- Validate trimmed nonempty name/key/model, an absolute HTTP(S) Base URL with a
  host and no userinfo/query/fragment, reserved-ID collision, public-field
  credential collision, and credential-in-URL collision in both renderer and
  Rust. Errors are generic and never echo the field values.
- Claude, Codex, and Grok Build all expose 「拉取模型」 through
  `providers.fetchModels` and render grouped chips for fetched plus typed IDs.
- Claude Code only: if the typed Base URL pathname contains an explicit `v1`
  segment, show a warn-only FieldFeedback that the Claude client will call
  `/v1/v1/XXXX` and that the usual path is `/v1/XXXX`. Hostname `v1.example.com`
  is not a v1 path. The warning must not block save. Codex and Grok Build must
  not show this warning. The Claude placeholder must not include `/v1`.
- The V2 port is `applyQuickSetupWithResult(request, app)`. Codex may attach
  optional `codexFeatures.imageExtension` / `codexFeatures.websockets`; Claude
  and Grok Build must omit `codexFeatures`. Grok Build uses reserved ID
  `fyagent-v2-quick-setup-grokbuild` and live `~/.grok/config.toml`. OpenCode
  is not a `ProviderAppId` and must not call this port.
- Rust derives one stable reserved Provider ID per app. The renderer cannot
  submit a generic Provider, arbitrary ID, category, metadata, usage script,
  icon, sort order, or live-config fragment.
- Codex `imageExtension: true` derives `requires_openai_auth = false` plus
  provider-scoped `experimental_bearer_token` equal to `apiKey`, because
  current Codex ignores `auth.json` in that mode. `imageExtension: false`
  keeps `requires_openai_auth = true` and writes `apiKey` only to
  `auth.OPENAI_API_KEY`. The host owns this projection; the renderer still
  sends only the minimum request.
- One per-app/config critical section serializes quick setup with every writer
  of the same Provider/current/live files. Guarded internals never reacquire the
  non-reentrant public lock.
- The operation captures exact task-owned DB/current/live/backup/runtime
  preimages, applies the normalized request, synchronizes current/live state,
  and compensates every mutated surface if a later required step fails.
  Non-critical projection warnings are explicit; an incomplete compensation is
  `ROLLBACK_PARTIAL_STATE_UNKNOWN`, not success and not "rolled back" copy.
- Compute `warningCodes` from this normalized, committed request while the
  per-app guard is still held. The command must not release the guard and reread
  the fixed reserved row to infer warnings, because a later serialized request
  may already own that row.
- File/database work runs off the Tauri IPC/UI thread. Repeat clicks are locked
  in the renderer, but backend serialization is authoritative across windows
  and other callers.
- After success, reread the sanitized Provider snapshot. Claim only that the
  fixed Quick Setup Provider ID is active when `currentId` equals the reserved
  ID. This is not proof that the reread contains this request's exact bytes: a
  later serialized writer may legitimately have replaced the same reserved
  row. A failed/mismatched reread is unconfirmed even when apply returned
  success. The apply control lives in the sticky detail heading with the panel
  title, not at the end of the form.
- Codex may report live-byte change and stable warning codes. Restart, process
  availability, model availability, login, subscription reuse, and endpoint
  health are separate and remain unclaimed.

### Sanitized Provider summary

- The native summary command builds one snapshot under the same app guard and
  returns only `id` and `name`. It never serializes generic Provider settings,
  notes, metadata, website/category, usage credentials, or live fragments to
  V2.
- Before projection, inspect every app-specific credential carrier, including
  settings JSON, Codex TOML bearer fields, and usage-script credentials. If a
  public ID/name collides with a credential, fail the whole summary generically.
- Map key must equal summary ID. A nonempty current ID must exist in the same
  safe map. The Tauri adapter runtime-validates this exact wire again before
  React Query sees it.
- The normal browser adapter returns native-only unavailability. Rich fixtures
  live only in focused tests and are always labelled/non-authoritative.

## 4. Validation & Error Matrix

| Condition                                                                                  | Required result                                                                                         |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Catalog version/order/ID/link/capability/evidence state drifts                             | Exact Rust/V2 contract test fails                                                                       |
| V1 `officialUrl`, catalog v2/future, unknown enum, duplicate ID, non-HTTPS URL, or Codex link arrives | Runtime parse fails; catalog is unavailable                                                  |
| Codex is selected                                                                          | Show the managed installer below the identity heading and no official-link button                       |
| A non-Codex entry is selected                                                              | Do not read or subscribe to the Codex installer                                                         |
| Native external open fails                                                                 | Show fixed controlled failure text; do not install or configure                                         |
| QoderWork/TRAE selected                                                                    | Only catalog-declared and native-port capabilities are available; vendor-private writes remain unavailable |
| Models Qoder/TRAE shows 「打开官方设置」 or 「打开 TRAE 官方模型设置」                      | Component test fails; Qoder has no 「管理 MCP」; TRAE stays guidance-only                                 |
| Models Qoder/TRAE shows 「测试连通」                                                       | Component test fails; model probe belongs on WorkBuddy, Claude, Codex, Grok Build, and OpenCode only      |
| Models 「测试连通」 renders before any selectable model ID                                 | Component test fails; the button is owned by `ModelConnectivityTest` and hidden when `modelIds` is empty |
| `stream_check_url` is empty, not HTTP(S), `file://`, missing a host, or has userinfo/query/fragment | Command error `服务地址无效` or `base_url 为空`; no network probe and no API key |
| Models 「测试连通」 calls `checkReachability` / `stream_check_url` or `stream_check_provider` | Port/page test fails; model probe is `checkModel` → `stream_check_model` only |
| `stream_check_model` is empty, not HTTP(S), `file://`, missing a host, has userinfo/query/fragment, or has an empty model ID | Command error `服务地址无效` / `base_url 为空` / `模型 ID 为空`; no model request |
| Model probe failure hides the upstream body behind generic network copy                    | Component test fails; `message` from `checkModel` is shown to the user |
| Claude Base URL pathname contains a `v1` segment                                           | Warn-only FieldFeedback; save remains enabled                                                             |
| Native observation fails on Models                                                         | Show controlled unavailable/unknown; never infer absence                                                |
| Runtime value is unknown                                                                   | Preserve `null`/`unverified`; never display "not installed"                                            |
| Agent directory mounts Hooks editor, MCP validation, observation, or unsupported lists     | Page test fails; those surfaces stay off the Agent directory                                            |
| Non-Codex Agent detail omits 「产品介绍」 or Codex detail shows that region                 | Page test fails; intros are page-local copy, never catalog `description`                                |
| Agent directory intro or Codex installer copy names FyAgent                                | Intro/page/installer test fails; Agent directory copy describes the third-party product only            |
| QoderWork CN catalog description or Agent intro mentions Hooks                             | Catalog/intro test fails; Qoder user-facing copy must not name Hooks                                    |
| TRAE Models attempts sqlite save or fetch-and-apply                                        | Forbidden; GET observation and catalog guidance only, never 请回 TRAE 保存                              |
| TRAE/OpenCode GET snapshot or Debug/log contains `ak`/`sk`/`apiKey`                        | Security regression test fails                                                                          |
| External MCP result contains an original env/header value                                  | Reject the result and expose no copy action                                                             |
| Models target missing or unknown                                                           | Select QoderWork CN; issue no write                                                                     |
| OpenCode is the Models target                                                              | Mount `opencodeModels` CRUD; do not call Provider quick setup or the Codex installer                    |
| A displayed model ID has no local vendor icon resolver                                     | Asset mapping test fails; never load `https?://` icons                                                  |
| Any selector lacks a local icon                                                            | Asset mapping/unit/browser gate fails                                                                   |
| WorkBuddy remote/local ID contains a complete API key                                      | Generic fail-closed error before DTO/cache/DOM/write                                                    |
| WorkBuddy revision or overwrite token drifts                                               | Write nothing; reread before claiming state                                                             |
| Provider Base URL has userinfo/query/fragment or a credential component                    | Reject before DB/current/live mutation                                                                  |
| Provider request is empty, generic, wrong-ID, or has public/secret collision               | Reject in Rust; no state mutation                                                                       |
| Codex `imageExtension: true` omits `experimental_bearer_token` while `requires_openai_auth` is false | Host derivation/test fails; current Codex would not send `auth.json`'s key |
| Concurrent Provider/live writer                                                            | Serialize or detect conflict; never return a split DB/current/live state                                |
| Required atomic step fails and compensation succeeds                                       | Return `APPLY_FAILED_ROLLED_BACK`; UI may say rollback confirmed                                        |
| Compensation is incomplete                                                                 | Return `ROLLBACK_PARTIAL_STATE_UNKNOWN`; stop writes and state that authority is unknown                |
| Mutation succeeds but sanitized reread fails/mismatches                                    | Show the atomic apply result as unconfirmed; never claim fixed-ID activation                            |
| Mutation succeeds and another serialized request replaces the reserved row                 | Keep this request's guard-time warnings; reread may confirm only fixed-ID activation, never exact bytes |
| Browser preview calls authoritative read/write                                             | Return native-only unavailable; never return production-looking fake state                              |
| API key appears in URL/storage/query/log/error/DOM/snapshot                                | Security regression test fails                                                                          |

## 5. Good / Base / Bad Cases

- Good: `/models` opens on QoderWork CN at the top, all seven local icons
  render, Qoder states 官方不支持第三方模型配置, does not render 「管理 MCP」
  or 「打开官方设置」 or 「测试连通」. TRAE Models has no
  「打开 TRAE 官方模型设置」 and no 「测试连通」. Grok Build sits after WorkBuddy and uses
  Provider quick setup. After a model ID exists, WorkBuddy/Claude/Codex/Grok
  Build/OpenCode 「测试连通」 opens a searchable grouped picker and
  `checkModel` shows the upstream error body on failure.
- Good: OpenCode's Models panel lists existing sanitized provider/model IDs,
  fetches, adds, deletes, and saves through `opencodeModels`; it never submits
  Provider quick setup.
- Good: TRAE Models states that custom models must be added in TRAE Work CN,
  does not mount a fetch/save editor, and `get_traework_model_ids` may list
  currently cached custom IDs without `ak`/`sk`. Fixture sqlite under
  `FYAGENT_TEST_HOME` never points at the interactive TRAE profile.
- Good: Qoder Hooks saves a previewed revisioned request and reports the
  required restart without claiming the running process consumed it.
- Good: Claude Code renders independent CLI and Desktop official-site actions
  in the detail identity, while Codex renders no link and reuses the existing
  native installer contract through a V2 port, placed below the title.
- Good: a Codex quick setup passes one minimum request to Rust, applies under the
  shared config lock, returns request-attributed non-secret warnings/live-change
  state, clears the key, and describes a matching `currentId` reread only as
  fixed-ID activation confirmation.
- Base: browser preview renders the pages but authoritative panels report that
  desktop state is unavailable; test-only fixtures may exercise UI branches.
- Bad: hard-code a second capability matrix, treat `null` runtime as absence,
  pass an executable/path to launch, retain a key for retry, expose an MCP
  secret template, compose generic add/update/switch calls in React, or say
  "rolled back" after partial compensation.

## 6. Tests Required

Run:

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
```

Required focused coverage includes:

- exact catalog v4 version/order/variant/capability/mode/reason/evidence/link
  ID/label/HTTPS matrix and v2/v3/future/unknown/excess fail-closed cases,
  Grok Build product URL, Claude CLI/Desktop order, OpenCode product/CLI order,
  Codex zero-link behavior, and command registration;
- seven local Agent assets, official Qoder PNG / TRAE PNG digests, Qoder default,
  exact Models order including Grok Build, master/detail keyboard/ARIA, four
  maintained viewports; displayed model IDs resolve only bundled vendor SVGs;
- exact official-link IPC through `ExternalLinkButton` /
  `useOpenExternal`, renderer official-site display labels in the
  identity, one FeatureProvider lock/error behavior, Codex negative-link
  behavior, and negative download/login/config behavior;
- independently scrolling catalog panes, a clamped keyboard/pointer separator,
  the 760px stack hiding that separator, and the shared catalog page inset
  (`gap: 0`, no extra Agents/Models page `gap` or `padding-top`);
- normal browser native-only reads/writes and rich fake-Tauri test isolation;
- WorkBuddy plus OpenCode discovery success/truncation/failure/duplicate
  lock, revision, frozen overwrite, expired token, TOCTOU, authoritative
  reread, API-key lifecycle, and malicious ID/credential collisions;
  TRAE GET tests use fixture sqlite, prefer the colon Work CN key when
  an underscore IDE map also exists, and assert GET DTO JSON has no `ak`/`sk`.
  TRAE Models tests prove guidance copy and the absence of fetch/save
  controls; they never invoke `save_traework_models`.
- minimum Provider request/unknown-field rejection, fixed derived IDs/shapes,
  empty/URL/credential collisions, success warnings, current reread mismatch,
  full rollback, rollback-partial structured outcome, and secret-free errors;
  Codex image-extension derivation must write `experimental_bearer_token`
  when `requires_openai_auth` is false and must keep the key in `auth.json`
  only when image-extension is off;
- barrier/timeout tests across quick setup and generic add/update/delete/switch,
  current/live reapply, MCP config writers, post-write observation failure, and
  all Codex live/catalog files; no deadlock and no split state;
- concurrent same-reserved-ID tests prove each response keeps warnings computed
  for its own guarded request, and renderer copy never treats an ID-only reread
  as exact configuration-content confirmation;
- Provider summary app allowlist, credential carriers, exact DTO, key/ID/current
  consistency, Tauri runtime parser, React Query/DOM secret-negative scans;
- StrictMode replay, repeat-click locks, no API
  key in DOM/hash/localStorage/sessionStorage/query cache or logged fixtures.
  Models page keep-alive across primary-route switches and previously opened
  target panels; the other primary routes keep the same in-session page.
  Secrets stay in component memory only. Immediate WorkBuddy
  existing-model delete after an unrecoverable-delete confirmation.
- Models Qoder/TRAE details must not render 「打开官方设置」 or
  「打开 TRAE 官方模型设置」; Qoder states 官方不支持第三方模型配置 and
  has no 「管理 MCP」 or 「测试连通」.
  WorkBuddy, Claude, Codex, Grok Build, and OpenCode expose 「测试连通」 only
  after selectable model IDs exist. The picker searches and filters by group,
  then `checkModel` sends a real streaming request. Failure shows the
  backend `message`. `stream_check_model` validates the draft HTTP(S) URL the
  same way as `validate_probe_url` and never resets the circuit breaker.
  Claude, Codex, and Grok Build all expose 「拉取模型」.
  Claude shows a warn-only explicit `/v1` pathname notice and a placeholder
  without `/v1`.
  Agent directory tests prove only `direct` capability jumps, no capability-item
  grid or catalog description, shared official
  primary buttons, page-local 「产品介绍」 on non-Codex details, Codex without
  that region, no FyAgent host copy on Agent directory surfaces, and the absence of observation/Hooks/MCP panels. Product
  pages have no outer h1/subtitle.

Browser tests prove renderer/IPC wiring only. Rust tests prove service/command
contracts. Real Windows Tauri HIL and an isolated/reversible native mutation are
separate acceptance evidence.

## 7. Wrong vs Correct

Wrong: write TRAE custom models into local sqlite, or write OpenCode through
Provider quick setup.

```ts
showNotice("请回 TRAE 保存");
await ports.traeWork.saveModels(request);
await ports.providers.applyQuickSetupWithResult(request, "opencode");
```

Correct: observe TRAE cached IDs only and persist OpenCode through its port.

```ts
await ports.traeWork.getModelIds();
await ports.opencodeModels.saveModels(request);
```

Wrong: let the renderer submit and activate a generic Provider in independent
steps.

```ts
await ports.providers.updateWithResult(app, provider);
await ports.providers.switchWithResult(app, provider.id);
```

Correct: submit the minimum request once, then independently confirm the safe
native snapshot.

```ts
await ports.providers.applyQuickSetupWithResult(
  {
    name,
    baseUrl,
    apiKey,
    modelId,
  },
  app,
);
const summary = await ports.providers.getSummary(app);
if (summary.currentId !== QUICK_SETUP_PROVIDER_IDS[app]) {
  showUnconfirmedState();
}
```

Wrong: read the first catalog URL or manufacture a Codex website action in the
renderer.

```ts
await ports.settings.openExternal(entry.officialLinks[0].url);
```

Correct: official catalog links belong on the Agent directory. Models Qoder/TRAE
panels must not clone those links as settings buttons. Qoder Models has no MCP
jump.

```ts
<InlineNotice>官方不支持第三方模型配置</InlineNotice>
```

Wrong: probe a saved Provider, or use URL-only reachability as 「测试连通」,
or hang that button on Qoder/TRAE, or show it before any model ID exists.

```ts
await invoke("stream_check_provider", { appType: "claude", providerId });
await ports.providers.checkReachability(baseUrl.trim());
<Button>测试连通</Button> // Qoder / TRAE, or before models are pulled
```

Correct: `ModelConnectivityTest` is the only Models owner. It stays hidden
until `modelIds.length > 0`, then `checkModel` → `stream_check_model`.
Qoder and TRAE Models have no probe method or button. Failure copy uses
`result.message`.

```ts
<ModelConnectivityTest
  modelIds={draftModelIds}
  onProbe={(modelId) =>
    ports.workbuddy.checkModel({
      app: "workbuddy",
      baseUrl: baseUrl.trim(),
      apiKey: apiKeyRef.current.trim(),
      modelId,
    })
  }
/>
```

Wrong: stack a Models page flex gap on top of the shared feature header
margin, so the catalog columns sit lower than Agent directory.

```css
.fy-models-page {
  gap: 16px;
}
```

Correct: both catalog pages use only `.fy-feature-page` padding (20px).
Agent and Models omit the page-level `.fy-feature-header`. `.fy-catalog-page`
keeps `gap: 0`.
