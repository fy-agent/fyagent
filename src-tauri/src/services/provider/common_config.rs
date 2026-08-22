use serde_json::Value;

use crate::app_config::AppType;
use crate::error::AppError;

pub(super) fn extract_common_config_snippet_from_settings(
    app_type: AppType,
    settings_config: &Value,
) -> Result<String, AppError> {
    match app_type {
        AppType::Claude => extract_claude_common_config(settings_config),
        AppType::ClaudeDesktop => Ok(String::new()),
        AppType::Codex => extract_codex_common_config(settings_config),
        AppType::Gemini => extract_gemini_common_config(settings_config),
        AppType::GrokBuild => Ok(String::new()),
        AppType::OpenCode => extract_opencode_common_config(settings_config),
        AppType::OpenClaw => extract_openclaw_common_config(settings_config),
        AppType::Hermes => Ok(String::new()),
    }
}

/// Return true when a config key can carry authentication material and must
/// never enter a shared common-config snippet.
pub(super) fn is_sensitive_config_key(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();

    const SENSITIVE_SUFFIXES: &[&str] = &[
        "_KEY",
        "_API_KEY",
        "_ACCESS_KEY",
        "_ACCESS_KEY_ID",
        "_KEY_ID",
        "_PRIVATE_KEY",
        "_APIKEY",
        "_ACCESSKEY",
        "_SECRETKEY",
        "_APITOKEN",
        "_AUTH_TOKEN",
        "_TOKEN",
        "_PAT",
        "_PWD",
        "_PASS",
        "_PASSPHRASE",
        "_CREDS",
    ];
    const SENSITIVE_EXACT: &[&str] = &[
        "APIKEY",
        "API_KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "CREDENTIALS",
    ];
    const SENSITIVE_CONTAINS: &[&str] = &[
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "BEARER_TOKEN",
    ];

    SENSITIVE_EXACT.contains(&upper.as_str())
        || SENSITIVE_SUFFIXES
            .iter()
            .any(|suffix| upper.ends_with(suffix))
        || SENSITIVE_CONTAINS
            .iter()
            .any(|fragment| upper.contains(fragment))
}

fn extract_claude_common_config(settings: &Value) -> Result<String, AppError> {
    let mut config = settings.clone();
    const ENV_PROVIDER_SPECIFIC_EXCLUDES: &[&str] = &[
        "ANTHROPIC_MODEL",
        "ANTHROPIC_REASONING_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
        "ANTHROPIC_DEFAULT_FABLE_MODEL",
        "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
        "CLAUDE_CODE_SUBAGENT_MODEL",
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
        "ANTHROPIC_BASE_URL",
    ];
    const TOP_LEVEL_EXCLUDES: &[&str] = &["apiBaseUrl", "primaryModel", "smallFastModel"];

    if let Some(env) = config.get_mut("env").and_then(Value::as_object_mut) {
        let sensitive: Vec<String> = env
            .keys()
            .filter(|key| is_sensitive_config_key(key))
            .cloned()
            .collect();
        for key in ENV_PROVIDER_SPECIFIC_EXCLUDES {
            env.remove(*key);
        }
        for key in &sensitive {
            env.remove(key);
        }
        if env.is_empty() {
            config.as_object_mut().map(|obj| obj.remove("env"));
        }
    }

    if let Some(obj) = config.as_object_mut() {
        let sensitive: Vec<String> = obj
            .keys()
            .filter(|key| is_sensitive_config_key(key))
            .cloned()
            .collect();
        for key in TOP_LEVEL_EXCLUDES {
            obj.remove(*key);
        }
        for key in &sensitive {
            obj.remove(key);
        }
    }

    if config.as_object().is_none_or(|obj| obj.is_empty()) {
        return Ok("{}".to_string());
    }
    serde_json::to_string_pretty(&config)
        .map_err(|error| AppError::Message(format!("Serialization failed: {error}")))
}

fn extract_codex_common_config(settings: &Value) -> Result<String, AppError> {
    let config_toml = settings.get("config").and_then(Value::as_str).unwrap_or("");
    if config_toml.is_empty() {
        return Ok(String::new());
    }

    let mut doc = config_toml
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| AppError::Message(format!("TOML parse error: {error}")))?;
    let root = doc.as_table_mut();
    for key in [
        "model",
        "model_provider",
        "base_url",
        "wire_api",
        "model_providers",
        "mcp_servers",
        "experimental_bearer_token",
        "model_catalog_json",
    ] {
        root.remove(key);
    }

    if let Some(mcp_table) = root
        .get_mut("mcp")
        .and_then(|item| item.as_table_like_mut())
    {
        mcp_table.remove("servers");
        if mcp_table.is_empty() {
            root.remove("mcp");
        }
    }

    if root
        .get(crate::codex_config::CODEX_WEB_SEARCH_FIELD)
        .and_then(|item| item.as_str())
        == Some(crate::codex_config::CODEX_WEB_SEARCH_DISABLED)
    {
        root.remove(crate::codex_config::CODEX_WEB_SEARCH_FIELD);
    }

    let mut cleaned = String::new();
    let mut blank_run = 0usize;
    for line in doc.to_string().lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                cleaned.push('\n');
            }
            continue;
        }
        blank_run = 0;
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    Ok(cleaned.trim().to_string())
}

fn extract_gemini_common_config(settings: &Value) -> Result<String, AppError> {
    let env = settings.get("env").and_then(Value::as_object);
    let mut snippet = serde_json::Map::new();
    if let Some(env) = env {
        for (key, value) in env {
            if key == "GOOGLE_GEMINI_BASE_URL" || is_sensitive_config_key(key) {
                continue;
            }
            let Value::String(value) = value else {
                continue;
            };
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                snippet.insert(key.to_string(), Value::String(trimmed.to_string()));
            }
        }
    }
    if snippet.is_empty() {
        return Ok("{}".to_string());
    }
    serde_json::to_string_pretty(&Value::Object(snippet))
        .map_err(|error| AppError::Message(format!("Serialization failed: {error}")))
}

fn extract_opencode_common_config(settings: &Value) -> Result<String, AppError> {
    let mut config = settings.clone();
    if let Some(obj) = config.as_object_mut() {
        if let Some(options) = obj.get_mut("options").and_then(Value::as_object_mut) {
            options.remove("apiKey");
            options.remove("baseURL");
        }
    }
    serialize_json_common_config(config)
}

fn extract_openclaw_common_config(settings: &Value) -> Result<String, AppError> {
    let mut config = settings.clone();
    if let Some(obj) = config.as_object_mut() {
        obj.remove("apiKey");
        obj.remove("baseUrl");
    }
    serialize_json_common_config(config)
}

fn serialize_json_common_config(config: Value) -> Result<String, AppError> {
    if config.is_null() || config.as_object().is_some_and(|obj| obj.is_empty()) {
        return Ok("{}".to_string());
    }
    serde_json::to_string_pretty(&config)
        .map_err(|error| AppError::Message(format!("Serialization failed: {error}")))
}
