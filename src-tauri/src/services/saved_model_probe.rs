//! Lightweight saved-model connectivity & latency probe (`#426`).
//!
//! Resolves credentials from saved local configuration on the native side so
//! secrets never cross IPC, enforces existing URL and coding-plan credential
//! guards, sends a minimal 1-token / lightweight request, classifies common
//! failure modes into actionable user-facing categories, and never deletes or
//! mutates saved models (even on HTTP 404).

use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    app_config::AppType,
    services::{
        opencode_models::resolve_saved_opencode_probe_connection,
        provider::{batch_setup::BatchSetupTargetId, ProviderCredentials},
        provider_api::{ensure_generic_api_allowed, validate_credential_scope, ApiProtocol},
        stream_check::StreamCheckService,
        workbuddy::{current_paths, load_workbuddy_files, url::normalize_workbuddy_base_url},
    },
    store::AppState,
};

const QUICK_PROBE_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedModelProbeCategory {
    Ok,
    InvalidKey,
    QuotaExhausted,
    EndpointOrModelMismatch,
    NetworkUnreachable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedModelProbeRequest {
    pub target: BatchSetupTargetId,
    #[serde(default)]
    pub provider_id: Option<String>,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedModelProbeResult {
    pub category: SavedModelProbeCategory,
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub http_status: Option<u16>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnthropicAuthMode {
    BearerToken,
    ApiKeyHeader,
}

struct ResolvedProbeSpec {
    url: String,
    api_key: Option<String>,
    protocol: ApiProtocol,
    anthropic_auth: AnthropicAuthMode,
    model_id: String,
}

pub fn classify_http_probe_status(status: u16) -> (SavedModelProbeCategory, &'static str) {
    match status {
        200..=299 => (SavedModelProbeCategory::Ok, "连接可用"),
        401 | 403 => (
            SavedModelProbeCategory::InvalidKey,
            "鉴权失败：API Key 无效、已过期或无该模型权限",
        ),
        402 | 429 => (
            SavedModelProbeCategory::QuotaExhausted,
            "额度不足或触发并发限流，请检查账户余额",
        ),
        400 | 404 | 405 | 422 => (
            SavedModelProbeCategory::EndpointOrModelMismatch,
            "模型 ID 不匹配或接口路径不兼容（未改动已保存配置）",
        ),
        _ => (
            SavedModelProbeCategory::NetworkUnreachable,
            "上游服务异常，请稍后重试",
        ),
    }
}

pub async fn probe_saved_model(
    state: &AppState,
    request: SavedModelProbeRequest,
) -> Result<SavedModelProbeResult, String> {
    let model_id = request.model_id.trim().to_string();
    if model_id.is_empty() || model_id.len() > 256 {
        return Err("无效的模型 ID".to_string());
    }

    let spec = resolve_saved_probe_spec(
        state,
        &request.target,
        request.provider_id.as_deref(),
        &model_id,
    )?;
    execute_lightweight_probe(spec).await
}

fn build_validated_spec(
    target: BatchSetupTargetId,
    base_url: &str,
    api_key: Option<String>,
    protocol: ApiProtocol,
    anthropic_auth: AnthropicAuthMode,
    model_id: String,
) -> Result<ResolvedProbeSpec, String> {
    StreamCheckService::validate_probe_url(base_url).map_err(|e| e.to_string())?;
    let key_str = api_key.as_deref().unwrap_or("");
    ensure_generic_api_allowed(base_url, key_str).map_err(|e| e.to_string())?;
    validate_credential_scope(base_url, key_str, protocol).map_err(|e| e.to_string())?;

    let url = build_target_endpoint_url(target, base_url, protocol)?;
    Ok(ResolvedProbeSpec {
        url,
        api_key,
        protocol,
        anthropic_auth,
        model_id,
    })
}

fn resolve_saved_probe_spec(
    state: &AppState,
    target: &BatchSetupTargetId,
    provider_id: Option<&str>,
    requested_model_id: &str,
) -> Result<ResolvedProbeSpec, String> {
    match target {
        BatchSetupTargetId::Workbuddy => {
            let paths = current_paths();
            let (loaded, _) =
                load_workbuddy_files(&paths).map_err(|_| "无法读取 WorkBuddy 配置文件".to_string())?;
            let entry = loaded
                .document
                .models()
                .iter()
                .find(|item| {
                    item.get("id").and_then(Value::as_str).map(str::trim) == Some(requested_model_id)
                })
                .ok_or_else(|| format!("未在 WorkBuddy 配置中找到模型 '{requested_model_id}'"))?;
            let base_url = entry
                .get("url")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "该模型缺少接口地址".to_string())?;
            let api_key = entry
                .get("apiKey")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            build_validated_spec(
                BatchSetupTargetId::Workbuddy,
                base_url,
                api_key,
                ApiProtocol::Chat,
                AnthropicAuthMode::ApiKeyHeader,
                requested_model_id.to_string(),
            )
        }
        BatchSetupTargetId::Opencode => {
            let pid = provider_id
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "缺少 OpenCode 供应商标识".to_string())?;
            let (base_url, api_key) = resolve_saved_opencode_probe_connection(pid)?;
            build_validated_spec(
                BatchSetupTargetId::Opencode,
                &base_url,
                (!api_key.is_empty()).then_some(api_key),
                ApiProtocol::Chat,
                AnthropicAuthMode::ApiKeyHeader,
                requested_model_id.to_string(),
            )
        }
        BatchSetupTargetId::Omp => {
            let pid = provider_id
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "缺少 omp 供应商标识".to_string())?;
            let (base_url, api, api_key) =
                crate::omp_config::resolve_omp_provider_probe_target(pid)
                    .map_err(|e| e.to_string())?;
            let protocol = match api.as_str() {
                "openai-responses" => ApiProtocol::Responses,
                "anthropic-messages" => ApiProtocol::Anthropic,
                _ => ApiProtocol::Chat,
            };
            build_validated_spec(
                BatchSetupTargetId::Omp,
                &base_url,
                api_key,
                protocol,
                AnthropicAuthMode::ApiKeyHeader,
                requested_model_id.to_string(),
            )
        }
        BatchSetupTargetId::Claude | BatchSetupTargetId::Codex | BatchSetupTargetId::Grokbuild => {
            let app_type = match target {
                BatchSetupTargetId::Claude => AppType::Claude,
                BatchSetupTargetId::Codex => AppType::Codex,
                BatchSetupTargetId::Grokbuild => AppType::GrokBuild,
                _ => unreachable!(),
            };
            let target_pid = match provider_id.map(str::trim).filter(|s| !s.is_empty()) {
                Some(id) => id.to_string(),
                None => state
                    .db
                    .get_current_provider(app_type.as_str())
                    .map_err(|_| "无法读取当前选中的供应商".to_string())?
                    .ok_or_else(|| "当前尚未选择激活的供应商".to_string())?,
            };
            let stored = state
                .db
                .get_provider_by_id(&target_pid, app_type.as_str())
                .map_err(|_| "无法读取供应商记录".to_string())?
                .ok_or_else(|| "未找到对应的已保存供应商".to_string())?;
            let resolved = ProviderCredentials::resolve(&state.db, app_type.as_str(), &stored)
                .map_err(|_| "无法解析已保存供应商的密钥凭据".to_string())?;

            extract_provider_probe_spec(*target, &app_type, &resolved, requested_model_id)
        }
    }
}

fn extract_provider_probe_spec(
    target: BatchSetupTargetId,
    app_type: &AppType,
    provider: &crate::provider::Provider,
    requested_model_id: &str,
) -> Result<ResolvedProbeSpec, String> {
    let settings = &provider.settings_config;
    match app_type {
        AppType::Claude => {
            let env = settings
                .get("env")
                .and_then(Value::as_object)
                .ok_or_else(|| "Claude 供应商配置缺少 env 字段".to_string())?;
            let base_url = env
                .get("ANTHROPIC_BASE_URL")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "官方登录账号请直接在客户端内使用".to_string())?;
            let auth_token = env
                .get("ANTHROPIC_AUTH_TOKEN")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let api_key_env = env
                .get("ANTHROPIC_API_KEY")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let (key, anthropic_auth) = if let Some(tok) = auth_token {
                (Some(tok.to_string()), AnthropicAuthMode::BearerToken)
            } else if let Some(k) = api_key_env {
                (Some(k.to_string()), AnthropicAuthMode::ApiKeyHeader)
            } else {
                (None, AnthropicAuthMode::ApiKeyHeader)
            };
            let model_id = if !requested_model_id.trim().is_empty() {
                requested_model_id.trim().to_string()
            } else {
                env.get("ANTHROPIC_MODEL")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "未指定模型 ID".to_string())?
                    .to_string()
            };
            build_validated_spec(
                target,
                base_url,
                key,
                ApiProtocol::Anthropic,
                anthropic_auth,
                model_id,
            )
        }
        AppType::Codex => {
            let config_text = settings
                .get("config")
                .and_then(Value::as_str)
                .ok_or_else(|| "Codex 供应商缺少 config 字段".to_string())?;
            let parsed: toml::Value = config_text
                .parse()
                .map_err(|_| "Codex config.toml 语法无效".to_string())?;
            let selected = parsed
                .get("model_provider")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| "官方登录账号请直接在 Codex 内使用".to_string())?;
            let mp = parsed
                .get("model_providers")
                .and_then(|m| m.get(selected))
                .ok_or_else(|| "未找到对应的 model_providers 节点".to_string())?;
            let base_url = mp
                .get("base_url")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| "Codex 供应商缺少 base_url".to_string())?;
            let protocol = match mp.get("wire_api").and_then(toml::Value::as_str) {
                Some("chat") => ApiProtocol::Chat,
                _ => ApiProtocol::Responses,
            };
            let api_key = crate::codex_config::extract_codex_api_key(
                settings.get("auth"),
                Some(config_text),
            );
            let model_id = if !requested_model_id.trim().is_empty() {
                requested_model_id.trim().to_string()
            } else {
                parsed
                    .get("model")
                    .and_then(toml::Value::as_str)
                    .ok_or_else(|| "未指定模型 ID".to_string())?
                    .to_string()
            };
            build_validated_spec(
                target,
                base_url,
                api_key,
                protocol,
                AnthropicAuthMode::ApiKeyHeader,
                model_id,
            )
        }
        AppType::GrokBuild => {
            let config_text = settings
                .get("config")
                .and_then(Value::as_str)
                .ok_or_else(|| "Grok Build 供应商缺少 config 字段".to_string())?;
            let parsed: toml::Value = config_text
                .parse()
                .map_err(|_| "Grok Build config.toml 语法无效".to_string())?;
            let default_model = parsed
                .get("models")
                .and_then(|m| m.get("default"))
                .and_then(toml::Value::as_str);
            let effective_model = if !requested_model_id.trim().is_empty() {
                requested_model_id.trim()
            } else {
                default_model.ok_or_else(|| "未指定模型 ID".to_string())?
            };
            let entry = parsed
                .get("model")
                .and_then(|m| {
                    m.get(effective_model)
                        .or_else(|| default_model.and_then(|def| m.get(def)))
                })
                .ok_or_else(|| "未找到对应的 Grok Build 模型配置".to_string())?;
            let base_url = entry
                .get("base_url")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| "缺少 base_url".to_string())?;
            let api_key = entry
                .get("api_key")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            build_validated_spec(
                target,
                base_url,
                api_key,
                ApiProtocol::Responses,
                AnthropicAuthMode::ApiKeyHeader,
                effective_model.to_string(),
            )
        }
        _ => Err("不支持的目标类型".to_string()),
    }
}

pub(crate) fn build_target_endpoint_url(
    target: BatchSetupTargetId,
    base_url: &str,
    protocol: ApiProtocol,
) -> Result<String, String> {
    if target == BatchSetupTargetId::Workbuddy {
        let normalized =
            normalize_workbuddy_base_url(base_url).map_err(|_| "服务地址无效".to_string())?;
        let mut url = normalized.base_url;
        let path = match url.path().trim_end_matches('/') {
            "" | "/" => "/chat/completions".to_string(),
            p => format!("{p}/chat/completions"),
        };
        url.set_path(&path);
        return Ok(url.to_string());
    }
    Ok(build_endpoint_url(base_url, protocol))
}

pub(crate) fn build_endpoint_url(base_url: &str, protocol: ApiProtocol) -> String {
    match protocol {
        ApiProtocol::Anthropic => {
            format!("{}/v1/messages", base_url.trim().trim_end_matches('/'))
        }
        ApiProtocol::Responses => {
            format!("{}/responses", strip_known_endpoints(base_url))
        }
        ApiProtocol::Chat => {
            format!("{}/chat/completions", strip_known_endpoints(base_url))
        }
    }
}

fn strip_known_endpoints(base: &str) -> String {
    let mut base = base.trim().trim_end_matches('/').to_string();
    let lower = base.to_ascii_lowercase();
    for suffix in ["/chat/completions", "/messages", "/responses", "/models"] {
        if lower.ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            break;
        }
    }
    base
}

async fn execute_lightweight_probe(spec: ResolvedProbeSpec) -> Result<SavedModelProbeResult, String> {
    let client = Client::builder()
        .timeout(QUICK_PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法初始化网络探测客户端".to_string())?;

    let body = match spec.protocol {
        ApiProtocol::Anthropic => json!({
            "model": spec.model_id,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "ping" }]
        }),
        ApiProtocol::Responses => json!({
            "model": spec.model_id,
            "input": "ping",
            "max_output_tokens": 16
        }),
        ApiProtocol::Chat => json!({
            "model": spec.model_id,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "ping" }]
        }),
    };

    let mut req = client.post(&spec.url).header("Content-Type", "application/json");
    if spec.protocol == ApiProtocol::Anthropic {
        req = req.header("anthropic-version", "2023-06-01");
        if let Some(key) = &spec.api_key {
            req = match spec.anthropic_auth {
                AnthropicAuthMode::BearerToken => {
                    req.header("Authorization", format!("Bearer {key}"))
                }
                AnthropicAuthMode::ApiKeyHeader => req.header("x-api-key", key.as_str()),
            };
        }
    } else if let Some(key) = &spec.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let start = Instant::now();
    match req.json(&body).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            let (category, base_msg) = classify_http_probe_status(status);
            let ok = category == SavedModelProbeCategory::Ok;
            let message = if ok {
                format!("{base_msg} · {latency_ms}ms")
            } else {
                format!("{base_msg} (HTTP {status})")
            };
            Ok(SavedModelProbeResult {
                category,
                ok,
                latency_ms: Some(latency_ms),
                http_status: Some(status),
                message,
            })
        }
        Err(err) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let detail = if err.is_timeout() {
                "请求超时（超过 12 秒未响应）"
            } else if err.is_connect() {
                "无法建立网络连接，请检查接口地址或代理"
            } else {
                "网络请求失败，请检查地址与 TLS 配置"
            };
            Ok(SavedModelProbeResult {
                category: SavedModelProbeCategory::NetworkUnreachable,
                ok: false,
                latency_ms: Some(latency_ms),
                http_status: None,
                message: detail.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_http_statuses_into_five_actionable_categories() {
        assert_eq!(classify_http_probe_status(200).0, SavedModelProbeCategory::Ok);
        assert_eq!(
            classify_http_probe_status(401).0,
            SavedModelProbeCategory::InvalidKey
        );
        assert_eq!(
            classify_http_probe_status(403).0,
            SavedModelProbeCategory::InvalidKey
        );
        assert_eq!(
            classify_http_probe_status(402).0,
            SavedModelProbeCategory::QuotaExhausted
        );
        assert_eq!(
            classify_http_probe_status(429).0,
            SavedModelProbeCategory::QuotaExhausted
        );
        assert_eq!(
            classify_http_probe_status(404).0,
            SavedModelProbeCategory::EndpointOrModelMismatch
        );
        assert_eq!(
            classify_http_probe_status(502).0,
            SavedModelProbeCategory::NetworkUnreachable
        );
    }

    #[test]
    fn builds_exact_client_probe_urls_and_enforces_url_safety_guards() {
        assert_eq!(
            build_endpoint_url("https://open.bigmodel.cn/api/paas/v4/", ApiProtocol::Chat),
            "https://open.bigmodel.cn/api/paas/v4/chat/completions"
        );
        assert_eq!(
            build_endpoint_url("https://ark.cn-beijing.volces.com/api/v3", ApiProtocol::Responses),
            "https://ark.cn-beijing.volces.com/api/v3/responses"
        );
        // DeepSeek Codex/GrokBuild root responses URL is NOT forced to /v1
        assert_eq!(
            build_endpoint_url("https://api.deepseek.com", ApiProtocol::Responses),
            "https://api.deepseek.com/responses"
        );
        // Claude Code joins /v1/messages directly without collapsing /v1
        assert_eq!(
            build_endpoint_url("https://api.deepseek.com/anthropic", ApiProtocol::Anthropic),
            "https://api.deepseek.com/anthropic/v1/messages"
        );
        assert_eq!(
            build_endpoint_url("https://gateway.example/v1", ApiProtocol::Anthropic),
            "https://gateway.example/v1/v1/messages"
        );
        // Reject query/userinfo URLs and tool-only coding plan endpoints
        assert!(build_validated_spec(
            BatchSetupTargetId::Codex,
            "https://api.example.com/v1?leak=1",
            Some("sk-test".to_string()),
            ApiProtocol::Responses,
            AnthropicAuthMode::ApiKeyHeader,
            "gpt-5".to_string(),
        )
        .is_err());
        assert!(build_validated_spec(
            BatchSetupTargetId::Opencode,
            "https://coding.dashscope.aliyuncs.com/v1",
            Some("sk-sp-123456".to_string()),
            ApiProtocol::Chat,
            AnthropicAuthMode::ApiKeyHeader,
            "qwen3-coder-plus".to_string(),
        )
        .is_err());
    }
}
