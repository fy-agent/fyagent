//! Closed Agent install/action DTOs. Renderer input is limited to canonical
//! catalog IDs and opaque backend-generated snapshot/target capabilities.

use serde::{Deserialize, Serialize};

use crate::services::external_agents::AgentCatalogId;

pub const AGENT_INSTALL_READINESS_CONTRACT_VERSION: u16 = 3;
pub const AGENT_INSTALL_READINESS_REVIEWED_AT: &str = "2026-08-29";
pub const AGENT_INSTALLATION_INVENTORY_CONTRACT_VERSION: u16 = 1;
pub const AGENT_ACTION_CONTRACT_VERSION: u16 = 2;

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
pub enum InstallationInventoryState {
    NotObserved,
    Single,
    Multiple,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationScope {
    CurrentUser,
    AllUsers,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationOwner {
    VendorInstaller,
    PackageManager,
    Fyagent,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationPackageKind {
    AppBundle,
    Exe,
    Msi,
    Msix,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationEvidenceCode {
    BundleIdentity,
    FileIdentity,
    KnownPath,
    PathLookup,
    AppPathsRegistration,
    UninstallRegistration,
    MsixPackage,
    CodexDesktopAdapter,
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
    TargetSelectionRequired,
    TargetChanged,
    TargetNotExecutable,
    TargetScopeUnsupported,
    InventoryExpired,
    CandidateConflict,
    AuthorizationRequired,
    PermissionDenied,
    ApplicationRunning,
    InstallationVerificationFailed,
    RollbackRestored,
    RecoveryRequired,
    ExecutorNotImplemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionJobStage {
    Checking,
    Downloading,
    Staging,
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
    pub inventory_state: InstallationInventoryState,
    pub requires_target_selection: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationCandidateDto {
    pub candidate_id: String,
    pub candidate_revision: String,
    pub agent_id: AgentCatalogId,
    pub scope: InstallationScope,
    pub owner: InstallationOwner,
    pub package_kind: InstallationPackageKind,
    pub local_version: Option<String>,
    pub launch_eligible: bool,
    pub install_eligible: bool,
    pub update_eligible: bool,
    pub reason_codes: Vec<AgentReasonCode>,
    pub evidence_codes: Vec<InstallationEvidenceCode>,
    pub location_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshInstallDestinationDto {
    pub destination_id: String,
    pub destination_revision: String,
    pub scope: InstallationScope,
    pub owner: InstallationOwner,
    pub package_kind: InstallationPackageKind,
    pub requires_elevation: bool,
    pub writable: bool,
    pub eligible: bool,
    pub reason_codes: Vec<AgentReasonCode>,
    pub location_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallationInventoryDto {
    pub contract_version: u16,
    pub inventory_id: String,
    pub agent_id: AgentCatalogId,
    pub state: InstallationInventoryState,
    pub candidates: Vec<InstallationCandidateDto>,
    pub fresh_destinations: Vec<FreshInstallDestinationDto>,
    pub reason_codes: Vec<AgentReasonCode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAgentActionRequest {
    pub agent_id: AgentCatalogId,
    pub action: AgentActionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_release_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_target_revision: Option<String>,
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

pub fn validate_opaque_inventory_id(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("i1:")
        && value[3..]
            .bytes()
            .all(|character| character.is_ascii_hexdigit())
}

pub fn validate_opaque_target_id(value: &str) -> bool {
    value.len() == 35
        && (value.starts_with("c1:") || value.starts_with("d1:"))
        && value[3..]
            .bytes()
            .all(|character| character.is_ascii_hexdigit())
}

pub fn validate_opaque_target_revision(value: &str) -> bool {
    value.len() == 67
        && value.starts_with("r1:")
        && value[3..]
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
}
