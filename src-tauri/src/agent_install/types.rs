//! Closed Agent install/action DTOs. Renderer input is limited to canonical
//! catalog IDs and opaque backend-generated snapshot/target capabilities.

use serde::{Deserialize, Serialize};

use crate::services::external_agents::AgentCatalogId;

pub const AGENT_INSTALL_READINESS_CONTRACT_VERSION: u16 = 4;
pub const AGENT_INSTALL_READINESS_REVIEWED_AT: &str = "2026-08-31";
pub const AGENT_INSTALLATION_INVENTORY_CONTRACT_VERSION: u16 = 1;
pub const AGENT_ACTION_CONTRACT_VERSION: u16 = 4;
pub const AGENT_AUTH_CONTRACT_VERSION: u16 = 1;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSurface {
    Cli,
    Desktop,
}

pub fn legal_surfaces(agent_id: AgentCatalogId) -> &'static [AgentSurface] {
    super::lifecycle_policy::legal_surfaces(agent_id)
}

pub fn default_surface(agent_id: AgentCatalogId) -> AgentSurface {
    super::lifecycle_policy::default_surface(agent_id)
}

pub fn surface_is_legal(agent_id: AgentCatalogId, surface: AgentSurface) -> bool {
    legal_surfaces(agent_id).contains(&surface)
}

pub fn resolve_requested_surface(
    agent_id: AgentCatalogId,
    surface: Option<AgentSurface>,
) -> Result<AgentSurface, AgentReasonCode> {
    let surface = surface.unwrap_or_else(|| default_surface(agent_id));
    if surface_is_legal(agent_id, surface) {
        Ok(surface)
    } else {
        Err(AgentReasonCode::SurfaceNotSupported)
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthIntent {
    Login,
    Logout,
    ConnectProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthAuthority {
    Verified,
    Unverified,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthAccountState {
    LoggedIn,
    LoggedOut,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthProviderConnectionState {
    Configured,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthManagedDestination {
    AuthCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthReasonCode {
    AuthStateUnknown,
    AuthObserverUnavailable,
    AuthOutputInvalid,
    InteractiveUserUnavailable,
    OperationConflict,
    ProviderSelectionRequired,
    ProviderChanged,
    MonitoringStopped,
    TimedOut,
    HandoffOnly,
    ManagedByAuthCenter,
    TargetSelectionRequired,
    TargetChanged,
    TargetNotExecutable,
    InventoryExpired,
    CommandFailed,
    Cancelled,
    ExecutorNotImplemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthSessionStage {
    Preparing,
    Launching,
    AwaitingUser,
    Verifying,
    Verified,
    HandoffComplete,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthSessionOutcome {
    VerifiedLoggedIn,
    VerifiedLoggedOut,
    VerifiedProviderChange,
    HandoffOnly,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAuthProviderSummaryDto {
    pub provider_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AgentAuthObservationDto {
    Account {
        contract_version: u16,
        agent_id: AgentCatalogId,
        ownership: AgentAuthOwnership,
        authority: AgentAuthAuthority,
        state: AgentAuthAccountState,
        allowed_intents: Vec<AgentAuthIntent>,
        checked_at: String,
        reason_codes: Vec<AgentAuthReasonCode>,
    },
    ProviderConnections {
        contract_version: u16,
        agent_id: AgentCatalogId,
        ownership: AgentAuthOwnership,
        authority: AgentAuthAuthority,
        state: AgentAuthProviderConnectionState,
        providers: Vec<AgentAuthProviderSummaryDto>,
        allowed_intents: Vec<AgentAuthIntent>,
        checked_at: String,
        reason_codes: Vec<AgentAuthReasonCode>,
    },
    HandoffOnly {
        contract_version: u16,
        agent_id: AgentCatalogId,
        ownership: AgentAuthOwnership,
        authority: AgentAuthAuthority,
        allowed_intents: Vec<AgentAuthIntent>,
        checked_at: String,
        reason_codes: Vec<AgentAuthReasonCode>,
    },
    FyagentManaged {
        contract_version: u16,
        agent_id: AgentCatalogId,
        ownership: AgentAuthOwnership,
        authority: AgentAuthAuthority,
        destination: AgentAuthManagedDestination,
        allowed_intents: Vec<AgentAuthIntent>,
        checked_at: String,
        reason_codes: Vec<AgentAuthReasonCode>,
    },
    Unavailable {
        contract_version: u16,
        agent_id: AgentCatalogId,
        ownership: AgentAuthOwnership,
        authority: AgentAuthAuthority,
        allowed_intents: Vec<AgentAuthIntent>,
        checked_at: String,
        reason_codes: Vec<AgentAuthReasonCode>,
    },
}

impl AgentAuthObservationDto {
    pub fn agent_id(&self) -> AgentCatalogId {
        match self {
            Self::Account { agent_id, .. }
            | Self::ProviderConnections { agent_id, .. }
            | Self::HandoffOnly { agent_id, .. }
            | Self::FyagentManaged { agent_id, .. }
            | Self::Unavailable { agent_id, .. } => *agent_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAgentAuthSessionRequest {
    pub agent_id: AgentCatalogId,
    pub intent: AgentAuthIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_target_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAuthSessionSnapshot {
    pub contract_version: u16,
    pub session_id: String,
    pub agent_id: AgentCatalogId,
    pub intent: AgentAuthIntent,
    pub stage: AgentAuthSessionStage,
    pub can_stop_waiting: bool,
    pub outcome: Option<AgentAuthSessionOutcome>,
    pub observation: AgentAuthObservationDto,
    pub reason_code: Option<AgentAuthReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAuthErrorDto {
    pub reason_code: AgentAuthReasonCode,
}

impl AgentAuthErrorDto {
    pub fn new(reason_code: AgentAuthReasonCode) -> Self {
        Self { reason_code }
    }
}

impl From<AgentAuthReasonCode> for AgentAuthErrorDto {
    fn from(reason_code: AgentAuthReasonCode) -> Self {
        Self::new(reason_code)
    }
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
    InstallerArtifactUnavailable,
    InstallationVerificationFailed,
    InstallerUserCancelled,
    InstallerProcessUnobservable,
    InstallerTimedOut,
    InstallerExitedNonzero,
    RollbackRestored,
    RecoveryRequired,
    ExecutorNotImplemented,
    SurfaceNotSupported,
    ActionNotSupported,
    ApplicationLaunchFailed,
    HelperNotPackaged,
    HelperSignatureInvalid,
    HelperInstallAuthorizationCancelled,
    HelperInstallFailed,
    HelperUpdateRequired,
    HelperDowngradeRejected,
    HelperProtocolIncompatible,
    HelperPeerRejected,
    OperationAuthorizationCancelled,
    OperationAuthorizationInvalid,
    SourceCapabilityInvalid,
    SourceChanged,
    TargetSlotInvalid,
    HelperRemovalFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionJobStage {
    Checking,
    Downloading,
    Staging,
    LaunchingInstaller,
    AwaitingUser,
    Installing,
    VerifyingInstallation,
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSurfaceReadinessDto {
    pub surface: AgentSurface,
    pub install_state: AgentInstallState,
    pub inventory_state: InstallationInventoryState,
    pub requires_target_selection: bool,
    pub update_state: AgentUpdateState,
    pub release_id: Option<String>,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub source_kind: AgentSourceKind,
    pub allowed_actions: Vec<AgentActionId>,
    pub reason_codes: Vec<AgentReasonCode>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<AgentSurfaceReadinessDto>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<AgentSurface>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<AgentSurface>,
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
    pub surface: AgentSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionTransferPhase {
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentActionTransferSample {
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub attempt: u8,
    pub max_attempts: u8,
}

impl AgentActionTransferSample {
    pub fn from_progress_bytes(
        completed_bytes: u64,
        total_bytes: u64,
        attempt: u8,
        max_attempts: u8,
    ) -> Self {
        let total_bytes =
            (total_bytes > 0 && total_bytes >= completed_bytes).then_some(total_bytes);
        Self {
            completed_bytes,
            total_bytes,
            attempt,
            max_attempts,
        }
    }

    pub(crate) fn is_well_formed(self) -> bool {
        self.attempt >= 1 && self.max_attempts >= self.attempt
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActionTransferSnapshot {
    pub phase: AgentActionTransferPhase,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub attempt: u8,
    pub max_attempts: u8,
    pub sequence: u64,
    pub observed_at: String,
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
    pub transfer: Option<AgentActionTransferSnapshot>,
    pub surface: AgentSurface,
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

pub fn validate_opaque_auth_provider_id(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("p1:")
        && value[3..]
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
}

pub fn validate_auth_session_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .map(|parsed| parsed.hyphenated().to_string() == value)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn account_observation() -> AgentAuthObservationDto {
        AgentAuthObservationDto::Account {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            agent_id: AgentCatalogId::ClaudeCode,
            ownership: AgentAuthOwnership::AgentOwned,
            authority: AgentAuthAuthority::Verified,
            state: AgentAuthAccountState::LoggedOut,
            allowed_intents: vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
            checked_at: "2026-08-30T00:00:00Z".into(),
            reason_codes: Vec::new(),
        }
    }

    #[test]
    fn auth_request_denies_unknown_or_locator_fields() {
        let request = json!({
            "agentId": "claude-code",
            "intent": "login"
        });
        assert!(serde_json::from_value::<StartAgentAuthSessionRequest>(request).is_ok());

        for (field, value) in [
            ("url", json!("https://example.invalid")),
            ("path", json!("/tmp/claude")),
            ("command", json!("claude auth login")),
            ("token", json!("secret")),
            ("bypass", json!(true)),
        ] {
            let mut request = json!({
                "agentId": "claude-code",
                "intent": "login"
            });
            request
                .as_object_mut()
                .expect("request is an object")
                .insert(field.into(), value);
            assert!(
                serde_json::from_value::<StartAgentAuthSessionRequest>(request).is_err(),
                "unexpectedly accepted {field}"
            );
        }
    }

    #[test]
    fn auth_observation_and_session_use_exact_bounded_wire_keys() {
        let observation = serde_json::to_value(account_observation()).unwrap();
        assert_eq!(
            sorted_keys(&observation),
            [
                "agentId",
                "allowedIntents",
                "authority",
                "checkedAt",
                "contractVersion",
                "kind",
                "ownership",
                "reasonCodes",
                "state",
            ]
        );
        let session = AgentAuthSessionSnapshot {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            session_id: "123e4567-e89b-12d3-a456-426614174000".into(),
            agent_id: AgentCatalogId::ClaudeCode,
            intent: AgentAuthIntent::Login,
            stage: AgentAuthSessionStage::Verified,
            can_stop_waiting: false,
            outcome: Some(AgentAuthSessionOutcome::VerifiedLoggedIn),
            observation: account_observation(),
            reason_code: None,
        };
        let session = serde_json::to_value(session).unwrap();
        assert_eq!(
            sorted_keys(&session),
            [
                "agentId",
                "canStopWaiting",
                "contractVersion",
                "intent",
                "observation",
                "outcome",
                "reasonCode",
                "sessionId",
                "stage",
            ]
        );
        let wire = session.to_string();
        for forbidden in [
            "http://",
            "https://",
            "token",
            "secret",
            "executablePath",
            "command",
            "signer",
        ] {
            assert!(!wire.contains(forbidden), "wire leaked {forbidden}");
        }
    }

    #[test]
    fn auth_capability_identifiers_are_canonical() {
        assert!(validate_opaque_auth_provider_id(&format!(
            "p1:{}",
            "a".repeat(32)
        )));
        assert!(!validate_opaque_auth_provider_id(&format!(
            "p1:{}",
            "A".repeat(32)
        )));
        assert!(validate_auth_session_id(
            "123e4567-e89b-12d3-a456-426614174000"
        ));
        assert!(!validate_auth_session_id(
            "123E4567-E89B-12D3-A456-426614174000"
        ));
    }

    #[test]
    fn action_job_snapshot_carries_closed_transfer_telemetry() {
        let snapshot = AgentActionJobSnapshot {
            contract_version: AGENT_ACTION_CONTRACT_VERSION,
            job_id: "123e4567-e89b-12d3-a456-426614174000".into(),
            agent_id: AgentCatalogId::QoderWork,
            action: AgentActionId::Install,
            stage: AgentActionJobStage::Downloading,
            cancellable: true,
            reason_code: None,
            transfer: Some(AgentActionTransferSnapshot {
                phase: AgentActionTransferPhase::Download,
                completed_bytes: 1_048_576,
                total_bytes: Some(2_097_152),
                attempt: 1,
                max_attempts: 1,
                sequence: 2,
                observed_at: "2026-08-31T00:00:00Z".into(),
            }),
            surface: AgentSurface::Desktop,
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(
            sorted_keys(&value),
            [
                "action",
                "agentId",
                "cancellable",
                "contractVersion",
                "jobId",
                "reasonCode",
                "stage",
                "surface",
                "transfer",
            ]
        );
        let transfer = value.get("transfer").and_then(Value::as_object).unwrap();
        assert_eq!(
            sorted_keys(&Value::Object(transfer.clone())),
            [
                "attempt",
                "completedBytes",
                "maxAttempts",
                "observedAt",
                "phase",
                "sequence",
                "totalBytes",
            ]
        );
        let wire = value.to_string();
        for forbidden in [
            "http://",
            "https://",
            "path",
            "url",
            "bytesPerSecond",
            "speed",
        ] {
            assert!(!wire.contains(forbidden), "wire leaked {forbidden}");
        }
        assert_eq!(
            AgentActionTransferSample::from_progress_bytes(8, 0, 1, 1).total_bytes,
            None
        );
        assert_eq!(
            AgentActionTransferSample::from_progress_bytes(12, 8, 1, 1).total_bytes,
            None
        );
    }

    #[test]
    fn surface_contract_rejects_unknown_values_and_illegal_product_pairs() {
        assert!(serde_json::from_value::<AgentSurface>(json!("cli")).is_ok());
        assert!(serde_json::from_value::<AgentSurface>(json!("desktop")).is_ok());
        assert!(serde_json::from_value::<AgentSurface>(json!("web")).is_err());
        assert_eq!(
            legal_surfaces(AgentCatalogId::OpenCode),
            &[AgentSurface::Desktop]
        );
        assert_eq!(
            default_surface(AgentCatalogId::OpenCode),
            AgentSurface::Desktop
        );
        assert_eq!(
            default_surface(AgentCatalogId::QoderWork),
            AgentSurface::Desktop
        );
        assert_eq!(
            default_surface(AgentCatalogId::ClaudeCode),
            AgentSurface::Desktop
        );
        assert!(!surface_is_legal(
            AgentCatalogId::QoderWork,
            AgentSurface::Cli
        ));
        assert!(!surface_is_legal(
            AgentCatalogId::ClaudeCode,
            AgentSurface::Cli
        ));
        assert!(surface_is_legal(
            AgentCatalogId::ClaudeCode,
            AgentSurface::Desktop
        ));
        assert_eq!(
            resolve_requested_surface(AgentCatalogId::OpenCode, None),
            Ok(AgentSurface::Desktop)
        );
        assert_eq!(
            resolve_requested_surface(AgentCatalogId::ClaudeCode, Some(AgentSurface::Cli)),
            Err(AgentReasonCode::SurfaceNotSupported)
        );
        assert_eq!(
            resolve_requested_surface(AgentCatalogId::QoderWork, Some(AgentSurface::Cli)),
            Err(AgentReasonCode::SurfaceNotSupported)
        );
        assert!(serde_json::from_value::<StartAgentActionRequest>(json!({
            "agentId": "opencode",
            "action": "install",
            "surface": "desktop"
        }))
        .is_ok());
        assert!(serde_json::from_value::<StartAgentActionRequest>(json!({
            "agentId": "opencode",
            "action": "install",
            "surface": "web"
        }))
        .is_err());
        assert!(serde_json::from_value::<StartAgentActionRequest>(json!({
            "agentId": "qoderwork",
            "action": "launch",
            "bundleId": "ai.opencode.desktop"
        }))
        .is_err());
    }

    #[test]
    fn helper_reason_codes_round_trip_as_snake_case_wire_values() {
        let cases = [
            (AgentReasonCode::HelperNotPackaged, "helper_not_packaged"),
            (
                AgentReasonCode::HelperSignatureInvalid,
                "helper_signature_invalid",
            ),
            (
                AgentReasonCode::HelperInstallAuthorizationCancelled,
                "helper_install_authorization_cancelled",
            ),
            (
                AgentReasonCode::HelperInstallFailed,
                "helper_install_failed",
            ),
            (
                AgentReasonCode::HelperUpdateRequired,
                "helper_update_required",
            ),
            (
                AgentReasonCode::HelperDowngradeRejected,
                "helper_downgrade_rejected",
            ),
            (
                AgentReasonCode::HelperProtocolIncompatible,
                "helper_protocol_incompatible",
            ),
            (AgentReasonCode::HelperPeerRejected, "helper_peer_rejected"),
            (
                AgentReasonCode::OperationAuthorizationCancelled,
                "operation_authorization_cancelled",
            ),
            (
                AgentReasonCode::OperationAuthorizationInvalid,
                "operation_authorization_invalid",
            ),
            (
                AgentReasonCode::SourceCapabilityInvalid,
                "source_capability_invalid",
            ),
            (AgentReasonCode::SourceChanged, "source_changed"),
            (AgentReasonCode::TargetSlotInvalid, "target_slot_invalid"),
            (
                AgentReasonCode::HelperRemovalFailed,
                "helper_removal_failed",
            ),
        ];
        for (code, expected) in cases {
            assert_eq!(serde_json::to_value(code).unwrap(), json!(expected));
            assert_eq!(
                serde_json::from_value::<AgentReasonCode>(json!(expected)).unwrap(),
                code
            );
        }
        assert_eq!(
            serde_json::to_value(AgentReasonCode::ActionNotSupported).unwrap(),
            json!("action_not_supported")
        );
        assert_eq!(
            serde_json::from_value::<AgentReasonCode>(json!("action_not_supported")).unwrap(),
            AgentReasonCode::ActionNotSupported
        );
        assert!(serde_json::from_value::<AgentReasonCode>(json!("smjobbless")).is_err());
        assert_eq!(AGENT_ACTION_CONTRACT_VERSION, 4);
        assert_eq!(AGENT_INSTALL_READINESS_CONTRACT_VERSION, 4);
        assert_eq!(AGENT_INSTALL_READINESS_REVIEWED_AT, "2026-08-31");
    }

    fn sorted_keys(value: &Value) -> Vec<&str> {
        let mut keys = value
            .as_object()
            .expect("wire value is an object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }
}
