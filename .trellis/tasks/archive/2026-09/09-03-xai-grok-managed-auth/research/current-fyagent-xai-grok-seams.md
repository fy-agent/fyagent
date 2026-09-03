# Research: Current FyAgent xAI / Grok seams

- **Query**: locate xAI OAuth manager, Grok Agent HandoffOnly, `auth.json` / GROK_HOME / helper / registry seams
- **Scope**: internal
- **Date**: 2026-09-03
- **FyAgent tree**: `ea2ecdc9efb681760fbd9b581465162384d08a25` (`dev/laiyongjie`)

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/proxy/providers/xai_oauth_auth.rs` | xAI Device Code manager; writes `xai_oauth_auth.json`, not Grok `auth.json` |
| `src-tauri/src/commands/xai_oauth.rs` | SuperGrok quota via manager token; comments that Grok CLI `auth.json` is a different file |
| `src-tauri/src/commands/auth.rs` | Compatibility Auth Center commands; `xai_oauth` is a closed provider id |
| `src-tauri/src/proxy/forwarder.rs` | Per-request xAI token: vault resolver first, else `XaiOAuthManager` |
| `src-tauri/src/services/managed_auth/service.rs` | Vault refresh for `ManagedAuthProvider::Xai` with `refresh_owner=fyagent` |
| `src-tauri/src/services/managed_auth/migration.rs` | Migrates `xai_oauth_auth.json` → vault as `purpose=proxy_upstream` |
| `src-tauri/src/services/managed_auth/core.rs` | Enums already include `grok_native` purpose/owner |
| `src-tauri/src/database/schema.rs` | CHECK includes `purpose=grok_native`, `refresh_owner=grok_native` |
| `src-tauri/src/agent_install/auth_actions.rs` | Grok observation/login/logout is HandoffOnly |
| `src-tauri/src/agent_install/auth_sessions.rs` | `HandoffComplete` → `HandoffOnly` without verification poll |
| `src-tauri/src/agent_install/types.rs` | Closed `HandoffOnly` reason/outcome/DTO |
| `src-tauri/src/grok_config.rs` | Live `config.toml` dir: settings override or `~/.grok` |
| `src-tauri/src/settings.rs` | `grok_config_dir` override; no `GROK_HOME` env read |
| `src-tauri/src/services/subscription_grok.rs` | Read-only parse of Grok `auth.json` for official-quota footer |
| `src-tauri/src/services/env_checker.rs` | `GROK_HOME` / `GROK_BIN_DIR` are **not** treated as credential env conflicts |
| `src-tauri/src/services/tooling/grok.rs` | macOS Grok **install/update** owner (`native_internal` / `official_npm`) |
| `src-tauri/user-helper/src/grok.rs` | Helper actions: `observe` / `install` / `update` only |
| `src-tauri/user-helper/src/cli.rs` | `grok-tool` wire; no Auth action |
| `src/v2/shared/platform/tauri/feature-ports/grokTooling.ts` | V2 install/npm port; no Auth |
| `src/config/grokBuildProviderPresets.ts` | Official Grok preset: empty config, CLI-owned login |
| `src/components/providers/forms/GrokBuildProviderForm.tsx` | Official hint: FyAgent does not write `~/.grok/auth.json` |
| `src/components/settings/DirectorySettings.tsx` | UI for `grokConfigDir` override |
| `.trellis/spec/backend/external-agent-auth.md` | Grok has no verified status surface |
| `.trellis/spec/backend/external-agent-lifecycle.md` | Install/npm owner is Tooling, not Auth |
| `.trellis/spec/backend/managed-auth.md` | Proxy must not refresh `grok_native` sessions |

No matches in product code for `auth_provider_command` or `GROK_AUTH_PROVIDER_COMMAND`.

### Code Patterns

#### 1. xAI manager is Proxy-purpose Device Code, not Grok native store

`xai_oauth_auth.rs` documents OIDC discovery + Device Authorization Grant. Constants:

```19:29:src-tauri/src/proxy/providers/xai_oauth_auth.rs
const XAI_ISSUER: &str = "https://auth.x.ai";
const XAI_DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
const XAI_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
const XAI_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
const XAI_USER_AGENT: &str = "fyagent-xai-oauth";
```

Storage path is FyAgent data dir `xai_oauth_auth.json`, not Grok home:

```216:226:src-tauri/src/proxy/providers/xai_oauth_auth.rs
    pub fn new(data_dir: PathBuf) -> Self {
        let manager = Self {
            // ...
            storage_path: data_dir.join("xai_oauth_auth.json"),
```

Device flow POSTs `client_id` + `scope` to the discovered device endpoint; poll uses `grant_type=urn:ietf:params:oauth:grant-type:device_code` and classifies `authorization_pending` / `slow_down` / `access_denied` / `expired_token` (`239:346:src-tauri/src/proxy/providers/xai_oauth_auth.rs`).

Discovery requires issuer `https://auth.x.ai` and HTTPS host `auth.x.ai` port 443 with empty userinfo (`865:995:src-tauri/src/proxy/providers/xai_oauth_auth.rs`).

Refresh uses `grant_type=refresh_token` with the same client_id and scope; `invalid_grant` / `invalid_token` / HTTP 401/403 mark reauth (`892:918:src-tauri/src/proxy/providers/xai_oauth_auth.rs`). Rotated refresh tokens are persisted under a per-account lock (`576:622:src-tauri/src/proxy/providers/xai_oauth_auth.rs`).

Unix write is `0600` temp+rename; Windows is create_new temp, delete destination, rename (`798:838:src-tauri/src/proxy/providers/xai_oauth_auth.rs`). Vault seal skips plaintext writes (`644:648:src-tauri/src/proxy/providers/xai_oauth_auth.rs`).

#### 2. Two quota paths; same public client_id, different files

```15:20:src-tauri/src/commands/xai_oauth.rs
/// 与 `get_codex_oauth_quota` 平行：数据走 fyagent 自管的 xAI OAuth token，
/// 而非 Grok CLI 的 ~/.grok/auth.json。两者是同一个 OAuth client
/// （client_id 与 Grok CLI 一致），token 对 grok.com 账单端点等效，因此
/// 复用 `subscription_grok::query_grok_quota`
```

Grok official footer reads Grok CLI `auth.json` as a **scope→entry map**, prefers `https://auth.x.ai::` then legacy `https://accounts.x.ai/sign-in`, uses field `key` as Bearer, **does not refresh**:

```1:14:src-tauri/src/services/subscription_grok.rs
//! 读取 Grok CLI 的 OAuth 凭据（~/.grok/auth.json），调用 grok.com 的
//! gRPC-web billing 端点查询 SuperGrok 订阅的 credit 用量。
//! ...
//! - token 刷新由 Grok CLI 自己负责（约 7 天过期），本模块只读不刷新，
```

```38:40:src-tauri/src/services/subscription_grok.rs
fn read_grok_credentials() -> GrokCredentials {
    let auth_path = crate::grok_config::get_grok_config_dir().join("auth.json");
```

#### 3. Agent Grok is HandoffOnly

If `grok` CLI is available, observation is unverified handoff with Login+Logout; otherwise observer unavailable:

```69:86:src-tauri/src/agent_install/auth_actions.rs
        AgentCatalogId::GrokBuild => {
            let available = tokio::task::spawn_blocking(|| ensure_tool_available(GROK_TOOL_ID))
                .await
                .ok()
                .and_then(Result::ok)
                .is_some();
            if available {
                handoff_only_observation(
                    agent_id,
                    vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
                )
            } else {
                unavailable_observation(
                    agent_id,
                    AgentAuthOwnership::AgentOwned,
                    AgentAuthReasonCode::AuthObserverUnavailable,
                )
            }
        }
```

Login launches `grok login`; logout runs `grok logout`. Both return `HandoffComplete` immediately:

```163:169:src-tauri/src/agent_install/auth_actions.rs
        (AgentCatalogId::GrokBuild, AgentAuthIntent::Login) => {
            launch_closed_cli(GROK_TOOL_ID, "grok login", "grok_login")?;
            Ok(AuthLaunchDisposition::HandoffComplete)
        }
        (AgentCatalogId::GrokBuild, AgentAuthIntent::Logout) => {
            run_closed_cli(GROK_TOOL_ID, &["logout"])?;
            Ok(AuthLaunchDisposition::HandoffComplete)
        }
```

Session runtime does **not** poll for verified login. `HandoffComplete` terminalizes as `HandoffOnly`:

```542:551:src-tauri/src/agent_install/auth_sessions.rs
    if disposition == AuthLaunchDisposition::HandoffComplete {
        let current = observe_agent_auth(request.agent_id).await;
        let _ = store.transition(
            &session_id,
            AgentAuthSessionStage::HandoffComplete,
            current,
            Some(AgentAuthSessionOutcome::HandoffOnly),
            Some(AgentAuthReasonCode::HandoffOnly),
        );
        return;
    }
```

Spec restates this: `.trellis/spec/backend/external-agent-auth.md` lines 97–98, 133, 145–147, 175.

DTO construction (`504:516:src-tauri/src/agent_install/auth_actions.rs`) sets `AgentAuthOwnership::AgentOwned`, `AgentAuthAuthority::Unverified`, `AgentAuthReasonCode::HandoffOnly`.

#### 4. GROK_HOME vs FyAgent override

Official Grok home is `$GROK_HOME` or `~/.grok`. FyAgent live config dir is **settings `grok_config_dir`**, else `HOME/.grok`:

```25:28:src-tauri/src/grok_config.rs
pub fn get_grok_config_dir() -> PathBuf {
    crate::settings::get_grok_override_dir().unwrap_or_else(|| get_home_dir().join(".grok"))
}
```

```997:1002:src-tauri/src/settings.rs
pub fn get_grok_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .grok_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}
```

There is no `std::env::var("GROK_HOME")` read in `src-tauri`. Env-conflict checker explicitly ignores `GROK_HOME` (`214:226:src-tauri/src/services/env_checker.rs`). Settings UI exposes `grokDir` (`src/components/settings/DirectorySettings.tsx`, `SettingsPage.tsx`).

Official provider copy states FyAgent does not write `~/.grok/auth.json` (`src/config/grokBuildProviderPresets.ts` lines 41–53; i18n `grokOfficialHint`).

#### 5. Helper is install lifecycle, not Auth

`user-helper` `GrokToolAction` is Observe/Install/Update (`64:68:src-tauri/user-helper/src/grok.rs`). V2 port invokes `install_official_npm` / `install_native` (`src/v2/shared/platform/tauri/feature-ports/grokTooling.ts`). Auth child must not treat this helper as `auth_provider_command`.

#### 6. Managed-auth vault already isolates Proxy xAI from `grok_native`

Legacy xAI JSON migrates only as Proxy + FyAgent owner:

```428:447:src-tauri/src/services/managed_auth/migration.rs
        result.push(LegacyCredentialInput {
            migration_id: XAI_MIGRATION_ID,
            provider: ManagedAuthProvider::Xai,
            purpose: CredentialPurpose::ProxyUpstream,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            // ...
            refresh_owner: RefreshOwner::Fyagent,
```

Resolver refuses non-proxy purposes and non-fyagent owners (`680:708:src-tauri/src/services/managed_auth/service.rs`). Schema CHECK already lists `grok_native` (`src-tauri/src/database/schema.rs` ~1738–1743). No Grok consumer adapter writes `purpose=grok_native` yet.

Forwarder prefers vault material then falls back to `XaiOAuthManager` (`1744:1767:src-tauri/src/proxy/forwarder.rs`).

### Related Specs

- `.trellis/spec/backend/managed-auth.md` — Proxy `purpose=proxy_upstream` + `refresh_owner=fyagent`; native owners never refreshed by Proxy
- `.trellis/spec/backend/external-agent-auth.md` — Grok handoff_only
- `.trellis/spec/backend/external-agent-lifecycle.md` — Grok npm/Mainland install owner
- `.trellis/spec/frontend/user-facing-copy.md` — install copy must not claim mainland sign-in/inference

## Caveats / Not Found

- No FyAgent writer for Grok `auth.json`, `auth.json.lock`, or `[auth] auth_provider_command`.
- FyAgent does not parse official `GrokAuth` fields (`auth_mode`, `refresh_token`, `oidc_client_id`, `user_id`, team principal). Quota reader only needs `key` + optional `expires_at`.
- Whether a user-exported `GROK_HOME` and FyAgent `grok_config_dir` can diverge on a real machine is not proven here; source shows they are independent knobs.
- `commands/xai_oauth.rs` “same OAuth client” refers to **client_id**, not a shared refresh-token lineage. Current stores are separate files.
