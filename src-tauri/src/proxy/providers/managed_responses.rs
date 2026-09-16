//! Final wire policy for subscription credentials, independent of the consuming
//! Agent. Runs after editable overrides; API-key and native-client auth routes
//! are deliberately excluded. HTTP, refresh and streaming stay with their owners.

use serde_json::{Map, Value};

use crate::provider::Provider;
use crate::proxy::ProxyError;

const ENCRYPTED_REASONING: &str = "reasoning.encrypted_content";

pub(crate) fn prepare_request(
    provider: &Provider,
    endpoint: &str,
    body: &mut Value,
) -> Result<(), ProxyError> {
    let path = endpoint.split('?').next().unwrap_or(endpoint);
    // Compact has a different schema. Do not inject generation-only fields.
    if !matches!(path, "/responses" | "/v1/responses") {
        return Ok(());
    }
    if provider.is_xai_oauth() {
        super::transform_codex_responses_xai_sanitize::sanitize_xai_responses_request(body);
        prepare_grok(body)?;
    } else if provider.is_codex_oauth() {
        let fields = body.as_object_mut().ok_or_else(invalid_body)?;
        fields.insert("store".into(), Value::Bool(false));
        fields.remove("max_output_tokens");
        let include = fields
            .entry("include")
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(invalid_body)?;
        if !include.iter().all(Value::is_string) {
            return Err(invalid_body());
        }
        let mut seen = std::collections::HashSet::new();
        include.retain(|item| seen.insert(item.as_str().unwrap_or_default().to_owned()));
        if !include
            .iter()
            .any(|item| item.as_str() == Some(ENCRYPTED_REASONING))
        {
            include.push(Value::String(ENCRYPTED_REASONING.into()));
        }
    }
    Ok(())
}

fn invalid_body() -> ProxyError {
    ProxyError::InvalidRequest("Invalid subscription Responses request".into())
}

fn prepare_grok(body: &mut Value) -> Result<(), ProxyError> {
    let fields = body.as_object_mut().ok_or_else(invalid_body)?;
    let model = fields
        .get("model")
        .and_then(Value::as_str)
        .and_then(|model| model.rsplit('/').next())
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .ok_or_else(invalid_body)?
        .to_ascii_lowercase();
    fields.insert("model".into(), Value::String(model));
    hoist_instructions(fields)?;
    // Subscription aliases do not accept the API-key service's reasoning knobs
    // or replayed encrypted state. Function calls/results are not reasoning.
    fields.remove("reasoning");
    fields.remove("prompt_cache_retention");
    if let Some(include) = fields.get_mut("include") {
        let include = include.as_array_mut().ok_or_else(invalid_body)?;
        if !include.iter().all(Value::is_string) {
            return Err(invalid_body());
        }
        include.retain(|item| item.as_str() != Some(ENCRYPTED_REASONING));
        if include.is_empty() {
            fields.remove("include");
        }
    }
    if let Some(format) = fields.remove("response_format") {
        // An explicit Responses text policy wins over a legacy Chat field.
        fields
            .entry("text")
            .or_insert_with(|| serde_json::json!({"format": format}));
    }
    Ok(())
}

fn hoist_instructions(fields: &mut Map<String, Value>) -> Result<(), ProxyError> {
    let mut instructions = match fields.get("instructions") {
        Some(Value::String(text)) => vec![text.clone()],
        Some(_) => return Err(invalid_body()),
        None => Vec::new(),
    };
    let Some(Value::Array(input)) = fields.get_mut("input") else {
        return Ok(());
    };
    let mut retained = Vec::with_capacity(input.len());
    let mut hoisted = false;
    for item in std::mem::take(input) {
        if item.get("type").and_then(Value::as_str) == Some("reasoning") {
            continue;
        }
        let is_message = item.get("type").is_none()
            || item.get("type").and_then(Value::as_str) == Some("message");
        if is_message
            && matches!(
                item.get("role").and_then(Value::as_str),
                Some("system" | "developer")
            )
        {
            let text = instruction_text(item.get("content"))?;
            if !text.trim().is_empty() {
                instructions.push(text);
                hoisted = true;
            }
            continue;
        }
        if is_message
            && item.get("role").is_some()
            && item.get("content").and_then(Value::as_str) == Some("")
        {
            continue;
        }
        retained.push(item);
    }
    *input = retained;
    if hoisted {
        instructions.retain(|text| !text.is_empty());
        fields.insert(
            "instructions".into(),
            Value::String(instructions.join("\n\n")),
        );
    }
    Ok(())
}

fn instruction_text(content: Option<&Value>) -> Result<String, ProxyError> {
    match content {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|part| {
                part.as_str()
                    .or_else(|| part.get("text").and_then(Value::as_str))
                    .map(str::to_owned)
                    .ok_or_else(invalid_body)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|parts| parts.join("\n")),
        // Reject rather than silently erase non-text system instructions.
        _ => Err(invalid_body()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderMeta;
    use serde_json::json;

    fn provider(kind: &str) -> Provider {
        let mut provider = Provider::with_id("fixture".into(), "fixture".into(), json!({}), None);
        provider.meta = Some(ProviderMeta {
            provider_type: Some(kind.into()),
            ..Default::default()
        });
        provider
    }

    #[test]
    fn openai_final_policy_is_idempotent_and_does_not_touch_native_or_api_key() {
        let original =
            json!({"store":true,"max_output_tokens":123,"input":"hello", "include":["foo","foo"]});
        let mut body = original.clone();
        prepare_request(&provider("codex_oauth"), "/responses", &mut body).unwrap();
        assert_eq!(
            body,
            json!({"store":false,"input":"hello","include":["foo",ENCRYPTED_REASONING]})
        );
        let expected = body.clone();
        prepare_request(&provider("codex_oauth"), "/v1/responses?test=1", &mut body).unwrap();
        assert_eq!(body, expected);
        for (kind, path) in [
            ("codex", "/responses"),
            ("codex_oauth", "/responses/compact"),
        ] {
            let mut unchanged = original.clone();
            prepare_request(&provider(kind), path, &mut unchanged).unwrap();
            assert_eq!(unchanged, original);
        }
    }

    #[test]
    fn grok_keeps_tool_roundtrip_and_hoists_instructions_without_private_reasoning() {
        let mut body = json!({
            "model":"grok-cli/GROK-BUILD", "instructions":"existing",
            "input":[
                {"role":"system","content":"system"},
                {"role":"developer","content":[{"type":"input_text","text":"developer"}]},
                {"type":"reasoning","encrypted_content":"private"},
                {"role":"assistant","content":""},
                {"type":"function_call","call_id":"call1","name":"tool","arguments":"{}"},
                {"type":"function_call_output","call_id":"call1","output":""},
                {"role":"user","content":"hello"}
            ],
            "reasoning":{"effort":"high"},"include":[ENCRYPTED_REASONING,"web_search_call.action.sources"],
            "prompt_cache_retention":"24h", "response_format":{"type":"json_object"}
        });
        prepare_request(&provider("xai_oauth"), "/responses", &mut body).unwrap();
        assert_eq!(body["model"], "grok-build");
        assert_eq!(body["instructions"], "existing\n\nsystem\n\ndeveloper");
        assert_eq!(body["input"].as_array().unwrap().len(), 3);
        assert_eq!(body["input"][0]["type"], "function_call");
        assert_eq!(body["input"][1]["output"], "");
        assert_eq!(body["include"], json!(["web_search_call.action.sources"]));
        assert_eq!(body["text"]["format"]["type"], "json_object");
        for key in ["reasoning", "prompt_cache_retention", "response_format"] {
            assert!(body.get(key).is_none());
        }
        let expected = body.clone();
        prepare_request(&provider("xai_oauth"), "/responses", &mut body).unwrap();
        assert_eq!(body, expected);
    }

    #[test]
    fn unsupported_instruction_content_is_rejected_not_silently_dropped() {
        let mut body = json!({"model":"grok-build","input":[{"role":"system","content":[{"type":"input_image","image_url":"secret"}]}]});
        let error = prepare_request(&provider("xai_oauth"), "/responses", &mut body).unwrap_err();
        assert!(!error.to_string().contains("secret"));
    }
}
