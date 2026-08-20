# Research: Claude Code v1 URL gate

- **Query**: Where Claude Code base URL is entered in V2 UI and backend; how URL is joined with `/v1`; exact files to add a warn-only gate when the user explicitly types a v1 path.
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

User requirement (item 6): during Claude Code model configuration, if the user explicitly types a URL that contains a v1 path, warn that Claude will call `/v1/v1/XXXX`. If there is no explicit v1, do not warn.

No such warn-only gate exists today in V2 Models or V1 Claude forms. V2 Claude placeholder currently includes `/v1`. Claude Code presets typically omit `/v1`. FyAgent’s **proxy** Claude adapter concatenates `base_url + endpoint` and then collapses `/v1/v1`; Claude Code itself uses `ANTHROPIC_BASE_URL` and the Anthropic SDK appends `/v1/...` without that collapse.

### Files Found

| File Path | Description |
|---|---|
| `src/v2/pages/models/Page.tsx` | V2 Claude/Codex/GrokBuild `ProviderPanel`; Claude base URL input + 拉取模型 |
| `src/v2/pages/models/quickSetup.ts` | Shared HTTP URL validation; no `/v1` warning |
| `src/v2/pages/models/feedback.tsx` | Field notices used by other model panels |
| `src/v2/shared/platform/tauri/features.ts` | `applyQuickSetupWithResult` / `fetchModels` IPC |
| `src-tauri/src/commands/provider.rs` | Quick setup writes `ANTHROPIC_BASE_URL` as typed |
| `src/components/providers/forms/ClaudeFormFields.tsx` | V1 Claude endpoint field |
| `src/components/providers/forms/hooks/useBaseUrlState.ts` | V1 writes `settings_config.env.ANTHROPIC_BASE_URL` |
| `src/components/providers/forms/shared/EndpointField.tsx` | Shared endpoint input; full-URL toggle hint only |
| `src/config/claudeProviderPresets.ts` | Claude presets: most `ANTHROPIC_BASE_URL` values have no `/v1` |
| `src-tauri/src/proxy/providers/claude.rs` | `extract_base_url`; `build_url` joins then strips `/v1/v1` |
| `src-tauri/src/services/model_fetch.rs` | `/v1/models` candidate construction for 拉取模型 |
| `.trellis/spec/frontend/v2-agent-models.md` | V2 Models quick-setup contract; no v1 warning |

## Code Patterns

### V2 UI — where Claude Code URL is typed

Models page targets include `claude` (`src/v2/shared/features/directory.ts:79-87`). `renderTargetPanel` mounts `ProviderPanel` for `claude` / `codex` / `grokbuild` (`Page.tsx:1349-1357`).

The Claude base URL field is the shared ProviderPanel input:

```1190:1206:src/v2/pages/models/Page.tsx
        <div className="fy-control-field">
          <label htmlFor={`${app}-quick-setup-base-url`}>服务地址</label>
          <Input
            ref={baseUrlInputRef}
            id={`${app}-quick-setup-base-url`}
            name={`${app}-quick-setup-base-url`}
            type="url"
            value={baseUrl}
            onChange={(event) => setBaseUrl(event.target.value)}
            placeholder="https://gateway.example/v1"
```

For `app === "claude"` the DOM id is `claude-quick-setup-base-url`. The same placeholder `/v1` is used for Codex and Grok Build. `validateQuickSetup` (`quickSetup.ts:63-106`) only checks HTTP(S), no userinfo, no query/hash, and API-key collision. It does not inspect path segments for `v1`.

Claude-only extra control is **拉取模型** (`Page.tsx:1272-1297`), which calls `ports.providers.fetchModels(baseUrl, apiKey)` **before** save. That is a `/models` list fetch, not a v1-path warning.

Save path: `validateQuickSetup` → `buildQuickSetupRequest` → `ports.providers.applyQuickSetupWithResult` (`Page.tsx:999-1041`). No warningCodes exist for Claude v1 (existing warningCodes are Codex-only: `CODEX_WEBSOCKET_*`).

### V2 backend — how the URL is stored

```235:244:src-tauri/src/commands/provider.rs
            AppType::Claude => (
                "fyagent-v2-quick-setup-claude",
                serde_json::json!({
                    "env": {
                        "ANTHROPIC_BASE_URL": base_url,
                        "ANTHROPIC_AUTH_TOKEN": api_key,
                        "ANTHROPIC_MODEL": model_id,
                    }
                }),
            ),
```

`base_url` is `self.base_url.trim()` with no `/v1` strip or reject (`provider.rs:215`). Reserved id is `fyagent-v2-quick-setup-claude`. Live write goes through `ProviderService::apply_quick_setup` (`services/provider/mod.rs:4079`).

Claude Code runtime reads `ANTHROPIC_BASE_URL` from live settings. FyAgent does not append `/v1` when writing that env var.

### V1 UI — same env key, different chrome

V1 Claude form:

- `ClaudeFormFields.tsx:747-774` — `EndpointField` id `baseUrl`, placeholder `providerForm.apiEndpointPlaceholder` (`zh.json:1166` = `https://your-api-endpoint.com`, **no** `/v1`).
- Hint `providerForm.apiHint` (`zh.json:1197`): “填写兼容 Claude API 的服务端点地址，不要以斜杠结尾”. No `/v1/v1` wording.
- `useBaseUrlState.ts:86-98` writes `config.env.ANTHROPIC_BASE_URL = sanitized` (trim only).
- Full-URL toggle (`EndpointField` + `isFullUrl`) is a **proxy** “do not concatenate path” switch, not a v1-path warning.

### How `/v1` is joined

**Claude Code / Anthropic SDK (direct):** `ANTHROPIC_BASE_URL` is the SDK `baseURL`. The client requests `/v1/messages` (and related `/v1/...` routes) relative to that base. If the user stores `https://api.example.com/v1`, the resulting path is `https://api.example.com/v1/v1/messages`.

**FyAgent proxy adapter (traffic through FyAgent):** `ClaudeAdapter::build_url` concatenates then collapses duplicate `/v1/v1`:

```854:865:src-tauri/src/proxy/providers/claude.rs
        let mut base = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );

        // 去除重复的 /v1/v1（可能由 base_url 与 endpoint 都带版本导致）
        while base.contains("/v1/v1") {
            base = base.replace("/v1/v1", "/v1");
        }
```

`extract_base_url` (`claude.rs:703-718`) returns `ANTHROPIC_BASE_URL` with trailing slashes stripped only.

**拉取模型 (`fetch_models_for_config`):** `model_fetch.rs:173-184` — if the typed base already ends with `/v{N}`, it appends `/models` not `/v1/models`. That path is OpenAI-compatible model listing, not Claude Code’s `/v1/messages` join. A Claude URL **without** `/v1` (preset style, e.g. `https://api.moonshot.cn/anthropic`) gets `{base}/v1/models` as the first candidate.

**Claude presets:** `src/config/claudeProviderPresets.ts` stores values such as `https://api.moonshot.cn/anthropic`, `https://api.siliconflow.cn`, `https://api.deepseek.com/anthropic` — typically **no** terminal `/v1`. Codex/OpenAI presets **do** use `/v1`. V2 Models reuses one placeholder for Claude and Codex.

### Exact files to add the gate

Warn-only, Claude only, when the user **explicitly** types a v1 **path** (not hostname `v1.example.com`).

| Layer | File | What exists | Gate placement |
|---|---|---|---|
| V2 Models (primary) | `src/v2/pages/models/Page.tsx` | `ProviderPanel` shared by claude/codex/grokbuild; Claude input `claude-quick-setup-base-url` | `onChange` / render of the 服务地址 field when `app === "claude"`; keep Codex/GrokBuild silent |
| V2 validation helper | `src/v2/pages/models/quickSetup.ts` | `validateQuickSetup` currently errors only | Optional warn helper (do not turn into a blocking `errors.baseUrl` unless product later requires block) |
| V2 notice chrome | `src/v2/pages/models/feedback.tsx` | `Notice` / `FieldFeedback` | Non-error warning copy: Claude will call `/v1/v1/XXXX`; usual path is `/v1/XXXX` |
| V2 spec | `.trellis/spec/frontend/v2-agent-models.md` | Quick-setup signatures; no v1 warning | Contract for Claude-only warn |
| V1 (if the same gate applies to classic provider form) | `src/components/providers/forms/ClaudeFormFields.tsx` | `EndpointField` at `:747` | Hint under the endpoint when path contains `/v1` |
| V1 shared input | `src/components/providers/forms/shared/EndpointField.tsx` | Hint slot already exists | Only if Claude passes a v1-specific hint; do not change Codex |
| V1 state | `src/components/providers/forms/hooks/useBaseUrlState.ts` | Trim-only write | Not required for warn-only |
| Backend write | `src-tauri/src/commands/provider.rs:235-244` | Stores URL as typed | Not required for warn-only; no reject today |
| Proxy join | `src-tauri/src/proxy/providers/claude.rs:829-865` | Collapses `/v1/v1` for **proxied** requests | Independent of the UI warning; Claude Code direct calls do not use this collapse |

Detection shape implied by the requirement: pathname contains an explicit `/v1` segment (examples: `https://gateway.example/v1`, `https://gateway.example/v1/`, `https://gateway.example/api/v1`). Absence of that segment → no warning. Hostname `v1.example.com` is not a v1 path.

## Related Specs

- `.trellis/spec/frontend/v2-agent-models.md` — Claude quick setup `fyagent-v2-quick-setup-claude`, `ANTHROPIC_*` live shape; no v1 path warning today.
- `.trellis/spec/backend/codex-provider-configuration.md` — Codex TOML `base_url` (often `/v1`); not Claude Code.

## Caveats / Not Found

- No existing UI string for `/v1/v1/XXXX`.
- V2 placeholder `https://gateway.example/v1` would itself trip an “explicit v1 path” warning on Claude if the gate matches the placeholder pattern.
- Proxy `build_url` collapse means **FyAgent-forwarded** Claude traffic may still succeed with a doubled path; **Claude Code talking to `ANTHROPIC_BASE_URL` directly** (proxy off, or Claude Code concatenating before the proxy) still produces `/v1/v1/...`.
- `isFullUrl` in V1 is a different switch (proxy uses the URL as a complete request URL). It is not the item 6 gate.
- Item 6 as stated is warn-only; current `validateQuickSetup` failures are blocking. Mixing them would change save behavior.