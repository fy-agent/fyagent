//! Native omp (Oh My Pi) configuration service.
//!
//! This service resolves the active omp profile, reads only redacted YAML
//! summaries, previews an intent with an opaque bound identity, and applies a
//! lossless CST patch to `models.yml` plus an optional `config.yml` default
//! role update.

use crate::config::{atomic_write_private, get_home_dir};
use crate::error::AppError;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValueSerde;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};
#[cfg(target_os = "macos")]
use std::time::Duration;
use std::time::Instant;
use url::Url;
use uuid::Uuid;

mod yaml_patch;
use yaml_patch::{parse_yaml_strict, patch_default_model_in_yaml, patch_provider_in_yaml};

const MAX_OMP_FILE_BYTES: u64 = 1024 * 1024;
const MISSING_REVISION: &str = "missing";
const DEFAULT_PROFILE: &str = "default";
const MAX_PENDING_PLANS: usize = 16;
#[cfg(target_os = "macos")]
const OMP_VERSION_TIMEOUT: Duration = Duration::from_millis(1500);

type HmacSha256 = Hmac<Sha256>;

static OMP_FILE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static PENDING_PLANS: LazyLock<Mutex<HashMap<String, PreparedOmpSave>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
static TEST_OMP_ENV: LazyLock<Mutex<Option<OmpResolverInput>>> = LazyLock::new(|| Mutex::new(None));

#[cfg(test)]
static TEST_WRITE_FAULT: LazyLock<Mutex<Option<OmpWriteFault>>> =
    LazyLock::new(|| Mutex::new(None));

#[cfg(test)]
#[derive(Debug, Clone)]
enum OmpWriteFault {
    ConfigBeforeWrite,
    ConfigBeforeWriteWithExternalModels(Vec<u8>),
}

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpProviderSummary {
    pub id: String,
    pub base_url: Option<String>,
    pub api: Option<String>,
    pub auth: Option<String>,
    pub has_api_key: bool,
    pub has_command_key: bool,
    pub models: Vec<OmpModelSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpModelSummary {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpConfigRead {
    pub profile: String,
    pub models_path: String,
    pub config_path: String,
    pub models_revision: String,
    pub config_revision: String,
    pub providers: Vec<OmpProviderSummary>,
    pub default_model: Option<String>,
    pub models_file_status: OmpFileStatus,
    pub config_file_status: OmpFileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OmpFileStatus {
    Ok,
    NotFound,
    LegacyJsonOnly,
    InvalidYaml,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmpProviderPatch {
    pub id: String,
    pub base_url: String,
    pub api: String,
    pub api_key: Option<String>,
    pub auth: Option<String>,
    pub models: Vec<OmpModelInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmpModelInput {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmpSavePlan {
    pub provider: OmpProviderPatch,
    pub default_model: Option<String>,
    pub expected_models_revision: String,
    pub expected_config_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OmpChangedFile {
    Models,
    Config,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpSavePreview {
    pub plan_id: String,
    pub plan_digest: String,
    pub profile: String,
    pub models_path: String,
    pub config_path: String,
    pub provider: OmpProviderSummary,
    pub default_model: Option<String>,
    pub changed_files: Vec<OmpChangedFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpApplyConfirmation {
    pub plan_id: String,
    pub plan_digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpSaveResult {
    pub models_path: String,
    pub config_path: String,
    pub models_revision: String,
    pub config_revision: String,
    pub provider: OmpProviderSummary,
    pub default_model: Option<String>,
    pub models_written: bool,
    pub config_written: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpCliDiscovery {
    pub detected: bool,
    pub version: Option<String>,
    pub executable_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OmpResolvedPaths {
    profile: String,
    agent_dir: PathBuf,
    models_path: PathBuf,
    config_path: PathBuf,
    legacy_json_path: PathBuf,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OmpResolverInput {
    home: PathBuf,
    cwd: PathBuf,
    cli_profile: Option<String>,
    omp_profile: Option<String>,
    pi_profile: Option<String>,
    pi_config_dir: Option<String>,
    pi_coding_agent_dir: Option<String>,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    existed: bool,
    bytes: Vec<u8>,
    revision: String,
    content_hash: String,
    status: OmpFileStatus,
}

impl FileSnapshot {
    fn preimage(&self) -> Option<&[u8]> {
        self.existed.then_some(self.bytes.as_slice())
    }
}

#[derive(Debug, Clone)]
struct PreparedOmpSave {
    paths: OmpResolvedPaths,
    models_before: FileSnapshot,
    config_before: FileSnapshot,
    patched_models: Vec<u8>,
    patched_config: Option<Vec<u8>>,
    models_changed: bool,
    config_changed: bool,
    created_at: Instant,
    preview: OmpSavePreview,
}

// ── Profile resolution ──────────────────────────────────────────────────────

fn resolve_omp_paths() -> Result<OmpResolvedPaths, AppError> {
    #[cfg(test)]
    if let Some(input) = TEST_OMP_ENV.lock().expect("lock omp test env").clone() {
        return resolve_omp_paths_from_input(input);
    }

    resolve_omp_paths_from_input(OmpResolverInput {
        home: get_home_dir(),
        cwd: std::env::current_dir()
            .map_err(|e| AppError::Config(format!("omp working directory: {e}")))?,
        cli_profile: None,
        omp_profile: std::env::var("OMP_PROFILE").ok(),
        pi_profile: std::env::var("PI_PROFILE").ok(),
        pi_config_dir: std::env::var("PI_CONFIG_DIR").ok(),
        pi_coding_agent_dir: std::env::var("PI_CODING_AGENT_DIR").ok(),
    })
}

fn resolve_omp_paths_from_input(input: OmpResolverInput) -> Result<OmpResolvedPaths, AppError> {
    if !input.home.is_absolute() {
        return Err(AppError::InvalidInput(format!(
            "omp home must be absolute: {}",
            input.home.display()
        )));
    }

    let profile = input
        .cli_profile
        .as_deref()
        .or(input.omp_profile.as_deref())
        .or(input.pi_profile.as_deref())
        .unwrap_or(DEFAULT_PROFILE)
        .trim();
    let profile = if profile.is_empty() {
        DEFAULT_PROFILE
    } else {
        profile
    }
    .to_string();
    validate_profile_name(&profile)?;

    let root_name = input
        .pi_config_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(".omp");
    validate_root_name(root_name)?;
    let omp_root = input.home.join(root_name);

    let (agent_dir, source) = if profile != DEFAULT_PROFILE {
        (
            omp_root.join("profiles").join(&profile).join("agent"),
            format!("named profile '{profile}'"),
        )
    } else if let Some(dir) = input
        .pi_coding_agent_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let dir = PathBuf::from(dir);
        let dir = if dir.is_absolute() {
            dir
        } else {
            input.cwd.join(dir)
        };
        (dir, "PI_CODING_AGENT_DIR".to_string())
    } else {
        (omp_root.join("agent"), "omp default".to_string())
    };

    if !agent_dir.is_absolute() {
        return Err(AppError::InvalidInput(format!(
            "omp agent directory must resolve to an absolute path: {}",
            agent_dir.display()
        )));
    }

    let models_path = effective_yaml_path(&agent_dir, "models");
    let config_path = effective_yaml_path(&agent_dir, "config");
    let legacy_json_path = agent_dir.join("models.json");

    Ok(OmpResolvedPaths {
        profile,
        agent_dir,
        models_path,
        config_path,
        legacy_json_path,
        source,
    })
}

fn effective_yaml_path(dir: &Path, stem: &str) -> PathBuf {
    let yml = dir.join(format!("{stem}.yml"));
    if yml.exists() {
        return yml;
    }
    let yaml = dir.join(format!("{stem}.yaml"));
    if yaml.exists() {
        return yaml;
    }
    yml
}

fn validate_root_name(value: &str) -> Result<(), AppError> {
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(AppError::InvalidInput(
            "PI_CONFIG_DIR must be a config root name under the user home".to_string(),
        ));
    }
    validate_bounded_component("PI_CONFIG_DIR", value, 64)
}

fn validate_profile_name(value: &str) -> Result<(), AppError> {
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(AppError::InvalidInput(format!(
            "invalid omp profile name '{value}'"
        )));
    }
    validate_bounded_component("omp profile", value, 64)
}

fn validate_bounded_component(label: &str, value: &str, max: usize) -> Result<(), AppError> {
    if value.is_empty() || value.len() > max {
        return Err(AppError::InvalidInput(format!(
            "{label} must be 1..={max} characters"
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(AppError::InvalidInput(format!(
            "{label} contains unsupported characters"
        )));
    }
    Ok(())
}

// ── Reading ─────────────────────────────────────────────────────────────────

pub fn read_omp_config() -> Result<OmpConfigRead, AppError> {
    let _guard = lock_omp_file()?;
    read_omp_config_locked()
}

fn read_omp_config_locked() -> Result<OmpConfigRead, AppError> {
    let paths = resolve_omp_paths()?;
    let models = read_models_snapshot(&paths.models_path, &paths.legacy_json_path)?;
    let config = read_yaml_snapshot(&paths.config_path, "omp config")?;

    let providers = match models.status {
        OmpFileStatus::Ok => parse_providers_from_yaml(&models.bytes)?,
        _ => Vec::new(),
    };
    let default_model = match config.status {
        OmpFileStatus::Ok => parse_default_model_from_bytes(&config.bytes)?,
        _ => None,
    };

    Ok(OmpConfigRead {
        profile: paths.profile,
        models_path: display_path(&paths.models_path),
        config_path: display_path(&paths.config_path),
        models_revision: models.revision,
        config_revision: config.revision,
        providers,
        default_model,
        models_file_status: models.status,
        config_file_status: config.status,
    })
}

fn parse_providers_from_yaml(bytes: &[u8]) -> Result<Vec<OmpProviderSummary>, AppError> {
    let content = utf8(bytes, "omp models")?;
    parse_yaml_strict(content, "omp models")?;
    let value: YamlValueSerde = serde_yaml::from_str(content)
        .map_err(|e| AppError::Config(format!("omp models YAML parse failed: {e}")))?;

    let mut output = Vec::new();
    let Some(root) = as_mapping(&value) else {
        return Ok(output);
    };
    let Some(providers) = root
        .get(YamlValueSerde::String("providers".to_string()))
        .and_then(as_mapping)
    else {
        return Ok(output);
    };

    for (key, provider_value) in providers {
        let Some(id) = scalar_string(key) else {
            continue;
        };
        let Some(provider) = as_mapping(provider_value) else {
            continue;
        };
        let api_key = provider
            .get(YamlValueSerde::String("apiKey".to_string()))
            .and_then(scalar_string_or_tag);
        let models = provider
            .get(YamlValueSerde::String("models".to_string()))
            .and_then(as_sequence)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let model = as_mapping(item)?;
                        let id = model
                            .get(YamlValueSerde::String("id".to_string()))
                            .and_then(scalar_string)?;
                        let name = model
                            .get(YamlValueSerde::String("name".to_string()))
                            .and_then(scalar_string);
                        Some(OmpModelSummary { id, name })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        output.push(OmpProviderSummary {
            id,
            base_url: provider
                .get(YamlValueSerde::String("baseUrl".to_string()))
                .and_then(scalar_string),
            api: provider
                .get(YamlValueSerde::String("api".to_string()))
                .and_then(scalar_string),
            auth: provider
                .get(YamlValueSerde::String("auth".to_string()))
                .and_then(scalar_string),
            has_api_key: api_key
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
            has_command_key: api_key
                .as_deref()
                .is_some_and(|value| value.trim_start().starts_with('!')),
            models,
        });
    }
    Ok(output)
}

pub(crate) fn resolve_omp_provider_probe_target(
    provider_id: &str,
) -> Result<(String, String, Option<String>), AppError> {
    let _guard = lock_omp_file()?;
    let paths = resolve_omp_paths()?;
    let snapshot = read_models_snapshot(&paths.models_path, &paths.legacy_json_path)?;
    if !matches!(snapshot.status, OmpFileStatus::Ok) {
        return Err(AppError::Config("omp models.yml 尚未就绪".to_string()));
    }
    let content = utf8(&snapshot.bytes, "omp models")?;
    let value: YamlValueSerde = serde_yaml::from_str(content)
        .map_err(|e| AppError::Config(format!("omp models YAML parse failed: {e}")))?;
    let provider = as_mapping(&value)
        .and_then(|root| root.get(YamlValueSerde::String("providers".to_string())))
        .and_then(as_mapping)
        .and_then(|providers| providers.get(YamlValueSerde::String(provider_id.to_string())))
        .and_then(as_mapping)
        .ok_or_else(|| AppError::InvalidInput(format!("未找到 omp 供应商 '{provider_id}'")))?;

    let base_url = provider
        .get(YamlValueSerde::String("baseUrl".to_string()))
        .and_then(scalar_string)
        .ok_or_else(|| AppError::InvalidInput("omp 供应商缺少 baseUrl".to_string()))?;
    let api = provider
        .get(YamlValueSerde::String("api".to_string()))
        .and_then(scalar_string)
        .unwrap_or_else(|| "openai-completions".to_string());
    let auth = provider
        .get(YamlValueSerde::String("auth".to_string()))
        .and_then(scalar_string);
    let raw_key = provider
        .get(YamlValueSerde::String("apiKey".to_string()))
        .and_then(scalar_string_or_tag);

    let resolved_key = if auth.as_deref() == Some("none") {
        None
    } else if let Some(raw) = raw_key {
        let trimmed = raw.trim();
        if trimmed.starts_with('!') {
            return Err(AppError::InvalidInput(
                "omp 使用 !命令引用鉴权，请在 omp 终端内测试".to_string(),
            ));
        }
        let looks_like_env_var = !trimmed.is_empty()
            && trimmed.contains('_')
            && trimmed
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_uppercase() || b == b'_')
            && trimmed
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        let is_env_identifier = !trimmed.is_empty()
            && trimmed
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if is_env_identifier {
            match std::env::var(trimmed) {
                Ok(val) if !val.trim().is_empty() => Some(val),
                _ if looks_like_env_var => {
                    return Err(AppError::InvalidInput(format!(
                        "环境变量 {trimmed} 未注入当前桌面进程，请在终端验证或改用直连密钥"
                    )));
                }
                _ => Some(trimmed.to_string()),
            }
        } else {
            Some(trimmed.to_string())
        }
    } else {
        None
    };

    Ok((base_url, api, resolved_key))
}

fn parse_default_model_from_bytes(bytes: &[u8]) -> Result<Option<String>, AppError> {
    let content = utf8(bytes, "omp config")?;
    parse_yaml_strict(content, "omp config")?;
    let value: YamlValueSerde = serde_yaml::from_str(content)
        .map_err(|e| AppError::Config(format!("omp config YAML parse failed: {e}")))?;
    Ok(as_mapping(&value)
        .and_then(|root| root.get(YamlValueSerde::String("modelRoles".to_string())))
        .and_then(as_mapping)
        .and_then(|roles| roles.get(YamlValueSerde::String("default".to_string())))
        .and_then(scalar_string))
}

fn as_mapping(value: &YamlValueSerde) -> Option<&serde_yaml::Mapping> {
    match value {
        YamlValueSerde::Mapping(map) => Some(map),
        YamlValueSerde::Tagged(tagged) => as_mapping(&tagged.value),
        _ => None,
    }
}

fn as_sequence(value: &YamlValueSerde) -> Option<&Vec<YamlValueSerde>> {
    match value {
        YamlValueSerde::Sequence(seq) => Some(seq),
        YamlValueSerde::Tagged(tagged) => as_sequence(&tagged.value),
        _ => None,
    }
}

fn scalar_string(value: &YamlValueSerde) -> Option<String> {
    match value {
        YamlValueSerde::String(value) => Some(value.clone()),
        YamlValueSerde::Tagged(tagged) => scalar_string(&tagged.value),
        _ => None,
    }
}

fn scalar_string_or_tag(value: &YamlValueSerde) -> Option<String> {
    match value {
        YamlValueSerde::String(value) => Some(value.clone()),
        YamlValueSerde::Tagged(tagged) => {
            let tag = tagged.tag.to_string();
            if !tag.starts_with("!!") {
                let normalized_tag = if tag.starts_with('!') {
                    tag
                } else {
                    format!("!{tag}")
                };
                match scalar_string(&tagged.value) {
                    Some(rest) if !rest.trim().is_empty() => {
                        Some(format!("{normalized_tag} {rest}"))
                    }
                    _ => Some(normalized_tag),
                }
            } else {
                scalar_string(&tagged.value)
            }
        }
        _ => None,
    }
}

// ── Preview / apply ─────────────────────────────────────────────────────────

pub fn preview_omp_save(plan: &OmpSavePlan) -> Result<OmpSavePreview, AppError> {
    let _guard = lock_omp_file()?;
    let prepared = prepare_omp_save(plan)?;
    let preview = prepared.preview.clone();
    store_pending_plan(prepared)?;
    Ok(preview)
}

pub fn apply_omp_save(request: &OmpApplyConfirmation) -> Result<OmpSaveResult, AppError> {
    let _guard = lock_omp_file()?;
    let prepared = take_pending_plan(request)?;
    verify_prepared_preimage(&prepared)?;
    apply_prepared_save(prepared)
}

fn prepare_omp_save(plan: &OmpSavePlan) -> Result<PreparedOmpSave, AppError> {
    validate_save_plan(plan)?;

    let paths = resolve_omp_paths()?;
    let models_before = read_models_snapshot(&paths.models_path, &paths.legacy_json_path)?;
    if models_before.revision != plan.expected_models_revision {
        return Err(AppError::Conflict(
            "omp models.yml changed since the preview was created".to_string(),
        ));
    }
    if matches!(models_before.status, OmpFileStatus::LegacyJsonOnly) {
        return Err(AppError::InvalidInput(
            "omp uses legacy models.json; migrate to models.yml via omp CLI before editing"
                .to_string(),
        ));
    }

    let config_before = read_yaml_snapshot(&paths.config_path, "omp config")?;
    if config_before.revision != plan.expected_config_revision {
        return Err(AppError::Conflict(
            "omp config.yml changed since the preview was created".to_string(),
        ));
    }

    let models_content = match models_before.status {
        OmpFileStatus::Ok => utf8(&models_before.bytes, "omp models")?.to_string(),
        OmpFileStatus::NotFound => String::new(),
        OmpFileStatus::LegacyJsonOnly | OmpFileStatus::InvalidYaml => unreachable!(),
    };
    let patched_models = patch_provider_in_yaml(&models_content, &plan.provider)?.into_bytes();

    let patched_config = if let Some(default_model) = &plan.default_model {
        let config_content = match config_before.status {
            OmpFileStatus::Ok => utf8(&config_before.bytes, "omp config")?.to_string(),
            OmpFileStatus::NotFound => String::new(),
            OmpFileStatus::LegacyJsonOnly | OmpFileStatus::InvalidYaml => unreachable!(),
        };
        Some(patch_default_model_in_yaml(&config_content, default_model)?.into_bytes())
    } else {
        None
    };

    for (label, bytes) in [
        ("models", Some(patched_models.as_slice())),
        ("config", patched_config.as_deref()),
    ] {
        if bytes.is_some_and(|bytes| bytes.len() as u64 > MAX_OMP_FILE_BYTES) {
            return Err(AppError::InvalidInput(format!(
                "omp {label} would exceed the 1 MiB file limit"
            )));
        }
    }

    let provider = parse_providers_from_yaml(&patched_models)?
        .into_iter()
        .find(|provider| provider.id == plan.provider.id)
        .ok_or_else(|| {
            AppError::Config(format!(
                "omp provider '{}' missing from preview",
                plan.provider.id
            ))
        })?;
    validate_provider_auth_after_merge(&provider)?;

    let models_changed = patched_models != models_before.bytes;
    let config_changed = patched_config
        .as_ref()
        .is_some_and(|bytes| bytes != &config_before.bytes);
    let default_model = if let Some(default_model) = &plan.default_model {
        Some(default_model.clone())
    } else if matches!(config_before.status, OmpFileStatus::Ok) {
        parse_default_model_from_bytes(&config_before.bytes)?
    } else {
        None
    };
    let plan_id = new_plan_id();
    let plan_digest = plan_digest_for(
        &plan_id,
        plan,
        &paths,
        &models_before.revision,
        &config_before.revision,
    )?;
    let preview = OmpSavePreview {
        plan_id,
        plan_digest,
        profile: paths.profile.clone(),
        models_path: display_path(&paths.models_path),
        config_path: display_path(&paths.config_path),
        provider,
        default_model,
        changed_files: preview_changed_files(models_changed, config_changed),
        warnings: Vec::new(),
    };

    Ok(PreparedOmpSave {
        paths,
        models_before,
        config_before,
        patched_models,
        patched_config,
        models_changed,
        config_changed,
        created_at: Instant::now(),
        preview,
    })
}

fn apply_prepared_save(prepared: PreparedOmpSave) -> Result<OmpSaveResult, AppError> {
    if prepared.models_changed {
        ensure_omp_parent_dir(&prepared.paths.models_path)?;
    }
    if prepared.config_changed {
        ensure_omp_parent_dir(&prepared.config_before.path)?;
    }

    if prepared.models_changed {
        write_backup_if_existing(&prepared.models_before)?;
    }
    if prepared.config_changed {
        write_backup_if_existing(&prepared.config_before)?;
    }

    if prepared.models_changed {
        write_private_maybe_fault(
            &prepared.paths.models_path,
            &prepared.patched_models,
            OmpWriteTarget::Models,
        )?;
    }

    if prepared.config_changed {
        let patched_config = prepared
            .patched_config
            .as_ref()
            .expect("changed config has patched bytes");
        if let Err(write_error) = write_private_maybe_fault(
            &prepared.config_before.path,
            patched_config,
            OmpWriteTarget::Config,
        ) {
            let expected_models_hash = content_hash(&prepared.patched_models);
            let restored = if prepared.models_changed {
                crate::config::restore_file_preimage_if_owned(
                    &prepared.models_before.path,
                    prepared.models_before.preimage(),
                    Some(&expected_models_hash),
                )
                .is_ok()
            } else {
                true
            };
            if restored {
                return Err(AppError::Config(format!(
                    "omp config.yml write failed; models.yml was restored. Please reread and preview again: {write_error}"
                )));
            }
            return Err(AppError::Config(format!(
                "omp config.yml write failed; models.yml changed before rollback, so external changes were preserved. Please reread before saving again: {write_error}"
            )));
        }
    }

    result_success(prepared)
}

fn result_success(prepared: PreparedOmpSave) -> Result<OmpSaveResult, AppError> {
    let models = read_models_snapshot(
        &prepared.paths.models_path,
        &prepared.paths.legacy_json_path,
    )?;
    let config = read_yaml_snapshot(&prepared.paths.config_path, "omp config")?;
    let provider = parse_providers_from_yaml(&models.bytes)?
        .into_iter()
        .find(|provider| provider.id == prepared.preview.provider.id)
        .ok_or_else(|| AppError::Config("omp provider missing after save".to_string()))?;
    let default_model = if matches!(config.status, OmpFileStatus::Ok) {
        parse_default_model_from_bytes(&config.bytes)?
    } else {
        None
    };
    Ok(OmpSaveResult {
        models_path: display_path(&prepared.paths.models_path),
        config_path: display_path(&prepared.paths.config_path),
        models_revision: models.revision,
        config_revision: config.revision,
        provider,
        default_model,
        models_written: prepared.models_changed,
        config_written: prepared.config_changed,
    })
}

fn preview_changed_files(models_changed: bool, config_changed: bool) -> Vec<OmpChangedFile> {
    let mut changed = Vec::new();
    if models_changed {
        changed.push(OmpChangedFile::Models);
    }
    if config_changed {
        changed.push(OmpChangedFile::Config);
    }
    changed
}

fn store_pending_plan(prepared: PreparedOmpSave) -> Result<(), AppError> {
    let mut pending = PENDING_PLANS
        .lock()
        .map_err(|e| AppError::Config(format!("omp pending plan lock is poisoned: {e}")))?;
    while pending.len() >= MAX_PENDING_PLANS {
        let Some(oldest) = pending
            .iter()
            .min_by_key(|(_, prepared)| prepared.created_at)
            .map(|(id, _)| id.clone())
        else {
            break;
        };
        pending.remove(&oldest);
    }
    pending.insert(prepared.preview.plan_id.clone(), prepared);
    Ok(())
}

fn take_pending_plan(request: &OmpApplyConfirmation) -> Result<PreparedOmpSave, AppError> {
    let mut pending = PENDING_PLANS
        .lock()
        .map_err(|e| AppError::Config(format!("omp pending plan lock is poisoned: {e}")))?;
    let prepared = pending.remove(&request.plan_id).ok_or_else(|| {
        AppError::Conflict("omp save preview expired; reread and preview again".to_string())
    })?;
    if !constant_time_eq(
        prepared.preview.plan_digest.as_bytes(),
        request.plan_digest.as_bytes(),
    ) {
        return Err(AppError::Conflict(
            "omp save confirmation does not match the preview".to_string(),
        ));
    }
    Ok(prepared)
}

fn verify_prepared_preimage(prepared: &PreparedOmpSave) -> Result<(), AppError> {
    let paths = resolve_omp_paths()?;
    if paths.profile != prepared.paths.profile
        || paths.models_path != prepared.paths.models_path
        || paths.config_path != prepared.paths.config_path
    {
        return Err(AppError::Conflict(
            "omp active profile or config paths changed since preview".to_string(),
        ));
    }
    ensure_content_hash(
        &prepared.models_before.path,
        &prepared.models_before.content_hash,
    )?;
    ensure_content_hash(
        &prepared.config_before.path,
        &prepared.config_before.content_hash,
    )
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

// ── Input validation / plan identity ────────────────────────────────────────

fn validate_save_plan(plan: &OmpSavePlan) -> Result<(), AppError> {
    validate_provider_id(&plan.provider.id)?;
    validate_base_url(&plan.provider.base_url)?;
    validate_api(&plan.provider.api)?;
    if let Some(auth) = &plan.provider.auth {
        validate_auth(auth)?;
    }
    if let Some(api_key) = &plan.provider.api_key {
        validate_api_key_reference(api_key)?;
    }
    if plan.provider.models.is_empty() {
        return Err(AppError::InvalidInput(
            "at least one omp model must be supplied".to_string(),
        ));
    }
    for model in &plan.provider.models {
        validate_model_id(&model.id)?;
        if let Some(name) = &model.name {
            validate_display_value("model name", name, 256)?;
        }
    }
    if let Some(default_model) = &plan.default_model {
        let prefix = format!("{}/", plan.provider.id);
        if !default_model.starts_with(&prefix) {
            return Err(AppError::InvalidInput(
                "default model selector must target the provider being saved".to_string(),
            ));
        }
        validate_model_id(&default_model[prefix.len()..])?;
        if !plan
            .provider
            .models
            .iter()
            .any(|model| default_model == &format!("{}/{}", plan.provider.id, model.id))
        {
            return Err(AppError::InvalidInput(
                "default model selector must match one supplied model".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_provider_id(value: &str) -> Result<(), AppError> {
    if value.is_empty() || value.len() > 80 || value.contains('/') {
        return Err(AppError::InvalidInput(
            "omp provider id must be 1..=80 characters and must not contain '/'".to_string(),
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(AppError::InvalidInput(
            "omp provider id contains unsupported characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_base_url(value: &str) -> Result<(), AppError> {
    let url = Url::parse(value)
        .map_err(|e| AppError::InvalidInput(format!("invalid omp baseUrl: {e}")))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::InvalidInput(
            "omp baseUrl must be an http(s) URL with a host and no credentials".to_string(),
        ));
    }
    Ok(())
}

fn validate_api(value: &str) -> Result<(), AppError> {
    match value {
        "openai-completions" | "openai-responses" => Ok(()),
        _ => Err(AppError::InvalidInput(format!(
            "unsupported omp provider api '{value}'"
        ))),
    }
}

fn validate_auth(value: &str) -> Result<(), AppError> {
    match value {
        "apiKey" | "none" | "oauth" => Ok(()),
        _ => Err(AppError::InvalidInput(format!(
            "unsupported omp auth mode '{value}'"
        ))),
    }
}

fn validate_provider_auth_after_merge(provider: &OmpProviderSummary) -> Result<(), AppError> {
    if matches!(provider.auth.as_deref(), Some("none" | "oauth")) || provider.has_api_key {
        return Ok(());
    }
    Err(AppError::InvalidInput(format!(
        "omp provider '{}' requires a non-empty apiKey when auth is apiKey or omitted",
        provider.id
    )))
}

fn validate_model_id(value: &str) -> Result<(), AppError> {
    validate_display_value("model id", value, 160)?;
    if value.starts_with('/') || value.ends_with('/') || value.contains("//") {
        return Err(AppError::InvalidInput(
            "omp model id must not start/end with '/' or contain empty path parts".to_string(),
        ));
    }
    Ok(())
}

fn validate_api_key_reference(value: &str) -> Result<(), AppError> {
    validate_display_value("api key reference", value, 512)
}

fn validate_display_value(label: &str, value: &str, max: usize) -> Result<(), AppError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(|c| c.is_control()) {
        return Err(AppError::InvalidInput(format!(
            "{label} must be non-empty text up to {max} characters"
        )));
    }
    Ok(())
}

fn new_plan_id() -> String {
    format!("omp-plan:{}", Uuid::new_v4())
}

fn plan_digest_for(
    plan_id: &str,
    plan: &OmpSavePlan,
    paths: &OmpResolvedPaths,
    models_revision: &str,
    config_revision: &str,
) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(plan_mac_key()).expect("fixed HMAC key");
    update_mac(&mut mac, b"v2");
    update_mac(&mut mac, plan_id.as_bytes());
    update_mac(&mut mac, paths.profile.as_bytes());
    update_mac(&mut mac, path_bytes(&paths.models_path).as_bytes());
    update_mac(&mut mac, models_revision.as_bytes());
    update_mac(&mut mac, path_bytes(&paths.config_path).as_bytes());
    update_mac(&mut mac, config_revision.as_bytes());
    let plan_bytes = serde_json::to_vec(plan)
        .map_err(|e| AppError::Config(format!("serialize omp plan: {e}")))?;
    update_mac(&mut mac, &plan_bytes);
    Ok(hex(mac.finalize().into_bytes().as_slice()))
}

fn update_mac(mac: &mut HmacSha256, bytes: &[u8]) {
    mac.update(&(bytes.len() as u64).to_be_bytes());
    mac.update(bytes);
}

fn plan_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

// ── CLI discovery & direct orchestration ────────────────────────────────────

#[cfg(any(target_os = "macos", test))]
const MAX_OMP_VERSION_BYTES: usize = 64;

#[cfg(any(target_os = "macos", test))]
pub(crate) fn parse_omp_version_output(output: &str) -> Option<String> {
    let line = output.lines().next()?.trim();
    let value = line
        .strip_prefix("omp/")
        .or_else(|| line.strip_prefix("omp "))
        .unwrap_or(line);
    let value = value.strip_prefix('v').unwrap_or(value);
    if value.is_empty()
        || value.len() > MAX_OMP_VERSION_BYTES
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
    {
        return None;
    }
    let (without_build, _) = value
        .split_once('+')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    let (core, _) = without_build
        .split_once('-')
        .map_or((without_build, None), |(a, b)| (a, Some(b)));
    let numbers: Vec<_> = core.split('.').collect();
    if numbers.len() != 3
        || numbers
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some(value.to_string())
}

pub(crate) fn omp_candidates(
    local_app_data: Option<&Path>,
    home: Option<&Path>,
    path_dirs: Vec<PathBuf>,
    is_windows: bool,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let push_unique = |list: &mut Vec<PathBuf>, item: PathBuf| {
        if !list.contains(&item) {
            list.push(item);
        }
    };
    if is_windows {
        if let Some(local) = local_app_data {
            push_unique(&mut candidates, local.join("omp").join("omp.exe"));
            push_unique(
                &mut candidates,
                local.join("Programs").join("omp").join("omp.exe"),
            );
        }
        if let Some(home) = home {
            push_unique(
                &mut candidates,
                home.join(".bun").join("bin").join("omp.exe"),
            );
            push_unique(
                &mut candidates,
                home.join(".bun").join("bin").join("omp.cmd"),
            );
            push_unique(
                &mut candidates,
                home.join(".omp").join("bin").join("omp.exe"),
            );
            push_unique(
                &mut candidates,
                home.join("AppData")
                    .join("Roaming")
                    .join("npm")
                    .join("omp.cmd"),
            );
        }
        for dir in path_dirs {
            push_unique(&mut candidates, dir.join("omp.exe"));
            push_unique(&mut candidates, dir.join("omp.cmd"));
        }
    } else {
        if let Some(home) = home {
            push_unique(&mut candidates, home.join(".bun").join("bin").join("omp"));
            push_unique(&mut candidates, home.join(".omp").join("bin").join("omp"));
            push_unique(&mut candidates, home.join(".local").join("bin").join("omp"));
        }
        push_unique(&mut candidates, PathBuf::from("/opt/homebrew/bin/omp"));
        push_unique(&mut candidates, PathBuf::from("/usr/local/bin/omp"));
        for dir in path_dirs {
            push_unique(&mut candidates, dir.join("omp"));
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
pub async fn discover_omp_cli() -> Result<OmpCliDiscovery, AppError> {
    tokio::task::spawn_blocking(|| {
        let home = get_home_dir();
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Local"));
        let path_dirs = std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();
        let candidates = omp_candidates(Some(&local_app_data), Some(&home), path_dirs, true);
        for candidate in candidates {
            if candidate.is_file() {
                return Ok(OmpCliDiscovery {
                    detected: true,
                    version: None,
                    executable_path: Some(display_path(&candidate)),
                });
            }
        }
        Ok(OmpCliDiscovery {
            detected: false,
            version: None,
            executable_path: None,
        })
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))?
}

#[cfg(target_os = "macos")]
pub async fn discover_omp_cli() -> Result<OmpCliDiscovery, AppError> {
    tokio::task::spawn_blocking(|| {
        let home = get_home_dir();
        let path_dirs = std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();
        let candidates = omp_candidates(None, Some(&home), path_dirs, false);
        for candidate in candidates {
            if !candidate.is_file() {
                continue;
            }
            let version = try_get_omp_version(&candidate);
            return Ok(OmpCliDiscovery {
                detected: true,
                version,
                executable_path: Some(display_path(&candidate)),
            });
        }
        Ok(OmpCliDiscovery {
            detected: false,
            version: None,
            executable_path: None,
        })
    })
    .await
    .map_err(|error| AppError::Message(error.to_string()))?
}

#[cfg(target_os = "macos")]
fn try_get_omp_version(exe_path: &Path) -> Option<String> {
    let mut child = Command::new(exe_path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + OMP_VERSION_TIMEOUT;
    loop {
        match child.try_wait().ok()? {
            Some(_) => {
                let output = child.wait_with_output().ok()?;
                if !output.status.success() {
                    return None;
                }
                return parse_omp_version_output(&String::from_utf8_lossy(&output.stdout));
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OmpRollbackCheckpoint {
    models_path: PathBuf,
    models_existed: bool,
    models_preimage: Vec<u8>,
    written_models_hash: Option<String>,
    config_path: PathBuf,
    config_existed: bool,
    config_preimage: Vec<u8>,
    written_config_hash: Option<String>,
}

impl OmpRollbackCheckpoint {
    pub(crate) fn rollback(&self) -> Result<(), AppError> {
        let _guard = lock_omp_file()?;
        if let Some(expected_hash) = &self.written_models_hash {
            crate::config::restore_file_preimage_if_owned(
                &self.models_path,
                self.models_existed
                    .then_some(self.models_preimage.as_slice()),
                Some(expected_hash),
            )?;
        }
        if let Some(expected_hash) = &self.written_config_hash {
            crate::config::restore_file_preimage_if_owned(
                &self.config_path,
                self.config_existed
                    .then_some(self.config_preimage.as_slice()),
                Some(expected_hash),
            )?;
        }
        Ok(())
    }
}

pub(crate) fn apply_omp_provider_direct(
    patch: OmpProviderPatch,
    default_model: Option<String>,
    expected_revisions: Option<(String, String)>,
) -> Result<(OmpSaveResult, OmpRollbackCheckpoint), AppError> {
    let current = read_omp_config()?;
    let (expected_models_revision, expected_config_revision) =
        expected_revisions.unwrap_or_else(|| {
            (
                current.models_revision.clone(),
                current.config_revision.clone(),
            )
        });
    let models_before = {
        let paths = resolve_omp_paths()?;
        read_models_snapshot(&paths.models_path, &paths.legacy_json_path)?
    };
    let config_before = {
        let paths = resolve_omp_paths()?;
        read_yaml_snapshot(&paths.config_path, "omp config")?
    };
    let preview = preview_omp_save(&OmpSavePlan {
        provider: patch,
        default_model,
        expected_models_revision,
        expected_config_revision,
    })?;
    let saved = apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })?;
    let paths = resolve_omp_paths()?;
    let written_models_hash = saved
        .models_written
        .then(|| fs::read(&paths.models_path).ok().map(|b| content_hash(&b)))
        .flatten();
    let written_config_hash = saved
        .config_written
        .then(|| fs::read(&paths.config_path).ok().map(|b| content_hash(&b)))
        .flatten();
    Ok((
        saved,
        OmpRollbackCheckpoint {
            models_path: models_before.path,
            models_existed: models_before.existed,
            models_preimage: models_before.bytes,
            written_models_hash,
            config_path: config_before.path,
            config_existed: config_before.existed,
            config_preimage: config_before.bytes,
            written_config_hash,
        },
    ))
}

// ── File utilities ──────────────────────────────────────────────────────────

fn read_models_snapshot(path: &Path, legacy_json_path: &Path) -> Result<FileSnapshot, AppError> {
    if !path.exists() && legacy_json_path.exists() {
        return Ok(FileSnapshot {
            path: path.to_path_buf(),
            existed: false,
            bytes: Vec::new(),
            revision: MISSING_REVISION.to_string(),
            content_hash: MISSING_REVISION.to_string(),
            status: OmpFileStatus::LegacyJsonOnly,
        });
    }
    read_yaml_snapshot(path, "omp models")
}

fn read_yaml_snapshot(path: &Path, label: &str) -> Result<FileSnapshot, AppError> {
    match fs::File::open(path) {
        Ok(_) => {
            let bytes = read_file_limited(path, label)?;
            let content = utf8(&bytes, label)?;
            parse_yaml_strict(content, label)?;
            Ok(FileSnapshot {
                path: path.to_path_buf(),
                existed: true,
                revision: revision(&bytes),
                content_hash: content_hash(&bytes),
                bytes,
                status: OmpFileStatus::Ok,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileSnapshot {
            path: path.to_path_buf(),
            existed: false,
            bytes: Vec::new(),
            revision: MISSING_REVISION.to_string(),
            content_hash: MISSING_REVISION.to_string(),
            status: OmpFileStatus::NotFound,
        }),
        Err(e) => Err(AppError::io(path, e)),
    }
}

fn read_file_limited(path: &Path, label: &str) -> Result<Vec<u8>, AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::io(path, e))?;
    let metadata = file.metadata().map_err(|e| AppError::io(path, e))?;
    if metadata.len() > MAX_OMP_FILE_BYTES {
        return Err(AppError::InvalidInput(format!(
            "{label} file exceeds the 1 MiB limit: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_OMP_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| AppError::io(path, e))?;
    if bytes.len() as u64 > MAX_OMP_FILE_BYTES {
        return Err(AppError::InvalidInput(format!(
            "{label} file exceeds the 1 MiB limit: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

fn utf8<'a>(bytes: &'a [u8], label: &str) -> Result<&'a str, AppError> {
    std::str::from_utf8(bytes)
        .map_err(|e| AppError::Config(format!("{label} file must be UTF-8: {e}")))
}

fn revision(bytes: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(revision_mac_key()).expect("fixed HMAC key");
    mac.update(bytes);
    hex(mac.finalize().into_bytes().as_slice())
}

fn content_hash(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes).as_slice())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn revision_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn random_mac_key() -> [u8; 32] {
    let mut key = [0_u8; 32];
    key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    key
}

fn ensure_content_hash(path: &Path, expected: &str) -> Result<(), AppError> {
    let actual = match fs::File::open(path) {
        Ok(_) => content_hash(&read_file_limited(path, "omp file")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => MISSING_REVISION.to_string(),
        Err(e) => return Err(AppError::io(path, e)),
    };
    if actual == expected {
        Ok(())
    } else {
        Err(AppError::Conflict(format!(
            "omp file changed since preview: {}",
            path.display()
        )))
    }
}

fn write_backup_if_existing(snapshot: &FileSnapshot) -> Result<(), AppError> {
    if snapshot.existed {
        let backup = crate::config::rolling_backup_path(&snapshot.path);
        crate::config::write_backup_file(&backup, &snapshot.bytes)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum OmpWriteTarget {
    Models,
    Config,
}

fn write_private_maybe_fault(
    path: &Path,
    bytes: &[u8],
    target: OmpWriteTarget,
) -> Result<(), AppError> {
    #[cfg(test)]
    if matches!(target, OmpWriteTarget::Config) {
        if let Some(fault) = TEST_WRITE_FAULT
            .lock()
            .expect("lock omp write fault")
            .take()
        {
            match fault {
                OmpWriteFault::ConfigBeforeWrite => {
                    return Err(AppError::IoContext {
                        context: "injected omp config write failure".to_string(),
                        source: std::io::Error::other("injected failure"),
                    });
                }
                OmpWriteFault::ConfigBeforeWriteWithExternalModels(external) => {
                    let models = path.with_file_name("models.yml");
                    fs::write(&models, external).map_err(|e| AppError::io(&models, e))?;
                    return Err(AppError::IoContext {
                        context: "injected omp config write failure".to_string(),
                        source: std::io::Error::other("injected failure with external edit"),
                    });
                }
            }
        }
    }
    let _ = target;
    atomic_write_private(path, bytes)
}

fn ensure_omp_parent_dir(path: &Path) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Config(format!(
            "omp path has no parent directory: {}",
            path.display()
        ))
    })?;
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    Ok(())
}

fn lock_omp_file() -> Result<MutexGuard<'static, ()>, AppError> {
    OMP_FILE_LOCK
        .lock()
        .map_err(|e| AppError::Config(format!("omp file lock is poisoned: {e}")))
}

fn display_path(path: &Path) -> String {
    crate::config::display_user_path(path)
}

fn path_bytes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod test_support {
    use super::*;

    pub(crate) struct TestOmpEnv {
        _dir: tempfile::TempDir,
        previous: Option<OmpResolverInput>,
    }

    impl TestOmpEnv {
        pub(crate) fn new() -> Self {
            let dir = tempfile::tempdir().expect("create omp test home");
            Self::with_input(dir, None, None, None, None, None)
        }

        pub(crate) fn with_input(
            dir: tempfile::TempDir,
            cli_profile: Option<&str>,
            omp_profile: Option<&str>,
            pi_profile: Option<&str>,
            pi_config_dir: Option<&str>,
            pi_coding_agent_dir: Option<&str>,
        ) -> Self {
            #[cfg(target_os = "windows")]
            crate::initialize_windows_user_context().expect("Windows test user context");
            let input = OmpResolverInput {
                home: dir.path().to_path_buf(),
                cwd: dir.path().join("working-directory"),
                cli_profile: cli_profile.map(ToOwned::to_owned),
                omp_profile: omp_profile.map(ToOwned::to_owned),
                pi_profile: pi_profile.map(ToOwned::to_owned),
                pi_config_dir: pi_config_dir.map(ToOwned::to_owned),
                pi_coding_agent_dir: pi_coding_agent_dir.map(ToOwned::to_owned),
            };
            let previous = TEST_OMP_ENV
                .lock()
                .expect("lock omp test env")
                .replace(input);
            Self {
                _dir: dir,
                previous,
            }
        }
    }

    impl Drop for TestOmpEnv {
        fn drop(&mut self) {
            *TEST_OMP_ENV.lock().expect("lock omp test env") = self.previous.take();
            *TEST_WRITE_FAULT.lock().expect("lock omp write fault") = None;
        }
    }

    pub(crate) fn arm_config_write_fault() {
        *TEST_WRITE_FAULT.lock().expect("lock omp write fault") =
            Some(OmpWriteFault::ConfigBeforeWrite);
    }

    pub(crate) fn arm_config_write_fault_with_external_models(bytes: Vec<u8>) {
        *TEST_WRITE_FAULT.lock().expect("lock omp write fault") =
            Some(OmpWriteFault::ConfigBeforeWriteWithExternalModels(bytes));
    }
}

#[cfg(test)]
mod tests;
