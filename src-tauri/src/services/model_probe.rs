//! Draft-model connectivity probe.
//!
//! Projects one request per app onto the same wire contract the target client
//! will use after Quick Setup (`ProbeRequestSpec`), then sends that single
//! streaming request. This is not URL reachability: the first SSE chunk is
//! success. It never looks up a saved Provider, never guesses a second URL,
//! and never touches the failover circuit breaker.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::services::stream_check::{HealthStatus, StreamCheckService};

const MIN_PROBE_INPUT_TOKENS: usize = 1024;
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

/// App-specific request projection. Transport only sends this object.
#[derive(Debug, Clone, PartialEq)]
struct ProbeRequestSpec {
    url: String,
    headers: Vec<(String, String)>,
    body: Value,
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
    codex_image_extension: bool,
) -> Result<ModelProbeResult, AppError> {
    probe_with_client(
        &crate::proxy::http_client::get(),
        app,
        base_url,
        api_key,
        model_id,
        codex_image_extension,
    )
    .await
}

pub async fn probe_with_client(
    client: &Client,
    app: ModelProbeApp,
    base_url: &str,
    api_key: &str,
    model_id: &str,
    codex_image_extension: bool,
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
        let result = probe_once(
            client,
            app,
            base_url.trim(),
            api_key,
            model_id,
            codex_image_extension,
        )
        .await;
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
    codex_image_extension: bool,
) -> Result<(u16, String), AppError> {
    let (spec, actual_model) =
        build_probe_spec(app, base_url, api_key, model_id, codex_image_extension)?;
    let status = send_stream_request(client, &spec).await?;
    Ok((status, actual_model))
}

fn protocol_for_app(app: ModelProbeApp) -> ProbeProtocol {
    match app {
        ModelProbeApp::Claude => ProbeProtocol::AnthropicMessages,
        ModelProbeApp::Codex | ModelProbeApp::GrokBuild => ProbeProtocol::OpenAiResponses,
        ModelProbeApp::WorkBuddy | ModelProbeApp::OpenCode => ProbeProtocol::OpenAiChat,
    }
}

fn build_probe_spec(
    app: ModelProbeApp,
    base_url: &str,
    api_key: &str,
    model_id: &str,
    codex_image_extension: bool,
) -> Result<(ProbeRequestSpec, String), AppError> {
    let protocol = protocol_for_app(app);
    let (actual_model, reasoning_effort) = parse_model_with_effort(model_id);
    let url = probe_url(app, base_url)?;
    let body = request_body(protocol, &actual_model, reasoning_effort.as_deref());
    let headers = probe_headers(app, api_key, codex_image_extension);
    Ok((ProbeRequestSpec { url, headers, body }, actual_model))
}

fn probe_url(app: ModelProbeApp, base_url: &str) -> Result<String, AppError> {
    match app {
        ModelProbeApp::Claude => Ok(claude_code_messages_url(base_url)),
        ModelProbeApp::Codex | ModelProbeApp::GrokBuild => {
            Ok(openai_compatible_url(base_url, "responses"))
        }
        ModelProbeApp::OpenCode => Ok(openai_compatible_url(base_url, "chat/completions")),
        ModelProbeApp::WorkBuddy => workbuddy_chat_completions_url(base_url),
    }
}

/// Claude Code / Anthropic SDK join: `{ANTHROPIC_BASE_URL}/v1/messages`.
/// Do not collapse `/v1/v1` and do not retry an alternate path.
fn claude_code_messages_url(base_url: &str) -> String {
    format!("{}/v1/messages", base_url.trim().trim_end_matches('/'))
}

/// Codex CLI / OpenCode openai-compatible join: `{base}/{endpoint}`.
/// `base` is used as-is (typically already includes `/v1`). Terminal
/// `/messages` `/responses` `/chat/completions` `/models` suffixes are
/// stripped so a pasted full endpoint still maps to one determined URL.
fn openai_compatible_url(base_url: &str, endpoint: &str) -> String {
    format!(
        "{}/{}",
        strip_known_endpoints(base_url),
        endpoint.trim_start_matches('/')
    )
}

fn workbuddy_chat_completions_url(base_url: &str) -> Result<String, AppError> {
    let normalized = crate::services::workbuddy::url::normalize_workbuddy_base_url(base_url)
        .map_err(|_| AppError::Message("服务地址无效".to_string()))?;
    let mut url = normalized.base_url;
    let path = match url.path().trim_end_matches('/') {
        "" | "/" => "/chat/completions".to_string(),
        path => format!("{path}/chat/completions"),
    };
    url.set_path(&path);
    Ok(url.to_string())
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

fn probe_headers(
    app: ModelProbeApp,
    api_key: &str,
    codex_image_extension: bool,
) -> Vec<(String, String)> {
    let mut headers = match app {
        ModelProbeApp::Claude => claude_headers(api_key),
        ModelProbeApp::Codex | ModelProbeApp::GrokBuild => openai_headers(api_key, true),
        ModelProbeApp::WorkBuddy | ModelProbeApp::OpenCode => openai_headers(api_key, false),
    };
    if app == ModelProbeApp::Codex && codex_image_extension {
        headers.push((
            crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER.to_string(),
            crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE.to_string(),
        ));
    }
    headers
}

fn claude_headers(api_key: &str) -> Vec<(String, String)> {
    let mut headers = vec![
        header("anthropic-version", "2023-06-01"),
        header("content-type", "application/json"),
        header("accept", "text/event-stream"),
        header("accept-encoding", "identity"),
        header("user-agent", "claude-cli/2.1.2 (external, cli)"),
        header("x-app", "cli"),
        header("x-stainless-lang", "js"),
        header("x-stainless-os", os_name()),
        header("x-stainless-arch", arch_name()),
    ];
    // V2 Quick Setup writes ANTHROPIC_AUTH_TOKEN. Claude Code and ClaudeAdapter
    // send that as Authorization: Bearer. Do not send x-api-key here; if Quick
    // Setup later exposes ANTHROPIC_API_KEY, add an explicit auth-mode enum.
    if !api_key.is_empty() {
        headers.push(header("authorization", format!("Bearer {api_key}")));
    }
    headers
}

fn openai_headers(api_key: &str, event_stream_accept: bool) -> Vec<(String, String)> {
    let mut headers = vec![header("content-type", "application/json")];
    if event_stream_accept {
        headers.push(header("accept", "text/event-stream"));
        headers.push(header("accept-encoding", "identity"));
        headers.push(header(
            "user-agent",
            format!("codex_cli_rs/0.80.0 ({}; {})", os_name(), arch_name()),
        ));
        headers.push(header("originator", "codex_cli_rs"));
    } else {
        headers.push(header("accept", "application/json"));
    }
    if !api_key.is_empty() {
        headers.push(header("authorization", format!("Bearer {api_key}")));
    }
    headers
}

fn header(name: &str, value: impl Into<String>) -> (String, String) {
    (name.to_string(), value.into())
}

#[cfg(test)]
fn spec_header<'a>(spec: &'a ProbeRequestSpec, name: &str) -> Option<&'a str> {
    spec.headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
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

/// Lower bound for unique Latin text on cl100k-class tokenizers.
fn estimate_input_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Shared user content for every app probe. Numbered unique lines keep the
/// tokenizer from collapsing repetition, so short-prompt interceptors that
/// require ~1K input tokens still see a real-looking request.
fn probe_user_content() -> &'static str {
    static CONTENT: OnceLock<String> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let mut text = String::from(
            "Reply with a single letter. Numbered lines below fill the input window so short-prompt filters do not reject this connectivity check.\n",
        );
        let mut index = 1_u32;
        while estimate_input_tokens(&text) < MIN_PROBE_INPUT_TOKENS {
            text.push_str(&format!(
                "{index:04}. Distinct connectivity-check context for model probe input accounting.\n"
            ));
            index += 1;
        }
        text
    })
}

fn request_body(protocol: ProbeProtocol, model: &str, reasoning_effort: Option<&str>) -> Value {
    let content = probe_user_content();
    match protocol {
        ProbeProtocol::AnthropicMessages => json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": content }],
            "stream": true
        }),
        ProbeProtocol::OpenAiChat => {
            let mut body = json!({
                "model": model,
                "messages": [{ "role": "user", "content": content }],
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
                "input": [{ "role": "user", "content": content }],
                "stream": true
            });
            if let Some(effort) = reasoning_effort {
                body["reasoning"] = json!({ "effort": effort });
            }
            body
        }
    }
}

async fn send_stream_request(client: &Client, spec: &ProbeRequestSpec) -> Result<u16, AppError> {
    let body = serde_json::to_vec(&spec.body)
        .map_err(|error| AppError::Message(format!("序列化探测请求失败: {error}")))?;
    // Use `.body()` rather than `.json()`: reqwest's `.json()` already inserts
    // Content-Type, and `.header()` appends, producing two identical
    // `Content-Type: application/json` lines. Node-style gateways join those
    // into `application/json, application/json` and return 400
    // `Unsupported content type`.
    let mut request = client.post(&spec.url).timeout(TIMEOUT).body(body);
    for (name, value) in &spec.headers {
        request = request.header(name.as_str(), value);
    }

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
    if cfg!(target_os = "macos") {
        "MacOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "unknown"
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

    use crate::provider::Provider;
    use crate::proxy::providers::{ClaudeAdapter, ProviderAdapter};
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

    fn read_http_request(stream: &mut impl Read) -> String {
        let mut data = Vec::new();
        let mut chunk = [0_u8; 2048];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    data.extend_from_slice(&chunk[..n]);
                    if let Some((header_end, body_len)) = http_body_target(&data) {
                        if data.len() >= header_end + body_len {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&data).into_owned()
    }

    fn http_body_target(data: &[u8]) -> Option<(usize, usize)> {
        let header_end = data.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
        let headers = std::str::from_utf8(&data[..header_end]).ok()?;
        let body_len = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        Some((header_end, body_len))
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
                    captured
                        .lock()
                        .expect("requests")
                        .push(read_http_request(&mut stream));
                    let _ = stream.write_all(&response);
                    let _ = stream.flush();
                }
            }
        });
        (format!("http://127.0.0.1:{port}"), requests)
    }

    fn spec_user_content(spec: &ProbeRequestSpec) -> &str {
        spec.body
            .pointer("/messages/0/content")
            .or_else(|| spec.body.pointer("/input/0/content"))
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    fn spec_for(app: ModelProbeApp, base_url: &str, model_id: &str) -> ProbeRequestSpec {
        build_probe_spec(app, base_url, "sk-test", model_id, false)
            .expect("spec")
            .0
    }

    #[test]
    fn each_app_projects_one_determined_url() {
        assert_eq!(
            spec_for(ModelProbeApp::Claude, "https://gateway.example", "m").url,
            "https://gateway.example/v1/messages"
        );
        assert_eq!(
            spec_for(
                ModelProbeApp::Claude,
                "https://gateway.example/anthropic",
                "m"
            )
            .url,
            "https://gateway.example/anthropic/v1/messages"
        );
        assert_eq!(
            spec_for(ModelProbeApp::Claude, "https://gateway.example/v1", "m").url,
            "https://gateway.example/v1/v1/messages"
        );
        assert_eq!(
            spec_for(ModelProbeApp::Codex, "https://gateway.example", "m").url,
            "https://gateway.example/responses"
        );
        assert_eq!(
            spec_for(ModelProbeApp::Codex, "https://gateway.example/v1", "m").url,
            "https://gateway.example/v1/responses"
        );
        assert_eq!(
            spec_for(ModelProbeApp::GrokBuild, "https://gateway.example/v1", "m").url,
            "https://gateway.example/v1/responses"
        );
        assert_eq!(
            spec_for(ModelProbeApp::OpenCode, "https://gateway.example", "m").url,
            "https://gateway.example/chat/completions"
        );
        assert_eq!(
            spec_for(
                ModelProbeApp::OpenCode,
                "https://gateway.example/v1/chat/completions",
                "m"
            )
            .url,
            "https://gateway.example/v1/chat/completions"
        );
        assert_eq!(
            spec_for(ModelProbeApp::WorkBuddy, "https://gateway.example", "m").url,
            "https://gateway.example/v1/chat/completions"
        );
        assert_eq!(
            spec_for(
                ModelProbeApp::WorkBuddy,
                "https://gateway.example/openai",
                "m"
            )
            .url,
            "https://gateway.example/openai/v1/chat/completions"
        );
    }

    #[test]
    fn claude_probe_matches_quick_setup_auth_token_contract() {
        let base = "https://gateway.example/anthropic";
        let token = "sk-ant-test";
        let model = "claude-test";
        let provider = Provider::with_id(
            "fyagent-v2-quick-setup-claude".to_string(),
            "Claude".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": base,
                    "ANTHROPIC_AUTH_TOKEN": token,
                    "ANTHROPIC_MODEL": model,
                }
            }),
            None,
        );
        let adapter = ClaudeAdapter::new();
        let auth = adapter.extract_auth(&provider).expect("auth");
        let headers = adapter.get_auth_headers(&auth).expect("headers");
        assert_eq!(headers[0].0.as_str(), "authorization");
        assert_eq!(
            headers[0].1.to_str().expect("header value"),
            format!("Bearer {token}")
        );

        let spec = build_probe_spec(ModelProbeApp::Claude, base, token, model, false)
            .expect("spec")
            .0;
        assert_eq!(
            spec_header(&spec, "authorization"),
            Some("Bearer sk-ant-test")
        );
        assert!(spec_header(&spec, "x-api-key").is_none());
        assert_eq!(spec.url, format!("{base}/v1/messages"));
        assert_eq!(spec.body["max_tokens"], json!(1));
    }

    #[test]
    fn chat_probe_omits_max_tokens_even_for_o_series() {
        let workbuddy = spec_for(ModelProbeApp::WorkBuddy, "https://gateway.example/v1", "o3");
        let opencode = spec_for(
            ModelProbeApp::OpenCode,
            "https://gateway.example/v1",
            "o4-mini",
        );
        assert!(workbuddy.body.get("max_tokens").is_none());
        assert!(opencode.body.get("max_tokens").is_none());
        assert_eq!(workbuddy.body["stream"], json!(true));
        assert_eq!(opencode.body["stream"], json!(true));
        assert_eq!(spec_header(&workbuddy, "accept"), Some("application/json"));
        assert_eq!(spec_header(&opencode, "accept"), Some("application/json"));
        assert!(spec_header(&workbuddy, "originator").is_none());
    }

    #[test]
    fn all_apps_pad_probe_input_to_at_least_1k_tokens() {
        let apps = [
            ModelProbeApp::Claude,
            ModelProbeApp::Codex,
            ModelProbeApp::GrokBuild,
            ModelProbeApp::WorkBuddy,
            ModelProbeApp::OpenCode,
        ];
        let expected = probe_user_content();
        assert!(estimate_input_tokens(expected) >= MIN_PROBE_INPUT_TOKENS);
        for app in apps {
            let spec = spec_for(app, "https://gateway.example/v1", "m");
            assert_eq!(spec_user_content(&spec), expected);
            assert_ne!(spec_user_content(&spec), "ping");
        }
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
            ProbeProtocol::OpenAiResponses
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
            false,
        )
        .await
        .expect("probe");
        assert!(result.success);
        assert_eq!(result.model_used, "gpt-test");
        assert_eq!(result.http_status, Some(200));
        let captured = requests.lock().expect("requests").join("\n");
        let captured_lower = captured.to_ascii_lowercase();
        assert!(captured.contains("POST /v1/chat/completions"));
        assert!(captured.contains("Bearer sk-test"));
        assert!(captured.contains("\"stream\":true") || captured.contains("\"stream\": true"));
        assert!(captured_lower.contains("accept: application/json"));
        assert!(!captured_lower.contains("text/event-stream"));
        assert!(!captured.contains("max_tokens"));
        let content_types: Vec<&str> = captured
            .lines()
            .filter(|line| line.to_ascii_lowercase().starts_with("content-type:"))
            .collect();
        assert_eq!(
            content_types,
            ["content-type: application/json"],
            "wire Content-Type must be a single bare application/json, got:\n{captured}"
        );
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
            &base,
            "sk-test",
            "claude-test",
            false,
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
            false,
        )
        .await
        .expect_err("empty model");
        assert!(error.to_string().contains("模型 ID 为空"));
    }

    #[tokio::test]
    async fn codex_responses_probe_omits_output_limit_and_bounds_actor_header() {
        let body = "event: response.created\ndata: {\"type\":\"response.created\"}\n\n";
        let (base, requests) = spawn_server(vec![http_response("200 OK", body)]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::Codex,
            &format!("{base}/v1"),
            "sk-test",
            "gpt-test",
            true,
        )
        .await
        .expect("probe");
        assert!(result.success);
        let captured = requests.lock().expect("requests").join("\n");
        assert!(captured.contains("POST /v1/responses"));
        assert!(!captured.contains("max_output_tokens"));
        assert!(captured.contains(crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER));
        assert!(captured.contains(crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE));
    }

    #[tokio::test]
    async fn claude_probe_sends_bearer_to_claude_code_path() {
        let body = "event: message_start\ndata: {\"type\":\"message_start\"}\n\n";
        let (base, requests) = spawn_server(vec![http_response("200 OK", body)]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::Claude,
            &format!("{base}/anthropic"),
            "sk-ant-test",
            "claude-test",
            false,
        )
        .await
        .expect("probe");
        assert!(result.success);
        let captured = requests.lock().expect("requests").join("\n");
        assert!(captured.contains("POST /anthropic/v1/messages"));
        assert!(!captured.contains("POST /anthropic/messages "));
        assert!(captured.contains("Bearer sk-ant-test"));
        assert!(!captured.to_ascii_lowercase().contains("x-api-key"));
        assert!(captured.contains("\"max_tokens\":1") || captured.contains("\"max_tokens\": 1"));
    }

    #[tokio::test]
    async fn http_400_returns_body_without_url_fallback() {
        let (base, requests) = spawn_server(vec![http_response(
            "400 Bad Request",
            r#"{"error":{"message":"invalid_request_error: schema"}}"#,
        )]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::Claude,
            &format!("{base}/anthropic"),
            "sk-ant-test",
            "claude-test",
            false,
        )
        .await
        .expect("probe result");
        assert!(!result.success);
        assert_eq!(result.http_status, Some(400));
        assert!(result.message.contains("invalid_request_error: schema"));
        assert_eq!(requests.lock().expect("requests").len(), 1);
        let captured = requests.lock().expect("requests").join("\n");
        assert!(captured.contains("POST /anthropic/v1/messages"));
    }

    #[tokio::test]
    async fn grok_build_probe_uses_responses() {
        let body = "event: response.created\ndata: {\"type\":\"response.created\"}\n\n";
        let (base, requests) = spawn_server(vec![http_response("200 OK", body)]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::GrokBuild,
            &format!("{base}/v1"),
            "sk-test",
            "grok-test",
            false,
        )
        .await
        .expect("probe");
        assert!(result.success);
        let captured = requests.lock().expect("requests").join("\n");
        assert!(captured.contains("POST /v1/responses"));
        assert!(!captured.contains("chat/completions"));
        assert!(!captured.contains("max_output_tokens"));
    }

    #[tokio::test]
    async fn opencode_chat_probe_streams_without_event_stream_accept() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"OK\"}}]}\n\n";
        let (base, requests) = spawn_server(vec![http_response("200 OK", body)]);
        let result = probe_with_client(
            &loopback_client(),
            ModelProbeApp::OpenCode,
            &format!("{base}/v1"),
            "sk-test",
            "o3",
            false,
        )
        .await
        .expect("probe");
        assert!(result.success);
        let captured = requests.lock().expect("requests").join("\n");
        let captured_lower = captured.to_ascii_lowercase();
        assert!(captured.contains("POST /v1/chat/completions"));
        assert!(captured.contains("\"stream\":true") || captured.contains("\"stream\": true"));
        assert!(captured_lower.contains("accept: application/json"));
        assert!(!captured_lower.contains("text/event-stream"));
        assert!(!captured.contains("max_tokens"));
        let content_types: Vec<&str> = captured
            .lines()
            .filter(|line| line.to_ascii_lowercase().starts_with("content-type:"))
            .collect();
        assert_eq!(
            content_types,
            ["content-type: application/json"],
            "wire Content-Type must be a single bare application/json, got:\n{captured}"
        );
    }
}
