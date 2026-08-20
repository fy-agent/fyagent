# Research: Model-page connectivity test (except Qoder and Trae)

- **Query**: Old backend connectivity/probe/test-connection code; current V2 model pages; which products except qoder/trae; existing IPC; how to expose test-before-config.
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

User requirement (item 8): add connectivity test on V2 model pages except Qoder and Trae. Recover from old backend code. Purpose: allow a connectivity test **before** configuration (draft URL, not a saved provider).

Old backend is `stream_check_*`: GET `base_url`, any HTTP response = reachable, no model request, no API key required. It currently requires a **saved** `provider_id`. V2 Models has **no** stream-check port. Draft-URL network calls that already exist are `fetch_models_for_config` / WorkBuddy / OpenCode model list fetches (authenticated `/models`), which are not the same as reachability.

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/services/stream_check.rs` | Reachability service: GET base URL, TTFB, no auth, no breaker |
| `src-tauri/src/commands/stream_check.rs` | IPC: `stream_check_provider`, `stream_check_all_providers`, config get/save |
| `src-tauri/src/database/dao/stream_check.rs` | Logs + `stream_check_config` setting |
| `src/lib/api/connectivity-check.ts` | V1 invoke wrappers |
| `src/hooks/useStreamCheck.ts` | V1 toast UX |
| `src/components/providers/ProviderList.tsx` | Wires `checkProvider(provider.id)` |
| `src/components/providers/ProviderCard.tsx` | Hides test for `category === "official"` |
| `src/components/providers/ProviderActions.tsx` | “检测连通” button |
| `src/components/usage/ConnectivityCheckConfigPanel.tsx` | Settings: timeout / retries / degraded threshold |
| `src/v2/pages/models/Page.tsx` | V2 models: WorkBuddy + ProviderPanel + Qoder/Trae/OpenCode |
| `src/v2/pages/models/QoderModelsPanel.tsx` | Guidance only; no third-party config |
| `src/v2/pages/models/TraeModelsPanel.tsx` | Read-only model IDs; no write |
| `src/v2/pages/models/OpenCodeModelsPanel.tsx` | Draft URL + 拉取模型 before save |
| `src/v2/shared/features/ports.ts` | V2 ports; no stream-check; has `fetchModels` / WorkBuddy / OpenCode / Trae probe |
| `src/v2/shared/platform/tauri/features.ts` | Tauri adapters for those ports |
| `src-tauri/src/commands/model_fetch.rs` | `fetch_models_for_config(base_url, api_key, ...)` — draft URL, requires key |
| `src-tauri/src/services/model_fetch.rs` | GET `/v1/models` candidates |
| `src-tauri/src/commands/workbuddy.rs` | `fetch_workbuddy_models` |
| `src-tauri/src/commands/opencode_models.rs` | `fetch_opencode_provider_models` |
| `src-tauri/src/commands/traework.rs` | `test_traework_model_endpoint` (excluded product) |
| `.trellis/spec/frontend/v2-agent-models.md` | Models page product matrix |

## Code Patterns

### V2 model products (except Qoder and Trae)

`MODEL_DIRECTORY_IDS` (`directory.ts:79-87`): `qoderwork`, `trae`, `workbuddy`, `grokbuild`, `codex`, `claude`, `opencode`.

| Target | Panel | Item 8 | Current network-before-save |
|---|---|---|---|
| `qoderwork` | `QoderModelsPanel.tsx` | Exclude | None. Copy: 官方不支持第三方模型配置 |
| `trae` | `TraeModelsPanel.tsx` | Exclude | Read-only `getModelIds`. Native `test_traework_model_endpoint` exists but is not mounted on this panel |
| `workbuddy` | `WorkBuddyPanel` in `Page.tsx` | Include | `ports.workbuddy.fetchModels({ baseUrl, apiKey, allowNoApiKey })` then save |
| `grokbuild` | `ProviderPanel` `app="grokbuild"` | Include | No fetch/test button; save-only quick setup |
| `codex` | `ProviderPanel` `app="codex"` | Include | No fetch/test button; save-only quick setup |
| `claude` | `ProviderPanel` `app="claude"` | Include | “拉取模型” → `ports.providers.fetchModels(baseUrl, apiKey)` |
| `opencode` | `OpenCodeModelsPanel.tsx` | Include | `fetchProviderModels` with draft URL+key before save |

### Old backend connectivity (recover this)

`StreamCheckService` (`services/stream_check.rs:1-17`, `193-223`):

- GET user `base_url` (or Copilot override).
- Any HTTP status (200/4xx/5xx) → reachable (`success: true`).
- Only DNS / connect refused / TLS / timeout → failed.
- Latency = TTFB (`send()` returns on headers; body not read).
- Does **not** send a chat/completions/messages request.
- Does **not** touch the failover circuit breaker.
- Optional custom User-Agent from provider meta.
- Config: `timeout_secs` default 8, `max_retries` 1, `degraded_threshold_ms` 6000 (`stream_check.rs:50-59`).

IPC today (`commands/stream_check.rs`):

```16:24:src-tauri/src/commands/stream_check.rs
/// 连通性检查（单个供应商）
#[tauri::command]
pub async fn stream_check_provider(
    state: State<'_, AppState>,
    copilot_state: State<'_, CopilotAuthState>,
    app_type: AppType,
    provider_id: String,
) -> Result<StreamCheckResult, AppError> {
```

`stream_check_provider` loads the provider from DB by `provider_id`. Official `category == "official"` is skipped in batch (`stream_check.rs:78-80`) and errors in `resolve_base_url` (`stream_check.rs:169-172`). There is **no** command that accepts a draft URL.

Registered in `src-tauri/src/lib.rs:2096-2099`. Frontend: `src/lib/api/connectivity-check.ts`, `useStreamCheck.ts`. V1 UI: Provider list “检测连通” (`ProviderActions.tsx:308-324`), hidden for official (`ProviderCard.tsx:560-567`). Settings panel: `ConnectivityCheckConfigPanel`.

`probe_reachability` (`stream_check.rs:198-223`) is the reusable core: `(client, base_url, timeout, optional UA) -> HTTP status`. `check_once` still goes through `Provider` to resolve URL.

### Existing IPC that already takes a draft URL (not reachability)

These run **before** save but they are authenticated model-list probes, not `stream_check` reachability.

| IPC | File | Args | Behavior |
|---|---|---|---|
| `fetch_models_for_config` | `commands/model_fetch.rs:94-114` | `baseUrl`, `apiKey`, optional `isFullUrl` / `modelsUrl` / UA | GET OpenAI-compatible `/v1/models` (or `/models` if base already ends `/v{N}`); **API key required** |
| `fetch_workbuddy_models` | `commands/workbuddy.rs:31` | `WorkBuddyFetchModelsRequest` `{ baseUrl, apiKey, allowNoApiKey }` | Draft fetch; V2 WorkBuddy “拉取” button (`Page.tsx:211-218`, `:742`) |
| `fetch_opencode_provider_models` | `commands/opencode_models.rs:14` | same draft shape | OpenCode panel before save |
| `test_traework_model_endpoint` | `commands/traework.rs:20` | `requestId` + Trae model request | Vendor probe; item 8 excludes Trae |

V2 `ProvidersPort.fetchModels` (`ports.ts:123`, `tauri/features.ts:1446-1451`) invokes `fetch_models_for_config` with only `{ baseUrl, apiKey }` — no `isFullUrl`. Used by Claude 拉取模型 (`Page.tsx:952-977`). Codex and Grok Build share `ProviderPanel` but do not render that button (`Page.tsx:1272`).

V2 feature ports have **no** `streamCheck` / connectivity method (`ports.ts` `ProvidersPort` / `WorkBuddyPort` / `OpenCodeModelsPort`). Browser preview stubs `fetchModels` as `rejectNativeOnly` (`platform/browser/features.ts:70-75`).

### How to expose test-before-config (what exists to recover)

Item 8’s purpose is connectivity **before configuration**. Current `stream_check_provider(appType, providerId)` cannot do that: the V2 Claude/Codex/GrokBuild form is a draft until `apply_provider_quick_setup_with_result`; WorkBuddy/OpenCode drafts live in React state.

Recoverable pieces:

1. **`StreamCheckService::probe_reachability`** — already URL-shaped, no provider required internally. Callers today wrap it with DB provider lookup.
2. **`StreamCheckConfig`** — timeout/retry/degraded already persisted; V1 settings UI exists.
3. **Result DTO** `StreamCheckResult` — `status`, `success`, `message`, `responseTimeMs`, `httpStatus`.
4. **V1 UX** — toast copy in `useStreamCheck.ts` (reachable / slow / unreachable).

Gap vs item 8: there is no IPC of the form `stream_check_url({ baseUrl })` or `stream_check_draft({ app, baseUrl })`. Adding a V2 Models button that calls `stream_check_provider` would only work **after** a provider row exists (`fyagent-v2-quick-setup-claude` etc.), which is after configuration.

Related but different: “拉取模型” proves `/models` + API key, not “port/gateway reachable”. Official providers have no probe target; V2 quick-setup rows are `category: custom`, so they would be eligible if a draft URL command existed.

### Exact files if a before-config control is added

| Layer | File | Role |
|---|---|---|
| Recovered service | `src-tauri/src/services/stream_check.rs` | `probe_reachability` / `build_result` |
| New or extended IPC | `src-tauri/src/commands/stream_check.rs` + `src-tauri/src/lib.rs` | Today only `provider_id`; draft URL is not accepted |
| V2 port | `src/v2/shared/features/ports.ts` | No connectivity method today |
| V2 Tauri adapter | `src/v2/shared/platform/tauri/features.ts` | Wire invoke |
| V2 Claude/Codex/GrokBuild | `src/v2/pages/models/Page.tsx` `ProviderPanel` | No test button; Claude only has 拉取模型 |
| V2 WorkBuddy | `src/v2/pages/models/Page.tsx` `WorkBuddyPanel` | Has 拉取 (`fetchModels`), not reachability |
| V2 OpenCode | `src/v2/pages/models/OpenCodeModelsPanel.tsx` | Has 拉取, not reachability |
| Do not mount | `QoderModelsPanel.tsx`, `TraeModelsPanel.tsx` | Item 8 exceptions |
| Spec | `.trellis/spec/frontend/v2-agent-models.md` | No connectivity-before-save contract |

V1 `src/lib/api/connectivity-check.ts` / `useStreamCheck.ts` remain the saved-provider path; they are not used by V2 Models.

## Related Specs

- `.trellis/spec/frontend/v2-agent-models.md` — product matrix; Qoder `models.write` unsupported; TRAE `assisted` + vendor UI; WorkBuddy/OpenCode dedicated fetch/save; Claude/Codex/GrokBuild quick setup.
- No `.trellis/spec` document currently names `stream_check` or V2 connectivity-before-config.

## Caveats / Not Found

- No V2 Models connectivity button and no draft-URL stream-check IPC.
- `fetch_models_for_config` is not a drop-in recovery of `stream_check`: it requires an API key and hits `/models`.
- Trae already has `test_traework_model_endpoint`; item 8 says not to add connectivity on Trae/Qoder model pages.
- Reachability treats 401/403 as success (“gateway up”). That matches the old backend comment: 可达 ≠ 配置正确.
- Copilot base URL override in `stream_check` is V1-provider-specific; V2 Models products in item 8 do not use that path.