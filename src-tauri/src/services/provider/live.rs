//! Live 配置的读取、首次导入和按模式分发的写入。
//!
//! 切换式应用（Claude Code、Codex、Gemini CLI、Grok Build）的客户端文件只经写入引擎写
//! （`*_direct.rs`），这里只负责分发；Claude Desktop 和累加式应用仍在这里写。

use serde_json::{json, Value};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::app_config::AppType;
use crate::config::{get_claude_settings_path, read_json_file, write_json_file, write_text_file};
use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::mcp::McpService;
use crate::store::AppState;

use super::normalize_claude_models_in_value;
const CODEX_OAUTH_CLAUDE_MAX_CONTEXT_TOKENS: &str = "372000";
const CODEX_OAUTH_CLAUDE_AUTO_COMPACT_WINDOW: &str = "372000";
const KIMI_FOR_CODING_CONTEXT_TOKENS: &str = "262144";

/// Model env keys Claude Code may route requests through. The defaults above
/// are calibrated against gpt-5.6's Codex catalog, so every configured model
/// must belong to that family before they are injected — gpt-5.5's upstream
/// catalog oscillates between 272K and 372K and must not inherit them.
const CODEX_OAUTH_MODEL_ENV_KEYS: [&str; 6] = [
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_FABLE_MODEL",
    "CLAUDE_CODE_SUBAGENT_MODEL",
];

pub(crate) fn provider_exists_in_live_config(
    app_type: &AppType,
    provider_id: &str,
) -> Result<bool, AppError> {
    match app_type {
        AppType::OpenCode => crate::opencode_config::get_providers()
            .map(|providers| providers.contains_key(provider_id)),
        AppType::OpenClaw => crate::openclaw_config::get_providers()
            .map(|providers| providers.contains_key(provider_id)),
        AppType::Hermes => crate::hermes_config::get_providers()
            .map(|providers| providers.contains_key(provider_id)),
        _ => Ok(false),
    }
}

fn json_is_subset(target: &Value, source: &Value) -> bool {
    match source {
        Value::Object(source_map) => {
            let Some(target_map) = target.as_object() else {
                return false;
            };
            source_map.iter().all(|(key, source_value)| {
                target_map
                    .get(key)
                    .is_some_and(|target_value| json_is_subset(target_value, source_value))
            })
        }
        Value::Array(source_arr) => {
            let Some(target_arr) = target.as_array() else {
                return false;
            };
            json_array_contains_subset(target_arr, source_arr)
        }
        _ => target == source,
    }
}

fn json_array_contains_subset(target_arr: &[Value], source_arr: &[Value]) -> bool {
    let mut matched = vec![false; target_arr.len()];

    source_arr.iter().all(|source_item| {
        if let Some((index, _)) = target_arr.iter().enumerate().find(|(index, target_item)| {
            !matched[*index] && json_is_subset(target_item, source_item)
        }) {
            matched[index] = true;
            true
        } else {
            false
        }
    })
}

fn json_remove_array_items(target_arr: &mut Vec<Value>, source_arr: &[Value]) {
    for source_item in source_arr {
        if let Some(index) = target_arr
            .iter()
            .position(|target_item| json_is_subset(target_item, source_item))
        {
            target_arr.remove(index);
        }
    }
}

fn json_deep_remove(target: &mut Value, source: &Value) {
    let (Some(target_map), Some(source_map)) = (target.as_object_mut(), source.as_object()) else {
        return;
    };

    for (key, source_value) in source_map {
        let mut remove_key = false;

        if let Some(target_value) = target_map.get_mut(key) {
            if source_value.is_object() && target_value.is_object() {
                json_deep_remove(target_value, source_value);
                remove_key = target_value.as_object().is_some_and(|obj| obj.is_empty());
            } else if let (Some(target_arr), Some(source_arr)) =
                (target_value.as_array_mut(), source_value.as_array())
            {
                json_remove_array_items(target_arr, source_arr);
                remove_key = target_arr.is_empty();
            } else if json_is_subset(target_value, source_value) {
                remove_key = true;
            }
        }

        if remove_key {
            target_map.remove(key);
        }
    }
}

fn toml_value_is_subset(target: &toml_edit::Value, source: &toml_edit::Value) -> bool {
    match (target, source) {
        (toml_edit::Value::String(target), toml_edit::Value::String(source)) => {
            target.value() == source.value()
        }
        (toml_edit::Value::Integer(target), toml_edit::Value::Integer(source)) => {
            target.value() == source.value()
        }
        (toml_edit::Value::Float(target), toml_edit::Value::Float(source)) => {
            target.value() == source.value()
        }
        (toml_edit::Value::Boolean(target), toml_edit::Value::Boolean(source)) => {
            target.value() == source.value()
        }
        (toml_edit::Value::Datetime(target), toml_edit::Value::Datetime(source)) => {
            target.value() == source.value()
        }
        (toml_edit::Value::Array(target), toml_edit::Value::Array(source)) => {
            toml_array_contains_subset(target, source)
        }
        (toml_edit::Value::InlineTable(target), toml_edit::Value::InlineTable(source)) => {
            source.iter().all(|(key, source_item)| {
                target
                    .get(key)
                    .is_some_and(|target_item| toml_value_is_subset(target_item, source_item))
            })
        }
        _ => false,
    }
}

fn toml_array_contains_subset(target: &toml_edit::Array, source: &toml_edit::Array) -> bool {
    let mut matched = vec![false; target.len()];
    let target_items: Vec<&toml_edit::Value> = target.iter().collect();

    source.iter().all(|source_item| {
        if let Some((index, _)) = target_items
            .iter()
            .enumerate()
            .find(|(index, target_item)| {
                !matched[*index] && toml_value_is_subset(target_item, source_item)
            })
        {
            matched[index] = true;
            true
        } else {
            false
        }
    })
}

fn toml_remove_array_items(target: &mut toml_edit::Array, source: &toml_edit::Array) {
    for source_item in source.iter() {
        let index = {
            let target_items: Vec<&toml_edit::Value> = target.iter().collect();
            target_items
                .iter()
                .enumerate()
                .find(|(_, target_item)| toml_value_is_subset(target_item, source_item))
                .map(|(index, _)| index)
        };

        if let Some(index) = index {
            target.remove(index);
        }
    }
}

fn toml_item_is_subset(target: &Item, source: &Item) -> bool {
    if let Some(source_table) = source.as_table_like() {
        let Some(target_table) = target.as_table_like() else {
            return false;
        };
        return source_table.iter().all(|(key, source_item)| {
            target_table
                .get(key)
                .is_some_and(|target_item| toml_item_is_subset(target_item, source_item))
        });
    }

    match (target.as_value(), source.as_value()) {
        (Some(target_value), Some(source_value)) => {
            toml_value_is_subset(target_value, source_value)
        }
        _ => false,
    }
}

fn remove_toml_item(target: &mut Item, source: &Item) {
    if let Some(source_table) = source.as_table_like() {
        if let Some(target_table) = target.as_table_like_mut() {
            remove_toml_table_like(target_table, source_table);
            if target_table.is_empty() {
                *target = Item::None;
            }
            return;
        }
    }

    if let Some(source_value) = source.as_value() {
        let mut remove_item = false;

        if let Some(target_value) = target.as_value_mut() {
            match (target_value, source_value) {
                (toml_edit::Value::Array(target_arr), toml_edit::Value::Array(source_arr)) => {
                    toml_remove_array_items(target_arr, source_arr);
                    remove_item = target_arr.is_empty();
                }
                (target_value, source_value)
                    if toml_value_is_subset(target_value, source_value) =>
                {
                    remove_item = true;
                }
                _ => {}
            }
        }

        if remove_item {
            *target = Item::None;
        }
    }
}

fn remove_toml_table_like(target: &mut dyn TableLike, source: &dyn TableLike) {
    let keys: Vec<String> = source.iter().map(|(key, _)| key.to_string()).collect();

    for key in keys {
        let mut remove_key = false;
        if let (Some(target_item), Some(source_item)) = (target.get_mut(&key), source.get(&key)) {
            remove_toml_item(target_item, source_item);
            remove_key = target_item.is_none()
                || target_item
                    .as_table_like()
                    .is_some_and(|table_like| table_like.is_empty());
        }

        if remove_key {
            target.remove(&key);
        }
    }
}

fn settings_contain_common_config(app_type: &AppType, settings: &Value, snippet: &str) -> bool {
    let trimmed = snippet.trim();
    if trimmed.is_empty() {
        return false;
    }

    match app_type {
        AppType::Claude => match serde_json::from_str::<Value>(trimmed) {
            Ok(source) if source.is_object() => json_is_subset(settings, &source),
            _ => false,
        },
        AppType::Codex => {
            let config_toml = settings.get("config").and_then(Value::as_str).unwrap_or("");
            if config_toml.trim().is_empty() {
                return false;
            }

            let target_doc = match config_toml.parse::<DocumentMut>() {
                Ok(doc) => doc,
                Err(_) => return false,
            };
            let source_doc = match trimmed.parse::<DocumentMut>() {
                Ok(doc) => doc,
                Err(_) => return false,
            };

            toml_item_is_subset(target_doc.as_item(), source_doc.as_item())
        }
        AppType::Gemini => match serde_json::from_str::<Value>(trimmed) {
            Ok(Value::Object(source_map)) => {
                let Some(target_map) = settings.get("env").and_then(Value::as_object) else {
                    return false;
                };
                source_map.iter().all(|(key, source_value)| {
                    target_map
                        .get(key)
                        .is_some_and(|target_value| json_is_subset(target_value, source_value))
                })
            }
            _ => false,
        },
        AppType::GrokBuild
        | AppType::OpenCode
        | AppType::OpenClaw
        | AppType::Hermes
        | AppType::ClaudeDesktop => false,
    }
}

pub(crate) fn provider_uses_common_config(
    app_type: &AppType,
    provider: &Provider,
    snippet: Option<&str>,
) -> bool {
    match provider
        .meta
        .as_ref()
        .and_then(|meta| meta.common_config_enabled)
    {
        Some(explicit) => explicit && snippet.is_some_and(|value| !value.trim().is_empty()),
        None => snippet.is_some_and(|value| {
            settings_contain_common_config(app_type, &provider.settings_config, value)
        }),
    }
}

pub(crate) fn remove_common_config_from_settings(
    app_type: &AppType,
    settings: &Value,
    snippet: &str,
) -> Result<Value, AppError> {
    let trimmed = snippet.trim();
    if trimmed.is_empty() {
        return Ok(settings.clone());
    }

    match app_type {
        AppType::Claude => {
            let source = serde_json::from_str::<Value>(trimmed)
                .map_err(|e| AppError::Message(format!("Invalid Claude common config: {e}")))?;
            let mut result = settings.clone();
            json_deep_remove(&mut result, &source);
            Ok(result)
        }
        AppType::Codex => {
            let mut result = settings.clone();
            let config_toml = settings.get("config").and_then(Value::as_str).unwrap_or("");
            let mut target_doc = if config_toml.trim().is_empty() {
                DocumentMut::new()
            } else {
                config_toml
                    .parse::<DocumentMut>()
                    .map_err(|_| AppError::Message("provider_codex_config_invalid".into()))?
            };
            let source_doc = trimmed
                .parse::<DocumentMut>()
                .map_err(|_| AppError::Message("provider_codex_common_config_invalid".into()))?;

            remove_toml_table_like(target_doc.as_table_mut(), source_doc.as_table());
            if let Some(obj) = result.as_object_mut() {
                obj.insert("config".to_string(), Value::String(target_doc.to_string()));
            }
            Ok(result)
        }
        AppType::Gemini => {
            let source = serde_json::from_str::<Value>(trimmed)
                .map_err(|e| AppError::Message(format!("Invalid Gemini common config: {e}")))?;
            let mut result = settings.clone();
            if let Some(env) = result.get_mut("env") {
                json_deep_remove(env, &source);
            }
            Ok(result)
        }
        AppType::GrokBuild
        | AppType::OpenCode
        | AppType::OpenClaw
        | AppType::Hermes
        | AppType::ClaudeDesktop => Ok(settings.clone()),
    }
}

fn apply_common_config_to_settings(
    app_type: &AppType,
    settings: &Value,
    snippet: &str,
) -> Result<Value, AppError> {
    let trimmed = snippet.trim();
    if trimmed.is_empty() {
        return Ok(settings.clone());
    }

    match app_type {
        AppType::Claude => {
            let source = serde_json::from_str::<Value>(trimmed)
                .map_err(|e| AppError::Message(format!("Invalid Claude common config: {e}")))?;
            let mut result = settings.clone();
            json_deep_merge(&mut result, &source);
            Ok(result)
        }
        AppType::Codex => {
            let mut result = settings.clone();
            let config_toml = settings.get("config").and_then(Value::as_str).unwrap_or("");
            let mut target_doc = if config_toml.trim().is_empty() {
                DocumentMut::new()
            } else {
                config_toml
                    .parse::<DocumentMut>()
                    .map_err(|_| AppError::Message("provider_codex_config_invalid".into()))?
            };
            let source_doc = trimmed
                .parse::<DocumentMut>()
                .map_err(|_| AppError::Message("provider_codex_common_config_invalid".into()))?;

            merge_toml_table_like(target_doc.as_table_mut(), source_doc.as_table());
            if let Some(obj) = result.as_object_mut() {
                obj.insert("config".to_string(), Value::String(target_doc.to_string()));
            }
            Ok(result)
        }
        AppType::Gemini => {
            let source = serde_json::from_str::<Value>(trimmed)
                .map_err(|e| AppError::Message(format!("Invalid Gemini common config: {e}")))?;
            let mut result = settings.clone();
            if let Some(env) = result.get_mut("env") {
                json_deep_merge(env, &source);
            } else if let Some(obj) = result.as_object_mut() {
                obj.insert("env".to_string(), source);
            }
            Ok(result)
        }
        AppType::GrokBuild
        | AppType::OpenCode
        | AppType::OpenClaw
        | AppType::Hermes
        | AppType::ClaudeDesktop => Ok(settings.clone()),
    }
}

/// Pure Codex projection shared by credential admission and its native
/// consumers. Callers pass one snippet snapshot; no secret is resolved here.
pub(crate) fn build_codex_credential_projection(
    provider: &Provider,
    snippet: Option<&str>,
) -> Result<Provider, AppError> {
    let mut projected = provider.clone();
    if provider_uses_common_config(&AppType::Codex, provider, snippet) {
        if let Some(snippet) = snippet {
            projected.settings_config = apply_common_config_to_settings(
                &AppType::Codex,
                &provider.settings_config,
                snippet,
            )?;
        }
    }
    Ok(projected)
}

pub(crate) fn build_effective_settings_with_common_config(
    db: &Database,
    app_type: &AppType,
    provider: &Provider,
) -> Result<Value, AppError> {
    if *app_type == AppType::Codex {
        // The credential owner validates and materializes the same effective
        // snapshot. Applying another snippet afterwards could redirect a key
        // after its destination had already been checked.
        return Ok(super::ProviderCredentials::resolve(db, "codex", provider)?.settings_config);
    }
    let snippet = db.get_config_snippet(app_type.as_str())?;
    let resolved = super::ProviderCredentials::resolve(db, app_type.as_str(), provider)?;
    let mut effective_settings = resolved.settings_config;

    if provider_uses_common_config(app_type, provider, snippet.as_deref()) {
        if let Some(snippet_text) = snippet.as_deref() {
            match apply_common_config_to_settings(app_type, &effective_settings, snippet_text) {
                Ok(settings) => effective_settings = settings,
                Err(err) => {
                    log::warn!(
                        "Failed to apply common config for {} provider '{}': {err}",
                        app_type.as_str(),
                        provider.id
                    );
                }
            }
        }
    }

    if matches!(app_type, AppType::Claude) {
        apply_codex_oauth_claude_context_defaults(&mut effective_settings, provider);
        apply_kimi_for_coding_context_defaults(&mut effective_settings, provider);
    }

    Ok(effective_settings)
}

pub(crate) fn write_live_with_common_config(
    db: &Database,
    app_type: &AppType,
    provider: &Provider,
) -> Result<(), AppError> {
    let mut effective_provider = provider.clone();
    effective_provider.settings_config =
        build_effective_settings_with_common_config(db, app_type, provider)?;

    if matches!(app_type, AppType::ClaudeDesktop) {
        crate::claude_desktop_config::apply_provider(db, provider)?;
        log::info!(
            "Claude Desktop 3P profile '{}' written for provider '{}'",
            crate::claude_desktop_config::PROFILE_ID,
            provider.id
        );
        return Ok(());
    }

    if super::is_quick_setup_provider_id(app_type, &effective_provider.id) {
        return write_quick_setup_live_snapshot(app_type, &effective_provider);
    }

    if matches!(app_type, AppType::Codex) {
        let snippet = db.get_config_snippet(app_type.as_str())?;
        let common_snippet = provider_uses_common_config(app_type, provider, snippet.as_deref())
            .then_some(snippet)
            .flatten();
        return write_codex_live_snapshot(&effective_provider, common_snippet.as_deref());
    }

    match app_type {
        AppType::Claude => {
            super::claude_direct::reapply(db, Some(provider), &effective_provider).map(|_| ())
        }
        AppType::Gemini => super::gemini_direct::reapply(db, &effective_provider).map(|_| ()),
        AppType::GrokBuild => {
            super::grok_direct::reapply(db, Some(provider), &effective_provider).map(|_| ())
        }
        _ => write_live_snapshot(app_type, &effective_provider),
    }
}

#[cfg(test)]
fn codex_managed_oauth_live_auth(
    chatgpt_account_id: &str,
    access_token: &str,
    id_token: Option<&str>,
    refresh_token: &str,
    last_refresh: &str,
) -> Value {
    // 与原生 Codex 浏览器登录的形状对齐：tokens 字段顺序 id_token、access_token、
    // refresh_token、account_id，并带顶层 last_refresh。**必须**包含 refresh_token，
    // 否则 Codex CLI 在 access_token 过期后无法自刷新（“裸跑 codex” 会静默失效）。
    crate::codex_config::codex_managed_oauth_auth_value(
        chatgpt_account_id,
        access_token,
        id_token,
        refresh_token,
        last_refresh,
    )
}

/// 切换前用同一配置引擎验证目标，并解析 SecretRef；拒绝时不能先移动 current 指针。
pub(crate) fn preflight_codex_live_write_for_state(
    state: &AppState,
    provider: &Provider,
) -> Result<(), AppError> {
    super::codex_direct::preflight(state.db.as_ref(), provider)
}

fn write_quick_setup_live_snapshot(
    app_type: &AppType,
    provider: &Provider,
) -> Result<(), AppError> {
    match app_type {
        AppType::Claude => write_quick_setup_claude_live(provider),
        AppType::Codex => write_quick_setup_codex_live(provider),
        AppType::GrokBuild => write_quick_setup_grok_live(provider),
        _ => Err(AppError::Message(
            "Quick Setup patch write is unsupported for this application".to_string(),
        )),
    }
}

fn write_quick_setup_claude_live(provider: &Provider) -> Result<(), AppError> {
    let path = get_claude_settings_path();
    let current = if path.exists() {
        read_json_file::<Value>(&path)?
    } else {
        json!({})
    };
    write_json_file(
        &path,
        &build_claude_quick_setup_live_projection(&current, provider)?,
    )
}

pub(super) fn build_claude_quick_setup_live_projection(
    current: &Value,
    provider: &Provider,
) -> Result<Value, AppError> {
    let mut current = current.clone();
    let current_obj = current.as_object_mut().ok_or_else(|| {
        AppError::Config("Claude settings.json root must be an object".to_string())
    })?;
    let source_env = provider
        .settings_config
        .get("env")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::Config("Claude Quick Setup env is missing".to_string()))?;
    if !current_obj.get("env").is_some_and(Value::is_object) {
        if current_obj.contains_key("env") {
            return Err(AppError::Config(
                "Claude settings.json env must be an object".to_string(),
            ));
        }
        current_obj.insert("env".to_string(), json!({}));
    }
    let current_env = current_obj
        .get_mut("env")
        .and_then(Value::as_object_mut)
        .expect("Claude env was normalized to an object");
    for key in [
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_MODEL",
    ] {
        let value = source_env
            .get(key)
            .ok_or_else(|| AppError::Config(format!("Claude Quick Setup is missing {key}")))?;
        current_env.insert(key.to_string(), value.clone());
    }

    Ok(current)
}

fn write_quick_setup_codex_live(provider: &Provider) -> Result<(), AppError> {
    let config_path = crate::codex_config::get_codex_config_path();
    let current_config = if config_path.exists() {
        std::fs::read_to_string(&config_path).map_err(|error| AppError::io(&config_path, error))?
    } else {
        String::new()
    };
    let current_live = json!({ "auth": json!({}), "config": current_config });
    let projected = build_codex_quick_setup_live_projection(&current_live, provider)?;
    let patched_config = projected
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::Config("Codex Quick Setup projection is missing config".to_string())
        })?;
    crate::codex_config::write_codex_live_config_atomic(Some(patched_config))
}

/// Pure projection for the fixed Codex Quick Setup provider.
///
/// Change Plan preview/readback and the real writer must agree on the exact
/// post-write shape. In particular, Quick Setup is a targeted patch over the
/// current `config.toml`; unrelated user fields/tables survive and therefore
/// belong in the expected projection rather than being treated as drift.
pub(crate) fn build_codex_quick_setup_live_projection(
    current_live: &Value,
    provider: &Provider,
) -> Result<Value, AppError> {
    let settings = provider.settings_config.as_object().ok_or_else(|| {
        AppError::Config("Codex Quick Setup settings must be an object".to_string())
    })?;
    let desired_config = settings
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Config("Codex Quick Setup config is missing".to_string()))?;
    let desired_auth = settings
        .get("auth")
        .ok_or_else(|| AppError::Config("Codex Quick Setup auth is missing".to_string()))?;
    let desired_key = desired_auth
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Config("Codex Quick Setup API key is missing".to_string()))?;
    let current_config = current_live
        .get("config")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let patched_config =
        crate::codex_config::patch_codex_quick_setup_live_config(current_config, desired_config)?;

    let _ = desired_key;
    let patched_config =
        crate::codex_config::prepare_codex_provider_live_config(desired_auth, &patched_config)?;
    let projected_auth = current_live
        .get("auth")
        .cloned()
        .unwrap_or_else(|| json!({}));

    Ok(json!({ "auth": projected_auth, "config": patched_config }))
}

fn write_quick_setup_grok_live(provider: &Provider) -> Result<(), AppError> {
    let desired_config = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Config("Grok Build Quick Setup config is missing".to_string()))?;
    let path = crate::grok_config::get_grok_config_path();
    let current_config = if path.exists() {
        std::fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?
    } else {
        String::new()
    };
    let patched = patch_grok_quick_setup_config(&current_config, desired_config)?;
    write_text_file(&path, &patched)
}

/// One model-selection patch for both ordinary Quick Setup and managed Proxy
/// activation. Preserve other models, MCP, hooks and user settings verbatim.
pub(crate) fn patch_grok_quick_setup_config(
    current_config: &str,
    desired_config: &str,
) -> Result<String, AppError> {
    let desired = desired_config.parse::<DocumentMut>().map_err(|error| {
        AppError::Message(format!("Invalid Grok Build Quick Setup TOML: {error}"))
    })?;
    let selected_model = desired
        .get("models")
        .and_then(|item| item.get("default"))
        .and_then(Item::as_str)
        .ok_or_else(|| AppError::Config("Grok Build Quick Setup model is missing".to_string()))?;
    let desired_model = desired
        .get("model")
        .and_then(|item| item.get(selected_model))
        .and_then(Item::as_table_like)
        .ok_or_else(|| {
            AppError::Config("Grok Build Quick Setup model table is missing".to_string())
        })?;

    let mut target = if current_config.trim().is_empty() {
        DocumentMut::new()
    } else {
        current_config.parse::<DocumentMut>().map_err(|error| {
            AppError::Message(format!("Invalid Grok Build config.toml: {error}"))
        })?
    };
    if target.get("models").is_none() {
        target["models"] = toml_edit::table();
    }
    let models = target
        .get_mut("models")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::Config("Grok Build models must be an editable table".to_string())
        })?;
    models.insert("default", toml_edit::value(selected_model));

    if target.get("model").is_none() {
        target["model"] = toml_edit::table();
    }
    let models_root = target
        .get_mut("model")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::Config("Grok Build model must be an editable table".to_string())
        })?;
    if models_root.get(selected_model).is_none() {
        models_root.insert(selected_model, toml_edit::table());
    }
    let target_model = models_root
        .get_mut(selected_model)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::Config("Grok Build selected model must be editable".to_string())
        })?;
    for key in [
        "model",
        "base_url",
        "name",
        "api_key",
        "api_backend",
        "context_window",
    ] {
        let item = desired_model
            .get(key)
            .ok_or_else(|| AppError::Config(format!("Grok Build Quick Setup is missing {key}")))?;
        target_model.insert(key, item.clone());
    }

    let patched = target.to_string();
    crate::grok_config::validate_config_toml(&patched)?;
    Ok(patched)
}

pub(crate) fn strip_common_config_from_live_settings(
    db: &Database,
    app_type: &AppType,
    provider: &Provider,
    live_settings: Value,
) -> Value {
    let snippet = match db.get_config_snippet(app_type.as_str()) {
        Ok(snippet) => snippet,
        Err(err) => {
            log::warn!(
                "Failed to load common config for {} while backfilling '{}': {err}",
                app_type.as_str(),
                provider.id
            );
            return restore_live_settings_for_provider_backfill(app_type, provider, live_settings);
        }
    };

    let backfill_settings = if provider_uses_common_config(app_type, provider, snippet.as_deref()) {
        match snippet.as_deref() {
            Some(snippet_text) => {
                match remove_common_config_from_settings(app_type, &live_settings, snippet_text) {
                    Ok(settings) => settings,
                    Err(err) => {
                        log::warn!(
                            "Failed to strip common config for {} provider '{}': {err}",
                            app_type.as_str(),
                            provider.id
                        );
                        live_settings
                    }
                }
            }
            None => live_settings,
        }
    } else {
        live_settings
    };

    restore_live_settings_for_provider_backfill(app_type, provider, backfill_settings)
}

/// 与 `apply_codex_oauth_claude_context_defaults` 严格对称：注入产物只活在
/// live，切走回填时必须剥掉，否则程序默认值会固化成供应商的"用户显式值"，
/// 之后调整默认值或更换模型时旧值永远压住新默认。仅当"注入会发生且注入的
/// 就是这个值、且存储配置本来没有显式值"时才剥；用户显式存储的值和手改
/// live 成其他数字的值都保留。
fn strip_injected_codex_oauth_context_defaults(settings: &mut Value, provider: &Provider) {
    if !provider.is_codex_oauth() {
        return;
    }
    let provider_env = provider
        .settings_config
        .get("env")
        .and_then(Value::as_object);
    if !provider_env_targets_gpt56(provider_env) {
        return;
    }
    let Some(env) = settings.get_mut("env").and_then(Value::as_object_mut) else {
        return;
    };
    for (key, default_value) in [
        (
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
            CODEX_OAUTH_CLAUDE_MAX_CONTEXT_TOKENS,
        ),
        (
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            CODEX_OAUTH_CLAUDE_AUTO_COMPACT_WINDOW,
        ),
    ] {
        let stored_explicit = provider_env.is_some_and(|e| e.contains_key(key));
        if stored_explicit {
            continue;
        }
        if env.get(key).and_then(Value::as_str) == Some(default_value) {
            env.remove(key);
        }
    }
}

fn strip_injected_kimi_for_coding_context_defaults(settings: &mut Value, provider: &Provider) {
    if !is_kimi_for_coding_provider(provider) {
        return;
    }
    let provider_env = provider
        .settings_config
        .get("env")
        .and_then(Value::as_object);
    let Some(env) = settings.get_mut("env").and_then(Value::as_object_mut) else {
        return;
    };
    for key in [
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
    ] {
        if provider_env.is_some_and(|provider_env| provider_env.contains_key(key)) {
            continue;
        }
        if env.get(key).and_then(Value::as_str) == Some(KIMI_FOR_CODING_CONTEXT_TOKENS) {
            env.remove(key);
        }
    }
}

fn restore_live_settings_for_provider_backfill(
    app_type: &AppType,
    provider: &Provider,
    live_settings: Value,
) -> Value {
    if matches!(app_type, AppType::Claude) {
        let mut settings = live_settings;
        strip_injected_codex_oauth_context_defaults(&mut settings, provider);
        strip_injected_kimi_for_coding_context_defaults(&mut settings, provider);
        return settings;
    }
    if matches!(app_type, AppType::GrokBuild) {
        let mut settings = live_settings;
        if let Err(err) = crate::grok_config::strip_grok_mcp_servers_from_settings(&mut settings) {
            log::warn!(
                "Failed to strip Grok Build mcp_servers while backfilling '{}': {err}",
                provider.id
            );
        }
        return settings;
    }
    if !matches!(app_type, AppType::Codex) {
        return live_settings;
    }

    let mut settings = live_settings;
    let restore_provider_token = !crate::proxy::providers::is_codex_official_provider(provider)
        && crate::codex_config::should_restore_codex_provider_token_for_backfill(
            provider.category.as_deref(),
            &provider.settings_config,
        );
    if let Err(err) = crate::codex_config::restore_codex_settings_for_backfill(
        &mut settings,
        &provider.settings_config,
        restore_provider_token,
    ) {
        log::warn!(
            "Failed to restore Codex settings while backfilling '{}': {err}",
            provider.id
        );
    }

    // A source with provider-owned credentials must retain them when Live
    // has no credential (for example header/env authentication or logout).
    // Official follow-login cards deliberately retain Live's logged-out state.
    if restore_provider_token
        && settings
            .get("auth")
            .is_none_or(|auth| !crate::codex_config::codex_auth_has_login_material(auth))
    {
        if let (Some(stored_auth), Some(obj)) = (
            provider
                .settings_config
                .get("auth")
                .filter(|auth| crate::codex_config::extract_codex_auth_api_key(auth).is_some()),
            settings.as_object_mut(),
        ) {
            obj.insert("auth".to_string(), stored_auth.clone());
        }
    }

    strip_codex_managed_oauth_auth_for_backfill(provider, &mut settings);

    // MCP 服务器归 DB mcp_servers 表所有，live 里的 [mcp_servers] 是同步投影；
    // 回填时剥掉，否则已删除的服务器会随供应商快照复活（逐条 reconcile 清不掉孤儿）。
    if let Err(err) = crate::codex_config::strip_codex_mcp_servers_from_settings(&mut settings) {
        log::warn!(
            "Failed to strip mcp_servers while backfilling '{}': {err}",
            provider.id
        );
    }

    // 统一会话开关注入的共享 `custom` 路由只属于 live 配置；切换回填时
    // 必须剥掉，否则官方供应商的存储配置被污染，关闭开关后无法还原。
    if provider.category.as_deref() == Some("official")
        || crate::proxy::providers::is_codex_official_provider(provider)
    {
        if let Err(err) =
            crate::codex_config::strip_codex_unified_session_bucket_from_settings(&mut settings)
        {
            log::warn!(
                "Failed to strip unified session bucket while backfilling '{}': {err}",
                provider.id
            );
        }
    }

    // `modelCatalog` is a fyagent–private field whose SSOT is the DB. Live's
    // `config.toml` only carries a lossy projection (`model_catalog_json` →
    // generated catalog file) that proxy takeover/restore cycles and Codex.app
    // config rewrites can drop, so `read_live_settings` may reconstruct it as
    // absent. Never let a switch-away backfill from Live erase the stored
    // mapping: prefer the DB provider's `modelCatalog`, falling back to whatever
    // Live reconstructed only when the DB has none.
    if let Some(stored_catalog) = provider.settings_config.get("modelCatalog") {
        if let Some(obj) = settings.as_object_mut() {
            obj.insert("modelCatalog".to_string(), stored_catalog.clone());
        }
    }

    settings
}

/// 回填（backfill）托管 Codex 官方 provider 时，剥离 live 的 `auth`。
///
/// 托管 provider 的存储配置**永远不应**持久化真实 OAuth token：token 由
/// `CodexOAuthManager` 按账号集中保管，provider 配置只保留绑定与占位 auth。
///
/// 因此无论 live 里当前是什么（我们写入的托管 auth、被 CLI 轮换过的 token、
/// 还是用户自己浏览器登录的原生 auth，后者含真实 access/refresh_token），
/// 都统一替换为 provider 存储的占位 auth，避免把真实凭据回填进 DB 配置。
fn strip_codex_managed_oauth_auth_for_backfill(provider: &Provider, settings: &mut Value) {
    let is_managed = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.managed_account_id_for("codex_oauth"))
        .is_some();
    if !is_managed {
        return;
    }

    // live 里没有 auth 就无需处理。
    if settings.get("auth").is_none() {
        return;
    }

    let stored_auth = provider
        .settings_config
        .get("auth")
        .filter(|auth| auth.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("auth".to_string(), stored_auth);
    }
}

pub(crate) fn normalize_provider_common_config_for_storage(
    db: &Database,
    app_type: &AppType,
    provider: &mut Provider,
) -> Result<(), AppError> {
    if matches!(app_type, AppType::Codex)
        && crate::proxy::providers::is_codex_official_provider(provider)
    {
        crate::codex_config::strip_codex_unified_session_bucket_from_settings(
            &mut provider.settings_config,
        )?;
    }
    let uses_common_config = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.common_config_enabled)
        .unwrap_or(false);

    if !uses_common_config {
        return Ok(());
    }

    let Some(snippet) = db.get_config_snippet(app_type.as_str())? else {
        return Ok(());
    };

    if snippet.trim().is_empty() {
        return Ok(());
    }

    match remove_common_config_from_settings(app_type, &provider.settings_config, &snippet) {
        Ok(settings) => provider.settings_config = settings,
        Err(err) => {
            log::warn!(
                "Failed to normalize common config before saving {} provider '{}': {err}",
                app_type.as_str(),
                provider.id
            );
        }
    }

    Ok(())
}

fn write_codex_live_snapshot(
    provider: &Provider,
    common_snippet: Option<&str>,
) -> Result<(), AppError> {
    let obj = provider
        .settings_config
        .as_object()
        .ok_or_else(|| AppError::Config("Codex 供应商配置必须是 JSON 对象".to_string()))?;
    let auth = obj
        .get("auth")
        .ok_or_else(|| AppError::Config("Codex 供应商配置缺少 'auth' 字段".to_string()))?;
    let config_str = obj.get("config").and_then(|v| v.as_str());
    let category = if crate::proxy::providers::is_codex_official_provider(provider) {
        Some("official")
    } else {
        provider.category.as_deref()
    };
    let profile = crate::proxy::providers::resolve_codex_catalog_tool_profile(provider);
    crate::codex_config::write_codex_provider_live_with_common_snippet(
        &provider.settings_config,
        category,
        auth,
        config_str,
        profile,
        common_snippet,
    )
}

/// Write live configuration snapshot for a provider
pub(crate) fn write_live_snapshot(app_type: &AppType, provider: &Provider) -> Result<(), AppError> {
    match app_type {
        AppType::Claude => {
            return Err(AppError::localized(
                "claude.live.requires_engine",
                "Claude Code 配置只能经关键字段写入流程写入",
                "Claude Code configuration must be written through the key-field write flow",
            ));
        }
        AppType::ClaudeDesktop => {
            return Err(AppError::localized(
                "claude_desktop.live.requires_db_context",
                "Claude Desktop 配置写入需要通过供应商切换流程执行",
                "Claude Desktop configuration must be written through the provider switch flow",
            ));
        }
        AppType::Codex => write_codex_live_snapshot(provider, None)?,
        AppType::Gemini => {
            return Err(AppError::localized(
                "gemini.live.requires_engine",
                "Gemini CLI 配置只能经关键字段写入流程写入",
                "Gemini CLI configuration must be written through the key-field write flow",
            ));
        }
        AppType::GrokBuild => {
            return Err(AppError::localized(
                "grokbuild.live.requires_engine",
                "Grok Build 配置只能经关键字段写入流程写入",
                "Grok Build configuration must be written through the key-field write flow",
            ));
        }
        AppType::OpenCode => {
            // OpenCode uses additive mode - write provider to config
            use crate::opencode_config;

            if provider.uses_subscription_proxy()
                || crate::services::opencode_models::is_managed_proxy_provider_id(&provider.id)
            {
                return Err(AppError::Message(
                    "OpenCode subscription configuration requires the managed proxy flow".into(),
                ));
            }

            // Defensive check: if settings_config is a full config structure, extract provider fragment
            let config_to_write = if let Some(obj) = provider.settings_config.as_object() {
                // Detect full config structure (has $schema or top-level provider field)
                if obj.contains_key("$schema") || obj.contains_key("provider") {
                    log::warn!(
                        "OpenCode provider '{}' has full config structure in settings_config, attempting to extract fragment",
                        provider.id
                    );
                    // Try to extract from provider.{id}
                    obj.get("provider")
                        .and_then(|p| p.get(&provider.id))
                        .cloned()
                        .ok_or_else(|| {
                            AppError::Message("OpenCode provider fragment is missing".into())
                        })?
                } else {
                    provider.settings_config.clone()
                }
            } else {
                provider.settings_config.clone()
            };

            // Preserve the original provider object on write.
            // Re-serializing typed config can drop unknown nested fields or nulls.
            if !config_to_write.is_object() {
                return Err(AppError::Message(
                    "OpenCode provider must be an object".into(),
                ));
            }
            let (_, format) = opencode_config::provider_fragment(
                &provider.id,
                &provider.settings_config,
                provider.opencode_config_format(),
            )?;
            opencode_config::set_provider_with_format(&provider.id, config_to_write, format)?;
        }
        AppType::OpenClaw => {
            // OpenClaw uses additive mode - write provider to config
            use crate::openclaw_config;

            if !provider.settings_config.is_object() {
                return Err(AppError::Message(
                    "OpenClaw provider must be an object".into(),
                ));
            }
            openclaw_config::set_provider(&provider.id, provider.settings_config.clone())?;
        }

        AppType::Hermes => {
            crate::hermes_config::set_provider(&provider.id, provider.settings_config.clone())?;
            log::debug!("Hermes provider '{}' written to live config", provider.id);
        }
    }
    Ok(())
}

/// Sync all providers to live configuration (for additive mode apps)
///
/// Writes all providers from the database to the live configuration file.
/// Used for OpenCode and other additive mode applications.
fn sync_all_providers_to_live(state: &AppState, app_type: &AppType) -> Result<(), AppError> {
    let providers = state.db.get_all_providers(app_type.as_str())?;
    let mut synced_count = 0usize;

    for provider in providers.values() {
        if provider
            .meta
            .as_ref()
            .and_then(|meta| meta.live_config_managed)
            == Some(false)
        {
            continue;
        }

        if let Err(e) = write_live_with_common_config(state.db.as_ref(), app_type, provider) {
            log::warn!(
                "Failed to sync {:?} provider '{}' to live: {e}",
                app_type,
                provider.id
            );
            continue;
        }
        synced_count += 1;
    }

    log::info!("Synced {synced_count} {app_type:?} providers to live config");
    Ok(())
}

/// 把累加式应用的全部供应商同步到 live，再重投影它的 MCP。
pub(crate) fn sync_additive_app_to_live(
    state: &AppState,
    app_type: &AppType,
) -> Result<(), AppError> {
    if app_type.is_additive_mode() {
        sync_all_providers_to_live(state, app_type)?;
    } else {
        let current_id = match crate::settings::get_effective_current_provider(&state.db, app_type)?
        {
            Some(id) => id,
            None => return Ok(()),
        };

        let providers = state.db.get_all_providers(app_type.as_str())?;
        if let Some(provider) = providers.get(&current_id) {
            write_live_with_common_config(state.db.as_ref(), app_type, provider)?;
        }
    }

    // 本函数语义是"把这个应用同步到 live"，MCP 重投影也只针对该应用；
    // 全量 sync_all_enabled 会把无关应用的 live 损坏牵连进来。投影失败
    // 上抛（不降级）：这里没有已变更的 DB 状态需要保护，调用方重试即可。
    McpService::sync_enabled_for_app_inner(state, app_type)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveSyncOutcome {
    /// 按直连投影写了 live。
    WroteLive,
    /// 应用在代理模式：live 是代理契约，没有按直连写。
    ProxyMode,
}

/// 把 `provider` 同步到 live，按应用的模式处理：
/// - 直连模式：按直连投影写 live；
/// - 代理模式：live 是代理契约。`provider` 是代理路由的那家、或在 Stack 名单里时按新契约
///   重写（契约没变就不动）；其余供应商（包括直连指针那家）只在退出代理时写回，这里不碰
///   live。
///
/// A backup row by itself is not evidence: stale rows survive crashes and failed
/// restores. Likewise, an enabled flag left behind by an interrupted teardown
/// must not suppress a real live write unless there is corroborating evidence.
fn proxy_owns_live_config(
    state: &AppState,
    app_type: &AppType,
    has_live_backup: bool,
    live_taken_over: bool,
) -> bool {
    if live_taken_over {
        return true;
    }

    let takeover_enabled =
        match futures::executor::block_on(state.db.get_proxy_config_for_app(app_type.as_str())) {
            Ok(config) => config.enabled,
            Err(err) => {
                log::warn!(
                    "读取 {} 代理接管标志失败，按未接管处理并继续写入 live 配置；\
                     若该应用此刻确实处于接管状态，本次写入会覆盖接管的 live: {err}",
                    app_type.as_str()
                );
                false
            }
        };

    // Provider updates may already hold this app's switch lock. Occupancy is not
    // evidence of takeover here: a stale backup must still allow a live write.
    takeover_enabled
        && has_live_backup
        && futures::executor::block_on(state.proxy_service.is_running())
}

/// Sync a provider to live while respecting proxy takeover ownership.
pub(crate) fn sync_live_for_provider_respecting_takeover(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
) -> Result<LiveSyncOutcome, AppError> {
    if crate::mode::current::is_proxy(app_type) {
        return sync_live_for_provider_respecting_mode(state, app_type, provider, None);
    }
    let has_live_backup =
        match futures::executor::block_on(state.db.get_live_backup(app_type.as_str())) {
            Ok(backup) => backup.is_some(),
            Err(err) => {
                log::warn!(
                    "读取 {} Live 备份失败，按无备份处理并继续写入 live 配置: {err}",
                    app_type.as_str()
                );
                false
            }
        };
    let live_taken_over = state
        .proxy_service
        .detect_takeover_in_live_config_for_app(app_type);

    if !proxy_owns_live_config(state, app_type, has_live_backup, live_taken_over) {
        // A stale backup must follow the provider too, otherwise a later restore
        // can resurrect the old URL and undo the live write we are making now.
        if has_live_backup {
            if let Err(err) = futures::executor::block_on(
                state.proxy_service.update_live_backup_from_provider_inner(
                    app_type.as_str(),
                    provider,
                    None,
                ),
            ) {
                log::warn!(
                    "刷新 {} 残留 Live 备份失败（不影响本次 live 写入）: {err}",
                    app_type.as_str()
                );
            }
        }
        write_live_with_common_config(state.db.as_ref(), app_type, provider)?;
        return Ok(LiveSyncOutcome::WroteLive);
    }

    // Takeover owns live: update the restore source, and refresh proxy-safe
    // projections while the proxy is running.
    futures::executor::block_on(state.proxy_service.update_live_backup_from_provider_inner(
        app_type.as_str(),
        provider,
        None,
    ))
    .map_err(|e| AppError::Message(format!("更新 Live 备份失败: {e}")))?;

    if !futures::executor::block_on(state.proxy_service.is_running()) {
        return Ok(LiveSyncOutcome::ProxyMode);
    }

    match app_type {
        AppType::Claude => futures::executor::block_on(
            state
                .proxy_service
                .sync_claude_live_from_provider_while_proxy_active(provider),
        )
        .map_err(|e| AppError::Message(format!("同步 Claude Live 配置失败: {e}")))?,
        AppType::Codex if live_taken_over => futures::executor::block_on(
            state
                .proxy_service
                .sync_codex_live_from_provider_while_proxy_active(provider),
        )
        .map_err(|e| AppError::Message(format!("同步 Codex Live 配置失败: {e}")))?,
        AppType::GrokBuild if live_taken_over => futures::executor::block_on(
            state
                .proxy_service
                .sync_grok_live_from_provider_while_proxy_active(provider),
        )
        .map_err(|e| AppError::Message(format!("同步 Grok Build Live 配置失败: {e}")))?,
        _ => {}
    }

    Ok(LiveSyncOutcome::ProxyMode)
}

/// 把正在用的那家（代理模式下是代理路由）同步到 live；没有正在用的那家时返回 `None`。
/// 返回时已经放开切换锁。
pub(crate) fn sync_current_provider_for_app_respecting_mode(
    state: &AppState,
    app_type: &AppType,
) -> Result<Option<LiveSyncOutcome>, AppError> {
    let _switch_guard = crate::mode::controller::lock_settled_blocking(state, app_type)?;
    let current_id = match crate::mode::current::provider_for(
        &state.db,
        app_type,
        crate::mode::current::Purpose::InUse,
    )? {
        Some(id) => id,
        None => return Ok(None),
    };

    let providers = state.db.get_all_providers(app_type.as_str())?;
    let Some(provider) = providers.get(&current_id) else {
        return Ok(None);
    };

    sync_live_for_provider_respecting_mode(state, app_type, provider, None).map(Some)
}

/// Sync current provider to live configuration
///
/// 使用有效的当前供应商 ID（验证过存在性）。
/// 优先从本地 settings 读取，验证后 fallback 到数据库的 is_current 字段。
/// 这确保了配置导入后无效 ID 会自动 fallback 到数据库。
///
/// For additive mode apps (OpenCode), all providers are synced instead of just the current one.
pub fn sync_current_to_live(state: &AppState) -> Result<(), AppError> {
    let mut failures = Vec::new();

    // Sync providers based on mode
    for app_type in AppType::all() {
        let result = if app_type.is_additive_mode() {
            // Additive mode: sync ALL providers
            sync_all_providers_to_live(state, &app_type)
        } else {
            // Switch mode: sync only current provider. During proxy takeover,
            // update the restore backup instead of rewriting the taken-over
            // live file.
            sync_current_provider_for_app_respecting_takeover(state, &app_type)
        };

        if let Err(error) = result {
            log::warn!("同步 Provider 到 {app_type:?} 失败: {error}");
            failures.push(format!("provider/{}: {error}", app_type.as_str()));
        }
    }

    // MCP sync（best-effort 逐应用投影，内部已聚合失败）。错误暂存到
    // Skill 同步之后再返回：MCP 的失败不该跳过 Skill 同步，但调用方
    //（配置导入 / 云同步恢复）需要知道结果不完整。
    if let Err(error) = McpService::sync_all_enabled_inner(state) {
        failures.push(format!("mcp: {error}"));
    }

    // Skill sync
    for app_type in AppType::all() {
        if let Err(e) = crate::services::skill::SkillService::sync_to_app(&state.db, &app_type) {
            log::warn!("同步 Skill 到 {app_type:?} 失败: {e}");
            failures.push(format!("skill/{}: {e}", app_type.as_str()));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "部分 live 配置同步失败: {}",
            failures.join("; ")
        )))
    }
}

/// Read current live settings for an app type
pub fn read_live_settings(app_type: AppType) -> Result<Value, AppError> {
    match app_type {
        AppType::Codex => {
            let mut result = crate::codex_config::read_codex_live_settings()?;
            // `modelCatalog` is a fyagent private field that lives only in
            // the DB SSOT plus the `fyagent-model-catalog.json` projection
            // file — it is never inlined into `auth.json` or `config.toml`.
            // Reverse-parse the projection so the edit form for the active
            // Codex provider doesn't see an empty mapping table.
            if let Ok(Some(model_catalog)) =
                crate::codex_config::read_codex_model_catalog_simplified_from_live()
            {
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("modelCatalog".to_string(), model_catalog);
                }
            }
            Ok(result)
        }
        AppType::Claude => {
            let path = get_claude_settings_path();
            if !path.exists() {
                return Err(AppError::localized(
                    "claude.live.missing",
                    "Claude Code 配置文件不存在",
                    "Claude settings file is missing",
                ));
            }
            read_json_file(&path)
        }
        AppType::ClaudeDesktop => Err(AppError::localized(
            "claude_desktop.live.read_unsupported",
            "Claude Desktop 3P 配置不支持作为通用 live 配置导入，请使用“从 Claude 导入兼容供应商”。",
            "Claude Desktop 3P configuration cannot be imported as a generic live config. Use 'Import compatible providers from Claude' instead.",
        )),
        AppType::Gemini => {
            use crate::gemini_config::{
                env_to_json, get_gemini_env_path, get_gemini_settings_path, read_gemini_env,
            };

            // Read .env file (environment variables)
            let env_path = get_gemini_env_path();
            if !env_path.exists() {
                return Err(AppError::localized(
                    "gemini.env.missing",
                    "Gemini .env 文件不存在",
                    "Gemini .env file not found",
                ));
            }

            let env_map = read_gemini_env()?;
            let env_json = env_to_json(&env_map);
            let env_obj = env_json.get("env").cloned().unwrap_or_else(|| json!({}));

            // Read settings.json file (MCP config etc.)
            let settings_path = get_gemini_settings_path();
            let config_obj = if settings_path.exists() {
                read_json_file(&settings_path)?
            } else {
                json!({})
            };

            // Return complete structure: { "env": {...}, "config": {...} }
            Ok(json!({
                "env": env_obj,
                "config": config_obj
            }))
        }
        AppType::OpenCode => {
            use crate::opencode_config::{get_opencode_config_path, read_opencode_config_from_path};

            let config_path = get_opencode_config_path();
            if !config_path.try_exists().map_err(|e| AppError::io(&config_path, e))? {
                return Err(AppError::localized(
                    "opencode.config.missing",
                    "OpenCode 配置文件不存在",
                    "OpenCode configuration file not found",
                ));
            }

            let config = read_opencode_config_from_path(&config_path)?;
            Ok(config)
        }
        AppType::GrokBuild => crate::grok_config::read_grok_live_settings(),
        AppType::OpenClaw => {
            use crate::openclaw_config::{get_openclaw_config_path, read_openclaw_config};

            let config_path = get_openclaw_config_path();
            if !config_path.exists() {
                return Err(AppError::localized(
                    "openclaw.config.missing",
                    "OpenClaw 配置文件不存在",
                    "OpenClaw configuration file not found",
                ));
            }

            let config = read_openclaw_config()?;
            Ok(config)
        }
        AppType::Hermes => {
            let config_path = crate::hermes_config::get_hermes_config_path();
            if !config_path.exists() {
                return Err(AppError::localized(
                    "hermes.config.missing",
                    "Hermes 配置文件不存在",
                    "Hermes configuration file not found",
                ));
            }
            let yaml_config = crate::hermes_config::read_hermes_config()?;
            let config = crate::hermes_config::yaml_to_json(&yaml_config)?;
            Ok(config)
        }
    }
}

/// Import default configuration from live files
///
/// Returns `Ok(true)` if a provider was actually imported,
/// `Ok(false)` if skipped (providers already exist for this app).
pub fn import_default_config(state: &AppState, app_type: AppType) -> Result<bool, AppError> {
    // Additive mode apps (OpenCode, OpenClaw) should use their dedicated
    // import_xxx_providers_from_live functions, not this generic default config import
    if app_type.is_additive_mode() {
        return Ok(false);
    }

    // 允许 "只有官方 seed 预设" 的情况下继续导入 live：
    // - 启动编排顺序是先 import 后 seed，新用户启动时 providers 为空，导入照常
    // - 老用户已有非 seed provider，跳过导入（正确）
    // - 用户手动点 ProviderEmptyState 的导入按钮时，与官方 seed 共存而不被阻塞
    if state.db.has_non_official_seed_provider(app_type.as_str())? {
        return Ok(false);
    }

    // 拒绝把"代理模式下的 Live"导入为供应商：代理模式下 Live 里只有
    // PROXY_MANAGED 占位符和本地代理地址，不是用户的真实配置。一旦导入，
    // 它会成为直连指针（SSOT），退出代理时会把占位符当真实配置写回 Live。
    // 典型触发场景：代理模式下切换 app_config_dir 并重启，新数据库首启导入。
    if state.proxy_service.live_has_proxy_placeholder(&app_type) {
        return Err(AppError::localized(
            "provider.import.live_taken_over",
            "Live 配置当前处于代理接管状态（包含占位符），不能导入为供应商。请先关闭代理接管或恢复 Live 配置后重试。",
            "The live config is currently taken over by the proxy (contains placeholders) and cannot be imported as a provider. Disable proxy takeover or restore the live config first.",
        ));
    }

    let settings_config = match app_type {
        AppType::Codex => crate::codex_config::read_codex_live_settings()?,
        AppType::GrokBuild => {
            let mut settings = crate::grok_config::read_grok_live_settings()?;
            let config = settings
                .get("config")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // 官方登录态（无自定义模型表）在这里必须报错：本函数也被启动
            // 自动导入调用，而全项目惯例是"启动自动导入只产出 default，
            // 从不产出官方条目"——否则删掉的官方条目每次重启都会复活。
            // 官方态的成功导入（补官方条目并激活）只挂在手动导入的命令层
            // （`import_default_config_internal`）。
            crate::grok_config::validate_config_toml(config)?;
            crate::grok_config::strip_grok_mcp_servers_from_settings(&mut settings)?;
            settings
        }
        AppType::Claude => {
            let settings_path = get_claude_settings_path();
            if !settings_path.exists() {
                return Err(AppError::localized(
                    "claude.live.missing",
                    "Claude Code 配置文件不存在",
                    "Claude settings file is missing",
                ));
            }
            let mut v = read_json_file::<Value>(&settings_path)?;
            let _ = normalize_claude_models_in_value(&mut v);
            v
        }
        AppType::ClaudeDesktop => {
            return Err(AppError::localized(
                "claude_desktop.import_unsupported",
                "Claude Desktop 3P 配置不能通过通用导入读取，请使用“从 Claude 导入兼容供应商”。",
                "Claude Desktop 3P config cannot be imported through the generic import flow. Use 'Import compatible providers from Claude' instead.",
            ));
        }
        AppType::Gemini => {
            use crate::gemini_config::{
                env_to_json, get_gemini_env_path, get_gemini_settings_path, read_gemini_env,
            };

            // Read .env file (environment variables)
            let env_path = get_gemini_env_path();
            if !env_path.exists() {
                return Err(AppError::localized(
                    "gemini.live.missing",
                    "Gemini 配置文件不存在",
                    "Gemini configuration file is missing",
                ));
            }

            let env_map = read_gemini_env()?;
            let env_json = env_to_json(&env_map);
            let env_obj = env_json.get("env").cloned().unwrap_or_else(|| json!({}));

            // Read settings.json file (MCP config etc.)
            let settings_path = get_gemini_settings_path();
            let config_obj = if settings_path.exists() {
                read_json_file(&settings_path)?
            } else {
                json!({})
            };

            // Return complete structure: { "env": {...}, "config": {...} }
            json!({
                "env": env_obj,
                "config": config_obj
            })
        }
        // OpenCode, OpenClaw and Hermes use additive mode and are handled by early return above
        AppType::OpenCode | AppType::OpenClaw | AppType::Hermes => {
            unreachable!("additive mode apps are handled by early return")
        }
    };

    let mut provider = Provider::with_id(
        "default".to_string(),
        "default".to_string(),
        settings_config,
        None,
    );
    provider.category = Some(
        if matches!(app_type, AppType::Codex) {
            let config_text = provider
                .settings_config
                .get("config")
                .and_then(Value::as_str);
            let has_provider_key = crate::codex_config::extract_codex_api_key(
                provider.settings_config.get("auth"),
                config_text,
            )
            .is_some();
            let has_login_material = provider
                .settings_config
                .get("auth")
                .is_some_and(crate::codex_config::codex_auth_has_login_material);

            if has_login_material && !has_provider_key {
                "official"
            } else {
                "custom"
            }
        } else {
            "custom"
        }
        .to_string(),
    );

    state.db.save_provider(app_type.as_str(), &provider)?;
    state
        .db
        .set_current_provider(app_type.as_str(), &provider.id)?;
    crate::settings::set_current_provider(&app_type, Some(provider.id.as_str()))?;

    // 初次导入已有配置时随手补出官方入口，对齐其它应用"首启动 = 导入 default
    // + 播种官方条目"的观感。grokbuild 种子晚于 `official_providers_seeded`
    // flag 引入，存量库的主播种不会再跑，只能挂在导入动作上补。
    // 只在导入成功时执行；live 完全不可导入（文件缺失/语法错误/残缺配置）
    // 不会到达这里。失败只 warn。
    if matches!(app_type, AppType::GrokBuild) {
        if let Err(e) = state.db.ensure_official_seed_by_id(
            crate::database::GROKBUILD_OFFICIAL_PROVIDER_ID,
            AppType::GrokBuild,
        ) {
            log::warn!("Failed to ensure grokbuild-official seed after import: {e}");
        }
    }

    Ok(true) // 真正导入了
}

/// Decide whether startup should auto-import the current live config as `default`.
///
/// This is intentionally stricter than the manual import path:
/// if the app already has any provider row at all (including official seeds),
/// startup must skip auto-import to avoid recreating `default` on each launch.
pub fn should_import_default_config_on_startup(
    state: &AppState,
    app_type: &AppType,
) -> Result<bool, AppError> {
    if app_type.is_additive_mode() {
        return Ok(false);
    }

    Ok(!state.db.has_any_provider_for_app(app_type.as_str())?)
}

/// Remove an OpenCode provider from the live configuration
///
/// This is specific to OpenCode's additive mode - removing a provider
/// from the opencode.json file.
pub(crate) fn remove_opencode_provider_from_live(provider_id: &str) -> Result<(), AppError> {
    use crate::opencode_config;

    // Check if OpenCode config directory exists
    if !opencode_config::get_opencode_dir().exists() {
        log::debug!("OpenCode config directory doesn't exist, skipping removal of '{provider_id}'");
        return Ok(());
    }

    opencode_config::remove_provider(provider_id)?;
    log::info!("OpenCode provider '{provider_id}' removed from live config");

    Ok(())
}

/// Import all providers from OpenCode live config to database
///
/// This imports existing providers from ~/.config/opencode/opencode.json
/// into the FyAgent database. Each provider found will be added to the
/// database with is_current set to false.
pub fn import_opencode_providers_from_live(state: &AppState) -> Result<usize, AppError> {
    use crate::opencode_config;
    use crate::provider::OpenCodeConfigFormat;

    let providers = opencode_config::get_providers_with_format()?;
    if providers.is_empty() {
        return Ok(0);
    }

    let mut imported = 0;
    let mut updated = 0;
    let existing_ids = state.db.get_provider_ids("opencode")?;

    for (id, (config, format)) in providers {
        let native_format = (format == OpenCodeConfigFormat::V2).then_some(format);
        if id.trim().is_empty()
            || !config.is_object()
            || crate::services::opencode_models::is_managed_proxy_provider_id(&id)
            || config.pointer("/options/apiKey").and_then(Value::as_str) == Some("PROXY_MANAGED")
        {
            // Managed live entries are loopback projections, never saved
            // upstream definitions or new API-key providers.
            continue;
        }
        // Typed metadata is advisory; unknown vendor shapes still retain raw authority.
        let display_name =
            serde_json::from_value::<crate::provider::OpenCodeProviderConfig>(config.clone())
                .ok()
                .and_then(|projection| projection.name)
                .or_else(|| {
                    config
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
        let settings_config = config;

        if existing_ids.contains(&id) {
            match state.db.get_provider_by_id(&id, "opencode") {
                Ok(Some(existing)) => {
                    if existing.uses_subscription_proxy() {
                        continue;
                    }
                    let display_name = display_name
                        .clone()
                        .unwrap_or_else(|| existing.name.clone());
                    if existing.settings_config != settings_config
                        || existing.name != display_name
                        || existing.opencode_config_format() != native_format
                    {
                        let mut provider = existing;
                        provider.name = display_name;
                        provider.settings_config = settings_config;
                        provider
                            .meta
                            .get_or_insert_with(Default::default)
                            .opencode_config_format = native_format;
                        if let Err(e) = state.db.save_provider("opencode", &provider) {
                            log::warn!(
                                "Failed to update OpenCode provider '{id}' from live config: {e}"
                            );
                        } else {
                            updated += 1;
                            log::info!("Updated OpenCode provider '{id}' from live config");
                        }
                    }
                }
                Ok(None) => {
                    log::warn!("OpenCode provider '{id}' disappeared while importing live config")
                }
                Err(e) => log::warn!("Failed to look up OpenCode provider '{id}': {e}"),
            }
            continue;
        }

        // Create provider
        let display_name = display_name.unwrap_or_else(|| id.clone());
        let mut provider = Provider::with_id(id.clone(), display_name, settings_config, None);
        provider.meta = Some(crate::provider::ProviderMeta {
            live_config_managed: Some(true),
            opencode_config_format: native_format,
            ..Default::default()
        });

        // Save to database
        if let Err(e) = state.db.save_provider("opencode", &provider) {
            log::warn!("Failed to import OpenCode provider '{id}': {e}");
            continue;
        }

        imported += 1;
        log::info!("Imported OpenCode provider '{id}' from live config");
    }

    Ok(imported + updated)
}

/// Import all providers from OpenClaw live config to database
///
/// This imports existing providers from ~/.openclaw/openclaw.json
/// into the FyAgent database. Each provider found will be added to the
/// database with is_current set to false.
pub fn import_openclaw_providers_from_live(state: &AppState) -> Result<usize, AppError> {
    use crate::openclaw_config;

    let providers = openclaw_config::get_providers()?;
    if providers.is_empty() {
        return Ok(0);
    }

    let mut imported = 0;
    let mut updated = 0;
    let existing_ids = state.db.get_provider_ids("openclaw")?;

    for (id, config) in providers {
        if id.trim().is_empty() || !config.is_object() {
            continue;
        }
        let display_name =
            serde_json::from_value::<crate::openclaw_config::OpenClawProviderConfig>(
                config.clone(),
            )
            .ok()
            .and_then(|projection| {
                projection
                    .models
                    .first()
                    .and_then(|model| model.name.clone())
            })
            .unwrap_or_else(|| id.clone());
        let settings_config = config;

        if existing_ids.contains(&id) {
            match state.db.get_provider_by_id(&id, "openclaw") {
                Ok(Some(existing)) => {
                    if existing.settings_config != settings_config {
                        let mut provider = existing;
                        provider.settings_config = settings_config;
                        if let Err(e) = state.db.save_provider("openclaw", &provider) {
                            log::warn!(
                                "Failed to update OpenClaw provider '{id}' from live config: {e}"
                            );
                        } else {
                            updated += 1;
                            log::info!("Updated OpenClaw provider '{id}' from live config");
                        }
                    }
                }
                Ok(None) => {
                    log::warn!("OpenClaw provider '{id}' disappeared while importing live config")
                }
                Err(e) => log::warn!("Failed to look up OpenClaw provider '{id}': {e}"),
            }
            continue;
        }

        // Create provider
        let mut provider = Provider::with_id(id.clone(), display_name, settings_config, None);
        provider.meta = Some(crate::provider::ProviderMeta {
            live_config_managed: Some(true),
            ..Default::default()
        });

        // Save to database
        if let Err(e) = state.db.save_provider("openclaw", &provider) {
            log::warn!("Failed to import OpenClaw provider '{id}': {e}");
            continue;
        }

        imported += 1;
        log::info!("Imported OpenClaw provider '{id}' from live config");
    }

    Ok(imported + updated)
}

/// Import all providers from Hermes live config to database
///
/// This imports existing providers from ~/.hermes/config.yaml
/// into the FyAgent database. Each provider found will be added to the
/// database with is_current set to false.
pub fn import_hermes_providers_from_live(state: &AppState) -> Result<usize, AppError> {
    use crate::hermes_config;

    let providers = hermes_config::get_providers()?;
    if providers.is_empty() {
        return Ok(0);
    }

    let mut imported = 0;
    let mut updated = 0;
    let existing_ids = state.db.get_provider_ids("hermes")?;

    for (name, config) in providers {
        // Validate: skip entries with empty name
        if name.trim().is_empty() {
            log::warn!("Skipping Hermes provider with empty name");
            continue;
        }

        if existing_ids.contains(&name) {
            match state.db.get_provider_by_id(&name, "hermes") {
                Ok(Some(existing)) => {
                    if existing.settings_config != config {
                        let mut provider = existing;
                        provider.settings_config = config;
                        if let Err(e) = state.db.save_provider("hermes", &provider) {
                            log::warn!(
                                "Failed to update Hermes provider '{name}' from live config: {e}"
                            );
                        } else {
                            updated += 1;
                            log::info!("Updated Hermes provider '{name}' from live config");
                        }
                    }
                }
                Ok(None) => {
                    log::warn!("Hermes provider '{name}' disappeared while importing live config")
                }
                Err(e) => log::warn!("Failed to look up Hermes provider '{name}': {e}"),
            }
            continue;
        }

        // Create provider
        let mut provider = Provider::with_id(name.clone(), name.clone(), config, None);
        provider.meta = Some(crate::provider::ProviderMeta {
            live_config_managed: Some(true),
            ..Default::default()
        });

        // Save to database
        if let Err(e) = state.db.save_provider("hermes", &provider) {
            log::warn!("Failed to import Hermes provider '{name}': {e}");
            continue;
        }

        imported += 1;
        log::info!("Imported Hermes provider '{name}' from live config");
    }

    Ok(imported + updated)
}

/// Remove a Hermes provider from live config
///
/// This removes a specific provider from ~/.hermes/config.yaml
/// without affecting other providers in the file.
pub fn remove_hermes_provider_from_live(provider_id: &str) -> Result<(), AppError> {
    use crate::hermes_config;

    // Check if Hermes config directory exists
    if !hermes_config::get_hermes_dir().exists() {
        log::debug!("Hermes config directory doesn't exist, skipping removal of '{provider_id}'");
        return Ok(());
    }

    hermes_config::remove_provider(provider_id)?;
    log::info!("Hermes provider '{provider_id}' removed from live config");

    Ok(())
}

/// Remove an OpenClaw provider from live config
///
/// This removes a specific provider from ~/.openclaw/openclaw.json
/// without affecting other providers in the file.
pub fn remove_openclaw_provider_from_live(provider_id: &str) -> Result<(), AppError> {
    use crate::openclaw_config;

    // Check if OpenClaw config directory exists
    if !openclaw_config::get_openclaw_dir().exists() {
        log::debug!("OpenClaw config directory doesn't exist, skipping removal of '{provider_id}'");
        return Ok(());
    }

    openclaw_config::remove_provider(provider_id)?;
    log::info!("OpenClaw provider '{provider_id}' removed from live config");

    Ok(())
}

pub(crate) fn write_live_for_state(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
) -> Result<(), AppError> {
    let db = state.db.as_ref();
    if matches!(app_type, AppType::Claude) {
        // Claude 不再整份写，也不合并片段：只替换关键字段和独有字段。live 当前对应的
        // 就是这个供应商（同步、退出代理写回），它带进来的独有字段按同一行比对。
        super::claude_direct::reapply(db, Some(provider), provider)?;
        return Ok(());
    }
    if matches!(app_type, AppType::Codex) {
        // Codex 同理：只替换关键字段和独有字段，不合并片段、不补回 MCP。
        super::codex_direct::write_direct(
            db,
            &state.codex_oauth_manager,
            crate::mode::state::op::APPLY,
            super::codex_direct::Owner::Provider(provider),
            Some(provider),
            crate::mode::state::PendingTarget::default(),
        )?;
        return Ok(());
    }
    if matches!(app_type, AppType::Gemini) {
        // Gemini、Grok Build 同理：只替换关键字段，不合并片段、不补回 MCP。
        super::gemini_direct::reapply(db, provider)?;
        return Ok(());
    }
    if matches!(app_type, AppType::GrokBuild) {
        super::grok_direct::reapply(db, Some(provider), provider)?;
        return Ok(());
    }

    if matches!(app_type, AppType::ClaudeDesktop) {
        crate::claude_desktop_config::apply_provider(db, provider)?;
        log::info!(
            "Claude Desktop 3P profile '{}' written for provider '{}'",
            crate::claude_desktop_config::PROFILE_ID,
            provider.id
        );
        return Ok(());
    }

    write_live_snapshot(app_type, provider)
}

pub(crate) fn sync_live_for_provider_respecting_mode(
    state: &AppState,
    app_type: &AppType,
    provider: &Provider,
    prev: Option<&Provider>,
) -> Result<LiveSyncOutcome, AppError> {
    if crate::mode::current::is_proxy(app_type) {
        futures::executor::block_on(crate::mode::controller::resync_saved_row_locked(
            state, app_type, provider,
        ))
        .map_err(AppError::Message)?;
        return Ok(LiveSyncOutcome::ProxyMode);
    }
    if matches!(app_type, AppType::Claude) {
        super::claude_direct::reapply(state.db.as_ref(), prev.or(Some(provider)), provider)?;
    } else if matches!(app_type, AppType::GrokBuild) {
        // 编辑前的行用来推断旧版写的表（还没有写入记录时），改了表名也能删掉旧表。
        super::grok_direct::reapply(state.db.as_ref(), prev.or(Some(provider)), provider)?;
    } else {
        write_live_for_state(state, app_type, provider)?;
    }
    Ok(LiveSyncOutcome::WroteLive)
}

pub(crate) fn sync_current_provider_for_app_to_live(
    state: &AppState,
    app_type: &AppType,
) -> Result<(), AppError> {
    if app_type.is_additive_mode() {
        return sync_additive_app_to_live(state, app_type);
    } else {
        let current_id = match crate::settings::get_effective_current_provider(&state.db, app_type)?
        {
            Some(id) => id,
            None => return Ok(()),
        };

        let providers = state.db.get_all_providers(app_type.as_str())?;
        if let Some(provider) = providers.get(&current_id) {
            write_live_with_common_config(state.db.as_ref(), app_type, provider)?;
        }
    }

    // 本函数语义是"把这个应用同步到 live"，MCP 重投影也只针对该应用；
    // 全量 sync_all_enabled 会把无关应用的 live 损坏牵连进来。投影失败
    // 上抛（不降级）：这里没有已变更的 DB 状态需要保护，调用方重试即可。
    McpService::sync_enabled_for_app_inner(state, app_type)?;

    Ok(())
}

fn provider_env_targets_gpt56(provider_env: Option<&serde_json::Map<String, Value>>) -> bool {
    let Some(env) = provider_env else {
        return false;
    };
    let mut saw_model = false;
    for key in CODEX_OAUTH_MODEL_ENV_KEYS {
        let Some(value) = env.get(key) else {
            continue;
        };
        let Some(model) = value.as_str() else {
            return false;
        };
        let model = model.trim();
        if model.is_empty() {
            continue;
        }
        saw_model = true;
        if !model.to_ascii_lowercase().starts_with("gpt-5.6") {
            return false;
        }
    }
    saw_model
}

fn is_kimi_for_coding_provider(provider: &Provider) -> bool {
    provider
        .settings_config
        .pointer("/env/ANTHROPIC_BASE_URL")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(|url| url.trim_end_matches('/'))
        == Some("https://api.kimi.com/coding")
}

fn apply_codex_oauth_claude_context_defaults(settings: &mut Value, provider: &Provider) {
    if !provider.is_codex_oauth() {
        return;
    }

    // Read provider-owned values before mutably borrowing the effective
    // settings. This also deliberately prevents a legacy common-config
    // snippet from overriding model-specific context limits.
    let provider_env = provider
        .settings_config
        .get("env")
        .and_then(Value::as_object);
    let Some(root) = settings.as_object_mut() else {
        return;
    };
    let env = root.entry("env".to_string()).or_insert_with(|| json!({}));
    let Some(env) = env.as_object_mut() else {
        log::warn!(
            "Cannot apply Codex OAuth Claude context defaults for '{}': env is not an object",
            provider.id
        );
        return;
    };

    let inject_defaults = provider_env_targets_gpt56(provider_env);
    for (key, default_value) in [
        (
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
            CODEX_OAUTH_CLAUDE_MAX_CONTEXT_TOKENS,
        ),
        (
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            CODEX_OAUTH_CLAUDE_AUTO_COMPACT_WINDOW,
        ),
    ] {
        match provider_env.and_then(|provider_env| provider_env.get(key)) {
            Some(value) => {
                env.insert(key.to_string(), value.clone());
            }
            None if inject_defaults => {
                env.insert(key.to_string(), Value::String(default_value.to_string()));
            }
            // 老模型不注入默认值，同时剥掉遗留共享片段可能带进来的值
            None => {
                env.remove(key);
            }
        }
    }
}

fn apply_kimi_for_coding_context_defaults(settings: &mut Value, provider: &Provider) {
    if !is_kimi_for_coding_provider(provider) {
        return;
    }

    let provider_env = provider
        .settings_config
        .get("env")
        .and_then(Value::as_object);
    let Some(env) = settings.get_mut("env").and_then(Value::as_object_mut) else {
        return;
    };

    for key in [
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
    ] {
        let value = provider_env
            .and_then(|provider_env| provider_env.get(key))
            .cloned()
            .unwrap_or_else(|| Value::String(KIMI_FOR_CODING_CONTEXT_TOKENS.to_string()));
        env.insert(key.to_string(), value);
    }
}

pub(crate) fn sanitize_claude_settings_for_live(settings: &Value) -> Value {
    let mut v = settings.clone();
    if let Some(obj) = v.as_object_mut() {
        // Internal-only fields - never write to Claude Code settings.json
        obj.remove("api_format");
        obj.remove("apiFormat");
        obj.remove("openrouter_compat_mode");
        obj.remove("openrouterCompatMode");
    }
    v
}

fn json_deep_merge(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, source_value) in source_map {
                match target_map.get_mut(key) {
                    Some(target_value) => json_deep_merge(target_value, source_value),
                    None => {
                        target_map.insert(key.clone(), source_value.clone());
                    }
                }
            }
        }
        (target_value, source_value) => {
            *target_value = source_value.clone();
        }
    }
}

fn merge_toml_item(target: &mut Item, source: &Item) {
    if let Some(source_table) = source.as_table_like() {
        if let Some(target_table) = target.as_table_like_mut() {
            merge_toml_table_like(target_table, source_table);
            return;
        }
    }

    *target = source.clone();
}

fn merge_toml_table_like(target: &mut dyn TableLike, source: &dyn TableLike) {
    for (key, source_item) in source.iter() {
        match target.get_mut(key) {
            Some(target_item) => merge_toml_item(target_item, source_item),
            None => {
                target.insert(key, source_item.clone());
            }
        }
    }
}

fn sync_current_provider_for_app_respecting_takeover(
    state: &AppState,
    app_type: &AppType,
) -> Result<(), AppError> {
    let current_id = match crate::settings::get_effective_current_provider(&state.db, app_type)? {
        Some(id) => id,
        None => return Ok(()),
    };

    let providers = state.db.get_all_providers(app_type.as_str())?;
    let Some(provider) = providers.get(&current_id) else {
        return Ok(());
    };

    sync_live_for_provider_respecting_takeover(state, app_type, provider).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AuthBinding, AuthBindingSource, ProviderMeta};
    use serde_json::json;

    #[test]
    fn kimi_for_coding_effective_settings_backfill_256k_context() {
        let db = Database::memory().expect("create memory db");
        let provider = Provider::with_id(
            "kimi-for-coding".to_string(),
            "Kimi For Coding".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
                    "ANTHROPIC_MODEL": "kimi-for-coding",
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "262144"
                }
            }),
            None,
        );

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        assert_eq!(
            effective["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("262144")
        );
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("262144")
        );
    }

    #[test]
    fn kimi_for_coding_context_defaults_preserve_user_overrides() {
        let db = Database::memory().expect("create memory db");
        let provider = Provider::with_id(
            "kimi-for-coding".to_string(),
            "Kimi For Coding".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding",
                    "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "300000",
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "250000"
                }
            }),
            None,
        );

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        assert_eq!(
            effective["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("300000")
        );
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("250000")
        );
    }

    #[test]
    fn kimi_for_coding_backfill_strips_only_injected_context_default() {
        let db = Database::memory().expect("create memory db");
        let provider = Provider::with_id(
            "kimi-for-coding".to_string(),
            "Kimi For Coding".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "262144"
                }
            }),
            None,
        );

        let live = build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
            .expect("build effective settings");
        let backfilled =
            strip_common_config_from_live_settings(&db, &AppType::Claude, &provider, live);
        assert!(backfilled["env"]
            .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
            .is_none());
        assert_eq!(
            backfilled["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("262144")
        );
    }

    #[test]
    fn codex_oauth_effective_settings_backfill_gpt_context_defaults() {
        let db = Database::memory().expect("create memory db");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_MODEL": "gpt-5.6"
                }
            }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            ..Default::default()
        });

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        assert_eq!(
            effective["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("372000")
        );
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("372000")
        );
    }

    #[test]
    fn codex_oauth_context_defaults_preserve_user_overrides() {
        let db = Database::memory().expect("create memory db");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({
                "env": {
                    "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "500000",
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "350000"
                }
            }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            ..Default::default()
        });

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        assert_eq!(
            effective["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("500000")
        );
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("350000")
        );
    }

    #[test]
    fn codex_oauth_context_defaults_ignore_legacy_common_config_values() {
        let db = Database::memory().expect("create memory db");
        db.set_config_snippet(
            AppType::Claude.as_str(),
            Some(
                json!({
                    "env": {
                        "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "262144",
                        "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "262144"
                    }
                })
                .to_string(),
            ),
        )
        .expect("save legacy common config");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({ "env": { "ANTHROPIC_MODEL": "gpt-5.6" } }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            common_config_enabled: Some(true),
            ..Default::default()
        });

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        assert_eq!(
            effective["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("372000")
        );
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("372000")
        );
    }

    #[test]
    fn codex_oauth_context_defaults_skip_non_gpt56_models() {
        let db = Database::memory().expect("create memory db");
        db.set_config_snippet(
            AppType::Claude.as_str(),
            Some(
                json!({
                    "env": {
                        "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "262144",
                        "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "262144"
                    }
                })
                .to_string(),
            ),
        )
        .expect("save legacy common config");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_MODEL": "gpt-5.5",
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "300000"
                }
            }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            common_config_enabled: Some(true),
            ..Default::default()
        });

        let effective =
            build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
                .expect("build effective settings");
        // 旧模型不注入 372K 默认值，遗留共享片段带进来的值也要剥掉
        assert!(effective["env"]
            .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
            .is_none());
        // 用户显式写在供应商配置里的值仍然生效
        assert_eq!(
            effective["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("300000")
        );
    }

    /// 往返不动点：注入产物只活在 live，切走回灌后存储配置必须与注入前一致，
    /// 否则程序默认值固化成"用户显式值"，之后调默认值永远压不动。
    #[test]
    fn codex_oauth_backfill_strips_injected_context_defaults() {
        let db = Database::memory().expect("create memory db");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({ "env": { "ANTHROPIC_MODEL": "gpt-5.6" } }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            ..Default::default()
        });

        // 模拟写 live：注入了两个上下文默认值
        let live = build_effective_settings_with_common_config(&db, &AppType::Claude, &provider)
            .expect("build effective settings");
        assert_eq!(
            live["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("372000")
        );

        // 模拟切走回灌：注入产物被剥掉，其余字段原样保留
        let backfilled =
            strip_common_config_from_live_settings(&db, &AppType::Claude, &provider, live);
        assert!(backfilled["env"]
            .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
            .is_none());
        assert!(backfilled["env"]
            .get("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
            .is_none());
        assert_eq!(backfilled["env"]["ANTHROPIC_MODEL"], json!("gpt-5.6"));
    }

    #[test]
    fn codex_oauth_backfill_keeps_user_context_values() {
        let db = Database::memory().expect("create memory db");
        let mut provider = Provider::with_id(
            "codex-oauth".to_string(),
            "Codex".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_MODEL": "gpt-5.6",
                    "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "500000"
                }
            }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            ..Default::default()
        });

        // live 里：MAX 是用户显式值；ACW 被用户手改成了非默认数字
        let live = json!({
            "env": {
                "ANTHROPIC_MODEL": "gpt-5.6",
                "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "500000",
                "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "300000"
            }
        });
        let backfilled =
            strip_common_config_from_live_settings(&db, &AppType::Claude, &provider, live);
        assert_eq!(
            backfilled["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
            json!("500000")
        );
        assert_eq!(
            backfilled["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"],
            json!("300000")
        );
    }

    #[test]
    fn claude_common_config_apply_and_remove_roundtrip_for_non_overlapping_fields() {
        let settings = json!({
            "env": {
                "ANTHROPIC_API_KEY": "sk-test"
            }
        });
        let snippet = r#"{
  "includeCoAuthoredBy": false,
  "env": {
    "CLAUDE_CODE_USE_BEDROCK": "1"
  }
}"#;

        let applied =
            apply_common_config_to_settings(&AppType::Claude, &settings, snippet).unwrap();
        assert_eq!(applied["includeCoAuthoredBy"], json!(false));
        assert_eq!(applied["env"]["CLAUDE_CODE_USE_BEDROCK"], json!("1"));

        let stripped =
            remove_common_config_from_settings(&AppType::Claude, &applied, snippet).unwrap();
        assert_eq!(stripped, settings);
    }

    #[test]
    fn codex_common_config_apply_and_remove_roundtrip_for_non_overlapping_fields() {
        let settings = json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test"
            },
            "config": "model_provider = \"openai\"\n[general]\nmodel = \"gpt-5\"\n"
        });
        let snippet = "[shared]\nreasoning = \"medium\"\n";

        let applied = apply_common_config_to_settings(&AppType::Codex, &settings, snippet).unwrap();
        let applied_config = applied["config"].as_str().unwrap_or_default();
        assert!(applied_config.contains("[shared]"));
        assert!(applied_config.contains("reasoning = \"medium\""));

        let stripped =
            remove_common_config_from_settings(&AppType::Codex, &applied, snippet).unwrap();
        assert_eq!(stripped, settings);
    }

    #[test]
    fn codex_managed_oauth_live_auth_matches_codex_cli_shape() {
        assert_eq!(
            codex_managed_oauth_live_auth(
                "acct-managed",
                "access-token",
                Some("id-token"),
                "refresh-token",
                "2026-01-02T03:04:05.000000000Z",
            ),
            json!({
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": {
                    "id_token": "id-token",
                    "access_token": "access-token",
                    "refresh_token": "refresh-token",
                    "account_id": "acct-managed"
                },
                "last_refresh": "2026-01-02T03:04:05.000000000Z"
            }),
            "managed live auth must carry refresh_token + last_refresh so the Codex CLI can self-refresh"
        );
    }

    #[test]
    fn codex_managed_oauth_live_auth_without_id_token_omits_it() {
        assert_eq!(
            codex_managed_oauth_live_auth(
                "acct-managed",
                "access-token",
                None,
                "refresh-token",
                "2026-01-02T03:04:05.000000000Z",
            ),
            json!({
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": {
                    "access_token": "access-token",
                    "refresh_token": "refresh-token",
                    "account_id": "acct-managed"
                },
                "last_refresh": "2026-01-02T03:04:05.000000000Z"
            }),
            "without a stored id_token the field is omitted rather than written as null"
        );
    }

    #[test]
    fn follow_login_backfill_preserves_latest_live_tokens() {
        let mut provider = Provider::with_id(
            "follow-login".to_string(),
            "OpenAI Official".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        provider.category = Some("official".to_string());
        let mut live_settings = json!({
            "auth": {
                "auth_mode": "chatgpt",
                "tokens": {
                    "access_token": "live-access-secret",
                    "refresh_token": "live-refresh-secret"
                }
            },
            "config": ""
        });

        strip_codex_managed_oauth_auth_for_backfill(&provider, &mut live_settings);

        assert_eq!(
            live_settings["auth"]["tokens"]["refresh_token"],
            json!("live-refresh-secret")
        );
    }

    #[test]
    fn category_less_fixed_follow_login_backfill_preserves_auth_and_strips_live_only_config() {
        let provider = Provider::with_id(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string(),
            "OpenAI Official".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        let injected_config = crate::codex_config::inject_codex_unified_session_bucket("")
            .expect("inject unified session bucket");
        let live_settings = json!({
            "auth": {
                "auth_mode": "chatgpt",
                "tokens": { "refresh_token": "live-refresh-secret" }
            },
            "config": injected_config
        });

        let backfilled =
            restore_live_settings_for_provider_backfill(&AppType::Codex, &provider, live_settings);

        assert_eq!(
            backfilled["auth"]["tokens"]["refresh_token"],
            json!("live-refresh-secret")
        );
        assert_eq!(backfilled["config"], json!(""));
    }

    #[test]
    fn category_less_fixed_follow_login_backfill_preserves_logout() {
        let provider = Provider::with_id(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": {
                    "auth_mode": "chatgpt",
                    "tokens": { "refresh_token": "old-refresh-token" }
                },
                "config": "model = \"old-model\"\n"
            }),
            None,
        );
        assert!(crate::proxy::providers::is_codex_official_provider(
            &provider
        ));

        for live_auth in [json!({}), json!({ "auth_mode": "chatgpt" })] {
            let live_settings = json!({
                "auth": live_auth,
                "config": "model = \"live-model\"\n"
            });
            let backfilled = restore_live_settings_for_provider_backfill(
                &AppType::Codex,
                &provider,
                live_settings.clone(),
            );

            assert_eq!(
                backfilled, live_settings,
                "a legacy official card must not restore the stored login after logout"
            );
        }
    }

    #[test]
    fn category_less_fixed_third_party_backfill_keeps_stored_api_key() {
        let provider = Provider::with_id(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string(),
            "Custom API".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-db-only" },
                "config": "model_provider = \"custom\"\n"
            }),
            None,
        );
        assert!(!crate::proxy::providers::is_codex_official_provider(
            &provider
        ));
        let backfilled = restore_live_settings_for_provider_backfill(
            &AppType::Codex,
            &provider,
            json!({ "auth": {}, "config": "model_provider = \"custom\"\n" }),
        );

        assert_eq!(backfilled, provider.settings_config);
    }

    #[test]
    fn backfill_never_persists_native_tokens_into_managed_provider_config() {
        // 托管 provider 的存储配置以占位 auth 表示；但 live 里此刻是用户自己
        // 浏览器登录的原生 auth（含真实 refresh_token）。backfill 必须把 live
        // auth 换回存储占位，绝不把原生 access/refresh_token 回填进 DB 配置。
        let mut provider = Provider::with_id(
            "openai-official".to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": {},
                "config": ""
            }),
            None,
        );
        provider.category = Some("official".to_string());
        provider.meta = Some(ProviderMeta {
            auth_binding: Some(AuthBinding {
                source: AuthBindingSource::ManagedAccount,
                auth_provider: Some("codex_oauth".to_string()),
                account_id: Some("acct-managed".to_string()),
            }),
            ..Default::default()
        });

        let mut live_settings = json!({
            "auth": {
                "OPENAI_API_KEY": null,
                "tokens": {
                    "id_token": "native-id",
                    "access_token": "native-access-secret",
                    "refresh_token": "native-refresh-secret",
                    "account_id": "acct-native"
                },
                "last_refresh": "2026-01-01T00:00:00Z"
            },
            "config": ""
        });

        strip_codex_managed_oauth_auth_for_backfill(&provider, &mut live_settings);

        assert_eq!(
            live_settings.get("auth"),
            Some(&json!({})),
            "managed provider backfill must reset live auth to the stored placeholder"
        );
        let serialized = live_settings.to_string();
        assert!(
            !serialized.contains("native-refresh-secret"),
            "native refresh_token must not leak into a managed provider's backfilled config"
        );
        assert!(
            !serialized.contains("native-access-secret"),
            "native access_token must not leak into a managed provider's backfilled config"
        );
    }

    #[test]
    fn backfill_leaves_non_managed_provider_auth_untouched() {
        // 非托管 provider 不受此剥离影响：其 auth 就该原样保留。
        let mut provider = Provider::with_id(
            "custom".to_string(),
            "Custom".to_string(),
            json!({ "auth": { "OPENAI_API_KEY": "user-key" }, "config": "" }),
            None,
        );
        provider.category = Some("custom".to_string());

        let mut live_settings = json!({
            "auth": { "OPENAI_API_KEY": "user-key" },
            "config": ""
        });
        let before = live_settings.clone();
        strip_codex_managed_oauth_auth_for_backfill(&provider, &mut live_settings);
        assert_eq!(
            live_settings, before,
            "non-managed provider auth must be left untouched by managed-oauth backfill stripping"
        );
    }

    #[test]
    fn explicit_common_config_flag_overrides_legacy_subset_detection() {
        let mut provider = Provider::with_id(
            "claude-test".to_string(),
            "Claude Test".to_string(),
            json!({
                "includeCoAuthoredBy": false
            }),
            None,
        );
        provider.meta = Some(crate::provider::ProviderMeta {
            common_config_enabled: Some(false),
            ..Default::default()
        });

        assert!(
            !provider_uses_common_config(
                &AppType::Claude,
                &provider,
                Some(r#"{ "includeCoAuthoredBy": false }"#),
            ),
            "explicit false should win over legacy subset detection"
        );
    }

    #[test]
    fn claude_common_config_array_subset_detection_and_strip_preserve_extra_items() {
        let settings = json!({
            "allowedTools": ["tool1", "tool2"]
        });
        let snippet = r#"{
  "allowedTools": ["tool1"]
}"#;

        assert!(
            settings_contain_common_config(&AppType::Claude, &settings, snippet),
            "array subset should be detected for legacy providers"
        );

        let stripped =
            remove_common_config_from_settings(&AppType::Claude, &settings, snippet).unwrap();
        assert_eq!(
            stripped,
            json!({
                "allowedTools": ["tool2"]
            })
        );
    }

    #[test]
    fn codex_common_config_array_subset_detection_and_strip_preserve_extra_items() {
        let settings = json!({
            "auth": {},
            "config": "allowed_tools = [\"tool1\", \"tool2\"]\n"
        });
        let snippet = "allowed_tools = [\"tool1\"]\n";

        assert!(
            settings_contain_common_config(&AppType::Codex, &settings, snippet),
            "TOML array subset should be detected for legacy providers"
        );

        let stripped =
            remove_common_config_from_settings(&AppType::Codex, &settings, snippet).unwrap();
        assert_eq!(stripped["auth"], json!({}));
        let stripped_config = stripped["config"].as_str().unwrap_or_default();
        let parsed = stripped_config
            .parse::<DocumentMut>()
            .expect("stripped codex config should remain valid TOML");
        let allowed_tools = parsed["allowed_tools"]
            .as_array()
            .expect("allowed_tools should remain an array");
        let values: Vec<&str> = allowed_tools
            .iter()
            .map(|value| value.as_str().expect("tool id should be string"))
            .collect();
        assert_eq!(values, vec!["tool2"]);
    }

    #[test]
    fn codex_switch_backfill_preserves_stored_model_catalog_when_live_lacks_it() {
        // Reproduces the data-loss bug: switching away from a Codex provider
        // backfills the outgoing provider from Live, but Live's config.toml had
        // already lost its `model_catalog_json` projection (proxy cycle /
        // Codex.app rewrite), so `read_live_settings` reconstructs no catalog.
        // The stored mapping must survive the backfill.
        let mut provider = Provider::with_id(
            "deepseek".to_string(),
            "DeepSeek".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-deepseek" },
                "config": "model_provider = \"custom\"\nmodel = \"deepseek-v4-pro\"\n",
                "modelCatalog": {
                    "models": [
                        { "model": "deepseek-v4-pro", "contextWindow": 1_000_000 }
                    ]
                }
            }),
            None,
        );
        provider.category = Some("cn_official".to_string());

        // Live snapshot as captured during switch: no `modelCatalog` field.
        let live_settings = json!({
            "auth": { "OPENAI_API_KEY": "sk-deepseek" },
            "config": "model_provider = \"custom\"\nmodel = \"deepseek-v4-pro\"\n"
        });

        let result =
            restore_live_settings_for_provider_backfill(&AppType::Codex, &provider, live_settings);

        assert_eq!(
            result.get("modelCatalog"),
            provider.settings_config.get("modelCatalog"),
            "switch-away backfill must keep the DB-stored modelCatalog when Live has none"
        );
    }

    #[test]
    fn codex_switch_backfill_keeps_live_catalog_when_db_has_none() {
        // When the DB provider has no stored catalog, a catalog reconstructed
        // from Live (if any) should be left intact — the DB-preference overlay
        // must not wipe it.
        let mut provider = Provider::with_id(
            "deepseek".to_string(),
            "DeepSeek".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-deepseek" },
                "config": "model_provider = \"custom\"\nmodel = \"deepseek-v4-pro\"\n"
            }),
            None,
        );
        provider.category = Some("cn_official".to_string());

        let live_settings = json!({
            "auth": { "OPENAI_API_KEY": "sk-deepseek" },
            "config": "model_provider = \"custom\"\nmodel = \"deepseek-v4-pro\"\n",
            "modelCatalog": { "models": [ { "model": "deepseek-v4-pro" } ] }
        });

        let result = restore_live_settings_for_provider_backfill(
            &AppType::Codex,
            &provider,
            live_settings.clone(),
        );

        assert_eq!(
            result.get("modelCatalog"),
            live_settings.get("modelCatalog"),
            "backfill must keep the Live-reconstructed catalog when the DB has none"
        );
    }

    #[test]
    fn codex_switch_backfill_keeps_stored_auth_when_live_has_no_credential() {
        // Repro of #7433: the provider table declares its own Authorization
        // header, so the switch injects no bearer token into config.toml, and
        // default mode deletes the shared auth.json. Live is `{ auth: {}, … }`
        // while the stored key is the only remaining copy — the switch-away
        // backfill must keep it, while still capturing the Live config.toml.
        let mut provider = Provider::with_id(
            "header-auth".to_string(),
            "Header Auth".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-db-only" },
                "config": "model_provider = \"custom\"\nmodel = \"old-model\"\n"
            }),
            None,
        );
        provider.category = Some("custom".to_string());

        let live_settings = json!({
            "auth": {},
            "config": "model_provider = \"custom\"\nmodel = \"live-model\"\n"
        });

        let result =
            restore_live_settings_for_provider_backfill(&AppType::Codex, &provider, live_settings);

        assert_eq!(
            result.get("auth"),
            Some(&json!({ "OPENAI_API_KEY": "sk-db-only" })),
            "a credential-less Live auth.json must not erase the stored provider key"
        );
        assert_eq!(
            result.get("config"),
            Some(&json!(
                "model_provider = \"custom\"\nmodel = \"live-model\"\n"
            )),
            "Live still owns the config.toml snapshot"
        );
    }

    #[test]
    fn codex_switch_backfill_keeps_live_auth_when_it_carries_material() {
        // Positive control: a Live auth.json that does carry material stays
        // authoritative (the manual `~/.codex/auth.json` edit path).
        let mut provider = Provider::with_id(
            "custom".to_string(),
            "Custom".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-db-stale" },
                "config": "model_provider = \"custom\"\n"
            }),
            None,
        );
        provider.category = Some("custom".to_string());

        let live_settings = json!({
            "auth": { "OPENAI_API_KEY": "sk-live" },
            "config": "model_provider = \"custom\"\n"
        });

        let result =
            restore_live_settings_for_provider_backfill(&AppType::Codex, &provider, live_settings);

        assert_eq!(
            result.get("auth"),
            Some(&json!({ "OPENAI_API_KEY": "sk-live" }))
        );
    }

    #[test]
    fn codex_switch_backfill_strips_synced_mcp_servers() {
        // Live 里的 [mcp_servers] 是 MCP 同步的投影（SSOT 在 DB 表），
        // 回填进供应商存储配置会让已删除的服务器随快照复活。
        let provider = Provider::with_id(
            "prov".to_string(),
            "Prov".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-test" },
                "config": "model = \"gpt-5.5\"\n"
            }),
            None,
        );

        let live_settings = json!({
            "auth": { "OPENAI_API_KEY": "sk-test" },
            "config": "model = \"gpt-5.5\"\n\n[mcp_servers.echo]\ntype = \"stdio\"\ncommand = \"echo\"\n"
        });

        let result =
            restore_live_settings_for_provider_backfill(&AppType::Codex, &provider, live_settings);

        let config_text = result
            .get("config")
            .and_then(|v| v.as_str())
            .expect("config text");
        assert!(
            !config_text.contains("mcp_servers"),
            "backfill must strip synced [mcp_servers] from the stored provider config, got: {config_text}"
        );
        assert!(
            config_text.contains("model = \"gpt-5.5\""),
            "non-MCP content must survive the strip"
        );
    }

    #[test]
    fn grok_switch_backfill_strips_synced_mcp_servers() {
        let provider = Provider::with_id(
            "grok".to_string(),
            "Grok".to_string(),
            json!({
                "config": "[models]\ndefault = \"grok-4.5\"\n\n[model.\"grok-4.5\"]\nmodel = \"grok-4.5\"\nbase_url = \"https://example.com/v1\"\nname = \"Example\"\napi_key = \"secret\"\napi_backend = \"responses\"\ncontext_window = 500000\n"
            }),
            None,
        );
        let live_settings = json!({
            "config": "[models]\ndefault = \"grok-4.5\"\n\n[model.\"grok-4.5\"]\nmodel = \"grok-4.5\"\nbase_url = \"https://example.com/v1\"\nname = \"Example\"\napi_key = \"secret\"\napi_backend = \"responses\"\ncontext_window = 500000\n\n[mcp_servers.echo]\ncommand = \"echo\"\n"
        });

        let result = restore_live_settings_for_provider_backfill(
            &AppType::GrokBuild,
            &provider,
            live_settings,
        );
        let config_text = result
            .get("config")
            .and_then(Value::as_str)
            .expect("config text");

        assert!(!config_text.contains("mcp_servers"));
        assert!(config_text.contains("model = \"grok-4.5\""));
    }

    #[test]
    fn claude_common_config_remove_strips_what_old_versions_merged() {
        let settings = json!({
            "env": {
                "ANTHROPIC_API_KEY": "sk-test"
            }
        });
        let snippet = r#"{
  "includeCoAuthoredBy": false,
  "env": {
    "CLAUDE_CODE_USE_BEDROCK": "1"
  }
}"#;

        // 旧版切换时把片段深合并进行里的样子。
        let applied = json!({
            "env": {
                "ANTHROPIC_API_KEY": "sk-test",
                "CLAUDE_CODE_USE_BEDROCK": "1"
            },
            "includeCoAuthoredBy": false
        });

        let stripped =
            remove_common_config_from_settings(&AppType::Claude, &applied, snippet).unwrap();
        assert_eq!(stripped, settings);
    }

    #[test]
    fn codex_common_config_remove_strips_what_old_versions_merged() {
        let settings = json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test"
            },
            "config": "model_provider = \"openai\"\n[general]\nmodel = \"gpt-5\"\n"
        });
        let snippet = "[shared]\nreasoning = \"medium\"\n";

        // 旧版切换时把片段合并进行里的样子。
        let applied = json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test"
            },
            "config": "model_provider = \"openai\"\n[general]\nmodel = \"gpt-5\"\n\n[shared]\nreasoning = \"medium\"\n"
        });

        let stripped =
            remove_common_config_from_settings(&AppType::Codex, &applied, snippet).unwrap();
        assert_eq!(stripped, settings);
    }
}
