//! Cross-app model health overview, one-click config cloning, and API key rotation.
//!
//! Reads active configurations across all six configurable AI clients on the
//! native side so raw credentials never cross IPC. Both clone and key-rotation
//! reuse `batch_setup`'s atomic backup, write, readback, and per-target
//! rollback pipeline.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

use super::batch_setup::{
    apply_batch_provider_setup, normalize_batch_api_key, preview_batch_provider_setup,
    BatchProviderApplyRequest, BatchProviderApplyResult, BatchProviderPreviewItem,
    BatchProviderPreviewRequest, BatchProviderPreviewResult, BatchSetupTargetId,
    BatchSetupTargetSpec,
};
use super::ProviderCredentials;
use crate::{
    app_config::AppType,
    omp_config::{self, OmpFileStatus},
    provider::Provider,
    services::{
        opencode_models::{get_opencode_model_snapshot, resolve_saved_opencode_probe_connection},
        provider_api::ApiProtocol,
        workbuddy::{current_paths, load_workbuddy_files},
    },
    store::AppState,
};

const ALL_CONFIGURABLE_TARGETS: [BatchSetupTargetId; 6] = [
    BatchSetupTargetId::Workbuddy,
    BatchSetupTargetId::Codex,
    BatchSetupTargetId::Claude,
    BatchSetupTargetId::Opencode,
    BatchSetupTargetId::Omp,
    BatchSetupTargetId::Grokbuild,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCredentialSource {
    ApiKey,
    Oauth,
    EnvOrCommand,
    None,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetModelSnapshot {
    pub target: BatchSetupTargetId,
    pub configured: bool,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub base_url: Option<String>,
    pub endpoint_host: Option<String>,
    pub model_id: Option<String>,
    pub protocol: Option<ApiProtocol>,
    pub model_count: usize,
    pub credential_source: ModelCredentialSource,
    pub can_probe: bool,
    pub can_clone_source: bool,
    pub can_rotate_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHealthOverview {
    pub targets: Vec<TargetModelSnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloneModelConfigRequest {
    pub source_target: BatchSetupTargetId,
    pub destination_targets: Vec<BatchSetupTargetId>,
    #[serde(default)]
    pub model_id_override: Option<String>,
    #[serde(default)]
    pub base_url_overrides: HashMap<BatchSetupTargetId, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApiKeyRotationPreviewRequest {
    pub targets: Vec<BatchSetupTargetId>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApiKeyRotationApplyRequest {
    pub api_key: String,
    pub targets: Vec<BatchSetupTargetId>,
}

impl fmt::Debug for ApiKeyRotationApplyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyRotationApplyRequest")
            .field("api_key", &"[REDACTED]")
            .field("target_count", &self.targets.len())
            .finish()
    }
}

struct NativeTargetDetails {
    snapshot: TargetModelSnapshot,
    api_key: Option<Zeroizing<String>>,
    extra_model_ids: Vec<String>,
}

fn extract_endpoint_host(base_url: Option<&str>) -> Option<String> {
    let raw = base_url?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = reqwest::Url::parse(raw).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    match parsed.port() {
        Some(port) => Some(format!("{host}:{port}")),
        None => Some(host),
    }
}

fn unconfigured_details(target: BatchSetupTargetId, note: Option<String>) -> NativeTargetDetails {
    NativeTargetDetails {
        snapshot: TargetModelSnapshot {
            target,
            configured: false,
            provider_id: None,
            provider_name: None,
            base_url: None,
            endpoint_host: None,
            model_id: None,
            protocol: None,
            model_count: 0,
            credential_source: ModelCredentialSource::None,
            can_probe: false,
            can_clone_source: false,
            can_rotate_key: false,
            status_note: note,
        },
        api_key: None,
        extra_model_ids: Vec::new(),
    }
}

fn is_proxy_managed_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.eq_ignore_ascii_case("PROXY_MANAGED") || trimmed.eq_ignore_ascii_case("proxy-managed")
}

fn inspect_workbuddy_target() -> NativeTargetDetails {
    let paths = current_paths();
    let Ok((loaded, _)) = load_workbuddy_files(&paths) else {
        return unconfigured_details(BatchSetupTargetId::Workbuddy, None);
    };
    let models = loaded.document.models();
    let Some(first) = models.iter().find(|item| {
        item.get("id")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
            && item
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
    }) else {
        return unconfigured_details(BatchSetupTargetId::Workbuddy, None);
    };

    let model_id = first
        .get("id")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let base_url = first
        .get("url")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let provider_name = first
        .get("vendor")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| extract_endpoint_host(Some(&base_url)));
    let raw_key = first
        .get("apiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && !is_proxy_managed_placeholder(s))
        .map(|s| Zeroizing::new(s.to_string()));
    let is_subscription_proxy = first
        .get("apiKey")
        .and_then(Value::as_str)
        .is_some_and(is_proxy_managed_placeholder);

    let endpoint_host = extract_endpoint_host(Some(&base_url));
    let extra_model_ids: Vec<String> = models
        .iter()
        .filter(|item| {
            extract_endpoint_host(item.get("url").and_then(Value::as_str)) == endpoint_host
        })
        .filter_map(|item| item.get("id").and_then(Value::as_str).map(str::trim))
        .filter(|id| !id.is_empty() && *id != model_id)
        .map(ToOwned::to_owned)
        .collect();

    let credential_source = if is_subscription_proxy {
        ModelCredentialSource::Oauth
    } else if raw_key.is_some() {
        ModelCredentialSource::ApiKey
    } else {
        ModelCredentialSource::None
    };

    NativeTargetDetails {
        snapshot: TargetModelSnapshot {
            target: BatchSetupTargetId::Workbuddy,
            configured: true,
            provider_id: None,
            provider_name,
            base_url: Some(base_url),
            endpoint_host,
            model_id: Some(model_id),
            protocol: Some(ApiProtocol::Chat),
            model_count: models.len(),
            credential_source,
            can_probe: !is_subscription_proxy,
            can_clone_source: raw_key.is_some(),
            can_rotate_key: !is_subscription_proxy,
            status_note: is_subscription_proxy.then(|| "使用订阅转发".to_string()),
        },
        api_key: raw_key,
        extra_model_ids,
    }
}

fn inspect_opencode_target() -> NativeTargetDetails {
    let Ok(snapshot) = get_opencode_model_snapshot() else {
        return unconfigured_details(BatchSetupTargetId::Opencode, None);
    };
    let Some(provider) = snapshot
        .providers
        .iter()
        .find(|p| !p.id.starts_with("fyagent-") || !p.model_ids.is_empty())
        .or_else(|| snapshot.providers.first())
    else {
        return unconfigured_details(BatchSetupTargetId::Opencode, None);
    };

    let Some(first_model) = provider.model_ids.first().cloned() else {
        return unconfigured_details(BatchSetupTargetId::Opencode, None);
    };
    let extra_model_ids: Vec<String> = provider.model_ids.iter().skip(1).cloned().collect();

    match resolve_saved_opencode_probe_connection(&provider.id) {
        Ok((base_url, api_key)) => {
            let clean_key = api_key.trim();
            let has_real_key = !clean_key.is_empty() && !is_proxy_managed_placeholder(clean_key);
            let endpoint_host = extract_endpoint_host(Some(&base_url));
            NativeTargetDetails {
                snapshot: TargetModelSnapshot {
                    target: BatchSetupTargetId::Opencode,
                    configured: true,
                    provider_id: Some(provider.id.clone()),
                    provider_name: Some(provider.name.clone()),
                    base_url: Some(base_url),
                    endpoint_host,
                    model_id: Some(first_model),
                    protocol: Some(ApiProtocol::Chat),
                    model_count: provider.model_ids.len(),
                    credential_source: if has_real_key {
                        ModelCredentialSource::ApiKey
                    } else {
                        ModelCredentialSource::None
                    },
                    can_probe: true,
                    can_clone_source: has_real_key,
                    can_rotate_key: true,
                    status_note: None,
                },
                api_key: has_real_key.then(|| Zeroizing::new(clean_key.to_string())),
                extra_model_ids,
            }
        }
        Err(reason) => {
            let is_managed = reason.contains("受管订阅");
            NativeTargetDetails {
                snapshot: TargetModelSnapshot {
                    target: BatchSetupTargetId::Opencode,
                    configured: true,
                    provider_id: Some(provider.id.clone()),
                    provider_name: Some(provider.name.clone()),
                    base_url: None,
                    endpoint_host: None,
                    model_id: Some(first_model),
                    protocol: Some(ApiProtocol::Chat),
                    model_count: provider.model_ids.len(),
                    credential_source: if is_managed {
                        ModelCredentialSource::Oauth
                    } else {
                        ModelCredentialSource::None
                    },
                    can_probe: false,
                    can_clone_source: false,
                    can_rotate_key: false,
                    status_note: Some(reason),
                },
                api_key: None,
                extra_model_ids,
            }
        }
    }
}

fn inspect_omp_target() -> NativeTargetDetails {
    let Ok(read) = omp_config::read_omp_config() else {
        return unconfigured_details(BatchSetupTargetId::Omp, None);
    };
    if matches!(read.models_file_status, OmpFileStatus::LegacyJsonOnly) {
        return unconfigured_details(
            BatchSetupTargetId::Omp,
            Some("旧版 models.json 需先迁移".to_string()),
        );
    }
    if matches!(read.models_file_status, OmpFileStatus::InvalidYaml)
        || matches!(read.config_file_status, OmpFileStatus::InvalidYaml)
    {
        return unconfigured_details(
            BatchSetupTargetId::Omp,
            Some("YAML 语法无法解析".to_string()),
        );
    }

    let default_pair = read
        .default_model
        .as_deref()
        .and_then(|s| s.split_once('/'))
        .map(|(p, m)| (p.trim(), m.trim()));

    let selected = default_pair
        .and_then(|(pid, _)| read.providers.iter().find(|p| p.id == pid))
        .or_else(|| read.providers.iter().find(|p| !p.models.is_empty()))
        .or_else(|| read.providers.first());

    let Some(provider) = selected else {
        return unconfigured_details(BatchSetupTargetId::Omp, None);
    };

    let model_id = default_pair
        .filter(|(pid, _)| *pid == provider.id)
        .map(|(_, m)| m.to_string())
        .or_else(|| provider.models.first().map(|m| m.id.clone()));

    let Some(model_id) = model_id else {
        return unconfigured_details(BatchSetupTargetId::Omp, None);
    };

    let extra_model_ids: Vec<String> = provider
        .models
        .iter()
        .map(|m| m.id.clone())
        .filter(|id| id != &model_id)
        .collect();

    let protocol = match provider.api.as_deref() {
        Some("openai-responses") => ApiProtocol::Responses,
        Some("anthropic-messages") => ApiProtocol::Anthropic,
        _ => ApiProtocol::Chat,
    };
    let endpoint_host = extract_endpoint_host(provider.base_url.as_deref());
    let auth_str = provider.auth.as_deref();

    let probe_res = omp_config::resolve_omp_provider_probe_target(&provider.id);
    let (credential_source, api_key, can_probe, can_clone_source, status_note) = match probe_res {
        Ok((_, _, Some(key))) if !key.trim().is_empty() => (
            if provider.has_command_key {
                ModelCredentialSource::EnvOrCommand
            } else {
                ModelCredentialSource::ApiKey
            },
            Some(Zeroizing::new(key.trim().to_string())),
            true,
            true,
            None,
        ),
        Ok(_) => (
            if auth_str == Some("oauth") {
                ModelCredentialSource::Oauth
            } else {
                ModelCredentialSource::None
            },
            None,
            auth_str == Some("none"),
            false,
            (auth_str == Some("oauth")).then(|| "OAuth 凭据不支持克隆".to_string()),
        ),
        Err(err) => (
            if provider.has_command_key {
                ModelCredentialSource::EnvOrCommand
            } else {
                ModelCredentialSource::None
            },
            None,
            false,
            false,
            Some(err.to_string()),
        ),
    };

    NativeTargetDetails {
        snapshot: TargetModelSnapshot {
            target: BatchSetupTargetId::Omp,
            configured: true,
            provider_id: Some(provider.id.clone()),
            provider_name: Some(provider.id.clone()),
            base_url: provider.base_url.clone(),
            endpoint_host,
            model_id: Some(model_id),
            protocol: Some(protocol),
            model_count: provider.models.len(),
            credential_source,
            can_probe,
            can_clone_source,
            can_rotate_key: auth_str != Some("oauth") && !provider.has_command_key,
            status_note,
        },
        api_key,
        extra_model_ids,
    }
}

fn inspect_db_provider_target(state: &AppState, target: BatchSetupTargetId) -> NativeTargetDetails {
    let app_type = match target {
        BatchSetupTargetId::Claude => AppType::Claude,
        BatchSetupTargetId::Codex => AppType::Codex,
        BatchSetupTargetId::Grokbuild => AppType::GrokBuild,
        _ => return unconfigured_details(target, None),
    };
    let Ok(Some(current_id)) = state.db.get_current_provider(app_type.as_str()) else {
        return unconfigured_details(target, None);
    };
    let Ok(Some(stored)) = state.db.get_provider_by_id(&current_id, app_type.as_str()) else {
        return unconfigured_details(target, None);
    };

    if stored.category.as_deref() == Some("official") || stored.uses_subscription_proxy() {
        return NativeTargetDetails {
            snapshot: TargetModelSnapshot {
                target,
                configured: true,
                provider_id: Some(stored.id),
                provider_name: Some(stored.name),
                base_url: None,
                endpoint_host: None,
                model_id: None,
                protocol: None,
                model_count: 1,
                credential_source: ModelCredentialSource::Oauth,
                can_probe: false,
                can_clone_source: false,
                can_rotate_key: false,
                status_note: Some("官方订阅账号".to_string()),
            },
            api_key: None,
            extra_model_ids: Vec::new(),
        };
    }

    let resolved = ProviderCredentials::resolve(&state.db, app_type.as_str(), &stored)
        .unwrap_or_else(|_| stored.clone());
    extract_db_provider_details(target, &app_type, &resolved)
}

fn extract_db_provider_details(
    target: BatchSetupTargetId,
    app_type: &AppType,
    provider: &Provider,
) -> NativeTargetDetails {
    let settings = &provider.settings_config;
    let (base_url, model_id, protocol, api_key): (
        Option<String>,
        Option<String>,
        Option<ApiProtocol>,
        Option<Zeroizing<String>>,
    ) = match app_type {
        AppType::Claude => {
            let env = settings.get("env").and_then(Value::as_object);
            let base = env
                .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let model = env
                .and_then(|e| e.get("ANTHROPIC_MODEL"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let key = env
                .and_then(|e| {
                    e.get("ANTHROPIC_AUTH_TOKEN")
                        .or_else(|| e.get("ANTHROPIC_API_KEY"))
                })
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty() && !is_proxy_managed_placeholder(s))
                .map(|s| Zeroizing::new(s.to_string()));
            (base, model, Some(ApiProtocol::Anthropic), key)
        }
        AppType::Codex => {
            let config_text = settings.get("config").and_then(Value::as_str);
            let parsed = config_text.and_then(|text| text.parse::<toml::Value>().ok());
            let selected = parsed
                .as_ref()
                .and_then(|p| p.get("model_provider"))
                .and_then(toml::Value::as_str);
            let mp = parsed
                .as_ref()
                .zip(selected)
                .and_then(|(p, s)| p.get("model_providers").and_then(|m| m.get(s)));
            let base = mp
                .and_then(|m| m.get("base_url"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let model = parsed
                .as_ref()
                .and_then(|p| p.get("model"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let proto = match mp
                .and_then(|m| m.get("wire_api"))
                .and_then(toml::Value::as_str)
            {
                Some("chat") => ApiProtocol::Chat,
                _ => ApiProtocol::Responses,
            };
            let key = crate::codex_config::extract_codex_api_key(settings.get("auth"), config_text)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !is_proxy_managed_placeholder(s))
                .map(Zeroizing::new);
            (base, model, Some(proto), key)
        }
        AppType::GrokBuild => {
            let config_text = settings.get("config").and_then(Value::as_str);
            let parsed = config_text.and_then(|text| text.parse::<toml::Value>().ok());
            let default_model = parsed
                .as_ref()
                .and_then(|p| p.get("models"))
                .and_then(|m| m.get("default"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let entry = parsed
                .as_ref()
                .zip(default_model.as_deref())
                .and_then(|(p, def)| p.get("model").and_then(|m| m.get(def)));
            let base = entry
                .and_then(|e| e.get("base_url"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let key = entry
                .and_then(|e| e.get("api_key"))
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty() && !is_proxy_managed_placeholder(s))
                .map(|s| Zeroizing::new(s.to_string()));
            (base, default_model, Some(ApiProtocol::Responses), key)
        }
        _ => (None, None, None, None),
    };

    let endpoint_host = extract_endpoint_host(base_url.as_deref());
    let has_base = base_url.is_some();
    let has_model = model_id.is_some();
    let has_key = api_key.is_some();

    NativeTargetDetails {
        snapshot: TargetModelSnapshot {
            target,
            configured: has_base || has_model,
            provider_id: Some(provider.id.clone()),
            provider_name: Some(provider.name.clone()),
            base_url,
            endpoint_host,
            model_id,
            protocol,
            model_count: usize::from(has_model),
            credential_source: if has_key {
                ModelCredentialSource::ApiKey
            } else {
                ModelCredentialSource::None
            },
            can_probe: has_base && has_model,
            can_clone_source: has_base && has_model && has_key,
            can_rotate_key: has_base && has_model,
            status_note: None,
        },
        api_key,
        extra_model_ids: Vec::new(),
    }
}

fn inspect_target(state: &AppState, target: BatchSetupTargetId) -> NativeTargetDetails {
    match target {
        BatchSetupTargetId::Workbuddy => inspect_workbuddy_target(),
        BatchSetupTargetId::Opencode => inspect_opencode_target(),
        BatchSetupTargetId::Omp => inspect_omp_target(),
        BatchSetupTargetId::Claude | BatchSetupTargetId::Codex | BatchSetupTargetId::Grokbuild => {
            inspect_db_provider_target(state, target)
        }
    }
}

pub fn get_model_health_overview(state: &AppState) -> Result<ModelHealthOverview, String> {
    let targets = ALL_CONFIGURABLE_TARGETS
        .into_iter()
        .map(|target| inspect_target(state, target).snapshot)
        .collect();
    Ok(ModelHealthOverview { targets })
}

/// Maps well-known provider base URLs between OpenAI-compatible and Anthropic
/// endpoints when cloning across heterogeneous targets. Returns `(mapped_url,
/// optional_warning)`.
pub(crate) fn adapt_base_url_for_target(
    source_target: BatchSetupTargetId,
    source_base_url: &str,
    dest_target: BatchSetupTargetId,
) -> (String, Option<String>) {
    let trimmed = source_base_url.trim().trim_end_matches('/');
    let host = extract_endpoint_host(Some(trimmed)).unwrap_or_default();

    let dest_wants_anthropic = dest_target == BatchSetupTargetId::Claude;
    let source_is_anthropic = source_target == BatchSetupTargetId::Claude;

    if dest_wants_anthropic == source_is_anthropic {
        return (trimmed.to_string(), None);
    }

    // Known built-in providers with separate OpenAI and Anthropic paths.
    let known_pair: Option<(&str, &str)> = match host.as_str() {
        "api.deepseek.com" => Some((
            "https://api.deepseek.com/v1",
            "https://api.deepseek.com/anthropic",
        )),
        "api.moonshot.cn" => Some((
            "https://api.moonshot.cn/v1",
            "https://api.moonshot.cn/anthropic",
        )),
        "open.bigmodel.cn" => Some((
            "https://open.bigmodel.cn/api/paas/v4",
            "https://open.bigmodel.cn/api/anthropic",
        )),
        "dashscope.aliyuncs.com" => Some((
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "https://dashscope.aliyuncs.com/apps/anthropic",
        )),
        "ark.cn-beijing.volces.com" => Some((
            "https://ark.cn-beijing.volces.com/api/v3",
            "https://ark.cn-beijing.volces.com/api/coding",
        )),
        "api.minimaxi.com" => Some((
            "https://api.minimaxi.com/v1",
            "https://api.minimaxi.com/anthropic",
        )),
        _ => None,
    };

    if let Some((openai_url, anthropic_url)) = known_pair {
        if dest_wants_anthropic {
            return (anthropic_url.to_string(), None);
        }
        return (openai_url.to_string(), None);
    }

    let warning = if dest_wants_anthropic {
        Some("Claude 使用 Anthropic 协议；若该网关有独立 Anthropic 地址，请在上方覆盖".to_string())
    } else {
        Some("已从 Claude 地址克隆；若该网关 OpenAI 路径不同，请在上方覆盖".to_string())
    };
    (trimmed.to_string(), warning)
}

fn default_protocol_for_clone(
    source_protocol: Option<ApiProtocol>,
    dest_target: BatchSetupTargetId,
) -> ApiProtocol {
    match dest_target {
        BatchSetupTargetId::Claude => ApiProtocol::Anthropic,
        BatchSetupTargetId::Workbuddy | BatchSetupTargetId::Opencode | BatchSetupTargetId::Omp => {
            ApiProtocol::Chat
        }
        BatchSetupTargetId::Codex | BatchSetupTargetId::Grokbuild => match source_protocol {
            Some(ApiProtocol::Chat) => ApiProtocol::Chat,
            _ => ApiProtocol::Responses,
        },
    }
}

fn sanitize_provider_slug(raw: &str) -> String {
    let mut slug = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.' | ' ') && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "fyagent-cloned".to_string()
    } else {
        format!("fyagent-{}", &trimmed[..trimmed.len().min(32)])
    }
}

struct PlannedCloneSpec {
    spec: BatchSetupTargetSpec,
    warning: Option<String>,
}

fn dedup_targets(targets: &[BatchSetupTargetId]) -> Vec<BatchSetupTargetId> {
    let mut seen = std::collections::HashSet::with_capacity(targets.len());
    let mut out = Vec::with_capacity(targets.len());
    for target in targets {
        if seen.insert(*target) {
            out.push(*target);
        }
    }
    out
}

fn build_clone_specs(
    state: &AppState,
    request: &CloneModelConfigRequest,
) -> Result<(NativeTargetDetails, Vec<PlannedCloneSpec>), String> {
    let destinations: Vec<BatchSetupTargetId> = dedup_targets(&request.destination_targets)
        .into_iter()
        .filter(|target| *target != request.source_target)
        .collect();
    if destinations.is_empty() {
        return Err("请至少选择 1 个其他目标应用".to_string());
    }

    let source = inspect_target(state, request.source_target);
    if !source.snapshot.configured {
        return Err("来源应用尚未配置模型，无法克隆".to_string());
    }
    let source_base_url = source
        .snapshot
        .base_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "来源应用缺少服务地址（官方账号不支持克隆）".to_string())?;
    let model_id = request
        .model_id_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| source.snapshot.model_id.clone())
        .ok_or_else(|| "来源应用缺少模型 ID".to_string())?;
    let provider_name = source
        .snapshot
        .provider_name
        .clone()
        .or_else(|| source.snapshot.endpoint_host.clone())
        .unwrap_or_else(|| "Cloned Provider".to_string());
    let provider_id = sanitize_provider_slug(&provider_name);

    let mut specs = Vec::with_capacity(destinations.len());
    for dest in destinations {
        let (base_url, warning) = if let Some(custom_url) = request
            .base_url_overrides
            .get(&dest)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            (custom_url.to_string(), None)
        } else {
            adapt_base_url_for_target(request.source_target, source_base_url, dest)
        };
        let spec = BatchSetupTargetSpec {
            target: dest,
            provider_id: provider_id.clone(),
            provider_name: provider_name.clone(),
            base_url,
            model_id: model_id.clone(),
            extra_model_ids: Vec::new(),
            protocol: Some(default_protocol_for_clone(source.snapshot.protocol, dest)),
        };
        specs.push(PlannedCloneSpec { spec, warning });
    }

    Ok((source, specs))
}

pub fn preview_clone_model_config(
    state: &AppState,
    request: CloneModelConfigRequest,
) -> Result<BatchProviderPreviewResult, String> {
    let (source, planned) = build_clone_specs(state, &request)?;
    if source.api_key.is_none() {
        return Err(
            "来源应用未保存可直接克隆的 API Key（官方登录或命令密钥不支持克隆）".to_string(),
        );
    }
    let warnings: HashMap<BatchSetupTargetId, String> = planned
        .iter()
        .filter_map(|item| item.warning.clone().map(|w| (item.spec.target, w)))
        .collect();
    let specs: Vec<BatchSetupTargetSpec> = planned.into_iter().map(|item| item.spec).collect();
    let mut preview = preview_batch_provider_setup(BatchProviderPreviewRequest { targets: specs })?;
    for item in &mut preview.items {
        if item.warning.is_none() {
            if let Some(extra_warn) = warnings.get(&item.target) {
                item.warning = Some(extra_warn.clone());
            }
        }
    }
    Ok(preview)
}

pub fn apply_clone_model_config(
    state: &AppState,
    request: CloneModelConfigRequest,
) -> Result<BatchProviderApplyResult, String> {
    let (source, planned) = build_clone_specs(state, &request)?;
    let api_key = source.api_key.ok_or_else(|| {
        "来源应用未保存可直接克隆的 API Key（官方登录或命令密钥不支持克隆）".to_string()
    })?;
    let specs: Vec<BatchSetupTargetSpec> = planned.into_iter().map(|item| item.spec).collect();
    apply_batch_provider_setup(
        state,
        BatchProviderApplyRequest {
            api_key: api_key.as_str().to_string(),
            targets: specs,
        },
    )
}

fn build_rotation_spec(details: &NativeTargetDetails) -> Result<BatchSetupTargetSpec, String> {
    if !details.snapshot.configured || !details.snapshot.can_rotate_key {
        return Err(details
            .snapshot
            .status_note
            .clone()
            .unwrap_or_else(|| "当前应用未配置自定义 API Key，无法轮换".to_string()));
    }
    let base_url = details
        .snapshot
        .base_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "缺少服务地址，无法轮换".to_string())?;
    let model_id = details
        .snapshot
        .model_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "缺少模型 ID，无法轮换".to_string())?;
    let provider_name = details
        .snapshot
        .provider_name
        .clone()
        .or_else(|| details.snapshot.endpoint_host.clone())
        .unwrap_or_else(|| "Custom Provider".to_string());
    let provider_id = details
        .snapshot
        .provider_id
        .clone()
        .unwrap_or_else(|| sanitize_provider_slug(&provider_name));

    Ok(BatchSetupTargetSpec {
        target: details.snapshot.target,
        provider_id,
        provider_name,
        base_url,
        model_id,
        extra_model_ids: details.extra_model_ids.clone(),
        protocol: details.snapshot.protocol,
    })
}

pub fn preview_api_key_rotation(
    state: &AppState,
    request: ApiKeyRotationPreviewRequest,
) -> Result<BatchProviderPreviewResult, String> {
    let targets = dedup_targets(&request.targets);
    if targets.is_empty() || targets.len() > 8 {
        return Err("请至少选择 1 个要轮换 Key 的应用".to_string());
    }
    let mut ready_specs = Vec::new();
    let mut blocked_items = Vec::new();

    for target in targets {
        let details = inspect_target(state, target);
        match build_rotation_spec(&details) {
            Ok(spec) => ready_specs.push(spec),
            Err(reason) => blocked_items.push(BatchProviderPreviewItem {
                target,
                paths: Vec::new(),
                base_url: details.snapshot.base_url.unwrap_or_default(),
                model_id: details.snapshot.model_id.unwrap_or_default(),
                ready: false,
                warning: Some(reason),
            }),
        }
    }

    let mut items = if ready_specs.is_empty() {
        Vec::new()
    } else {
        preview_batch_provider_setup(BatchProviderPreviewRequest {
            targets: ready_specs,
        })?
        .items
    };
    items.extend(blocked_items);
    Ok(BatchProviderPreviewResult { items })
}

pub fn apply_api_key_rotation(
    state: &AppState,
    request: ApiKeyRotationApplyRequest,
) -> Result<BatchProviderApplyResult, String> {
    let targets = dedup_targets(&request.targets);
    if targets.is_empty() || targets.len() > 8 {
        return Err("请至少选择 1 个要轮换 Key 的应用".to_string());
    }
    let normalized_key = normalize_batch_api_key(&request.api_key)?;
    let mut specs = Vec::new();
    for target in targets {
        let details = inspect_target(state, target);
        specs.push(build_rotation_spec(&details)?);
    }
    apply_batch_provider_setup(
        state,
        BatchProviderApplyRequest {
            api_key: normalized_key.as_str().to_string(),
            targets: specs,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_known_provider_urls_between_openai_and_anthropic() {
        let (claude_url, warn) = adapt_base_url_for_target(
            BatchSetupTargetId::Codex,
            "https://api.deepseek.com/v1",
            BatchSetupTargetId::Claude,
        );
        assert_eq!(claude_url, "https://api.deepseek.com/anthropic");
        assert!(warn.is_none());

        let (openai_url, warn2) = adapt_base_url_for_target(
            BatchSetupTargetId::Claude,
            "https://open.bigmodel.cn/api/anthropic",
            BatchSetupTargetId::Opencode,
        );
        assert_eq!(openai_url, "https://open.bigmodel.cn/api/paas/v4");
        assert!(warn2.is_none());
    }

    #[test]
    fn unknown_gateway_warns_when_cloning_across_anthropic_boundary() {
        let (url, warn) = adapt_base_url_for_target(
            BatchSetupTargetId::Codex,
            "https://gateway.internal.example/v1",
            BatchSetupTargetId::Claude,
        );
        assert_eq!(url, "https://gateway.internal.example/v1");
        assert!(warn.is_some());
    }

    #[test]
    fn overview_snapshot_serialization_never_contains_secret_fields() {
        let snap = TargetModelSnapshot {
            target: BatchSetupTargetId::Codex,
            configured: true,
            provider_id: Some("p1".to_string()),
            provider_name: Some("DeepSeek".to_string()),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
            endpoint_host: Some("api.deepseek.com".to_string()),
            model_id: Some("deepseek-v4-pro".to_string()),
            protocol: Some(ApiProtocol::Responses),
            model_count: 1,
            credential_source: ModelCredentialSource::ApiKey,
            can_probe: true,
            can_clone_source: true,
            can_rotate_key: true,
            status_note: None,
        };
        let serialized = serde_json::to_string(&snap).unwrap();
        let json: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let obj = json.as_object().expect("snapshot serializes to object");
        assert!(!obj.contains_key("apiKey"));
        assert!(!obj.contains_key("api_key"));
        assert!(!obj.contains_key("secret"));
        assert!(!serialized.contains("sk-"));
        assert_eq!(
            obj.get("credentialSource").and_then(|v| v.as_str()),
            Some("api_key")
        );
        assert_eq!(
            obj.get("endpointHost").and_then(|v| v.as_str()),
            Some("api.deepseek.com")
        );
    }

    #[test]
    fn rotation_rejects_oauth_or_unconfigured_targets() {
        let oauth_details = NativeTargetDetails {
            snapshot: TargetModelSnapshot {
                target: BatchSetupTargetId::Codex,
                configured: true,
                provider_id: Some("codex-official".to_string()),
                provider_name: Some("OpenAI Official".to_string()),
                base_url: None,
                endpoint_host: None,
                model_id: None,
                protocol: None,
                model_count: 1,
                credential_source: ModelCredentialSource::Oauth,
                can_probe: false,
                can_clone_source: false,
                can_rotate_key: false,
                status_note: Some("官方订阅账号".to_string()),
            },
            api_key: None,
            extra_model_ids: Vec::new(),
        };
        let err = build_rotation_spec(&oauth_details).unwrap_err();
        assert!(err.contains("官方订阅账号"));
    }
}
