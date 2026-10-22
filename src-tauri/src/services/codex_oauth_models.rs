//! Codex OAuth model list service.
//!
//! ChatGPT Codex exposes models through `chatgpt.com/backend-api/codex/models`,
//! which is not an OpenAI-compatible `/v1/models` endpoint.

use crate::proxy::providers::codex_oauth_auth::{
    CODEX_OAUTH_CLIENT_VERSION, CODEX_OAUTH_ORIGINATOR,
};
use crate::services::model_fetch::FetchedModel;
use serde_json::Value;
use std::time::Duration;

pub(crate) const CODEX_OAUTH_MODELS_URL: &str = "https://chatgpt.com/backend-api/codex/models";
const CODEX_OAUTH_FETCH_TIMEOUT_SECS: u64 = 15;
const ERROR_BODY_MAX_CHARS: usize = 512;

pub async fn fetch_models_with_token(
    token: &str,
    account_id: &str,
) -> Result<Vec<FetchedModel>, String> {
    let client = crate::proxy::http_client::get();
    let response = build_models_request(&client, token, account_id)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = truncate_body(response.text().await.unwrap_or_default());
        return Err(format!("HTTP {status}: {body}"));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    Ok(parse_models(value))
}

/// Upper bound on picker rows accepted from one catalog response.
const MAX_LISTED_MODELS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListedModelsError {
    /// The account token was rejected; the user has to log in again.
    Unauthorized,
    /// Network, redirect or non-success status.
    Unavailable,
    /// The body is not the Codex `{ "models": [...] }` catalog.
    InvalidResponse,
}

/// The account's own Codex picker: the rows the official client lists.
/// Rows marked `visibility: hide|none` stay out, as they do in Codex.
pub(crate) async fn fetch_listed_models_with_token(
    token: &str,
    account_id: &str,
) -> Result<Vec<String>, ListedModelsError> {
    let client = crate::proxy::http_client::get();
    let response = build_models_request(&client, token, account_id)
        .send()
        .await
        .map_err(|_| ListedModelsError::Unavailable)?;
    if response.url().host_str() != Some("chatgpt.com") {
        return Err(ListedModelsError::Unavailable);
    }
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(ListedModelsError::Unauthorized);
    }
    if !status.is_success() {
        return Err(ListedModelsError::Unavailable);
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| ListedModelsError::InvalidResponse)?;
    parse_listed_models(&value)
}

fn parse_listed_models(value: &Value) -> Result<Vec<String>, ListedModelsError> {
    let entries = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or(ListedModelsError::InvalidResponse)?;
    let mut ids: Vec<String> = Vec::new();
    for entry in entries {
        let obj = entry
            .as_object()
            .ok_or(ListedModelsError::InvalidResponse)?;
        if matches!(
            obj.get("visibility").and_then(Value::as_str),
            Some("hide" | "none")
        ) {
            continue;
        }
        let Some(id) = obj
            .get("slug")
            .and_then(Value::as_str)
            .filter(|id| is_picker_model_id(id))
        else {
            continue;
        };
        if !ids.iter().any(|known| known == id) {
            ids.push(id.to_string());
        }
    }
    if ids.len() > MAX_LISTED_MODELS {
        return Err(ListedModelsError::InvalidResponse);
    }
    Ok(ids)
}

/// Same admission as the subscription bind request's model ID.
fn is_picker_model_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':'))
}

fn build_models_request(
    client: &reqwest::Client,
    token: &str,
    account_id: &str,
) -> reqwest::RequestBuilder {
    client
        .get(CODEX_OAUTH_MODELS_URL)
        .query(&[("client_version", CODEX_OAUTH_CLIENT_VERSION)])
        .header("Authorization", format!("Bearer {token}"))
        .header("originator", CODEX_OAUTH_ORIGINATOR)
        .header("version", CODEX_OAUTH_CLIENT_VERSION)
        .header("chatgpt-account-id", account_id)
        .timeout(Duration::from_secs(CODEX_OAUTH_FETCH_TIMEOUT_SECS))
}

fn parse_models(value: Value) -> Vec<FetchedModel> {
    let entries = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .or_else(|| value.get("items").and_then(Value::as_array))
        .or_else(|| value.as_array());

    let mut models = Vec::new();

    if let Some(entries) = entries {
        for entry in entries {
            push_model_entry(&mut models, entry, None);
        }
    }

    if let Some(model_map) = value.get("models").and_then(Value::as_object) {
        for (key, entry) in model_map {
            push_model_entry(&mut models, entry, Some(key));
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
}

fn push_model_entry(models: &mut Vec<FetchedModel>, entry: &Value, fallback_id: Option<&str>) {
    if let Some(id) = entry.as_str().map(str::trim).filter(|id| !id.is_empty()) {
        models.push(FetchedModel {
            id: id.to_string(),
            owned_by: Some("Codex".to_string()),
        });
        return;
    }

    let Some(obj) = entry.as_object() else {
        if let Some(id) = fallback_id.map(str::trim).filter(|id| !id.is_empty()) {
            models.push(FetchedModel {
                id: id.to_string(),
                owned_by: Some("Codex".to_string()),
            });
        }
        return;
    };

    let Some(id) = string_field(obj, &["slug", "id", "model", "name"]).or_else(|| {
        fallback_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
    }) else {
        return;
    };
    let owned_by = string_field(
        obj,
        &[
            "owned_by", "ownedBy", "provider", "vendor", "category", "owner",
        ],
    )
    .or_else(|| Some("Codex".to_string()));

    models.push(FetchedModel { id, owned_by });
}

fn string_field(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| obj.get(*key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn truncate_body(body: String) -> String {
    if body.chars().count() <= ERROR_BODY_MAX_CHARS {
        body
    } else {
        let mut s: String = body.chars().take(ERROR_BODY_MAX_CHARS).collect();
        s.push_str("...");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn codex_oauth_model_discovery_uses_gpt6_compatible_identity() {
        let request = build_models_request(&reqwest::Client::new(), "test-token", "test-account")
            .build()
            .unwrap();
        assert_eq!(request.headers()["authorization"], "Bearer test-token");
        assert_eq!(request.headers()["chatgpt-account-id"], "test-account");
        assert_eq!(request.headers()["originator"], "codex_cli_rs");
        let version = request
            .url()
            .query_pairs()
            .find(|(key, _)| key == "client_version")
            .unwrap()
            .1
            .into_owned();
        let parts: Vec<u32> = version
            .split('.')
            .map(|part| part.parse().unwrap())
            .collect();
        // GPT-6.1 Sol is absent from the 0.155.0 catalog for this account; the
        // catalog gate sits above the model's advertised minimal_client_version.
        assert!(parts.as_slice() >= [0, 159, 0].as_slice());
        assert_eq!(request.headers()["version"], version);
        let models = parse_models(json!({"models": [
            {"slug": "gpt-6.1-sol", "minimal_client_version": "0.153.0"},
            {"slug": "gpt-6-sol", "minimal_client_version": "0.155.0"}
        ]}));
        assert_eq!(models[0].id, "gpt-6-sol");
        assert_eq!(models[1].id, "gpt-6.1-sol");
    }

    #[test]
    fn parse_codex_oauth_models_accepts_openai_style_data() {
        let models = parse_models(json!({
            "data": [
                { "id": "gpt-5.4", "owned_by": "openai" },
                { "id": "gpt-5.4-mini", "ownedBy": "openai" }
            ]
        }));

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-5.4");
        assert_eq!(models[0].owned_by.as_deref(), Some("openai"));
        assert_eq!(models[1].id, "gpt-5.4-mini");
        assert_eq!(models[1].owned_by.as_deref(), Some("openai"));
    }

    #[test]
    fn parse_codex_oauth_models_accepts_model_list_shape() {
        let models = parse_models(json!({
            "models": [
                { "slug": "gpt-5.3-codex", "display_name": "GPT-5.3 Codex" },
                "gpt-5.5"
            ]
        }));

        assert_eq!(
            models.into_iter().map(|model| model.id).collect::<Vec<_>>(),
            vec!["gpt-5.3-codex".to_string(), "gpt-5.5".to_string()]
        );
    }

    #[test]
    fn parse_codex_oauth_models_deduplicates_ids() {
        let models = parse_models(json!({
            "data": [
                { "id": "gpt-5.4" },
                { "model": "gpt-5.4" }
            ]
        }));

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-5.4");
    }

    #[test]
    fn listed_models_keep_picker_rows_in_server_order() {
        let models = parse_listed_models(&json!({"models": [
            {"slug": "gpt-6-sol", "visibility": "list"},
            {"slug": "codex-auto-review", "visibility": "hide"},
            {"slug": "internal-row", "visibility": "none"},
            {"slug": "gpt-5.5"},
            {"slug": "gpt-6-sol", "visibility": "list"},
            {"slug": "bad id with spaces"},
            {"display_name": "no slug"}
        ]}))
        .unwrap();
        assert_eq!(models, vec!["gpt-6-sol".to_string(), "gpt-5.5".to_string()]);
    }

    #[test]
    fn listed_models_reject_non_catalog_shapes() {
        for body in [
            json!({"data": [{"id": "gpt-5.5"}]}),
            json!({"models": {"gpt-5.5": {}}}),
            json!({"models": ["gpt-5.5"]}),
            json!([]),
        ] {
            assert_eq!(
                parse_listed_models(&body),
                Err(ListedModelsError::InvalidResponse),
                "{body}"
            );
        }
        let too_many: Vec<Value> = (0..=MAX_LISTED_MODELS)
            .map(|index| json!({"slug": format!("m-{index}")}))
            .collect();
        assert_eq!(
            parse_listed_models(&json!({ "models": too_many })),
            Err(ListedModelsError::InvalidResponse)
        );
        assert_eq!(parse_listed_models(&json!({"models": []})), Ok(Vec::new()));
    }

    #[test]
    fn parse_codex_oauth_models_accepts_model_map_shape() {
        let models = parse_models(json!({
            "models": {
                "gpt-5.4": { "display_name": "GPT-5.4" },
                "gpt-5.5": { "slug": "gpt-5.5" }
            }
        }));

        assert_eq!(
            models.into_iter().map(|model| model.id).collect::<Vec<_>>(),
            vec!["gpt-5.4".to_string(), "gpt-5.5".to_string()]
        );
    }
}
