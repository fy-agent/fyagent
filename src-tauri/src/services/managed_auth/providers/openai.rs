//! OpenAI official login protocol for Managed Auth.
//!
//! Browser loopback PKCE constants come from OpenAI Codex
//! `36984da4424cb91b6bc88c6af8d73207930ac729` (`codex-rs/login`).
//! Device Code HTTP is the existing FyAgent Codex manager flow, moved here
//! so Proxy and Managed Auth share one protocol.

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;

use crate::proxy::http_client;

#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenAiOAuthError {
    #[error("authorization is still pending")]
    AuthorizationPending,
    #[error("authorization was denied")]
    AccessDenied,
    #[error("authorization expired")]
    ExpiredToken,
    #[error("token exchange failed")]
    TokenFetchFailed,
    #[error("refresh token is no longer valid")]
    RefreshTokenInvalid,
    #[error("network request failed")]
    NetworkError,
    #[error("identity or token payload is invalid")]
    ParseError,
    #[error("callback listener failed")]
    IoError,
    #[error("login was cancelled")]
    Cancelled,
}

pub(crate) const OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub(crate) const OPENAI_ISSUER: &str = "https://auth.openai.com";
pub(crate) const OPENAI_OFFICIAL_HOST: &str = "auth.openai.com";
pub(crate) const OPENAI_DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";
pub(crate) const OPENAI_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
pub(crate) const OPENAI_SCOPE: &str =
    "openid profile email offline_access api.connectors.read api.connectors.invoke";
pub(crate) const OPENAI_ORIGINATOR: &str = "codex_cli_rs";
pub(crate) const LOOPBACK_PREFERRED_PORT: u16 = 1455;
pub(crate) const LOOPBACK_FALLBACK_PORT: u16 = 1457;
pub(crate) const CALLBACK_PATH: &str = "/auth/callback";
const MAX_CALLBACK_BYTES: usize = 8192;
const CALLBACK_DEADLINE: Duration = Duration::from_secs(15 * 60);
const OAUTH_HTTP_TIMEOUT: Duration = Duration::from_secs(20);
const CODEX_USER_AGENT: &str = "fyagent-codex-oauth";
const DEVICE_CODE_DEFAULT_EXPIRES_IN: u64 = 900;
const POLLING_SAFETY_MARGIN_SECS: u64 = 3;

#[derive(Clone, Debug)]
pub(crate) struct OpenAiOAuthEndpoints {
    pub authorize_url: String,
    pub token_url: String,
    pub device_usercode_url: String,
    pub device_token_url: String,
    pub device_verification_url: String,
    pub device_redirect_uri: String,
    pub client_id: String,
}

impl OpenAiOAuthEndpoints {
    pub(crate) fn production() -> Self {
        Self {
            authorize_url: format!("{OPENAI_ISSUER}/oauth/authorize"),
            token_url: format!("{OPENAI_ISSUER}/oauth/token"),
            device_usercode_url: format!("{OPENAI_ISSUER}/api/accounts/deviceauth/usercode"),
            device_token_url: format!("{OPENAI_ISSUER}/api/accounts/deviceauth/token"),
            device_verification_url: OPENAI_DEVICE_VERIFICATION_URL.to_string(),
            device_redirect_uri: OPENAI_DEVICE_REDIRECT_URI.to_string(),
            client_id: OPENAI_CLIENT_ID.to_string(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_issuer(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            authorize_url: format!("{base}/oauth/authorize"),
            token_url: format!("{base}/oauth/token"),
            device_usercode_url: format!("{base}/api/accounts/deviceauth/usercode"),
            device_token_url: format!("{base}/api/accounts/deviceauth/token"),
            device_verification_url: format!("{base}/codex/device"),
            device_redirect_uri: format!("{base}/deviceauth/callback"),
            client_id: OPENAI_CLIENT_ID.to_string(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PkceCodes {
    pub code_verifier: String,
    pub code_challenge: String,
}

pub(crate) fn generate_pkce() -> PkceCodes {
    let bytes = random_bytes::<64>();
    let code_verifier = URL_SAFE_NO_PAD.encode(bytes);
    let digest = Sha256::digest(code_verifier.as_bytes());
    PkceCodes {
        code_verifier,
        code_challenge: URL_SAFE_NO_PAD.encode(digest),
    }
}

pub(crate) fn generate_state() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    let mut offset = 0;
    while offset < N {
        let uuid = Uuid::new_v4();
        let source = uuid.as_bytes();
        let take = (N - offset).min(source.len());
        out[offset..offset + take].copy_from_slice(&source[..take]);
        offset += take;
    }
    out
}

pub(crate) fn build_authorize_url(
    endpoints: &OpenAiOAuthEndpoints,
    redirect_uri: &str,
    pkce: &PkceCodes,
    state: &str,
) -> String {
    let mut url = Url::parse(&endpoints.authorize_url).expect("authorize URL is constant");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", &endpoints.client_id);
        query.append_pair("redirect_uri", redirect_uri);
        query.append_pair("scope", OPENAI_SCOPE);
        query.append_pair("code_challenge", &pkce.code_challenge);
        query.append_pair("code_challenge_method", "S256");
        query.append_pair("id_token_add_organizations", "true");
        query.append_pair("codex_cli_simplified_flow", "true");
        query.append_pair("state", state);
        query.append_pair("originator", OPENAI_ORIGINATOR);
    }
    url.to_string()
}

pub(crate) fn loopback_redirect_uri(port: u16) -> String {
    format!("http://localhost:{port}{CALLBACK_PATH}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoopbackBindOutcome {
    BothBusy,
}

pub(crate) fn bind_registered_loopback() -> Result<LoopbackListener, LoopbackBindOutcome> {
    match bind_loopback(LOOPBACK_PREFERRED_PORT) {
        Ok(listener) => Ok(listener),
        Err(_) => match bind_loopback(LOOPBACK_FALLBACK_PORT) {
            Ok(listener) => Ok(listener),
            Err(_) => Err(LoopbackBindOutcome::BothBusy),
        },
    }
}

pub(crate) struct LoopbackListener {
    pub port: u16,
    pub(crate) listener: TcpListener,
}

pub(crate) fn bind_loopback_port(port: u16) -> std::io::Result<LoopbackListener> {
    bind_loopback(port)
}

fn bind_loopback(port: u16) -> std::io::Result<LoopbackListener> {
    let std_listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))?;
    std_listener.set_nonblocking(true)?;
    let actual = std_listener.local_addr()?.port();
    let listener = TcpListener::from_std(std_listener)?;
    Ok(LoopbackListener {
        port: actual,
        listener,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallbackDecision {
    Authorized { code: String },
    Denied,
    Invalid,
}

pub(crate) fn validate_callback_request(
    method: &str,
    path: &str,
    host: Option<&str>,
    expected_port: u16,
    content_length: Option<usize>,
    raw_target: &str,
    expected_state: &str,
) -> CallbackDecision {
    if !method.eq_ignore_ascii_case("GET") {
        return CallbackDecision::Invalid;
    }
    if path != CALLBACK_PATH {
        return CallbackDecision::Invalid;
    }
    if !host_is_loopback(host, expected_port) {
        return CallbackDecision::Invalid;
    }
    if content_length.unwrap_or(0) > MAX_CALLBACK_BYTES {
        return CallbackDecision::Invalid;
    }
    let parsed = match Url::parse(&format!("http://127.0.0.1{raw_target}")) {
        Ok(url) => url,
        Err(_) => return CallbackDecision::Invalid,
    };
    if parsed.path() != CALLBACK_PATH {
        return CallbackDecision::Invalid;
    }
    let params: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    let Some(state) = params.get("state") else {
        return CallbackDecision::Invalid;
    };
    if !constant_eq(state, expected_state) {
        return CallbackDecision::Invalid;
    }
    if params.get("error").is_some_and(|value| !value.is_empty()) {
        return CallbackDecision::Denied;
    }
    match params.get("code") {
        Some(code) if !code.is_empty() && code.len() <= 512 => {
            CallbackDecision::Authorized { code: code.clone() }
        }
        _ => CallbackDecision::Invalid,
    }
}

fn host_is_loopback(host: Option<&str>, expected_port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim();
    let allowed = [
        format!("127.0.0.1:{expected_port}"),
        format!("localhost:{expected_port}"),
        format!("[::1]:{expected_port}"),
    ];
    allowed
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(host))
}

fn constant_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

pub(crate) async fn accept_one_callback(
    listener: LoopbackListener,
    expected_state: String,
    generation: u64,
    expected_generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> Result<CallbackDecision, OpenAiOAuthError> {
    let port = listener.port;
    let accept = tokio::time::timeout(CALLBACK_DEADLINE, async {
        loop {
            if *cancel.borrow()
                || expected_generation.load(std::sync::atomic::Ordering::SeqCst) != generation
            {
                return Err(OpenAiOAuthError::Cancelled);
            }
            tokio::select! {
                biased;
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                accepted = listener.listener.accept() => {
                    let (mut stream, _) = accepted.map_err(|_| OpenAiOAuthError::IoError)?;
                    let mut buf = vec![0u8; MAX_CALLBACK_BYTES + 1];
                    let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
                        .await
                        .map_err(|_| OpenAiOAuthError::NetworkError)?
                        .map_err(|_| OpenAiOAuthError::IoError)?;
                    if read == 0 || read > MAX_CALLBACK_BYTES {
                        let _ = write_callback_response(&mut stream, 400, "Bad Request").await;
                        return Ok(CallbackDecision::Invalid);
                    }
                    let request = parse_http_request(&buf[..read]);
                    let decision = match request {
                        Some((method, target, host, length)) => {
                            let path = target.split('?').next().unwrap_or(target.as_str());
                            validate_callback_request(
                                &method,
                                path,
                                host.as_deref(),
                                port,
                                length,
                                &target,
                                &expected_state,
                            )
                        }
                        None => CallbackDecision::Invalid,
                    };
                    let (status, body) = match decision {
                        CallbackDecision::Authorized { .. } => (200, "You can close this window."),
                        CallbackDecision::Denied => (403, "Authorization was cancelled."),
                        CallbackDecision::Invalid => (400, "Bad Request"),
                    };
                    let _ = write_callback_response(&mut stream, status, body).await;
                    return Ok(decision);
                }
            }
        }
    })
    .await;
    match accept {
        Ok(result) => result,
        Err(_) => Err(OpenAiOAuthError::ExpiredToken),
    }
}

fn parse_http_request(bytes: &[u8]) -> Option<(String, String, Option<String>, Option<usize>)> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut request = httparse::Request::new(&mut headers);
    match request.parse(bytes) {
        Ok(httparse::Status::Complete(_)) => {}
        _ => return None,
    }
    let method = request.method?.to_string();
    let target = request.path?.to_string();
    let host = request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("host"))
        .and_then(|header| std::str::from_utf8(header.value).ok())
        .map(ToOwned::to_owned);
    let length = request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("content-length"))
        .and_then(|header| std::str::from_utf8(header.value).ok())
        .and_then(|value| value.parse().ok());
    Some((method, target, host, length))
}

async fn write_callback_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        _ => "Bad Request",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await
}

pub(crate) fn open_system_browser(url: &str) -> Result<(), ErrorKind> {
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.kind());
    }
    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.kind());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        Err(ErrorKind::Unsupported)
    }
}

#[derive(Clone, Deserialize)]
pub(crate) struct OpenAiTokenGrant {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct OpenAiIdentity {
    pub subject: String,
    pub tenant: String,
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct DevicePollSuccess {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct IdTokenClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    organizations: Vec<OrgClaim>,
    #[serde(default, rename = "https://api.openai.com/auth")]
    openai_auth: Option<OpenAiAuthClaim>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OrgClaim {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAiAuthClaim {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceCodeGrant {
    pub device_auth_id: String,
    pub user_code: String,
    #[allow(dead_code)]
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub(crate) async fn request_device_usercode(
    endpoints: &OpenAiOAuthEndpoints,
) -> Result<DeviceCodeGrant, OpenAiOAuthError> {
    let response = send_bounded(
        http_client::get()
            .post(&endpoints.device_usercode_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", CODEX_USER_AGENT)
            .json(&serde_json::json!({ "client_id": endpoints.client_id })),
    )
    .await?;
    if !response.status().is_success() {
        return Err(OpenAiOAuthError::NetworkError);
    }
    let device: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|_| OpenAiOAuthError::ParseError)?;
    Ok(DeviceCodeGrant {
        device_auth_id: device.device_auth_id,
        user_code: sanitize_user_code(&device.user_code),
        verification_uri: endpoints.device_verification_url.clone(),
        expires_in: device.expires_in.unwrap_or(DEVICE_CODE_DEFAULT_EXPIRES_IN),
        interval: parse_interval(device.interval.as_ref()),
    })
}

pub(crate) async fn poll_device_authorization(
    endpoints: &OpenAiOAuthEndpoints,
    device_auth_id: &str,
    user_code: &str,
) -> Result<(String, String), OpenAiOAuthError> {
    let response = send_bounded(
        http_client::get()
            .post(&endpoints.device_token_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", CODEX_USER_AGENT)
            .json(&serde_json::json!({
                "device_auth_id": device_auth_id,
                "user_code": user_code,
            })),
    )
    .await?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
        return Err(OpenAiOAuthError::AuthorizationPending);
    }
    if status == reqwest::StatusCode::GONE {
        return Err(OpenAiOAuthError::ExpiredToken);
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(OpenAiOAuthError::AccessDenied);
    }
    if !status.is_success() {
        return Err(OpenAiOAuthError::TokenFetchFailed);
    }
    let success: DevicePollSuccess = response
        .json()
        .await
        .map_err(|_| OpenAiOAuthError::ParseError)?;
    Ok((success.authorization_code, success.code_verifier))
}

pub(crate) async fn exchange_authorization_code(
    endpoints: &OpenAiOAuthEndpoints,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<OpenAiTokenGrant, OpenAiOAuthError> {
    let response = send_bounded(
        http_client::get()
            .post(&endpoints.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", CODEX_USER_AGENT)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", endpoints.client_id.as_str()),
                ("code_verifier", code_verifier),
            ]),
    )
    .await?;
    if !response.status().is_success() {
        return Err(OpenAiOAuthError::TokenFetchFailed);
    }
    response
        .json()
        .await
        .map_err(|_| OpenAiOAuthError::ParseError)
}

pub(crate) async fn refresh_oauth_grant(
    refresh_token: &str,
) -> Result<OpenAiTokenGrant, OpenAiOAuthError> {
    refresh_oauth_grant_at(&OpenAiOAuthEndpoints::production(), refresh_token).await
}

pub(crate) async fn refresh_oauth_grant_at(
    endpoints: &OpenAiOAuthEndpoints,
    refresh_token: &str,
) -> Result<OpenAiTokenGrant, OpenAiOAuthError> {
    let response = send_bounded(
        http_client::get()
            .post(&endpoints.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", CODEX_USER_AGENT)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", endpoints.client_id.as_str()),
                ("scope", "openid profile email"),
            ]),
    )
    .await?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(OpenAiOAuthError::RefreshTokenInvalid);
    }
    if !status.is_success() {
        return Err(OpenAiOAuthError::TokenFetchFailed);
    }
    response
        .json()
        .await
        .map_err(|_| OpenAiOAuthError::ParseError)
}

pub(crate) fn extract_identity(
    grant: &OpenAiTokenGrant,
) -> Result<OpenAiIdentity, OpenAiOAuthError> {
    let id_claims = grant.id_token.as_deref().and_then(parse_jwt_claims);
    let access_claims = parse_jwt_claims(&grant.access_token);
    let id_subject = id_claims.as_ref().and_then(stable_subject);
    let access_subject = access_claims.as_ref().and_then(stable_subject);
    let subject = match (id_subject, access_subject) {
        (Some(left), Some(right)) if left != right => {
            return Err(OpenAiOAuthError::ParseError);
        }
        (Some(subject), _) | (_, Some(subject)) => subject,
        _ => {
            return Err(OpenAiOAuthError::ParseError);
        }
    };
    let tenant = id_claims
        .as_ref()
        .and_then(stable_tenant)
        .or_else(|| access_claims.as_ref().and_then(stable_tenant))
        .unwrap_or_default();
    let login = id_claims
        .as_ref()
        .and_then(|claims| claims.email.clone())
        .or_else(|| access_claims.and_then(|claims| claims.email))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| subject.clone());
    Ok(OpenAiIdentity {
        subject,
        tenant,
        login,
    })
}

fn stable_subject(claims: &IdTokenClaims) -> Option<String> {
    let top = claims.chatgpt_account_id.clone();
    let nested = claims
        .openai_auth
        .as_ref()
        .and_then(|auth| auth.chatgpt_account_id.clone());
    match (top, nested) {
        (Some(left), Some(right)) if left != right => None,
        (Some(subject), _) | (_, Some(subject)) => Some(subject),
        _ => None,
    }
}

fn stable_tenant(claims: &IdTokenClaims) -> Option<String> {
    claims
        .organizations
        .first()
        .and_then(|org| org.id.clone())
        .filter(|value| !value.is_empty())
}

fn parse_jwt_claims(token: &str) -> Option<IdTokenClaims> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    if parts.next().is_none() || parts.next().is_some() {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn parse_interval(value: Option<&serde_json::Value>) -> u64 {
    let raw = match value {
        Some(serde_json::Value::Number(number)) => number.as_u64().unwrap_or(5),
        Some(serde_json::Value::String(text)) => text.parse::<u64>().unwrap_or(5),
        _ => 5,
    };
    raw.max(1) + POLLING_SAFETY_MARGIN_SECS
}

fn sanitize_user_code(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect::<String>()
        .to_ascii_uppercase()
}

async fn send_bounded(
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, OpenAiOAuthError> {
    tokio::time::timeout(OAUTH_HTTP_TIMEOUT, request.send())
        .await
        .map_err(|_| OpenAiOAuthError::NetworkError)?
        .map_err(|_| OpenAiOAuthError::NetworkError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let pkce = generate_pkce();
        assert!(pkce.code_verifier.len() >= 43);
        let digest = Sha256::digest(pkce.code_verifier.as_bytes());
        assert_eq!(pkce.code_challenge, URL_SAFE_NO_PAD.encode(digest));
        assert!(!pkce.code_verifier.contains('='));
    }

    #[test]
    fn state_is_32_byte_entropy_encoded() {
        let state = generate_state();
        let decoded = URL_SAFE_NO_PAD.decode(&state).expect("state");
        assert_eq!(decoded.len(), 32);
        assert_ne!(state, generate_state());
    }

    #[test]
    fn authorize_url_uses_first_party_query_and_hides_nothing_required() {
        let endpoints = OpenAiOAuthEndpoints::production();
        let pkce = generate_pkce();
        let url = build_authorize_url(
            &endpoints,
            "http://localhost:1455/auth/callback",
            &pkce,
            "state-value",
        );
        let parsed = Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some(OPENAI_OFFICIAL_HOST));
        assert_eq!(parsed.path(), "/oauth/authorize");
        let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(query.get("originator").unwrap(), OPENAI_ORIGINATOR);
        assert_eq!(query.get("scope").unwrap(), OPENAI_SCOPE);
        assert!(!url.contains("code_verifier"));
    }

    #[test]
    fn callback_rejects_wrong_method_path_host_state_and_size() {
        let state = "expected-state";
        assert_eq!(
            validate_callback_request(
                "POST",
                CALLBACK_PATH,
                Some("127.0.0.1:1455"),
                1455,
                None,
                "/auth/callback?code=abc&state=expected-state",
                state,
            ),
            CallbackDecision::Invalid
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                "/other",
                Some("127.0.0.1:1455"),
                1455,
                None,
                "/other?code=abc&state=expected-state",
                state,
            ),
            CallbackDecision::Invalid
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                CALLBACK_PATH,
                Some("example.com:1455"),
                1455,
                None,
                "/auth/callback?code=abc&state=expected-state",
                state,
            ),
            CallbackDecision::Invalid
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                CALLBACK_PATH,
                Some("127.0.0.1:1455"),
                1455,
                Some(MAX_CALLBACK_BYTES + 1),
                "/auth/callback?code=abc&state=expected-state",
                state,
            ),
            CallbackDecision::Invalid
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                CALLBACK_PATH,
                Some("127.0.0.1:1455"),
                1455,
                None,
                "/auth/callback?code=abc&state=other-state",
                state,
            ),
            CallbackDecision::Invalid
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                CALLBACK_PATH,
                Some("127.0.0.1:1455"),
                1455,
                None,
                "/auth/callback?error=access_denied&state=expected-state",
                state,
            ),
            CallbackDecision::Denied
        );
        assert_eq!(
            validate_callback_request(
                "GET",
                CALLBACK_PATH,
                Some("localhost:1457"),
                1457,
                None,
                "/auth/callback?code=one-time&state=expected-state",
                state,
            ),
            CallbackDecision::Authorized {
                code: "one-time".to_string()
            }
        );
    }

    #[tokio::test]
    async fn occupying_preferred_port_falls_back_then_reports_busy() {
        let _preferred = std::net::TcpListener::bind(SocketAddr::from((
            [127, 0, 0, 1],
            LOOPBACK_PREFERRED_PORT,
        )));
        match bind_registered_loopback() {
            Ok(listener) => assert_eq!(listener.port, LOOPBACK_FALLBACK_PORT),
            Err(outcome) => assert_eq!(outcome, LoopbackBindOutcome::BothBusy),
        }
        let _fallback =
            std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], LOOPBACK_FALLBACK_PORT)));
        if _preferred.is_ok() && _fallback.is_ok() {
            assert_eq!(
                bind_registered_loopback().err(),
                Some(LoopbackBindOutcome::BothBusy)
            );
        }
    }

    #[test]
    fn conflicting_or_missing_subjects_are_rejected() {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let conflicting = URL_SAFE_NO_PAD.encode(
            br#"{"chatgpt_account_id":"acct-a","https://api.openai.com/auth":{"chatgpt_account_id":"acct-b"}}"#,
        );
        let missing = URL_SAFE_NO_PAD.encode(br#"{"email":"person@example.com"}"#);
        let valid = URL_SAFE_NO_PAD.encode(
            br#"{"chatgpt_account_id":"acct-1","email":"person@example.com","organizations":[{"id":"ws-1"}]}"#,
        );
        let grant = |payload: &str| OpenAiTokenGrant {
            access_token: format!("{header}.{payload}.sig"),
            refresh_token: Some("refresh".into()),
            id_token: None,
            expires_in: Some(3600),
        };
        assert!(extract_identity(&grant(&conflicting)).is_err());
        assert!(extract_identity(&grant(&missing)).is_err());
        let identity = extract_identity(&grant(&valid)).expect("identity");
        assert_eq!(identity.subject, "acct-1");
        assert_eq!(identity.tenant, "ws-1");
        assert_eq!(identity.login, "person@example.com");
    }
}
