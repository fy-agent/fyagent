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

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use bytes::Bytes;
use futures::StreamExt;
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
    let idle = Duration::from_secs(upstream_timeout_secs());
    if client_stream {
        return Ok(stream_translated_response(
            response,
            model_id.to_string(),
            idle,
        ));
    }
    let content_type = response.content_type().unwrap_or("").to_ascii_lowercase();
    let bytes = response
        .bytes_with_limit_idle(MAX_UPSTREAM_BODY, Some(idle))
        .await
        .map_err(body_read_code)?;
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
    Ok(json_response(&render_chat_json(model_id, &turn)))
}

fn body_read_code(error: ProxyError) -> WorkBuddySubscriptionErrorCode {
    match error {
        ProxyError::Timeout(_) | ProxyError::StreamIdleTimeout(_) => {
            WorkBuddySubscriptionErrorCode::Timeout
        }
        _ => WorkBuddySubscriptionErrorCode::UpstreamError,
    }
}

fn stream_translated_response(
    response: super::hyper_client::ProxyResponse,
    model_id: String,
    idle: Duration,
) -> Response {
    let stream = translate_upstream_stream(response, model_id, idle);
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(axum::body::Body::from_stream(stream))
        .unwrap_or_else(|_| {
            closed(
                StatusCode::BAD_GATEWAY,
                WorkBuddySubscriptionErrorCode::UpstreamError,
            )
        })
}

fn translate_upstream_stream(
    response: super::hyper_client::ProxyResponse,
    model_id: String,
    idle: Duration,
) -> impl futures::Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        let mut incoming = std::pin::pin!(response.limited_chunk_stream(MAX_UPSTREAM_BODY, Some(idle)));
        let mut buffer = String::new();
        let mut remainder = Vec::new();
        let mut turn = CollectedTurn::default();
        let mut cursor = EmitCursor::default();
        let mut terminal = false;
        while let Some(item) = incoming.next().await {
            match item {
                Err(error) => {
                    yield Ok(sse_error_event(body_read_code(error)));
                    terminal = true;
                    break;
                }
                Ok(chunk) => {
                    crate::proxy::sse::append_utf8_safe(&mut buffer, &mut remainder, &chunk);
                    while let Some(block) = crate::proxy::sse::take_sse_block(&mut buffer) {
                        match on_sse_block(&block, &mut turn, &mut cursor, &model_id) {
                            BlockEffect::Open(bytes) => {
                                if !bytes.is_empty() {
                                    yield Ok(Bytes::from(bytes));
                                }
                            }
                            BlockEffect::Done(bytes) => {
                                if !bytes.is_empty() {
                                    yield Ok(Bytes::from(bytes));
                                }
                                terminal = true;
                                break;
                            }
                            BlockEffect::Failed(code) => {
                                yield Ok(sse_error_event(code));
                                terminal = true;
                                break;
                            }
                        }
                    }
                    if terminal {
                        break;
                    }
                }
            }
        }
        if !terminal {
            if cursor.saw_event {
                yield Ok(sse_error_event(WorkBuddySubscriptionErrorCode::UpstreamError));
            } else {
                match json_stream_tail(&buffer, &model_id) {
                    BlockEffect::Done(bytes) => {
                        if !bytes.is_empty() {
                            yield Ok(Bytes::from(bytes));
                        }
                    }
                    BlockEffect::Failed(code) => yield Ok(sse_error_event(code)),
                    BlockEffect::Open(_) => {
                        yield Ok(sse_error_event(WorkBuddySubscriptionErrorCode::UpstreamError));
                    }
                }
            }
        }
    }
}

enum BlockEffect {
    Open(String),
    Done(String),
    Failed(WorkBuddySubscriptionErrorCode),
}

#[derive(Default)]
struct EmitCursor {
    role_sent: bool,
    text_sent: String,
    args_sent: HashMap<String, String>,
    tool_header_sent: HashSet<String>,
    saw_event: bool,
}

fn on_sse_block(
    block: &str,
    turn: &mut CollectedTurn,
    cursor: &mut EmitCursor,
    model_id: &str,
) -> BlockEffect {
    let Some((event_name, data)) = block_event_and_data(block) else {
        return BlockEffect::Open(String::new());
    };
    if data.trim() == "[DONE]" {
        return BlockEffect::Open(String::new());
    }
    cursor.saw_event = true;
    let event: Value = match serde_json::from_str(&data) {
        Ok(event) => event,
        Err(_) => return BlockEffect::Failed(WorkBuddySubscriptionErrorCode::UpstreamError),
    };
    if event_is_upstream_failure(&event_name, &event) {
        return BlockEffect::Failed(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    apply_sse_event(turn, &event);
    let mut out = emit_new_output(model_id, turn, cursor);
    if event_is_completed(&event) {
        if !cursor.role_sent {
            out = format!(
                "{}{out}",
                chunk(model_id, json!({"role": "assistant"}), None)
            );
            cursor.role_sent = true;
        }
        out.push_str(&finish_sse(model_id, turn));
        return BlockEffect::Done(out);
    }
    BlockEffect::Open(out)
}

fn json_stream_tail(buffer: &str, model_id: &str) -> BlockEffect {
    let trimmed = buffer.trim();
    if trimmed.is_empty() || trimmed.starts_with("data:") || trimmed.starts_with("event:") {
        return BlockEffect::Failed(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    let value: Value = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(_) => return BlockEffect::Failed(WorkBuddySubscriptionErrorCode::UpstreamError),
    };
    if value.get("error").is_some() && value.get("output").is_none() {
        return BlockEffect::Failed(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    BlockEffect::Done(render_chat_sse(model_id, &collect_json(&value)))
}

fn sse_error_event(code: WorkBuddySubscriptionErrorCode) -> Bytes {
    let token = code.as_str();
    let payload = json!({
        "error": {
            "message": token,
            "type": "server_error",
            "code": token
        }
    });
    Bytes::from(format!(
        "data: {}\n\n",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
    ))
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
    let mut completed = false;
    for block in text.split("\n\n") {
        let Some((event_name, data)) = block_event_and_data(block) else {
            continue;
        };
        if data.trim() == "[DONE]" {
            continue;
        }
        let event: Value = serde_json::from_str(&data)
            .map_err(|_| WorkBuddySubscriptionErrorCode::UpstreamError)?;
        if event_is_upstream_failure(&event_name, &event) {
            return Err(WorkBuddySubscriptionErrorCode::UpstreamError);
        }
        if event_is_completed(&event) {
            completed = true;
        }
        apply_sse_event(&mut turn, &event);
    }
    if !completed {
        return Err(WorkBuddySubscriptionErrorCode::UpstreamError);
    }
    Ok(turn)
}

fn block_event_and_data(block: &str) -> Option<(String, String)> {
    let mut event_name = String::new();
    let mut data = String::new();
    for line in block.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = crate::proxy::sse::strip_sse_field(line, "event") {
            event_name = rest.trim().to_string();
        }
        if let Some(rest) = crate::proxy::sse::strip_sse_field(line, "data") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
    }
    if data.is_empty() {
        None
    } else {
        Some((event_name, data))
    }
}

fn event_is_upstream_failure(event_name: &str, event: &Value) -> bool {
    if event_name.eq_ignore_ascii_case("error")
        || event_name.eq_ignore_ascii_case("response.failed")
        || event_name.eq_ignore_ascii_case("response.incomplete")
    {
        return true;
    }
    let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
    if matches!(kind, "error" | "response.failed" | "response.incomplete") {
        return true;
    }
    if let Some(response) = event.get("response") {
        let status = response.get("status").and_then(Value::as_str).unwrap_or("");
        if matches!(status, "failed" | "incomplete") {
            return true;
        }
        if meaningful_error(response.get("error")) && status != "completed" {
            return true;
        }
    }
    kind != "response.completed" && meaningful_error(event.get("error"))
}

fn event_is_completed(event: &Value) -> bool {
    if event.get("type").and_then(Value::as_str) != Some("response.completed") {
        return false;
    }
    let status = event
        .get("response")
        .and_then(|response| response.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("completed");
    status == "completed"
}

fn meaningful_error(value: Option<&Value>) -> bool {
    match value {
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Object(map)) => map
            .keys()
            .any(|key| matches!(key.as_str(), "message" | "code" | "type")),
        _ => false,
    }
}

fn emit_new_output(model_id: &str, turn: &CollectedTurn, cursor: &mut EmitCursor) -> String {
    let pending_text = text_suffix(&cursor.text_sent, &turn.text);
    let mut tool_emits = Vec::new();
    for (index, tool) in turn.tools.iter().enumerate() {
        let sent = cursor
            .args_sent
            .get(&tool.item_id)
            .map(String::as_str)
            .unwrap_or("");
        let header = !cursor.tool_header_sent.contains(&tool.item_id);
        let arg_suffix = if tool.arguments.starts_with(sent) {
            tool.arguments[sent.len()..].to_string()
        } else if sent.is_empty() {
            tool.arguments.clone()
        } else {
            String::new()
        };
        if header || !arg_suffix.is_empty() {
            tool_emits.push(PendingTool {
                index,
                item_id: tool.item_id.clone(),
                call_id: tool.call_id.clone(),
                name: tool.name.clone(),
                header,
                arg_suffix,
                arguments: tool.arguments.clone(),
            });
        }
    }
    if pending_text.is_none() && tool_emits.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    if !cursor.role_sent {
        out.push_str(&chunk(model_id, json!({"role": "assistant"}), None));
        cursor.role_sent = true;
    }
    if let Some(suffix) = pending_text {
        out.push_str(&chunk(model_id, json!({"content": suffix}), None));
        cursor.text_sent = turn.text.clone();
    }
    for tool in tool_emits {
        if tool.header {
            out.push_str(&chunk(
                model_id,
                json!({
                    "tool_calls": [{
                        "index": tool.index,
                        "id": tool.call_id,
                        "type": "function",
                        "function": { "name": tool.name, "arguments": "" }
                    }]
                }),
                None,
            ));
            cursor.tool_header_sent.insert(tool.item_id.clone());
        }
        if !tool.arg_suffix.is_empty() {
            out.push_str(&chunk(
                model_id,
                json!({
                    "tool_calls": [{
                        "index": tool.index,
                        "function": { "arguments": tool.arg_suffix }
                    }]
                }),
                None,
            ));
        }
        cursor.args_sent.insert(tool.item_id, tool.arguments);
    }
    out
}

struct PendingTool {
    index: usize,
    item_id: String,
    call_id: String,
    name: String,
    header: bool,
    arg_suffix: String,
    arguments: String,
}

fn text_suffix(sent: &str, next: &str) -> Option<String> {
    if next.starts_with(sent) {
        let rest = &next[sent.len()..];
        if rest.is_empty() {
            None
        } else {
            Some(rest.to_string())
        }
    } else if sent.is_empty() && !next.is_empty() {
        Some(next.to_string())
    } else {
        None
    }
}

fn finish_sse(model_id: &str, turn: &CollectedTurn) -> String {
    let finish = if turn.tools.is_empty() {
        "stop"
    } else {
        "tool_calls"
    };
    let mut body = chunk(model_id, json!({}), Some(finish));
    body.push_str("data: [DONE]\n\n");
    body
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

#[cfg(test)]
mod stream_tests {
    use super::translate_upstream_stream;
    use axum::http::{HeaderMap, StatusCode};
    use bytes::Bytes;
    use futures::StreamExt;
    use std::time::Duration;

    async fn collect_translated(
        chunks: Vec<Result<Bytes, std::io::Error>>,
        idle: Duration,
    ) -> String {
        let stream = futures::stream::iter(chunks);
        let response = super::super::hyper_client::ProxyResponse::streamed(
            StatusCode::OK,
            HeaderMap::new(),
            stream,
        );
        let body = translate_upstream_stream(response, "probe-model".to_string(), idle);
        let mut out = Vec::new();
        tokio::pin!(body);
        while let Some(item) = body.next().await {
            out.extend_from_slice(&item.expect("stream item"));
        }
        String::from_utf8(out).expect("utf-8")
    }

    fn delta(text: &str) -> String {
        format!(
            "event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"delta\":\"{text}\"}}\n\n"
        )
    }

    fn completed(text: &str) -> String {
        format!(
            "event: response.completed\ndata: {{\"type\":\"response.completed\",\"response\":{{\"status\":\"completed\",\"output\":[{{\"type\":\"message\",\"id\":\"msg_fixture\",\"content\":[{{\"type\":\"output_text\",\"text\":\"{text}\"}}]}}]}}}}\n\n"
        )
    }

    #[tokio::test]
    async fn stream_midway_error_reaches_client_without_done() {
        let failed = "event: response.failed\ndata: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\",\"error\":{\"message\":\"SENTINEL_UPSTREAM_SECRET\",\"type\":\"server_error\"}}}\n\n";
        let body = collect_translated(
            vec![
                Ok(Bytes::from(delta("partial-visible"))),
                Ok(Bytes::from(failed)),
            ],
            Duration::from_secs(2),
        )
        .await;
        assert!(body.contains("partial-visible"), "{body}");
        assert!(body.contains("\"code\":\"upstream_error\""), "{body}");
        assert!(!body.contains("data: [DONE]"), "{body}");
        assert!(!body.contains("\"finish_reason\":\"stop\""), "{body}");
        assert!(!body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
        assert!(!body.contains("SENTINEL_UPSTREAM_SECRET"), "{body}");
    }

    #[tokio::test]
    async fn stream_early_disconnect_reaches_client_without_done() {
        let body = collect_translated(
            vec![Ok(Bytes::from(delta("partial-visible")))],
            Duration::from_secs(2),
        )
        .await;
        assert!(body.contains("partial-visible"), "{body}");
        assert!(body.contains("\"code\":\"upstream_error\""), "{body}");
        assert!(!body.contains("data: [DONE]"), "{body}");
        assert!(!body.contains("\"finish_reason\":\"stop\""), "{body}");
    }

    #[tokio::test]
    async fn stream_read_error_reaches_client_without_done() {
        let body = collect_translated(
            vec![
                Ok(Bytes::from(delta("partial-visible"))),
                Err(std::io::Error::other("connection reset")),
            ],
            Duration::from_secs(2),
        )
        .await;
        assert!(body.contains("partial-visible"), "{body}");
        assert!(body.contains("\"code\":\"upstream_error\""), "{body}");
        assert!(!body.contains("data: [DONE]"), "{body}");
        assert!(!body.contains("\"finish_reason\":\"stop\""), "{body}");
    }

    #[tokio::test]
    async fn stream_idle_timeout_ends_without_done() {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        let response = super::super::hyper_client::ProxyResponse::streamed(
            StatusCode::OK,
            HeaderMap::new(),
            stream,
        );
        let body = translate_upstream_stream(
            response,
            "probe-model".to_string(),
            Duration::from_millis(200),
        );
        tokio::pin!(body);
        tx.send(Ok(Bytes::from(delta("partial-visible"))))
            .await
            .unwrap();
        let first = tokio::time::timeout(Duration::from_secs(1), body.next())
            .await
            .expect("delta is forwarded before the upstream continues")
            .expect("chunk")
            .expect("ok");
        let first = String::from_utf8(first.to_vec()).unwrap();
        assert!(first.contains("partial-visible"), "{first}");
        assert!(!first.contains("data: [DONE]"), "{first}");
        let started = std::time::Instant::now();
        let next = tokio::time::timeout(Duration::from_secs(2), body.next())
            .await
            .expect("idle timeout should end the stream")
            .expect("timeout event")
            .expect("ok");
        assert!(started.elapsed() < Duration::from_secs(2));
        let next = String::from_utf8(next.to_vec()).unwrap();
        assert!(next.contains("\"code\":\"timeout\""), "{next}");
        assert!(!next.contains("data: [DONE]"), "{next}");
        assert!(body.next().await.is_none());
        drop(tx);
    }

    #[tokio::test]
    async fn stream_completed_turn_still_ends_with_done() {
        let body = collect_translated(
            vec![
                Ok(Bytes::from(delta("fixture-ok"))),
                Ok(Bytes::from(completed("fixture-ok"))),
            ],
            Duration::from_secs(2),
        )
        .await;
        assert!(body.contains("fixture-ok"), "{body}");
        assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
        assert!(body.contains("data: [DONE]"), "{body}");
        assert!(!body.contains("\"code\":\"upstream_error\""), "{body}");
    }
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
