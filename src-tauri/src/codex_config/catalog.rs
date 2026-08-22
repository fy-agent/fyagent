use super::*;
use crate::config::{get_home_dir, path_is_within, write_json_file};
use crate::model_capabilities::{image_input_capability_from_modalities, ImageInputCapability};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCatalogToolProfile {
    ProxyChat,
    NativeResponses,
    Anthropic,
}

impl CodexCatalogToolProfile {
    pub fn from_api_format(api_format: Option<&str>) -> Self {
        match api_format {
            Some("anthropic") => CodexCatalogToolProfile::Anthropic,
            Some("openai_responses") => CodexCatalogToolProfile::NativeResponses,
            _ => CodexCatalogToolProfile::ProxyChat,
        }
    }
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(not(test))]
static CODEX_MODEL_CATALOG_TEMPLATE_CACHE: OnceCell<Value> = OnceCell::new();

pub(crate) const CODEX_WEB_SEARCH_FIELD: &str = "web_search";
pub(crate) const CODEX_WEB_SEARCH_DISABLED: &str = "disabled";

const CODEX_WEB_SEARCH_REJECT_HOSTS: &[&str] = &[
    "xiaomimimo.com",
    "longcat.chat",
    "minimax.io",
    "minimaxi.com",
];
const CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES: &[&str] =
    &["mimo", "longcat", "minimax", "qwen3-coder"];

pub(crate) fn codex_top_level_model(config_text: &str) -> Option<String> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get("model")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn codex_native_gateway_rejects_web_search(config_text: &str) -> bool {
    if let Some(base_url) = extract_codex_base_url(config_text) {
        let base_url = base_url.to_ascii_lowercase();
        if CODEX_WEB_SEARCH_REJECT_HOSTS
            .iter()
            .any(|host| base_url.contains(host))
        {
            return true;
        }
    }
    if let Some(model) = codex_top_level_model(config_text) {
        let model = model.to_ascii_lowercase();
        let model = model.rsplit('/').next().unwrap_or(model.as_str());
        if CODEX_WEB_SEARCH_REJECT_MODEL_PREFIXES
            .iter()
            .any(|prefix| model.starts_with(prefix))
        {
            return true;
        }
    }
    false
}

const CODEX_MODEL_CATALOG_TEMPLATE_SLUG: &str = "gpt-5.5";

pub(super) fn parse_codex_positive_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(n)) => n.as_u64().filter(|v| *v > 0),
        Some(Value::String(s)) => s.trim().parse::<u64>().ok().filter(|v| *v > 0),
        _ => None,
    }
}

pub(super) fn extract_codex_top_level_u64(config_text: &str, field: &str) -> Option<u64> {
    let doc = config_text.parse::<toml::Value>().ok()?;
    doc.get(field)
        .and_then(|value| value.as_integer())
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(super) fn codex_catalog_input_modalities(
    model: &str,
    declared_modalities: Option<&[String]>,
) -> Vec<String> {
    let modalities = match image_input_capability_from_modalities(model, declared_modalities) {
        ImageInputCapability::Unsupported => &["text"][..],
        ImageInputCapability::Supported | ImageInputCapability::Unknown => &["text", "image"][..],
    };
    modalities.iter().map(|item| (*item).to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexCatalogModelSpec {
    pub(super) model: String,
    pub(super) display_name: Option<String>,
    pub(super) context_window: Option<u64>,
    pub(super) supports_parallel_tool_calls: Option<bool>,
    pub(super) input_modalities: Option<Vec<String>>,
    pub(super) base_instructions: Option<String>,
}

pub(super) fn codex_catalog_model_entry(
    template: &Value,
    spec: &CodexCatalogModelSpec,
    priority: usize,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let mut entry = template.clone();
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };

    let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
    let context_window = spec.context_window.unwrap_or(default_context_window);
    entry_obj.insert("slug".to_string(), json!(spec.model));
    entry_obj.insert("display_name".to_string(), json!(display_name));
    entry_obj.insert("description".to_string(), json!(display_name));
    entry_obj.insert("context_window".to_string(), json!(context_window));
    entry_obj.insert("max_context_window".to_string(), json!(context_window));
    entry_obj.insert("priority".to_string(), json!(1000 + priority));
    entry_obj.insert("additional_speed_tiers".to_string(), json!([]));
    entry_obj.insert("service_tiers".to_string(), json!([]));
    entry_obj.insert("availability_nux".to_string(), Value::Null);
    entry_obj.insert("upgrade".to_string(), Value::Null);
    entry_obj.insert(
        "input_modalities".to_string(),
        json!(codex_catalog_input_modalities(
            &spec.model,
            spec.input_modalities.as_deref(),
        )),
    );

    if profile != CodexCatalogToolProfile::ProxyChat {
        for key in [
            "apply_patch_tool_type",
            "web_search_tool_type",
            "tools",
            "model_messages",
        ] {
            entry_obj.remove(key);
        }
        entry_obj.insert("shell_type".to_string(), json!("shell_command"));
        if let Some(base_instructions) = spec
            .base_instructions
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
        }
        if let Some(parallel) = spec.supports_parallel_tool_calls {
            entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
        }
    }

    entry
}

pub(super) fn codex_catalog_model_specs(settings: &Value) -> Vec<CodexCatalogModelSpec> {
    let Some(models) = settings
        .get("modelCatalog")
        .and_then(|catalog| catalog.get("models"))
        .and_then(|models| models.as_array())
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut specs = Vec::new();
    for model_config in models {
        let Some(model) = model_config
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|model| !model.is_empty())
        else {
            continue;
        };
        if !seen.insert(model.to_string()) {
            continue;
        }

        let display_name = model_config
            .get("displayName")
            .or_else(|| model_config.get("display_name"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        let context_window = parse_codex_positive_u64(
            model_config
                .get("contextWindow")
                .or_else(|| model_config.get("context_window")),
        );
        let supports_parallel_tool_calls = model_config
            .get("supportsParallelToolCalls")
            .or_else(|| model_config.get("supports_parallel_tool_calls"))
            .and_then(|value| value.as_bool());
        let input_modalities = model_config
            .get("inputModalities")
            .or_else(|| model_config.get("input_modalities"))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty());
        let base_instructions = model_config
            .get("baseInstructions")
            .or_else(|| model_config.get("base_instructions"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);

        specs.push(CodexCatalogModelSpec {
            model: model.to_string(),
            display_name,
            context_window,
            supports_parallel_tool_calls,
            input_modalities,
            base_instructions,
        });
    }
    specs
}

pub(super) fn find_codex_model_template(catalog: &Value) -> Option<Value> {
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .and_then(|models| {
            models.iter().find(|model| {
                model.get("slug").and_then(|slug| slug.as_str())
                    == Some(CODEX_MODEL_CATALOG_TEMPLATE_SLUG)
            })
        })
        .cloned()
}

pub(super) fn load_codex_model_template_from_cache() -> Result<Option<Value>, AppError> {
    let path = get_codex_config_dir().join("models_cache.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    let catalog: Value = serde_json::from_str(&text).map_err(|e| AppError::json(&path, e))?;
    Ok(find_codex_model_template(&catalog))
}

#[cfg(target_os = "macos")]
const CODEX_CLI_FIXED_CANDIDATES: &[&str] =
    &["codex", "/opt/homebrew/bin/codex", "/usr/local/bin/codex"];

#[cfg(windows)]
const CODEX_CLI_FIXED_CANDIDATES: &[&str] = &[];

pub(super) fn push_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    let key = candidate.to_string_lossy().into_owned();
    if seen.insert(key) {
        candidates.push(candidate);
    }
}

pub(super) fn push_existing_codex_cli_candidate(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    candidate: PathBuf,
) {
    if candidate.exists() {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

pub(super) fn push_codex_cli_candidates_from_version_dirs(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    versions_dir: PathBuf,
    suffix: &[&str],
) {
    let Ok(entries) = fs::read_dir(versions_dir) else {
        return;
    };
    let mut discovered = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let mut candidate = entry.path();
            for component in suffix {
                candidate.push(component);
            }
            candidate
        })
        .filter(|candidate| candidate.exists())
        .collect::<Vec<_>>();
    discovered.sort_by(|a, b| b.cmp(a));
    for candidate in discovered {
        push_codex_cli_candidate(candidates, seen, candidate);
    }
}

pub(super) fn push_home_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    home: &Path,
) {
    for relative in [
        ".nvm/current/bin/codex",
        ".volta/bin/codex",
        ".asdf/shims/codex",
        ".local/share/mise/shims/codex",
        ".config/mise/shims/codex",
        ".local/bin/codex",
        ".npm-global/bin/codex",
        ".npm-packages/bin/codex",
        ".local/share/pnpm/codex",
        "Library/pnpm/codex",
    ] {
        push_existing_codex_cli_candidate(candidates, seen, home.join(relative));
    }
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".nvm/versions/node"),
        &["bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join(".local/share/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
    push_codex_cli_candidates_from_version_dirs(
        candidates,
        seen,
        home.join("Library/Application Support/fnm/node-versions"),
        &["installation", "bin", "codex"],
    );
}

#[cfg(target_os = "macos")]
pub(super) fn push_env_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
) {
    for (env_key, suffix) in [
        ("NPM_CONFIG_PREFIX", &["bin", "codex"][..]),
        ("VOLTA_HOME", &["bin", "codex"][..]),
        ("ASDF_DATA_DIR", &["shims", "codex"][..]),
        ("MISE_DATA_DIR", &["shims", "codex"][..]),
        ("PNPM_HOME", &["codex"][..]),
    ] {
        let Some(prefix) = std::env::var_os(env_key) else {
            continue;
        };
        let mut candidate = PathBuf::from(prefix);
        for component in suffix {
            candidate.push(component);
        }
        push_existing_codex_cli_candidate(candidates, seen, candidate);
    }
    if let Some(nvm_dir) = std::env::var_os("NVM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(nvm_dir).join("versions/node"),
            &["bin", "codex"],
        );
    }
    if let Some(fnm_dir) = std::env::var_os("FNM_DIR") {
        push_codex_cli_candidates_from_version_dirs(
            candidates,
            seen,
            PathBuf::from(fnm_dir).join("node-versions"),
            &["installation", "bin", "codex"],
        );
    }
}

#[cfg(windows)]
pub(super) fn push_env_codex_cli_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
) {
    for directory in crate::windows_runtime::safe_command_search_paths() {
        for name in ["codex.cmd", "codex.exe", "codex"] {
            push_existing_codex_cli_candidate(candidates, seen, directory.join(name));
        }
    }
}

pub(super) fn codex_cli_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for candidate in CODEX_CLI_FIXED_CANDIDATES {
        push_codex_cli_candidate(&mut candidates, &mut seen, PathBuf::from(candidate));
    }
    push_env_codex_cli_candidates(&mut candidates, &mut seen);
    let home = get_home_dir();
    #[cfg(windows)]
    if crate::windows_runtime::is_local_command_path(&home) {
        push_home_codex_cli_candidates(&mut candidates, &mut seen, &home);
    }
    #[cfg(target_os = "macos")]
    push_home_codex_cli_candidates(&mut candidates, &mut seen, &home);
    candidates
}

pub(super) fn codex_bundled_cli_allowed(
    target_is_windows: bool,
    formal_windows_build: bool,
) -> bool {
    !target_is_windows || !formal_windows_build
}

pub(super) fn codex_bundled_models_command(
    candidate: &Path,
) -> Result<Command, crate::windows_runtime::WindowsStartupErrorCode> {
    let mut command = Command::new(candidate);
    command
        .args(["debug", "models", "--bundled"])
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        crate::windows_runtime::configure_shell_user_command(&mut command, candidate.parent())?;
        command
            .current_dir(crate::windows_runtime::require_interactive_user_context().user_profile());
        command.creation_flags(CREATE_NO_WINDOW);
    }
    Ok(command)
}

pub(super) fn load_codex_model_template_from_bundled() -> Result<Option<Value>, AppError> {
    if !codex_bundled_cli_allowed(
        cfg!(target_os = "windows"),
        crate::windows_runtime::formal_windows_build(),
    ) {
        return Ok(None);
    }
    for candidate in codex_cli_candidates() {
        let candidate_label = candidate.to_string_lossy();
        let mut command = match codex_bundled_models_command(&candidate) {
            Ok(command) => command,
            Err(error) => {
                log::debug!("failed to configure Codex CLI environment: {error}");
                continue;
            }
        };
        let output = match command.output() {
            Ok(output) => output,
            Err(err) => {
                log::debug!("failed to run `{candidate_label} debug models --bundled`: {err}");
                continue;
            }
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!("`{candidate_label} debug models --bundled` failed: {stderr}");
            continue;
        }
        let catalog: Value = match serde_json::from_slice(&output.stdout) {
            Ok(catalog) => catalog,
            Err(e) => {
                log::debug!(
                    "Failed to parse `{candidate_label} debug models --bundled` output: {e}"
                );
                continue;
            }
        };
        if let Some(template) = find_codex_model_template(&catalog) {
            return Ok(Some(template));
        }
    }
    Ok(None)
}

pub(super) fn load_codex_model_template_static() -> Option<Value> {
    let text = include_str!("../resources/gpt5_5_template.json");
    match serde_json::from_str(text) {
        Ok(template) => Some(template),
        Err(e) => {
            log::warn!("Failed to parse bundled gpt-5.5 template: {e}");
            None
        }
    }
}

pub(super) fn load_codex_native_responses_template() -> Value {
    let text = include_str!("../resources/codex_native_responses_template.json");
    serde_json::from_str(text).expect("bundled codex native responses template must be valid JSON")
}

const CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS: &[&str] = &["deepseek.com"];

pub(super) fn load_codex_deepseek_official_catalog_models() -> Vec<Value> {
    let text = include_str!("../resources/codex_deepseek_catalog_template.json");
    let catalog: Value =
        serde_json::from_str(text).expect("bundled DeepSeek official catalog must be valid JSON");
    catalog
        .get("models")
        .and_then(|models| models.as_array())
        .cloned()
        .unwrap_or_default()
}

pub(super) fn codex_official_vendor_catalog_models(
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Option<Vec<Value>> {
    if profile != CodexCatalogToolProfile::NativeResponses {
        return None;
    }
    let base_url = extract_codex_base_url(config_text)?;
    let parsed_base_url = url::Url::parse(&base_url).ok()?;
    if parsed_base_url.scheme() != "https" {
        return None;
    }
    let base_url_host = parsed_base_url
        .host_str()?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if CODEX_DEEPSEEK_OFFICIAL_CATALOG_HOSTS
        .iter()
        .any(|official_host| {
            base_url_host == *official_host
                || base_url_host
                    .strip_suffix(*official_host)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
    {
        let models = load_codex_deepseek_official_catalog_models();
        if !models.is_empty() {
            return Some(models);
        }
    }
    None
}

pub(super) fn codex_vendor_catalog_model_entry(
    vendor_models: &[Value],
    spec: &CodexCatalogModelSpec,
    priority: usize,
) -> Value {
    let matched = vendor_models.iter().find(|entry| {
        entry
            .get("slug")
            .and_then(|slug| slug.as_str())
            .is_some_and(|slug| slug.eq_ignore_ascii_case(&spec.model))
    });
    let mut entry = match matched {
        Some(found) => found.clone(),
        None => vendor_models.first().cloned().unwrap_or_else(|| json!({})),
    };
    let Some(entry_obj) = entry.as_object_mut() else {
        return json!({});
    };
    if matched.is_none() {
        let display_name = spec.display_name.as_deref().unwrap_or(&spec.model);
        entry_obj.insert("slug".to_string(), json!(spec.model));
        entry_obj.insert("display_name".to_string(), json!(display_name));
        entry_obj.insert("description".to_string(), json!(display_name));
        entry_obj.insert("priority".to_string(), json!(1000 + priority));
    }
    if let Some(display_name) = spec.display_name.as_deref() {
        entry_obj.insert("display_name".to_string(), json!(display_name));
    }
    if let Some(context_window) = spec.context_window {
        entry_obj.insert("context_window".to_string(), json!(context_window));
        entry_obj.insert("max_context_window".to_string(), json!(context_window));
    }
    if let Some(parallel) = spec.supports_parallel_tool_calls {
        entry_obj.insert("supports_parallel_tool_calls".to_string(), json!(parallel));
    }
    if let Some(modalities) = spec.input_modalities.as_deref() {
        entry_obj.insert("input_modalities".to_string(), json!(modalities));
    }
    if let Some(base_instructions) = spec
        .base_instructions
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        entry_obj.insert("base_instructions".to_string(), json!(base_instructions));
    }
    fill_template_fields_from_static(&mut entry);
    entry
}

const CODEX_CATALOG_PARSER_REQUIRED_FIELDS: &[&str] = &["supports_reasoning_summaries"];

pub(super) fn fill_template_fields_from_static(template: &mut Value) {
    let Some(static_template) = load_codex_model_template_static() else {
        return;
    };
    let (Some(template_obj), Some(static_obj)) =
        (template.as_object_mut(), static_template.as_object())
    else {
        return;
    };
    for key in CODEX_CATALOG_PARSER_REQUIRED_FIELDS {
        if !template_obj.contains_key(*key) {
            if let Some(value) = static_obj.get(*key) {
                template_obj.insert((*key).to_string(), value.clone());
            }
        }
    }
}

pub(super) fn load_codex_model_catalog_template_uncached() -> Result<Value, AppError> {
    if let Some(mut template) = load_codex_model_template_from_cache()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    if let Some(mut template) = load_codex_model_template_from_bundled()? {
        fill_template_fields_from_static(&mut template);
        return Ok(template);
    }
    if let Some(template) = load_codex_model_template_static() {
        return Ok(template);
    }
    Err(AppError::Message(format!(
        "Codex model catalog template `{CODEX_MODEL_CATALOG_TEMPLATE_SLUG}` not found. Please start Codex once so models_cache.json is available, or ensure the `codex` CLI is on PATH."
    )))
}

pub(super) fn get_or_load_codex_model_catalog_template<F>(
    cache: &OnceCell<Value>,
    loader: F,
) -> Result<Value, AppError>
where
    F: FnOnce() -> Result<Value, AppError>,
{
    cache.get_or_try_init(loader).cloned()
}

#[cfg(not(test))]
fn load_codex_model_catalog_template() -> Result<Value, AppError> {
    get_or_load_codex_model_catalog_template(
        &CODEX_MODEL_CATALOG_TEMPLATE_CACHE,
        load_codex_model_catalog_template_uncached,
    )
}

#[cfg(test)]
fn load_codex_model_catalog_template() -> Result<Value, AppError> {
    load_codex_model_catalog_template_uncached()
}

pub(super) fn codex_model_catalog_from_specs(
    specs: &[CodexCatalogModelSpec],
    template: &Value,
    profile: CodexCatalogToolProfile,
    default_context_window: u64,
) -> Value {
    let entries: Vec<Value> = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            codex_catalog_model_entry(template, spec, index, profile, default_context_window)
        })
        .collect();
    json!({ "models": entries })
}

pub(super) fn codex_model_catalog_from_settings(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<Option<Value>, AppError> {
    let specs = codex_catalog_model_specs(settings);
    if specs.is_empty() {
        return Ok(None);
    }
    if let Some(vendor_models) = codex_official_vendor_catalog_models(config_text, profile) {
        let entries: Vec<Value> = specs
            .iter()
            .enumerate()
            .map(|(index, spec)| codex_vendor_catalog_model_entry(&vendor_models, spec, index))
            .collect();
        return Ok(Some(json!({ "models": entries })));
    }
    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);
    let template = match profile {
        CodexCatalogToolProfile::NativeResponses | CodexCatalogToolProfile::Anthropic => {
            load_codex_native_responses_template()
        }
        CodexCatalogToolProfile::ProxyChat => load_codex_model_catalog_template()?,
    };
    Ok(Some(codex_model_catalog_from_specs(
        &specs,
        &template,
        profile,
        default_context_window,
    )))
}

pub(super) fn set_codex_model_catalog_json_field(
    config_text: &str,
    catalog_path: Option<&Path>,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    match catalog_path {
        Some(_) => {
            doc["model_catalog_json"] = toml_edit::value(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
        }
        None => {
            let should_remove = doc
                .get("model_catalog_json")
                .and_then(|item| item.as_str())
                .map(|path| {
                    Path::new(path).file_name().and_then(|name| name.to_str())
                        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME)
                })
                .unwrap_or(false);
            if should_remove {
                doc.as_table_mut().remove("model_catalog_json");
            }
        }
    }
    Ok(doc.to_string())
}

pub(super) fn set_codex_native_web_search_field(
    config_text: &str,
    disable: bool,
) -> Result<String, AppError> {
    let mut doc = config_text
        .parse::<DocumentMut>()
        .map_err(|e| AppError::Message(format!("Invalid Codex config.toml: {e}")))?;
    if disable {
        doc[CODEX_WEB_SEARCH_FIELD] = toml_edit::value(CODEX_WEB_SEARCH_DISABLED);
    } else {
        let owned = doc
            .get(CODEX_WEB_SEARCH_FIELD)
            .and_then(|item| item.as_str())
            == Some(CODEX_WEB_SEARCH_DISABLED);
        if owned {
            doc.as_table_mut().remove(CODEX_WEB_SEARCH_FIELD);
        }
    }
    Ok(doc.to_string())
}

pub fn prepare_codex_config_text_with_model_catalog(
    settings: &Value,
    config_text: &str,
    profile: CodexCatalogToolProfile,
) -> Result<String, AppError> {
    if let Some(catalog) = codex_model_catalog_from_settings(settings, config_text, profile)? {
        let catalog_path = get_codex_model_catalog_path();
        let config_text = set_codex_model_catalog_json_field(config_text, Some(&catalog_path))?;
        let disable_web_search = match profile {
            CodexCatalogToolProfile::Anthropic => true,
            CodexCatalogToolProfile::NativeResponses => {
                codex_native_gateway_rejects_web_search(&config_text)
            }
            CodexCatalogToolProfile::ProxyChat => false,
        };
        let config_text = set_codex_native_web_search_field(&config_text, disable_web_search)?;
        write_json_file(&catalog_path, &catalog)?;
        Ok(config_text)
    } else {
        let config_text = set_codex_model_catalog_json_field(config_text, None)?;
        let disable_web_search = profile == CodexCatalogToolProfile::Anthropic;
        set_codex_native_web_search_field(&config_text, disable_web_search)
    }
}

pub(super) const MAX_CODEX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;

pub fn read_codex_model_catalog_simplified_from_live() -> Result<Option<Value>, AppError> {
    let config_text = read_codex_config_text()?;
    let config_dir = get_codex_config_dir();
    let Some(catalog_path) = resolve_fyagent_catalog_path(&config_text, &config_dir) else {
        return Ok(None);
    };
    if !catalog_path.exists() {
        return Ok(None);
    }
    let catalog_text = match read_limited_string(&catalog_path, MAX_CODEX_CATALOG_BYTES) {
        Ok(text) => text,
        Err(error) => {
            log::warn!(
                "拒绝读取越界或过大的 Codex 模型目录 {}: {error}",
                catalog_path.display()
            );
            return Ok(None);
        }
    };
    Ok(build_simplified_catalog_from_texts(
        &config_text,
        &catalog_text,
    ))
}

pub(crate) fn read_limited_string(path: &Path, max_bytes: u64) -> Result<String, AppError> {
    let metadata = fs::metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.len() > max_bytes {
        return Err(AppError::Config(format!(
            "文件 {} 超过大小上限 {} 字节",
            path.display(),
            max_bytes
        )));
    }
    fs::read_to_string(path).map_err(|error| AppError::io(path, error))
}

pub(crate) fn read_codex_model_catalog_text(path: &Path) -> Result<String, AppError> {
    read_limited_string(path, MAX_CODEX_CATALOG_BYTES)
}

pub(crate) fn resolve_fyagent_catalog_path(config_text: &str, base_dir: &Path) -> Option<PathBuf> {
    if config_text.trim().is_empty() {
        return None;
    }
    let doc = config_text.parse::<DocumentMut>().ok()?;
    let catalog_path_str = doc
        .get("model_catalog_json")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let referenced_path = Path::new(catalog_path_str);
    let is_fyagent_owned = referenced_path.file_name().and_then(|name| name.to_str())
        == Some(FYAGENT_CODEX_MODEL_CATALOG_FILENAME);
    if !is_fyagent_owned {
        return None;
    }

    let is_unix_absolute = catalog_path_str.starts_with('/');
    let resolved = if referenced_path.is_absolute() || is_unix_absolute {
        referenced_path.to_path_buf()
    } else {
        base_dir.join(referenced_path)
    };

    if !path_is_within(base_dir, &resolved) {
        log::warn!(
            "Codex model_catalog_json 指向配置目录外: {}（允许目录: {}）",
            resolved.display(),
            base_dir.display()
        );
        return None;
    }

    if resolved.exists() {
        let canonical = match fs::canonicalize(&resolved) {
            Ok(path) => path,
            Err(error) => {
                log::warn!(
                    "Codex model_catalog_json canonicalize 失败: {}: {error}",
                    resolved.display()
                );
                return None;
            }
        };
        let canonical_base = fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        if !path_is_within(&canonical_base, &canonical) {
            log::warn!(
                "Codex model_catalog_json 经符号链接解析到配置目录外: {} -> {}（允许目录: {}）",
                resolved.display(),
                canonical.display(),
                canonical_base.display()
            );
            return None;
        }
        return Some(canonical);
    }

    Some(resolved)
}

pub(super) fn build_simplified_catalog_from_texts(
    config_text: &str,
    catalog_text: &str,
) -> Option<Value> {
    let catalog: Value = serde_json::from_str(catalog_text).ok()?;
    let models = catalog.get("models").and_then(|m| m.as_array())?;
    let default_context_window =
        extract_codex_top_level_u64(config_text, "model_context_window").unwrap_or(128_000);

    let mut entries = Vec::with_capacity(models.len());
    for entry in models {
        let Some(model) = entry
            .get("slug")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let mut obj = serde_json::Map::new();
        obj.insert("model".to_string(), json!(model));
        if let Some(display_name) = entry
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != model)
        {
            obj.insert("displayName".to_string(), json!(display_name));
        }
        if let Some(context_window) = entry
            .get("context_window")
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0 && *v != default_context_window)
        {
            obj.insert("contextWindow".to_string(), json!(context_window));
        }
        if let Some(parallel) = entry
            .get("supports_parallel_tool_calls")
            .and_then(|v| v.as_bool())
        {
            obj.insert("supportsParallelToolCalls".to_string(), json!(parallel));
        }
        if let Some(modalities) = entry.get("input_modalities").and_then(|v| v.as_array()) {
            let mods: Vec<String> = modalities
                .iter()
                .filter_map(|m| m.as_str())
                .map(str::to_string)
                .collect();
            let inferred = codex_catalog_input_modalities(model, None);
            if !mods.is_empty() && mods != inferred {
                obj.insert("inputModalities".to_string(), json!(mods));
            }
        }
        entries.push(Value::Object(obj));
    }

    (!entries.is_empty()).then(|| json!({ "models": entries }))
}
