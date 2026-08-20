//! Restricted TRAE Work model endpoint and external MCP validation.
//!
//! This module deliberately owns no vendor persistence and starts no process.
//! Endpoint probes use a short-lived, DNS-pinned client; MCP validation only
//! inspects the supplied document and executable metadata.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use futures::StreamExt;
use reqwest::{redirect::Policy, Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use url::{Host, Url};
use uuid::Uuid;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ENDPOINT_BYTES: usize = 4 * 1024;
const MAX_MODEL_ID_BYTES: usize = 512;
const MAX_API_KEY_BYTES: usize = 8 * 1024;
const MAX_ACTIVE_PROBES: usize = 16;

const MAX_MCP_CONFIG_BYTES: usize = 1024 * 1024;
const MAX_MCP_SERVERS: usize = 128;
const MAX_MCP_SERVER_ID_BYTES: usize = 128;
const MAX_MCP_COMMAND_BYTES: usize = 2 * 1024;
const MAX_MCP_ARGS: usize = 128;
const MAX_MCP_ARG_BYTES: usize = 8 * 1024;
const MAX_MCP_SECRET_FIELDS: usize = 128;
const MAX_MCP_SECRET_KEY_BYTES: usize = 256;
const MAX_MCP_SECRET_VALUE_BYTES: usize = 64 * 1024;

const REDACTED_TEMPLATE_VALUE: &str = "<redacted>";

#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct TraeSecret(String);

impl TraeSecret {
    fn trimmed(&self) -> &str {
        self.0.trim()
    }
}

impl fmt::Debug for TraeSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TraeSecret(<redacted>)")
    }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraeApiFormat {
    OpenaiChatCompletions,
    AnthropicMessages,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraeUrlMode {
    BaseUrl,
    CompleteUrl,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraeModelConfigRequest {
    api_format: TraeApiFormat,
    url_mode: TraeUrlMode,
    url: String,
    model_id: String,
    api_key: TraeSecret,
    allow_no_api_key: bool,
    allow_loopback: bool,
    allow_private_network: bool,
}

impl fmt::Debug for TraeModelConfigRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraeModelConfigRequest")
            .field("api_format", &"<closed>")
            .field("url_mode", &"<closed>")
            .field("url", &"<redacted>")
            .field("model_id", &"<redacted>")
            .field("api_key", &self.api_key)
            .field("allow_no_api_key", &self.allow_no_api_key)
            .field("allow_loopback", &self.allow_loopback)
            .field("allow_private_network", &self.allow_private_network)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraeEndpointProbeTerminalState {
    Valid,
    Reachable,
    AuthRejected,
    ModelRejected,
    NetworkRejected,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraeReasonCode {
    #[serde(rename = "TRAE_MODEL_CONFIG_VALID")]
    ModelConfigValid,
    #[serde(rename = "TRAE_ENDPOINT_REACHABLE")]
    EndpointReachable,
    #[serde(rename = "TRAE_ENDPOINT_AUTH_REJECTED")]
    EndpointAuthRejected,
    #[serde(rename = "TRAE_ENDPOINT_MODEL_REJECTED")]
    EndpointModelRejected,
    #[serde(rename = "TRAE_ENDPOINT_HTTP_REJECTED")]
    EndpointHttpRejected,
    #[serde(rename = "TRAE_ENDPOINT_NETWORK_REJECTED")]
    EndpointNetworkRejected,
    #[serde(rename = "TRAE_ENDPOINT_TIMEOUT")]
    EndpointTimeout,
    #[serde(rename = "TRAE_ENDPOINT_CANCELLED")]
    EndpointCancelled,
    #[serde(rename = "TRAE_DNS_RESOLUTION_FAILED")]
    DnsResolutionFailed,
    #[serde(rename = "TRAE_DNS_ADDRESS_REJECTED")]
    DnsAddressRejected,
    #[serde(rename = "TRAE_DNS_ADDRESS_CLASS_MIXED")]
    DnsAddressClassMixed,
    #[serde(rename = "TRAE_ENDPOINT_RESPONSE_TOO_LARGE")]
    EndpointResponseTooLarge,
    #[serde(rename = "PROXY_DNS_PIN_UNSUPPORTED")]
    ProxyDnsPinUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraeDurationBucket {
    #[serde(rename = "lt_1s")]
    LessThanOneSecond,
    #[serde(rename = "1s_to_3s")]
    OneToThreeSeconds,
    #[serde(rename = "3s_to_10s")]
    ThreeToTenSeconds,
    #[serde(rename = "gte_10s")]
    TenSecondsOrMore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraeHttpStatusClass {
    #[serde(rename = "2xx")]
    Success,
    #[serde(rename = "3xx")]
    Redirection,
    #[serde(rename = "4xx")]
    ClientError,
    #[serde(rename = "5xx")]
    ServerError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraeEndpointProbeResult {
    pub request_id: String,
    pub state: TraeEndpointProbeTerminalState,
    pub reason_code: TraeReasonCode,
    pub duration_bucket: TraeDurationBucket,
    pub status_class: Option<TraeHttpStatusClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraeEndpointCancelResult {
    pub request_id: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraeErrorCode {
    #[serde(rename = "TRAE_INVALID_MODEL_CONFIG")]
    InvalidModelConfig,
    #[serde(rename = "TRAE_INVALID_URL")]
    InvalidUrl,
    #[serde(rename = "TRAE_HTTPS_REQUIRED")]
    HttpsRequired,
    #[serde(rename = "TRAE_API_KEY_REQUIRED")]
    ApiKeyRequired,
    #[serde(rename = "TRAE_CREDENTIAL_COLLISION")]
    CredentialCollision,
    #[serde(rename = "TRAE_LOOPBACK_CONSENT_REQUIRED")]
    LoopbackConsentRequired,
    #[serde(rename = "TRAE_PRIVATE_NETWORK_CONSENT_REQUIRED")]
    PrivateNetworkConsentRequired,
    #[serde(rename = "TRAE_INVALID_REQUEST_ID")]
    InvalidRequestId,
    #[serde(rename = "TRAE_DUPLICATE_REQUEST_ID")]
    DuplicateRequestId,
    #[serde(rename = "TRAE_PROBE_CAPACITY_REACHED")]
    ProbeCapacityReached,
    #[serde(rename = "TRAE_STATE_UNAVAILABLE")]
    StateUnavailable,
    #[serde(rename = "TRAE_MCP_CONFIG_TOO_LARGE")]
    McpConfigTooLarge,
    #[serde(rename = "TRAE_MCP_INVALID_ROOT")]
    McpInvalidRoot,
    #[serde(rename = "TRAE_MCP_INVALID_SERVER")]
    McpInvalidServer,
    #[serde(rename = "TRAE_MCP_INVALID_TRANSPORT")]
    McpInvalidTransport,
    #[serde(rename = "TRAE_MODELS_STORE_UNAVAILABLE")]
    ModelsStoreUnavailable,
    #[serde(rename = "TRAE_MODELS_WRITE_FAILED")]
    ModelsWriteFailed,
    #[serde(rename = "TRAE_MODELS_BACKUP_FAILED")]
    ModelsBackupFailed,
    #[serde(rename = "TRAE_MODELS_NO_TARGET")]
    ModelsNoTarget,
    #[serde(rename = "TRAE_OVERWRITE_TOKEN_INVALID")]
    OverwriteTokenInvalid,
    #[serde(rename = "TRAE_OVERWRITE_TOKEN_EXPIRED")]
    OverwriteTokenExpired,
    #[serde(rename = "TRAE_SAVE_PROBE_REJECTED")]
    SaveProbeRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraeErrorDto {
    pub code: TraeErrorCode,
}

impl TraeErrorDto {
    pub(crate) const fn new(code: TraeErrorCode) -> Self {
        Self { code }
    }
}

pub(crate) struct ValidatedModelRequest {
    api_format: TraeApiFormat,
    endpoint: Url,
    model_id: String,
    api_key: TraeSecret,
    allow_loopback: bool,
    allow_private_network: bool,
}

pub fn validate_traework_model_config(
    request: TraeModelConfigRequest,
) -> Result<TraeEndpointProbeResult, TraeErrorDto> {
    let _validated = validate_model_request(request)?;
    Ok(TraeEndpointProbeResult {
        request_id: Uuid::new_v4().hyphenated().to_string(),
        state: TraeEndpointProbeTerminalState::Valid,
        reason_code: TraeReasonCode::ModelConfigValid,
        // Pure validation performs no timed external operation; keep this wire
        // deterministic instead of exposing scheduler latency.
        duration_bucket: TraeDurationBucket::LessThanOneSecond,
        status_class: None,
    })
}

pub(crate) fn validate_model_request(
    request: TraeModelConfigRequest,
) -> Result<ValidatedModelRequest, TraeErrorDto> {
    if request.url.is_empty()
        || request.url.len() > MAX_ENDPOINT_BYTES
        || request.model_id.is_empty()
        || request.model_id.len() > MAX_MODEL_ID_BYTES
        || request.api_key.0.len() > MAX_API_KEY_BYTES
        || has_control(&request.url)
        || has_control(&request.model_id)
        || has_control(&request.api_key.0)
    {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidModelConfig));
    }

    let model_id = request.model_id.trim();
    let api_key = request.api_key.trimmed();
    if model_id.is_empty() {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidModelConfig));
    }
    if api_key.is_empty() && !request.allow_no_api_key {
        return Err(TraeErrorDto::new(TraeErrorCode::ApiKeyRequired));
    }
    if !api_key.is_empty() && model_id.contains(api_key) {
        return Err(TraeErrorDto::new(TraeErrorCode::CredentialCollision));
    }

    let mut endpoint = parse_endpoint_url(&request.url, api_key)?;
    if request.url_mode == TraeUrlMode::BaseUrl {
        append_format_endpoint(&mut endpoint, request.api_format)?;
    }

    if endpoint.scheme() == "http" && endpoint.host_str().is_none() {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidUrl));
    }

    if let Some(host) = literal_ip(&endpoint) {
        admit_address_class(
            classify_ip(host),
            request.allow_loopback,
            request.allow_private_network,
        )?;
        if endpoint.scheme() == "http" && classify_ip(host) == AddressClass::Public {
            return Err(TraeErrorDto::new(TraeErrorCode::HttpsRequired));
        }
    } else if endpoint.scheme() == "http"
        && !request.allow_loopback
        && !request.allow_private_network
    {
        return Err(TraeErrorDto::new(TraeErrorCode::HttpsRequired));
    }

    Ok(ValidatedModelRequest {
        api_format: request.api_format,
        endpoint,
        model_id: model_id.to_owned(),
        api_key: request.api_key,
        allow_loopback: request.allow_loopback,
        allow_private_network: request.allow_private_network,
    })
}

fn parse_endpoint_url(raw: &str, api_key: &str) -> Result<Url, TraeErrorDto> {
    if has_control(raw) {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidUrl));
    }
    let parsed = Url::parse(raw).map_err(|_| TraeErrorDto::new(TraeErrorCode::InvalidUrl))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.port_or_known_default().is_none()
        || decoded_has_control(parsed.path())
        || metadata_hostname(parsed.host_str().unwrap_or_default())
    {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidUrl));
    }

    if !api_key.is_empty() {
        let decoded_path = percent_decode_lossy(parsed.path());
        if raw.contains(api_key)
            || parsed.host_str().is_some_and(|host| host.contains(api_key))
            || decoded_path.contains(api_key)
        {
            return Err(TraeErrorDto::new(TraeErrorCode::CredentialCollision));
        }
    }
    Ok(parsed)
}

fn append_format_endpoint(
    endpoint: &mut Url,
    api_format: TraeApiFormat,
) -> Result<(), TraeErrorDto> {
    let suffix: &[&str] = match api_format {
        TraeApiFormat::OpenaiChatCompletions => &["v1", "chat", "completions"],
        TraeApiFormat::AnthropicMessages => &["v1", "messages"],
    };
    let ends_in_v1 = endpoint
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .is_some_and(|segment| segment.eq_ignore_ascii_case("v1"));
    let mut segments = endpoint
        .path_segments_mut()
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::InvalidUrl))?;
    segments.pop_if_empty();
    for segment in suffix.iter().skip(usize::from(ends_in_v1)) {
        segments.push(segment);
    }
    Ok(())
}

fn has_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn decoded_has_control(value: &str) -> bool {
    percent_decode_lossy(value).chars().any(char::is_control)
}

fn percent_decode_lossy(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                output.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn literal_ip(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        Host::Ipv4(address) => Some(IpAddr::V4(address)),
        Host::Ipv6(address) => Some(IpAddr::V6(address)),
        Host::Domain(_) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AddressClass {
    Public,
    Private,
    Loopback,
    Blocked,
}

fn classify_ip(address: IpAddr) -> AddressClass {
    match address {
        IpAddr::V4(address) => classify_ipv4(address),
        IpAddr::V6(address) => classify_ipv6(address),
    }
}

fn classify_ipv4(address: Ipv4Addr) -> AddressClass {
    let [a, b, c, _d] = address.octets();
    if a == 127 {
        return AddressClass::Loopback;
    }
    if a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168) {
        return AddressClass::Private;
    }
    if a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        // Multicast, reserved space, and the all-ones broadcast are all in
        // 224.0.0.0/4 or 240.0.0.0/4.
        || a >= 224
        || is_metadata_ip(IpAddr::V4(address))
    {
        AddressClass::Blocked
    } else {
        AddressClass::Public
    }
}

fn classify_ipv6(address: Ipv6Addr) -> AddressClass {
    if address.is_loopback() {
        return AddressClass::Loopback;
    }
    if let Some(mapped) = address.to_ipv4_mapped() {
        return classify_ipv4(mapped);
    }
    let segments = address.segments();
    if address.is_unspecified()
        || (segments[0] & 0xff00) == 0xff00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || is_ipv6_prefix(address, "100::".parse().unwrap(), 64)
        || is_ipv6_prefix(address, "2001:db8::".parse().unwrap(), 32)
        || is_metadata_ip(IpAddr::V6(address))
    {
        return AddressClass::Blocked;
    }
    if (segments[0] & 0xfe00) == 0xfc00 {
        return AddressClass::Private;
    }
    if (segments[0] & 0xe000) == 0x2000 {
        AddressClass::Public
    } else {
        AddressClass::Blocked
    }
}

fn is_ipv6_prefix(address: Ipv6Addr, prefix: Ipv6Addr, bits: u32) -> bool {
    let mask = if bits == 0 {
        0
    } else {
        u128::MAX << (128 - bits)
    };
    (u128::from(address) & mask) == (u128::from(prefix) & mask)
}

fn is_metadata_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => matches!(
            address.octets(),
            [169, 254, 169, 254] | [169, 254, 0, 23] | [100, 100, 100, 200] | [192, 0, 0, 192]
        ),
        IpAddr::V6(address) => address == "fd00:ec2::254".parse::<Ipv6Addr>().unwrap(),
    }
}

fn metadata_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    matches!(
        host.as_str(),
        "metadata" | "instance-data" | "metadata.google.internal"
    ) || host.ends_with(".metadata.google.internal")
}

fn admit_address_class(
    class: AddressClass,
    allow_loopback: bool,
    allow_private_network: bool,
) -> Result<(), TraeErrorDto> {
    match class {
        AddressClass::Public => Ok(()),
        AddressClass::Private if allow_private_network => Ok(()),
        AddressClass::Private => Err(TraeErrorDto::new(
            TraeErrorCode::PrivateNetworkConsentRequired,
        )),
        AddressClass::Loopback if allow_loopback => Ok(()),
        AddressClass::Loopback => Err(TraeErrorDto::new(TraeErrorCode::LoopbackConsentRequired)),
        AddressClass::Blocked => Err(TraeErrorDto::new(TraeErrorCode::InvalidUrl)),
    }
}

fn duration_bucket(duration: Duration) -> TraeDurationBucket {
    if duration < Duration::from_secs(1) {
        TraeDurationBucket::LessThanOneSecond
    } else if duration < Duration::from_secs(3) {
        TraeDurationBucket::OneToThreeSeconds
    } else if duration < Duration::from_secs(10) {
        TraeDurationBucket::ThreeToTenSeconds
    } else {
        TraeDurationBucket::TenSecondsOrMore
    }
}

fn status_class(status: u16) -> Option<TraeHttpStatusClass> {
    match status / 100 {
        2 => Some(TraeHttpStatusClass::Success),
        3 => Some(TraeHttpStatusClass::Redirection),
        4 => Some(TraeHttpStatusClass::ClientError),
        5 => Some(TraeHttpStatusClass::ServerError),
        _ => None,
    }
}

#[derive(Default)]
struct TraeEndpointProbeStateInner {
    active: HashMap<Uuid, Arc<ProbeCancellation>>,
}

#[derive(Default)]
pub struct TraeEndpointProbeState {
    inner: Mutex<TraeEndpointProbeStateInner>,
}

impl TraeEndpointProbeState {
    pub fn register(
        &self,
        request_id: &str,
    ) -> Result<TraeEndpointProbeRegistration<'_>, TraeErrorDto> {
        let parsed = parse_request_id(request_id)?;
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| TraeErrorDto::new(TraeErrorCode::StateUnavailable))?;
        if inner.active.contains_key(&parsed) {
            return Err(TraeErrorDto::new(TraeErrorCode::DuplicateRequestId));
        }
        if inner.active.len() >= MAX_ACTIVE_PROBES {
            return Err(TraeErrorDto::new(TraeErrorCode::ProbeCapacityReached));
        }
        let cancellation = Arc::new(ProbeCancellation::default());
        inner.active.insert(parsed, Arc::clone(&cancellation));
        drop(inner);
        Ok(TraeEndpointProbeRegistration {
            state: self,
            request_id: parsed,
            cancellation,
        })
    }

    pub fn cancel(&self, request_id: &str) -> Result<TraeEndpointCancelResult, TraeErrorDto> {
        let parsed = parse_request_id(request_id)?;
        let cancellation = self
            .inner
            .lock()
            .map_err(|_| TraeErrorDto::new(TraeErrorCode::StateUnavailable))?
            .active
            .get(&parsed)
            .cloned();
        let cancelled = cancellation.is_some();
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        Ok(TraeEndpointCancelResult {
            request_id: parsed.hyphenated().to_string(),
            cancelled,
        })
    }

    fn finish(&self, request_id: Uuid, cancellation: &Arc<ProbeCancellation>) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner
                .active
                .get(&request_id)
                .is_some_and(|current| Arc::ptr_eq(current, cancellation))
            {
                inner.active.remove(&request_id);
            }
        }
    }

    #[cfg(test)]
    fn active_len(&self) -> usize {
        self.inner.lock().unwrap().active.len()
    }
}

pub struct TraeEndpointProbeRegistration<'a> {
    state: &'a TraeEndpointProbeState,
    request_id: Uuid,
    cancellation: Arc<ProbeCancellation>,
}

impl fmt::Debug for TraeEndpointProbeRegistration<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraeEndpointProbeRegistration")
            .field("request_id", &"<redacted>")
            .field("cancellation", &"<redacted>")
            .finish()
    }
}

impl TraeEndpointProbeRegistration<'_> {
    pub fn request_id(&self) -> String {
        self.request_id.hyphenated().to_string()
    }

    pub fn cancellation(&self) -> Arc<ProbeCancellation> {
        Arc::clone(&self.cancellation)
    }
}

impl Drop for TraeEndpointProbeRegistration<'_> {
    fn drop(&mut self) {
        self.state.finish(self.request_id, &self.cancellation);
    }
}

fn parse_request_id(request_id: &str) -> Result<Uuid, TraeErrorDto> {
    let parsed = Uuid::parse_str(request_id)
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::InvalidRequestId))?;
    if parsed.hyphenated().to_string() != request_id {
        return Err(TraeErrorDto::new(TraeErrorCode::InvalidRequestId));
    }
    Ok(parsed)
}

#[derive(Default)]
pub struct ProbeCancellation {
    requested: AtomicBool,
    notify: tokio::sync::Notify,
}

impl ProbeCancellation {
    fn cancel(&self) {
        self.requested.store(true, Ordering::Release);
        // One probe future observes each token. `notify_one` retains a permit
        // if cancellation wins the small gap before the future first polls.
        self.notify.notify_one();
    }

    fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    async fn cancelled(&self) {
        let notified = self.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

type ResolveFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<SocketAddr>, ()>> + Send + 'a>>;

trait TraeDnsResolver: Send + Sync {
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a>;
}

struct SystemDnsResolver;

impl TraeDnsResolver for SystemDnsResolver {
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a> {
        Box::pin(async move {
            tokio::net::lookup_host((host, port))
                .await
                .map(|addresses| addresses.collect())
                .map_err(|_| ())
        })
    }
}

#[derive(Clone, Copy)]
enum TraeProbeProxyPolicy {
    Direct,
    Proxied,
    Unsupported,
}

fn current_proxy_policy() -> TraeProbeProxyPolicy {
    match crate::proxy::http_client::installer_proxy_configuration() {
        Ok(crate::proxy::http_client::InstallerProxyConfiguration::Direct) => {
            TraeProbeProxyPolicy::Direct
        }
        Ok(crate::proxy::http_client::InstallerProxyConfiguration::Explicit(_))
        | Ok(crate::proxy::http_client::InstallerProxyConfiguration::System) => {
            TraeProbeProxyPolicy::Proxied
        }
        Err(()) => TraeProbeProxyPolicy::Unsupported,
    }
}

#[derive(Clone, Copy)]
struct ProbeLimits {
    connect_timeout: Duration,
    total_timeout: Duration,
    max_body_bytes: usize,
}

impl Default for ProbeLimits {
    fn default() -> Self {
        Self {
            connect_timeout: CONNECT_TIMEOUT,
            total_timeout: PROBE_TIMEOUT,
            max_body_bytes: MAX_RESPONSE_BYTES,
        }
    }
}

pub async fn test_traework_model_endpoint(
    request_id: String,
    request: TraeModelConfigRequest,
    cancellation: Arc<ProbeCancellation>,
) -> Result<TraeEndpointProbeResult, TraeErrorDto> {
    let validated = validate_model_request(request)?;
    let parsed_request_id = parse_request_id(&request_id)?;
    Ok(probe_with_dependencies(
        parsed_request_id,
        validated,
        cancellation,
        &SystemDnsResolver,
        current_proxy_policy(),
        ProbeLimits::default(),
    )
    .await)
}

async fn probe_with_dependencies(
    request_id: Uuid,
    request: ValidatedModelRequest,
    cancellation: Arc<ProbeCancellation>,
    resolver: &dyn TraeDnsResolver,
    proxy_policy: TraeProbeProxyPolicy,
    limits: ProbeLimits,
) -> TraeEndpointProbeResult {
    let started = Instant::now();
    let operation = probe_once(request, resolver, proxy_policy, limits);
    let outcome = tokio::select! {
        _ = cancellation.cancelled() => ProbeOutcome::cancelled(),
        timed = tokio::time::timeout(limits.total_timeout, operation) => {
            match timed {
                Ok(outcome) => outcome,
                Err(_) => ProbeOutcome::timeout(),
            }
        }
    };
    TraeEndpointProbeResult {
        request_id: request_id.hyphenated().to_string(),
        state: outcome.state,
        reason_code: outcome.reason_code,
        duration_bucket: duration_bucket(started.elapsed()),
        status_class: outcome.status_class,
    }
}

#[derive(Clone, Copy)]
struct ProbeOutcome {
    state: TraeEndpointProbeTerminalState,
    reason_code: TraeReasonCode,
    status_class: Option<TraeHttpStatusClass>,
}

impl ProbeOutcome {
    const fn new(
        state: TraeEndpointProbeTerminalState,
        reason_code: TraeReasonCode,
        status_class: Option<TraeHttpStatusClass>,
    ) -> Self {
        Self {
            state,
            reason_code,
            status_class,
        }
    }

    const fn network(reason_code: TraeReasonCode) -> Self {
        Self::new(
            TraeEndpointProbeTerminalState::NetworkRejected,
            reason_code,
            None,
        )
    }

    const fn timeout() -> Self {
        Self::new(
            TraeEndpointProbeTerminalState::Timeout,
            TraeReasonCode::EndpointTimeout,
            None,
        )
    }

    const fn cancelled() -> Self {
        Self::new(
            TraeEndpointProbeTerminalState::Cancelled,
            TraeReasonCode::EndpointCancelled,
            None,
        )
    }
}

async fn probe_once(
    request: ValidatedModelRequest,
    resolver: &dyn TraeDnsResolver,
    proxy_policy: TraeProbeProxyPolicy,
    limits: ProbeLimits,
) -> ProbeOutcome {
    if matches!(proxy_policy, TraeProbeProxyPolicy::Unsupported) {
        return ProbeOutcome::network(TraeReasonCode::ProxyDnsPinUnsupported);
    }
    if matches!(proxy_policy, TraeProbeProxyPolicy::Proxied) {
        let client = match build_proxied_client(limits) {
            Ok(client) => client,
            Err(()) => return ProbeOutcome::network(TraeReasonCode::ProxyDnsPinUnsupported),
        };
        return complete_probe_http(client, &request, limits).await;
    }

    let host = request.endpoint.host_str().unwrap_or_default().to_owned();
    let port = request.endpoint.port_or_known_default().unwrap_or_default();
    let mut addresses = match resolver.resolve(&host, port).await {
        Ok(addresses) if !addresses.is_empty() => addresses,
        _ => return ProbeOutcome::network(TraeReasonCode::DnsResolutionFailed),
    };
    addresses.sort_unstable();
    addresses.dedup();

    let address_class = match approve_resolved_addresses(&addresses) {
        Ok(class) => class,
        Err(reason) => return ProbeOutcome::network(reason),
    };
    if matches!(address_class, AddressClass::Loopback) && !request.allow_loopback
        || matches!(address_class, AddressClass::Private) && !request.allow_private_network
    {
        return ProbeOutcome::network(TraeReasonCode::DnsAddressRejected);
    }
    if request.endpoint.scheme() == "http" && matches!(address_class, AddressClass::Public) {
        return ProbeOutcome::network(TraeReasonCode::DnsAddressRejected);
    }

    let client = match build_pinned_client(&host, &addresses, limits.connect_timeout) {
        Ok(client) => client,
        Err(()) => return ProbeOutcome::network(TraeReasonCode::EndpointNetworkRejected),
    };
    complete_probe_http(client, &request, limits).await
}

async fn complete_probe_http(
    client: Client,
    request: &ValidatedModelRequest,
    limits: ProbeLimits,
) -> ProbeOutcome {
    let response = match send_model_probe(&client, request).await {
        Ok(response) => response,
        Err(_) => {
            return ProbeOutcome::network(TraeReasonCode::EndpointNetworkRejected);
        }
    };
    let status = response.status().as_u16();
    if consume_bounded_body(response, limits.max_body_bytes)
        .await
        .is_err()
    {
        return ProbeOutcome::network(TraeReasonCode::EndpointResponseTooLarge);
    }

    let class = status_class(status);
    match status {
        200..=299 => ProbeOutcome::new(
            TraeEndpointProbeTerminalState::Reachable,
            TraeReasonCode::EndpointReachable,
            class,
        ),
        401 | 403 => ProbeOutcome::new(
            TraeEndpointProbeTerminalState::AuthRejected,
            TraeReasonCode::EndpointAuthRejected,
            class,
        ),
        400 | 404 | 409 | 422 => ProbeOutcome::new(
            TraeEndpointProbeTerminalState::ModelRejected,
            TraeReasonCode::EndpointModelRejected,
            class,
        ),
        _ => ProbeOutcome::new(
            TraeEndpointProbeTerminalState::NetworkRejected,
            TraeReasonCode::EndpointHttpRejected,
            class,
        ),
    }
}

fn approve_resolved_addresses(addresses: &[SocketAddr]) -> Result<AddressClass, TraeReasonCode> {
    let mut classes = HashSet::new();
    for address in addresses {
        let class = classify_ip(address.ip());
        if class == AddressClass::Blocked {
            return Err(TraeReasonCode::DnsAddressRejected);
        }
        classes.insert(class);
    }
    if classes.len() != 1 {
        return Err(TraeReasonCode::DnsAddressClassMixed);
    }
    classes
        .into_iter()
        .next()
        .ok_or(TraeReasonCode::DnsResolutionFailed)
}

fn build_pinned_client(
    host: &str,
    addresses: &[SocketAddr],
    connect_timeout: Duration,
) -> Result<Client, ()> {
    Client::builder()
        .redirect(Policy::none())
        .connect_timeout(connect_timeout)
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(|_| ())
}

fn build_proxied_client(limits: ProbeLimits) -> Result<Client, ()> {
    let builder = Client::builder()
        .redirect(Policy::none())
        .connect_timeout(limits.connect_timeout)
        .timeout(limits.total_timeout)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd();
    crate::proxy::http_client::apply_installer_proxy(builder)?
        .build()
        .map_err(|_| ())
}

async fn send_model_probe(
    client: &Client,
    request: &ValidatedModelRequest,
) -> Result<Response, reqwest::Error> {
    let mut builder = client
        .post(request.endpoint.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json");
    let body = match request.api_format {
        TraeApiFormat::OpenaiChatCompletions => {
            if !request.api_key.trimmed().is_empty() {
                builder = builder.bearer_auth(request.api_key.trimmed());
            }
            json!({
                "model": request.model_id,
                "messages": [{"role": "user", "content": "ping"}],
                "max_tokens": 1,
                "stream": false
            })
        }
        TraeApiFormat::AnthropicMessages => {
            builder = builder.header("anthropic-version", "2023-06-01");
            if !request.api_key.trimmed().is_empty() {
                builder = builder.header("x-api-key", request.api_key.trimmed());
            }
            json!({
                "model": request.model_id,
                "messages": [{"role": "user", "content": "ping"}],
                "max_tokens": 1
            })
        }
    };
    builder.json(&body).send().await
}

async fn consume_bounded_body(response: Response, limit: usize) -> Result<(), ()> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(());
    }
    let mut total = 0usize;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ())?;
        total = total.checked_add(chunk.len()).ok_or(())?;
        if total > limit {
            return Err(());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ExternalMcpAgentId {
    #[serde(rename = "qoderwork")]
    QoderWork,
    #[serde(rename = "trae-work")]
    TraeWork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraeMcpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraeMcpReasonCode {
    #[serde(rename = "TRAE_MCP_SERVER_VALID")]
    ServerValid,
    #[serde(rename = "TRAE_MCP_UNKNOWN_FIELD")]
    UnknownField,
    #[serde(rename = "TRAE_MCP_INVALID_COMMAND")]
    InvalidCommand,
    #[serde(rename = "TRAE_MCP_COMMAND_NOT_FOUND")]
    CommandNotFound,
    #[serde(rename = "TRAE_MCP_INVALID_ARGS")]
    InvalidArgs,
    #[serde(rename = "TRAE_MCP_INVALID_ENV")]
    InvalidEnv,
    #[serde(rename = "TRAE_MCP_INVALID_URL")]
    InvalidUrl,
    #[serde(rename = "TRAE_MCP_UNSAFE_ADDRESS")]
    UnsafeAddress,
    #[serde(rename = "TRAE_MCP_INVALID_HEADERS")]
    InvalidHeaders,
    #[serde(rename = "TRAE_MCP_CONTROL_CHARACTER")]
    ControlCharacter,
    #[serde(rename = "TRAE_MCP_LIMIT_EXCEEDED")]
    LimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraeMcpFinding {
    pub server_id: String,
    pub transport: TraeMcpTransport,
    pub reason_codes: Vec<TraeMcpReasonCode>,
    pub executable_available: Option<bool>,
    pub has_secrets: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraeMcpValidationResult {
    pub agent_id: ExternalMcpAgentId,
    pub valid: bool,
    pub findings: Vec<TraeMcpFinding>,
    pub redacted_template: Value,
}

trait ExecutableResolver {
    fn executable_exists(&self, command: &str) -> bool;
}

struct SystemExecutableResolver;

impl ExecutableResolver for SystemExecutableResolver {
    fn executable_exists(&self, command: &str) -> bool {
        if (command.contains('/') || command.contains('\\')) && !Path::new(command).is_file() {
            return false;
        }
        crate::claude_mcp::validate_command_in_path(command).unwrap_or(false)
    }
}

pub fn validate_external_mcp_config(
    agent_id: ExternalMcpAgentId,
    config: Value,
) -> Result<TraeMcpValidationResult, TraeErrorDto> {
    validate_external_mcp_config_with(agent_id, config, &SystemExecutableResolver)
}

fn validate_external_mcp_config_with(
    agent_id: ExternalMcpAgentId,
    config: Value,
    executable_resolver: &dyn ExecutableResolver,
) -> Result<TraeMcpValidationResult, TraeErrorDto> {
    let encoded = serde_json::to_vec(&config)
        .map_err(|_| TraeErrorDto::new(TraeErrorCode::McpInvalidRoot))?;
    if encoded.len() > MAX_MCP_CONFIG_BYTES {
        return Err(TraeErrorDto::new(TraeErrorCode::McpConfigTooLarge));
    }

    let root = config
        .as_object()
        .ok_or_else(|| TraeErrorDto::new(TraeErrorCode::McpInvalidRoot))?;
    if root.len() != 1 || root.keys().any(|key| dangerous_key(key)) {
        return Err(TraeErrorDto::new(TraeErrorCode::McpInvalidRoot));
    }
    let servers = root
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| TraeErrorDto::new(TraeErrorCode::McpInvalidRoot))?;
    if servers.len() > MAX_MCP_SERVERS {
        return Err(TraeErrorDto::new(TraeErrorCode::McpConfigTooLarge));
    }

    let mut findings = Vec::with_capacity(servers.len());
    let mut redacted_servers = Map::new();
    for (server_id, server) in servers {
        validate_server_id(server_id)?;
        let server = server
            .as_object()
            .ok_or_else(|| TraeErrorDto::new(TraeErrorCode::McpInvalidServer))?;
        if server.keys().any(|key| dangerous_key(key)) {
            return Err(TraeErrorDto::new(TraeErrorCode::McpInvalidServer));
        }
        let has_command = server.contains_key("command");
        let has_url = server.contains_key("url");
        let transport = match (has_command, has_url) {
            (true, false) => TraeMcpTransport::Stdio,
            (false, true) => TraeMcpTransport::Http,
            _ => return Err(TraeErrorDto::new(TraeErrorCode::McpInvalidTransport)),
        };

        let (mut finding, redacted) = match transport {
            TraeMcpTransport::Stdio => {
                validate_stdio_server(server_id, server, executable_resolver)
            }
            TraeMcpTransport::Http => validate_http_server(server_id, server),
        };
        if finding.reason_codes.is_empty() {
            finding.reason_codes.push(TraeMcpReasonCode::ServerValid);
        }
        if let Some(redacted) = redacted {
            redacted_servers.insert(server_id.clone(), Value::Object(redacted));
        }
        findings.push(finding);
    }

    let valid = findings
        .iter()
        .all(|finding| finding.reason_codes.as_slice() == [TraeMcpReasonCode::ServerValid]);
    Ok(TraeMcpValidationResult {
        agent_id,
        valid,
        findings,
        redacted_template: json!({ "mcpServers": redacted_servers }),
    })
}

fn validate_server_id(server_id: &str) -> Result<(), TraeErrorDto> {
    if server_id.trim().is_empty()
        || server_id.len() > MAX_MCP_SERVER_ID_BYTES
        || server_id.trim() != server_id
        || has_control(server_id)
        || dangerous_key(server_id)
    {
        return Err(TraeErrorDto::new(TraeErrorCode::McpInvalidServer));
    }
    Ok(())
}

fn validate_stdio_server(
    server_id: &str,
    server: &Map<String, Value>,
    executable_resolver: &dyn ExecutableResolver,
) -> (TraeMcpFinding, Option<Map<String, Value>>) {
    let mut reason_codes = Vec::new();
    if server
        .keys()
        .any(|key| !matches!(key.as_str(), "command" | "args" | "env"))
    {
        push_reason(&mut reason_codes, TraeMcpReasonCode::UnknownField);
    }

    let command = server.get("command").and_then(Value::as_str);
    let command_valid = command.is_some_and(valid_command);
    if !command_valid {
        push_reason(&mut reason_codes, TraeMcpReasonCode::InvalidCommand);
        if command.is_some_and(has_control) {
            push_reason(&mut reason_codes, TraeMcpReasonCode::ControlCharacter);
        }
    }

    let args_valid = match server.get("args") {
        None => true,
        Some(value) => validate_string_array(value, MAX_MCP_ARGS, MAX_MCP_ARG_BYTES),
    };
    if !args_valid {
        push_reason(&mut reason_codes, TraeMcpReasonCode::InvalidArgs);
        if server.get("args").is_some_and(value_contains_control) {
            push_reason(&mut reason_codes, TraeMcpReasonCode::ControlCharacter);
        }
    }

    let (env_valid, has_secrets) = match server.get("env") {
        None => (true, false),
        Some(value) => (
            validate_secret_map(value),
            value.as_object().is_some_and(|map| !map.is_empty()),
        ),
    };
    if !env_valid {
        push_reason(&mut reason_codes, TraeMcpReasonCode::InvalidEnv);
        if server.get("env").is_some_and(value_contains_control) {
            push_reason(&mut reason_codes, TraeMcpReasonCode::ControlCharacter);
        }
    }

    let executable_available = command.filter(|_| command_valid).map(|command| {
        let available = executable_resolver.executable_exists(command);
        if !available {
            push_reason(&mut reason_codes, TraeMcpReasonCode::CommandNotFound);
        }
        available
    });

    let mut redacted = Map::new();
    if let Some(command) = command.filter(|_| command_valid) {
        redacted.insert("command".to_owned(), Value::String(command.to_owned()));
    }
    if args_valid {
        if let Some(args) = server.get("args") {
            redacted.insert("args".to_owned(), args.clone());
        }
    }
    if env_valid {
        if let Some(env) = server.get("env").and_then(Value::as_object) {
            redacted.insert("env".to_owned(), Value::Object(redact_secret_map(env)));
        }
    }

    (
        TraeMcpFinding {
            server_id: server_id.to_owned(),
            transport: TraeMcpTransport::Stdio,
            reason_codes,
            executable_available,
            has_secrets,
        },
        Some(redacted),
    )
}

fn validate_http_server(
    server_id: &str,
    server: &Map<String, Value>,
) -> (TraeMcpFinding, Option<Map<String, Value>>) {
    let mut reason_codes = Vec::new();
    if server
        .keys()
        .any(|key| !matches!(key.as_str(), "url" | "headers"))
    {
        push_reason(&mut reason_codes, TraeMcpReasonCode::UnknownField);
    }

    let raw_url = server.get("url").and_then(Value::as_str);
    let url_result = raw_url.map(validate_external_mcp_url);
    match url_result {
        Some(Ok(())) => {}
        Some(Err(TraeMcpReasonCode::UnsafeAddress)) => {
            push_reason(&mut reason_codes, TraeMcpReasonCode::UnsafeAddress)
        }
        _ => {
            push_reason(&mut reason_codes, TraeMcpReasonCode::InvalidUrl);
            if raw_url.is_some_and(has_control) {
                push_reason(&mut reason_codes, TraeMcpReasonCode::ControlCharacter);
            }
        }
    }

    let (headers_valid, has_secrets) = match server.get("headers") {
        None => (true, false),
        Some(value) => (
            validate_secret_map(value),
            value.as_object().is_some_and(|map| !map.is_empty()),
        ),
    };
    if !headers_valid {
        push_reason(&mut reason_codes, TraeMcpReasonCode::InvalidHeaders);
        if server.get("headers").is_some_and(value_contains_control) {
            push_reason(&mut reason_codes, TraeMcpReasonCode::ControlCharacter);
        }
    }

    let mut redacted = Map::new();
    if matches!(url_result, Some(Ok(()))) {
        redacted.insert(
            "url".to_owned(),
            Value::String(raw_url.unwrap_or_default().to_owned()),
        );
    }
    if headers_valid {
        if let Some(headers) = server.get("headers").and_then(Value::as_object) {
            redacted.insert(
                "headers".to_owned(),
                Value::Object(redact_secret_map(headers)),
            );
        }
    }

    (
        TraeMcpFinding {
            server_id: server_id.to_owned(),
            transport: TraeMcpTransport::Http,
            reason_codes,
            executable_available: None,
            has_secrets,
        },
        Some(redacted),
    )
}

fn valid_command(command: &str) -> bool {
    if command.is_empty()
        || command.len() > MAX_MCP_COMMAND_BYTES
        || command.trim() != command
        || has_control(command)
    {
        return false;
    }
    if command.contains('/') || command.contains('\\') {
        return Path::new(command).is_absolute();
    }
    !command.chars().any(char::is_whitespace)
}

fn validate_string_array(value: &Value, max_items: usize, max_item_bytes: usize) -> bool {
    value.as_array().is_some_and(|items| {
        items.len() <= max_items
            && items.iter().all(|item| {
                item.as_str()
                    .is_some_and(|item| item.len() <= max_item_bytes && !has_control(item))
            })
    })
}

fn validate_secret_map(value: &Value) -> bool {
    value.as_object().is_some_and(|map| {
        map.len() <= MAX_MCP_SECRET_FIELDS
            && map.iter().all(|(key, value)| {
                !key.is_empty()
                    && key.len() <= MAX_MCP_SECRET_KEY_BYTES
                    && !has_control(key)
                    && !dangerous_key(key)
                    && value.as_str().is_some_and(|value| {
                        value.len() <= MAX_MCP_SECRET_VALUE_BYTES && !has_control(value)
                    })
            })
    })
}

fn redact_secret_map(map: &Map<String, Value>) -> Map<String, Value> {
    map.keys()
        .map(|key| {
            (
                key.clone(),
                Value::String(REDACTED_TEMPLATE_VALUE.to_owned()),
            )
        })
        .collect()
}

fn validate_external_mcp_url(raw: &str) -> Result<(), TraeMcpReasonCode> {
    if raw.is_empty()
        || raw.len() > MAX_ENDPOINT_BYTES
        || has_control(raw)
        || decoded_has_control(raw)
    {
        return Err(TraeMcpReasonCode::InvalidUrl);
    }
    let parsed = Url::parse(raw).map_err(|_| TraeMcpReasonCode::InvalidUrl)?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.port_or_known_default().is_none()
        || decoded_has_control(parsed.path())
    {
        return Err(TraeMcpReasonCode::InvalidUrl);
    }
    let host = parsed.host_str().unwrap_or_default();
    if metadata_hostname(host)
        || host.eq_ignore_ascii_case("localhost")
        || host.to_ascii_lowercase().ends_with(".localhost")
    {
        return Err(TraeMcpReasonCode::UnsafeAddress);
    }
    if literal_ip(&parsed).is_some_and(|address| classify_ip(address) != AddressClass::Public) {
        return Err(TraeMcpReasonCode::UnsafeAddress);
    }
    Ok(())
}

fn dangerous_key(key: &str) -> bool {
    matches!(key, "__proto__" | "prototype" | "constructor")
}

fn value_contains_control(value: &Value) -> bool {
    match value {
        Value::String(value) => has_control(value),
        Value::Array(values) => values.iter().any(value_contains_control),
        Value::Object(values) => values
            .iter()
            .any(|(key, value)| has_control(key) || value_contains_control(value)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn push_reason(reasons: &mut Vec<TraeMcpReasonCode>, reason: TraeMcpReasonCode) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    use futures::future;

    use super::*;

    const SECRET_SENTINEL: &str = "fyagent-trae-secret-sentinel";

    fn model_request(
        api_format: &str,
        url_mode: &str,
        url: String,
        api_key: &str,
        allow_loopback: bool,
        allow_private_network: bool,
    ) -> TraeModelConfigRequest {
        serde_json::from_value(json!({
            "apiFormat": api_format,
            "urlMode": url_mode,
            "url": url,
            "modelId": "test-model",
            "apiKey": api_key,
            "allowNoApiKey": api_key.is_empty(),
            "allowLoopback": allow_loopback,
            "allowPrivateNetwork": allow_private_network
        }))
        .expect("test request must match the closed wire")
    }

    #[test]
    fn validation_wire_is_exact_and_secret_debug_is_redacted() {
        let request = model_request(
            "openai_chat_completions",
            "complete_url",
            "https://api.example.test/v1/chat/completions".to_owned(),
            SECRET_SENTINEL,
            false,
            false,
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains(SECRET_SENTINEL));
        assert!(debug.contains("<redacted>"));

        let result = validate_traework_model_config(request).expect("valid request");
        assert!(Uuid::parse_str(&result.request_id).is_ok());
        assert_eq!(result.state, TraeEndpointProbeTerminalState::Valid);
        let wire = serde_json::to_value(result).unwrap();
        assert_eq!(
            wire.as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<HashSet<_>>(),
            [
                "requestId",
                "state",
                "reasonCode",
                "durationBucket",
                "statusClass"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(wire["state"], "valid");
        assert_eq!(wire["reasonCode"], "TRAE_MODEL_CONFIG_VALID");
        assert_eq!(wire["durationBucket"], "lt_1s");
        assert!(wire["statusClass"].is_null());
        assert!(!wire.to_string().contains(SECRET_SENTINEL));
    }

    #[test]
    fn model_request_rejects_unknown_enums_fields_and_secret_collisions() {
        let mut base = json!({
            "apiFormat": "openai_chat_completions",
            "urlMode": "complete_url",
            "url": "https://api.example.test/v1/chat/completions",
            "modelId": "test-model",
            "apiKey": SECRET_SENTINEL,
            "allowNoApiKey": false,
            "allowLoopback": false,
            "allowPrivateNetwork": false
        });
        base["apiFormat"] = json!("gemini");
        assert!(serde_json::from_value::<TraeModelConfigRequest>(base.clone()).is_err());
        base["apiFormat"] = json!("openai_chat_completions");
        base["unexpected"] = json!(true);
        assert!(serde_json::from_value::<TraeModelConfigRequest>(base).is_err());

        let collision = model_request(
            "openai_chat_completions",
            "complete_url",
            "https://api.example.test/v1/chat/completions".to_owned(),
            SECRET_SENTINEL,
            false,
            false,
        );
        let mut collision = collision;
        collision.model_id = format!("model-{SECRET_SENTINEL}");
        assert_eq!(
            validate_traework_model_config(collision).unwrap_err().code,
            TraeErrorCode::CredentialCollision
        );
    }

    #[test]
    fn model_url_policy_rejects_credentials_query_fragment_controls_and_public_http() {
        for invalid in [
            "https://user@example.test/v1/chat/completions",
            "https://example.test/v1/chat/completions?token=x",
            "https://example.test/v1/chat/completions#secret",
            "file:///tmp/endpoint",
            "https://metadata.google.internal/v1/chat/completions",
        ] {
            let request = model_request(
                "openai_chat_completions",
                "complete_url",
                invalid.to_owned(),
                SECRET_SENTINEL,
                false,
                false,
            );
            assert!(
                validate_traework_model_config(request).is_err(),
                "URL must fail closed: {invalid}"
            );
        }

        let public_http = model_request(
            "openai_chat_completions",
            "complete_url",
            "http://203.0.113.1/v1/chat/completions".to_owned(),
            SECRET_SENTINEL,
            true,
            true,
        );
        assert!(validate_traework_model_config(public_http).is_err());
    }

    #[test]
    fn base_url_appends_only_the_closed_format_endpoint() {
        let openai = validate_model_request(model_request(
            "openai_chat_completions",
            "base_url",
            "https://api.example.test/gateway/v1".to_owned(),
            "",
            false,
            false,
        ))
        .unwrap();
        assert_eq!(openai.endpoint.path(), "/gateway/v1/chat/completions");

        let anthropic = validate_model_request(model_request(
            "anthropic_messages",
            "base_url",
            "https://api.example.test/gateway".to_owned(),
            "",
            false,
            false,
        ))
        .unwrap();
        assert_eq!(anthropic.endpoint.path(), "/gateway/v1/messages");
    }

    #[test]
    fn address_policy_blocks_metadata_and_mixed_classes() {
        assert_eq!(
            classify_ip("127.0.0.1".parse().unwrap()),
            AddressClass::Loopback
        );
        assert_eq!(
            classify_ip("10.0.0.1".parse().unwrap()),
            AddressClass::Private
        );
        assert_eq!(
            classify_ip("169.254.169.254".parse().unwrap()),
            AddressClass::Blocked
        );
        assert_eq!(
            classify_ip("fd00:ec2::254".parse().unwrap()),
            AddressClass::Blocked
        );
        assert_eq!(
            classify_ip("8.8.8.8".parse().unwrap()),
            AddressClass::Public
        );

        let mixed = [
            SocketAddr::new("8.8.8.8".parse().unwrap(), 443),
            SocketAddr::new("10.0.0.1".parse().unwrap(), 443),
        ];
        assert_eq!(
            approve_resolved_addresses(&mixed),
            Err(TraeReasonCode::DnsAddressClassMixed)
        );
        let rebinding = [
            SocketAddr::new("8.8.8.8".parse().unwrap(), 443),
            SocketAddr::new("169.254.169.254".parse().unwrap(), 443),
        ];
        assert_eq!(
            approve_resolved_addresses(&rebinding),
            Err(TraeReasonCode::DnsAddressRejected)
        );
    }

    #[test]
    fn request_ids_are_active_unique_and_terminal_cleanup_is_not_exhaustible() {
        let state = TraeEndpointProbeState::default();
        let request_id = Uuid::new_v4().hyphenated().to_string();
        let registration = state.register(&request_id).unwrap();
        assert_eq!(state.active_len(), 1);
        assert!(state.cancel(&request_id).unwrap().cancelled);
        assert!(registration.cancellation().is_cancelled());
        assert_eq!(
            state.register(&request_id).unwrap_err().code,
            TraeErrorCode::DuplicateRequestId
        );
        drop(registration);
        assert_eq!(state.active_len(), 0);
        assert!(!state.cancel(&request_id).unwrap().cancelled);

        // No completed-request history grows without bound. A request ID is
        // rejected only while its cancellation handle is active.
        drop(state.register(&request_id).unwrap());
        for _ in 0..300 {
            let sequential_id = Uuid::new_v4().hyphenated().to_string();
            drop(state.register(&sequential_id).unwrap());
            assert_eq!(state.active_len(), 0);
        }

        assert_eq!(
            state
                .register(&request_id.to_ascii_uppercase())
                .unwrap_err()
                .code,
            TraeErrorCode::InvalidRequestId
        );
    }

    struct FixedResolver(Vec<SocketAddr>);

    impl TraeDnsResolver for FixedResolver {
        fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> ResolveFuture<'a> {
            let addresses = self.0.clone();
            Box::pin(async move { Ok(addresses) })
        }
    }

    struct PendingResolver;

    impl TraeDnsResolver for PendingResolver {
        fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> ResolveFuture<'a> {
            Box::pin(future::pending())
        }
    }

    fn spawn_http_fixture(
        response: Vec<u8>,
    ) -> (u16, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (request_tx, request_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            let mut expected_length = None;
            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if expected_length.is_none() {
                    if let Some(headers_end) = find_bytes(&request, b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..headers_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length: ")
                                    .or_else(|| line.strip_prefix("Content-Length: "))
                            })
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        expected_length = Some(headers_end + 4 + content_length);
                    }
                }
                if expected_length.is_some_and(|length| request.len() >= length) {
                    break;
                }
            }
            request_tx
                .send(String::from_utf8_lossy(&request).into_owned())
                .unwrap();
            stream.write_all(&response).unwrap();
            stream.flush().unwrap();
        });
        (port, request_rx, handle)
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    #[tokio::test]
    async fn pinned_probe_preserves_host_and_sends_anthropic_secret_once() {
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec();
        let (port, request_rx, handle) = spawn_http_fixture(response);
        let request = validate_model_request(model_request(
            "anthropic_messages",
            "complete_url",
            format!("http://pinned.fixture.invalid:{port}/v1/messages"),
            SECRET_SENTINEL,
            true,
            false,
        ))
        .unwrap();
        let request_id = Uuid::new_v4();
        let result = probe_with_dependencies(
            request_id,
            request,
            Arc::new(ProbeCancellation::default()),
            &FixedResolver(vec![SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port)]),
            TraeProbeProxyPolicy::Direct,
            ProbeLimits::default(),
        )
        .await;
        assert_eq!(result.state, TraeEndpointProbeTerminalState::Reachable);
        assert_eq!(result.reason_code, TraeReasonCode::EndpointReachable);
        assert_eq!(result.status_class, Some(TraeHttpStatusClass::Success));
        let request = request_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        handle.join().unwrap();
        let request_lower = request.to_ascii_lowercase();
        assert!(request_lower.contains(&format!("host: pinned.fixture.invalid:{port}")));
        assert_eq!(request_lower.matches("x-api-key:").count(), 1);
        assert_eq!(request.matches(SECRET_SENTINEL).count(), 1);
        let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
        assert!(!body.contains(SECRET_SENTINEL));
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains(SECRET_SENTINEL));
    }

    #[tokio::test]
    async fn proxy_policy_fails_closed_before_dns_and_never_falls_back_direct() {
        let request = validate_model_request(model_request(
            "openai_chat_completions",
            "complete_url",
            "https://never-resolved.invalid/v1/chat/completions".to_owned(),
            "",
            false,
            false,
        ))
        .unwrap();
        let result = probe_with_dependencies(
            Uuid::new_v4(),
            request,
            Arc::new(ProbeCancellation::default()),
            &PendingResolver,
            TraeProbeProxyPolicy::Unsupported,
            ProbeLimits::default(),
        )
        .await;
        assert_eq!(
            result.state,
            TraeEndpointProbeTerminalState::NetworkRejected
        );
        assert_eq!(result.reason_code, TraeReasonCode::ProxyDnsPinUnsupported);
    }

    #[tokio::test]
    async fn proxied_probe_uses_shared_proxy_client_and_skips_dns_pin() {
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec();
        let (port, request_rx, handle) = spawn_http_fixture(response);
        let request = validate_model_request(model_request(
            "openai_chat_completions",
            "complete_url",
            format!("http://127.0.0.1:{port}/v1/chat/completions"),
            SECRET_SENTINEL,
            true,
            false,
        ))
        .unwrap();
        let result = probe_with_dependencies(
            Uuid::new_v4(),
            request,
            Arc::new(ProbeCancellation::default()),
            &PendingResolver,
            TraeProbeProxyPolicy::Proxied,
            ProbeLimits::default(),
        )
        .await;
        assert_eq!(result.state, TraeEndpointProbeTerminalState::Reachable);
        assert_eq!(result.reason_code, TraeReasonCode::EndpointReachable);
        let request = request_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        handle.join().unwrap();
        assert!(request.to_ascii_lowercase().contains("host: 127.0.0.1"));
        assert_eq!(request.matches(SECRET_SENTINEL).count(), 1);
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains(SECRET_SENTINEL));
    }

    #[tokio::test]
    async fn pending_dns_is_independently_cancellable_and_bounded_by_deadline() {
        let request = validate_model_request(model_request(
            "openai_chat_completions",
            "complete_url",
            "https://pending.invalid/v1/chat/completions".to_owned(),
            "",
            false,
            false,
        ))
        .unwrap();
        let cancellation = Arc::new(ProbeCancellation::default());
        let cancellation_for_task = Arc::clone(&cancellation);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation_for_task.cancel();
        });
        let cancelled = probe_with_dependencies(
            Uuid::new_v4(),
            request,
            cancellation,
            &PendingResolver,
            TraeProbeProxyPolicy::Direct,
            ProbeLimits {
                total_timeout: Duration::from_secs(1),
                ..ProbeLimits::default()
            },
        )
        .await;
        assert_eq!(cancelled.state, TraeEndpointProbeTerminalState::Cancelled);

        let request = validate_model_request(model_request(
            "openai_chat_completions",
            "complete_url",
            "https://pending.invalid/v1/chat/completions".to_owned(),
            "",
            false,
            false,
        ))
        .unwrap();
        let timed_out = probe_with_dependencies(
            Uuid::new_v4(),
            request,
            Arc::new(ProbeCancellation::default()),
            &PendingResolver,
            TraeProbeProxyPolicy::Direct,
            ProbeLimits {
                total_timeout: Duration::from_millis(20),
                ..ProbeLimits::default()
            },
        )
        .await;
        assert_eq!(timed_out.state, TraeEndpointProbeTerminalState::Timeout);
    }

    struct FixedExecutableResolver(bool);

    impl ExecutableResolver for FixedExecutableResolver {
        fn executable_exists(&self, _command: &str) -> bool {
            self.0
        }
    }

    #[test]
    fn mcp_validation_is_exact_no_execute_and_redacts_secret_values() {
        let config = json!({
            "mcpServers": {
                "stdio-server": {
                    "command": "trusted-mcp",
                    "args": ["--mode", "safe"],
                    "env": {"MCP_TOKEN": SECRET_SENTINEL}
                },
                "http-server": {
                    "url": "https://mcp.example.test/rpc",
                    "headers": {"Authorization": SECRET_SENTINEL}
                }
            }
        });
        let result = validate_external_mcp_config_with(
            ExternalMcpAgentId::TraeWork,
            config,
            &FixedExecutableResolver(true),
        )
        .unwrap();
        assert!(result.valid);
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.findings[0].transport, TraeMcpTransport::Stdio);
        assert_eq!(result.findings[0].executable_available, Some(true));
        assert!(result.findings[0].has_secrets);
        assert_eq!(result.findings[1].transport, TraeMcpTransport::Http);
        assert_eq!(result.findings[1].executable_available, None);
        assert!(result.findings[1].has_secrets);

        let wire = serde_json::to_string(&result).unwrap();
        assert!(!wire.contains(SECRET_SENTINEL));
        assert!(wire.contains(REDACTED_TEMPLATE_VALUE));
        assert!(wire.contains("TRAE_MCP_SERVER_VALID"));
    }

    #[test]
    fn stdio_explicit_directory_is_not_reported_as_an_executable() {
        let directory = tempfile::tempdir().unwrap();
        assert!(!SystemExecutableResolver.executable_exists(directory.path().to_str().unwrap()));
    }

    #[test]
    fn mcp_validation_reports_closed_reasons_without_secret_values() {
        let config = json!({
            "mcpServers": {
                "missing": {
                    "command": "missing-mcp",
                    "args": "not-an-array",
                    "env": {"TOKEN": SECRET_SENTINEL},
                    "extension": true
                },
                "unsafe-http": {
                    "url": "https://127.0.0.1/rpc",
                    "headers": {"Authorization": SECRET_SENTINEL}
                }
            }
        });
        let result = validate_external_mcp_config_with(
            ExternalMcpAgentId::QoderWork,
            config,
            &FixedExecutableResolver(false),
        )
        .unwrap();
        assert!(!result.valid);
        assert!(result.findings[0]
            .reason_codes
            .contains(&TraeMcpReasonCode::UnknownField));
        assert!(result.findings[0]
            .reason_codes
            .contains(&TraeMcpReasonCode::InvalidArgs));
        assert!(result.findings[0]
            .reason_codes
            .contains(&TraeMcpReasonCode::CommandNotFound));
        assert_eq!(
            result.findings[1].reason_codes,
            vec![TraeMcpReasonCode::UnsafeAddress]
        );
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains(SECRET_SENTINEL));
    }

    #[test]
    fn mcp_validation_rejects_mixed_transport_prototype_controls_and_bounds() {
        let mixed = json!({
            "mcpServers": {
                "mixed": {"command": "mcp", "url": "https://mcp.example.test"}
            }
        });
        assert_eq!(
            validate_external_mcp_config_with(
                ExternalMcpAgentId::TraeWork,
                mixed,
                &FixedExecutableResolver(true)
            )
            .unwrap_err()
            .code,
            TraeErrorCode::McpInvalidTransport
        );

        let prototype = json!({"mcpServers": {"__proto__": {"command": "mcp"}}});
        assert_eq!(
            validate_external_mcp_config_with(
                ExternalMcpAgentId::TraeWork,
                prototype,
                &FixedExecutableResolver(true)
            )
            .unwrap_err()
            .code,
            TraeErrorCode::McpInvalidServer
        );

        let control = json!({
            "mcpServers": {"server": {"command": "mcp\nserver"}}
        });
        let result = validate_external_mcp_config_with(
            ExternalMcpAgentId::TraeWork,
            control,
            &FixedExecutableResolver(true),
        )
        .unwrap();
        assert!(result.findings[0]
            .reason_codes
            .contains(&TraeMcpReasonCode::ControlCharacter));

        let oversized = json!({
            "mcpServers": {"server": {"command": "mcp", "args": ["x".repeat(MAX_MCP_CONFIG_BYTES)]}}
        });
        assert_eq!(
            validate_external_mcp_config_with(
                ExternalMcpAgentId::TraeWork,
                oversized,
                &FixedExecutableResolver(true)
            )
            .unwrap_err()
            .code,
            TraeErrorCode::McpConfigTooLarge
        );
    }

    #[test]
    fn external_mcp_agent_id_and_error_dto_are_closed() {
        assert!(serde_json::from_str::<ExternalMcpAgentId>("\"qoderwork\"").is_ok());
        assert!(serde_json::from_str::<ExternalMcpAgentId>("\"trae-work\"").is_ok());
        assert!(serde_json::from_str::<ExternalMcpAgentId>("\"codex\"").is_err());
        assert_eq!(
            serde_json::to_value(TraeErrorDto::new(TraeErrorCode::InvalidUrl)).unwrap(),
            json!({"code": "TRAE_INVALID_URL"})
        );
    }
}
