//! Pure Provider projections shared by IPC and portable exports. No native I/O.
use super::Provider;
use serde_json::Value;
const REF_KEY: &str = "credentialRef";

pub(crate) fn is_sensitive_config_key(name: &str) -> bool {
    let upper = name.replace('-', "_").to_ascii_uppercase();
    let collapsed = upper.replace('_', "");
    if matches!(
        collapsed.as_str(),
        "AUTH"
            | "AUTHORIZATION"
            | "PROXYAUTHORIZATION"
            | "COOKIE"
            | "SETCOOKIE"
            | "ACCESSTOKEN"
            | "REFRESHTOKEN"
            | "IDTOKEN"
            | "APIKEY"
            | "XAPIKEY"
            | "HEADERS"
            | "HTTPHEADERS"
            | "USAGESCRIPT"
    ) {
        return true;
    }

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
        "KEY",
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

fn collect_secrets(value: &Value, out: &mut Vec<String>) {
    fn leaves(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::String(s) if !s.is_empty() => out.push(s.clone()),
            Value::Object(m) => {
                for v in m.values() {
                    leaves(v, out);
                }
            }
            Value::Array(a) => {
                for v in a {
                    leaves(v, out);
                }
            }
            _ => {}
        }
    }
    match value {
        Value::Object(m) => {
            for (key, value) in m {
                if is_sensitive_config_key(key) {
                    leaves(value, out);
                } else {
                    collect_secrets(value, out);
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                collect_secrets(v, out);
            }
        }
        _ => {}
    }
}

/// Projection for renderer/ordinary export. It never resolves a native handle.
pub(crate) fn sanitize_provider_for_export(provider: &Provider) -> Provider {
    let mut known_keys = Vec::new();
    collect_secrets(
        &serde_json::to_value(provider).unwrap_or(Value::Null),
        &mut known_keys,
    );
    if let Some(config) = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
    {
        if let Ok(parsed) = toml::from_str::<toml::Value>(config) {
            collect_secrets(
                &serde_json::to_value(parsed).unwrap_or(Value::Null),
                &mut known_keys,
            );
        }
    }
    fn scrub(value: &mut Value) {
        match value {
            Value::Object(object) => {
                object.retain(|name, _| name != REF_KEY && !is_sensitive_config_key(name));
                for value in object.values_mut() {
                    scrub(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    scrub(value);
                }
            }
            _ => {}
        }
    }
    fn remove_known(value: &mut Value, secret: &str) {
        match value {
            Value::String(text) => *text = text.replace(secret, "[REDACTED]"),
            Value::Object(object) => {
                object.retain(|name, _| !name.contains(secret));
                for value in object.values_mut() {
                    remove_known(value, secret);
                }
            }
            Value::Array(values) => {
                for value in values {
                    remove_known(value, secret);
                }
            }
            _ => {}
        }
    }
    let mut clean = provider.clone();
    scrub(&mut clean.settings_config);
    if let Some(config) = clean.settings_config.get("config").and_then(Value::as_str) {
        // TOML strings need structured redaction; malformed input has no safe
        // ordinary export. Keep an empty shape so it requires user repair.
        fn scrub_table(table: &mut dyn toml_edit::TableLike) {
            let keys = table
                .iter()
                .map(|(name, _)| name.to_owned())
                .collect::<Vec<_>>();
            for name in keys {
                if is_sensitive_config_key(&name) {
                    table.remove(&name);
                } else if let Some(array) = table
                    .get_mut(&name)
                    .and_then(toml_edit::Item::as_array_of_tables_mut)
                {
                    for child in array.iter_mut() {
                        scrub_table(child);
                    }
                } else if let Some(child) = table
                    .get_mut(&name)
                    .and_then(toml_edit::Item::as_table_like_mut)
                {
                    scrub_table(child);
                }
            }
        }
        let config = config
            .parse::<toml_edit::DocumentMut>()
            .map(|mut document| {
                scrub_table(document.as_table_mut());
                document.to_string()
            })
            .unwrap_or_default();
        clean.settings_config["config"] = Value::String(config);
    }
    if let Some(meta) = clean.meta.as_mut() {
        if let Some(usage) = meta.usage_script.as_mut() {
            usage.api_key = None;
        }
    }
    if let Ok(mut value) = serde_json::to_value(&clean) {
        scrub(&mut value);
        for secret in known_keys {
            if !secret.is_empty() {
                remove_known(&mut value, &secret);
            }
        }
        if let Ok(redacted) = serde_json::from_value(value) {
            clean = redacted;
        }
    }
    clean
}

/// Ordinary SQL exports use a conservative connection/model whitelist. Free
/// form scripts, environment variables, authentication and unknown extensions
/// have no portable confidentiality contract and are omitted. Binary backups
/// remain the private, lossless recovery format.
pub(crate) fn portable_provider_for_export(provider: &Provider) -> Provider {
    fn project(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut safe = serde_json::Map::new();
                for (name, value) in map {
                    if is_sensitive_config_key(name) {
                        continue;
                    }
                    let upper = name.to_ascii_uppercase();
                    let semantic = matches!(
                        name.as_str(),
                        "name"
                            | "id"
                            | "model"
                            | "model_provider"
                            | "review_model"
                            | "model_reasoning_effort"
                            | "wire_api"
                            | "requires_openai_auth"
                            | "supports_websockets"
                            | "base_url"
                            | "baseURL"
                            | "baseUrl"
                            | "api"
                            | "type"
                            | "defaultModel"
                            | "contextWindow"
                            | "maxTokens"
                    ) || matches!(
                        upper.as_str(),
                        "ANTHROPIC_BASE_URL"
                            | "ANTHROPIC_MODEL"
                            | "ANTHROPIC_DEFAULT_HAIKU_MODEL"
                            | "ANTHROPIC_DEFAULT_SONNET_MODEL"
                            | "ANTHROPIC_DEFAULT_OPUS_MODEL"
                            | "GOOGLE_GEMINI_BASE_URL"
                            | "GEMINI_MODEL"
                            | "OPENAI_BASE_URL"
                            | "OPENAI_MODEL"
                    );
                    if value.is_object() || value.is_array() {
                        let nested = project(value);
                        if nested.as_object().is_some_and(|v| !v.is_empty())
                            || nested.as_array().is_some_and(|v| !v.is_empty())
                        {
                            safe.insert(name.clone(), nested);
                        }
                    } else if semantic {
                        if let Some(text) = value.as_str() {
                            if upper.contains("URL") {
                                if let Ok(mut url) = url::Url::parse(text) {
                                    if matches!(url.scheme(), "http" | "https") {
                                        let _ = url.set_username("");
                                        let _ = url.set_password(None);
                                        url.set_query(None);
                                        url.set_fragment(None);
                                        safe.insert(name.clone(), Value::String(url.to_string()));
                                    }
                                }
                            } else {
                                safe.insert(name.clone(), value.clone());
                            }
                        } else {
                            safe.insert(name.clone(), value.clone());
                        }
                    }
                }
                Value::Object(safe)
            }
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .filter(|v| v.is_object())
                    .map(project)
                    .collect(),
            ),
            _ => Value::Null,
        }
    }
    let mut clean = sanitize_provider_for_export(provider);
    let config = clean
        .settings_config
        .get("config")
        .and_then(Value::as_str)
        .map(str::to_owned);
    clean.settings_config = project(&clean.settings_config);
    if let Some(config) = config {
        let text = config
            .parse::<toml::Value>()
            .ok()
            .and_then(|parsed| serde_json::to_value(parsed).ok())
            .and_then(|parsed| serde_json::from_value::<toml::Value>(project(&parsed)).ok())
            .and_then(|parsed| toml::to_string(&parsed).ok())
            .unwrap_or_default();
        clean.settings_config["config"] = Value::String(text);
    }
    clean.meta = None;
    clean.notes = None;
    clean.in_failover_queue = false;
    clean
}

pub(crate) fn provider_contains_credentials(provider: &Provider) -> bool {
    let mut values = Vec::new();
    collect_secrets(
        &serde_json::to_value(provider).unwrap_or(Value::Null),
        &mut values,
    );
    if let Some(config) = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
    {
        match config.parse::<toml::Value>() {
            Ok(parsed) => collect_secrets(
                &serde_json::to_value(parsed).unwrap_or(Value::Null),
                &mut values,
            ),
            Err(_) => return true,
        }
    }
    !values.is_empty()
}
