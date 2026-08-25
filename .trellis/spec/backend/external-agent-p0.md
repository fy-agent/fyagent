# External Agent P0 Safety Contract

## 1. Scope / Trigger

Read this contract before changing the static Agent catalog, external-agent
runtime observation or launch, QoderWork/TRAE Work/WorkBuddy Skill or MCP
targets, Qoder Hooks, TRAE model endpoint preflight and read-only
`state.vscdb` observation, OpenCode `opencode.json` model persist, external MCP
validation, the Agent install-readiness and closed install/auth action façade,
or their Tauri permissions. These capabilities are deliberately
narrower than Provider,
proxy, prompt, session, installer, and vendor-private configuration domains.

P0 proves FyAgent-owned validation and controlled local file operations.
Vendor application detection, launch, Skill recognition inside the vendor UI,
Qoder restart effects, and model compatibility remain separate HIL evidence
and must stay `unverified` when that evidence was not executed. OpenCode
model file writes are in-scope native contracts; they are not proof that the
vendor process loaded the new rows. FyAgent must not write TRAE model sqlite;
Work CN listing requires TRAE cloud `add_custom_model`.

## 2. Signatures

The static catalog is deterministic, non-networking, and exact-versioned:

```text
get_agent_catalog() -> {
  contractVersion: 3,
  reviewedAt,
  agents: [{
    id, variantId, displayName, description,
    officialLinks: [{ id, label, url }],
    capabilities: [{ id, mode, reasonCode, evidenceIds }]
  }]
}
```

`id` is one of
`qoderwork | trae-work | workbuddy | grokbuild | codex | claude-code | opencode`,
in that catalog order. Pi is not a catalog ID. TRAE `displayName` is
`TRAE Work CN`; product URL is
`https://www.trae.cn/sem-work`. Every entry declares the same closed
11-capability sequence:

```text
product.open app.detect app.launch skills.read skills.write
hooks.read hooks.write models.validate models.write mcp.validate mcp.write
```

Runtime and launch are separate commands and accept no renderer path, URL, or
executable:

```text
get_external_agent_status({ agentId })
  -> { agentId, detected, running, version, installSource, capabilities }

launch_external_agent({ agentId, destination })
  -> { agentId, destination, state, reasonCode }

get_agent_install_readiness(agentId)
  -> AgentInstallReadinessDto

start_agent_action({ agentId, action, expectedReleaseId? })
  -> AgentActionResult | AgentActionErrorDto

cancel_agent_action({ jobId })
  -> AgentActionJobSnapshot | AgentActionErrorDto

get_agent_action_job({ jobId })
  -> AgentActionJobSnapshot | AgentActionErrorDto
```

`detected` and `running` are `boolean | null`. `destination` is exactly
`home | skills | hooks | models | mcp`. Install/action commands accept no
URL, path, command, token, hash, `packageFormat`, or bypass field. Request
serde uses `deny_unknown_fields`.

```text
agentId = qoderwork | trae-work | workbuddy | grokbuild | codex | claude-code | opencode
action  = install | update | launch | auth_login | auth_logout | auth_connect_provider
expectedReleaseId = "v1:" + 64 lowercase hex   // opaque; never a URL

AgentInstallReadinessDto {
  contractVersion: 2,
  agentId, reviewedAt,                 // reviewedAt = YYYY-MM-DD
  installState, updateState,
  releaseId?, localVersion?, remoteVersion?,
  authOwnership, authState, sourceKind,
  allowedActions, reasonCodes
}

AgentActionResult {
  contractVersion: 1,
  agentId, action, jobId?, stage, reasonCode?
}

AgentActionJobSnapshot {
  contractVersion: 1,
  jobId, agentId, action, stage, cancellable, reasonCode?
}

AgentActionErrorDto { reasonCode }
```

Qoder Hooks, TRAE preflight, TRAE model-id observation, and OpenCode model
commands are:

```text
get_qoderwork_hooks()
save_qoderwork_hooks({ request })

validate_traework_model_config({ request })
test_traework_model_endpoint({ requestId, request })
cancel_traework_model_endpoint({ requestId })
get_traework_model_ids()
get_opencode_model_snapshot()
fetch_opencode_provider_models({ request })
save_opencode_models({ request })
validate_external_mcp_config({ agentId, config })
```

Native Skills `SkillTargetId` is the leftover six AppType values plus
`qoderwork`, `trae-work`, and `workbuddy`. Native `McpTargetId` is the leftover
six AppType values plus the same three Skills/MCP-domain IDs. Direct V2
assignment uses catalog order
`qoderwork | trae-work | workbuddy | grokbuild | codex | claude | opencode`.
QoderWork, TRAE Work, and WorkBuddy never convert to `AppType`. Catalog
contract version is 4 and includes Grok Build (`https://x.ai/grok`).
`get_all_installed` may observe disk skills under every native
`SkillTargetId` without writing SQLite or SSOT.

## 3. Contracts

### Catalog, runtime, and permissions

- Catalog v2, future versions, unknown enums, excess fields, duplicate IDs,
  invalid order, and invalid official links fail closed in Rust and TypeScript.
- Static capability declarations never read local state. Runtime observation
  never converts unknown to false or infers installation from a settings path.
- Launch is positive only through a trusted executable/bundle/signing adapter.
  Without one it returns a controlled `unverified` or `unavailable` result.
- Tauri permissions keep observe, launch, Qoder write, and endpoint probe as
  separate sets. Because defining the first application ACL manifest makes
  Tauri enforce ACL for every application command, the local `main` capability
  also carries an explicit compatibility set covering the complete pre-ACL
  handler surface. The compatibility and feature-specific sets are disjoint,
  their union must equal the registered handler commands, no remote origin is
  granted, and no generic filesystem/shell permission is introduced.

### Install readiness and closed actions

- `get_agent_install_readiness` remains the only readiness projection. Its
  selector is the catalog's exact seven `AgentCatalogId` values; there is no
  second catalog, registry, generic installer, probe, doctor, or helper surface.
  Closed actions reuse this façade:
  `start_agent_action({ agentId, action, expectedReleaseId? })`,
  `cancel_agent_action({ jobId })`, and `get_agent_action_job({ jobId })`.
  Renderer input is only a catalog ID, a closed action
  (`install | update | launch | auth_login | auth_logout | auth_connect_provider`),
  and an optional opaque backend-generated `expectedReleaseId`. URL, path,
  command, token, hash, and bypass fields are rejected.
- Contract version is 2. The bounded DTO contains only contract version,
  canonical ID, reviewed timestamp, closed install/update/auth states, optional
  opaque `releaseId`, sanitized local/remote version strings, auth ownership,
  source kind, allowed actions, and closed reason codes. URL, path, hash,
  script, secret, package path, `packageFormat`, `managed_package`, and signer
  fingerprint fields are forbidden.
- Claude Code, Grok Build, and OpenCode install/update reuse Tooling
  (`claude` / `grok` / `opencode`). Gemini CLI, OpenClaw, and Hermes stay on
  their existing Tooling surfaces and must not be routed through this façade.
  Codex install/update remain `managed_by_codex_desktop` and keep the dedicated
  Codex Desktop installer; this façade must not occupy that job slot.
- QoderWork CN uses the fixed first-party `/qoder-work-cn/releases/latest/`
  User-x64 / macOS ARM64 / macOS x64 aliases. Remote semver stays `unknown`;
  do not invent a version from Last-Modified, ETag, or docs. TRAE Work CN
  resolves `data.solo` + `region=cn` from the official latest API and never
  reads `data.manifest` / TRAE Code. WorkBuddy uses `/v2/update` closed
  platform IDs and the official macOS `.zip -> .dmg` suffix rewrite. Source,
  schema, allowlist, or probe failure surfaces `source_not_verified` /
  official-page fallback; never pin a researched version URL.
- Windows EXE artifacts are not installed from elevated FyAgent. macOS DMG
  install is the current closed package-format path. The Catalog desktop
  path currently uses a bounded `hdiutil`/`ditto` deploy, not the full Codex
  Desktop atomic replace/rollback/job-directory adapter. That gap is
  residual risk, not a license to add a second downloader. Formal elevated
  Windows CLI/auth automation stays unavailable unless a later authenticated
  ordinary-user helper with closed enums is proven.
- Fail dominates and unknown never upgrades to green. Readiness creates no
  plan snapshot. Auth ownership is `fyagent_managed` (Codex Auth Center),
  `agent_owned` (Claude/Grok/desktop apps), or `provider_owned` (OpenCode
  connect-provider). FyAgent never reads vendor credential files.

#### Scenario: Agent install and closed-action façade

##### 1. Scope / Trigger

- Trigger: Agent Catalog install/update/launch/auth is no longer read-only
  readiness. This is a new Tauri command set plus cross-layer DTO, so
  code-spec depth is mandatory.
- Owner: `src-tauri/src/agent_install/` and
  `commands/agent_install_readiness.rs`. Codex Desktop install/update stay
  on [Codex Desktop Installer](./codex-desktop-installer.md). Formal
  elevated Windows CLI stays on
  [Windows Runtime Security](./windows-runtime-security.md).
- Pi remains out of catalog, install, auth, UI, and tests.

##### 2. Signatures

```text
get_agent_install_readiness(agentId: AgentCatalogId)
  -> AgentInstallReadinessDto

start_agent_action(StartAgentActionRequest)
  -> Result<AgentActionResult, AgentActionErrorDto>

cancel_agent_action(jobId: String)
  -> Result<AgentActionJobSnapshot, AgentActionErrorDto>

get_agent_action_job(jobId: String)
  -> Result<AgentActionJobSnapshot, AgentActionErrorDto>
```

Closed wire enums (snake_case):

```text
installState = not_installed | installed | installed_not_runnable | unknown | unavailable
updateState  = unavailable | unknown | up_to_date | update_available | latest_unknown
authOwnership = fyagent_managed | agent_owned | provider_owned | unavailable
authState     = unknown | logged_in | logged_out | provider_connection_required | unavailable
sourceKind    = cli_tooling | managed_desktop | codex_desktop
stage         = checking | downloading | installing | verifying_installation
                | succeeded | failed | cancelled
reasonCode    = official_page_only | source_not_verified | platform_unsupported
                | interactive_user_unavailable | installed_not_runnable
                | auth_state_unknown | provider_connection_required
                | credential_store_unsupported | binding_account_missing
                | binding_identity_mismatch | operation_conflict | cancelled
                | managed_by_codex_desktop | native_projection_unavailable
                | refresh_required | executor_not_implemented
```

No environment key. Product hosts are Rust constants, not env or renderer
input.

##### 3. Contracts

- Request: only `agentId + action` and optional opaque `expectedReleaseId`.
  Response: closed states, optional opaque `releaseId`, sanitized version
  strings, ownership, allowed actions, reason codes. Forbidden on the wire:
  URL, path, hash, script, token/secret/`apiKey`, `packageFormat`,
  `managed_package`, signer fingerprint.
- Mapping:
  - `claude-code` / `grokbuild` / `opencode` → Tooling `claude` / `grok` /
    `opencode` (`sourceKind=cli_tooling`). Gemini CLI, OpenClaw, Hermes stay
    on existing Tooling surfaces.
  - `qoderwork` / `trae-work` / `workbuddy` → first-party managed-desktop
    adapters (`sourceKind=managed_desktop`).
  - `codex` install/update → `managed_by_codex_desktop`, empty
    `allowedActions`, no Agent job slot.
- QoderWork CN: code-owned `https://static.qoder.com.cn/qoder-work-cn/releases/latest/`
  User-x64 EXE / macOS ARM64 DMG / macOS x64 DMG. Redirect host allowlist is
  `static.qoder.com.cn`. `remoteVersion` stays absent. Action means “current
  latest alias”, not a claimed semver. Windows ARM64 is
  `platform_unsupported`.
- TRAE Work CN: metadata `https://api.trae.cn/icube/api/v1/native/version/trae/cn/latest`
  with same-schema fallback `api.trae.ai`. Select `data.solo` + `region=cn`.
  Download host `lf-cdn.trae.com.cn`, path prefix
  `/obj/trae-com-cn/pkg/app/releases/stable/`, filename
  `TraeWork_CN-*`. Never `data.manifest` or `TraeCode_*`.
- WorkBuddy: `https://www.workbuddy.cn/v2/update?platform=` one of
  `workbuddy-darwin-x64 | workbuddy-darwin-arm64 | workbuddy-win32-x64-user`.
  Download host `download.codebuddy.cn`. macOS rewrites the exact `.zip`
  suffix to `.dmg` after allowlist validation.
- `releaseId` is `v1:` plus SHA-256 hex of a canonical field list. TRAE and
  WorkBuddy require `expectedReleaseId` to match a force-refreshed source.
  Qoder may omit it; if present it must still match the refreshed alias
  hash. Drift → `refresh_required`.
- Fetch: HTTPS only, no userinfo, **no explicit non-default port**, every
  redirect hop rechecked against the product host allowlist, hop cap inherited
  from the installer transport,
  metadata ≤ 1 MiB, artifact ≤ 2 GiB. Scheme downgrade / unknown host /
  excess hops → `source_not_verified` and official-page fallback. A cancelled
  fetch is `cancelled`, not a source/schema failure. Never pin
  a researched version URL such as TRAE `2.3.76922` or WorkBuddy
  `5.3.14.36279234`.
- Jobs: one in-process slot. Second non-terminal start →
  `operation_conflict`. Cancel is allowed until `installing`; after that
  `cancellable=false`. Download success is not installed; post-install
  reread is required. Unknown bundle identity stays `unknown`.
- Windows EXE/NSIS is a recognized format that reports
  `interactive_user_unavailable` from elevated FyAgent. macOS DMG is the
  current closed deploy path.
- Auth: Claude may run official `claude auth login/logout` and parse
  official `claude auth status` JSON/exit-code only. Grok launches official
  `grok login/logout`; status stays `unknown` without a structured command.
  OpenCode launches the official CLI connect flow and, when detected, shows
  `provider_connection_required` rather than a global logged-in bool.
  Qoder/WorkBuddy `auth_login` stays `auth_state_unknown` until a verified
  bundle/login surface exists. TRAE `auth_login`/`launch` may open the
  trusted TRAE app when present. Never read vendor token files or Keychain.

##### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown Agent ID, unknown action, or extra request field | Reject; no job |
| Renderer supplies URL/path/command/token/hash/bypass | Reject; no job |
| `expectedReleaseId` not `v1:`+64 hex | `refresh_required` |
| TRAE/WorkBuddy missing or drifted `expectedReleaseId` | `refresh_required` |
| Qoder provided `expectedReleaseId` that no longer matches the alias | `refresh_required` |
| Codex `install`/`update` through this façade | `managed_by_codex_desktop` |
| Host/scheme/redirect/body/schema/artifact grammar fails | `source_not_verified` or `official_page_only`; no stale version URL |
| Explicit non-default port on a metadata or artifact URL | `source_not_verified` + `official_page_only`; default HTTPS only |
| Fetch/job is cancelled | `cancelled`; never remap cancel to `source_not_verified` |
| Windows ARM64 desktop source, or unsupported arch | `platform_unsupported` |
| Windows EXE from elevated FyAgent | `interactive_user_unavailable` |
| Formal elevated Windows Claude/Grok/OpenCode CLI/auth | `interactive_user_unavailable` |
| Second overlapping job, unknown `jobId`, or cancel after `installing` | `operation_conflict` |
| TRAE `data.manifest` / `TraeCode_*` selected | `source_not_verified`; Work package not started |
| Secret/token/URL appears in DTO, error, log, or DOM | Security regression |

##### 5. Good/Base/Bad Cases

- Good: TRAE fixture `data.solo`/`region=cn` resolves `2.3.76922` Work
  packages; `data.manifest` Code packages are rejected. Renderer starts
  install only with the backend `releaseId`.
- Good: WorkBuddy fixture parses `5.3.14.36279234` and the official
  `.zip → .dmg` rewrite. Those version strings are test fixtures, not
  production fallbacks.
- Base: Qoder readiness has no `remoteVersion`. Install revalidates the
  fixed `/latest/` alias and does not invent semver from Last-Modified/ETag.
- Bad: start a job with a CDN URL, route Gemini through this façade, occupy
  the Codex Desktop job slot, or paint logged-in from `~/.claude` file
  existence.

##### 6. Tests Required

- Closed enum/DTO: exact keys, `deny_unknown_fields`, forbidden-wire scan,
  opaque `releaseId` grammar, seven catalog IDs, no Pi.
- Source parsers: Qoder three aliases + Windows ARM64 unsupported + no
  invented semver; TRAE solo/CN vs manifest/Code; WorkBuddy three platform
  IDs + `.zip → .dmg`; host/scheme/userinfo/redirect/body-cap failures.
- Jobs: single-flight, cancel-before-installing, refuse cancel after
  `installing`, unknown job id, Codex install/update reason code.
- Tooling mapping only: Claude/Grok/OpenCode. Existing Gemini/OpenClaw/Hermes
  lifecycle tests remain green and unrouted.
- Auth negatives: no vendor credential-file reads; OpenCode is not a global
  auth bool; Claude status uses official JSON/exit-code when present.
- ACL union includes `start_agent_action`, `cancel_agent_action`,
  `get_agent_action_job`. Renderer port parses at the adapter, never in
  page-local casts.
- Native DMG/EXE/UAC HIL is residual risk, not a portable-test pass.

##### 7. Wrong vs Correct

#### Wrong

```ts
await invoke("start_agent_action", {
  agentId: "qoderwork",
  action: "install",
  url: "https://static.qoder.com.cn/.../QoderWorkCN-Setup-User-x64.exe",
});
```

#### Correct

```ts
const readiness = await ports.agentInstallReadiness.get("qoderwork");
await ports.agentInstallReadiness.startAction({
  agentId: "qoderwork",
  action: "install",
  expectedReleaseId: readiness.releaseId ?? undefined,
});
```

#### Wrong

```rust
readiness.remote_version = last_modified_header; // fake Qoder semver
fallback_url = "https://lf-cdn.trae.com.cn/.../2.3.76922/..."; // researched pin
```

#### Correct

```rust
// Qoder remoteVersion stays None; action revalidates the /latest/ alias.
// TRAE/WorkBuddy failures return official_page_only, never a pinned version URL.
```

### Design Decision: Qoder 不伪造远端 semver

**Context**: QoderWork CN only exposes versionless `/releases/latest/` aliases.
Last-Modified/ETag/docs are not a semantic version.

**Options Considered**:

1. Invent `remoteVersion` from HTTP validators or research notes
2. Keep remote semver unknown and install “current latest”
3. Disable Qoder install until a version API exists

**Decision**: Option 2. TRAE/WorkBuddy keep strict `expectedReleaseId`
coherence because their APIs expose a real version.

**Example**:

```text
Qoder: versionless_latest = true, display_version = None
TRAE/WorkBuddy: expectedReleaseId must match refreshed opaque releaseId
```

**Extensibility**: If Qoder later publishes a first-party version endpoint,
add it as a source adapter change; do not start guessing from headers.

### Common Mistake: 把调研时的版本 URL 当成 fallback

**Symptom**: TRAE/WorkBuddy API fails and the installer still downloads
`2.3.76922` / `5.3.14.36279234`.

**Cause**: Treating a research fixture as a stale-recovery package.

**Fix**: Surface `source_not_verified` / official-page fallback.

**Prevention**: Fixture versions belong only in tests, never in production
constants.

### Skills and persistence

- Database schema 18 adds default-false `enabled_workbuddy` on Skills (and the
  MCP `enabled_workbuddy` column). Schema 17 already added Skills
  `enabled_qoderwork` and `enabled_trae_work`. Schema 19 adds the matching MCP
  columns `enabled_qoderwork` and `enabled_trae_work` default false. Migration
  preserves every legacy row and leftover Gemini / Grok / Hermes flags; DAO
  reads and writes all stored flags.
- QoderWork, TRAE Work, and WorkBuddy Skill destinations are derived only from
  trusted home as `.qoderworkcn/skills`, `.trae-cn/skills`, and
  `.workbuddy/skills`. All three are copy-only destinations inside the shared
  `SkillService::sync_to_app_dir` / `remove_from_target` path. V2 Skills
  install, assign, unassign, ZIP, restore, and import for every catalog target
  (`qoderwork`, `trae-work`, `workbuddy`, `grokbuild`, `codex`, `claude`,
  `opencode`) use `install_skillhub` / `install_from_zip` /
  `restore_from_backup_for_target` / `toggle_skill_app` / `import_from_apps`;
  do not add per-agent Skill commands. `import_from_apps` syncs only dests
  that are not already present. Vendor directory-swap checks compare volume +
  inode, not mtime. Qoder Hooks remain trusted-home `.qoderwork/settings.json`
  and must not be retargeted to `.qoderworkcn`.
- Direct MCP live files: WorkBuddy writes trusted-home `.workbuddy/mcp.json`;
  QoderWork CN writes `{trusted-home}/.qoderworkcn/mcp.json`; TRAE Work CN
  writes TRAE SOLO CN `User/mcp.json`. Each is a Claude-style `mcpServers` map,
  backs up first, and skips when neither the home/User directory nor the file
  exists. Do not write Qoder `userData/mcp.json` or TRAE `state.vscdb` for MCP.
  WorkBuddy may import hidden `.mcp.json` when the official `mcp.json` is
  absent, and a first official write may seed from that hidden file. Qoder
  import may normalize `type: "streamable-http"` to `http` before validation.
- Catalog `mcp.write` for QoderWork CN and TRAE Work CN is `direct` +
  `dedicated_native_contract`. Agents “打开 MCP” appears only when
  `mcp.write === "direct"`.
- Target adapters reuse the existing SkillService archive, conflict, hash,
  copy, path-validation, and authoritative-reread behavior. Skill adapters do
  not enter Provider, proxy, prompt, session, or MCP live-file writers.

### Qoder Hooks document

- The only document is trusted-home `.qoderwork/settings.json`, bounded to
  2 MiB. IPC exposes only revision, existence, supported groups,
  `restartRequired: true`, and projection support.
- Structured writes support the closed event set, validate bounds without
  executing commands, preserve unknown top-level JSON, and replace only
  `hooks`. An unsupported hooks shape blocks the write.
- Save holds a per-document lock, compares the expected HMAC revision, and
  requires a bounded, expiring, request-digest-bound, one-use overwrite token
  when reviewed content has drifted. It writes backup first, uses a
  same-directory temporary file, flushes/syncs, atomically replaces, and
  authoritatively rereads.
- Windows operations pin and revalidate directory handles and reject
  reparse/hard-link/identity races. Errors never claim rollback or success when
  final authority is unknown.

### TRAE model endpoint preflight

- Validation accepts only the closed API format and URL-mode enums and returns
  a backend-generated canonical UUID v4. The same ID must be echoed into one
  endpoint request; at most 16 probes may be active and every terminal path
  removes its cancellation handle.
- API keys use a non-serializable redacted type and exist only for the current
  request. Public request fields may not equal the full credential.
- Default transport is HTTPS, zero redirect, 3-second connect timeout,
  10-second overall deadline, 1 MiB response-body cap, and no decompression.
- Resolve and classify all A/AAAA results before connecting, reject blocked or
  mixed classes, and pin the approved socket while retaining original
  Host/SNI. Explicit/system proxy modes fail with
  `PROXY_DNS_PIN_UNSUPPORTED` until that invariant can be proven; never fall
  back to direct.
- Results contain only the closed terminal state, reason code, duration bucket,
  status class, and request ID. They never include URL, model, key, response
  body, headers, or transport diagnostics.

### TRAE model observation

TRAE Work CN is the catalog product name. After v0.1.18 the desktop app is
the renamed TRAE SOLO; the live store on this host is still TRAE SOLO CN
`User/globalStorage/state.vscdb` (macOS
`~/Library/Application Support/TRAE SOLO CN/...`). There is no separate
`Application Support/TRAE Work CN` folder. Bundle id remains
`cn.trae.solo.app`. Catalog `models.validate` and `models.write` for TRAE
are `assisted` + `vendor_ui_required`.

#### Scenario: TRAE Work CN custom-model observation

##### 1. Scope / Trigger

- Trigger: `get_traework_model_ids` reads the local TRAE SOLO CN model-list
  cache so FyAgent can display currently cached custom IDs. This is not
  Work CN persist. The TRAE Work CN UI refreshes from TRAE cloud `model` /
  `model_list` and only keeps customs registered via `add_custom_model`.
  A running or launching TRAE process overwrites `state.vscdb` from that
  cloud list. FyAgent must not write this sqlite document.

##### 2. Signatures

- SQLite `ItemTable` in TRAE SOLO CN `state.vscdb`; `value` is TEXT
- `ItemTable.key` suffix `AI.agent.model.model_list_map`
- Work CN live key: `{userId}:AI.agent.model.model_list_map` (colon)
- IDE/legacy key: `{userId}_AI.agent.model.model_list_map` (underscore)
- Command: `get_traework_model_ids() -> { modelIds, revision, truncated }`
- There is no `save_traework_models` or `fetch_traework_models` command.

##### 3. Contracts

- GET opens the colon Work CN key when both keys exist. Underscore is the
  IDE map. `ORDER BY key LIMIT 1` is still forbidden; collect matching keys
  and prefer `:{suffix}` when present, else `_{suffix}`.
- GET projects secret-free custom rows (`is_preset == false`) from every
  present Work list, deduped. Prefer `display_name` or the `name` suffix
  after `//` as the model ID. `ak`/`sk` never appear in DTO, serde JSON,
  logs, or Debug.
- FyAgent never binds or replaces the colon document. Custom models must be
  added in TRAE Work CN. Local-only rows without a server `custom_model_id`
  are dropped when TRAE launches.

##### 4. Validation & Error Matrix

- Both colon and underscore keys exist -> read colon Work CN map
- Missing/unparseable document -> fail closed
- Custom id collides with a stored secret -> fail closed
- Any write/upsert into `state.vscdb` model-list map -> forbidden

##### 5. Good/Base/Bad Cases

- Good: GET on the live colon map returns cached custom IDs without `ak`/`sk`.
- Base: fixture sqlite under `FYAGENT_TEST_HOME` may still be colon-only
  with legacy `solo_work_lite`/`solo_work_remote` snake rows.
- Bad: prefer the underscore IDE key when the colon Work CN key exists,
  write sqlite to make Work CN list a model, or expose `save_traework_models`.

##### 6. Tests Required

- Fixture with both keys: GET uses the colon Work CN map and leaves the
  underscore IDE map unchanged.
- Legacy colon-only fixture: GET never serializes `ak`/`sk`.
- Command surface: `save_traework_models` / `fetch_traework_models` are not
  registered.

##### 7. Wrong vs Correct

- Wrong: write `{userId}:AI.agent.model.model_list_map` or treat sqlite as
  Work CN UI persist.
- Correct: prefer the colon key for GET observation only. Work CN listing
  requires TRAE cloud `add_custom_model`.

### Design Decision: TRAE Work CN 不把 sqlite 当作模型写入面

**Context**: Direct `ItemTable` upserts into `{userId}:AI.agent.model.model_list_map`
do not make Work CN list a model. Work CN refreshes from cloud `model` /
`model_list` and drops local-only customs.

**Options Considered**:

1. Continue SAVE into `state.vscdb` and tell users to reopen TRAE
2. Implement cloud `add_custom_model`
3. Remove fetch/save commands; GET observation only

**Decision**: Option 3. `get_traework_model_ids` is the only model-list IPC.
`fetch_traework_models` and `save_traework_models` must not exist.

**Example**:

```text
get_traework_model_ids() -> { modelIds, revision, truncated }
```

**Extensibility**: Cloud registration, if ever added, is a separate command
with its own closed DTO. It must not reuse sqlite upsert.

### Common Mistake: treating `state.vscdb` as Work CN persist

**Symptom**: `ItemTable` upsert into `{userId}:AI.agent.model.model_list_map`
succeeds, then TRAE launch drops the row.

**Cause**: Work CN refreshes from cloud `model` / `model_list`. Local-only
customs without `add_custom_model` are not listing authority.

**Fix**: Keep `get_traework_model_ids` as GET observation. Do not register
`save_traework_models` or `fetch_traework_models`.

**Prevention**: Any sqlite write of the model-list map is forbidden. GET DTO
JSON must not contain `ak` / `sk`. Prefer the colon Work CN key over the
underscore IDE key.

### OpenCode model persist

- Snapshot/save run under `lock_opencode_config` against live `opencode.json`
  providers. Snapshot IDs are sanitized. Unknown provider fields are preserved.
  GET JSON must not contain `apiKey`. Revision/overwrite semantics match
  WorkBuddy. `get_opencode_models` (CLI runtime list) is not the write path.

### External MCP validation

- Input is exactly `{ mcpServers: object }`; each entry is exactly stdio
  `{ command, args?, env? }` or HTTP `{ url, headers? }`.
- Reject unknown/mixed transport fields, prototype-pollution keys, control
  characters, unsafe URLs/addresses, and every configured size/count limit.
- Stdio checks path/executable availability without invoking a process, shell,
  installer, or server. HTTP performs URL and literal-address validation only;
  it performs no DNS or network operation.
- Findings expose only server ID, transport, closed reason codes,
  `boolean | null` executable availability, and `hasSecrets`. Templates replace
  all env/header values with `<redacted>` before IPC or clipboard use.

## 4. Validation & Error Matrix

| Condition                                                                                  | Required result                                                                            |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| Catalog version/order/enum/link drifts                                                     | Reject the whole catalog; do not render a legacy fallback                                  |
| Runtime detection is unavailable                                                           | Return `null`/`unverified`; never report not installed                                     |
| Launch lacks trusted runtime identity                                                      | Return controlled unverified/unavailable; start nothing                                    |
| Schema 16 data migrates                                                                    | Preserve all old rows/flags and default both new flags to false                            |
| Schema 17 data migrates to 18                                                              | Preserve leftover flags and default `enabled_workbuddy` to false                           |
| Schema 18 data migrates to 19                                                              | Preserve leftover flags and default MCP `enabled_qoderwork` / `enabled_trae_work` to false |
| Qoder/TRAE/WorkBuddy MCP home and file are both absent                                     | Skip live write; do not create the vendor directory                                        |
| TRAE sqlite model-list write is requested                                                  | Forbidden; GET observation only; Work CN listing requires `add_custom_model`               |
| TRAE/OpenCode GET JSON contains `ak`/`sk`/`apiKey`                                         | Security regression gate fails                                                             |
| WorkBuddy is added as `AppType`                                                            | Type test fails                                                                            |
| Skill destination is linked, escaped, raced, or hash-drifted                               | Fail closed; do not claim sync                                                             |
| Qoder JSON/hooks projection is unsafe                                                      | Return controlled unsupported/invalid result; write nothing                                |
| Qoder revision drifts                                                                      | Require one-use overwrite confirmation or return concurrent modification                   |
| TRAE URL/DNS/proxy cannot preserve policy                                                  | Return a closed rejection code before an unsafe connection                                 |
| TRAE request is cancelled/times out/fails                                                  | Remove active state and return only a sanitized terminal result                            |
| MCP server mixes transports or exceeds limits                                              | Reject; execute and persist nothing                                                        |
| A secret reaches DTO, error, log, DOM, query, storage, URL, snapshot, or default clipboard | Security regression gate fails                                                             |
| Install readiness receives an unknown/legacy Agent ID or an excess/sensitive DTO field     | Reject; do not fall back to another ID or infer readiness                                   |
| Renderer supplies URL/path/command/token/bypass, or a generic installer helper is requested | Reject; do not start a job or write command                                               |
| Pi is added to catalog, install, auth, UI, or tests                                        | Contract regression; catalog remains the existing seven IDs                               |
| TRAE/WorkBuddy `expectedReleaseId` is missing or drifted after refresh                     | `refresh_required`; do not start a download                                               |
| Qoder readiness invents `remoteVersion` from Last-Modified/ETag/docs                       | Contract regression; remote semver stays absent                                           |
| TRAE resolver selects `data.manifest` or a `TraeCode_*` URL                                | Fail closed; Work package is not started                                                  |
| Codex install/update is started on the Agent job slot                                      | `managed_by_codex_desktop`; Codex Desktop installer remains the owner                     |
| Gemini CLI / OpenClaw / Hermes is routed through the Agent façade                          | Contract regression; those Tooling surfaces stay independent                              |
| Formal elevated Windows CLI/auth or Windows EXE deploy is attempted from FyAgent           | `interactive_user_unavailable`; no generic path/shell helper                              |
| A second non-terminal Agent job starts, or cancel is requested after `installing`          | `operation_conflict`                                                                      |

## 5. Good / Base / Bad Cases

- **Good:** a Qoder hooks save preserves unrelated top-level keys, verifies the
  expected revision under lock, writes a backup and atomic replacement, rereads
  the file, and tells the renderer that restart is still required.
- **Good:** a TRAE probe validates a canonical request ID, approves and pins all
  resolved addresses, observes cancellation/deadline/body limits, then returns
  only `reachable` plus non-sensitive buckets.
- **Base:** an external Agent has no trusted runtime identity. Catalog guidance
  and official links remain available, while detection and launch stay
  `unverified`.
- **Good:** Agent detail reads one bounded readiness DTO and may start a closed
  `agentId + action` job. Codex install/update still delegate to the existing
  managed installer. Claude/Grok/OpenCode reuse Tooling. Qoder/TRAE/WorkBuddy
  use first-party source adapters.
- **Bad:** infer installation from `.qoderwork`, accept a renderer executable,
  route Qoder/TRAE through `AppType`, fall back around a proxy pin failure,
  execute an MCP command, or serialize a credential-bearing error.

## 6. Tests Required

Run the full host and renderer gates:

```powershell
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run lint:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
git diff --check
```

Focused Rust coverage must include catalog fail-closed parsing, status/launch
unknown semantics, schema 16-to-17, 17-to-18, and 18-to-19 preservation, Skill
path/TOCTOU /hash handling including WorkBuddy copy dest, Qoder
projection/revision/token, TRAE URL/DNS/pin plus fixture-sqlite GET
observation that prefers the colon Work CN key, projects present Work lists
without writing `state.vscdb`, and keeps GET DTO secret-free,
OpenCode snapshot/save, WorkBuddy `mcp.json` skip/write (hidden `.mcp.json`
import/seed only),
QoderWork `~/.qoderworkcn/mcp.json` skip/write, TRAE `User/mcp.json`
skip/write, and MCP union/no-execute/redaction. Agent install/action coverage must include
closed DTO/`deny_unknown_fields`/forbidden-wire scans, opaque `v1:` release-id
grammar, Qoder versionless aliases, TRAE `data.solo`/CN vs `data.manifest`/Code,
WorkBuddy platform IDs and `.zip → .dmg`, redirect allowlist failures, job
single-flight and post-`installing` cancel refusal, Codex
`managed_by_codex_desktop` non-occupation, Claude/Grok/OpenCode Tooling mapping
without rerouting Gemini/OpenClaw/Hermes, and auth-file non-reads. Renderer
tests must assert
exact command/payload wires, V2 seven Skills and seven MCP targets in catalog
order, leftover Gemini / Hermes backend flag round-trip, disk-observed
installed Skills, secret cleanup on every terminal or lifecycle path, catalog
geometry at the maintained viewports and 760/761px, keyboard/focus behavior,
and browser non-authority. ACL union still equals every `generate_handler!`
command, including `start_agent_action`, `cancel_agent_action`, and
`get_agent_action_job`.

The host permission test must derive the registered `generate_handler!`
commands and require exact equality with the disjoint union of all active app
permission manifests. Checking only the newly added commands is insufficient:
a partial app ACL silently revokes every unrelated application command.

Automated fixtures prove only their executed layer. They never upgrade real
vendor detection, launch, configuration acceptance, restart effectiveness, or
Skill loading to verified.

## 7. Wrong vs Correct

Wrong: write TRAE custom models into local sqlite.

```rust
save_traework_models_at(&paths, &request)?;
```

Correct: observe cached IDs only.

```rust
get_traework_model_ids_at(&paths)?;
```

Wrong: let the renderer choose process/filesystem/network authority.

```ts
await invoke("launch_external_agent", { path: form.executable });
await invoke("save_qoderwork_hooks", { path: form.settingsPath, rawJson });
await fetch(form.url, { headers: { Authorization: `Bearer ${apiKey}` } });
await invoke("start_agent_action", {
  agentId: "trae-work",
  action: "install",
  url: researchedCdnUrl,
});
```

Correct: send only closed IDs and bounded request DTOs to the narrow native
commands, then accept sanitized terminal results.

```ts
await invoke("launch_external_agent", {
  agentId: "qoderwork",
  destination: "hooks",
});
const validated = await invoke("validate_traework_model_config", { request });
await invoke("test_traework_model_endpoint", {
  requestId: validated.requestId,
  request,
});
const readiness = await ports.agentInstallReadiness.get("trae-work");
await ports.agentInstallReadiness.startAction({
  agentId: "trae-work",
  action: "install",
  expectedReleaseId: readiness.releaseId ?? undefined,
});
```

Wrong: retain secrets or manufacture positive vendor evidence.

```ts
queryClient.setQueryData(["trae", request], result);
localStorage.setItem("trae-key", apiKey);
setStatus("TRAE configuration saved");
```

Correct: keep credentials in the component/current invoke, clear them on every
terminal/lifecycle path, and describe success only as FyAgent preflight.

```ts
try {
  await ports.trae.testEndpoint(requestId, request);
} finally {
  clearSensitiveDraft();
}
```

Wrong: write Qoder builtin `userData/mcp.json` or TRAE `state.vscdb` for MCP.

```rust
let path = app_data_dir.join("mcp.json");
```

Correct: write only the vendor live MCP file, and skip when home/User and file
are both absent.

```rust
let path = home.join(".qoderworkcn").join("mcp.json");
if !home.exists() && !path.exists() {
    return Ok(());
}
```

Wrong: write WorkBuddy MCP to hidden `.mcp.json`, or copy QoderWork CN Skills
to international `.qoderwork/skills`.

```rust
home.join(".workbuddy").join(".mcp.json");
home.join(".qoderwork").join("skills");
```

Correct: official WorkBuddy MCP is `mcp.json`; QoderWork CN Skills are
`.qoderworkcn/skills`. Hooks stay `.qoderwork/settings.json`.

```rust
home.join(".workbuddy").join("mcp.json");
home.join(".qoderworkcn").join("skills");
```
