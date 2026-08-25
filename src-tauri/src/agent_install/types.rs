//! Closed Agent install/action DTOs. Renderer input is only a catalog ID,
//! a closed action, and an optional opaque backend-generated release ID.

use serde::{Deserialize, Serialize};

use crate::services::external_agents::AgentCatalogId;

pub const AGENT_INSTALL_READINESS_CONTRACT_VERSION: u16 = 2;
pub const AGENT_INSTALL_READINESS_REVIEWED_AT: &str = "2026-08-25";
pub const AGENT_ACTION_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionId {
    Install,
    Update,
    Launch,
    AuthLogin,
    AuthLogout,
    AuthConnectProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInstallState {
    NotInstalled,
    Installed,
    InstalledNotRunnable,
    Unknown,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentUpdateState {
    Unavailable,
    Unknown,
    UpToDate,
    UpdateAvailable,
    LatestUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthOwnership {
    FyagentManaged,
    AgentOwned,
    ProviderOwned,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthState {
    Unknown,
    LoggedIn,
    LoggedOut,
    ProviderConnectionRequired,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSourceKind {
    CliTooling,
    ManagedDesktop,
    CodexDesktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReasonCode {
    OfficialPageOnly,
    SourceNotVerified,
    PlatformUnsupported,
    InteractiveUserUnavailable,
    InstalledNotRunnable,
    AuthStateUnknown,
    ProviderConnectionRequired,
    CredentialStoreUnsupported,
    BindingAccountMissing,
    BindingIdentityMismatch,
    OperationConflict,
    Cancelled,
    ManagedByCodexDesktop,
    NativeProjectionUnavailable,
    RefreshRequired,
    ExecutorNotImplemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionJobStage {
    Checking,
    Downloading,
    Installing,
    VerifyingInstallation,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallReadinessDto {
    pub contract_version: u16,
    pub agent_id: AgentCatalogId,
    pub reviewed_at: &'static str,
    pub install_state: AgentInstallState,
    pub update_state: AgentUpdateState,
    pub release_id: Option<String>,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub auth_ownership: AgentAuthOwnership,
    pub auth_state: AgentAuthState,
    pub source_kind: AgentSourceKind,
    pub allowed_actions: Vec<AgentActionId>,
    pub reason_codes: Vec<AgentReasonCode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAgentActionRequest {
    pub agent_id: AgentCatalogId,
    pub action: AgentActionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_release_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActionResult {
    pub contract_version: u16,
    pub agent_id: AgentCatalogId,
    pub action: AgentActionId,
    pub job_id: Option<String>,
    pub stage: AgentActionJobStage,
    pub reason_code: Option<AgentReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActionJobSnapshot {
    pub contract_version: u16,
    pub job_id: String,
    pub agent_id: AgentCatalogId,
    pub action: AgentActionId,
    pub stage: AgentActionJobStage,
    pub cancellable: bool,
    pub reason_code: Option<AgentReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActionErrorDto {
    pub reason_code: AgentReasonCode,
}

impl AgentActionErrorDto {
    pub fn new(reason_code: AgentReasonCode) -> Self {
        Self { reason_code }
    }
}

impl From<AgentReasonCode> for AgentActionErrorDto {
    fn from(reason_code: AgentReasonCode) -> Self {
        Self::new(reason_code)
    }
}

pub fn validate_opaque_release_id(value: &str) -> bool {
    value.len() == 67
        && value.starts_with("v1:")
        && value[3..]
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
}
