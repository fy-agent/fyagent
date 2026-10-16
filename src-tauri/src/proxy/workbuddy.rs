//! Dedicated WorkBuddy chat route.
//!
//! G0 (WorkBuddy 5.8.2, OpenAI JS SDK) posts Chat Completions with `stream: true`
//! to `{base}/chat/completions`. The saved base ends in `/v1`, so the client path
//! is `/v1/chat/completions`. This route keeps that shape under `/workbuddy` and
//! does not share Codex's `/v1/chat/completions`.
//!
//! Knife 2 would point the client at `http://127.0.0.1:{port}/workbuddy/v1`.
//! This module does not write `~/.workbuddy/models.json`.

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use http::Extensions;
use http_body_util::BodyExt;
use serde_json::{json, Map, Value};
use tauri::Manager;

use crate::app_config::AppType;
use crate::commands::ManagedAuthState;
use crate::services::workbuddy::error::WorkBuddySubscriptionErrorCode;
use crate::services::workbuddy::subscription::{self, WorkBuddySubscriptionRecord};

use super::forwarder::RequestForwarder;
use super::server::ProxyState;
use super::types::{CopilotOptimizerConfig, OptimizerConfig, RectifierConfig};
use super::ProxyError;

pub(crate) const WORKBUDDY_CHAT_COMPLETIONS_PATH: &str = "/workbuddy/v1/chat/completions";

const MAX_UPSTREAM_BODY: usize = 2 * 1024 * 1024;
const DEFAULT_UPSTREAM_TIMEOUT_SECS: u64 = 20;

#[derive(Clone, Copy)]
pub(crate) struct WorkbuddyProxyCredentialAccepted;

#[cfg(test)]
static UPSTREAM_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(crate) fn set_workbuddy_upstream_timeout_for_test(seconds: u64) {
    UPSTREAM_TIMEOUT_SECS.store(seconds, Ordering::SeqCst);
}

fn upstream_timeout_secs() -> u64 {
    #[cfg(test)]
    {
        let override_secs = UPSTREAM_TIMEOUT_SECS.load(Ordering::SeqCst);
        if override_secs > 0 {
            return override_secs;
        }
    }
    DEFAULT_UPSTREAM_TIMEOUT_SECS
}

pub(crate) fn proxy_credential_rejected_response() -> Response {
    closed(
        StatusCode::UNAUTHORIZED,
        WorkBuddySubscriptionErrorCode::ProxyCredentialRejected,
    )
}

pub async fn handle_chat_completions(
    State(state): State<ProxyState>,
    request: axum::extract::Request,
) -> Response {
    if request
        .extensions()
        .get::<WorkbuddyProxyCredentialAccepted>()
        .is_none()
    {
        return proxy_credential_rejected_response();
    }
    if request.method() != http::Method::POST {
        return closed(
            StatusCode::METHOD_NOT_ALLOWED,
            WorkBuddySubscriptionErrorCode::UnknownRoute,
        );
    }
    let (_parts, body) = request.with_limited_body().into_parts();
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return closed(
                StatusCode::BAD_REQUEST,
                WorkBuddySubscriptionErrorCode::InvalidRequest,
            )
        }
    };
    let chat: Value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) if value.is_object() => value,
        _ => {
            return closed(
                StatusCode::BAD_REQUEST,
                WorkBuddySubscriptionErrorCode::InvalidRequest,
            )
        }
    };
    let record = match subscription::load_record(&state.db) {
        Ok(Some(record)) if record.is_servable() => record,
        _ => {
            return closed(
                StatusCode::NOT_FOUND,
                WorkBuddySubscriptionErrorCode::SubscriptionUnavailable,
            )
        }
    };
    let requested = chat.get("model").and_then(Value::as_str).unwrap_or("");
    if requested != record.model_id() {
        return closed(
            StatusCode::BAD_REQUEST,
            WorkBuddySubscriptionErrorCode::ModelMismatch,
        );
    }
    if admit(&state, &record).is_err() {
        subscription::mark_account_unavailable(&state.db);
        return closed(
            StatusCode::UNAUTHORIZED,
            WorkBuddySubscriptionErrorCode::AccountUnavailable,
        );
    }
    let Some(responses) = chat_to_responses(&chat, record.model_id()) else {
        return closed(
            StatusCode::BAD_REQUEST,
            WorkBuddySubscriptionErrorCode::InvalidRequest,
        );
    };
    let Some(provider) = subscription::build_provider(&record) else {
        return closed(
            StatusCode::NOT_FOUND,
            WorkBuddySubscriptionErrorCode::SubscriptionUnavailable,
        );
    };
    let client_stream = chat.get("stream").and_then(Value::as_bool).unwrap_or(false);
    let forwarder = build_forwarder(&state, &provider.id);
    let forwarded = forwarder
        .forward_with_retry(
            &AppType::Codex,
            http::Method::POST,
            "/responses",
            responses,
            HeaderMap::new(),
            Extensions::new(),
            vec![provider],
        )
        .await;
    let result = match forwarded {
        Ok(result) => result,
        Err(error) => return map_forward_error(error.error),
    };
    match render_client_response(result.response, record.model_id(), client_stream).await {
        Ok(response) => response,
        Err(code) => closed(
            StatusCode::from_u16(code.http_status()).unwrap_or(StatusCode::BAD_GATEWAY),
            code,
        ),
    }
}

pub async fn reject_unknown_route(request: axum::extract::Request) -> Response {
    if request
        .extensions()
        .get::<WorkbuddyProxyCredentialAccepted>()
        .is_none()
    {
        return proxy_credential_rejected_response();
    }
    closed(
        StatusCode::NOT_FOUND,
        WorkBuddySubscriptionErrorCode::UnknownRoute,
    )
}

fn admit(state: &ProxyState, record: &WorkBuddySubscriptionRecord) -> Result<(), ()> {
    #[cfg(test)]
    if let Some(auth) = subscription::workbuddy_subscription_auth_for_test() {
        return subscription::admit_record(auth.as_ref(), record)
            .map(|_| ())
            .map_err(|_| ());
    }
    let handle = state.app_handle.as_ref().ok_or(())?;
    let managed = handle.try_state::<ManagedAuthState>().ok_or(())?;
    subscription::admit_record(managed.0.as_ref(), record)
        .map(|_| ())
        .map_err(|_| ())
}

fn build_forwarder(state: &ProxyState, provider_id: &str) -> RequestForwarder {
    let timeout = upstream_timeout_secs();
    let mut rectifier = RectifierConfig::default();
    rectifier.enabled = false;
    rectifier.request_thinking_signature = false;
    rectifier.request_thinking_budget = false;
    rectifier.request_media_fallback = false;
    rectifier.request_media_heuristic = false;
    RequestForwarder::new(
        state.provider_router.clone(),
        timeout,
        state.status.clone(),
        state.current_providers.clone(),
        state.gemini_shadow.clone(),
        state.codex_chat_history.clone(),
        state.failover_manager.clone(),
        state.app_handle.clone(),
        provider_id.to_string(),
        "workbuddy".to_string(),
        false,
        timeout,
        timeout,
        rectifier,
        OptimizerConfig::default(),
        CopilotOptimizerConfig {
            enabled: false,
            ..CopilotOptimizerConfig::default()
        },
        0,
    )
    .stack_request(true)
}

fn chat_to_responses(chat: &Value, model_id: &str) -> Option<Value> {
    let messages = chat.get("messages")?.as_array()?;
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in messages {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        match role {
            "system" | "developer" => {
                if let Some(text) = text_from_content(message.get("content")) {
                    if !text.is_empty() {
                        instructions.push(text);
                    }
                }
            }
            "user" => {
                let text = text_from_content(message.get("content")).unwrap_or_default();
                input.push(json!({
                    "role": "user",
                    "content": [{ "type": "input_text", "text": text }]
                }));
            }
            "assistant" => {
                if let Some(text) = text_from_content(message.get("content")) {
                    if !text.is_empty() {
                        input.push(json!({
                            "role": "assistant",
                            "content": [{ "type": "output_text", "text": text }]
                        }));
                    }
                }
                if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        let call_id = call.get("id").and_then(Value::as_str)?;
                        let function = call.get("function")?;
                        let name = function.get("name").and_then(Value::as_str)?;
                        let arguments = function
                            .get("arguments")
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        input.push(json!({
                            "type": "function_call",
                            "call_id": call_id,
                            "name": name,
                            "arguments": arguments
                        }));
                    }
                }
            }
            "tool" => {
                let call_id = message.get("tool_call_id").and_then(Value::as_str)?;
                let output = text_from_content(message.get("content")).unwrap_or_default();
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output
                }));
            }
            _ => {}
        }
    }
    let mut body = json!({
        "model": model_id,
        "stream": chat.get("stream").and_then(Value::as_bool).unwrap_or(false),
        "instructions": instructions.join("\n\n"),
        "input": input
    });
    if let Some(tools) = chat.get("tools").and_then(Value::as_array) {
        let converted: Vec<Value> = tools.iter().filter_map(convert_tool).collect();
        if !converted.is_empty() {
            body["tools"] = Value::Array(converted);
        }
    }
    Some(body)
}

fn convert_tool(tool: &Value) -> Option<Value> {
    if tool.get("type").and_then(Value::as_str) != Some("function") {
        return None;
    }
    let function = tool.get("function")?;
    let name = function.get("name").and_then(Value::as_str)?;
    let mut converted = Map::new();
    converted.insert("type".to_string(), json!("function"));
    converted.insert("name".to_string(), json!(name));
    if let Some(description) = function.get("description") {
        converted.insert("description".to_string(), description.clone());
    }
    if let Some(parameters) = function.get("parameters") {
        converted.insert("parameters".to_string(), parameters.clone());
    }
    Some(Value::Object(converted))
}

fn text_from_content(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                if let Some(value) = part.as_str() {
                    text.push_str(value);
                } else if let Some(value) = part.get("text").and_then(Value::as_str) {
                    text.push_str(value);
                }
            }
            Some(text)
        }
        _ => None,
    }
}

#[derive(Default)]
struct CollectedTurn {
    text: String,
    tools: Vec<CollectedTool>,
}

struct CollectedTool {
    item_id: String,
    call_id: String,
    name: String,
    arguments: String,
    arguments_done: bool,
}

async fn render_client_response(
    response: super::hyper_client::ProxyResponse,
    model_id: &str,
    client_stream: bool,
) -> Result<Response, WorkBuddySubscriptionErrorCode> {
    if !response.status().is_success() {
        return Err(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    let content_type = response.content_type().unwrap_or("").to_ascii_lowercase();
    let bytes = response
        .bytes_with_limit(MAX_UPSTREAM_BODY)
        .await
        .map_err(|_| WorkBuddySubscriptionErrorCode::UpstreamError)?;
    if bytes.len() > MAX_UPSTREAM_BODY {
        return Err(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    let text =
        std::str::from_utf8(&bytes).map_err(|_| WorkBuddySubscriptionErrorCode::UpstreamError)?;
    let sse = content_type.contains("text/event-stream")
        || text.trim_start().starts_with("data:")
        || text.trim_start().starts_with("event:");
    let turn = if sse {
        collect_sse(text)?
    } else {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| WorkBuddySubscriptionErrorCode::UpstreamError)?;
        if value.get("error").is_some() && value.get("output").is_none() {
            return Err(WorkBuddySubscriptionErrorCode::UpstreamError);
        }
        collect_json(&value)
    };
    if client_stream {
        Ok(sse_response(&render_chat_sse(model_id, &turn)))
    } else {
        Ok(json_response(&render_chat_json(model_id, &turn)))
    }
}

fn collect_json(value: &Value) -> CollectedTurn {
    let mut turn = CollectedTurn::default();
    if value.get("output").is_some() {
        absorb_output(&mut turn, value);
    } else if let Some(response) = value.get("response") {
        absorb_output(&mut turn, response);
    }
    turn
}

fn collect_sse(text: &str) -> Result<CollectedTurn, WorkBuddySubscriptionErrorCode> {
    let mut turn = CollectedTurn::default();
    for block in text.split("\n\n") {
        let mut data = String::new();
        for line in block.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(rest) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(rest.trim_start());
            }
        }
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let event: Value = serde_json::from_str(&data)
            .map_err(|_| WorkBuddySubscriptionErrorCode::UpstreamError)?;
        apply_sse_event(&mut turn, &event);
    }
    Ok(turn)
}

fn apply_sse_event(turn: &mut CollectedTurn, event: &Value) {
    match event.get("type").and_then(Value::as_str).unwrap_or("") {
        "response.output_text.delta" => {
            if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                turn.text.push_str(delta);
            }
        }
        "response.output_item.added" | "response.output_item.done" => {
            if let Some(item) = event.get("item") {
                absorb_item(turn, item, false);
            }
        }
        "response.function_call_arguments.delta" => {
            let item_id = event.get("item_id").and_then(Value::as_str).unwrap_or("");
            let delta = event.get("delta").and_then(Value::as_str).unwrap_or("");
            if let Some(tool) = turn.tools.iter_mut().find(|tool| tool.item_id == item_id) {
                if !tool.arguments_done {
                    tool.arguments.push_str(delta);
                }
            }
        }
        "response.function_call_arguments.done" => {
            let item_id = event.get("item_id").and_then(Value::as_str).unwrap_or("");
            if let Some(tool) = turn.tools.iter_mut().find(|tool| tool.item_id == item_id) {
                if let Some(arguments) = event.get("arguments").and_then(Value::as_str) {
                    tool.arguments = arguments.to_string();
                }
                tool.arguments_done = true;
            }
        }
        "response.completed" => {
            if let Some(response) = event.get("response") {
                if response
                    .get("output")
                    .and_then(Value::as_array)
                    .is_some_and(|output| !output.is_empty())
                {
                    turn.text.clear();
                    turn.tools.clear();
                    absorb_output(turn, response);
                }
            }
        }
        _ => {}
    }
}

fn absorb_output(turn: &mut CollectedTurn, response: &Value) {
    let Some(output) = response.get("output").and_then(Value::as_array) else {
        return;
    };
    for item in output {
        absorb_item(turn, item, true);
    }
}

fn absorb_item(turn: &mut CollectedTurn, item: &Value, from_output: bool) {
    match item.get("type").and_then(Value::as_str).unwrap_or("") {
        "message" => {
            if !from_output && !turn.text.is_empty() {
                return;
            }
            if let Some(parts) = item.get("content").and_then(Value::as_array) {
                let mut text = String::new();
                for part in parts {
                    if let Some(value) = part.get("text").and_then(Value::as_str) {
                        text.push_str(value);
                    }
                }
                if from_output {
                    turn.text.push_str(&text);
                } else if turn.text.is_empty() {
                    turn.text = text;
                }
            }
        }
        "function_call" => {
            let call_id = item
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let item_id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(call_id.as_str())
                .to_string();
            if turn.tools.iter().any(|tool| tool.item_id == item_id) {
                if from_output {
                    if let Some(tool) = turn.tools.iter_mut().find(|tool| tool.item_id == item_id) {
                        if let Some(arguments) = item.get("arguments").and_then(Value::as_str) {
                            tool.arguments = arguments.to_string();
                            tool.arguments_done = true;
                        }
                    }
                }
                return;
            }
            turn.tools.push(CollectedTool {
                item_id,
                call_id,
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                arguments: item
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                arguments_done: from_output,
            });
        }
        _ => {}
    }
}

fn render_chat_sse(model_id: &str, turn: &CollectedTurn) -> String {
    let mut body = String::new();
    body.push_str(&chunk(model_id, json!({"role": "assistant"}), None));
    if !turn.text.is_empty() {
        body.push_str(&chunk(model_id, json!({"content": turn.text}), None));
    }
    for (index, tool) in turn.tools.iter().enumerate() {
        body.push_str(&chunk(
            model_id,
            json!({
                "tool_calls": [{
                    "index": index,
                    "id": tool.call_id,
                    "type": "function",
                    "function": { "name": tool.name, "arguments": "" }
                }]
            }),
            None,
        ));
        if !tool.arguments.is_empty() {
            body.push_str(&chunk(
                model_id,
                json!({
                    "tool_calls": [{
                        "index": index,
                        "function": { "arguments": tool.arguments }
                    }]
                }),
                None,
            ));
        }
    }
    let finish = if turn.tools.is_empty() {
        "stop"
    } else {
        "tool_calls"
    };
    body.push_str(&chunk(model_id, json!({}), Some(finish)));
    body.push_str("data: [DONE]\n\n");
    body
}

fn render_chat_json(model_id: &str, turn: &CollectedTurn) -> Value {
    let mut message = Map::new();
    message.insert("role".to_string(), json!("assistant"));
    if turn.text.is_empty() {
        message.insert("content".to_string(), Value::Null);
    } else {
        message.insert("content".to_string(), json!(turn.text));
    }
    if !turn.tools.is_empty() {
        let calls = turn
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "id": tool.call_id,
                    "type": "function",
                    "function": { "name": tool.name, "arguments": tool.arguments }
                })
            })
            .collect::<Vec<_>>();
        message.insert("tool_calls".to_string(), Value::Array(calls));
    }
    let finish = if turn.tools.is_empty() {
        "stop"
    } else {
        "tool_calls"
    };
    json!({
        "id": "chatcmpl-workbuddy",
        "object": "chat.completion",
        "created": 0,
        "model": model_id,
        "choices": [{
            "index": 0,
            "message": Value::Object(message),
            "finish_reason": finish
        }]
    })
}

fn chunk(model_id: &str, delta: Value, finish: Option<&str>) -> String {
    let payload = json!({
        "id": "chatcmpl-workbuddy",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": model_id,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": finish
        }]
    });
    format!(
        "data: {}\n\n",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
    )
}

fn sse_response(body: &str) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/event-stream")],
        body.to_string(),
    )
        .into_response()
}

fn json_response(body: &Value) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn map_forward_error(error: ProxyError) -> Response {
    let code = match error {
        ProxyError::Timeout(_) | ProxyError::StreamIdleTimeout(_) => {
            WorkBuddySubscriptionErrorCode::Timeout
        }
        ProxyError::AuthError(_) => WorkBuddySubscriptionErrorCode::AccountUnavailable,
        ProxyError::InvalidRequest(_) | ProxyError::TransformError(_) => {
            WorkBuddySubscriptionErrorCode::InvalidRequest
        }
        _ => WorkBuddySubscriptionErrorCode::UpstreamError,
    };
    closed(
        StatusCode::from_u16(code.http_status()).unwrap_or(StatusCode::BAD_GATEWAY),
        code,
    )
}

fn closed(status: StatusCode, code: WorkBuddySubscriptionErrorCode) -> Response {
    let token = code.as_str();
    let body = json!({
        "error": {
            "message": token,
            "type": "invalid_request_error",
            "code": token
        }
    });
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
