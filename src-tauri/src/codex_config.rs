use std::path::{Path, PathBuf};

use crate::config::{read_json_file, write_text_file};
use crate::error::AppError;
use crate::provider::{Provider, ProviderMeta};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use toml_edit::{DocumentMut, Item, TableLike};

#[cfg(test)]
use once_cell::sync::OnceCell;
#[cfg(test)]
use std::collections::HashSet;

mod auth;
mod catalog;
mod features;
mod storage;

pub(crate) use auth::codex_auth_has_credential_login_material;
#[cfg(test)]
use auth::codex_live_auth_is_stale_third_party_residue;
pub use auth::{
    clear_stale_codex_live_auth_after_official_switch, codex_auth_has_login_material,
    extract_codex_auth_api_key, should_restore_codex_provider_token_for_backfill,
};

#[cfg(test)]
use catalog::*;
pub(crate) use catalog::{
    codex_top_level_model, read_codex_model_catalog_text, resolve_fyagent_catalog_path,
    CODEX_WEB_SEARCH_DISABLED, CODEX_WEB_SEARCH_FIELD,
};
pub use catalog::{
    prepare_codex_config_text_with_model_catalog, read_codex_model_catalog_simplified_from_live,
    CodexCatalogToolProfile,
};

#[cfg(test)]
pub use features::CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING;
use features::{
    active_codex_provider_table, inspect_managed_image_header, set_managed_image_header,
    ManagedHeaderInspection,
};
pub use features::{
    analyze_codex_provider_features, codex_provider_save_warning_codes,
    patch_codex_provider_features, prepare_codex_provider_features_for_save,
    validate_codex_provider_features, CodexProviderFeatureIntent, CodexProviderFeaturePatchResult,
    CodexProviderFeatureState, CODEX_IMAGE_EXTENSION_HEADER, CODEX_IMAGE_EXTENSION_VALUE,
};
#[cfg(test)]
use features::{
    codex_provider_config_text, set_provider_config_text, CodexImageExtensionState,
    CODEX_FEATURE_INVALID_TOML, CODEX_FEATURE_INVALID_WEBSOCKET,
    CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED_WARNING,
};

pub use storage::{
    get_codex_auth_path, get_codex_config_dir, get_codex_config_path, get_codex_model_catalog_path,
    read_and_validate_codex_config_text, read_codex_config_text, validate_config_toml,
    write_codex_live_atomic,
};

pub const FYAGENT_CODEX_MODEL_PROVIDER_ID: &str = "custom";
/// Temporary model-provider id used while the built-in `codex-official`
/// provider is routed through FyAgent.  A dedicated id is an ownership
/// marker: unlike a generic localhost `base_url`, it can be detected and
/// cleaned up without mistaking a user's own local provider for takeover.
pub const FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID: &str = "fyagent-official";
pub const FYAGENT_CODEX_MODEL_CATALOG_FILENAME: &str = "fyagent-model-catalog.json";
const CODEX_PROXY_AUTH_PLACEHOLDER: &str = "PROXY_MANAGED";

/// Reserved built-in provider IDs from OpenAI Codex's config/model-provider
/// catalog. Keep in sync with Codex `RESERVED_MODEL_PROVIDER_IDS` and legacy
/// removed provider aliases.
const CODEX_RESERVED_MODEL_PROVIDER_IDS: &[&str] = &[
    "amazon-bedrock",
    "openai",
    "ollama",
    "lmstudio",
    "oss",
    "ollama-chat",
];

fn active_codex_model_provider_id(doc: &DocumentMut) -> Option<String> {
    doc.get("model_provider")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

pub(crate) fn is_custom_codex_model_provider_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && !CODEX_RESERVED_MODEL_PROVIDER_IDS
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(id))
}

/// Write only Codex `config.toml` for provider switching.
///
/// Codex login state lives in `auth.json`; provider routing, endpoint, model,
/// and provider-scoped bearer tokens live in `config.toml`. Provider switches
/// should not overwrite the user's ChatGPT login cache.
pub fn write_codex_live_config_atomic(config_text_opt: Option<&str>) -> Result<(), AppError> {
    let config_path = get_codex_config_path();
    let cfg_text = match config_text_opt {
        Some(config_text) => config_text.to_string(),
        None => String::new(),
    };

    if !cfg_text.trim().is_empty() {
        toml::from_str::<toml::Table>(&cfg_text).map_err(|e| AppError::toml(&config_path, e))?;
    }

    write_text_file(&config_path, &cfg_text)
}

pub fn extract_codex_api_key(auth: Option<&Value>, config_text: Option<&str>) -> Option<String> {
    auth.and_then(extract_codex_auth_api_key)
        .or_else(|| config_text.and_then(extract_codex_experimental_bearer_token))
}

/// Extract the upstream base URL from a Codex `config.toml` string.
///
/// Prefers the active `[model_providers.<model_provider>].base_url`, falling
/// back to a top-level `base_url`. Deliberately never reads a non-active
/// `[model_providers.*]` section — the frontend `extractCodexBaseUrl`
/// (`getRecoverableBaseUrlAssignments`) excludes those too, and a leftover
/// section unrelated to the active provider must not leak into `{{baseUrl}}`.
pub fn extract_codex_base_url(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<toml::Value>().ok()?;

    if let Some(active_provider) = doc.get("model_provider").and_then(|v| v.as_str()) {
        if let Some(base_url) = doc
            .get("model_providers")
            .and_then(|providers| providers.get(active_provider))
            .and_then(|provider| provider.get("base_url"))
            .and_then(|v| v.as_str())
        {
            return Some(base_url.to_string());
        }
    }

    doc.get("base_url")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

/// Decide the `config.toml` text to write during a takeover-off restore,
/// projecting the model catalog **only when `settings` carries an inline
/// `modelCatalog`**.
///
/// Restore feeds back a stored backup, and Codex backups come in two shapes that
/// need opposite handling:
///
/// - **Snapshot backup** (`read_codex_live_settings`): `{ auth, config }` with no
///   inline `modelCatalog`. Its `config.toml` text already carries whatever
///   `model_catalog_json` pointer existed at backup time, and the generated
///   catalog file on disk is untouched. Here we must keep the config **raw** —
///   running catalog projection would see "no specs" and strip the live pointer.
/// - **Provider-rebuilt backup** (`update_live_backup_from_provider`): the DB
///   provider's settings, i.e. `{ auth, config (no pointer), modelCatalog
///   (inline DB SSOT) }`. Here the pointer/catalog file must be (re)generated
///   from the inline `modelCatalog`, or the mapping is lost on restore.
///
/// Gating on the presence of the inline `modelCatalog` key routes each shape
/// correctly; an empty inline catalog still projects (and so correctly drops a
/// now-stale pointer), while an absent key leaves the text untouched. This is
/// **orthogonal to auth** — a provider-rebuilt backup can pair an inline
/// `modelCatalog` with empty `auth.json` (the API key living in the config's
/// `experimental_bearer_token`), so the caller must decide config projection
/// independently of whether it writes or deletes `auth.json`.
pub fn prepare_codex_live_config_text_with_optional_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<String, AppError> {
    if settings.get("modelCatalog").is_some() {
        prepare_codex_config_text_with_model_catalog(settings, config_text, profile)
    } else {
        Ok(config_text.to_string())
    }
}

pub fn write_codex_provider_live_with_catalog(
    settings: &Value,
    category: Option<&str>,
    auth: &Value,
    config_text: Option<&str>,
    profile: CodexCatalogToolProfile,
) -> Result<(), AppError> {
    let prepared_config = config_text
        .map(|text| prepare_codex_config_text_with_model_catalog(settings, text, profile))
        .transpose()?;

    write_codex_live_for_provider(category, auth, prepared_config.as_deref())
}

/// Extract a provider-scoped `experimental_bearer_token` from Codex `config.toml`.
///
/// Mobile compat: third-party providers may store the API key inside
/// `[model_providers.<id>].experimental_bearer_token` while keeping the
/// user's ChatGPT login cache intact in `auth.json`. Falls back to the
/// top-level `experimental_bearer_token` when no active model provider is set.
pub fn extract_codex_experimental_bearer_token(config_text: &str) -> Option<String> {
    if !config_text.contains("experimental_bearer_token") {
        return None;
    }
    let doc = config_text.parse::<DocumentMut>().ok()?;
    let provider_id = active_codex_model_provider_id(&doc);

    let top_level_token = || {
        doc.get("experimental_bearer_token")
            .and_then(|item| item.as_str())
    };
    let token = match provider_id.as_deref() {
        Some(id) if is_custom_codex_model_provider_id(id) => doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get(id))
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("experimental_bearer_token"))
            .and_then(|item| item.as_str())
            .or_else(top_level_token),
        Some(_) => top_level_token(),
        None => top_level_token(),
    };

    token
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

fn set_codex_experimental_bearer_token(config_text: &str, token: &str) -> Result<String, AppError> {
    if config_text.trim().is_empty() {
        return Err(AppError::localized(
            "provider.codex.config.missing",
            "Codex 第三方供应商缺少 config.toml 配置，无法写入 bearer token",
            "Codex third-party provider is missing config.toml, cannot write bearer token",
        ));
    }

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    let Some(provider_id) = active_codex_model_provider_id(&doc) else {
        doc["experimental_bearer_token"] = toml_edit::value(token);
        return Ok(doc.to_string());
    };

    if !is_custom_codex_model_provider_id(&provider_id) {
        // Reserved Codex provider IDs are owned by the CLI. Keep third-party
        // bearer tokens at the top level so we do not shadow built-in tables.
        doc["experimental_bearer_token"] = toml_edit::value(token);
        return Ok(doc.to_string());
    }

    if let Some(model_providers) = doc
        .get_mut("model_providers")
        .and_then(|item| item.as_table_mut())
    {
        if let Some(provider_table) = model_providers
            .get_mut(provider_id.as_str())
            .and_then(|item| item.as_table_mut())
        {
            provider_table["experimental_bearer_token"] = toml_edit::value(token);
            return Ok(doc.to_string());
        }
    }

    doc["experimental_bearer_token"] = toml_edit::value(token);
    Ok(doc.to_string())
}

pub fn remove_codex_experimental_bearer_token_if(
    config_text: &str,
    predicate: impl Fn(&str) -> bool,
) -> Result<String, AppError> {
    if config_text.trim().is_empty() || !config_text.contains("experimental_bearer_token") {
        return Ok(config_text.to_string());
    }

    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    if let Some(provider_id) = active_codex_model_provider_id(&doc) {
        if let Some(provider_table) = doc
            .get_mut("model_providers")
            .and_then(|item| item.as_table_mut())
            .and_then(|table| table.get_mut(provider_id.as_str()))
            .and_then(|item| item.as_table_mut())
        {
            let should_remove = provider_table
                .get("experimental_bearer_token")
                .and_then(|item| item.as_str())
                .map(str::trim)
                .is_some_and(&predicate);
            if should_remove {
                provider_table.remove("experimental_bearer_token");
            }
        }
    }

    let should_remove_top_level = doc
        .get("experimental_bearer_token")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .is_some_and(&predicate);
    if should_remove_top_level {
        doc.as_table_mut().remove("experimental_bearer_token");
    }
    Ok(doc.to_string())
}

fn remove_codex_experimental_bearer_token(config_text: &str) -> Result<String, AppError> {
    remove_codex_experimental_bearer_token_if(config_text, |_| true)
}

/// Read the current Codex live settings as a `{ auth, config }` object.
///
/// Missing `auth.json` collapses to `{}` so a config-only third-party install
/// is still importable; both files missing is treated as "no live install".
/// A `config.toml` that exists but is empty is a valid state — e.g. the
/// official seed after stale-auth cleanup — and must stay readable.
pub fn read_codex_live_settings() -> Result<Value, AppError> {
    let auth_path = get_codex_auth_path();
    let auth_present = auth_path.exists();
    let auth: Value = if auth_present {
        read_json_file(&auth_path)?
    } else {
        json!({})
    };
    let cfg_text = read_and_validate_codex_config_text()?;
    if !auth_present && !get_codex_config_path().exists() {
        return Err(AppError::localized(
            "codex.live.missing",
            "Codex 配置文件不存在",
            "Codex configuration is missing",
        ));
    }
    Ok(json!({ "auth": auth, "config": cfg_text }))
}

/// `[model_providers.custom]` entry that makes an official (ChatGPT OAuth)
/// provider behave like Codex's built-in `openai` entry while running under
/// the shared custom id: `requires_openai_auth` routes auth to the ChatGPT
/// login in `auth.json` (base_url then defaults to the official Codex
/// backend), `name = "OpenAI"` keeps Codex's `is_openai()` feature gates
/// (web search, remote compaction). Callers opt into `supports_websockets`
/// only when their own route contract or the user's explicit draft requires
/// it; omission never serializes an overriding `false`.
fn codex_official_provider_table(
    base_url: Option<&str>,
    supports_websockets: bool,
) -> toml_edit::Table {
    let mut table = toml_edit::Table::new();
    table["name"] = toml_edit::value("OpenAI");
    table["requires_openai_auth"] = toml_edit::value(true);
    if supports_websockets {
        table["supports_websockets"] = toml_edit::value(true);
    }
    table["wire_api"] = toml_edit::value("responses");
    if let Some(base_url) = base_url {
        table["base_url"] = toml_edit::value(base_url.trim_end_matches('/'));
    }
    table
}

fn codex_unified_official_provider_table() -> toml_edit::Table {
    codex_official_provider_table(None, true)
}

fn remove_codex_proxy_placeholders_from_providers(providers: &mut toml_edit::Table) {
    for (_, item) in providers.iter_mut() {
        if let Some(table) = item.as_table_mut() {
            let should_remove = table
                .get("experimental_bearer_token")
                .and_then(|item| item.as_str())
                == Some(CODEX_PROXY_AUTH_PLACEHOLDER);
            if should_remove {
                table.remove("experimental_bearer_token");
            }
        } else if let Some(table) = item.as_inline_table_mut() {
            let should_remove = table
                .get("experimental_bearer_token")
                .and_then(|value| value.as_str())
                == Some(CODEX_PROXY_AUTH_PLACEHOLDER);
            if should_remove {
                table.remove("experimental_bearer_token");
            }
        }
    }
}

/// Project the built-in Codex official provider through the local proxy while
/// keeping authentication owned by Codex itself.
///
/// The resulting custom provider explicitly opts into OpenAI authentication,
/// so Codex forwards its existing ChatGPT login to the local `/responses`
/// endpoint.  No API key or bearer placeholder is written to `auth.json`.
pub fn apply_codex_official_proxy_route(
    config_text: &str,
    proxy_base_url: &str,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    let (supports_websockets, image_extension_enabled) = active_codex_provider_table(&doc)
        .map(|(_, table)| {
            (
                table.get("supports_websockets").and_then(Item::as_bool) == Some(true),
                matches!(
                    inspect_managed_image_header(table),
                    ManagedHeaderInspection::Controlled { .. }
                ),
            )
        })
        .unwrap_or((false, false));

    // A third-party takeover may have left the proxy placeholder in config.toml.
    // The official route must use Codex's native OpenAI login instead.
    doc.as_table_mut().remove("experimental_bearer_token");
    doc["model_provider"] = toml_edit::value(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID);

    let mut providers = match doc.as_table_mut().remove("model_providers") {
        Some(item) => item.into_table().map_err(|_| {
            AppError::Message(
                "Invalid Codex config.toml: model_providers must be a table".to_string(),
            )
        })?,
        None => {
            let mut table = toml_edit::Table::new();
            table.set_implicit(true);
            table
        }
    };

    // Clean only FyAgent's placeholder from every stale provider table. Real
    // user bearer tokens are preserved, as are all unrelated provider fields.
    remove_codex_proxy_placeholders_from_providers(&mut providers);

    // The local proxy currently exposes HTTP/SSE, not Codex websocket routes,
    // but the user's explicit provider capability is preserved. Save-time UI
    // warnings communicate the runtime risk without rewriting their choice.
    let mut table = codex_official_provider_table(Some(proxy_base_url), supports_websockets);
    if image_extension_enabled {
        set_managed_image_header(&mut table, true);
    }

    providers.insert(
        FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID,
        toml_edit::Item::Table(table),
    );
    doc["model_providers"] = toml_edit::Item::Table(providers);
    Ok(doc.to_string())
}

/// Whether a live Codex config is the official route projected by FyAgent.
pub fn codex_config_has_official_proxy_route(config_text: &str) -> bool {
    if !config_text.contains(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID) {
        return false;
    }
    config_text
        .parse::<DocumentMut>()
        .ok()
        .and_then(|doc| {
            doc.get("model_provider")
                .and_then(|item| item.as_str())
                .map(str::to_string)
        })
        .as_deref()
        == Some(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID)
}

/// Remove only the official takeover route owned by FyAgent. This is a
/// last-resort crash cleanup when no live backup or provider SSOT is usable.
pub fn remove_codex_official_proxy_route(config_text: &str) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    if doc.get("model_provider").and_then(|item| item.as_str())
        != Some(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID)
    {
        return Ok(config_text.to_string());
    }

    doc.as_table_mut().remove("model_provider");
    if let Some(item) = doc.as_table_mut().remove("model_providers") {
        let mut providers = item.into_table().map_err(|_| {
            AppError::Message(
                "Invalid Codex config.toml: model_providers must be a table".to_string(),
            )
        })?;
        providers.remove(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID);
        remove_codex_proxy_placeholders_from_providers(&mut providers);
        if !providers.is_empty() {
            doc["model_providers"] = toml_edit::Item::Table(providers);
        }
    }
    Ok(doc.to_string())
}

fn table_matches_codex_unified_official_provider(table: &toml_edit::Table) -> bool {
    table.len() == 4
        && table.get("name").and_then(|item| item.as_str()) == Some("OpenAI")
        && table
            .get("requires_openai_auth")
            .and_then(|item| item.as_bool())
            == Some(true)
        && table
            .get("supports_websockets")
            .and_then(|item| item.as_bool())
            == Some(true)
        && table.get("wire_api").and_then(|item| item.as_str()) == Some("responses")
}

/// 统一 Codex 会话历史：把官方供应商的 live 配置改写为以共享的
/// `custom` model_provider 标识运行（认证仍走 `auth.json` 的 ChatGPT 登录），
/// 使开关开启后创建的官方会话与第三方会话共用同一个 resume 历史桶。
///
/// 两种情况拒绝注入、原样返回：
/// - 配置已有显式 `model_provider`：用户手工指定的路由不被覆盖；
/// - 配置已有形态不同的 `[model_providers.custom]` 表：设置 `model_provider`
///   会激活这张我们不认识的表（可能带第三方 base_url/token，会把 ChatGPT
///   OAuth 流量路由到错误后端），宁可让开关对该配置不生效。
pub fn inject_codex_unified_session_bucket(config_text: &str) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    if doc.get("model_provider").is_some() {
        return Ok(config_text.to_string());
    }

    let existing_custom_conflicts = doc
        .get("model_providers")
        .and_then(|item| item.as_table())
        .and_then(|providers| providers.get(FYAGENT_CODEX_MODEL_PROVIDER_ID))
        .and_then(|item| item.as_table())
        .is_some_and(|table| !table_matches_codex_unified_official_provider(table));
    if existing_custom_conflicts {
        log::warn!(
            "官方 Codex 配置已存在自定义 [model_providers.custom]，跳过统一会话路由注入以避免激活未知路由"
        );
        return Ok(config_text.to_string());
    }

    doc["model_provider"] = toml_edit::value(FYAGENT_CODEX_MODEL_PROVIDER_ID);

    if doc.get("model_providers").is_none() {
        let mut parent = toml_edit::Table::new();
        parent.set_implicit(true);
        doc["model_providers"] = toml_edit::Item::Table(parent);
    }
    if let Some(providers) = doc["model_providers"].as_table_mut() {
        if !providers.contains_key(FYAGENT_CODEX_MODEL_PROVIDER_ID) {
            providers.insert(
                FYAGENT_CODEX_MODEL_PROVIDER_ID,
                toml_edit::Item::Table(codex_unified_official_provider_table()),
            );
        }
    }
    Ok(doc.to_string())
}

/// `inject_codex_unified_session_bucket` 的反向操作：从配置文本里剥掉注入的
/// 统一会话路由，保证切换回填不会把它带进数据库的存储配置（关闭开关后
/// 切换即可完全还原）。仅当形态与注入产物完全一致时才剥离；第三方模板和
/// 用户自定义的 `custom` 条目（带 base_url 等差异字段）原样保留。
pub fn strip_codex_unified_session_bucket(config_text: &str) -> Result<String, AppError> {
    if !config_text.contains("model_provider") {
        return Ok(config_text.to_string());
    }
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;

    if doc.get("model_provider").and_then(|item| item.as_str())
        != Some(FYAGENT_CODEX_MODEL_PROVIDER_ID)
    {
        return Ok(config_text.to_string());
    }
    let matches_injected = doc
        .get("model_providers")
        .and_then(|item| item.as_table())
        .and_then(|providers| providers.get(FYAGENT_CODEX_MODEL_PROVIDER_ID))
        .and_then(|item| item.as_table())
        .is_some_and(table_matches_codex_unified_official_provider);
    if !matches_injected {
        return Ok(config_text.to_string());
    }

    doc.as_table_mut().remove("model_provider");
    let providers_empty = doc["model_providers"]
        .as_table_mut()
        .map(|providers| {
            providers.remove(FYAGENT_CODEX_MODEL_PROVIDER_ID);
            providers.is_empty()
        })
        .unwrap_or(false);
    if providers_empty {
        doc.as_table_mut().remove("model_providers");
    }
    Ok(doc.to_string())
}

/// 统一会话开关开启时，把官方供应商 `{ auth, config }` 设置对象中的
/// config 文本注入共享 custom 路由；开关关闭或非官方供应商时不做改动。
///
/// 普通 live 写入（`write_codex_live_for_provider`）与代理接管备份
/// （`update_live_backup_from_provider`）两条落盘路径共用：接管期间
/// live 归代理所有，注入必须进备份，接管释放恢复的 live 才带统一路由。
pub fn apply_codex_unified_session_bucket_to_settings(
    category: Option<&str>,
    settings: &mut Value,
) -> Result<(), AppError> {
    if category != Some("official") || !crate::settings::unify_codex_session_history() {
        return Ok(());
    }
    let config_text = settings
        .get("config")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let injected = inject_codex_unified_session_bucket(&config_text)?;
    if injected != config_text {
        if let Some(obj) = settings.as_object_mut() {
            obj.insert("config".to_string(), Value::String(injected));
        }
    }
    Ok(())
}

/// Backfill helper: strip the unified-session injection from a live
/// `{ auth, config }` settings object before it is stored back to the DB.
pub fn strip_codex_unified_session_bucket_from_settings(
    settings: &mut Value,
) -> Result<(), AppError> {
    let Some(config_text) = settings
        .get("config")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };
    let stripped = strip_codex_unified_session_bucket(&config_text)?;
    if stripped != config_text {
        if let Some(obj) = settings.as_object_mut() {
            obj.insert("config".to_string(), Value::String(stripped));
        }
    }
    Ok(())
}

/// Backfill helper: strip `[mcp_servers]` from a live `{ auth, config }`
/// settings object before it is stored back to the DB.
///
/// MCP 服务器的 SSOT 是 DB 的 mcp_servers 表，live `config.toml` 里的
/// `[mcp_servers]` 只是每次写 live 之后由 MCP 同步重新投影的产物。若回填时
/// 烙进供应商存储配置，已在应用里删除的服务器会随下次激活该供应商被写回
/// live，而逐条 reconcile 只认识 DB 现存条目、永远清不掉这种孤儿。
pub fn strip_codex_mcp_servers_from_settings(settings: &mut Value) -> Result<(), AppError> {
    let Some(config_text) = settings
        .get("config")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };
    if !config_text.contains("mcp") {
        return Ok(());
    }
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    let mut changed = doc.as_table_mut().remove("mcp_servers").is_some();
    // 历史错误格式 [mcp.servers] 一并清理（live 侧 MCP 同步也做同样迁移）
    if let Some(mcp_tbl) = doc.get_mut("mcp").and_then(|item| item.as_table_like_mut()) {
        if mcp_tbl.remove("servers").is_some() {
            changed = true;
        }
        if mcp_tbl.is_empty() {
            doc.as_table_mut().remove("mcp");
        }
    }
    if changed {
        if let Some(obj) = settings.as_object_mut() {
            obj.insert("config".to_string(), Value::String(doc.to_string()));
        }
    }
    Ok(())
}

/// Route a Codex live write between full auth+config or config-only.
///
/// Official providers with usable login material own `auth.json`. Third-party
/// providers only touch `config.toml` when the compatibility setting is enabled
/// so the user's ChatGPT login cache survives provider switches.
///
/// 统一会话开关开启时，官方配置在落盘前注入共享的 `custom` 路由
/// （见 `inject_codex_unified_session_bucket`）。
pub fn write_codex_live_for_provider(
    category: Option<&str>,
    auth: &Value,
    config_text: Option<&str>,
) -> Result<(), AppError> {
    let unified_official_config =
        if category == Some("official") && crate::settings::unify_codex_session_history() {
            Some(inject_codex_unified_session_bucket(
                config_text.unwrap_or(""),
            )?)
        } else {
            None
        };
    let config_text = unified_official_config.as_deref().or(config_text);

    let should_write_auth = (category == Some("official") && codex_auth_has_login_material(auth))
        || (category != Some("official")
            && !crate::settings::preserve_codex_official_auth_on_switch());

    if should_write_auth {
        let projected_config = match config_text {
            Some(text) => Some(project_codex_live_config_when_openai_auth_disabled(
                auth, text,
            )?),
            None => None,
        };
        write_codex_live_atomic(auth, projected_config.as_deref())
    } else {
        let live_config = prepare_codex_provider_live_config(auth, config_text.unwrap_or(""))?;
        write_codex_live_config_atomic(Some(&live_config))
    }
}

/// Current Codex ignores `auth.json` when the active provider sets
/// `requires_openai_auth = false`. Project the stored `OPENAI_API_KEY` into
/// provider-scoped `experimental_bearer_token` so live requests still authenticate.
/// `requires_openai_auth = true` or a missing field keeps the stored TOML as-is
/// so the API key continues to live only in `auth.json`.
fn project_codex_live_config_when_openai_auth_disabled(
    auth: &Value,
    config_text: &str,
) -> Result<String, AppError> {
    if active_codex_provider_disables_openai_auth(config_text) {
        prepare_codex_provider_live_config(auth, config_text)
    } else {
        Ok(config_text.to_string())
    }
}

fn active_codex_provider_disables_openai_auth(config_text: &str) -> bool {
    let Ok(doc) = config_text.parse::<DocumentMut>() else {
        return false;
    };
    active_codex_provider_table(&doc)
        .and_then(|(_, table)| table.get("requires_openai_auth").and_then(Item::as_bool))
        == Some(false)
}

/// Build the live Codex config for provider switching.
///
/// The stored provider keeps its API key in `auth.OPENAI_API_KEY`. Live Codex
/// requests can use a provider-scoped `experimental_bearer_token`, so switching
/// providers only needs to update `config.toml`; `auth.json` stays as the user's
/// long-lived ChatGPT login cache.
pub fn prepare_codex_provider_live_config(
    auth: &Value,
    config_text: &str,
) -> Result<String, AppError> {
    let token = extract_codex_auth_api_key(auth)
        .or_else(|| extract_codex_experimental_bearer_token(config_text));

    Ok(match token {
        Some(token) => set_codex_experimental_bearer_token(config_text, &token)?,
        None => config_text.to_string(),
    })
}

/// Apply only the fields owned by the V2 Quick Setup form to an existing
/// Codex `config.toml`. The existing document is the authority for every
/// unrelated field/table/comment; the Quick Setup provider is intentionally a
/// patch, not a complete replacement snapshot.
pub fn patch_codex_quick_setup_live_config(
    current_config: &str,
    desired_config: &str,
) -> Result<String, AppError> {
    let mut target = if current_config.trim().is_empty() {
        DocumentMut::new()
    } else {
        current_config
            .parse::<DocumentMut>()
            .map_err(|error| AppError::Message(format!("Invalid Codex config.toml: {error}")))?
    };
    let desired = desired_config
        .parse::<DocumentMut>()
        .map_err(|error| AppError::Message(format!("Invalid Codex quick setup TOML: {error}")))?;

    let provider_id = active_codex_model_provider_id(&desired).ok_or_else(|| {
        AppError::Message("Codex quick setup is missing model_provider".to_string())
    })?;
    let desired_provider = active_codex_provider_table(&desired)
        .map(|(_, table)| table)
        .ok_or_else(|| {
            AppError::Message("Codex quick setup provider table is missing".to_string())
        })?;

    target["model_provider"] = toml_edit::value(&provider_id);
    if let Some(desired_model) = desired.get("model").and_then(Item::as_str) {
        target["model"] = toml_edit::value(desired_model);
    }

    if target.get("model_providers").is_none() {
        target["model_providers"] = toml_edit::table();
    }
    let providers = target
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::Message("Codex model_providers must be an editable table".to_string())
        })?;
    if providers.get(&provider_id).is_none() {
        providers.insert(&provider_id, toml_edit::table());
    }
    let target_provider = providers
        .get_mut(&provider_id)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::Message("Codex quick setup provider must be an editable table".to_string())
        })?;

    for key in ["name", "base_url", "wire_api", "requires_openai_auth"] {
        if let Some(item) = desired_provider.get(key) {
            target_provider.insert(key, item.clone());
        }
    }

    // Historical reserved Quick Setup rows predate the explicit capability
    // intent and therefore do not carry `requires_openai_auth`. Treat absence
    // as "preserve current capability fields" rather than inventing a default.
    // New V2 requests always carry the field, so their false/true choices are
    // authoritative for image/WebSocket/token ownership.
    if desired_provider.get("requires_openai_auth").is_some() {
        let image_enabled = match inspect_managed_image_header(desired_provider) {
            ManagedHeaderInspection::Controlled { .. } => true,
            ManagedHeaderInspection::Missing => false,
            ManagedHeaderInspection::Conflict { .. } | ManagedHeaderInspection::Invalid => {
                return Err(AppError::Message(
                    "Codex quick setup image header is invalid".to_string(),
                ))
            }
        };
        set_managed_image_header(target_provider, image_enabled);

        if desired_provider
            .get("supports_websockets")
            .and_then(Item::as_bool)
            == Some(true)
        {
            target_provider.insert("supports_websockets", toml_edit::value(true));
        } else {
            target_provider.remove("supports_websockets");
        }

        if let Some(token) = desired_provider
            .get("experimental_bearer_token")
            .and_then(Item::as_str)
        {
            target_provider.insert("experimental_bearer_token", toml_edit::value(token));
        } else {
            target_provider.remove("experimental_bearer_token");
        }
    }

    Ok(target.to_string())
}

/// During DB backfill, lift a live `experimental_bearer_token` back into
/// `auth.OPENAI_API_KEY` so the stored provider keeps its canonical shape
/// and generated live tokens don't leak into stored provider TOML.
///
/// Only intervenes when the live config actually carries a bearer token —
/// otherwise the function is a no-op so the caller's normal backfill path
/// (which keeps live `auth` as the authoritative source) is unaffected.
pub fn restore_codex_provider_token_for_backfill(
    settings: &mut Value,
    template_settings: &Value,
) -> Result<(), AppError> {
    let Some(config_text) = settings
        .get("config")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };

    let Some(token) = extract_codex_experimental_bearer_token(&config_text) else {
        return Ok(());
    };

    let cleaned_config = remove_codex_experimental_bearer_token(&config_text)?;

    if let Some(obj) = settings.as_object_mut() {
        obj.insert("config".to_string(), Value::String(cleaned_config));

        let mut auth = template_settings
            .get("auth")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        if let Some(auth_obj) = auth.as_object_mut() {
            auth_obj.insert("OPENAI_API_KEY".to_string(), Value::String(token));
        }
        obj.insert("auth".to_string(), auth);
    }

    Ok(())
}

pub fn restore_codex_settings_for_backfill(
    settings: &mut Value,
    template_settings: &Value,
    restore_provider_token: bool,
) -> Result<(), AppError> {
    if restore_provider_token {
        restore_codex_provider_token_for_backfill(settings, template_settings)?;
    }
    Ok(())
}

/// Update a field in Codex config.toml using toml_edit (syntax-preserving).
///
/// Supported fields:
/// - `"base_url"`: writes to `[model_providers.<current>].base_url` if `model_provider` exists,
///   otherwise falls back to top-level `base_url`.
/// - `"wire_api"`: writes to `[model_providers.<current>].wire_api` if `model_provider` exists,
///   otherwise falls back to top-level `wire_api`.
/// - `"model"` / `"model_catalog_json"`: writes to top-level field.
///
/// Empty value removes the field.
pub fn update_codex_toml_field(toml_str: &str, field: &str, value: &str) -> Result<String, String> {
    let mut doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| format!("TOML parse error: {e}"))?;

    let trimmed = value.trim();

    match field {
        "base_url" | "wire_api" => {
            let model_provider = doc
                .get("model_provider")
                .and_then(|item| item.as_str())
                .map(str::to_string);

            if let Some(provider_key) = model_provider {
                // Ensure [model_providers] table exists
                //
                // 用 as_table_like_mut 而非 as_table_mut：用户把配置写成 inline table
                // （`model_providers = { foo = {...} }`，TOML 合法）时 as_table_mut
                // 返回 None，会一路掉进下面的顶层 fallback——用户改的 base_url 被写到
                // 了错误层级且毫无提示。
                if doc
                    .get("model_providers")
                    .is_none_or(|item| item.as_table_like().is_none())
                {
                    // 键存在但不是表（`model_providers = 42`）时，下面这行会把用户
                    // 手写的值替换掉。旧代码在这种形状下会掉进顶层 fallback 而不动
                    // 它，所以归一化必须留痕——与 mcp/codex.rs、mcp/grokbuild.rs、
                    // opencode_config.rs 的同款处理保持一致。
                    if doc
                        .get("model_providers")
                        .is_some_and(|item| !item.is_none())
                    {
                        log::warn!("config.toml 的 model_providers 不是表，已重置为空表");
                    }
                    doc["model_providers"] = toml_edit::table();
                }

                if let Some(model_providers) = doc
                    .get_mut("model_providers")
                    .and_then(toml_edit::Item::as_table_like_mut)
                {
                    // Ensure [model_providers.<provider_key>] table exists
                    if !model_providers.contains_key(&provider_key) {
                        model_providers.insert(&provider_key, toml_edit::table());
                    }

                    if let Some(provider_table) = model_providers
                        .get_mut(&provider_key)
                        .and_then(toml_edit::Item::as_table_like_mut)
                    {
                        if trimmed.is_empty() {
                            provider_table.remove(field);
                        } else {
                            provider_table.insert(field, toml_edit::value(trimmed));
                        }
                        return Ok(doc.to_string());
                    }
                }

                log::warn!(
                    "config.toml 的 [model_providers.{provider_key}] 结构异常，{field} 改写为顶层字段"
                );
            }

            // Fallback: no model_provider or structure mismatch → top-level field
            if trimmed.is_empty() {
                doc.as_table_mut().remove(field);
            } else {
                doc[field] = toml_edit::value(trimmed);
            }
        }
        "model" | "model_catalog_json" => {
            if trimmed.is_empty() {
                doc.as_table_mut().remove(field);
            } else {
                doc[field] = toml_edit::value(trimmed);
            }
        }
        _ => return Err(format!("unsupported field: {field}")),
    }

    Ok(doc.to_string())
}

/// Remove `base_url` from the active model_provider section only if it matches `predicate`.
/// Also removes top-level `base_url` if it matches.
/// Used by proxy cleanup to strip local proxy URLs without touching user-configured URLs.
pub fn remove_codex_toml_base_url_if(toml_str: &str, predicate: impl Fn(&str) -> bool) -> String {
    let mut doc = match toml_str.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(_) => return toml_str.to_string(),
    };

    let model_provider = doc
        .get("model_provider")
        .and_then(|item| item.as_str())
        .map(str::to_string);

    if let Some(provider_key) = model_provider {
        if let Some(model_providers) = doc
            .get_mut("model_providers")
            .and_then(|v| v.as_table_mut())
        {
            if let Some(provider_table) = model_providers
                .get_mut(provider_key.as_str())
                .and_then(|v| v.as_table_mut())
            {
                let should_remove = provider_table
                    .get("base_url")
                    .and_then(|item| item.as_str())
                    .map(&predicate)
                    .unwrap_or(false);
                if should_remove {
                    provider_table.remove("base_url");
                }
            }
        }
    }

    // Fallback: also clean up top-level base_url if it matches
    let should_remove_root = doc
        .get("base_url")
        .and_then(|item| item.as_str())
        .map(&predicate)
        .unwrap_or(false);
    if should_remove_root {
        doc.as_table_mut().remove("base_url");
    }

    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn feature_provider(
        config: &str,
        api_format: Option<&str>,
        image_extension_configured: Option<bool>,
    ) -> Provider {
        let mut provider = Provider::with_id(
            "fixture".to_owned(),
            "Fixture provider".to_owned(),
            json!({
                "auth": { "OPENAI_API_KEY": "fixture-key" },
                "config": config,
            }),
            None,
        );
        let meta = ProviderMeta {
            api_format: api_format.map(str::to_owned),
            image_extension_configured,
            ..ProviderMeta::default()
        };
        provider.meta = Some(meta);
        provider
    }

    fn feature_config(extra: &str) -> String {
        format!(
            r#"# keep leading comment
model_provider = "fixture"

[model_providers.fixture]
# keep provider comment
base_url = "https://gateway.example.test/v1"
wire_api = "responses"
unknown_scalar = 42
{extra}"#
        )
    }

    #[test]
    fn feature_patch_preserves_comments_order_unknown_fields_and_unrelated_headers() {
        let config = feature_config(
            r#"http_headers = { "X-Company-ID" = "company", "User-Agent" = "fixture" } # keep header comment
"#,
        );
        let provider = feature_provider(&config, Some("openai_responses"), Some(false));

        let patched = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: Some(true),
            },
            false,
        )
        .expect("native-capability patch");

        assert!(patched.toml_text.contains("# keep leading comment"));
        assert!(patched.toml_text.contains("# keep provider comment"));
        assert!(patched.toml_text.contains("# keep header comment"));
        let base_url_at = patched.toml_text.find("base_url").unwrap();
        let wire_api_at = patched.toml_text.find("wire_api").unwrap();
        let unknown_at = patched.toml_text.find("unknown_scalar").unwrap();
        let headers_at = patched.toml_text.find("http_headers").unwrap();
        assert!(base_url_at < wire_api_at && wire_api_at < unknown_at && unknown_at < headers_at);

        let parsed: toml::Value = toml::from_str(&patched.toml_text).expect("patched TOML");
        let table = &parsed["model_providers"]["fixture"];
        assert_eq!(table["unknown_scalar"].as_integer(), Some(42));
        assert_eq!(
            table["http_headers"]["X-Company-ID"].as_str(),
            Some("company")
        );
        assert_eq!(
            table["http_headers"]["User-Agent"].as_str(),
            Some("fixture")
        );
        assert_eq!(
            table["http_headers"][CODEX_IMAGE_EXTENSION_HEADER].as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
        );
        assert_eq!(table["supports_websockets"].as_bool(), Some(true));
        assert_eq!(patched.image_extension_configured, Some(true));
    }

    #[test]
    fn image_header_is_case_insensitive_and_off_removes_only_the_controlled_key() {
        let config = feature_config(
            r#"http_headers = { "X-OpenAI-Actor-Authorization" = "local-image-extension", "X-Company-ID" = "company" }
"#,
        );
        let provider = feature_provider(&config, Some("openai_responses"), Some(true));
        assert!(matches!(
            analyze_codex_provider_features(&provider, false).image_extension,
            CodexImageExtensionState::On
        ));

        let on = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )
        .expect("idempotent on patch");
        assert_eq!(
            on.toml_text
                .to_ascii_lowercase()
                .matches(CODEX_IMAGE_EXTENSION_HEADER)
                .count(),
            1,
            "on must not add a second header case variant"
        );

        let off = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(false),
                websockets: None,
            },
            false,
        )
        .expect("off patch");
        let parsed: toml::Value = toml::from_str(&off.toml_text).expect("patched TOML");
        let headers = parsed["model_providers"]["fixture"]["http_headers"]
            .as_table()
            .expect("headers table");
        assert!(headers
            .keys()
            .all(|key| !key.eq_ignore_ascii_case(CODEX_IMAGE_EXTENSION_HEADER)));
        assert_eq!(headers["X-Company-ID"].as_str(), Some("company"));
        assert_eq!(off.image_extension_configured, Some(true));
    }

    #[test]
    fn conflicting_or_nonstring_headers_are_repaired_only_by_explicit_toggle() {
        let conflicting = feature_config(
            r#"http_headers = { "x-openai-actor-authorization" = "local-image-extension", "X-OpenAI-Actor-Authorization" = "private-conflict-value" }
"#,
        );
        let provider = feature_provider(&conflicting, Some("openai_responses"), None);
        assert!(matches!(
            analyze_codex_provider_features(&provider, false).image_extension,
            CodexImageExtensionState::Conflict { .. }
        ));
        validate_codex_provider_features(&provider)
            .expect("unrelated provider fields may be saved with a header conflict");
        let mut saved_provider = provider.clone();
        prepare_codex_provider_features_for_save(&mut saved_provider, false)
            .expect("unrelated save preserves the conflict");
        assert_eq!(
            saved_provider.settings_config["config"].as_str(),
            Some(conflicting.as_str())
        );

        let repaired = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(false),
                websockets: None,
            },
            false,
        )
        .expect("explicit off removes every case variant");
        assert!(!repaired.toml_text.contains("private-conflict-value"));
        assert_eq!(
            repaired
                .toml_text
                .to_ascii_lowercase()
                .matches(CODEX_IMAGE_EXTENSION_HEADER)
                .count(),
            0
        );
        let repaired_on = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )
        .expect("explicit on normalizes every case variant");
        assert_eq!(
            repaired_on
                .toml_text
                .to_ascii_lowercase()
                .matches(CODEX_IMAGE_EXTENSION_HEADER)
                .count(),
            1
        );
        let parsed: toml::Value = toml::from_str(&repaired_on.toml_text).expect("normalized TOML");
        assert_eq!(
            parsed["model_providers"]["fixture"]["http_headers"][CODEX_IMAGE_EXTENSION_HEADER]
                .as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
        );
        assert_eq!(
            provider.settings_config["config"].as_str(),
            Some(conflicting.as_str())
        );
        assert_eq!(
            saved_provider
                .meta
                .as_ref()
                .and_then(|meta| meta.image_extension_configured),
            None,
            "conflict-only saves must leave the migration marker unfinished"
        );

        let invalid = feature_config("http_headers = [\"private-invalid-value\"]\n");
        let provider = feature_provider(&invalid, Some("openai_responses"), Some(true));
        validate_codex_provider_features(&provider)
            .expect("an unrelated save preserves an invalid header field");
        let repaired = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )
        .expect("explicit on replaces an invalid header field");
        assert!(!repaired.toml_text.contains("private-invalid-value"));
        let parsed: toml::Value = toml::from_str(&repaired.toml_text).expect("repaired TOML");
        assert_eq!(
            parsed["model_providers"]["fixture"]["http_headers"][CODEX_IMAGE_EXTENSION_HEADER]
                .as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
        );
        assert_eq!(
            provider.settings_config["config"].as_str(),
            Some(invalid.as_str())
        );

        let removed = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(false),
                websockets: None,
            },
            false,
        )
        .expect("explicit off deletes an invalid header field");
        let parsed: toml::Value = toml::from_str(&removed.toml_text).expect("repaired TOML");
        assert!(parsed["model_providers"]["fixture"]
            .get("http_headers")
            .is_none());
    }

    #[test]
    fn legacy_marker_is_read_only_until_the_provider_is_saved() {
        let config = feature_config("# historical record has no owned header\n");
        let mut provider = feature_provider(&config, Some("openai_responses"), None);

        assert!(matches!(
            analyze_codex_provider_features(&provider, false).image_extension,
            CodexImageExtensionState::LegacyPendingOn
        ));
        assert_eq!(
            provider.settings_config["config"].as_str(),
            Some(config.as_str())
        );
        assert_eq!(
            provider
                .meta
                .as_ref()
                .and_then(|meta| meta.image_extension_configured),
            None
        );

        prepare_codex_provider_features_for_save(&mut provider, false)
            .expect("the real save applies the deferred default");
        let saved = provider.settings_config["config"].as_str().unwrap();
        let parsed: toml::Value = toml::from_str(saved).expect("saved TOML");
        assert_eq!(
            parsed["model_providers"]["fixture"]["http_headers"][CODEX_IMAGE_EXTENSION_HEADER]
                .as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
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
    fn false_marker_is_unfinished_for_a_new_provider_and_is_completed_on_save() {
        let config = feature_config("");
        let mut provider = feature_provider(&config, Some("openai_responses"), Some(false));

        assert!(matches!(
            analyze_codex_provider_features(&provider, true).image_extension,
            CodexImageExtensionState::On
        ));

        prepare_codex_provider_features_for_save(&mut provider, true)
            .expect("new provider save applies the default");
        let saved: toml::Value = toml::from_str(
            provider.settings_config["config"]
                .as_str()
                .expect("saved config"),
        )
        .expect("saved TOML");
        assert_eq!(
            saved["model_providers"]["fixture"]["http_headers"][CODEX_IMAGE_EXTENSION_HEADER]
                .as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
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
    fn false_marker_is_unfinished_for_a_historical_provider_and_is_completed_on_save() {
        let config = feature_config("# historical record has no owned header\n");
        let mut provider = feature_provider(&config, Some("openai_responses"), Some(false));

        assert!(matches!(
            analyze_codex_provider_features(&provider, false).image_extension,
            CodexImageExtensionState::LegacyPendingOn
        ));

        prepare_codex_provider_features_for_save(&mut provider, false)
            .expect("historical provider save applies the deferred default");
        let saved: toml::Value = toml::from_str(
            provider.settings_config["config"]
                .as_str()
                .expect("saved config"),
        )
        .expect("saved TOML");
        assert_eq!(
            saved["model_providers"]["fixture"]["http_headers"][CODEX_IMAGE_EXTENSION_HEADER]
                .as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
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
    fn new_provider_explicit_image_intent_controls_patch_state_and_first_save() {
        let config = feature_config("");
        let provider = feature_provider(&config, Some("openai_responses"), None);
        assert!(matches!(
            analyze_codex_provider_features(&provider, true).image_extension,
            CodexImageExtensionState::On
        ));

        let enabled = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            true,
        )
        .expect("new-provider on patch");
        assert!(matches!(
            enabled.state.image_extension,
            CodexImageExtensionState::On
        ));
        assert_eq!(enabled.image_extension_configured, Some(true));

        let disabled = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(false),
                websockets: None,
            },
            true,
        )
        .expect("new-provider off patch");
        assert!(matches!(
            disabled.state.image_extension,
            CodexImageExtensionState::Off
        ));
        assert_eq!(disabled.image_extension_configured, Some(true));

        let mut first_save = provider.clone();
        set_provider_config_text(&mut first_save, disabled.toml_text)
            .expect("apply form-only draft before save");
        first_save
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .image_extension_configured = disabled.image_extension_configured;
        prepare_codex_provider_features_for_save(&mut first_save, true)
            .expect("explicit off survives first save");

        let saved: toml::Value = toml::from_str(
            first_save.settings_config["config"]
                .as_str()
                .expect("saved config"),
        )
        .expect("saved TOML");
        assert!(saved["model_providers"]["fixture"]
            .get("http_headers")
            .is_none());
        assert_eq!(
            first_save
                .meta
                .as_ref()
                .and_then(|meta| meta.image_extension_configured),
            Some(true)
        );
    }

    #[test]
    fn websocket_is_editable_for_every_api_format_and_repairs_invalid_types() {
        let config = feature_config("supports_websockets = true\n");
        for api_format in ["openai_responses", "openai_chat", "anthropic"] {
            let provider = feature_provider(&config, Some(api_format), Some(false));
            let state = analyze_codex_provider_features(&provider, false);
            assert!(state.websockets.enabled, "format: {api_format}");
            assert!(state.websockets.compatible, "format: {api_format}");
            validate_codex_provider_features(&provider)
                .expect("every upstream format accepts the WebSocket field");

            let enabled = patch_codex_provider_features(
                &provider,
                &CodexProviderFeatureIntent {
                    image_extension: None,
                    websockets: Some(true),
                },
                false,
            )
            .expect("WebSocket remains enabled for every upstream format");
            let parsed: toml::Value = toml::from_str(&enabled.toml_text).expect("enabled TOML");
            assert_eq!(
                parsed["model_providers"]["fixture"]["supports_websockets"].as_bool(),
                Some(true),
                "format: {api_format}"
            );
        }

        let provider = feature_provider(&config, Some("openai_chat"), Some(false));

        let repaired = patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(false),
            },
            false,
        )
        .expect("explicit false must remove the invalid field");
        let parsed: toml::Value = toml::from_str(&repaired.toml_text).expect("repaired TOML");
        assert!(parsed["model_providers"]["fixture"]
            .get("supports_websockets")
            .is_none());
        assert!(!repaired.toml_text.contains("supports_websockets = false"));

        let invalid = feature_config("supports_websockets = \"yes\"\n");
        let invalid_provider = feature_provider(&invalid, Some("anthropic"), Some(true));
        let invalid_state = analyze_codex_provider_features(&invalid_provider, false);
        assert!(invalid_state
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CODEX_FEATURE_INVALID_WEBSOCKET));
        validate_codex_provider_features(&invalid_provider)
            .expect("invalid field type is non-blocking until its switch is used");
        let repaired = patch_codex_provider_features(
            &invalid_provider,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(true),
            },
            false,
        )
        .expect("explicit on overwrites the invalid type");
        let parsed: toml::Value = toml::from_str(&repaired.toml_text).expect("repaired TOML");
        assert_eq!(
            parsed["model_providers"]["fixture"]["supports_websockets"].as_bool(),
            Some(true)
        );

        let removed = patch_codex_provider_features(
            &invalid_provider,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(false),
            },
            false,
        )
        .expect("explicit off removes the invalid type");
        let parsed: toml::Value = toml::from_str(&removed.toml_text).expect("repaired TOML");
        assert!(parsed["model_providers"]["fixture"]
            .get("supports_websockets")
            .is_none());
    }

    #[test]
    fn invalid_toml_is_visible_but_remains_the_only_capability_write_blocker() {
        let provider = feature_provider("[model_providers.fixture\n", None, None);
        let state = analyze_codex_provider_features(&provider, false);
        assert!(!state.applicable);
        assert!(state
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CODEX_FEATURE_INVALID_TOML));
        assert!(validate_codex_provider_features(&provider).is_err());
        assert!(patch_codex_provider_features(
            &provider,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )
        .is_err());
        assert_eq!(
            provider.settings_config["config"].as_str(),
            Some("[model_providers.fixture\n")
        );
    }

    #[test]
    fn feature_controls_apply_to_reserved_official_managed_and_third_party_providers() {
        let no_credential = feature_config("");
        let provider = Provider::with_id(
            "fixture".to_owned(),
            "Fixture provider".to_owned(),
            json!({ "auth": { "OPENAI_API_KEY": "" }, "config": no_credential }),
            None,
        );
        assert!(analyze_codex_provider_features(&provider, false).applicable);

        let official_url = feature_config("").replace(
            "https://gateway.example.test/v1",
            "https://api.openai.com/v1",
        );
        let provider = feature_provider(&official_url, Some("openai_responses"), Some(false));
        assert!(analyze_codex_provider_features(&provider, false).applicable);

        let mut provider =
            feature_provider(&feature_config(""), Some("openai_responses"), Some(false));
        provider.category = Some("official".to_owned());
        let official_state = analyze_codex_provider_features(&provider, false);
        assert!(official_state.applicable);
        assert!(matches!(
            official_state.image_extension,
            CodexImageExtensionState::Off
        ));

        let reserved_config = feature_config(&format!(
            "supports_websockets = true\nhttp_headers = {{ \"{CODEX_IMAGE_EXTENSION_HEADER}\" = \"{CODEX_IMAGE_EXTENSION_VALUE}\" }}\n"
        ))
            .replace("model_provider = \"fixture\"", "model_provider = \"OpenAI\"")
            .replace("[model_providers.fixture]", "[model_providers.OpenAI]");
        let reserved = feature_provider(&reserved_config, Some("openai_responses"), None);
        let reserved_state = analyze_codex_provider_features(&reserved, false);
        assert!(reserved_state.applicable);
        assert!(matches!(
            reserved_state.image_extension,
            CodexImageExtensionState::On
        ));
        assert!(reserved_state.websockets.enabled);

        let mut managed = feature_provider(&feature_config(""), Some("openai_responses"), None);
        managed
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .provider_type = Some("codex_oauth".to_owned());
        assert!(analyze_codex_provider_features(&managed, false).applicable);
    }

    #[test]
    fn official_provider_delays_generation_and_only_removes_an_owned_empty_skeleton() {
        let mut official = feature_provider("", Some("openai_responses"), None);
        official.id = crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_owned();
        official.category = Some("official".to_owned());

        let initial = analyze_codex_provider_features(&official, false);
        assert!(initial.applicable);
        assert!(!initial.provider_table_found);
        assert!(matches!(
            initial.image_extension,
            CodexImageExtensionState::Off
        ));
        assert!(!initial.websockets.enabled);
        assert_eq!(codex_provider_config_text(&official), "");

        let enabled = patch_codex_provider_features(
            &official,
            &CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: None,
            },
            false,
        )
        .expect("first explicit capability creates the official skeleton");
        assert_eq!(
            enabled.codex_native_capabilities_generated_provider,
            Some(true)
        );
        let enabled_doc: toml::Value =
            toml::from_str(&enabled.toml_text).expect("generated official TOML");
        assert_eq!(
            enabled_doc["model_provider"].as_str(),
            Some(FYAGENT_CODEX_MODEL_PROVIDER_ID)
        );
        let table = &enabled_doc["model_providers"][FYAGENT_CODEX_MODEL_PROVIDER_ID];
        assert_eq!(table["name"].as_str(), Some("OpenAI"));
        assert_eq!(table["requires_openai_auth"].as_bool(), Some(true));
        assert_eq!(table["wire_api"].as_str(), Some("responses"));
        assert_eq!(
            table["http_headers"][CODEX_IMAGE_EXTENSION_HEADER].as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
        );
        assert!(table.get("supports_websockets").is_none());

        set_provider_config_text(&mut official, enabled.toml_text).expect("apply form draft");
        official
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .codex_native_capabilities_generated_provider = Some(true);
        let disabled = patch_codex_provider_features(
            &official,
            &CodexProviderFeatureIntent {
                image_extension: Some(false),
                websockets: None,
            },
            false,
        )
        .expect("last disabled capability removes the owned skeleton");
        assert_eq!(
            disabled.codex_native_capabilities_generated_provider,
            Some(false)
        );
        assert!(disabled.toml_text.trim().is_empty());
    }

    #[test]
    fn official_provider_keeps_a_user_extended_generated_table() {
        let config = r#"model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
requires_openai_auth = true
wire_api = "responses"
supports_websockets = true
user_extension = "keep-me"
"#;
        let mut official = feature_provider(config, Some("openai_responses"), None);
        official.id = crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_owned();
        official.category = Some("official".to_owned());
        official
            .meta
            .get_or_insert_with(ProviderMeta::default)
            .codex_native_capabilities_generated_provider = Some(true);

        let disabled = patch_codex_provider_features(
            &official,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(false),
            },
            false,
        )
        .expect("disable WebSocket without deleting user extensions");
        let doc: toml::Value = toml::from_str(&disabled.toml_text).expect("patched TOML");
        assert_eq!(doc["model_provider"].as_str(), Some("custom"));
        let table = &doc["model_providers"]["custom"];
        assert_eq!(table["user_extension"].as_str(), Some("keep-me"));
        assert!(table.get("supports_websockets").is_none());
        assert_eq!(disabled.codex_native_capabilities_generated_provider, None);
    }

    #[test]
    fn official_provider_never_claims_or_removes_a_preexisting_custom_table() {
        let config = r#"[model_providers.custom]
name = "OpenAI"
requires_openai_auth = true
wire_api = "responses"
"#;
        let mut official = feature_provider(config, Some("openai_responses"), None);
        official.id = crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_owned();
        official.category = Some("official".to_owned());

        let enabled = patch_codex_provider_features(
            &official,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(true),
            },
            false,
        )
        .expect("reuse the user's existing custom table");
        assert_eq!(enabled.codex_native_capabilities_generated_provider, None);

        set_provider_config_text(&mut official, enabled.toml_text).expect("apply form draft");
        let disabled = patch_codex_provider_features(
            &official,
            &CodexProviderFeatureIntent {
                image_extension: None,
                websockets: Some(false),
            },
            false,
        )
        .expect("disable WebSocket without claiming the user's table");
        assert_eq!(disabled.codex_native_capabilities_generated_provider, None);
        let doc: toml::Value = toml::from_str(&disabled.toml_text).expect("patched TOML");
        assert_eq!(doc["model_provider"].as_str(), Some("custom"));
        assert_eq!(
            doc["model_providers"]["custom"]["name"].as_str(),
            Some("OpenAI")
        );
        assert!(doc["model_providers"]["custom"]
            .get("supports_websockets")
            .is_none());
    }

    #[test]
    fn websocket_save_warning_classifier_covers_models_catalog_and_proxy_risks() {
        let websocket_config =
            |models: &str| format!("{models}{}", feature_config("supports_websockets = true\n"));

        let gpt = feature_provider(
            &websocket_config("model = \"gpt-5.6-sol\"\nreview_model = \"openai/GPT-5.6-sol\"\n"),
            Some("openai_responses"),
            Some(true),
        );
        assert!(codex_provider_save_warning_codes(&gpt, false).is_empty());

        let non_gpt = feature_provider(
            &websocket_config("model = \"grok-4.5\"\n"),
            Some("openai_chat"),
            Some(true),
        );
        assert_eq!(
            codex_provider_save_warning_codes(&non_gpt, false),
            vec![CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING]
        );

        let empty_basename = feature_provider(
            &websocket_config("model = \"vendor/\"\n"),
            Some("openai_responses"),
            Some(true),
        );
        assert_eq!(
            codex_provider_save_warning_codes(&empty_basename, false),
            vec![CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING],
            "only an empty original model string is ignored"
        );

        let mut mixed_catalog = feature_provider(
            &websocket_config("model = \"vendor/gpt-5.6-sol\"\n"),
            Some("anthropic"),
            Some(true),
        );
        mixed_catalog.settings_config["modelCatalog"] = json!({
            "models": [
                { "model": "openai/GPT-5.6-sol" },
                { "model": "vendor/qwen3" },
                { "model": "" }
            ]
        });
        assert_eq!(
            codex_provider_save_warning_codes(&mixed_catalog, false),
            vec![CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING]
        );

        let no_models = feature_provider(
            &feature_config("supports_websockets = true\n"),
            Some("openai_responses"),
            Some(true),
        );
        assert!(codex_provider_save_warning_codes(&no_models, false).is_empty());
        assert_eq!(
            codex_provider_save_warning_codes(&no_models, true),
            vec![CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED_WARNING]
        );
        assert_eq!(
            codex_provider_save_warning_codes(&non_gpt, true),
            vec![
                CODEX_WEBSOCKET_NON_GPT_MODEL_WARNING,
                CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED_WARNING,
            ]
        );

        let websocket_off = feature_provider(
            &websocket_config("model = \"grok-4.5\"\n")
                .replace("supports_websockets = true", "# WebSocket disabled"),
            Some("openai_responses"),
            Some(true),
        );
        assert!(codex_provider_save_warning_codes(&websocket_off, true).is_empty());
    }

    #[test]
    fn catalog_tool_profile_from_api_format() {
        assert_eq!(
            CodexCatalogToolProfile::from_api_format(Some("anthropic")),
            CodexCatalogToolProfile::Anthropic
        );
        assert_eq!(
            CodexCatalogToolProfile::from_api_format(Some("openai_responses")),
            CodexCatalogToolProfile::NativeResponses
        );
        assert_eq!(
            CodexCatalogToolProfile::from_api_format(Some("openai_chat")),
            CodexCatalogToolProfile::ProxyChat
        );
        assert_eq!(
            CodexCatalogToolProfile::from_api_format(None),
            CodexCatalogToolProfile::ProxyChat
        );
    }

    #[test]
    fn unified_session_bucket_injects_for_empty_official_config() {
        let injected = inject_codex_unified_session_bucket("").expect("inject");
        let doc: toml::Table = toml::from_str(&injected).expect("parse injected config");

        assert_eq!(
            doc.get("model_provider").and_then(|v| v.as_str()),
            Some(FYAGENT_CODEX_MODEL_PROVIDER_ID)
        );
        let custom = doc["model_providers"][FYAGENT_CODEX_MODEL_PROVIDER_ID]
            .as_table()
            .expect("custom provider table");
        assert_eq!(custom.get("name").and_then(|v| v.as_str()), Some("OpenAI"));
        assert_eq!(
            custom.get("requires_openai_auth").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            custom.get("supports_websockets").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            custom.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );
    }

    #[test]
    fn official_proxy_route_uses_native_auth_and_local_responses_provider() {
        let input = r#"model = "gpt-5.4"
experimental_bearer_token = "PROXY_MANAGED"

[mcp_servers.example]
command = "example"
"#;
        let output = apply_codex_official_proxy_route(input, "http://127.0.0.1:15721/v1")
            .expect("apply official proxy route");
        let doc: toml::Value = toml::from_str(&output).expect("parse output");

        assert_eq!(
            doc.get("model_provider").and_then(toml::Value::as_str),
            Some(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID)
        );
        assert!(doc.get("experimental_bearer_token").is_none());
        assert!(
            doc.get("mcp_servers").is_some(),
            "unrelated config survives"
        );

        let provider = &doc["model_providers"][FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID];
        assert_eq!(
            provider.get("base_url").and_then(toml::Value::as_str),
            Some("http://127.0.0.1:15721/v1")
        );
        assert_eq!(
            provider
                .get("requires_openai_auth")
                .and_then(toml::Value::as_bool),
            Some(true)
        );
        assert!(provider.get("supports_websockets").is_none());
        assert!(codex_config_has_official_proxy_route(&output));
    }

    #[test]
    fn official_proxy_route_preserves_explicit_native_capabilities() {
        let input = format!(
            r#"model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
requires_openai_auth = true
wire_api = "responses"
supports_websockets = true
http_headers = {{ "{CODEX_IMAGE_EXTENSION_HEADER}" = "{CODEX_IMAGE_EXTENSION_VALUE}", "X-Company-ID" = "not-projected" }}
"#
        );
        let output = apply_codex_official_proxy_route(&input, "http://127.0.0.1:15721/v1")
            .expect("apply official proxy route");
        let doc: toml::Value = toml::from_str(&output).expect("parse output");
        let provider = &doc["model_providers"][FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID];

        assert_eq!(provider["supports_websockets"].as_bool(), Some(true));
        assert_eq!(
            provider["http_headers"][CODEX_IMAGE_EXTENSION_HEADER].as_str(),
            Some(CODEX_IMAGE_EXTENSION_VALUE)
        );
        assert!(provider["http_headers"].get("X-Company-ID").is_none());
    }

    #[test]
    fn official_proxy_route_cleanup_only_removes_owned_provider() {
        let projected =
            apply_codex_official_proxy_route("model = \"gpt-5.4\"\n", "http://127.0.0.1:15721/v1")
                .expect("project");
        let cleaned = remove_codex_official_proxy_route(&projected).expect("clean");
        let doc: toml::Value = toml::from_str(&cleaned).expect("parse cleaned");
        assert!(doc.get("model_provider").is_none());
        assert!(doc.get("model_providers").is_none());
        assert_eq!(
            doc.get("model").and_then(toml::Value::as_str),
            Some("gpt-5.4")
        );
    }

    #[test]
    fn official_proxy_route_rejects_non_table_model_providers_without_panicking() {
        for input in [
            "model_providers = 3\n",
            "[[model_providers]]\nname = \"broken\"\n",
        ] {
            let result = apply_codex_official_proxy_route(input, "http://127.0.0.1:15721/v1");
            assert!(result.is_err());
        }
    }

    #[test]
    fn official_proxy_route_normalizes_inline_tables_and_cleans_stale_placeholder() {
        let input = r#"model_provider = "rightcode"
model_providers = { rightcode = { name = "RightCode", experimental_bearer_token = "PROXY_MANAGED" } }
"#;
        let projected = apply_codex_official_proxy_route(input, "http://127.0.0.1:15721/v1")
            .expect("project inline provider table");
        let projected_doc: toml::Value = toml::from_str(&projected).expect("parse projected");
        assert!(projected_doc["model_providers"]["rightcode"]
            .get("experimental_bearer_token")
            .is_none());
        assert!(projected_doc["model_providers"]
            .get(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID)
            .is_some());

        let cleaned = remove_codex_official_proxy_route(&projected).expect("clean projected");
        let cleaned_doc: toml::Value = toml::from_str(&cleaned).expect("parse cleaned");
        assert!(cleaned_doc.get("model_provider").is_none());
        assert!(cleaned_doc["model_providers"].get("rightcode").is_some());
        assert!(cleaned_doc["model_providers"]
            .get(FYAGENT_CODEX_OFFICIAL_PROXY_PROVIDER_ID)
            .is_none());
    }

    #[test]
    fn unified_session_bucket_preserves_other_keys_and_explicit_routing() {
        let with_catalog = "model_catalog_json = \"fyagent-model-catalog.json\"\n";
        let injected = inject_codex_unified_session_bucket(with_catalog).expect("inject");
        assert!(injected.contains("model_catalog_json"));
        assert!(injected.contains("model_provider = \"custom\""));

        // 用户显式指定过 model_provider 的官方配置不被覆盖
        let explicit = "model_provider = \"openai_https\"\n";
        let unchanged = inject_codex_unified_session_bucket(explicit).expect("inject");
        assert_eq!(unchanged, explicit);
    }

    #[test]
    fn unified_session_bucket_skips_conflicting_custom_table() {
        // 残留的非注入形态 custom 表：设置 model_provider 会把官方流量
        // 路由到表里的第三方端点，必须整体拒绝注入。
        let stale = r#"[model_providers.custom]
name = "Relay"
base_url = "https://relay.example/v1"
"#;
        let unchanged = inject_codex_unified_session_bucket(stale).expect("inject");
        assert_eq!(unchanged, stale);

        // 已是注入形态的 custom 表（如重复注入）则照常补上 model_provider
        let injected_once = inject_codex_unified_session_bucket("").expect("inject");
        let reinjected = inject_codex_unified_session_bucket(&injected_once).expect("re-inject");
        assert_eq!(reinjected, injected_once);
    }

    #[test]
    fn unified_session_bucket_strip_round_trips_injection() {
        let injected = inject_codex_unified_session_bucket("").expect("inject");
        let stripped = strip_codex_unified_session_bucket(&injected).expect("strip");
        assert_eq!(stripped.trim(), "");

        let with_catalog = "model_catalog_json = \"fyagent-model-catalog.json\"\n";
        let injected = inject_codex_unified_session_bucket(with_catalog).expect("inject");
        let stripped = strip_codex_unified_session_bucket(&injected).expect("strip");
        assert_eq!(stripped, with_catalog);
    }

    #[test]
    fn unified_session_bucket_strip_keeps_third_party_custom_entry() {
        // 第三方模板同样用 custom 路由，但条目带 base_url 等差异字段，
        // 形态不等于注入产物，必须原样保留。
        let third_party = r#"model_provider = "custom"

[model_providers.custom]
name = "Relay"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
        let untouched = strip_codex_unified_session_bucket(third_party).expect("strip");
        assert_eq!(untouched, third_party);
    }

    #[test]
    fn unified_session_bucket_strip_from_settings_only_touches_config() {
        let injected = inject_codex_unified_session_bucket("").expect("inject");
        let mut settings = json!({
            "auth": { "tokens": { "access_token": "secret" } },
            "config": injected,
        });
        strip_codex_unified_session_bucket_from_settings(&mut settings).expect("strip settings");
        assert_eq!(
            settings
                .get("config")
                .and_then(|v| v.as_str())
                .map(str::trim),
            Some("")
        );
        assert!(settings.pointer("/auth/tokens/access_token").is_some());
    }

    #[test]
    fn strip_mcp_servers_from_settings_removes_table_and_legacy_form() {
        let mut settings = json!({
            "auth": { "OPENAI_API_KEY": "sk-test" },
            "config": "# user comment\nmodel = \"gpt-5.5\"\n\n[mcp_servers.echo]\ntype = \"stdio\"\ncommand = \"echo\"\n\n[mcp.servers.legacy]\ncommand = \"noop\"\n",
        });
        strip_codex_mcp_servers_from_settings(&mut settings).expect("strip mcp");
        let config = settings
            .get("config")
            .and_then(|v| v.as_str())
            .expect("config text");
        assert!(!config.contains("mcp_servers"), "got: {config}");
        assert!(
            !config.contains("[mcp"),
            "legacy [mcp.servers] gone: {config}"
        );
        assert!(config.contains("# user comment"), "comments preserved");
        assert!(config.contains("model = \"gpt-5.5\""));
    }

    #[test]
    fn strip_mcp_servers_from_settings_is_noop_without_mcp() {
        let original = "# comment\nmodel = \"gpt-5.5\"\n";
        let mut settings = json!({
            "auth": {},
            "config": original,
        });
        strip_codex_mcp_servers_from_settings(&mut settings).expect("strip mcp");
        assert_eq!(
            settings.get("config").and_then(|v| v.as_str()),
            Some(original),
            "config text must be byte-identical when nothing is stripped"
        );
    }

    #[test]
    fn extract_base_url_prefers_active_provider_section() {
        let input = r#"model_provider = "azure"

[model_providers.azure]
base_url = "https://azure.example.com/v1"

[model_providers.other]
base_url = "https://other.example.com/v1"
"#;

        assert_eq!(
            extract_codex_base_url(input).as_deref(),
            Some("https://azure.example.com/v1")
        );
    }

    #[test]
    fn extract_base_url_falls_back_to_top_level_only() {
        let top_level = r#"base_url = "https://top-level.example.com/v1""#;
        assert_eq!(
            extract_codex_base_url(top_level).as_deref(),
            Some("https://top-level.example.com/v1")
        );
    }

    // Mirrors the frontend extractCodexBaseUrl: a non-active provider section
    // is never a credential source, whether the active provider points
    // elsewhere (e.g. the built-in "openai") or none is selected at all.
    #[test]
    fn extract_base_url_ignores_non_active_provider_sections() {
        let mismatched = r#"model_provider = "openai"

[model_providers.custom]
base_url = "https://leftover.example.com/v1"
"#;
        assert_eq!(extract_codex_base_url(mismatched), None);

        let no_active = r#"[model_providers.any]
base_url = "https://single.example.com/v1"
"#;
        assert_eq!(extract_codex_base_url(no_active), None);
    }

    #[test]
    fn prepare_provider_live_config_rejects_key_without_config() {
        let err = prepare_codex_provider_live_config(&json!({"OPENAI_API_KEY": "sk-test"}), "")
            .expect_err("empty config with API key should not truncate live config");

        assert!(
            err.to_string().contains("config.toml"),
            "error should explain missing config.toml, got: {err}"
        );
    }

    #[test]
    fn prepare_provider_live_config_uses_top_level_token_for_reserved_provider() {
        let input = r#"model_provider = "openai"
model = "gpt-5"
"#;

        let output =
            prepare_codex_provider_live_config(&json!({"OPENAI_API_KEY": "sk-test"}), input)
                .expect("prepare live config");
        let parsed: toml::Value = toml::from_str(&output).expect("parse output");

        assert_eq!(
            parsed
                .get("experimental_bearer_token")
                .and_then(|v| v.as_str()),
            Some("sk-test")
        );
        assert!(
            parsed.get("model_providers").is_none(),
            "reserved provider tables should not be synthesized"
        );
    }

    #[test]
    fn active_provider_disables_openai_auth_only_for_explicit_false() {
        let disabled = r#"model_provider = "custom"

[model_providers.custom]
requires_openai_auth = false
"#;
        let enabled = r#"model_provider = "custom"

[model_providers.custom]
requires_openai_auth = true
"#;
        let missing = r#"model_provider = "custom"

[model_providers.custom]
name = "Gateway"
"#;
        assert!(active_codex_provider_disables_openai_auth(disabled));
        assert!(!active_codex_provider_disables_openai_auth(enabled));
        assert!(!active_codex_provider_disables_openai_auth(missing));
        assert!(!active_codex_provider_disables_openai_auth("not toml {"));
    }

    #[test]
    fn project_live_config_injects_bearer_token_only_when_openai_auth_is_disabled() {
        let disabled = r#"model_provider = "custom"
model = "gpt-5.4"

[model_providers.custom]
name = "Gateway"
base_url = "https://gateway.example/v1"
wire_api = "responses"
requires_openai_auth = false
"#;
        let enabled = disabled.replace(
            "requires_openai_auth = false",
            "requires_openai_auth = true",
        );
        let auth = json!({ "OPENAI_API_KEY": "sk-image" });

        let projected =
            project_codex_live_config_when_openai_auth_disabled(&auth, disabled).expect("project");
        let parsed: toml::Value = toml::from_str(&projected).expect("parse projected");
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("custom"))
                .and_then(|v| v.get("experimental_bearer_token"))
                .and_then(|v| v.as_str()),
            Some("sk-image")
        );

        let unchanged =
            project_codex_live_config_when_openai_auth_disabled(&auth, &enabled).expect("keep");
        assert_eq!(unchanged, enabled);
        assert!(
            !unchanged.contains("experimental_bearer_token"),
            "requires_openai_auth=true must keep the API key in auth.json only"
        );
    }

    #[test]
    fn extract_bearer_uses_top_level_token_for_reserved_provider() {
        let input = r#"model_provider = "openai"
experimental_bearer_token = "top-level-key"

[model_providers.openai]
experimental_bearer_token = "stale-table-key"
"#;

        assert_eq!(
            extract_codex_experimental_bearer_token(input).as_deref(),
            Some("top-level-key")
        );
    }

    #[test]
    fn should_not_restore_provider_token_for_oauth_only_template() {
        let oauth_template = json!({
            "auth": {
                "auth_mode": "chatgpt",
                "tokens": {
                    "access_token": "oauth-access"
                }
            }
        });
        let api_key_template = json!({
            "auth": {
                "OPENAI_API_KEY": "sk-test"
            }
        });

        assert!(
            !should_restore_codex_provider_token_for_backfill(Some("custom"), &oauth_template),
            "OAuth-only templates should not backfill bearer tokens into OPENAI_API_KEY"
        );
        assert!(
            should_restore_codex_provider_token_for_backfill(Some("custom"), &api_key_template),
            "custom API-key providers should still restore provider bearer tokens"
        );
        assert!(
            !should_restore_codex_provider_token_for_backfill(Some("official"), &api_key_template),
            "official providers should never restore third-party bearer tokens"
        );
    }

    #[test]
    fn credential_login_material_only_counts_real_credentials() {
        assert!(codex_auth_has_credential_login_material(&json!({
            "tokens": { "access_token": "t" }
        })));
        assert!(codex_auth_has_credential_login_material(&json!({
            "tokens": { "refresh_token": "r" }
        })));
        assert!(codex_auth_has_credential_login_material(&json!({
            "personal_access_token": "pat"
        })));

        // API key and pure metadata are not credentials in this predicate's
        // sense — they must not shield a stale key from cleanup.
        assert!(!codex_auth_has_credential_login_material(&json!({
            "OPENAI_API_KEY": "sk-x"
        })));
        assert!(!codex_auth_has_credential_login_material(&json!({
            "OPENAI_API_KEY": "sk-x",
            "last_refresh": "2026-01-01T00:00:00Z",
            "tokens": { "account_id": "acct-meta-only" }
        })));
        assert!(!codex_auth_has_credential_login_material(&json!({})));
    }

    #[test]
    fn stale_third_party_residue_detection() {
        // Shapes a preserve-off third-party switch leaves behind: cleared.
        assert!(codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": "sk-third-party"
        })));
        assert!(codex_live_auth_is_stale_third_party_residue(&json!({
            "auth_mode": "apikey",
            "OPENAI_API_KEY": "sk-third-party"
        })));
        assert!(codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": "sk-third-party",
            "last_refresh": "2026-01-01T00:00:00Z",
            "tokens": { "account_id": "acct-meta-only" }
        })));

        // Anything carrying a real credential must survive untouched.
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": "sk-x",
            "tokens": { "access_token": "t" }
        })));
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": { "access_token": "official-oauth-token" }
        })));

        // Nothing to clear.
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({})));
        assert!(!codex_live_auth_is_stale_third_party_residue(&json!({
            "OPENAI_API_KEY": ""
        })));
    }

    #[test]
    fn prepare_provider_live_config_does_not_create_incomplete_provider_table() {
        let input = r#"model_provider = "vendor_x"
model = "gpt-5"
"#;

        let output =
            prepare_codex_provider_live_config(&json!({"OPENAI_API_KEY": "sk-test"}), input)
                .expect("prepare live config");
        let parsed: toml::Value = toml::from_str(&output).expect("parse output");

        assert_eq!(
            parsed
                .get("experimental_bearer_token")
                .and_then(|v| v.as_str()),
            Some("sk-test")
        );
        assert!(
            parsed.get("model_providers").is_none(),
            "missing provider tables should not be synthesized without endpoint fields"
        );
    }

    #[test]
    fn prepare_provider_live_config_preserves_custom_provider_id() {
        let input = r#"model_provider = "vendor_alpha"
model = "gpt-5.4"
profile = "work"

[model_providers.vendor_alpha]
name = "Vendor Alpha"
base_url = "https://alpha.example/v1"
wire_api = "responses"

[profiles.work]
model_provider = "vendor_alpha"
model = "gpt-5.4"
"#;

        let result =
            prepare_codex_provider_live_config(&json!({"OPENAI_API_KEY": "sk-test"}), input)
                .expect("prepare live config");
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("vendor_alpha")
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("custom"))
                .is_none(),
            "provider writes should not force custom provider ids"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("vendor_alpha"))
                .and_then(|v| v.get("experimental_bearer_token"))
                .and_then(|v| v.as_str()),
            Some("sk-test")
        );
        assert_eq!(
            parsed
                .get("profiles")
                .and_then(|v| v.get("work"))
                .and_then(|v| v.get("model_provider"))
                .and_then(|v| v.as_str()),
            Some("vendor_alpha"),
            "profile provider references should be preserved"
        );
    }

    #[test]
    fn backfill_preserves_live_model_provider_id() {
        let mut live_settings = json!({
            "auth": {},
            "config": r#"model_provider = "vendor_beta"

[model_providers.vendor_beta]
name = "Vendor Beta"
base_url = "https://beta.example/v1"
wire_api = "responses"
"#,
        });
        let template_settings = json!({
            "auth": {},
            "config": r#"model_provider = "custom"

[model_providers.custom]
name = "Custom"
base_url = "https://custom.example/v1"
wire_api = "responses"
"#,
        });

        restore_codex_settings_for_backfill(&mut live_settings, &template_settings, false).unwrap();
        let config = live_settings.get("config").and_then(Value::as_str).unwrap();
        let parsed: toml::Value = toml::from_str(config).unwrap();

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("vendor_beta")
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|v| v.get("vendor_beta"))
                .is_some(),
            "backfill should not rewrite user-selected provider tables"
        );
    }

    #[test]
    fn base_url_writes_into_correct_model_provider_section() {
        let input = r#"model_provider = "any"
model = "gpt-5.1-codex"

[model_providers.any]
name = "any"
wire_api = "responses"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://example.com/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .expect("base_url should be in model_providers.any");
        assert_eq!(base_url, "https://example.com/v1");

        // Should NOT have top-level base_url
        assert!(parsed.get("base_url").is_none());

        // wire_api preserved
        let wire_api = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("wire_api"))
            .and_then(|v| v.as_str());
        assert_eq!(wire_api, Some("responses"));
    }

    #[test]
    fn wire_api_writes_into_correct_model_provider_section() {
        let input = r#"model_provider = "chat_only"
model = "gpt-5.1-codex"

[model_providers.chat_only]
name = "Chat Only"
base_url = "https://example.com/v1"
wire_api = "chat"
"#;

        let result = update_codex_toml_field(input, "wire_api", "responses").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let provider = parsed
            .get("model_providers")
            .and_then(|v| v.get("chat_only"))
            .expect("model_providers.chat_only should exist");

        assert_eq!(
            provider.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );
        assert_eq!(
            provider.get("base_url").and_then(|v| v.as_str()),
            Some("https://example.com/v1")
        );
        assert!(parsed.get("wire_api").is_none());
    }

    #[test]
    fn base_url_creates_section_when_missing() {
        let input = r#"model_provider = "custom"
model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://custom.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .expect("should create section and set base_url");
        assert_eq!(base_url, "https://custom.api/v1");
    }

    #[test]
    fn base_url_falls_back_to_top_level_without_model_provider() {
        let input = r#"model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://fallback.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("base_url")
            .and_then(|v| v.as_str())
            .expect("should set top-level base_url");
        assert_eq!(base_url, "https://fallback.api/v1");
    }

    #[test]
    fn base_url_writes_into_inline_table_provider_section() {
        // inline table 是合法 TOML，但 as_table_mut() 对它返回 None。旧代码会因此
        // 掉进「写顶层字段」的 fallback：用户改的 base_url 落在错误层级，
        // Codex 读不到，且界面毫无提示。
        let input = r#"model_provider = "any"
model_providers = { any = { name = "any", base_url = "https://old.api/v1", wire_api = "responses" } }
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        assert_eq!(
            parsed["model_providers"]["any"]["base_url"].as_str(),
            Some("https://new.api/v1"),
            "must update the provider section, not a top-level field"
        );
        assert!(
            parsed.get("base_url").is_none(),
            "must not leak a top-level base_url fallback"
        );
        assert_eq!(
            parsed["model_providers"]["any"]["wire_api"].as_str(),
            Some("responses"),
            "sibling fields must survive"
        );
    }

    #[test]
    fn clearing_base_url_removes_only_from_correct_section() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "https://old.api/v1"
wire_api = "responses"

[mcp_servers.context7]
command = "npx"
"#;

        let result = update_codex_toml_field(input, "base_url", "").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        // base_url removed from model_providers.any
        let any_section = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .expect("model_providers.any should exist");
        assert!(any_section.get("base_url").is_none());

        // wire_api preserved
        assert_eq!(
            any_section.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );

        // mcp_servers untouched
        assert!(parsed.get("mcp_servers").is_some());
    }

    #[test]
    fn model_field_operates_on_top_level() {
        let input = r#"model_provider = "any"
model = "gpt-4"

[model_providers.any]
name = "any"
"#;

        let result = update_codex_toml_field(input, "model", "gpt-5").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(parsed.get("model").and_then(|v| v.as_str()), Some("gpt-5"));

        // Clear model
        let result2 = update_codex_toml_field(&result, "model", "").unwrap();
        let parsed2: toml::Value = toml::from_str(&result2).unwrap();
        assert!(parsed2.get("model").is_none());
    }

    #[test]
    fn preserves_comments_and_whitespace() {
        let input = r#"# My Codex config
model_provider = "any"
model = "gpt-4"

# Provider section
[model_providers.any]
name = "any"
base_url = "https://old.api/v1"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();

        // Comments should be preserved
        assert!(result.contains("# My Codex config"));
        assert!(result.contains("# Provider section"));
    }

    #[test]
    fn does_not_misplace_when_profiles_section_follows() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "https://old.api/v1"

[profiles.default]
model = "gpt-4"
"#;

        let result = update_codex_toml_field(input, "base_url", "https://new.api/v1").unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        // base_url in correct section
        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str());
        assert_eq!(base_url, Some("https://new.api/v1"));

        // profiles section untouched
        let profile_model = parsed
            .get("profiles")
            .and_then(|v| v.get("default"))
            .and_then(|v| v.get("model"))
            .and_then(|v| v.as_str());
        assert_eq!(profile_model, Some("gpt-4"));
    }

    #[test]
    fn remove_base_url_if_predicate() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
base_url = "http://127.0.0.1:5000/v1"
wire_api = "responses"
"#;

        let result =
            remove_codex_toml_base_url_if(input, |url| url.starts_with("http://127.0.0.1"));
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let any_section = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .unwrap();
        assert!(any_section.get("base_url").is_none());
        assert_eq!(
            any_section.get("wire_api").and_then(|v| v.as_str()),
            Some("responses")
        );
    }

    #[test]
    fn remove_base_url_if_keeps_non_matching() {
        let input = r#"model_provider = "any"

[model_providers.any]
base_url = "https://production.api/v1"
"#;

        let result =
            remove_codex_toml_base_url_if(input, |url| url.starts_with("http://127.0.0.1"));
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let base_url = parsed
            .get("model_providers")
            .and_then(|v| v.get("any"))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str());
        assert_eq!(base_url, Some("https://production.api/v1"));
    }

    #[test]
    fn dynamic_template_backfills_parser_required_fields_from_static() {
        // Simulate a template cloned from a models_cache.json written by a
        // Codex build whose ModelInfo lacks parser-side required fields such
        // as `supports_reasoning_summaries` (codex >= 0.144.5 rejects the
        // whole catalog file without it).
        let mut template = json!({
            "slug": "gpt-5.5",
            "context_window": 272_000,
            "supports_parallel_tool_calls": false
        });
        fill_template_fields_from_static(&mut template);

        assert_eq!(
            template
                .get("supports_reasoning_summaries")
                .and_then(Value::as_bool),
            Some(true)
        );
        // Keys already present in the dynamic template are never overwritten.
        assert_eq!(
            template
                .get("supports_parallel_tool_calls")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            template.get("context_window").and_then(Value::as_u64),
            Some(272_000)
        );
        // Optional capability fields must NOT be backfilled: for the catalog
        // parser "missing" means the parser default, not the static
        // template's value.
        assert!(template.get("supports_search_tool").is_none());
        assert!(template.get("supports_image_detail_original").is_none());
        assert!(template.get("web_search_tool_type").is_none());
    }

    #[test]
    fn proxy_chat_catalog_entries_carry_reasoning_summaries_flag() {
        // End to end: a stale dynamic template, once backfilled, must yield
        // catalog entries codex 0.144.5+ can parse.
        let mut template = json!({ "slug": "gpt-5.5" });
        fill_template_fields_from_static(&mut template);
        let specs = vec![CodexCatalogModelSpec {
            model: "k3".to_string(),
            display_name: Some("Kimi K3".to_string()),
            context_window: Some(262_144),
            supports_parallel_tool_calls: None,
            input_modalities: None,
            base_instructions: None,
        }];
        let catalog = codex_model_catalog_from_specs(
            &specs,
            &template,
            CodexCatalogToolProfile::ProxyChat,
            128_000,
        );
        assert_eq!(
            catalog["models"][0]
                .get("supports_reasoning_summaries")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn codex_model_catalog_uses_provider_models_and_context() {
        let template = json!({
            "slug": "gpt-5.5",
            "display_name": "GPT-5.5",
            "description": "Frontier model",
            "base_instructions": "gpt-5.5 base instructions",
            "model_messages": {
                "instructions_template": "gpt-5.5 instructions template",
                "instructions_variables": {
                    "personality_default": "",
                    "personality_friendly": "",
                    "personality_pragmatic": ""
                }
            },
            "additional_speed_tiers": ["fast"],
            "service_tiers": [
                {
                    "id": "priority",
                    "name": "Fast",
                    "description": "1.5x speed, increased usage"
                }
            ],
            "availability_nux": {
                "message": "GPT-5.5 is now available."
            },
            "upgrade": {
                "target": "gpt-5.5"
            },
            "context_window": 272000,
            "max_context_window": 272000
        });
        let settings = json!({
            "modelCatalog": {
                "models": [
                    {
                        "model": "deepseek-v4-flash",
                        "displayName": "DeepSeek V4 Flash",
                        "contextWindow": "64000"
                    },
                    {
                        "model": "kimi-k2",
                        "display_name": "Kimi K2"
                    }
                ]
            }
        });
        let specs = codex_catalog_model_specs(&settings);
        let catalog = codex_model_catalog_from_specs(
            &specs,
            &template,
            CodexCatalogToolProfile::ProxyChat,
            128_000,
        );
        let models = catalog
            .get("models")
            .and_then(|value| value.as_array())
            .expect("models should be an array");

        assert_eq!(models.len(), 2);
        assert_eq!(
            models[0].get("slug").and_then(|value| value.as_str()),
            Some("deepseek-v4-flash")
        );
        assert_eq!(
            models[0]
                .get("context_window")
                .and_then(|value| value.as_u64()),
            Some(64_000)
        );
        assert_eq!(
            models[1]
                .get("context_window")
                .and_then(|value| value.as_u64()),
            Some(128_000)
        );
        assert!(
            models[0].get("model_messages").is_some(),
            "Codex requires model_messages in custom catalogs"
        );
        assert_eq!(
            models[0]
                .get("base_instructions")
                .and_then(|value| value.as_str()),
            Some("gpt-5.5 base instructions")
        );
        assert_eq!(
            models[0].get("model_messages"),
            template.get("model_messages"),
            "custom catalog entries should keep the gpt-5.5 agent template"
        );
        assert_eq!(
            models[0].get("additional_speed_tiers"),
            Some(&json!([])),
            "generated third-party entries should not inherit OpenAI speed tiers"
        );
        assert!(
            models[0]
                .get("availability_nux")
                .is_some_and(|value| value.is_null()),
            "generated third-party entries should not inherit GPT-5.5 launch messaging"
        );
    }

    #[test]
    fn native_responses_profile_suppresses_apply_patch_and_keeps_shell() {
        // Native (direct) /responses providers must NOT emit a freeform
        // apply_patch (type=="custom") tool — gateways like MiMo reject it.
        // The native profile uses the bundled clean template and relies on
        // shell_type="shell_command" for edits, plus per-row overrides.
        let settings = json!({
            "modelCatalog": {
                "models": [
                    {
                        "model": "MiniMax-M3",
                        "displayName": "MiniMax-M3",
                        "contextWindow": 1_000_000,
                        "supportsParallelToolCalls": true,
                        "inputModalities": ["text", "image"],
                        "baseInstructions": "You are Codex, a coding agent based on MiniMax-M3."
                    }
                ]
            }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            "",
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("native catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let entry = &catalog["models"][0];
        assert_eq!(
            entry.get("slug").and_then(|v| v.as_str()),
            Some("MiniMax-M3")
        );
        assert_eq!(
            entry.get("shell_type").and_then(|v| v.as_str()),
            Some("shell_command"),
            "native entries edit via shell, not the custom apply_patch tool"
        );
        assert!(
            entry.get("apply_patch_tool_type").is_none(),
            "native entries must NOT declare a freeform apply_patch tool"
        );
        // `base_instructions` is REQUIRED by Codex's catalog parser, so it must
        // be present — and the per-row official override must win over the
        // template default.
        assert_eq!(
            entry.get("base_instructions").and_then(|v| v.as_str()),
            Some("You are Codex, a coding agent based on MiniMax-M3."),
            "per-row baseInstructions override must apply (and field must exist)"
        );
        assert!(
            entry.get("model_messages").is_none(),
            "native entries must not carry the gpt-5.5 model_messages persona text"
        );
        assert_eq!(
            entry.get("supports_parallel_tool_calls"),
            Some(&json!(true)),
            "per-row supportsParallelToolCalls override must apply"
        );
        assert_eq!(
            entry.get("input_modalities"),
            Some(&json!(["text", "image"])),
            "per-row inputModalities override must apply"
        );
        assert_eq!(
            entry.get("context_window").and_then(|v| v.as_u64()),
            Some(1_000_000)
        );
    }

    #[test]
    fn catalog_infers_image_input_independently_of_tool_profile() {
        // Start from a deliberately text-only template to prove that every
        // profile overwrites template defaults with shared capability logic.
        let template = json!({
            "input_modalities": ["text"],
            "apply_patch_tool_type": "freeform"
        });
        let specs = vec![
            CodexCatalogModelSpec {
                model: "gpt-5.4".to_string(),
                display_name: Some("GPT 5.4".to_string()),
                context_window: Some(128_000),
                supports_parallel_tool_calls: None,
                input_modalities: None,
                base_instructions: None,
            },
            CodexCatalogModelSpec {
                model: "deepseek/deepseek-v4-pro".to_string(),
                display_name: Some("DeepSeek V4 Pro".to_string()),
                context_window: Some(128_000),
                supports_parallel_tool_calls: None,
                input_modalities: None,
                base_instructions: None,
            },
            CodexCatalogModelSpec {
                model: "glm-5.2v".to_string(),
                display_name: Some("GLM 5.2V".to_string()),
                context_window: Some(128_000),
                supports_parallel_tool_calls: None,
                input_modalities: None,
                base_instructions: None,
            },
            CodexCatalogModelSpec {
                model: "deepseek-v4-flash".to_string(),
                display_name: Some("Explicit Visual Override".to_string()),
                context_window: Some(128_000),
                supports_parallel_tool_calls: None,
                input_modalities: Some(vec!["text".to_string(), "image".to_string()]),
                base_instructions: None,
            },
            CodexCatalogModelSpec {
                model: "custom-text-alias".to_string(),
                display_name: Some("Explicit Text Override".to_string()),
                context_window: Some(128_000),
                supports_parallel_tool_calls: None,
                input_modalities: Some(vec!["text".to_string()]),
                base_instructions: None,
            },
        ];

        for profile in [
            CodexCatalogToolProfile::ProxyChat,
            CodexCatalogToolProfile::NativeResponses,
            CodexCatalogToolProfile::Anthropic,
        ] {
            let catalog = codex_model_catalog_from_specs(&specs, &template, profile, 128_000);
            let models = catalog["models"].as_array().expect("models array");
            let modalities = |slug: &str| {
                models
                    .iter()
                    .find(|entry| entry["slug"] == slug)
                    .and_then(|entry| entry.get("input_modalities"))
                    .cloned()
                    .unwrap_or(Value::Null)
            };

            assert_eq!(modalities("gpt-5.4"), json!(["text", "image"]));
            assert_eq!(modalities("deepseek/deepseek-v4-pro"), json!(["text"]));
            assert_eq!(modalities("glm-5.2v"), json!(["text", "image"]));
            assert_eq!(
                modalities("deepseek-v4-flash"),
                json!(["text", "image"]),
                "explicit provider metadata must override the text-only registry"
            );
            assert_eq!(modalities("custom-text-alias"), json!(["text"]));
        }
    }

    #[test]
    fn native_responses_catalog_always_carries_base_instructions() {
        // Regression guard for the "missing field `base_instructions`" parse
        // error: Codex refuses to load a model catalog whose entries lack
        // base_instructions. Synthesized presets carry no per-row override, so
        // the entry MUST inherit the template's neutral default rather than
        // dropping the field entirely.
        let settings = json!({
            "modelCatalog": { "models": [{ "model": "qwen3-coder-plus" }] }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            "",
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("native catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let base = catalog["models"][0]
            .get("base_instructions")
            .and_then(|v| v.as_str());
        assert!(
            base.is_some_and(|s| !s.trim().is_empty()),
            "every native entry must carry a non-empty base_instructions (Codex requires it)"
        );
    }

    const DEEPSEEK_NATIVE_CONFIG: &str = r#"model = "deepseek-v4-flash"
model_provider = "custom"

[model_providers.custom]
name = "deepseek"
base_url = "https://api.deepseek.com"
wire_api = "responses"
"#;

    #[test]
    fn deepseek_host_native_catalog_mirrors_official_entries() {
        // DeepSeek publishes an official Codex models.json (freeform
        // apply_patch + GPT-5 harness + low/high/max reasoning levels). For a
        // deepseek.com native provider the generated catalog must mirror it
        // verbatim instead of the stripped neutral template — the harness
        // tells the model to use apply_patch, so stripping the tool while
        // keeping the harness would be self-inconsistent.
        let settings = json!({
            "modelCatalog": {
                "models": [
                    { "model": "deepseek-v4-flash", "displayName": "DeepSeek V4 Flash" },
                    { "model": "deepseek-v4-pro", "contextWindow": 500_000 }
                ]
            }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("vendor catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let flash = &catalog["models"][0];
        assert_eq!(
            flash.get("slug").and_then(|v| v.as_str()),
            Some("deepseek-v4-flash")
        );
        assert_eq!(
            flash.get("apply_patch_tool_type").and_then(|v| v.as_str()),
            Some("freeform"),
            "official DeepSeek entries keep the freeform apply_patch grant"
        );
        assert!(
            flash
                .get("base_instructions")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.starts_with("You are Codex, an agent based on GPT-5")),
            "official GPT-5 harness must survive verbatim"
        );
        let efforts: Vec<&str> = flash["supported_reasoning_levels"]
            .as_array()
            .expect("official reasoning levels array")
            .iter()
            .filter_map(|level| level.get("effort").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(efforts, vec!["low", "high", "max"]);
        assert_eq!(flash.get("supports_search_tool"), Some(&json!(true)));
        assert_eq!(
            flash.get("web_search_tool_type").and_then(|v| v.as_str()),
            Some("text")
        );
        assert_eq!(
            flash.get("supports_reasoning_summaries"),
            Some(&json!(true))
        );
        assert_eq!(flash.get("input_modalities"), Some(&json!(["text"])));
        assert!(
            flash.get("model_messages").is_some(),
            "official entries are mirrored verbatim, incl. model_messages"
        );
        // No explicit contextWindow on the row: the official 1m window must
        // survive instead of being clobbered by the 128k default.
        assert_eq!(
            flash.get("context_window").and_then(|v| v.as_u64()),
            Some(1_048_576)
        );
        // Explicit user display name still wins over the official one.
        assert_eq!(
            flash.get("display_name").and_then(|v| v.as_str()),
            Some("DeepSeek V4 Flash")
        );

        let pro = &catalog["models"][1];
        assert_eq!(
            pro.get("slug").and_then(|v| v.as_str()),
            Some("deepseek-v4-pro")
        );
        // Explicit user context window override wins…
        assert_eq!(
            pro.get("context_window").and_then(|v| v.as_u64()),
            Some(500_000)
        );
        assert_eq!(
            pro.get("max_context_window").and_then(|v| v.as_u64()),
            Some(500_000)
        );
        // …while the untouched official display name is kept.
        assert_eq!(
            pro.get("display_name").and_then(|v| v.as_str()),
            Some("DeepSeek-V4-Pro")
        );
    }

    #[test]
    fn deepseek_official_catalog_unknown_model_clones_flagship() {
        // A user-added model id the official file doesn't know keeps the
        // gateway's capability profile (clone of the flagship entry) without
        // impersonating it: own slug/name, demoted priority, and the official
        // context window rather than the 128k synthetic default.
        let settings = json!({
            "modelCatalog": { "models": [{ "model": "deepseek-v4-lite" }] }
        });

        let catalog = codex_model_catalog_from_settings(
            &settings,
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses,
        )
        .expect("vendor catalog generation should not error")
        .expect("non-empty modelCatalog must yield a catalog");

        let entry = &catalog["models"][0];
        assert_eq!(
            entry.get("slug").and_then(|v| v.as_str()),
            Some("deepseek-v4-lite")
        );
        assert_eq!(
            entry.get("display_name").and_then(|v| v.as_str()),
            Some("deepseek-v4-lite")
        );
        assert!(
            entry
                .get("priority")
                .and_then(|v| v.as_u64())
                .is_some_and(|p| p >= 1000),
            "clones must sort after official entries"
        );
        assert_eq!(
            entry.get("apply_patch_tool_type").and_then(|v| v.as_str()),
            Some("freeform")
        );
        assert_eq!(
            entry.get("context_window").and_then(|v| v.as_u64()),
            Some(1_048_576),
            "absent contextWindow keeps the flagship's official window"
        );
        assert!(entry
            .get("base_instructions")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty()));
    }

    #[test]
    fn official_vendor_catalog_gated_by_native_profile_and_host() {
        // The official mirror is a capability GRANT, so the gate must be
        // narrow: native `/responses` profile AND the vendor's own host. Chat
        // runs through the proxy converter (gpt-5.5 contract), the Anthropic
        // transform drops custom tools, and aggregators hosting the same
        // model may reject freeform tools — all of them keep their templates.
        assert!(codex_official_vendor_catalog_models(
            DEEPSEEK_NATIVE_CONFIG,
            CodexCatalogToolProfile::NativeResponses
        )
        .is_some_and(|models| !models.is_empty()));

        for profile in [
            CodexCatalogToolProfile::ProxyChat,
            CodexCatalogToolProfile::Anthropic,
        ] {
            assert!(
                codex_official_vendor_catalog_models(DEEPSEEK_NATIVE_CONFIG, profile).is_none(),
                "only the NativeResponses profile may mirror the official catalog"
            );
        }

        let minimax_config = r#"model = "MiniMax-M3"
model_provider = "custom"

[model_providers.custom]
name = "minimax"
base_url = "https://api.minimaxi.com/v1"
wire_api = "responses"
"#;
        assert!(
            codex_official_vendor_catalog_models(
                minimax_config,
                CodexCatalogToolProfile::NativeResponses
            )
            .is_none(),
            "non-DeepSeek native hosts keep the neutral template"
        );

        for trusted_base_url in ["https://deepseek.com/v1", "https://API.DeepSeek.COM./v1"] {
            let config = format!(
                r#"model_provider = "custom"

[model_providers.custom]
base_url = "{trusted_base_url}"
wire_api = "responses"
"#
            );
            assert!(
                codex_official_vendor_catalog_models(
                    &config,
                    CodexCatalogToolProfile::NativeResponses
                )
                .is_some_and(|models| !models.is_empty()),
                "the exact DeepSeek host and its subdomains retain the official catalog: {trusted_base_url}"
            );
        }

        for untrusted_base_url in [
            "http://api.deepseek.com/v1",
            "ftp://api.deepseek.com/v1",
            "https://api.deepseek.com.evil.example/v1",
            "https://notdeepseek.com/v1",
            "https://deepseek.com@evil.example/v1",
            "https://aggregator.example/deepseek.com/v1",
        ] {
            let config = format!(
                r#"model_provider = "custom"

[model_providers.custom]
base_url = "{untrusted_base_url}"
wire_api = "responses"
"#
            );
            assert!(
                codex_official_vendor_catalog_models(
                    &config,
                    CodexCatalogToolProfile::NativeResponses
                )
                .is_none(),
                "untrusted URL must not receive the official DeepSeek capability catalog: {untrusted_base_url}"
            );
        }

        assert!(
            codex_official_vendor_catalog_models("", CodexCatalogToolProfile::NativeResponses)
                .is_none()
        );
    }

    #[test]
    fn proxy_chat_profile_still_keeps_apply_patch() {
        // Regression guard for Mode A: the proxy-chat profile must keep the
        // freeform apply_patch tool (the proxy rewrites custom<->function).
        let template = load_codex_native_responses_template();
        let specs = vec![CodexCatalogModelSpec {
            model: "x".to_string(),
            display_name: Some("x".to_string()),
            context_window: Some(128_000),
            supports_parallel_tool_calls: None,
            input_modalities: None,
            base_instructions: None,
        }];
        // Using a gpt-5.5-shaped template under ProxyChat must NOT strip
        // apply_patch_tool_type. (The native template lacks it, so synthesize
        // one with the field present to prove ProxyChat leaves it intact.)
        let mut proxy_template = template.clone();
        proxy_template["apply_patch_tool_type"] = json!("freeform");
        let catalog = codex_model_catalog_from_specs(
            &specs,
            &proxy_template,
            CodexCatalogToolProfile::ProxyChat,
            128_000,
        );
        assert_eq!(
            catalog["models"][0]
                .get("apply_patch_tool_type")
                .and_then(|v| v.as_str()),
            Some("freeform"),
            "ProxyChat must preserve apply_patch_tool_type (no native stripping)"
        );
    }

    #[test]
    fn model_catalog_json_field_writes_relative_filename() {
        let input = r#"model_provider = "any"

[model_providers.any]
name = "any"
"#;
        let catalog_path = Path::new("/tmp/fyagent-model-catalog.json");

        let result = set_codex_model_catalog_json_field(input, Some(catalog_path)).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed
                .get("model_catalog_json")
                .and_then(|value| value.as_str()),
            Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("any"))
                .and_then(|value| value.get("model_catalog_json"))
                .is_none(),
            "model_catalog_json should stay top-level"
        );
    }

    #[test]
    fn native_web_search_field_disables_at_top_level() {
        // Native `/responses` gateways reject the web_search tool, so the
        // NativeResponses profile must write the top-level disable line even
        // when sections are present (it must NOT land inside a section).
        let input = r#"model_provider = "custom"

[model_providers.custom]
name = "xiaomi_mimo"
"#;
        let result = set_codex_native_web_search_field(input, true).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed.get("web_search").and_then(|value| value.as_str()),
            Some("disabled")
        );
        assert!(
            parsed
                .get("model_providers")
                .and_then(|value| value.get("custom"))
                .and_then(|value| value.get("web_search"))
                .is_none(),
            "web_search should stay top-level"
        );
    }

    #[test]
    fn native_web_search_field_removes_own_sentinel_when_not_disabled() {
        // Switching away from a native provider must re-enable web search by
        // removing fyagent's own "disabled" sentinel.
        let input = r#"model = "gpt-5.5"
web_search = "disabled"
"#;
        let result = set_codex_native_web_search_field(input, false).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed.get("web_search").is_none(),
            "fyagent's disabled sentinel should be removed when not native"
        );
    }

    #[test]
    fn native_web_search_field_preserves_user_value() {
        // A user's own web_search value must never be clobbered by cleanup,
        // only fyagent's "disabled" sentinel is owned/removable.
        let input = r#"web_search = "enabled"
"#;
        let result = set_codex_native_web_search_field(input, false).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed.get("web_search").and_then(|value| value.as_str()),
            Some("enabled"),
            "a user-set web_search value must be preserved"
        );
    }

    #[test]
    fn anthropic_profile_disables_web_search_without_catalog() {
        // Regression: even when no model catalog is generated (empty/absent
        // modelCatalog), an Anthropic provider must still disable web_search — the
        // Responses→Anthropic transform drops the hosted tool, so leaving it on
        // exposes a dead tool. The None-catalog branch previously always left it on.
        let config = "model = \"claude-sonnet-4-6\"\n";
        let settings = serde_json::json!({});

        let anthropic = prepare_codex_config_text_with_model_catalog(
            &settings,
            config,
            CodexCatalogToolProfile::Anthropic,
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&anthropic).unwrap();
        assert_eq!(
            parsed.get("web_search").and_then(|v| v.as_str()),
            Some("disabled"),
            "Anthropic profile must disable web_search even with no catalog"
        );

        // ProxyChat on the same no-catalog path must NOT add a disable line.
        let proxy = prepare_codex_config_text_with_model_catalog(
            &settings,
            config,
            CodexCatalogToolProfile::ProxyChat,
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&proxy).unwrap();
        assert!(
            parsed.get("web_search").is_none(),
            "ProxyChat profile must not disable web_search on the no-catalog path"
        );
    }

    #[test]
    fn web_search_blacklist_disables_only_known_reject_gateways() {
        let cfg = |model: &str, base_url: &str| {
            format!(
                "model_provider = \"custom\"\nmodel = \"{model}\"\n\n[model_providers.custom]\nname = \"x\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n"
            )
        };

        // Blacklisted by host (first-party reject gateways) → disable.
        for (model, host) in [
            ("mimo-v2.5-pro", "https://api.xiaomimimo.com/v1"),
            ("mimo-v2.5", "https://token-plan-cn.xiaomimimo.com/v1"),
            ("LongCat-2.0", "https://api.longcat.chat/openai/v1"),
            ("MiniMax-M3", "https://api.minimax.io/v1"),
            ("MiniMax-M3", "https://api.minimaxi.com/v1"),
        ] {
            assert!(
                codex_native_gateway_rejects_web_search(&cfg(model, host)),
                "{host} should be blacklisted"
            );
        }

        // Blacklisted by MODEL brand even on an aggregator host (SiliconFlow
        // fronting a reject vendor's model) → disable.
        for (model, host) in [
            ("MiniMax-M3", "https://api.siliconflow.cn/v1"),
            ("MiniMaxAI/MiniMax-M3", "https://api.siliconflow.cn/v1"),
            ("mimo-v2.5-pro", "https://some-aggregator.example/v1"),
            (
                "qwen/qwen3-coder-plus",
                "https://some-aggregator.example/v1",
            ),
        ] {
            assert!(
                codex_native_gateway_rejects_web_search(&cfg(model, host)),
                "{model} @ {host} should be blacklisted by model brand"
            );
        }

        // Qwen3-Coder is blacklisted by model, not by DashScope host. This keeps
        // general Qwen models that support built-in web_search on the same host
        // enabled while protecting the native qwen3-coder-plus preset.
        assert!(codex_native_gateway_rejects_web_search(&cfg(
            "qwen3-coder-plus",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
        )));
        assert!(!codex_native_gateway_rejects_web_search(&cfg(
            "qwen3.7-plus",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
        )));

        // NOT blacklisted → keep Codex default (relays/GPT, DouBao, general Qwen,
        // and any unknown provider incl. an aggregator serving a non-reject model).
        for (model, host) in [
            ("gpt-5.5", "https://www.packyapi.com/v1"),
            ("gpt-5-codex", "https://aihubmix.com/v1"),
            (
                "doubao-seed-2-1-pro-260628",
                "https://ark.cn-beijing.volces.com/api/v3",
            ),
            ("Pro/moonshotai/Kimi-K2.6", "https://api.siliconflow.cn/v1"),
        ] {
            assert!(
                !codex_native_gateway_rejects_web_search(&cfg(model, host)),
                "{model} @ {host} should NOT be blacklisted"
            );
        }
    }

    #[test]
    fn resolve_catalog_path_returns_none_when_config_missing_field() {
        let base = PathBuf::from("/tmp/.codex");
        assert!(resolve_fyagent_catalog_path("", &base).is_none());
        assert!(
            resolve_fyagent_catalog_path("model = \"gpt-5\"", &base).is_none(),
            "no model_catalog_json field should yield None"
        );
    }

    #[test]
    fn resolve_catalog_path_accepts_fyagent_owned_file() {
        let base = PathBuf::from("/tmp/.codex");
        let config = r#"model_catalog_json = "/tmp/.codex/fyagent-model-catalog.json"
"#;
        let resolved = resolve_fyagent_catalog_path(config, &base).expect("path resolves");
        assert_eq!(resolved, base.join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME));
    }

    #[test]
    fn resolve_catalog_path_rejects_user_owned_external_file() {
        let base = PathBuf::from("/tmp/.codex");
        let config = r#"model_catalog_json = "/Users/me/.codex/my-handwritten-catalog.json"
"#;
        assert!(
            resolve_fyagent_catalog_path(config, &base).is_none(),
            "external catalog files should be left alone"
        );
    }

    #[test]
    fn build_simplified_catalog_round_trips_user_input() {
        let config = "";
        let catalog = r#"{
            "models": [
                { "slug": "deepseek-v4-pro", "display_name": "deepseek-v4-pro", "context_window": 1000000 },
                { "slug": "deepseek-v4-flash", "display_name": "DeepSeek Flash", "context_window": 1000000 }
            ]
        }"#;
        let result = build_simplified_catalog_from_texts(config, catalog).expect("entries found");
        let models = result
            .get("models")
            .and_then(|m| m.as_array())
            .expect("models array");
        assert_eq!(models.len(), 2);

        // First entry: display_name == slug → displayName squashed; explicit
        // context_window != default 128_000 → preserved.
        assert_eq!(
            models[0].get("model").and_then(|v| v.as_str()),
            Some("deepseek-v4-pro")
        );
        assert!(models[0].get("displayName").is_none());
        assert_eq!(
            models[0].get("contextWindow").and_then(|v| v.as_u64()),
            Some(1_000_000)
        );

        // Second entry: display_name distinct from slug → preserved.
        assert_eq!(
            models[1].get("displayName").and_then(|v| v.as_str()),
            Some("DeepSeek Flash")
        );
    }

    #[test]
    fn build_simplified_catalog_squashes_default_context_window() {
        // Default fallback is 128_000 when config.toml has no model_context_window.
        let catalog = r#"{
            "models": [{ "slug": "kimi", "display_name": "kimi", "context_window": 128000 }]
        }"#;
        let result = build_simplified_catalog_from_texts("", catalog).expect("entry");
        let entry = &result.get("models").unwrap().as_array().unwrap()[0];
        assert!(
            entry.get("contextWindow").is_none(),
            "default 128_000 should be squashed so the form shows blank, matching the user's blank input"
        );
    }

    #[test]
    fn build_simplified_catalog_respects_explicit_model_context_window() {
        // When config.toml sets model_context_window, that becomes the default fallback.
        let config = r#"model_context_window = 200000
"#;
        let catalog = r#"{
            "models": [
                { "slug": "a", "display_name": "a", "context_window": 200000 },
                { "slug": "b", "display_name": "b", "context_window": 500000 }
            ]
        }"#;
        let result = build_simplified_catalog_from_texts(config, catalog).expect("entries");
        let models = result.get("models").unwrap().as_array().unwrap();
        // Matches default → squashed.
        assert!(models[0].get("contextWindow").is_none());
        // Different from default → preserved.
        assert_eq!(
            models[1].get("contextWindow").and_then(|v| v.as_u64()),
            Some(500_000)
        );
    }

    #[test]
    fn build_simplified_catalog_squashes_inferred_modalities_and_keeps_overrides() {
        let catalog = r#"{
            "models": [
                { "slug": "gpt-5.4", "input_modalities": ["text", "image"] },
                { "slug": "deepseek-v4-pro", "input_modalities": ["text"] },
                { "slug": "gpt-text-override", "input_modalities": ["text"] },
                { "slug": "deepseek-v4-flash", "input_modalities": ["text", "image"] }
            ]
        }"#;

        let result = build_simplified_catalog_from_texts("", catalog).expect("entries");
        let models = result.get("models").unwrap().as_array().unwrap();

        assert!(
            models[0].get("inputModalities").is_none(),
            "GPT text+image is inferred and must not become a sticky hidden override"
        );
        assert!(
            models[1].get("inputModalities").is_none(),
            "confirmed text-only capability is inferred and must remain registry-driven"
        );
        assert_eq!(
            models[2].get("inputModalities"),
            Some(&json!(["text"])),
            "an unknown model explicitly forced to text-only must round-trip"
        );
        assert_eq!(
            models[3].get("inputModalities"),
            Some(&json!(["text", "image"])),
            "an explicit image override for a registered text-only model must round-trip"
        );
    }

    #[test]
    fn build_simplified_catalog_returns_none_when_unparseable() {
        assert!(build_simplified_catalog_from_texts("", "not json").is_none());
        assert!(build_simplified_catalog_from_texts("", "{}").is_none());
        assert!(
            build_simplified_catalog_from_texts("", r#"{"models": []}"#).is_none(),
            "empty models array should yield None so the field is not inserted at all"
        );
        assert!(
            build_simplified_catalog_from_texts(
                "",
                r#"{"models": [{"display_name": "no slug"}]}"#,
            )
            .is_none(),
            "entries lacking slug are skipped; a fully-skipped catalog yields None"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn codex_cli_candidates_are_non_empty_on_macos() {
        let candidates = codex_cli_candidates();
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate == Path::new("codex")),
            "codex CLI candidates must include the PATH entry"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn codex_bundled_models_command_uses_expected_program_and_args() {
        let command = codex_bundled_models_command(Path::new("codex")).unwrap();
        assert_eq!(command.get_program(), "codex");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["debug", "models", "--bundled"]
        );
    }

    #[test]
    fn formal_windows_build_never_runs_user_codex_cli_fallback() {
        assert!(!codex_bundled_cli_allowed(true, true));
        assert!(codex_bundled_cli_allowed(true, false));
        assert!(codex_bundled_cli_allowed(false, true));
    }

    #[test]
    fn successful_model_catalog_template_load_is_cached() {
        use std::cell::Cell;

        let cache = OnceCell::new();
        let calls = Cell::new(0);
        let first = get_or_load_codex_model_catalog_template(&cache, || {
            calls.set(calls.get() + 1);
            Ok(json!({ "slug": "first" }))
        })
        .expect("first template load");
        let second = get_or_load_codex_model_catalog_template(&cache, || {
            calls.set(calls.get() + 1);
            Ok(json!({ "slug": "second" }))
        })
        .expect("cached template load");

        assert_eq!(first, json!({ "slug": "first" }));
        assert_eq!(second, first);
        assert_eq!(calls.get(), 1, "successful template should load only once");
    }

    #[test]
    fn failed_model_catalog_template_load_can_retry() {
        use std::cell::Cell;

        let cache = OnceCell::new();
        let calls = Cell::new(0);
        let first = get_or_load_codex_model_catalog_template(&cache, || {
            calls.set(calls.get() + 1);
            Err(AppError::Message("temporary failure".to_string()))
        });
        assert!(first.is_err());

        let second = get_or_load_codex_model_catalog_template(&cache, || {
            calls.set(calls.get() + 1);
            Ok(json!({ "slug": "recovered" }))
        })
        .expect("retry template load");

        assert_eq!(second, json!({ "slug": "recovered" }));
        assert_eq!(calls.get(), 2, "failed loads must not poison the cache");
    }

    #[test]
    fn codex_cli_candidates_include_user_node_manager_bins() {
        let temp_home = tempfile::tempdir().expect("create temp home");
        let home = temp_home.path();
        let expected = [
            home.join(".nvm/versions/node/v22.14.0/bin/codex"),
            home.join(".volta/bin/codex"),
            home.join(".asdf/shims/codex"),
            home.join(".local/share/mise/shims/codex"),
            home.join(".local/share/fnm/node-versions/v22.14.0/installation/bin/codex"),
        ];

        for candidate in &expected {
            std::fs::create_dir_all(candidate.parent().expect("candidate parent"))
                .expect("create candidate parent");
            std::fs::write(candidate, "").expect("create candidate");
        }

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        push_home_codex_cli_candidates(&mut candidates, &mut seen, home);

        for candidate in expected {
            assert!(
                candidates.contains(&candidate),
                "user-level Codex CLI candidate should be discovered: {}",
                candidate.display()
            );
        }
    }

    #[test]
    fn codex_cli_candidates_deduplicate_entries() {
        let temp_home = tempfile::tempdir().expect("create temp home");
        let home = temp_home.path();
        let candidate = home.join(".volta/bin/codex");
        std::fs::create_dir_all(candidate.parent().expect("candidate parent"))
            .expect("create candidate parent");
        std::fs::write(&candidate, "").expect("create candidate");

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        push_existing_codex_cli_candidate(&mut candidates, &mut seen, candidate.clone());
        push_home_codex_cli_candidates(&mut candidates, &mut seen, home);

        assert_eq!(
            candidates.iter().filter(|path| **path == candidate).count(),
            1,
            "duplicate candidates should be removed"
        );
    }

    #[test]
    fn static_template_is_valid_json_with_slug() {
        let template =
            load_codex_model_template_static().expect("static template must parse as valid JSON");
        assert_eq!(
            template.get("slug").and_then(|v| v.as_str()),
            Some("gpt-5.5"),
            "static template slug must be gpt-5.5"
        );
    }

    #[test]
    fn static_template_has_required_keys() {
        let template =
            load_codex_model_template_static().expect("static template must parse as valid JSON");
        for key in &[
            "model_messages",
            "base_instructions",
            "context_window",
            "display_name",
        ] {
            assert!(
                template.get(key).is_some(),
                "static template must contain key '{key}'"
            );
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn set_catalog_json_field_writes_filename_ignoring_smb_unc_path() {
        let input = r#"model_provider = "custom"
model = "glm-5"
"#;
        let unc_path = Path::new(r"\\server\profiles\user\.codex\fyagent-model-catalog.json");

        let result = set_codex_model_catalog_json_field(input, Some(unc_path)).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        let written_path = parsed
            .get("model_catalog_json")
            .and_then(|v| v.as_str())
            .expect("model_catalog_json should be set");
        assert_eq!(
            written_path, FYAGENT_CODEX_MODEL_CATALOG_FILENAME,
            "should write only the relative filename, not the UNC path"
        );
    }

    #[test]
    fn set_catalog_json_field_writes_filename_for_any_path() {
        let input = r#"model_provider = "custom"
model = "glm-5"
"#;
        let regular_path = Path::new("/Users/user/.codex/fyagent-model-catalog.json");

        let result = set_codex_model_catalog_json_field(input, Some(regular_path)).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();

        assert_eq!(
            parsed.get("model_catalog_json").and_then(|v| v.as_str()),
            Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME),
            "should write only the relative filename, not the full path"
        );
    }

    #[test]
    fn set_catalog_json_none_removes_fyagent_owned_by_filename() {
        // A config copied from another supported host may contain an absolute
        // path. The None arm still removes the FyAgent-owned file by name.
        let input = r#"model_catalog_json = "/Users/me/.codex/fyagent-model-catalog.json"
"#;
        let result = set_codex_model_catalog_json_field(input, None).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed.get("model_catalog_json").is_none(),
            "None arm should remove fyagent-owned field regardless of path format"
        );
    }

    #[test]
    fn set_catalog_json_none_preserves_user_owned_catalog() {
        let input = r#"model_catalog_json = "/Users/me/.codex/my-custom-catalog.json"
"#;
        let result = set_codex_model_catalog_json_field(input, None).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed.get("model_catalog_json").and_then(|v| v.as_str()),
            Some("/Users/me/.codex/my-custom-catalog.json"),
            "None arm should NOT remove user-owned catalog"
        );
    }

    #[test]
    fn resolve_catalog_finds_relative_filename() {
        let config_text = r#"model_provider = "custom"
model_catalog_json = "fyagent-model-catalog.json"
"#;
        let base_dir = PathBuf::from("/Users/user/.codex");
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result,
            Some(base_dir.join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)),
            "relative filename should resolve under base_dir for file I/O"
        );
    }

    #[test]
    fn resolve_catalog_ignores_user_owned_relative() {
        let config_text = r#"model_catalog_json = "my-custom-catalog.json"
"#;
        let base_dir = PathBuf::from("/Users/user/.codex");
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result, None,
            "user-owned catalog should not be claimed by FyAgent"
        );
    }

    #[test]
    fn resolve_catalog_rejects_absolute_path_outside_config_dir() {
        let config_text = r#"model_catalog_json = "/tmp/secret/fyagent-model-catalog.json"
"#;
        let base_dir = PathBuf::from("/Users/user/.codex");
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result, None,
            "absolute path outside ~/.codex must not be accepted"
        );
    }

    #[test]
    fn resolve_catalog_accepts_absolute_path_inside_config_dir() {
        let config_text = r#"model_catalog_json = "/Users/user/.codex/fyagent-model-catalog.json"
"#;
        let base_dir = PathBuf::from("/Users/user/.codex");
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result,
            Some(base_dir.join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)),
            "absolute path inside ~/.codex should be accepted"
        );
    }

    #[test]
    fn resolve_catalog_rejects_traversal_to_parent_directory() {
        let config_text = r#"model_catalog_json = "../fyagent-model-catalog.json"
"#;
        let base_dir = PathBuf::from("/Users/user/.codex");
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result, None,
            "relative traversal outside ~/.codex must not be accepted"
        );
    }

    #[test]
    fn resolve_catalog_rejects_symlink_escaping_config_dir() {
        // 词法包含可被符号链接绕过：~/.codex/link -> 外部目录，
        // "link/fyagent-model-catalog.json" 词法上在 base 内，真实读取却落到
        // base 外。canonicalize 之后的二次校验必须拒绝。
        let temp = tempfile::tempdir().expect("tempdir");
        let base_dir = temp.path().join("codex");
        let outside_dir = temp.path().join("outside");
        fs::create_dir_all(&base_dir).expect("create base");
        fs::create_dir_all(&outside_dir).expect("create outside");
        let escaped_file = outside_dir.join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
        fs::write(&escaped_file, r#"{"models":[]}"#).expect("write escaped catalog");

        #[cfg(target_os = "macos")]
        std::os::unix::fs::symlink(&outside_dir, base_dir.join("link")).expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside_dir, base_dir.join("link")).expect("symlink");

        let config_text = r#"model_catalog_json = "link/fyagent-model-catalog.json"
"#;
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        assert_eq!(
            result, None,
            "symlink escaping the config dir must be rejected after canonicalization"
        );
    }

    #[test]
    fn resolve_catalog_accepts_real_file_inside_config_dir() {
        // 存在于 base 内的真实文件：canonical 校验通过后仍应接受
        let temp = tempfile::tempdir().expect("tempdir");
        let base_dir = temp.path().join("codex");
        fs::create_dir_all(&base_dir).expect("create base");
        let catalog_file = base_dir.join(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
        fs::write(&catalog_file, r#"{"models":[]}"#).expect("write catalog");

        let config_text = r#"model_catalog_json = "fyagent-model-catalog.json"
"#;
        let result = resolve_fyagent_catalog_path(config_text, &base_dir);
        let resolved = result.expect("real file inside config dir should be accepted");
        assert_eq!(
            resolved.file_name().and_then(|n| n.to_str()),
            Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
        );
    }

    #[test]
    fn set_catalog_json_none_removes_relative_path() {
        let input = r#"model_catalog_json = "fyagent-model-catalog.json"
"#;
        let result = set_codex_model_catalog_json_field(input, None).unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed.get("model_catalog_json").is_none(),
            "None arm should remove relative FyAgent-owned field"
        );
    }

    #[test]
    fn read_limited_string_rejects_oversized_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("huge.json");
        let file = std::fs::File::create(&path).expect("create");
        file.set_len(MAX_CODEX_CATALOG_BYTES + 1).expect("set_len");

        let result = read_limited_string(&path, MAX_CODEX_CATALOG_BYTES);
        assert!(
            result.is_err(),
            "file larger than MAX_CODEX_CATALOG_BYTES must be rejected"
        );
    }
}
