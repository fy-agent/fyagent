use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::provider::Provider;

use super::domain::ChangePlanErrorCode;
use super::sanitize::{is_safe_opaque_id, sanitize_display_name};

pub(crate) fn digest_json(domain: &str, value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(
        serde_json::to_vec(&canonical_json(value))
            .expect("serde_json::Value serialization is infallible"),
    );
    format!("{:x}", hasher.finalize())
}

pub(crate) fn digest_serializable<T: Serialize>(
    domain: &str,
    value: &T,
) -> Result<String, ChangePlanErrorCode> {
    let value = serde_json::to_value(value).map_err(|_| ChangePlanErrorCode::Internal)?;
    Ok(digest_json(domain, &value))
}

pub(crate) fn workbuddy_file_digest(bytes: Option<&[u8]>) -> String {
    digest_json(
        "fyagent.change-plan.workbuddy-file.v1",
        &json!({
            "present": bytes.is_some(),
            "sha256": bytes.map(|value| format!("{:x}", Sha256::digest(value))),
        }),
    )
}

pub(crate) fn workbuddy_baseline_digest(
    config_digest: &str,
    backup_digest: &str,
    revision: Option<&str>,
) -> String {
    digest_json(
        "fyagent.change-plan.workbuddy-baseline.v1",
        &json!({
            "configDigest": config_digest,
            "backupDigest": backup_digest,
            "revision": revision,
        }),
    )
}

pub(crate) fn provider_definition_digest(
    provider: &Provider,
) -> Result<String, ChangePlanErrorCode> {
    if !is_safe_opaque_id(&provider.id) {
        return Err(ChangePlanErrorCode::InvalidTarget);
    }
    let category = provider
        .category
        .as_deref()
        .filter(|value| is_safe_opaque_id(value))
        .unwrap_or("unclassified");
    let projection = json!({
        "id": provider.id,
        "name": sanitize_display_name(&provider.name),
        "category": category,
        "codex": credential_neutral_codex_projection(&provider.settings_config)?,
    });
    Ok(digest_json(
        "fyagent.change-plan.provider-definition.v2",
        &projection,
    ))
}

/// Build only the routing/model shape needed to compare Codex projections.
/// Authentication objects, headers, bearer fields, query strings, paths and
/// all unknown TOML fields are deliberately absent before hashing.
pub(crate) fn credential_neutral_codex_projection(
    settings: &Value,
) -> Result<Value, ChangePlanErrorCode> {
    let config = settings
        .get("config")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let parsed = config
        .parse::<toml::Value>()
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    let table = parsed.as_table().ok_or(ChangePlanErrorCode::Internal)?;

    let mut projection = Map::new();
    copy_semantic_string(table, "model_provider", &mut projection)?;
    copy_semantic_string(table, "model", &mut projection)?;
    copy_semantic_string(table, "review_model", &mut projection)?;

    if let Some(provider_id) = table.get("model_provider").and_then(toml::Value::as_str) {
        let provider_table = table
            .get("model_providers")
            .and_then(toml::Value::as_table)
            .and_then(|providers| providers.get(provider_id))
            .and_then(toml::Value::as_table);
        if let Some(provider_table) = provider_table {
            let mut provider_projection = Map::new();
            copy_semantic_string(provider_table, "name", &mut provider_projection)?;
            copy_semantic_string(provider_table, "wire_api", &mut provider_projection)?;
            copy_bool(
                provider_table,
                "requires_openai_auth",
                &mut provider_projection,
            )?;
            copy_bool(
                provider_table,
                "supports_websockets",
                &mut provider_projection,
            )?;
            if let Some(base_url) = provider_table.get("base_url") {
                let base_url = base_url.as_str().ok_or(ChangePlanErrorCode::Internal)?;
                provider_projection.insert(
                    "base_url_origin".to_string(),
                    Value::String(url_origin(base_url)?),
                );
            }
            projection.insert(
                "active_provider".to_string(),
                Value::Object(provider_projection),
            );
        }
    }

    Ok(Value::Object(projection))
}

fn copy_semantic_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    output: &mut Map<String, Value>,
) -> Result<(), ChangePlanErrorCode> {
    let Some(value) = table.get(key) else {
        return Ok(());
    };
    let value = value.as_str().ok_or(ChangePlanErrorCode::Internal)?;
    if value.is_empty() || value.chars().count() > 256 || value.chars().any(char::is_control) {
        return Err(ChangePlanErrorCode::Internal);
    }
    output.insert(key.to_string(), Value::String(value.to_string()));
    Ok(())
}

fn copy_bool(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    output: &mut Map<String, Value>,
) -> Result<(), ChangePlanErrorCode> {
    let Some(value) = table.get(key) else {
        return Ok(());
    };
    output.insert(
        key.to_string(),
        Value::Bool(value.as_bool().ok_or(ChangePlanErrorCode::Internal)?),
    );
    Ok(())
}

fn url_origin(value: &str) -> Result<String, ChangePlanErrorCode> {
    let parsed = url::Url::parse(value).map_err(|_| ChangePlanErrorCode::Internal)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(ChangePlanErrorCode::Internal);
    }
    let host = parsed.host_str().ok_or(ChangePlanErrorCode::Internal)?;
    let mut origin = format!("{}://{}", parsed.scheme(), host.to_ascii_lowercase());
    if let Some(port) = parsed.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    Ok(origin)
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(left, _)| *left);
            let mut result = Map::new();
            for (key, value) in entries {
                result.insert(key.clone(), canonical_json(value));
            }
            Value::Object(result)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings(secret: &str, model: &str) -> Value {
        json!({
            "auth": {"OPENAI_API_KEY": secret},
            "config": format!(
                "model_provider = \"custom\"\nmodel = \"{model}\"\n\
                 [model_providers.custom]\nname = \"Custom\"\nbase_url = \"https://example.test/v1?token={secret}\"\n\
                 wire_api = \"responses\"\nexperimental_bearer_token = \"{secret}\"\n\
                 [model_providers.custom.http_headers]\nAuthorization = \"Bearer {secret}\"\n"
            )
        })
    }

    #[test]
    fn credential_neutral_projection_excludes_all_secret_bearing_surfaces() {
        let left = credential_neutral_codex_projection(&settings("sentinel-one", "gpt-5"))
            .expect("projection");
        let right = credential_neutral_codex_projection(&settings("sentinel-two", "gpt-5"))
            .expect("projection");
        assert_eq!(
            left, right,
            "credentials must not influence the digest input"
        );
        let serialized = serde_json::to_string(&left).unwrap();
        for forbidden in [
            "sentinel",
            "experimental_bearer_token",
            "Authorization",
            "token=",
            "/v1",
        ] {
            assert!(!serialized.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn credential_neutral_projection_still_detects_routing_model_drift() {
        let left = credential_neutral_codex_projection(&settings("same", "gpt-5")).unwrap();
        let right = credential_neutral_codex_projection(&settings("same", "gpt-5.1")).unwrap();
        assert_ne!(
            digest_json("projection", &left),
            digest_json("projection", &right)
        );
    }
}
