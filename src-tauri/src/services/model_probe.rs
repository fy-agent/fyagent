//! Draft-model connectivity probe.
//!
//! Recovers the real streaming request from the pre-`a5903d86` Stream Check
//! path, but only for V2 Models draft URLs. This is not URL reachability:
//! it sends an authenticated one-token stream and treats the first SSE chunk
//! as success. It never looks up a saved Provider and never touches the
//! failover circuit breaker.

use std::time::{Duration, Instant};

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::services::stream_check::{HealthStatus, StreamCheckService};

const TEST_PROMPT: &str = "ping";
const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: u32 = 1;
const DEGRADED_THRESHOLD_MS: u64 = 6000;
const ERROR_BODY_MAX_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelProbeApp {
    Claude,
    Codex,
    GrokBuild,
    WorkBuddy,
    OpenCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeProtocol {
    AnthropicMessages,
    OpenAiChat,
    OpenAiResponses,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelProbeResult {
    pub status: HealthStatus,
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<u64>,
    pub http_status: Option<u16>,
    pub model_used: String,
    pub tested_at: i64,
    pub retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_category: Option<String>,
}

pub async fn probe(
    app: ModelProbeApp,
    base_url: &str,
    api_key: &str,
    model_id: &str,
) -> Result<ModelProbeResult, AppError> {
    probe_with_client(
        &crate::proxy::http_client::get(),
        app,
        base_url,
        api_key,
        model_id,
    )
    .await
}

pub async fn probe_with_client(
    client: &Client,
    app: ModelProbeApp,
    base_url: &str,
    api_key: &str,
    model_id: &str,
) -> Result<ModelProbeResult, AppError> {
    StreamCheckService::validate_probe_url(base_url)?;
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err(AppError::Message("模型 ID 为空".to_string()));
    }
    if !api_key.is_empty() && base_url.contains(api_key) {
        return Err(AppError::Message("服务地址无效".to_string()));
    }

    let mut last_result: Option<ModelProbeResult> = None;
    for attempt in 0..=MAX_RETRIES {
        let start = Instant::now();
        let result = probe_once(client, app, base_url.trim(), api_key, model_id).await;
        let wrapped = build_result(
            result,
            start.elapsed().as_millis() as u64,
            model_id,
            api_key,
        );
        let wrapped = ModelProbeResult {
            retry_count: attempt,
            ..wrapped
        };
        if wrapped.success {
            return Ok(wrapped);
        }
        if should_retry(&wrapped.message) && attempt < MAX_RETRIES {
            last_result = Some(wrapped);
            continue;
        }
        return Ok(wrapped);
    }
    Ok(last_result.unwrap_or_else(|| failed_result(model_id, 0, "Check failed")))
}

async fn probe_once(
    client: &Client,
    app: ModelProbeApp,
    base_url: &str,
    api_key: &str,
    model_id: &str,
) -> Result<(u16, String), AppError> {
    let protocol = protocol_for_app(app);
    let (actual_model, reasoning_effort) = parse_model_with_effort(model_id);
    let urls = resolve_probe_urls(base_url, endpoint_for(protocol));
    let body = request_body(protocol, &actual_model, reasoning_effort.as_deref());

    for (index, url) in urls.iter().enumerate() {
        match send_stream_request(client, protocol, url, api_key, &body).await {
            Ok(status) => return Ok((status, actual_model)),
            Err(error) => {
                if index == 0
                    && urls.len() > 1
                    && matches!(&error, AppError::HttpStatus { status: 404, .. })
                {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Err(AppError::Message("没有可用的模型探测端点".to_string()))
}

fn protocol_for_app(app: ModelProbeApp) -> ProbeProtocol {
    match app {
        ModelProbeApp::Claude => ProbeProtocol::AnthropicMessages,
        ModelProbeApp::Codex => ProbeProtocol::OpenAiResponses,
        ModelProbeApp::GrokBuild | ModelProbeApp::WorkBuddy | ModelProbeApp::OpenCode => {
            ProbeProtocol::OpenAiChat
        }
    }
}

fn endpoint_for(protocol: ProbeProtocol) -> &'static str {
    match protocol {
        ProbeProtocol::AnthropicMessages => "messages",
        ProbeProtocol::OpenAiChat => "chat/completions",
        ProbeProtocol::OpenAiResponses => "responses",
    }
}

fn strip_known_endpoints(base: &str) -> String {
    let mut base = base.trim().trim_end_matches('/').to_string();
    let lower = base.to_ascii_lowercase();
    for suffix in ["/chat/completions", "/messages", "/responses", "/models"] {
        if lower.ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            break;
        }
    }
    base.trim_end_matches('/').to_string()
}

fn is_origin_only_url(value: &str) -> bool {
    let trimmed = value.trim_end_matches('/');
    match trimmed.split_once("://") {
        Some((_scheme, rest)) => !rest.contains('/'),
        None => !trimmed.contains('/'),
    }
}

fn resolve_probe_urls(base_url: &str, endpoint: &str) -> Vec<String> {
    let base = strip_known_endpoints(base_url);
    let endpoint_suffix = format!("/{endpoint}");
    if base.to_ascii_lowercase().ends_with(&endpoint_suffix) {
        return vec![base];
    }
    if base.ends_with("/v1") {
        return vec![format!("{base}{endpoint_suffix}")];
    }
    if is_origin_only_url(&base) {
        vec![
            format!("{base}/v1{endpoint_suffix}"),
            format!("{base}{endpoint_suffix}"),
        ]
    } else {
        vec![
            format!("{base}{endpoint_suffix}"),
            format!("{base}/v1{endpoint_suffix}"),
        ]
    }
}

fn parse_model_with_effort(model: &str) -> (String, Option<String>) {
    if let Some(pos) = model.find('@').or_else(|| model.find('#')) {
        let actual_model = model[..pos].to_string();
        let effort = model[pos + 1..].to_string();
        if !effort.is_empty() {
            return (actual_model, Some(effort));
        }
    }
    (model.to_string(), None)
}

fn request_body(protocol: ProbeProtocol, model: &str, reasoning_effort: Option<&str>) -> Value {
    match protocol {
        ProbeProtocol::AnthropicMessages => json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": TEST_PROMPT }],
            "stream": true
        }),
        ProbeProtocol::OpenAiChat => {
            let mut body = json!({
                "model": model,
                "messages": [{ "role": "user", "content": TEST_PROMPT }],
                "max_tokens": 1,
                "stream": true
            });
            if let Some(effort) = reasoning_effort {
                if crate::proxy::providers::transform::supports_reasoning_effort(model) {
                    body["reasoning_effort"] = json!(effort);
                }
            }
            body
        }
        ProbeProtocol::OpenAiResponses => {
            let mut body = json!({
                "model": model,
                "input": [{ "role": "user", "content": TEST_PROMPT }],
                "max_output_tokens": 16,
                "stream": true
            });
            if let Some(effort) = reasoning_effort {
                body["reasoning"] = json!({ "effort": effort });
            }
            body
        }
    }
}

async fn send_stream_request(
    client: &Client,
    protocol: ProbeProtocol,
    url: &str,
    api_key: &str,
    body: &Value,
) -> Result<u16, AppError> {
    let mut request = client.post(url).timeout(TIMEOUT).json(body);
    request = match protocol {
        ProbeProtocol::AnthropicMessages => {
            let os_name = os_name();
            let arch_name = arch_name();
            let mut builder = request
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .header("accept", "text/event-stream")
                .header("accept-encoding", "identity")
                .header("user-agent", "claude-cli/2.1.2 (external, cli)")
                .header("x-app", "cli")
                .header("x-stainless-lang", "js")
                .header("x-stainless-os", os_name)
                .header("x-stainless-arch", arch_name);
            if !api_key.is_empty() {
                builder = builder.header("x-api-key", api_key);
            }
            builder
        }
        ProbeProtocol::OpenAiChat | ProbeProtocol::OpenAiResponses => {
            let mut builder = request
                .header("content-type", "application/json")
                .header("accept", "text/event-stream")
                .header("accept-encoding", "identity")
                .header(
                    "user-agent",
                    format!("codex_cli_rs/0.80.0 ({}; {})", os_name(), arch_name()),
                )
                .header("originator", "codex_cli_rs");
            if !api_key.is_empty() {
                builder = builder.header("authorization", format!("Bearer {api_key}"));
            }
            builder
        }
    };

    let response = request.send().await.map_err(map_request_error)?;
    let status = response.status().as_u16();
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(http_status_error(status, error_text));
    }

    let mut stream = response.bytes_stream();
    let chunk = stream
        .next()
        .await
        .ok_or_else(|| AppError::Message("没有收到模型响应".to_string()))?
        .map_err(|error| AppError::Message(format!("读取流失败: {error}")))?;
    let text = String::from_utf8_lossy(&chunk);
    if first_chunk_is_error(&text) {
        return Err(http_status_error(status, text.into_owned()));
    }
    Ok(status)
}

fn first_chunk_is_error(chunk: &str) -> bool {
    let lower = chunk.to_ascii_lowercase();
    lower.contains("event: error") || lower.contains("\"type\":\"error\"")
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "MacOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
}

fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        "x86" => "x86",
        other => other,
    }
}

fn map_request_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::Message("请求超时".to_string())
    } else if error.is_connect() {
        AppError::Message(format!("连接失败: {error}"))
    } else {
        AppError::Message(error.to_string())
    }
}

fn http_status_error(status: u16, body: String) -> AppError {
    AppError::HttpStatus {
        status,
        body: truncate_body(&body),
    }
}

fn truncate_body(body: &str) -> String {
    if body.chars().count() <= ERROR_BODY_MAX_CHARS {
        return body.to_string();
    }
    let truncated: String = body.chars().take(ERROR_BODY_MAX_CHARS).collect();
    format!("{truncated}…")
}

fn redact(text: &str, secret: &str) -> String {
    let secret = secret.trim();
    if secret.is_empty() {
        text.to_string()
    } else {
        text.replace(secret, "***")
    }
}

fn detect_error_category(status: u16, body: &str) -> Option<&'static str> {
    if !(400..500).contains(&status) {
        return None;
    }
    let lower = body.to_lowercase();
    let quota_indicators = [
        "coding_plan_hour_quota_exceeded",
        "coding_plan_week_quota_exceeded",
        "coding_plan_month_quota_exceeded",
    ];
    if quota_indicators.iter().any(|marker| lower.contains(marker)) {
        return Some("quotaExceeded");
    }
    if !lower.contains("model") {
        return None;
    }
    let indicators = [
        "model_not_found",
        "model not found",
        "does not exist",
        "invalid_model",
        "invalid model",
        "unknown_model",
        "unknown model",
        "is not a valid model",
        "not_found_error",
    ];
    indicators
        .iter()
        .any(|marker| lower.contains(marker))
        .then_some("modelNotFound")
}

fn should_retry(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("timeout")
        || lower.contains("超时")
        || lower.contains("abort")
        || lower.contains("timed out")
}

fn build_result(
    result: Result<(u16, String), AppError>,
    response_time_ms: u64,
    model_tested: &str,
    api_key: &str,
) -> ModelProbeResult {
    let tested_at = chrono::Utc::now().timestamp();
    match result {
        Ok((status, model)) => ModelProbeResult {
            status: if response_time_ms >= DEGRADED_THRESHOLD_MS {
                HealthStatus::Degraded
            } else {
                HealthStatus::Operational
            },
            success: true,
            message: if response_time_ms >= DEGRADED_THRESHOLD_MS {
                format!("模型 {model} 已响应，但首包较慢（{response_time_ms} ms）")
            } else {
                format!("模型 {model} 已响应（{response_time_ms} ms）")
            },
            response_time_ms: Some(response_time_ms),
            http_status: Some(status),
            model_used: model,
            tested_at,
            retry_count: 0,
            error_category: None,
        },
        Err(error) => {
            let (http_status, raw_message, error_category) = match &error {
                AppError::HttpStatus { status, body } => (
                    Some(*status),
                    format!("HTTP {status}: {body}"),
                    detect_error_category(*status, body).map(str::to_string),
                ),
                _ => (None, error.to_string(), None),
            };
            ModelProbeResult {
                status: HealthStatus::Failed,
                success: false,
                message: redact(&raw_message, api_key),
                response_time_ms: Some(response_time_ms),
                http_status,
                model_used: model_tested.to_string(),
                tested_at,
                retry_count: 0,
                error_category,
            }
        }
    }
}

fn failed_result(model: &str, retry_count: u32, message: &str) -> ModelProbeResult {
    ModelProbeResult {
        status: HealthStatus::Failed,
        success: false,
        message: message.to_string(),
        response_time_ms: None,
        http_status: None,
        model_used: model.to_string(),
        tested_at: chrono::Utc::now().timestamp(),
        retry_count,
        error_category: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
        time::Duration as StdDuration,
    };

    use reqwest::{redirect::Policy, Client};

    fn loopback_client() -> Client {
        Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .expect("loopback client")
    }

    fn http_response(status: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn spawn_server(responses: Vec<Vec<u8>>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        thread::spawn(move || {
            listener.set_nonblocking(false).expect("blocking listener");
            for response in responses {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = stream.set_read_timeout(Some(StdDuration::from_secs(3)));
                    let mut buf = [0_u8; 4096];
                    if let Ok(read) = stream.read(&mut buf) {
                        captured
                            .lock()
                            .expect("requests")
                            .push(String::from_utf8_lossy(&buf[..read]).into_owned());
                    }
                    let _ = stream.write_all(&response);
                    let _ = stream.flush();
                }
            }
        });
        (format!("http://127.0.0.1:{port}"), requests)
    }

    #[test]
    fn origin_only_urls_prefer_v1_then_bare_path() {
        assert_eq!(
            resolve_probe_urls("https://gateway.example", "chat/completions"),
            vec![
                "https://gateway.example/v1/chat/completions".to_string(),
                "https://gateway.example/chat/completions".to_string(),
            ]
        );
        assert_eq!(
            resolve_probe_urls("https://gateway.example/v1", "messages"),
            vec!["https://gateway.example/v1/messages".to_string()]
        );
        assert_eq!(
            resolve_probe_urls(
                "https://gateway.example/v1/chat/completions",
                "chat/completions"
            ),
            vec!["https://gateway.example/v1/chat/completions".to_string()]
        );
    }

    #[test]
    fn protocol_matches_product() {
        assert_eq!(
            protocol_for_app(ModelProbeApp::Claude),
            ProbeProtocol::AnthropicMessages
        );
        assert_eq!(
            protocol_for_app(ModelProbeApp::Codex),
            ProbeProtocol::OpenAiResponses
        );
        assert_eq!(
            protocol_for_app(ModelProbeApp::WorkBuddy),
            ProbeProtocol::OpenAiChat
        );
        assert_eq!(
            protocol_for_app(ModelProbeApp::GrokBuild),
            ProbeProtocol::OpenAiChat
        );
        assert_eq!(
            protocol_for_app(ModelProbeApp::OpenCode),
            ProbeProtocol::OpenAiChat
        );
    }

    #[test]
    fn classifies_model_not_found_from_body() {
        assert_eq!(
            detect_error_category(404, r#"{"error":{"code":"model_not_found"}}"#),
            Some("modelNotFound")
        );
        assert_eq!(detect_error_category(500, "model_not_found"), None);
    }

    #[test]
    fn redacts_api_key_from_error_message() {
        let result = build_result(
            Err(AppError::HttpStatus {
                status: 401,
                body: "invalid key sk-secret-value".to_string(),
            }),
            12,
            "gpt-test",
            "sk-secret-value",
        );
        assert!(!result.success);
        assert!(!result.message.contains("sk-secret-value"));
        assert!(result.message.contains("HTTP 401"));
        assert!(result.message.contains("***"));
    }

    #[tokio::test]
    async fn openai_chat_probe_succeeds_on_first_chunk() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        let (base, requests) = spawn_server(vec![http_response("200 OK", body)]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::WorkBuddy,
            &format!("{base}/v1"),
            "sk-test",
            "gpt-test",
        )
        .await
        .expect("probe");
        assert!(result.success);
        assert_eq!(result.model_used, "gpt-test");
        assert_eq!(result.http_status, Some(200));
        let captured = requests.lock().expect("requests").join("\n");
        assert!(captured.contains("POST /v1/chat/completions"));
        assert!(captured.contains("Bearer sk-test"));
    }

    #[tokio::test]
    async fn failed_probe_returns_upstream_error_body() {
        let (base, _) = spawn_server(vec![http_response(
            "401 Unauthorized",
            r#"{"error":{"message":"invalid api key"}}"#,
        )]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::Claude,
            &format!("{base}/v1"),
            "sk-test",
            "claude-test",
        )
        .await
        .expect("probe result");
        assert!(!result.success);
        assert_eq!(result.http_status, Some(401));
        assert!(result.message.contains("invalid api key"));
        assert!(result.message.contains("HTTP 401"));
    }

    #[tokio::test]
    async fn rejects_empty_model_before_network() {
        let error = probe_with_client(
            &loopback_client(),
            ModelProbeApp::Codex,
            "https://gateway.example/v1",
            "sk-test",
            "  ",
        )
        .await
        .expect_err("empty model");
        assert!(error.to_string().contains("模型 ID 为空"));
    }
}
