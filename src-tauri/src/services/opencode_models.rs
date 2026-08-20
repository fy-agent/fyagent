//! Secret-free OpenCode provider snapshot and revisioned model writes.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::{Mutex as StdMutex, OnceLock},
    time::{Duration, Instant},
};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::Sha256;
use uuid::Uuid;

use crate::opencode_config::{
    lock_opencode_config, read_opencode_config_bytes, write_opencode_config_value,
};

const MAX_MODELS: usize = 1_000;
const DEFAULT_NPM: &str = "@ai-sdk/openai-compatible";
const OVERWRITE_TOKEN_TTL: Duration = Duration::from_secs(3 * 60);
const OVERWRITE_TOKEN_EXPIRED_RETENTION: Duration = Duration::from_secs(3 * 60);
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OpenCodeModelsErrorCode {
    #[serde(rename = "OPENCODE_CONFIG_UNAVAILABLE")]
    ConfigUnavailable,
    #[serde(rename = "OPENCODE_WRITE_FAILED")]
    WriteFailed,
    #[serde(rename = "OPENCODE_BACKUP_FAILED")]
    BackupFailed,
    #[serde(rename = "OPENCODE_NO_TARGET")]
    NoTarget,
    #[serde(rename = "OPENCODE_OVERWRITE_TOKEN_INVALID")]
    OverwriteTokenInvalid,
    #[serde(rename = "OPENCODE_OVERWRITE_TOKEN_EXPIRED")]
    OverwriteTokenExpired,
    #[serde(rename = "OPENCODE_CREDENTIAL_COLLISION")]
    CredentialCollision,
    #[serde(rename = "OPENCODE_INVALID_REQUEST")]
    InvalidRequest,
    #[serde(rename = "OPENCODE_FETCH_FAILED")]
    FetchFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenCodeModelsErrorDto {
    pub code: OpenCodeModelsErrorCode,
}

impl OpenCodeModelsErrorDto {
    const fn new(code: OpenCodeModelsErrorCode) -> Self {
        Self { code }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchOpenCodeModelsRequest {
    pub base_url: String,
    pub api_key: String,
    pub allow_no_api_key: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveOpenCodeModelsRequest {
    pub provider_name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub selected_model_ids: Vec<String>,
    #[serde(default)]
    pub removed_model_ids: Vec<String>,
    pub expected_revision: Option<String>,
    #[serde(default)]
    pub overwrite_token: Option<String>,
}

impl fmt::Debug for FetchOpenCodeModelsRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FetchOpenCodeModelsRequest")
            .field("base_url", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .field("allow_no_api_key", &self.allow_no_api_key)
            .finish()
    }
}

impl fmt::Debug for SaveOpenCodeModelsRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveOpenCodeModelsRequest")
            .field("provider_name", &self.provider_name)
            .field("base_url", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .field("selected_model_id_count", &self.selected_model_ids.len())
            .field("removed_model_id_count", &self.removed_model_ids.len())
            .field("expected_revision", &"[REDACTED]")
            .field("overwrite_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FetchedModelRef {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FetchedModelList {
    pub models: Vec<FetchedModelRef>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeProviderSnapshot {
    pub id: String,
    pub name: String,
    pub model_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeModelSnapshot {
    pub providers: Vec<OpenCodeProviderSnapshot>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state")]
pub enum SaveOpenCodeModelsOutcome {
    #[serde(rename = "saved")]
    Saved {
        revision: String,
        #[serde(rename = "modelCount")]
        model_count: usize,
        #[serde(rename = "createdEntries")]
        created_entries: usize,
        #[serde(rename = "updatedEntries")]
        updated_entries: usize,
    },
    #[serde(rename = "overwrite_confirmation_required")]
    OverwriteConfirmationRequired {
        token: String,
        #[serde(rename = "existingIds")]
        existing_ids: Vec<String>,
    },
    #[serde(rename = "concurrent_modification")]
    ConcurrentModification,
}

struct PendingOverwrite {
    request_digest: [u8; 32],
    expected_revision: Option<String>,
    expires_at: Instant,
}

fn pending_overwrites() -> &'static StdMutex<HashMap<String, PendingOverwrite>> {
    static PENDING: OnceLock<StdMutex<HashMap<String, PendingOverwrite>>> = OnceLock::new();
    PENDING.get_or_init(|| StdMutex::new(HashMap::new()))
}

pub(crate) fn get_opencode_model_snapshot() -> Result<OpenCodeModelSnapshot, OpenCodeModelsErrorDto>
{
    let _guard = lock_opencode_config();
    let bytes = read_opencode_config_bytes()
        .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::ConfigUnavailable))?;
    let Some(bytes) = bytes else {
        return Ok(OpenCodeModelSnapshot {
            providers: Vec::new(),
            revision: None,
        });
    };
    let config = parse_config_object(&bytes)?;
    Ok(OpenCodeModelSnapshot {
        providers: project_providers(&config)?,
        revision: Some(revision_for(&bytes)),
    })
}

pub(crate) async fn fetch_opencode_provider_models(
    request: FetchOpenCodeModelsRequest,
) -> Result<FetchedModelList, OpenCodeModelsErrorDto> {
    if request.base_url.trim().is_empty() {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::InvalidRequest,
        ));
    }
    let models = super::model_fetch::fetch_models_optional_auth(
        request.base_url.trim(),
        request.api_key.trim(),
        false,
        request.allow_no_api_key,
    )
    .await
    .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::FetchFailed))?;
    project_fetched_models(models, request.api_key.trim())
}

pub(crate) fn save_opencode_models(
    request: SaveOpenCodeModelsRequest,
) -> Result<SaveOpenCodeModelsOutcome, OpenCodeModelsErrorDto> {
    let _guard = lock_opencode_config();
    save_opencode_models_locked(&request)
}

fn save_opencode_models_locked(
    request: &SaveOpenCodeModelsRequest,
) -> Result<SaveOpenCodeModelsOutcome, OpenCodeModelsErrorDto> {
    let selected = normalize_ids(&request.selected_model_ids);
    let removed = normalize_ids(&request.removed_model_ids);
    if selected
        .iter()
        .any(|id| removed.iter().any(|other| other == id))
    {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::InvalidRequest,
        ));
    }
    if selected.is_empty() && removed.is_empty() {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::NoTarget,
        ));
    }
    let credential = request.api_key.trim();
    if selected
        .iter()
        .any(|id| credential_matches_model_id(credential, id))
    {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::CredentialCollision,
        ));
    }
    if !selected.is_empty() && request.base_url.trim().is_empty() {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::InvalidRequest,
        ));
    }

    let pending = request
        .overwrite_token
        .as_deref()
        .map(|token| consume_overwrite_token(token, request, &selected, &removed))
        .transpose()?;

    let previous_bytes = read_opencode_config_bytes()
        .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::ConfigUnavailable))?;
    let current_revision = previous_bytes.as_deref().map(revision_for);
    if request.expected_revision != current_revision {
        return Ok(SaveOpenCodeModelsOutcome::ConcurrentModification);
    }

    let mut config = match previous_bytes.as_deref() {
        Some(bytes) if !bytes.is_empty() => parse_config_object(bytes)?,
        _ => json!({}),
    };
    let provider_id = resolve_provider_id(&config, request.provider_name.trim());
    let existing = existing_model_ids(&config, &provider_id)?;
    let confirmation = existing_targets(&existing, &selected, &removed);
    if let Some(pending) = pending {
        if pending.expected_revision != current_revision {
            return Ok(SaveOpenCodeModelsOutcome::ConcurrentModification);
        }
    } else if !confirmation.is_empty() {
        let token = issue_overwrite_token(request, &selected, &removed);
        return Ok(SaveOpenCodeModelsOutcome::OverwriteConfirmationRequired {
            token,
            existing_ids: confirmation,
        });
    }

    let (created, updated) = apply_provider_mutations(
        &mut config,
        &provider_id,
        request.provider_name.trim(),
        request.base_url.trim(),
        credential,
        &selected,
        &removed,
    )?;
    reject_secret_model_ids(&config, credential)?;

    let backup_path = crate::opencode_config::get_opencode_dir().join("opencode.json.backup");
    if let Some(parent) = backup_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::BackupFailed))?;
    }
    std::fs::write(&backup_path, previous_bytes.as_deref().unwrap_or(b"{}"))
        .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::BackupFailed))?;
    let written = write_opencode_config_value(&config)
        .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::WriteFailed))?;
    let model_count = existing_model_ids(&config, &provider_id)?.len();
    Ok(SaveOpenCodeModelsOutcome::Saved {
        revision: revision_for(&written),
        model_count,
        created_entries: created,
        updated_entries: updated,
    })
}

fn parse_config_object(bytes: &[u8]) -> Result<Value, OpenCodeModelsErrorDto> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::ConfigUnavailable))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::ConfigUnavailable,
        ))
    }
}

fn providers_object(config: &Value) -> Result<&Map<String, Value>, OpenCodeModelsErrorDto> {
    match config.get("provider") {
        None => Ok(EMPTY_PROVIDERS.get_or_init(Map::new)),
        Some(Value::Object(map)) => Ok(map),
        Some(_) => Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::ConfigUnavailable,
        )),
    }
}

static EMPTY_PROVIDERS: OnceLock<Map<String, Value>> = OnceLock::new();

fn providers_object_mut(
    config: &mut Value,
) -> Result<&mut Map<String, Value>, OpenCodeModelsErrorDto> {
    let object = config
        .as_object_mut()
        .ok_or_else(|| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::ConfigUnavailable))?;
    if !object.get("provider").is_some_and(Value::is_object) {
        object.insert("provider".into(), json!({}));
    }
    object
        .get_mut("provider")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::ConfigUnavailable))
}

fn project_providers(
    config: &Value,
) -> Result<Vec<OpenCodeProviderSnapshot>, OpenCodeModelsErrorDto> {
    reject_secret_model_ids(config, "")?;
    let mut providers = Vec::new();
    for (id, value) in providers_object(config)? {
        if id.trim().is_empty() {
            continue;
        }
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(id)
            .to_string();
        providers.push(OpenCodeProviderSnapshot {
            id: id.clone(),
            name,
            model_ids: model_ids_from_provider(value)?,
        });
    }
    Ok(providers)
}

fn model_ids_from_provider(provider: &Value) -> Result<Vec<String>, OpenCodeModelsErrorDto> {
    let Some(models) = provider.get("models") else {
        return Ok(Vec::new());
    };
    let Some(object) = models.as_object() else {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::ConfigUnavailable,
        ));
    };
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for key in object.keys() {
        let id = key.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        ids.push(id.to_string());
    }
    Ok(ids)
}

fn existing_model_ids(
    config: &Value,
    provider_id: &str,
) -> Result<HashSet<String>, OpenCodeModelsErrorDto> {
    let Some(provider) = providers_object(config)?.get(provider_id) else {
        return Ok(HashSet::new());
    };
    Ok(model_ids_from_provider(provider)?.into_iter().collect())
}

fn existing_targets(
    existing: &HashSet<String>,
    selected: &[String],
    removed: &[String],
) -> Vec<String> {
    let mut confirmation = Vec::new();
    let mut seen = HashSet::new();
    for id in selected.iter().chain(removed) {
        if existing.contains(id) && seen.insert(id.clone()) {
            confirmation.push(id.clone());
        }
    }
    confirmation
}

fn resolve_provider_id(config: &Value, provider_name: &str) -> String {
    let slug = slugify(provider_name);
    let Ok(providers) = providers_object(config) else {
        return slug;
    };
    if providers.contains_key(&slug) {
        return slug;
    }
    if providers.len() == 1 {
        return providers.keys().next().cloned().unwrap_or(slug);
    }
    slug
}

fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "fyagent".into()
    } else {
        slug
    }
}

fn apply_provider_mutations(
    config: &mut Value,
    provider_id: &str,
    provider_name: &str,
    base_url: &str,
    api_key: &str,
    selected: &[String],
    removed: &[String],
) -> Result<(usize, usize), OpenCodeModelsErrorDto> {
    let existing = existing_model_ids(config, provider_id)?;
    let providers = providers_object_mut(config)?;
    let provider = providers
        .entry(provider_id.to_string())
        .or_insert_with(default_provider);
    if !provider.is_object() {
        *provider = default_provider();
    }
    let object = provider
        .as_object_mut()
        .ok_or_else(|| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::WriteFailed))?;
    if object.get("npm").and_then(Value::as_str).is_none() {
        object.insert("npm".into(), json!(DEFAULT_NPM));
    }
    let display_name = if provider_name.is_empty() {
        provider_id
    } else {
        provider_name
    };
    object.insert("name".into(), json!(display_name));
    if !object.get("options").is_some_and(Value::is_object) {
        object.insert("options".into(), json!({}));
    }
    if let Some(options) = object.get_mut("options").and_then(Value::as_object_mut) {
        if !base_url.is_empty() {
            options.insert("baseURL".into(), json!(base_url));
        }
        if !api_key.is_empty() {
            options.insert("apiKey".into(), json!(api_key));
        }
    }
    if !object.get("models").is_some_and(Value::is_object) {
        object.insert("models".into(), json!({}));
    }
    let models = object
        .get_mut("models")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::WriteFailed))?;
    for id in removed {
        models.remove(id);
    }
    let mut created = 0usize;
    let mut updated = 0usize;
    for id in selected {
        if existing.contains(id) {
            updated += 1;
            if let Some(model) = models.get_mut(id) {
                if let Some(model_object) = model.as_object_mut() {
                    if !model_object.contains_key("name") {
                        model_object.insert("name".into(), json!(id));
                    }
                }
            } else {
                models.insert(id.clone(), json!({ "name": id }));
            }
        } else {
            created += 1;
            models.insert(id.clone(), json!({ "name": id }));
        }
    }
    Ok((created, updated))
}

fn default_provider() -> Value {
    json!({
        "npm": DEFAULT_NPM,
        "name": "",
        "options": {},
        "models": {}
    })
}

fn reject_secret_model_ids(
    config: &Value,
    submitted_key: &str,
) -> Result<(), OpenCodeModelsErrorDto> {
    let mut secrets = Vec::new();
    if !submitted_key.trim().is_empty() {
        secrets.push(submitted_key.trim().to_string());
    }
    for provider in providers_object(config)?.values() {
        if let Some(secret) = provider
            .pointer("/options/apiKey")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            secrets.push(secret.to_string());
        }
        for model_id in model_ids_from_provider(provider)? {
            if secrets
                .iter()
                .any(|secret| credential_matches_model_id(secret, &model_id))
            {
                return Err(OpenCodeModelsErrorDto::new(
                    OpenCodeModelsErrorCode::CredentialCollision,
                ));
            }
        }
    }
    Ok(())
}

fn project_fetched_models(
    models: Vec<super::model_fetch::FetchedModel>,
    api_key: &str,
) -> Result<FetchedModelList, OpenCodeModelsErrorDto> {
    let mut seen = HashSet::new();
    let mut projected = Vec::new();
    let mut truncated = false;
    for model in models {
        let id = model.id.trim().to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        if credential_matches_model_id(api_key, &id) {
            return Err(OpenCodeModelsErrorDto::new(
                OpenCodeModelsErrorCode::CredentialCollision,
            ));
        }
        if projected.len() >= MAX_MODELS {
            truncated = true;
            continue;
        }
        projected.push(FetchedModelRef {
            id,
            owned_by: model.owned_by,
        });
    }
    Ok(FetchedModelList {
        models: projected,
        truncated,
    })
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for raw in ids {
        let id = raw.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        result.push(id.to_string());
    }
    result
}

fn credential_matches_model_id(credential: &str, model_id: &str) -> bool {
    let credential = credential.trim();
    !credential.is_empty() && model_id.trim().contains(credential)
}

fn issue_overwrite_token(
    request: &SaveOpenCodeModelsRequest,
    selected: &[String],
    removed: &[String],
) -> String {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let pending = PendingOverwrite {
        request_digest: request_digest(request, selected, removed),
        expected_revision: request.expected_revision.clone(),
        expires_at: Instant::now() + OVERWRITE_TOKEN_TTL,
    };
    let mut pending_overwrites = lock_pending();
    let now = Instant::now();
    pending_overwrites.retain(|_, item| item.expires_at + OVERWRITE_TOKEN_EXPIRED_RETENTION > now);
    pending_overwrites.insert(token.clone(), pending);
    token
}

fn consume_overwrite_token(
    token: &str,
    request: &SaveOpenCodeModelsRequest,
    selected: &[String],
    removed: &[String],
) -> Result<PendingOverwrite, OpenCodeModelsErrorDto> {
    let pending = lock_pending().remove(token).ok_or_else(|| {
        OpenCodeModelsErrorDto::new(OpenCodeModelsErrorCode::OverwriteTokenInvalid)
    })?;
    if pending.expires_at <= Instant::now() {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::OverwriteTokenExpired,
        ));
    }
    if !constant_time_equals(
        &pending.request_digest,
        &request_digest(request, selected, removed),
    ) {
        return Err(OpenCodeModelsErrorDto::new(
            OpenCodeModelsErrorCode::OverwriteTokenInvalid,
        ));
    }
    Ok(pending)
}

fn lock_pending() -> std::sync::MutexGuard<'static, HashMap<String, PendingOverwrite>> {
    pending_overwrites()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn request_digest(
    request: &SaveOpenCodeModelsRequest,
    selected: &[String],
    removed: &[String],
) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(overwrite_mac_key())
        .expect("the fixed-size overwrite MAC key is always valid");
    mac.update(b"fyagent-opencode-overwrite-v1");
    update_length_prefixed(&mut mac, request.provider_name.trim().as_bytes());
    update_length_prefixed(&mut mac, request.base_url.trim().as_bytes());
    update_optional_string(&mut mac, request.expected_revision.as_deref());
    for id in selected {
        update_length_prefixed(&mut mac, id.as_bytes());
    }
    mac.update(&(selected.len() as u64).to_be_bytes());
    for id in removed {
        update_length_prefixed(&mut mac, id.as_bytes());
    }
    mac.update(&(removed.len() as u64).to_be_bytes());
    let api_key_digest = mac_bytes(api_key_mac_key(), request.api_key.as_bytes());
    update_length_prefixed(&mut mac, &api_key_digest);
    mac_bytes_from_mac(mac)
}

fn update_length_prefixed(mac: &mut HmacSha256, bytes: &[u8]) {
    mac.update(&(bytes.len() as u64).to_be_bytes());
    mac.update(bytes);
}

fn update_optional_string(mac: &mut HmacSha256, value: Option<&str>) {
    match value {
        Some(value) => {
            mac.update(&[1]);
            update_length_prefixed(mac, value.as_bytes());
        }
        None => mac.update(&[0]),
    }
}

fn mac_bytes(key: &[u8; 32], bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("the fixed-size MAC key is always valid");
    mac.update(bytes);
    mac_bytes_from_mac(mac)
}

fn mac_bytes_from_mac(mac: HmacSha256) -> [u8; 32] {
    let bytes = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    output
}

fn constant_time_equals(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn revision_for(bytes: &[u8]) -> String {
    mac_bytes(revision_mac_key(), bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn revision_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn overwrite_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn api_key_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn random_mac_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    struct TestHomeGuard(Option<std::ffi::OsString>);
    impl TestHomeGuard {
        fn set(home: &std::path::Path) -> Self {
            let guard = Self(std::env::var_os("FYAGENT_TEST_HOME"));
            std::env::set_var("FYAGENT_TEST_HOME", home);
            guard
        }
    }
    impl Drop for TestHomeGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
        }
    }

    fn write_config(home: &std::path::Path, value: &Value) {
        let dir = home.join(".config").join("opencode");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("opencode.json"),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }

    fn read_config(home: &std::path::Path) -> Value {
        serde_json::from_slice(
            &std::fs::read(home.join(".config").join("opencode").join("opencode.json")).unwrap(),
        )
        .unwrap()
    }

    fn seeded_config() -> Value {
        json!({
            "theme": "keep-me",
            "provider": {
                "gateway": {
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "Gateway",
                    "extra": { "keep": true },
                    "options": {
                        "baseURL": "https://old.example.test/v1",
                        "apiKey": "OPENCODE-SECRET"
                    },
                    "models": {
                        "existing-model": { "name": "Existing", "limit": { "context": 8 } }
                    }
                }
            }
        })
    }

    fn save_request(
        revision: Option<String>,
        selected: &[&str],
        removed: &[&str],
    ) -> SaveOpenCodeModelsRequest {
        SaveOpenCodeModelsRequest {
            provider_name: "Gateway".into(),
            base_url: "https://gateway.example.test/v1".into(),
            api_key: "USER-OPENCODE-KEY".into(),
            selected_model_ids: selected.iter().map(|id| (*id).to_string()).collect(),
            removed_model_ids: removed.iter().map(|id| (*id).to_string()).collect(),
            expected_revision: revision,
            overwrite_token: None,
        }
    }

    #[test]
    #[serial]
    fn snapshot_is_secret_free_and_preserves_ids() {
        let temp = tempfile::TempDir::new().unwrap();
        let _home = TestHomeGuard::set(temp.path());
        write_config(temp.path(), &seeded_config());
        let snapshot = get_opencode_model_snapshot().unwrap();
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.providers[0].id, "gateway");
        assert_eq!(snapshot.providers[0].name, "Gateway");
        assert_eq!(snapshot.providers[0].model_ids, vec!["existing-model"]);
        assert!(snapshot.revision.is_some());
        let debug = format!("{snapshot:?}");
        let json = serde_json::to_string(&snapshot).unwrap();
        for secret in ["OPENCODE-SECRET", "USER-OPENCODE-KEY"] {
            assert!(!debug.contains(secret));
            assert!(!json.contains(secret));
        }
        assert!(!json.contains("apiKey"));
    }

    #[test]
    #[serial]
    fn save_adds_model_keeps_unknown_fields_and_never_returns_secrets() {
        let temp = tempfile::TempDir::new().unwrap();
        let _home = TestHomeGuard::set(temp.path());
        write_config(temp.path(), &seeded_config());
        let revision = get_opencode_model_snapshot().unwrap().revision;
        let outcome = save_opencode_models(save_request(revision, &["new-model"], &[])).unwrap();
        match outcome {
            SaveOpenCodeModelsOutcome::Saved {
                created_entries,
                updated_entries,
                model_count,
                ..
            } => {
                assert_eq!(created_entries, 1);
                assert_eq!(updated_entries, 0);
                assert_eq!(model_count, 2);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
        let config = read_config(temp.path());
        assert_eq!(config["theme"], "keep-me");
        assert_eq!(config["provider"]["gateway"]["extra"]["keep"], true);
        assert_eq!(
            config["provider"]["gateway"]["models"]["existing-model"]["limit"]["context"],
            8
        );
        assert_eq!(
            config["provider"]["gateway"]["models"]["new-model"]["name"],
            "new-model"
        );
        assert_eq!(
            config["provider"]["gateway"]["options"]["apiKey"],
            "USER-OPENCODE-KEY"
        );
        let snapshot = get_opencode_model_snapshot().unwrap();
        let debug = format!("{snapshot:?}");
        assert!(!debug.contains("USER-OPENCODE-KEY"));
        assert!(!debug.contains("OPENCODE-SECRET"));
    }

    #[test]
    #[serial]
    fn updating_existing_model_requires_overwrite_token() {
        let temp = tempfile::TempDir::new().unwrap();
        let _home = TestHomeGuard::set(temp.path());
        write_config(temp.path(), &seeded_config());
        let revision = get_opencode_model_snapshot().unwrap().revision;
        let first =
            save_opencode_models(save_request(revision.clone(), &["existing-model"], &[])).unwrap();
        let SaveOpenCodeModelsOutcome::OverwriteConfirmationRequired {
            token,
            existing_ids,
        } = first
        else {
            panic!("expected overwrite: {first:?}");
        };
        assert_eq!(existing_ids, vec!["existing-model"]);
        let mut confirmed = save_request(revision, &["existing-model"], &[]);
        confirmed.overwrite_token = Some(token);
        match save_opencode_models(confirmed).unwrap() {
            SaveOpenCodeModelsOutcome::Saved {
                updated_entries, ..
            } => assert_eq!(updated_entries, 1),
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn stale_revision_is_concurrent_modification() {
        let temp = tempfile::TempDir::new().unwrap();
        let _home = TestHomeGuard::set(temp.path());
        write_config(temp.path(), &seeded_config());
        let outcome =
            save_opencode_models(save_request(Some("stale".into()), &["new-model"], &[])).unwrap();
        assert_eq!(outcome, SaveOpenCodeModelsOutcome::ConcurrentModification);
    }
}
