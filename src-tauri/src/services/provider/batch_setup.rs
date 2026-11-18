//! Cross-app batch preset assembly service.
//!
//! Previews and applies a single user-pasted API key across multiple selected
//! configurable clients (`workbuddy`, `codex`, `claude`, `opencode`, `omp`,
//! `grokbuild`). Each target executes through its authoritative atomic writer
//! with pre-write backup, readback verification, and independent per-target
//! rollback so a failure on one client never corrupts other succeeded clients.

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{
    claude_write_projection::ClaudeOverall, ClaudeQuickSetupApplyRequest, ProviderService,
};
use crate::{
    app_config::AppType,
    omp_config::{self, OmpFileStatus, OmpModelInput, OmpProviderPatch},
    provider::{Provider, ProviderMeta},
    services::{
        opencode_models::{
            get_opencode_model_snapshot, save_opencode_models, SaveOpenCodeModelsOutcome,
            SaveOpenCodeModelsRequest,
        },
        provider_api::{validate_credential_scope, ApiProtocol},
        workbuddy::{
            get_workbuddy_model_ids, get_workbuddy_status, save_workbuddy_models,
            SaveWorkBuddyModelsOutcome, SaveWorkBuddyModelsRequest,
        },
    },
    store::AppState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchSetupTargetId {
    Workbuddy,
    Codex,
    Claude,
    Opencode,
    Omp,
    Grokbuild,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchSetupTargetSpec {
    pub target: BatchSetupTargetId,
    pub provider_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub model_id: String,
    #[serde(default)]
    pub extra_model_ids: Vec<String>,
    #[serde(default)]
    pub protocol: Option<ApiProtocol>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchProviderPreviewRequest {
    pub targets: Vec<BatchSetupTargetSpec>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProviderPreviewItem {
    pub target: BatchSetupTargetId,
    pub paths: Vec<String>,
    pub base_url: String,
    pub model_id: String,
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProviderPreviewResult {
    pub items: Vec<BatchProviderPreviewItem>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchProviderApplyRequest {
    pub api_key: String,
    pub targets: Vec<BatchSetupTargetSpec>,
}

impl fmt::Debug for BatchProviderApplyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchProviderApplyRequest")
            .field("api_key", &"[REDACTED]")
            .field("target_count", &self.targets.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchTargetStatus {
    Applied,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTargetOutcome {
    pub target: BatchSetupTargetId,
    pub status: BatchTargetStatus,
    pub message: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProviderApplyResult {
    pub outcomes: Vec<BatchTargetOutcome>,
}

pub fn preview_batch_provider_setup(
    request: BatchProviderPreviewRequest,
) -> Result<BatchProviderPreviewResult, String> {
    if request.targets.is_empty() || request.targets.len() > 8 {
        return Err("请选择至少 1 个目标应用".to_string());
    }
    let mut items = Vec::with_capacity(request.targets.len());
    for spec in request.targets {
        items.push(preview_single_target(&spec));
    }
    Ok(BatchProviderPreviewResult { items })
}

fn preview_single_target(spec: &BatchSetupTargetSpec) -> BatchProviderPreviewItem {
    let base_url = spec.base_url.trim().to_string();
    let model_id = spec.model_id.trim().to_string();
    match spec.target {
        BatchSetupTargetId::Workbuddy => match futures::executor::block_on(get_workbuddy_status()) {
            Ok(status) => BatchProviderPreviewItem {
                target: spec.target,
                paths: vec![status.path],
                base_url,
                model_id,
                ready: true,
                warning: None,
            },
            Err(_) => BatchProviderPreviewItem {
                target: spec.target,
                paths: Vec::new(),
                base_url,
                model_id,
                ready: false,
                warning: Some("无法读取 WorkBuddy 配置文件，请检查权限".to_string()),
            },
        },
        BatchSetupTargetId::Codex | BatchSetupTargetId::Claude | BatchSetupTargetId::Grokbuild => {
            let app_type = match spec.target {
                BatchSetupTargetId::Codex => AppType::Codex,
                BatchSetupTargetId::Claude => AppType::Claude,
                BatchSetupTargetId::Grokbuild => AppType::GrokBuild,
                _ => unreachable!(),
            };
            match ProviderService::quick_setup_write_targets(&app_type) {
                Ok(targets) => BatchProviderPreviewItem {
                    target: spec.target,
                    paths: targets.into_iter().map(|t| t.path).collect(),
                    base_url,
                    model_id,
                    ready: true,
                    warning: None,
                },
                Err(_) => BatchProviderPreviewItem {
                    target: spec.target,
                    paths: Vec::new(),
                    base_url,
                    model_id,
                    ready: false,
                    warning: Some("无法解析目标配置文件路径".to_string()),
                },
            }
        }
        BatchSetupTargetId::Opencode => match get_opencode_model_snapshot() {
            Ok(snapshot) => BatchProviderPreviewItem {
                target: spec.target,
                paths: vec![snapshot.path],
                base_url,
                model_id,
                ready: true,
                warning: None,
            },
            Err(_) => BatchProviderPreviewItem {
                target: spec.target,
                paths: Vec::new(),
                base_url,
                model_id,
                ready: false,
                warning: Some("无法读取 OpenCode 配置文件（可能存在语法错误）".to_string()),
            },
        },
        BatchSetupTargetId::Omp => match omp_config::read_omp_config() {
            Ok(read) => {
                let warning = if matches!(read.models_file_status, OmpFileStatus::LegacyJsonOnly) {
                    Some("检测到旧版 models.json，请先运行一次 omp 迁移为 models.yml".to_string())
                } else if matches!(
                    read.models_file_status,
                    OmpFileStatus::InvalidYaml
                ) || matches!(read.config_file_status, OmpFileStatus::InvalidYaml)
                {
                    Some("检测到无效 YAML 语法，已跳过以保护原文件".to_string())
                } else {
                    None
                };
                BatchProviderPreviewItem {
                    target: spec.target,
                    paths: vec![read.models_path, read.config_path],
                    base_url,
                    model_id,
                    ready: warning.is_none(),
                    warning,
                }
            }
            Err(_) => BatchProviderPreviewItem {
                target: spec.target,
                paths: Vec::new(),
                base_url,
                model_id,
                ready: false,
                warning: Some("无法读取 omp 配置目录".to_string()),
            },
        },
    }
}

pub fn normalize_batch_api_key(raw: &str) -> Result<Zeroizing<String>, String> {
    let trimmed = Zeroizing::new(raw.trim().to_string());
    if trimmed.is_empty() {
        return Err("请输入有效的 API Key".to_string());
    }
    if trimmed.len() > 512 || trimmed.chars().any(char::is_control) {
        return Err("API Key 包含非法控制字符或超出长度限制".to_string());
    }
    Ok(trimmed)
}

pub fn apply_batch_provider_setup(
    state: &AppState,
    request: BatchProviderApplyRequest,
) -> Result<BatchProviderApplyResult, String> {
    if request.targets.is_empty() || request.targets.len() > 8 {
        return Err("请选择至少 1 个目标应用".to_string());
    }
    let api_key = normalize_batch_api_key(&request.api_key)?;
    let mut outcomes = Vec::with_capacity(request.targets.len());
    for spec in &request.targets {
        let outcome = apply_single_target(state, spec, api_key.as_str());
        outcomes.push(outcome);
    }
    Ok(BatchProviderApplyResult { outcomes })
}

fn collect_model_ids(spec: &BatchSetupTargetSpec) -> Vec<String> {
    let mut ids = vec![spec.model_id.trim().to_string()];
    for extra in &spec.extra_model_ids {
        let trimmed = extra.trim();
        if !trimmed.is_empty() && !ids.iter().any(|existing| existing == trimmed) {
            ids.push(trimmed.to_string());
        }
    }
    ids
}

fn build_provider_for_app(
    app_type: &AppType,
    spec: &BatchSetupTargetSpec,
    api_key: &str,
) -> Result<Provider, String> {
    let protocol = ApiProtocol::for_target(app_type.as_str(), spec.protocol)
        .map_err(|_| "当前目标不支持所选 API 协议".to_string())?;
    let name = spec.provider_name.trim().to_string();
    let base_url = spec.base_url.trim().to_string();
    let model_id = spec.model_id.trim().to_string();
    if name.is_empty() || base_url.is_empty() || model_id.is_empty() {
        return Err("供应商名称、服务地址与模型 ID 不能为空".to_string());
    }
    validate_credential_scope(&base_url, api_key, protocol)
        .map_err(|_| "当前服务地址不符合安全传输要求".to_string())?;

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
            let quote = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".to_string());
            let config = format!(
                "model_provider = \"custom\"\nmodel = {}\ndisable_response_storage = true\n\n[model_providers.custom]\nname = {}\nbase_url = {}\nwire_api = {}\nrequires_openai_auth = true",
                quote(&model_id),
                quote(&name),
                quote(&base_url),
                quote(protocol.wire_name()),
            );
            (
                "fyagent-v2-quick-setup-codex",
                serde_json::json!({
                    "auth": { "OPENAI_API_KEY": api_key },
                    "config": config,
                }),
            )
        }
        AppType::GrokBuild => {
            let model_v = toml_edit::Value::from(model_id.as_str()).to_string();
            let name_v = toml_edit::Value::from(name.as_str()).to_string();
            let endpoint_v = toml_edit::Value::from(base_url.as_str()).to_string();
            let key_v = toml_edit::Value::from(api_key).to_string();
            (
                "fyagent-v2-quick-setup-grokbuild",
                serde_json::json!({
                    "config": format!(
                        "[models]\ndefault = {model_v}\n\n[model.{model_v}]\nmodel = {model_v}\nbase_url = {endpoint_v}\nname = {name_v}\napi_key = {key_v}\napi_backend = \"{}\"\ncontext_window = {}\n",
                        crate::grok_config::DEFAULT_API_BACKEND,
                        crate::grok_config::DEFAULT_CONTEXT_WINDOW,
                    )
                }),
            )
        }
        _ => return Err("不支持的目标应用".to_string()),
    };

    let mut provider = Provider::with_id(id.to_string(), name, settings_config, None);
    provider.category = Some("custom".to_string());
    provider.notes = Some("Configured via FyAgent Batch Preset Setup".to_string());
    if protocol == ApiProtocol::Chat {
        provider
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .image_extension_configured = Some(true);
    }
    Ok(provider)
}

fn apply_single_target(
    state: &AppState,
    spec: &BatchSetupTargetSpec,
    api_key: &str,
) -> BatchTargetOutcome {
    match spec.target {
        BatchSetupTargetId::Workbuddy => apply_workbuddy(spec, api_key),
        BatchSetupTargetId::Codex => apply_codex_or_grok(state, AppType::Codex, spec, api_key),
        BatchSetupTargetId::Grokbuild => {
            apply_codex_or_grok(state, AppType::GrokBuild, spec, api_key)
        }
        BatchSetupTargetId::Claude => apply_claude(state, spec, api_key),
        BatchSetupTargetId::Opencode => apply_opencode(spec, api_key),
        BatchSetupTargetId::Omp => apply_omp(spec, api_key),
    }
}

fn apply_workbuddy(spec: &BatchSetupTargetSpec, api_key: &str) -> BatchTargetOutcome {
    let status = match futures::executor::block_on(get_workbuddy_status()) {
        Ok(s) => s,
        Err(_) => {
            return BatchTargetOutcome {
                target: BatchSetupTargetId::Workbuddy,
                status: BatchTargetStatus::Failed,
                message: "无法读取 WorkBuddy 当前状态，未改动配置文件".to_string(),
                paths: Vec::new(),
            };
        }
    };
    let paths = vec![status.path.clone()];
    let mut req = SaveWorkBuddyModelsRequest {
        base_url: spec.base_url.trim().to_string(),
        api_key: api_key.to_string(),
        allow_no_api_key: false,
        selected_model_ids: collect_model_ids(spec),
        manual_model_ids: Vec::new(),
        removed_model_ids: Vec::new(),
        clear_existing_api_keys: false,
        expected_revision: status.revision,
        overwrite_token: None,
    };
    let outcome = match futures::executor::block_on(save_workbuddy_models(req.clone())) {
        Ok(SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired { token, .. }) => {
            req.overwrite_token = Some(token);
            futures::executor::block_on(save_workbuddy_models(req))
        }
        other => other,
    };
    match outcome {
        Ok(SaveWorkBuddyModelsOutcome::Saved { .. }) => {
            let verified = futures::executor::block_on(get_workbuddy_model_ids())
                .map(|readback| readback.ids.iter().any(|id| id == spec.model_id.trim()))
                .unwrap_or(false);
            if verified {
                BatchTargetOutcome {
                    target: BatchSetupTargetId::Workbuddy,
                    status: BatchTargetStatus::Applied,
                    message: "已写入并完成回读校验".to_string(),
                    paths,
                }
            } else {
                BatchTargetOutcome {
                    target: BatchSetupTargetId::Workbuddy,
                    status: BatchTargetStatus::Failed,
                    message: "写入后回读校验未发现目标模型".to_string(),
                    paths,
                }
            }
        }
        Ok(SaveWorkBuddyModelsOutcome::ConcurrentModification) => BatchTargetOutcome {
            target: BatchSetupTargetId::Workbuddy,
            status: BatchTargetStatus::Failed,
            message: "检测到外部并发修改，已放弃覆盖以保护原文件".to_string(),
            paths,
        },
        _ => BatchTargetOutcome {
            target: BatchSetupTargetId::Workbuddy,
            status: BatchTargetStatus::Failed,
            message: "WorkBuddy 配置写入失败，已保留原文件".to_string(),
            paths,
        },
    }
}

fn apply_codex_or_grok(
    state: &AppState,
    app_type: AppType,
    spec: &BatchSetupTargetSpec,
    api_key: &str,
) -> BatchTargetOutcome {
    let paths = ProviderService::quick_setup_write_targets(&app_type)
        .map(|list| list.into_iter().map(|t| t.path).collect::<Vec<_>>())
        .unwrap_or_default();
    let provider = match build_provider_for_app(&app_type, spec, api_key) {
        Ok(p) => p,
        Err(msg) => {
            return BatchTargetOutcome {
                target: spec.target,
                status: BatchTargetStatus::Failed,
                message: msg,
                paths,
            };
        }
    };
    match ProviderService::apply_legacy_quick_setup(state, app_type, provider) {
        Ok(_) => BatchTargetOutcome {
            target: spec.target,
            status: BatchTargetStatus::Applied,
            message: "已写入并激活当前方案".to_string(),
            paths,
        },
        Err(_) => BatchTargetOutcome {
            target: spec.target,
            status: BatchTargetStatus::Failed,
            message: "写入失败，已自动回滚至原配置".to_string(),
            paths,
        },
    }
}

fn apply_claude(state: &AppState, spec: &BatchSetupTargetSpec, api_key: &str) -> BatchTargetOutcome {
    let paths = ProviderService::quick_setup_write_targets(&AppType::Claude)
        .map(|list| list.into_iter().map(|t| t.path).collect::<Vec<_>>())
        .unwrap_or_default();
    let provider = match build_provider_for_app(&AppType::Claude, spec, api_key) {
        Ok(p) => p,
        Err(msg) => {
            return BatchTargetOutcome {
                target: BatchSetupTargetId::Claude,
                status: BatchTargetStatus::Failed,
                message: msg,
                paths,
            };
        }
    };
    let preview = match ProviderService::preview_claude_quick_setup(state, provider) {
        Ok(p) => p,
        Err(_) => {
            return BatchTargetOutcome {
                target: BatchSetupTargetId::Claude,
                status: BatchTargetStatus::Failed,
                message: "无法生成 Claude 配置写入投影，原文件未改动".to_string(),
                paths,
            };
        }
    };
    let outcome = ProviderService::apply_claude_quick_setup_preview(
        state,
        ClaudeQuickSetupApplyRequest {
            preview_id: preview.preview_id,
        },
    );
    if outcome.overall == ClaudeOverall::Applied {
        BatchTargetOutcome {
            target: BatchSetupTargetId::Claude,
            status: BatchTargetStatus::Applied,
            message: "已写入 Claude settings.json 并完成回读校验".to_string(),
            paths,
        }
    } else {
        BatchTargetOutcome {
            target: BatchSetupTargetId::Claude,
            status: BatchTargetStatus::Failed,
            message: "Claude 配置写入未完成，已自动回滚".to_string(),
            paths,
        }
    }
}

fn apply_opencode(spec: &BatchSetupTargetSpec, api_key: &str) -> BatchTargetOutcome {
    let snapshot = match get_opencode_model_snapshot() {
        Ok(s) => s,
        Err(_) => {
            return BatchTargetOutcome {
                target: BatchSetupTargetId::Opencode,
                status: BatchTargetStatus::Failed,
                message: "无法读取 OpenCode 配置文件，已跳过写入".to_string(),
                paths: Vec::new(),
            };
        }
    };
    let paths = vec![snapshot.path.clone()];
    let mut req = SaveOpenCodeModelsRequest {
        provider_id: Some(spec.provider_id.trim().to_string()),
        provider_name: spec.provider_name.trim().to_string(),
        base_url: spec.base_url.trim().to_string(),
        api_key: api_key.to_string(),
        selected_model_ids: collect_model_ids(spec),
        removed_model_ids: Vec::new(),
        expected_revision: snapshot.revision,
        overwrite_token: None,
    };
    let outcome = match save_opencode_models(req.clone()) {
        Ok(SaveOpenCodeModelsOutcome::OverwriteConfirmationRequired { token, .. }) => {
            req.overwrite_token = Some(token);
            save_opencode_models(req)
        }
        other => other,
    };
    match outcome {
        Ok(SaveOpenCodeModelsOutcome::Saved { .. }) => BatchTargetOutcome {
            target: BatchSetupTargetId::Opencode,
            status: BatchTargetStatus::Applied,
            message: "已写入 OpenCode 配置并同步密钥".to_string(),
            paths,
        },
        Ok(SaveOpenCodeModelsOutcome::ConcurrentModification) => BatchTargetOutcome {
            target: BatchSetupTargetId::Opencode,
            status: BatchTargetStatus::Failed,
            message: "检测到外部并发修改，未覆盖文件".to_string(),
            paths,
        },
        _ => BatchTargetOutcome {
            target: BatchSetupTargetId::Opencode,
            status: BatchTargetStatus::Failed,
            message: "OpenCode 配置写入失败，已保留原文件".to_string(),
            paths,
        },
    }
}

pub(crate) fn apply_omp(spec: &BatchSetupTargetSpec, api_key: &str) -> BatchTargetOutcome {
    let read_before = omp_config::read_omp_config().ok();
    let paths = read_before
        .as_ref()
        .map(|r| vec![r.models_path.clone(), r.config_path.clone()])
        .unwrap_or_default();
    let omp_api = match spec.protocol {
        Some(ApiProtocol::Responses) => "openai-responses",
        _ => "openai-completions",
    };
    let models = collect_model_ids(spec)
        .into_iter()
        .map(|id| OmpModelInput { id, name: None })
        .collect::<Vec<_>>();
    let patch = OmpProviderPatch {
        id: format!("fyagent-{}", spec.provider_id.trim()),
        base_url: spec.base_url.trim().to_string(),
        api: omp_api.to_string(),
        api_key: Some(api_key.to_string()),
        auth: Some("apiKey".to_string()),
        models,
    };
    let default_selector = format!("{}/{}", patch.id, spec.model_id.trim());
    match omp_config::apply_omp_provider_direct(patch.clone(), Some(default_selector.clone()), None)
    {
        Ok((saved, checkpoint)) => {
            let verified = omp_config::read_omp_config()
                .map(|after| {
                    after.default_model.as_deref() == Some(default_selector.as_str())
                        && after.providers.iter().any(|p| p.id == patch.id)
                })
                .unwrap_or(false);
            if verified {
                BatchTargetOutcome {
                    target: BatchSetupTargetId::Omp,
                    status: BatchTargetStatus::Applied,
                    message: "已无损更新 omp models.yml 与默认角色".to_string(),
                    paths: vec![saved.models_path, saved.config_path],
                }
            } else {
                let _ = checkpoint.rollback();
                BatchTargetOutcome {
                    target: BatchSetupTargetId::Omp,
                    status: BatchTargetStatus::Failed,
                    message: "omp 回读校验不一致，已回滚至原文件".to_string(),
                    paths,
                }
            }
        }
        Err(_) => BatchTargetOutcome {
            target: BatchSetupTargetId::Omp,
            status: BatchTargetStatus::Failed,
            message: "omp 配置写入失败，原 YAML 文件未改动".to_string(),
            paths,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_trims_pasted_api_key_with_zeroizing() {
        let key = normalize_batch_api_key("  \n sk-batch-example-123 \t\r\n ").unwrap();
        assert_eq!(key.as_str(), "sk-batch-example-123");
        assert!(normalize_batch_api_key("   \n\t ").is_err());
        assert!(normalize_batch_api_key("sk-bad\0key").is_err());
    }

    #[test]
    #[serial_test::serial]
    fn omp_target_applies_and_rolls_back_cleanly_in_isolation() {
        let _env = crate::omp_config::test_support::TestOmpEnv::new();
        let spec = BatchSetupTargetSpec {
            target: BatchSetupTargetId::Omp,
            provider_id: "deepseek".to_string(),
            provider_name: "DeepSeek".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model_id: "deepseek-v4-pro".to_string(),
            extra_model_ids: vec!["deepseek-flash".to_string()],
            protocol: Some(ApiProtocol::Chat),
        };
        let preview = preview_batch_provider_setup(BatchProviderPreviewRequest {
            targets: vec![spec.clone()],
        })
        .unwrap();
        assert!(preview.items[0].ready);

        let outcome = apply_omp(&spec, "sk-test-omp-batch");
        assert_eq!(outcome.status, BatchTargetStatus::Applied);

        let readback = omp_config::read_omp_config().unwrap();
        assert_eq!(
            readback.default_model.as_deref(),
            Some("fyagent-deepseek/deepseek-v4-pro")
        );
        assert_eq!(readback.providers[0].models.len(), 2);

        // Fault inject config write on a second update: omp must fail cleanly and preserve the first write!
        crate::omp_config::test_support::arm_config_write_fault();
        let second_spec = BatchSetupTargetSpec {
            model_id: "deepseek-flash".to_string(),
            ..spec
        };
        let failed = apply_omp(&second_spec, "sk-test-omp-batch-2");
        assert_eq!(failed.status, BatchTargetStatus::Failed);

        let after_fail = omp_config::read_omp_config().unwrap();
        assert_eq!(
            after_fail.default_model.as_deref(),
            Some("fyagent-deepseek/deepseek-v4-pro")
        );
    }
}
