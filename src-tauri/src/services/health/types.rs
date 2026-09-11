//! Closed, metadata-only health wire. No native paths or error strings cross IPC.
use crate::services::external_agents::AgentCatalogId;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckId {
    Installation,
    Helper,
    Conflicts,
    Configuration,
    Secret,
    Endpoint,
    Auth,
    Model,
    Drift,
    Proxy,
    Restart,
    LastRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckState {
    Ok,
    Attention,
    Blocked,
    NotConfigured,
    Unknown,
    NotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthAction {
    Installation,
    Configuration,
    Authentication,
    ModelTest,
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthReasonCode {
    InstallationFound,
    InstallationNotFound,
    InstallationUnknown,
    InstallationNotRunnable,
    MultipleInstallations,
    InstallationConflict,
    SingleInstallation,
    HelperAvailable,
    HelperUnavailable,
    HelperNotRequired,
    ConfigurationPresent,
    ConfigurationMissing,
    ConfigurationUnreadable,
    CredentialAvailable,
    CredentialMissing,
    CredentialUnknown,
    CredentialNotRequired,
    EndpointConfigured,
    EndpointMissing,
    EndpointInvalid,
    EndpointNotChecked,
    AuthLoggedIn,
    AuthLoggedOut,
    AuthUnknown,
    AuthManaged,
    AuthHandoff,
    ModelConfigured,
    ModelMissing,
    ModelUnknown,
    ConfigurationInSync,
    ConfigurationDrifted,
    ConfigurationDriftUnknown,
    ProxyRunning,
    ProxyStopped,
    ProxyNotUsed,
    ProxyUnknown,
    RestartRequired,
    RestartNotRequired,
    RestartUnknown,
    RequestSucceeded,
    RequestFailed,
    RequestNotRecorded,
    NotSupported,
    ReadFailed,
    CheckTimeout,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub id: HealthCheckId,
    pub state: HealthCheckState,
    pub severity: HealthSeverity,
    pub reason_code: HealthReasonCode,
    pub checked_at: String,
    pub evidence_at: Option<String>,
    pub value: Option<String>,
    pub action: Option<HealthAction>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealthSnapshot {
    pub contract_version: u8,
    pub agent_id: AgentCatalogId,
    pub checked_at: String,
    pub checks: Vec<HealthCheck>,
}

pub(super) const CHECK_IDS: [HealthCheckId; 12] = [
    HealthCheckId::Installation,
    HealthCheckId::Helper,
    HealthCheckId::Conflicts,
    HealthCheckId::Configuration,
    HealthCheckId::Secret,
    HealthCheckId::Endpoint,
    HealthCheckId::Auth,
    HealthCheckId::Model,
    HealthCheckId::Drift,
    HealthCheckId::Proxy,
    HealthCheckId::Restart,
    HealthCheckId::LastRequest,
];

impl HealthCheck {
    pub(super) fn new(
        id: HealthCheckId,
        state: HealthCheckState,
        reason_code: HealthReasonCode,
        action: Option<HealthAction>,
        checked_at: &str,
    ) -> Self {
        let severity = match state {
            HealthCheckState::Blocked => HealthSeverity::Error,
            HealthCheckState::Attention
            | HealthCheckState::NotConfigured
            | HealthCheckState::Unknown => HealthSeverity::Warning,
            HealthCheckState::Ok | HealthCheckState::NotSupported => HealthSeverity::Info,
        };
        Self {
            id,
            state,
            severity,
            reason_code,
            checked_at: checked_at.into(),
            evidence_at: None,
            value: None,
            action,
        }
    }
}

/// Text is optional decoration, never the evidence for a successful check.
pub(super) fn safe_value(value: &str) -> Option<String> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || value.encode_utf16().count() > 80
        || value.starts_with(['/', '.', '~'])
        || !value
            .chars()
            .all(|c| c.is_alphanumeric() || " ._-/+()".contains(c))
        || [
            "sk-", "xai-", "eyj", "bearer", "token", "secret", "users/", "home/", "private/",
        ]
        .iter()
        .any(|s| lower.contains(s))
    {
        return None;
    }
    Some(value.to_string())
}
