use indexmap::IndexMap;
use tauri::{Emitter, Manager, State};

use crate::app_config::AppType;
use crate::codex_config::CodexProviderFeatureIntent;
use crate::commands::copilot::CopilotAuthState;
use crate::commands::xai_oauth::XaiOAuthState;
use crate::error::AppError;
use crate::provider::{ClaudeDesktopMode, Provider, ProviderMeta, ProviderMutationResult};
use crate::services::provider::QuickSetupApplyFailureCode;
use crate::services::{
    EndpointLatency, ProviderService, ProviderSortUpdate, QuickSetupWriteTarget, SpeedtestService,
    SwitchResult,
};
use crate::store::AppState;
use std::str::FromStr;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPublicSummary {
    id: String,
    name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPublicSummaryResult {
    providers: IndexMap<String, ProviderPublicSummary>,
    current_id: String,
    write_targets: Vec<QuickSetupWriteTarget>,
}

const PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE: &str = "Provider public summary is unavailable";

fn push_nonempty_credential(value: &str, output: &mut Vec<String>) {
    let value = value.trim();
    if !value.is_empty() {
        output.push(value.to_string());
    }
}

fn push_header_value_credentials(value: &str, output: &mut Vec<String>) {
    let value = value.trim();
    push_nonempty_credential(value, output);

    if let Some((scheme, credential)) = value.split_once(char::is_whitespace) {
        if ["bearer", "basic", "token", "apikey"]
            .iter()
            .any(|candidate| scheme.eq_ignore_ascii_case(candidate))
        {
            push_nonempty_credential(credential, output);
        }
    }

    for cookie in value.split(';') {
        if let Some((_, cookie_value)) = cookie.split_once('=') {
            push_nonempty_credential(cookie, output);
            push_nonempty_credential(cookie_value, output);
        }
    }
}

fn is_header_container_key(key: &str) -> bool {
    ["headers", "http_headers", "env_http_headers"]
        .iter()
        .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

fn is_custom_header_string_key(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    key == "CUSTOM_HEADERS" || key.ends_with("_CUSTOM_HEADERS")
}

fn collect_header_literal_credentials(
    value: &serde_json::Value,
    output: &mut Vec<String>,
) -> Result<(), ()> {
    match value {
        serde_json::Value::String(value) => {
            push_header_value_credentials(value, output);
            Ok(())
        }
        serde_json::Value::Object(values) => values
            .values()
            .try_for_each(|value| collect_header_literal_credentials(value, output)),
        serde_json::Value::Array(values) => values
            .iter()
            .try_for_each(|value| collect_header_literal_credentials(value, output)),
        _ => Err(()),
    }
}

fn collect_custom_header_string_credentials(value: &str, output: &mut Vec<String>) {
    push_nonempty_credential(value, output);
    for line in value.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some((_, header_value)) = line.split_once(':') {
            push_header_value_credentials(header_value, output);
        }
    }
}

fn collect_provider_credentials(
    value: &serde_json::Value,
    output: &mut Vec<String>,
) -> Result<(), ()> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if is_header_container_key(key) {
                    collect_header_literal_credentials(value, output)?;
                } else if is_custom_header_string_key(key) {
                    match value {
                        serde_json::Value::String(value) => {
                            collect_custom_header_string_credentials(value, output);
                        }
                        _ => collect_header_literal_credentials(value, output)?,
                    }
                } else if ProviderService::is_sensitive_config_key(key) {
                    if let Some(value) = value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
                        output.push(value.to_string());
                    }
                } else {
                    collect_provider_credentials(value, output)?;
                }
            }
            Ok(())
        }
        serde_json::Value::Array(values) => values
            .iter()
            .try_for_each(|value| collect_provider_credentials(value, output)),
        _ => Ok(()),
    }
}

fn provider_public_summary(provider: &Provider) -> Result<ProviderPublicSummary, String> {
    let mut credentials = Vec::new();
    collect_provider_credentials(&provider.settings_config, &mut credentials)
        .map_err(|_| PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string())?;
    if let Some(meta) = &provider.meta {
        if let Some(script) = &meta.usage_script {
            for credential in [
                script.api_key.as_deref(),
                script.access_token.as_deref(),
                script.access_key_id.as_deref(),
                script.secret_access_key.as_deref(),
            ]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            {
                credentials.push(credential.to_string());
            }
        }
        if let Some(overrides) = &meta.local_proxy_request_overrides {
            credentials.extend(
                overrides
                    .headers
                    .values()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
            );
        }
        if let Ok(meta) = serde_json::to_value(meta) {
            collect_provider_credentials(&meta, &mut credentials)
                .map_err(|_| PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string())?;
        }
    }
    if let Some(config) = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
    {
        let config = config
            .parse::<toml::Value>()
            .map_err(|_| PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string())?;
        let config = serde_json::to_value(config)
            .map_err(|_| PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string())?;
        collect_provider_credentials(&config, &mut credentials)
            .map_err(|_| PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string())?;
    }
    if [provider.id.as_str(), provider.name.as_str()]
        .into_iter()
        .any(|public| {
            let public = public.trim();
            !public.is_empty()
                && credentials
                    .iter()
                    .any(|credential| public.contains(credential))
        })
    {
        return Err(PROVIDER_PUBLIC_SUMMARY_UNAVAILABLE.to_string());
    }
    Ok(ProviderPublicSummary {
        id: provider.id.clone(),
        name: provider.name.clone(),
    })
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderQuickSetupRequest {
    name: String,
    base_url: String,
    api_key: String,
    model_id: String,
    /// Codex 原生能力意图（生图扩展 / WebSocket），仅 codex 目标生效。
    #[serde(default)]
    codex_features: Option<CodexProviderFeatureIntent>,
}

impl ProviderQuickSetupRequest {
    pub(crate) fn into_provider(
        self,
        app_type: &AppType,
    ) -> Result<Provider, ProviderQuickSetupCommandError> {
        let codex_features = self.codex_features.unwrap_or_default();
        let name = self.name.trim().to_string();
        let base_url = self.base_url.trim().to_string();
        let api_key = self.api_key.trim().to_string();
        let model_id = self.model_id.trim().to_string();
        if name.is_empty() || api_key.is_empty() || model_id.is_empty() {
            return Err(ProviderQuickSetupCommandError::new(
                QuickSetupApplyFailureCode::ApplyFailedRolledBack,
            ));
        }
        let reserved_id = match app_type {
            AppType::Claude => "fyagent-v2-quick-setup-claude",
            AppType::Codex => "fyagent-v2-quick-setup-codex",
            AppType::GrokBuild => "fyagent-v2-quick-setup-grokbuild",
            _ => "",
        };
        if name.contains(&api_key) || model_id.contains(&api_key) || reserved_id.contains(&api_key)
        {
            return Err(ProviderQuickSetupCommandError::new(
                QuickSetupApplyFailureCode::ApplyFailedRolledBack,
            ));
        }
        let (id, settings_config) = match app_type {
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
            AppType::Codex => {
                let quote = |value: &str| {
                    serde_json::to_string(value).expect("serializing a Rust string cannot fail")
                };
                // 开启内置生图扩展后，请求走本地 `x-openai-actor-authorization`
                // header，不再依赖 OpenAI 官方登录，故 requires_openai_auth=false。
                // 新版 Codex 在 requires_openai_auth=false 时不会读取 auth.json
                // 的 OPENAI_API_KEY，必须把同一把 key 同步到 provider 表的
                // experimental_bearer_token。关闭生图时仍只写 auth.json。
                let image_extension = codex_features.image_extension.unwrap_or(false);
                let websockets = codex_features.websockets.unwrap_or(false);
                let requires_openai_auth = !image_extension;
                let mut config = format!(
                    "model_provider = \"custom\"\nmodel = {}\ndisable_response_storage = true\n\n[model_providers.custom]\nname = {}\nbase_url = {}\nwire_api = \"responses\"\nrequires_openai_auth = {}",
                    quote(&model_id),
                    quote(&name),
                    quote(&base_url),
                    requires_openai_auth,
                );
                if image_extension {
                    let bearer_token = toml_edit::Value::from(api_key.as_str()).to_string();
                    config.push_str(&format!(
                        "\nhttp_headers = {{ \"{}\" = \"{}\" }}\nexperimental_bearer_token = {bearer_token}",
                        crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER,
                        crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE,
                    ));
                }
                if websockets {
                    config.push_str("\nsupports_websockets = true");
                }
                (
                    "fyagent-v2-quick-setup-codex",
                    serde_json::json!({
                        "auth": { "OPENAI_API_KEY": api_key },
                        "config": config,
                    }),
                )
            }
            AppType::GrokBuild => {
                let model_value = toml_edit::Value::from(model_id.as_str()).to_string();
                let name_value = toml_edit::Value::from(name.as_str()).to_string();
                let endpoint_value = toml_edit::Value::from(base_url.as_str()).to_string();
                let api_key_value = toml_edit::Value::from(api_key.as_str()).to_string();
                (
                    "fyagent-v2-quick-setup-grokbuild",
                    serde_json::json!({
                        "config": format!(
                            "[models]\ndefault = {model_value}\n\n[model.{model_value}]\nmodel = {model_value}\nbase_url = {endpoint_value}\nname = {name_value}\napi_key = {api_key_value}\napi_backend = \"{}\"\ncontext_window = {}\n",
                            crate::grok_config::DEFAULT_API_BACKEND,
                            crate::grok_config::DEFAULT_CONTEXT_WINDOW,
                        )
                    }),
                )
            }
            _ => unreachable!("quick setup app allowlist was checked before derivation"),
        };
        let mut provider = Provider::with_id(id.to_string(), name, settings_config, None);
        provider.category = Some("custom".to_string());
        provider.notes = Some("Created by FyAgent V2 quick setup".to_string());
        // 显式生图选择视为已完成迁移，避免 prepare_codex_provider_features_for_save
        // 的默认迁移覆盖用户的一键配置选择。
        if codex_features.image_extension.is_some() {
            provider
                .meta
                .get_or_insert_with(ProviderMeta::default)
                .image_extension_configured = Some(true);
        }
        Ok(provider)
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuickSetupCommandError {
    code: QuickSetupApplyFailureCode,
}

impl ProviderQuickSetupCommandError {
    fn new(code: QuickSetupApplyFailureCode) -> Self {
        Self { code }
    }
}

// 常量定义
const TEMPLATE_TYPE_GITHUB_COPILOT: &str = "github_copilot";
const TEMPLATE_TYPE_TOKEN_PLAN: &str = "token_plan";
const TEMPLATE_TYPE_BALANCE: &str = "balance";
const TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION: &str = "official_subscription";
const COPILOT_UNIT_PREMIUM: &str = "requests";

fn provider_mutation_warning_codes(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
) -> Vec<String> {
    if !matches!(app_type, AppType::Codex) {
        return Vec::new();
    }
    let takeover_enabled =
        futures::executor::block_on(state.db.get_proxy_config_for_app(AppType::Codex.as_str()))
            .map(|config| config.enabled)
            .unwrap_or(false)
            || state
                .proxy_service
                .detect_takeover_in_live_config_for_app(&AppType::Codex);
    crate::codex_config::codex_provider_save_warning_codes(provider, takeover_enabled)
}

/// 获取所有供应商
#[tauri::command]
pub fn get_providers(
    state: State<'_, AppState>,
    app: String,
) -> Result<IndexMap<String, Provider>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::list(state.inner(), app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_provider(state: State<'_, AppState>, app: String) -> Result<String, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::current(state.inner(), app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_summary(
    app_handle: tauri::AppHandle,
    app: String,
) -> Result<ProviderPublicSummaryResult, String> {
    let app_type = parse_provider_draft_app(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or_else(|| "Provider public summary is unavailable".to_string())?;
        let _guard =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app(app_type.as_str()));
        let all = state
            .db
            .get_all_providers(app_type.as_str())
            .map_err(|_| "Provider public summary is unavailable".to_string())?;
        let mut providers = IndexMap::new();
        for (key, provider) in all {
            let summary = provider_public_summary(&provider)?;
            if key != summary.id {
                return Err("Provider public summary is unavailable".to_string());
            }
            providers.insert(key, summary);
        }
        let write_targets = ProviderService::quick_setup_write_targets(&app_type)
            .map_err(|_| "Provider public summary is unavailable".to_string())?;
        let current_id = ProviderService::current(state.inner(), app_type)
            .map_err(|_| "Provider public summary is unavailable".to_string())?;
        if !current_id.is_empty() && !providers.contains_key(&current_id) {
            return Err("Provider public summary is unavailable".to_string());
        }
        Ok(ProviderPublicSummaryResult {
            providers,
            current_id,
            write_targets,
        })
    })
    .await
    .map_err(|_| "Provider public summary is unavailable".to_string())?
}

#[tauri::command]
pub fn add_provider(
    state: State<'_, AppState>,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] addToLive: Option<bool>,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::add(state.inner(), app_type, provider, addToLive.unwrap_or(true))
        .map_err(|e| e.to_string())
}

/// Compatible mutation envelope for clients that coordinate a Codex Desktop
/// restart. The original `add_provider` command intentionally keeps its bool
/// return type for existing renderer versions.
#[tauri::command]
pub fn add_provider_with_result(
    state: State<'_, AppState>,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] addToLive: Option<bool>,
) -> Result<ProviderMutationResult<bool>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let provider_id = provider.id.clone();
    let warning_fallback = provider.clone();
    let warning_app_type = app_type.clone();
    let mutation_app_type = app_type.clone();
    let mut result = ProviderService::with_live_config_result(app_type, || {
        ProviderService::add(
            state.inner(),
            mutation_app_type,
            provider,
            addToLive.unwrap_or(true),
        )
    })
    .map_err(|e| e.to_string())?;
    let saved_provider = state
        .db
        .get_provider_by_id(&provider_id, warning_app_type.as_str())
        .ok()
        .flatten()
        .unwrap_or(warning_fallback);
    result.warning_codes =
        provider_mutation_warning_codes(state.inner(), &warning_app_type, &saved_provider);
    Ok(result)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindXaiManagedRequest {
    pub app: String,
    pub account_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindXaiManagedResult {
    pub provider_id: String,
    pub provider_name: String,
    pub app: String,
    pub already_bound: bool,
    pub activated: bool,
}

fn parse_xai_bind_app(app: &str) -> Result<AppType, String> {
    match app.trim() {
        "claude" => Ok(AppType::Claude),
        "claude-desktop" => Ok(AppType::ClaudeDesktop),
        "codex" => Ok(AppType::Codex),
        _ => Err("SuperGrok bind supports only claude, claude-desktop, or codex".to_string()),
    }
}

fn xai_managed_provider_id(app_type: &AppType) -> &'static str {
    match app_type {
        AppType::Claude => "fyagent-v2-xai-oauth-claude",
        AppType::ClaudeDesktop => "fyagent-v2-xai-oauth-claude-desktop",
        AppType::Codex => "fyagent-v2-xai-oauth-codex",
        _ => "fyagent-v2-xai-oauth",
    }
}

fn build_xai_managed_provider(app_type: &AppType, account_id: &str) -> Provider {
    let name = match app_type {
        AppType::Codex => "xAI (Grok) OAuth".to_string(),
        _ => "xAI (Grok)".to_string(),
    };
    let settings_config = match app_type {
        AppType::Codex => serde_json::json!({
            "auth": {},
            "config": "model_provider = \"xai\"\nmodel = \"grok-4.5\"\n\n[model_providers.xai]\nname = \"xAI (Grok) OAuth\"\nbase_url = \"https://api.x.ai/v1\"\nwire_api = \"responses\"\n"
        }),
        _ => serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.x.ai/v1",
                "ANTHROPIC_MODEL": "grok-4.5",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "grok-4.5",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "grok-4.5",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "grok-4.5"
            }
        }),
    };
    let mut provider = Provider::with_id(
        xai_managed_provider_id(app_type).to_string(),
        name,
        settings_config,
        Some("https://x.ai/grok".to_string()),
    );
    provider.category = Some("third_party".to_string());
    provider.icon = Some("xai".to_string());
    let mut meta = crate::provider::ProviderMeta {
        provider_type: Some("xai_oauth".to_string()),
        auth_binding: Some(crate::provider::AuthBinding {
            source: crate::provider::AuthBindingSource::ManagedAccount,
            auth_provider: Some("xai_oauth".to_string()),
            account_id: Some(account_id.to_string()),
        }),
        ..Default::default()
    };
    if *app_type == AppType::ClaudeDesktop {
        if let Some(routes) = suggested_claude_desktop_routes(&provider) {
            meta.claude_desktop_mode = Some(crate::provider::ClaudeDesktopMode::Proxy);
            meta.claude_desktop_model_routes = routes;
        }
    }
    provider.meta = Some(meta);
    provider
}

/// Bind a logged-in SuperGrok account to Claude Code, Claude Desktop, or Codex.
/// Tokens stay in `xai_oauth_auth.json`. Codex is stored only; activation uses
/// the existing Change Plan switch.
#[tauri::command(rename_all = "camelCase")]
pub async fn bind_xai_managed_provider(
    request: BindXaiManagedRequest,
    app_handle: tauri::AppHandle,
    xai_state: State<'_, XaiOAuthState>,
) -> Result<BindXaiManagedResult, String> {
    let app_type = parse_xai_bind_app(&request.app)?;
    let manager = xai_state.0.read().await;
    let account_id = match request
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => id.to_string(),
        None => manager
            .default_account_id()
            .await
            .ok_or_else(|| "No usable xAI account available".to_string())?,
    };
    if !crate::proxy::providers::xai_oauth_auth::XaiOAuthManager::stored_account_is_usable(
        &account_id,
    ) {
        return Err("No usable xAI account available".to_string());
    }
    drop(manager);

    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or_else(|| "Provider state is unavailable".to_string())?;
        let provider = build_xai_managed_provider(&app_type, &account_id);
        let provider_id = provider.id.clone();
        let provider_name = provider.name.clone();
        let already_bound = state
            .db
            .get_provider_by_id(&provider_id, app_type.as_str())
            .ok()
            .flatten()
            .is_some();
        let activate = !matches!(app_type, AppType::Codex);
        if already_bound {
            state
                .db
                .save_provider(app_type.as_str(), &provider)
                .map_err(|error| error.to_string())?;
            if activate {
                ProviderService::switch(state.inner(), app_type.clone(), &provider_id)
                    .map_err(|error| error.to_string())?;
            }
        } else if activate {
            ProviderService::add(state.inner(), app_type.clone(), provider, true)
                .map_err(|error| error.to_string())?;
        } else {
            ProviderService::add_draft(state.inner(), app_type.clone(), provider)
                .map_err(|error| error.to_string())?;
        }

        Ok(BindXaiManagedResult {
            provider_id,
            provider_name,
            app: app_type.as_str().to_string(),
            already_bound,
            activated: activate,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

fn parse_provider_draft_app(app: &str) -> Result<AppType, String> {
    let app_type = AppType::from_str(app).map_err(|e| e.to_string())?;
    if !matches!(
        app_type,
        AppType::Claude | AppType::Codex | AppType::GrokBuild
    ) {
        return Err("Provider quick setup supports only claude, codex, or grokbuild".to_string());
    }
    Ok(app_type)
}

/// Atomically store and activate a bounded Claude/Codex quick-setup Provider.
#[tauri::command]
pub async fn apply_provider_quick_setup_with_result(
    app_handle: tauri::AppHandle,
    app: String,
    request: ProviderQuickSetupRequest,
) -> Result<ProviderMutationResult<SwitchResult>, ProviderQuickSetupCommandError> {
    let app_type = parse_provider_draft_app(&app).map_err(|_| {
        ProviderQuickSetupCommandError::new(QuickSetupApplyFailureCode::ApplyFailedRolledBack)
    })?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle.try_state::<AppState>().ok_or_else(|| {
            ProviderQuickSetupCommandError::new(QuickSetupApplyFailureCode::ApplyFailedRolledBack)
        })?;
        let provider = request.into_provider(&app_type)?;
        ProviderService::apply_quick_setup(state.inner(), app_type, provider).map_err(|error| {
            log::error!("Provider quick setup failed: {error}");
            ProviderQuickSetupCommandError::new(error.code)
        })
    })
    .await
    .map_err(|error| {
        log::error!("Provider quick setup worker failed: {error}");
        ProviderQuickSetupCommandError::new(QuickSetupApplyFailureCode::RollbackPartialStateUnknown)
    })?
}

#[tauri::command]
pub fn update_provider(
    state: State<'_, AppState>,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] originalId: Option<String>,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::update(state.inner(), app_type, originalId.as_deref(), provider)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_provider_with_result(
    state: State<'_, AppState>,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] originalId: Option<String>,
) -> Result<ProviderMutationResult<bool>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let provider_id = provider.id.clone();
    let warning_fallback = provider.clone();
    let warning_app_type = app_type.clone();
    let mutation_app_type = app_type.clone();
    let mut result = ProviderService::with_live_config_result(app_type, || {
        ProviderService::update(
            state.inner(),
            mutation_app_type,
            originalId.as_deref(),
            provider,
        )
    })
    .map_err(|e| e.to_string())?;
    let saved_provider = state
        .db
        .get_provider_by_id(&provider_id, warning_app_type.as_str())
        .ok()
        .flatten()
        .unwrap_or(warning_fallback);
    result.warning_codes =
        provider_mutation_warning_codes(state.inner(), &warning_app_type, &saved_provider);
    Ok(result)
}

#[tauri::command]
pub fn delete_provider(
    state: State<'_, AppState>,
    app: String,
    id: String,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::delete(state.inner(), app_type, &id)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_provider_with_result(
    state: State<'_, AppState>,
    app: String,
    id: String,
) -> Result<ProviderMutationResult<bool>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let mutation_app_type = app_type.clone();
    ProviderService::with_live_config_result(app_type, || {
        ProviderService::delete(state.inner(), mutation_app_type, &id).map(|_| true)
    })
    .map_err(|e| e.to_string())
}

/// Analyze a form-only Codex provider draft. This command never writes the
/// database or `~/.codex/config.toml`.
#[tauri::command]
pub fn analyze_codex_provider_features(
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] isNew: Option<bool>,
) -> Result<crate::codex_config::CodexProviderFeatureState, String> {
    require_codex_feature_app(&app)?;
    Ok(crate::codex_config::analyze_codex_provider_features(
        &provider,
        isNew.unwrap_or(false),
    ))
}

/// Apply a non-destructive feature patch to a form-only Codex TOML draft.
/// The caller must include the returned `tomlText` in a normal provider save;
/// no user file is written by this command.
#[tauri::command]
pub fn patch_codex_provider_features(
    app: String,
    provider: Provider,
    intent: crate::codex_config::CodexProviderFeatureIntent,
    #[allow(non_snake_case)] isNew: Option<bool>,
) -> Result<crate::codex_config::CodexProviderFeaturePatchResult, String> {
    require_codex_feature_app(&app)?;
    crate::codex_config::patch_codex_provider_features(&provider, &intent, isNew.unwrap_or(false))
        .map_err(|error| error.to_string())
}

fn require_codex_feature_app(app: &str) -> Result<(), String> {
    let app_type = AppType::from_str(app).map_err(|error| error.to_string())?;
    if matches!(app_type, AppType::Codex) {
        Ok(())
    } else {
        Err("Codex 原生能力仅适用于 Codex 应用".to_owned())
    }
}

#[tauri::command]
pub fn remove_provider_from_live_config(
    state: tauri::State<'_, AppState>,
    app: String,
    id: String,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::remove_from_live_config(state.inner(), app_type, &id)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

fn switch_provider_internal(
    state: &AppState,
    app_type: AppType,
    id: &str,
) -> Result<SwitchResult, AppError> {
    ProviderService::switch(state, app_type, id)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn switch_provider_test_hook(
    state: &AppState,
    app_type: AppType,
    id: &str,
) -> Result<SwitchResult, AppError> {
    switch_provider_internal(state, app_type, id)
}

#[tauri::command]
pub async fn switch_provider(
    app_handle: tauri::AppHandle,
    app: String,
    id: String,
) -> Result<SwitchResult, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or_else(|| "应用状态不可用".to_string())?;
        switch_provider_internal(state.inner(), app_type, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("供应商切换任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn switch_provider_with_result(
    app_handle: tauri::AppHandle,
    app: String,
    id: String,
) -> Result<ProviderMutationResult<SwitchResult>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle
            .try_state::<AppState>()
            .ok_or_else(|| "应用状态不可用".to_string())?;
        let mutation_app_type = app_type.clone();
        ProviderService::with_live_config_result(app_type, || {
            switch_provider_internal(state.inner(), mutation_app_type, &id)
        })
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|e| format!("供应商切换任务执行失败: {e}"))?
}

fn import_default_config_internal(state: &AppState, app_type: AppType) -> Result<bool, AppError> {
    // Keep the eligibility checks, live read, Provider/current writes, and
    // command-only post-processing in one per-app critical section. Calling
    // the public service wrapper below would acquire this same lock again.
    let _guard = ProviderService::lock_provider_mutation(state, &app_type);
    if matches!(app_type, AppType::GrokBuild) {
        // 官方登录态（live 语法合法且无自定义模型表）+ 用户手动导入：
        // 导入的正确结果是让 Grok Official 成为当前供应商，而非报错。
        // 只挂在命令层 = 只有手动动作可达；启动自动导入走 service 层、
        // 官方态照旧报错静默跳过，删掉的官方条目不会被重启复活
        //（全项目惯例：启动自动导入只产出 default，从不产出官方条目）。
        if let Ok(settings) = crate::grok_config::read_grok_live_settings() {
            let config = settings
                .get("config")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if crate::grok_config::is_official_live_config(config) {
                state.db.ensure_official_seed_by_id(
                    crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID,
                    AppType::GrokBuild,
                )?;
                state.db.set_current_provider(
                    app_type.as_str(),
                    crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID,
                )?;
                crate::settings::set_current_provider(
                    &app_type,
                    Some(crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID),
                )?;
                return Ok(true);
            }
        }

        // Safety net: 与 claude-desktop 导入同语义 —— 用户主动点导入是"重新
        // 整理该表"的隐式信号，把官方入口补回来。覆盖导入必然失败的场景
        //（live 文件缺失 / TOML 语法错误 / 残缺的自定义配置），避免
        // "报错 + 空列表"死胡同。失败只 warn，不影响导入主流程。
        if let Err(e) = state.db.ensure_official_seed_by_id(
            crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID,
            AppType::GrokBuild,
        ) {
            log::warn!("Failed to ensure grokbuild-official seed during import: {e}");
        }
    }

    let imported = ProviderService::import_default_config_with_lock_held(state, app_type.clone())?;

    if imported {
        // Extract common config snippet (mirrors old startup logic in lib.rs)
        if state
            .db
            .should_auto_extract_config_snippet(app_type.as_str())?
        {
            match ProviderService::extract_common_config_snippet(state, app_type.clone()) {
                Ok(snippet) if !snippet.is_empty() && snippet != "{}" => {
                    let _ = state
                        .db
                        .set_config_snippet(app_type.as_str(), Some(snippet));
                    let _ = state
                        .db
                        .set_config_snippet_cleared(app_type.as_str(), false);
                }
                _ => {}
            }
        }

        ProviderService::migrate_legacy_common_config_usage_if_needed(state, app_type.clone())?;
    }

    Ok(imported)
}

#[cfg_attr(not(feature = "test-hooks"), doc(hidden))]
pub fn import_default_config_test_hook(
    state: &AppState,
    app_type: AppType,
) -> Result<bool, AppError> {
    import_default_config_internal(state, app_type)
}

#[tauri::command]
pub fn import_default_config(state: State<'_, AppState>, app: String) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    import_default_config_internal(&state, app_type).map_err(Into::into)
}

#[tauri::command]
pub fn import_default_config_with_result(
    state: State<'_, AppState>,
    app: String,
) -> Result<ProviderMutationResult<bool>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    let mutation_app_type = app_type.clone();
    ProviderService::with_live_config_result(app_type, || {
        import_default_config_internal(state.inner(), mutation_app_type)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_claude_desktop_status(
    state: State<'_, AppState>,
) -> Result<crate::claude_desktop_config::ClaudeDesktopStatus, String> {
    let proxy_running = state.proxy_service.is_running().await;
    crate::claude_desktop_config::get_status(state.db.as_ref(), proxy_running)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_claude_desktop_default_routes(
) -> Vec<crate::claude_desktop_config::ClaudeDesktopDefaultRoute> {
    crate::claude_desktop_config::default_proxy_routes()
}

#[tauri::command]
pub fn import_claude_desktop_providers_from_claude(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let claude_providers = state
        .db
        .get_all_providers(AppType::Claude.as_str())
        .map_err(|e| e.to_string())?;
    let existing_ids = state
        .db
        .get_provider_ids(AppType::ClaudeDesktop.as_str())
        .map_err(|e| e.to_string())?;

    let mut imported = 0usize;
    for provider in claude_providers.values() {
        if existing_ids.contains(&provider.id) {
            continue;
        }

        let mut desktop_provider = provider.clone();
        desktop_provider.in_failover_queue = false;
        let meta = desktop_provider.meta.get_or_insert_with(Default::default);

        if crate::claude_desktop_config::is_compatible_direct_provider(provider)
            && claude_provider_models_are_claude_safe(provider)
        {
            meta.claude_desktop_mode = Some(ClaudeDesktopMode::Direct);
        } else if let Some(routes) = suggested_claude_desktop_routes(provider) {
            meta.claude_desktop_mode = Some(ClaudeDesktopMode::Proxy);
            meta.claude_desktop_model_routes = routes;
        } else {
            continue;
        }

        state
            .db
            .save_provider(AppType::ClaudeDesktop.as_str(), &desktop_provider)
            .map_err(|e| e.to_string())?;
        imported += 1;
    }

    // Safety net: 用户可能手动删除过 claude-desktop-official seed。
    // 用户主动点 import 是"重新整理 ClaudeDesktop 表"的隐式信号，把官方入口补回来。
    // 失败只 warn，不影响 imported 主流程；imported 计数语义保持纯净。
    if let Err(e) = state.db.ensure_official_seed_by_id(
        crate::database::CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
        AppType::ClaudeDesktop,
    ) {
        log::warn!("Failed to ensure claude-desktop-official seed during import: {e}");
    }

    Ok(imported)
}

#[tauri::command]
pub fn ensure_claude_desktop_official_provider(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .db
        .ensure_official_seed_by_id(
            crate::database::CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
            AppType::ClaudeDesktop,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_codex_official_provider(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .db
        .ensure_official_seed_by_id(crate::database::CODEX_OFFICIAL_PROVIDER_ID, AppType::Codex)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_grokbuild_official_provider(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .db
        .ensure_official_seed_by_id(
            crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID,
            AppType::GrokBuild,
        )
        .map_err(|e| e.to_string())
}

fn claude_provider_models_are_claude_safe(provider: &Provider) -> bool {
    let Some(env) = provider
        .settings_config
        .get("env")
        .and_then(|value| value.as_object())
    else {
        return true;
    };

    [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
    ]
    .into_iter()
    .filter_map(|key| env.get(key).and_then(|value| value.as_str()))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .all(crate::claude_desktop_config::is_claude_safe_model_id)
}

pub(crate) fn suggested_claude_desktop_routes(
    provider: &Provider,
) -> Option<std::collections::HashMap<String, crate::provider::ClaudeDesktopModelRoute>> {
    let env = provider
        .settings_config
        .get("env")
        .and_then(|value| value.as_object())?;
    let mut routes = std::collections::HashMap::new();
    let supports_1m_default = !matches!(
        provider
            .meta
            .as_ref()
            .and_then(|meta| meta.provider_type.as_deref()),
        Some("github_copilot") | Some("codex_oauth") | Some("xai_oauth")
    );

    fn add_route(
        routes: &mut std::collections::HashMap<String, crate::provider::ClaudeDesktopModelRoute>,
        env: &serde_json::Map<String, serde_json::Value>,
        route_key: &str,
        env_key: &str,
        supports_1m_default: bool,
    ) {
        let Some(raw_model) = env
            .get(env_key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return;
        };

        // Claude 端 env 值可能带 [1M] 后缀；Claude Desktop schema 不接受后缀，
        // 改用 supports1m 字段表达 1M 能力。在 import 边界做单向翻译。
        let marker = crate::claude_desktop_config::ONE_M_CONTEXT_MARKER.as_bytes();
        let raw_bytes = raw_model.as_bytes();
        let has_1m_marker = raw_bytes.len() >= marker.len()
            && raw_bytes[raw_bytes.len() - marker.len()..].eq_ignore_ascii_case(marker);
        let stripped_model: &str = if has_1m_marker {
            raw_model[..raw_model.len() - marker.len()].trim_end()
        } else {
            raw_model
        };
        if stripped_model.is_empty() {
            return;
        }
        let effective_supports_1m = supports_1m_default || has_1m_marker;
        let explicit_label_override = env
            .get(&format!("{env_key}_NAME"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let label_override = explicit_label_override.clone().or_else(|| {
            (!crate::claude_desktop_config::is_claude_safe_model_id(stripped_model))
                .then(|| stripped_model.to_string())
        });

        // 何时覆盖既有 label_override：原本为空 / 这次来的是 explicit _NAME /
        // 既有值只是 stripped_model 派生的占位（被 explicit 或更具体的值挤掉）。
        let should_overwrite = |existing: Option<&str>| {
            existing.is_none()
                || explicit_label_override.is_some()
                || existing == Some(stripped_model)
        };

        let merge_into = |existing: &mut crate::provider::ClaudeDesktopModelRoute| {
            let merged = existing.supports_1m.unwrap_or(false) || effective_supports_1m;
            existing.supports_1m = Some(merged);
            if should_overwrite(existing.label_override.as_deref()) {
                existing.label_override = label_override.clone();
            }
        };

        if let Some(existing) = routes
            .values_mut()
            .find(|existing| existing.model == stripped_model)
        {
            merge_into(existing);
            return;
        }

        routes
            .entry(route_key.to_string())
            .and_modify(merge_into)
            .or_insert_with(|| crate::provider::ClaudeDesktopModelRoute {
                model: stripped_model.to_string(),
                label_override,
                supports_1m: Some(effective_supports_1m),
            });
    }

    for spec in crate::claude_desktop_config::DEFAULT_PROXY_ROUTES {
        add_route(
            &mut routes,
            env,
            spec.route_id,
            spec.env_key,
            supports_1m_default,
        );
    }

    // 三个 default env_key 全空时用 ANTHROPIC_MODEL 派生兜底路由。
    if routes.is_empty() {
        let primary_route = crate::claude_desktop_config::DEFAULT_PROXY_ROUTES[0].route_id;
        add_route(
            &mut routes,
            env,
            primary_route,
            "ANTHROPIC_MODEL",
            supports_1m_default,
        );
    }

    (!routes.is_empty()).then_some(routes)
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn queryProviderUsage(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    copilot_state: State<'_, CopilotAuthState>,
    xai_state: State<'_, XaiOAuthState>,
    #[allow(non_snake_case)] providerId: String, // 使用 camelCase 匹配前端
    app: String,
) -> Result<crate::provider::UsageResult, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    // inner 可能以两种形式失败：
    //   1) 返回 Ok(UsageResult { success: false, .. }) —— 确定性失败（401、脚本
    //      报错、未知供应商等）。写进 UsageCache 并刷新托盘，让
    //      format_script_summary 的 success 守卫生效、suffix 自然消失。
    //   2) 返回 Err(String) —— 瞬时传输失败（网络/超时）及 DB/Copilot fetch 等。
    //      不写失败快照、不 emit：保留上一份托盘快照，与前端 react-query reject
    //      保留上次 data 的语义一致；否则失败快照会经 useUsageCacheBridge 盲写
    //      回 query 缓存，抹掉 reject 本该保留的旧值。
    let inner = query_provider_usage_inner(
        &state,
        &copilot_state,
        &xai_state,
        app_type.clone(),
        &providerId,
    )
    .await;
    if let Ok(snapshot) = &inner {
        let payload = serde_json::json!({
            "kind": "script",
            "appType": app_type.as_str(),
            "providerId": &providerId,
            "data": snapshot,
        });
        if let Err(e) = app_handle.emit("usage-cache-updated", payload) {
            log::error!("emit usage-cache-updated (script) 失败: {e}");
        }
        state
            .usage_cache
            .put_script(app_type, providerId, snapshot.clone());
        crate::tray::schedule_tray_refresh(&app_handle);
    }
    inner
}

/// Resolve `(base_url, api_key)` for native usage queries, delegating to the
/// per-app resolver on `Provider`. Missing provider → empty credentials.
fn resolve_native_credentials(app_type: &AppType, provider: Option<&Provider>) -> (String, String) {
    provider
        .map(|p| p.resolve_usage_credentials(app_type))
        .unwrap_or_default()
}

fn resolve_coding_plan_credentials(
    app_type: &AppType,
    provider: Option<&Provider>,
    usage_script: Option<&crate::provider::UsageScript>,
) -> (String, String) {
    let is_zenmux = usage_script
        .and_then(|s| s.coding_plan_provider.as_deref())
        .map(|provider| provider.eq_ignore_ascii_case("zenmux"))
        .unwrap_or(false);

    if !is_zenmux {
        return resolve_native_credentials(app_type, provider);
    }

    let script_base_url = usage_script
        .and_then(|s| s.base_url.as_deref())
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    let script_api_key = usage_script
        .and_then(|s| s.api_key.as_deref())
        .unwrap_or("")
        .to_string();

    if !script_base_url.is_empty() && !script_api_key.is_empty() {
        return (script_base_url, script_api_key);
    }

    let native = resolve_native_credentials(app_type, provider);
    if !native.0.is_empty() && !native.1.is_empty() {
        native
    } else {
        (script_base_url, script_api_key)
    }
}

async fn query_provider_usage_inner(
    state: &AppState,
    copilot_state: &CopilotAuthState,
    xai_state: &XaiOAuthState,
    app_type: AppType,
    provider_id: &str,
) -> Result<crate::provider::UsageResult, String> {
    // 从数据库读取供应商信息，检查特殊模板类型
    let providers = state
        .db
        .get_all_providers(app_type.as_str())
        .map_err(|e| format!("Failed to get providers: {e}"))?;
    let provider = providers.get(provider_id);
    let usage_script = provider
        .and_then(|p| p.meta.as_ref())
        .and_then(|m| m.usage_script.as_ref());
    let template_type = usage_script
        .and_then(|s| s.template_type.as_deref())
        .unwrap_or("");

    // ── GitHub Copilot 专用路径 ──
    if template_type == TEMPLATE_TYPE_GITHUB_COPILOT {
        let copilot_account_id = provider
            .and_then(|p| p.meta.as_ref())
            .and_then(|m| m.managed_account_id_for(TEMPLATE_TYPE_GITHUB_COPILOT));

        let auth_manager = copilot_state.0.read().await;
        let usage = match copilot_account_id.as_deref() {
            Some(account_id) => auth_manager
                .fetch_usage_for_account(account_id)
                .await
                .map_err(|e| format!("Failed to fetch Copilot usage: {e}"))?,
            None => auth_manager
                .fetch_usage()
                .await
                .map_err(|e| format!("Failed to fetch Copilot usage: {e}"))?,
        };
        let premium = &usage.quota_snapshots.premium_interactions;
        let used = premium.entitlement - premium.remaining;

        return Ok(crate::provider::UsageResult {
            success: true,
            data: Some(vec![crate::provider::UsageData {
                plan_name: Some(usage.copilot_plan),
                remaining: Some(premium.remaining as f64),
                total: Some(premium.entitlement as f64),
                used: Some(used as f64),
                unit: Some(COPILOT_UNIT_PREMIUM.to_string()),
                is_valid: Some(true),
                invalid_message: None,
                extra: Some(format!("Reset: {}", usage.quota_reset_date)),
            }]),
            error: None,
        });
    }

    // ── Coding Plan 专用路径 ──
    if template_type == TEMPLATE_TYPE_TOKEN_PLAN {
        let (base_url, api_key) =
            resolve_coding_plan_credentials(&app_type, provider, usage_script);

        // 火山方舟用账号 AK/SK 签名查询用量（存于 usage_script，与推理 api_key 分离）；
        // 其他供应商为 None，service 层沿用 api_key。
        let access_key_id = usage_script.and_then(|s| s.access_key_id.clone());
        let secret_access_key = usage_script.and_then(|s| s.secret_access_key.clone());
        // 智谱团队版：显式 provider 标识 + 组织/项目 ID（与个人版智谱 base_url 相同，
        // 靠 coding_plan_provider == "zhipu_team" 在 service 层路由）。
        let coding_plan_provider = usage_script.and_then(|s| s.coding_plan_provider.clone());
        let team_organization_id = usage_script.and_then(|s| s.team_organization_id.clone());
        let team_project_id = usage_script.and_then(|s| s.team_project_id.clone());

        let quota = crate::services::coding_plan::get_coding_plan_quota(
            &base_url,
            &api_key,
            access_key_id.as_deref(),
            secret_access_key.as_deref(),
            coding_plan_provider.as_deref(),
            team_organization_id.as_deref(),
            team_project_id.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to query coding plan: {e}"))?;

        // 将 SubscriptionQuota 转换为 UsageResult
        if !quota.success {
            return Ok(crate::provider::UsageResult {
                success: false,
                data: None,
                error: quota.error,
            });
        }

        // ZenMux 的 tier 携带 USD 额度信息，需要编码为 JSON extra
        let has_usd = quota
            .tiers
            .first()
            .map(|t| t.used_value_usd.is_some())
            .unwrap_or(false);
        let plan_label = quota
            .credential_message
            .as_deref()
            .and_then(|msg| msg.split(' ').next())
            .map(|tier| format!("ZenMux·{}", tier.to_uppercase()));
        let mut first_tier = true;

        let data: Vec<crate::provider::UsageData> = quota
            .tiers
            .iter()
            .map(|tier| {
                let total = 100.0;
                let used = tier.utilization;
                let remaining = total - used;
                let extra = if has_usd {
                    let mut extra_json = serde_json::json!({
                        "resetsAt": tier.resets_at,
                    });
                    if let Some(v) = tier.used_value_usd {
                        extra_json["usedValueUsd"] = serde_json::json!(v);
                    }
                    if let Some(v) = tier.max_value_usd {
                        extra_json["maxValueUsd"] = serde_json::json!(v);
                    }
                    if first_tier {
                        if let Some(ref label) = plan_label {
                            extra_json["planLabel"] = serde_json::json!(label);
                        }
                        first_tier = false;
                    }
                    Some(extra_json.to_string())
                } else {
                    tier.resets_at.clone()
                };
                crate::provider::UsageData {
                    plan_name: Some(tier.name.clone()),
                    remaining: Some(remaining),
                    total: Some(total),
                    used: Some(used),
                    unit: Some("%".to_string()),
                    is_valid: Some(true),
                    invalid_message: None,
                    extra,
                }
            })
            .collect();

        return Ok(crate::provider::UsageResult {
            success: true,
            data: if data.is_empty() { None } else { Some(data) },
            error: None,
        });
    }

    // ── 官方余额查询路径 ──
    if template_type == TEMPLATE_TYPE_BALANCE {
        // 按 app 区分的凭据存储格式提取 Base URL 与 API Key
        let (base_url, api_key) = resolve_native_credentials(&app_type, provider);

        return crate::services::balance::get_balance(&base_url, &api_key)
            .await
            .map_err(|e| format!("Failed to query balance: {e}"));
    }

    // ── 官方订阅额度查询路径 ──
    if template_type == TEMPLATE_TYPE_OFFICIAL_SUBSCRIPTION {
        if !usage_script.map(|s| s.enabled).unwrap_or(false) {
            return Ok(crate::provider::UsageResult {
                success: false,
                data: None,
                error: Some("Usage query is disabled".to_string()),
            });
        }

        // xAI OAuth 托管供应商的额度属绑定的 SuperGrok 账号，而非所在 app 的
        // CLI 凭据（对 codex/claude 而言 CLI 凭据是 ChatGPT/Claude 订阅，跨了
        // 订阅体系，查出来的数字张冠李戴）。
        let quota = if provider.map(Provider::is_xai_oauth).unwrap_or(false) {
            let account_id = provider
                .and_then(|p| p.meta.as_ref())
                .and_then(|m| m.managed_account_id_for("xai_oauth"));
            crate::commands::xai_oauth::query_xai_oauth_quota_for(xai_state, account_id).await?
        } else {
            crate::services::subscription::get_subscription_quota(app_type.as_str())
                .await
                .map_err(|e| format!("Failed to query subscription quota: {e}"))?
        };

        if !quota.success {
            return Ok(crate::provider::UsageResult {
                success: false,
                data: None,
                error: quota.error.or(quota.credential_message),
            });
        }

        let data: Vec<crate::provider::UsageData> = quota
            .tiers
            .iter()
            .map(|tier| crate::provider::UsageData {
                plan_name: Some(tier.name.clone()),
                remaining: Some(100.0 - tier.utilization),
                total: Some(100.0),
                used: Some(tier.utilization),
                unit: Some("%".to_string()),
                is_valid: Some(true),
                invalid_message: None,
                extra: tier.resets_at.clone(),
            })
            .collect();

        return Ok(crate::provider::UsageResult {
            success: true,
            data: if data.is_empty() { None } else { Some(data) },
            error: None,
        });
    }

    // ── 通用 JS 脚本路径 ──
    ProviderService::query_usage(state, app_type, provider_id)
        .await
        .map_err(|e| e.to_string())
}

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn testUsageScript(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] providerId: String,
    app: String,
    #[allow(non_snake_case)] scriptCode: String,
    timeout: Option<u64>,
    #[allow(non_snake_case)] apiKey: Option<String>,
    #[allow(non_snake_case)] baseUrl: Option<String>,
    #[allow(non_snake_case)] accessToken: Option<String>,
    #[allow(non_snake_case)] userId: Option<String>,
    #[allow(non_snake_case)] templateType: Option<String>,
) -> Result<crate::provider::UsageResult, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::test_usage_script(
        state.inner(),
        app_type,
        &providerId,
        &scriptCode,
        timeout.unwrap_or(10),
        apiKey.as_deref(),
        baseUrl.as_deref(),
        accessToken.as_deref(),
        userId.as_deref(),
        templateType.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_live_provider_settings(app: String) -> Result<serde_json::Value, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::read_live_settings(app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_api_endpoints(
    urls: Vec<String>,
    #[allow(non_snake_case)] timeoutSecs: Option<u64>,
) -> Result<Vec<EndpointLatency>, String> {
    SpeedtestService::test_endpoints(urls, timeoutSecs)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_custom_endpoints(
    state: State<'_, AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
) -> Result<Vec<crate::settings::CustomEndpoint>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::get_custom_endpoints(state.inner(), app_type, &providerId)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_custom_endpoint(
    state: State<'_, AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    url: String,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::add_custom_endpoint(state.inner(), app_type, &providerId, url)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_custom_endpoint(
    state: State<'_, AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    url: String,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::remove_custom_endpoint(state.inner(), app_type, &providerId, url)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_endpoint_last_used(
    state: State<'_, AppState>,
    app: String,
    #[allow(non_snake_case)] providerId: String,
    url: String,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::update_endpoint_last_used(state.inner(), app_type, &providerId, url)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_providers_sort_order(
    state: State<'_, AppState>,
    app: String,
    updates: Vec<ProviderSortUpdate>,
) -> Result<bool, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    ProviderService::update_sort_order(state.inner(), app_type, updates).map_err(|e| e.to_string())
}

use crate::provider::UniversalProvider;
use std::collections::HashMap;
use tauri::AppHandle;

#[derive(Clone, serde::Serialize)]
pub struct UniversalProviderSyncedEvent {
    pub action: String,
    pub id: String,
}

fn emit_universal_provider_synced(app: &AppHandle, action: &str, id: &str) {
    let _ = app.emit(
        "universal-provider-synced",
        UniversalProviderSyncedEvent {
            action: action.to_string(),
            id: id.to_string(),
        },
    );
}

#[tauri::command]
pub fn get_universal_providers(
    state: State<'_, AppState>,
) -> Result<HashMap<String, UniversalProvider>, String> {
    ProviderService::list_universal(state.inner()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_universal_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<UniversalProvider>, String> {
    ProviderService::get_universal(state.inner(), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_universal_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: UniversalProvider,
) -> Result<bool, String> {
    let id = provider.id.clone();
    let result =
        ProviderService::upsert_universal(state.inner(), provider).map_err(|e| e.to_string())?;

    emit_universal_provider_synced(&app, "upsert", &id);

    Ok(result)
}

#[tauri::command]
pub fn delete_universal_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let result =
        ProviderService::delete_universal(state.inner(), &id).map_err(|e| e.to_string())?;

    emit_universal_provider_synced(&app, "delete", &id);

    Ok(result)
}

#[tauri::command]
pub fn sync_universal_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let result =
        ProviderService::sync_universal_to_apps(state.inner(), &id).map_err(|e| e.to_string())?;

    emit_universal_provider_synced(&app, "sync", &id);

    Ok(result)
}

#[tauri::command]
pub fn import_opencode_providers_from_live(state: State<'_, AppState>) -> Result<usize, String> {
    crate::services::provider::import_opencode_providers_from_live(state.inner())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_opencode_live_provider_ids() -> Result<Vec<String>, String> {
    crate::opencode_config::get_providers()
        .map(|providers| providers.keys().cloned().collect())
        .map_err(|e| e.to_string())
}

// ============================================================================
// OpenClaw 专属命令 → 已迁移至 commands/openclaw.rs
// ============================================================================

#[cfg(test)]
mod provider_draft_command_tests {
    use super::{parse_provider_draft_app, provider_public_summary, ProviderQuickSetupRequest};
    use crate::app_config::AppType;
    use crate::provider::Provider;

    #[test]
    fn quick_setup_drafts_allow_claude_codex_and_grokbuild() {
        assert!(matches!(
            parse_provider_draft_app("claude"),
            Ok(AppType::Claude)
        ));
        assert!(matches!(
            parse_provider_draft_app("codex"),
            Ok(AppType::Codex)
        ));
        assert!(matches!(
            parse_provider_draft_app("grokbuild"),
            Ok(AppType::GrokBuild)
        ));
        for unsupported in ["gemini", "opencode", "hermes"] {
            assert!(
                parse_provider_draft_app(unsupported).is_err(),
                "{unsupported} must not enter the bounded quick-setup command"
            );
        }
    }

    #[test]
    fn atomic_quick_setup_command_is_registered_once() {
        let library_source = include_str!("../lib.rs");
        assert_eq!(
            library_source
                .matches("commands::apply_provider_quick_setup_with_result")
                .count(),
            1
        );
        assert_eq!(
            library_source
                .matches("commands::bind_xai_managed_provider")
                .count(),
            1
        );
    }

    #[test]
    fn xai_managed_bind_allows_only_claude_desktop_and_codex() {
        use super::{build_xai_managed_provider, parse_xai_bind_app};

        assert!(matches!(parse_xai_bind_app("claude"), Ok(AppType::Claude)));
        assert!(matches!(
            parse_xai_bind_app("claude-desktop"),
            Ok(AppType::ClaudeDesktop)
        ));
        assert!(matches!(parse_xai_bind_app("codex"), Ok(AppType::Codex)));
        for unsupported in ["grokbuild", "gemini", "workbuddy", "qoder"] {
            assert!(
                parse_xai_bind_app(unsupported).is_err(),
                "{unsupported} must not enter SuperGrok bind"
            );
        }

        let claude = build_xai_managed_provider(&AppType::Claude, "acct-xai");
        let desktop = build_xai_managed_provider(&AppType::ClaudeDesktop, "acct-xai");
        let codex = build_xai_managed_provider(&AppType::Codex, "acct-xai");
        assert_eq!(claude.id, "fyagent-v2-xai-oauth-claude");
        assert_eq!(desktop.id, "fyagent-v2-xai-oauth-claude-desktop");
        assert_eq!(codex.id, "fyagent-v2-xai-oauth-codex");
        assert_eq!(claude.name, "xAI (Grok)");
        assert_eq!(codex.name, "xAI (Grok) OAuth");
        let payload = serde_json::to_string(&claude).unwrap()
            + &serde_json::to_string(&desktop).unwrap()
            + &serde_json::to_string(&codex).unwrap();
        assert!(payload.contains("xai_oauth"));
        assert!(payload.contains("acct-xai"));
        assert!(!payload.contains("refresh"));
        assert!(!payload.contains("ANTHROPIC_AUTH_TOKEN"));
        assert!(!payload.contains("OPENAI_API_KEY"));
        let desktop_meta = desktop.meta.as_ref().expect("desktop meta");
        assert_eq!(
            desktop_meta.claude_desktop_mode,
            Some(crate::provider::ClaudeDesktopMode::Proxy)
        );
        assert!(!desktop_meta.claude_desktop_model_routes.is_empty());
    }

    #[test]
    fn public_summary_rejects_id_or_name_containing_any_credential_source() {
        let credential = "TEST-SECRET-PUBLIC-SUMMARY";
        for source in ["settings", "meta", "toml"] {
            let mut provider = Provider::with_id(
                "safe-id".to_string(),
                format!("prefix-{credential}-suffix"),
                if source == "settings" {
                    serde_json::json!({ "env": { "ANTHROPIC_AUTH_TOKEN": credential } })
                } else if source == "toml" {
                    serde_json::json!({ "config": format!("bearer_token = {credential:?}") })
                } else {
                    serde_json::json!({})
                },
                None,
            );
            if source == "meta" {
                provider.meta = Some(
                    serde_json::from_value(serde_json::json!({
                        "usage_script": {
                            "enabled": true, "language": "javascript", "code": "",
                            "apiKey": credential
                        }
                    }))
                    .unwrap(),
                );
            }
            let error = provider_public_summary(&provider).unwrap_err();
            assert!(!error.contains(credential));
        }
    }

    #[test]
    fn public_summary_serializes_only_id_and_name() {
        let mut provider = Provider::with_id(
            "safe-id".to_string(),
            "Safe name".to_string(),
            serde_json::json!({}),
            Some("https://example.test/?token=secret".to_string()),
        );
        provider.category = Some("custom".to_string());
        let serialized = serde_json::to_value(provider_public_summary(&provider).unwrap()).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!({ "id": "safe-id", "name": "Safe name" })
        );
    }

    #[test]
    fn public_summary_fails_closed_for_meta_credentials_headers_and_invalid_toml() {
        let credential = "TEST-SECRET-META-SUMMARY";
        for meta in [
            serde_json::json!({
                "usage_script": {
                    "enabled": true, "language": "javascript", "code": "",
                    "accessToken": credential, "accessKeyId": "another-secret",
                    "secretAccessKey": "third-secret"
                }
            }),
            serde_json::json!({
                "localProxyRequestOverrides": {
                    "headers": { "Cookie": credential, "Authorization": "other-secret" }
                }
            }),
        ] {
            let mut provider = Provider::with_id(
                "safe-id".to_string(),
                format!("prefix-{credential}-suffix"),
                serde_json::json!({}),
                None,
            );
            provider.meta = Some(serde_json::from_value(meta).unwrap());
            let error = provider_public_summary(&provider).unwrap_err();
            assert!(!error.contains(credential));
        }

        let provider = Provider::with_id(
            "safe-id".to_string(),
            "Safe name".to_string(),
            serde_json::json!({ "config": "invalid = [ toml" }),
            None,
        );
        assert_eq!(
            provider_public_summary(&provider).unwrap_err(),
            "Provider public summary is unavailable"
        );
    }

    #[test]
    fn public_summary_rejects_codex_and_claude_custom_header_credentials() {
        let credential = "TEST-HEADER-CREDENTIAL";
        for settings_config in [
            serde_json::json!({
                "config": format!(
                    "[model_providers.custom]\nhttp_headers = {{ Authorization = {:?}, Cookie = {:?} }}\nenv_http_headers = {{ \"X-API-Key\" = {:?} }}",
                    format!("Bearer {credential}"),
                    format!("session={credential}"),
                    credential,
                )
            }),
            serde_json::json!({
                "env": {
                    "ANTHROPIC_CUSTOM_HEADERS": format!(
                        "Cookie: session={credential}\nX-API-Key: {credential}"
                    )
                }
            }),
        ] {
            let provider = Provider::with_id(
                "safe-id".to_string(),
                format!("prefix-{credential}-suffix"),
                settings_config,
                None,
            );
            let error = provider_public_summary(&provider).unwrap_err();
            assert_eq!(error, "Provider public summary is unavailable");
            assert!(!error.contains(credential));
        }
    }

    #[test]
    fn quick_setup_request_rejects_unknown_fields_and_empty_required_values() {
        assert!(
            serde_json::from_value::<ProviderQuickSetupRequest>(serde_json::json!({
                "name": "Gateway", "baseUrl": "https://example.test/v1",
                "apiKey": "key", "modelId": "model", "category": "official"
            }))
            .is_err()
        );
        let request: ProviderQuickSetupRequest = serde_json::from_value(serde_json::json!({
            "name": " ", "baseUrl": "https://example.test/v1",
            "apiKey": "key", "modelId": "model"
        }))
        .unwrap();
        assert!(request.into_provider(&AppType::Codex).is_err());

        for (name, model_id) in [
            ("prefix-secret-key-suffix", "model-a"),
            ("Gateway", "prefix-secret-key-suffix"),
        ] {
            let request: ProviderQuickSetupRequest = serde_json::from_value(serde_json::json!({
                "name": name,
                "baseUrl": "https://example.test/v1",
                "apiKey": "secret-key",
                "modelId": model_id
            }))
            .unwrap();
            let error = request.into_provider(&AppType::Codex).unwrap_err();
            assert!(!format!("{error:?}").contains("secret-key"));
        }
    }

    #[test]
    fn quick_setup_request_derives_the_fixed_provider_shape() {
        let request: ProviderQuickSetupRequest = serde_json::from_value(serde_json::json!({
            "name": " Gateway ", "baseUrl": " https://example.test/v1 ",
            "apiKey": " secret-key ", "modelId": " model-a "
        }))
        .unwrap();
        let provider = request.into_provider(&AppType::Codex).unwrap();
        assert_eq!(provider.id, "fyagent-v2-quick-setup-codex");
        assert_eq!(provider.name, "Gateway");
        assert_eq!(provider.category.as_deref(), Some("custom"));
        assert!(provider.meta.is_none());
        assert!(!provider.in_failover_queue);
        assert_eq!(
            provider.settings_config["auth"]["OPENAI_API_KEY"],
            "secret-key"
        );
        let config = provider.settings_config["config"].as_str().unwrap();
        assert!(
            config.contains("requires_openai_auth = true"),
            "未开启生图时 requires_openai_auth 应为 true，实际 config:\n{config}"
        );
        assert!(
            !config.contains("experimental_bearer_token"),
            "未开启生图时不应写入 experimental_bearer_token，实际 config:\n{config}"
        );
    }

    #[test]
    fn quick_setup_request_writes_image_extension_and_websocket_features() {
        let request: ProviderQuickSetupRequest = serde_json::from_value(serde_json::json!({
            "name": "Gateway", "baseUrl": "https://example.test/v1",
            "apiKey": "secret-key", "modelId": "model-a",
            "codexFeatures": { "imageExtension": true, "websockets": true }
        }))
        .unwrap();
        let provider = request.into_provider(&AppType::Codex).unwrap();
        let config = provider.settings_config["config"].as_str().unwrap();
        assert!(
            config.contains("requires_openai_auth = false"),
            "开启生图后 requires_openai_auth 应为 false，实际 config:\n{config}"
        );
        assert!(
            config.contains(
                "http_headers = { \"x-openai-actor-authorization\" = \"local-image-extension\" }"
            ),
            "开启生图后应写入生图 header，实际 config:\n{config}"
        );
        assert!(
            config.contains("supports_websockets = true"),
            "开启 WebSocket 后应写入 supports_websockets，实际 config:\n{config}"
        );
        assert!(
            config.contains("experimental_bearer_token = \"secret-key\""),
            "开启生图后应把 API Key 同步到 experimental_bearer_token，实际 config:\n{config}"
        );
        assert_eq!(
            provider.settings_config["auth"]["OPENAI_API_KEY"], "secret-key",
            "开启生图时 auth.json 形状仍应保留 OPENAI_API_KEY"
        );
        assert_eq!(
            provider
                .meta
                .as_ref()
                .and_then(|meta| meta.image_extension_configured),
            Some(true)
        );
    }

    #[test]
    fn quick_setup_request_disabling_image_keeps_requires_openai_auth_true() {
        let request: ProviderQuickSetupRequest = serde_json::from_value(serde_json::json!({
            "name": "Gateway", "baseUrl": "https://example.test/v1",
            "apiKey": "secret-key", "modelId": "model-a",
            "codexFeatures": { "imageExtension": false }
        }))
        .unwrap();
        let provider = request.into_provider(&AppType::Codex).unwrap();
        let config = provider.settings_config["config"].as_str().unwrap();
        assert!(
            config.contains("requires_openai_auth = true"),
            "关闭生图后 requires_openai_auth 应保持 true，实际 config:\n{config}"
        );
        assert!(
            !config.contains("http_headers"),
            "关闭生图后不应写入生图 header，实际 config:\n{config}"
        );
        assert!(
            !config.contains("experimental_bearer_token"),
            "关闭生图后不应写入 experimental_bearer_token，实际 config:\n{config}"
        );
        assert_eq!(
            provider.settings_config["auth"]["OPENAI_API_KEY"], "secret-key",
            "关闭生图时应把 API Key 写到 auth.json"
        );
        // 显式关闭也视为已完成迁移，阻止默认迁移重新开启生图
        assert_eq!(
            provider
                .meta
                .as_ref()
                .and_then(|meta| meta.image_extension_configured),
            Some(true)
        );
    }
}

#[cfg(test)]
mod import_claude_desktop_tests {
    use super::suggested_claude_desktop_routes;
    use crate::provider::{Provider, ProviderMeta};
    use serde_json::json;

    fn make_provider(env: serde_json::Value, provider_type: Option<&str>) -> Provider {
        let mut p = Provider::with_id(
            "test-claude".to_string(),
            "Test".to_string(),
            json!({ "env": env }),
            None,
        );
        if let Some(pt) = provider_type {
            p.meta = Some(ProviderMeta {
                provider_type: Some(pt.to_string()),
                ..ProviderMeta::default()
            });
        }
        p
    }

    #[test]
    fn route_strips_1m_suffix_and_sets_supports_1m() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-5-20250929[1M]",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        let r = routes.get("claude-sonnet-5").expect("sonnet route present");
        assert_eq!(r.model, "claude-sonnet-4-5-20250929");
        assert!(
            !r.model.to_ascii_lowercase().contains("[1m]"),
            "model must not contain [1m] suffix"
        );
        assert_eq!(r.label_override, None);
        assert_eq!(r.supports_1m, Some(true));
    }

    #[test]
    fn route_preserves_model_without_suffix() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "kimi-k2",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        let r = routes.get("claude-sonnet-5").expect("sonnet route present");
        assert_eq!(r.model, "kimi-k2");
        assert_eq!(r.label_override.as_deref(), Some("kimi-k2"));
        // 默认 provider_type 缺省 → supports_1m_default = true
        assert_eq!(r.supports_1m, Some(true));
    }

    #[test]
    fn route_uses_claude_code_model_name_as_label_override() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "kimi-k2",
                "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME": "Kimi K2",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        let r = routes.get("claude-sonnet-5").expect("sonnet route present");
        assert_eq!(r.model, "kimi-k2");
        assert_eq!(r.label_override.as_deref(), Some("Kimi K2"));
    }

    #[test]
    fn route_1m_suffix_overrides_provider_type_default() {
        // github_copilot 默认 supports_1m_default = false，但 [1M] 后缀应强制 true
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "gpt-5-codex[1M]",
            }),
            Some("github_copilot"),
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        let r = routes.get("claude-sonnet-5").expect("sonnet route present");
        assert_eq!(r.model, "gpt-5-codex");
        assert_eq!(r.label_override.as_deref(), Some("gpt-5-codex"));
        assert_eq!(r.supports_1m, Some(true));
    }

    #[test]
    fn route_github_copilot_without_suffix_keeps_false() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "gpt-5-codex",
            }),
            Some("github_copilot"),
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        let r = routes.get("claude-sonnet-5").expect("sonnet route present");
        assert_eq!(r.model, "gpt-5-codex");
        assert_eq!(r.label_override.as_deref(), Some("gpt-5-codex"));
        assert_eq!(r.supports_1m, Some(false));
    }

    #[test]
    fn same_upstream_across_three_aliases_merges_to_one_route() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "MiniMax-M2",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "MiniMax-M2",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "MiniMax-M2",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        assert_eq!(routes.len(), 1, "three aliases → one merged route");
        let r = routes.get("claude-sonnet-5").expect("merged route present");
        assert_eq!(r.model, "MiniMax-M2");
        assert_eq!(r.label_override.as_deref(), Some("MiniMax-M2"));
    }

    #[test]
    fn same_upstream_with_partial_1m_marker_takes_or_aggregation() {
        // sonnet 带 [1M]，opus/haiku 不带 → 合并后 supports_1m == Some(true)
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "MiniMax-M2[1M]",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "MiniMax-M2",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "MiniMax-M2",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        assert_eq!(routes.len(), 1);
        let r = routes.get("claude-sonnet-5").expect("merged route present");
        assert_eq!(r.supports_1m, Some(true));
    }

    #[test]
    fn different_upstream_models_produce_separate_routes() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "GLM-4.6",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "GLM-4-Air",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "GLM-4-Flash",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        assert_eq!(routes.len(), 3);
        assert_eq!(routes.get("claude-sonnet-5").unwrap().model, "GLM-4.6");
        assert_eq!(routes.get("claude-opus-5").unwrap().model, "GLM-4-Air");
        assert_eq!(routes.get("claude-haiku-4-5").unwrap().model, "GLM-4-Flash");
        assert_eq!(
            routes
                .get("claude-sonnet-5")
                .unwrap()
                .label_override
                .as_deref(),
            Some("GLM-4.6")
        );
    }

    #[test]
    fn anthropic_model_fallback_only_triggers_when_empty() {
        // 三个 default env_key 都不填，仅 ANTHROPIC_MODEL
        let p = make_provider(
            json!({
                "ANTHROPIC_MODEL": "kimi-k2",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        assert_eq!(routes.len(), 1);
        let r = routes
            .get("claude-sonnet-5")
            .expect("fallback route present");
        assert_eq!(r.model, "kimi-k2");
        assert_eq!(r.label_override.as_deref(), Some("kimi-k2"));
    }

    #[test]
    fn existing_claude_prefix_not_duplicated() {
        let p = make_provider(
            json!({
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-5-20250929",
            }),
            None,
        );
        let routes = suggested_claude_desktop_routes(&p).expect("routes built");
        assert!(routes.contains_key("claude-sonnet-5"));
        assert!(!routes.contains_key("claude-claude-sonnet-4-5-20250929"));
        assert_eq!(
            routes.get("claude-sonnet-5").expect("route").label_override,
            None
        );
    }
}

#[cfg(test)]
mod native_query_credentials_tests {
    use super::{resolve_coding_plan_credentials, resolve_native_credentials};
    use crate::app_config::AppType;
    use crate::provider::{Provider, UsageScript};
    use serde_json::json;

    fn usage_script(
        coding_plan_provider: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
    ) -> UsageScript {
        UsageScript {
            enabled: true,
            language: "javascript".to_string(),
            code: String::new(),
            timeout: Some(10),
            api_key: api_key.map(str::to_string),
            base_url: base_url.map(str::to_string),
            access_token: None,
            user_id: None,
            template_type: Some("token_plan".to_string()),
            auto_query_interval: None,
            coding_plan_provider: coding_plan_provider.map(str::to_string),
            access_key_id: None,
            secret_access_key: None,
            team_organization_id: None,
            team_project_id: None,
        }
    }

    #[test]
    fn delegates_to_provider_for_codex() {
        let provider = Provider::with_id(
            "test".to_string(),
            "Test".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-codex" },
                "config": "model_provider = \"deepseek\"\n\
                           [model_providers.deepseek]\n\
                           base_url = \"https://api.deepseek.com\"\n",
            }),
            None,
        );
        let (base_url, api_key) = resolve_native_credentials(&AppType::Codex, Some(&provider));
        assert_eq!(base_url, "https://api.deepseek.com");
        assert_eq!(api_key, "sk-codex");
    }

    #[test]
    fn missing_provider_yields_empty() {
        let (base_url, api_key) = resolve_native_credentials(&AppType::Codex, None);
        assert!(base_url.is_empty());
        assert!(api_key.is_empty());
    }

    #[test]
    fn zenmux_coding_plan_uses_script_credentials_first() {
        let provider = Provider::with_id(
            "test".to_string(),
            "Test".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://provider.zenmux.example/v1",
                    "ANTHROPIC_AUTH_TOKEN": "sk-provider"
                }
            }),
            None,
        );
        let script = usage_script(
            Some("zenmux"),
            Some("https://script.zenmux.example/api/usage/"),
            Some("sk-script"),
        );

        let (base_url, api_key) =
            resolve_coding_plan_credentials(&AppType::Claude, Some(&provider), Some(&script));

        assert_eq!(base_url, "https://script.zenmux.example/api/usage");
        assert_eq!(api_key, "sk-script");
    }

    #[test]
    fn zenmux_coding_plan_falls_back_to_provider_credentials() {
        let provider = Provider::with_id(
            "test".to_string(),
            "Test".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://provider.zenmux.example/v1",
                    "ANTHROPIC_AUTH_TOKEN": "sk-provider"
                }
            }),
            None,
        );
        let script = usage_script(Some("zenmux"), Some("https://script.zenmux.example"), None);

        let (base_url, api_key) =
            resolve_coding_plan_credentials(&AppType::Claude, Some(&provider), Some(&script));

        assert_eq!(base_url, "https://provider.zenmux.example/v1");
        assert_eq!(api_key, "sk-provider");
    }
}
