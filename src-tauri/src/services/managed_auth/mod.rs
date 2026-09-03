//! Shared managed-auth transport contract and service boundary.
//!
//! The first delivery slice exposes an explicit unavailable observation so the
//! renderer and native ACL can evolve together. Later slices attach the
//! metadata repository, SecretRef vault, OAuth adapters and consumer adapters
//! behind this module instead of adding a second command surface.

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) mod consumers;
mod core;
mod login;
mod login_sessions;
mod migration;
pub(crate) mod providers;
mod repository;
mod secret_bundle;
mod service;

pub(crate) use core::{
    stable_connection_id, stable_credential_id, stable_identity_id, stable_revision,
    ConnectionRecord, ConnectionStatus, CredentialPurpose, CredentialRecord, CredentialStatus,
    CredentialWithIdentity, IdentityRecord, ManagedAuthCoreError, MigrationRecord, MigrationStatus,
    NewCredential, RefreshOwner,
};
pub(crate) use migration::{CODEX_MIGRATION_ID, COPILOT_MIGRATION_ID, XAI_MIGRATION_ID};
pub(crate) use repository::ManagedAuthRepository;
pub(crate) use secret_bundle::{ManagedAuthSecretBundle, ManagedAuthSecretBundleParts};
pub(crate) use service::{
    AccessMaterial, CompatibilityAccount, ManagedAuthService, NativeManagedAuthService,
};

pub const MANAGED_AUTH_CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthProvider {
    Openai,
    Xai,
    GithubCopilot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthConsumer {
    Codex,
    Grokbuild,
    Opencode,
    FyagentProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthLoginMethod {
    BrowserLoopback,
    DeviceCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthLoginPurpose {
    SaveOnly,
    ConnectConsumer,
    Reauthenticate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthHealth {
    Ready,
    Checking,
    RequiresReauth,
    MigrationBlocked,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthAccountAction {
    Reauthenticate,
    SetDefault,
    Remove,
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthConnectionState {
    Connected,
    Disconnected,
    Checking,
    RequiresReauth,
    PendingRestart,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthConnectionAction {
    ConnectAccount,
    SwitchAccount,
    Disconnect,
    Refresh,
    Restart,
    OpenConsumer,
    SwitchToOfficial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthRequestMode {
    OfficialSubscription,
    ThirdPartyApi,
    ProviderConnections,
    None,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthCredentialManager {
    Fyagent,
    Codex,
    Grokbuild,
    Opencode,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthReasonCode {
    NativeOnly,
    ObserverUnavailable,
    OperationConflict,
    RequiresReauth,
    MigrationBlocked,
    SecretUnavailable,
    ConnectionUnavailable,
    NativeProjectionUnavailable,
    TargetSelectionRequired,
    TargetChanged,
    PendingRestart,
    ExternalChangeDetected,
    ProviderNotSupported,
    CallbackUnavailable,
    DeviceCodeExpired,
    IdentityMismatch,
    PartialCompletion,
    Cancelled,
    TimedOut,
    LoginFailed,
    InvalidResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthLoginStage {
    Preparing,
    OpeningBrowser,
    AwaitingUser,
    ExchangingCode,
    SavingAccount,
    ConnectingConsumer,
    Verifying,
    Completed,
    Partial,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedAuthMutationOutcome {
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthProviderSummary {
    pub provider: ManagedAuthProvider,
    pub available: bool,
    pub login_methods: Vec<ManagedAuthLoginMethod>,
    pub consumers: Vec<ManagedAuthConsumer>,
    pub reason_codes: Vec<ManagedAuthReasonCode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthAccountSummary {
    pub account_id: String,
    pub revision: String,
    pub provider: ManagedAuthProvider,
    pub login: String,
    pub display_name: Option<String>,
    pub health: ManagedAuthHealth,
    pub is_default: bool,
    pub last_authenticated_at: Option<String>,
    pub connected_consumer_count: usize,
    pub plan_summary: Option<String>,
    pub quota_summary: Option<String>,
    pub allowed_actions: Vec<ManagedAuthAccountAction>,
    pub reason_codes: Vec<ManagedAuthReasonCode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthConnectionSummary {
    pub connection_id: String,
    pub revision: String,
    pub consumer: ManagedAuthConsumer,
    pub target_id: Option<String>,
    pub target_label: Option<String>,
    pub provider: Option<ManagedAuthProvider>,
    pub account_id: Option<String>,
    pub auth_status: ManagedAuthConnectionState,
    pub credential_manager: ManagedAuthCredentialManager,
    pub request_mode: ManagedAuthRequestMode,
    pub request_provider_label: Option<String>,
    pub official_session_preserved: Option<bool>,
    pub pending_restart: bool,
    pub allowed_actions: Vec<ManagedAuthConnectionAction>,
    pub checked_at: String,
    pub reason_codes: Vec<ManagedAuthReasonCode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthLoginSessionSnapshot {
    pub contract_version: u8,
    pub session_id: String,
    pub provider: ManagedAuthProvider,
    pub purpose: ManagedAuthLoginPurpose,
    pub consumer: Option<ManagedAuthConsumer>,
    pub method: ManagedAuthLoginMethod,
    pub stage: ManagedAuthLoginStage,
    pub can_cancel: bool,
    pub can_retry: bool,
    pub can_switch_to_device_code: bool,
    pub official_host: String,
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
    pub expires_at: Option<String>,
    pub account_id: Option<String>,
    pub connection_id: Option<String>,
    pub reason_code: Option<ManagedAuthReasonCode>,
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthOverview {
    pub contract_version: u8,
    pub checked_at: String,
    pub providers: Vec<ManagedAuthProviderSummary>,
    pub accounts: Vec<ManagedAuthAccountSummary>,
    pub connections: Vec<ManagedAuthConnectionSummary>,
    pub active_sessions: Vec<ManagedAuthLoginSessionSnapshot>,
    pub reason_codes: Vec<ManagedAuthReasonCode>,
}

impl ManagedAuthOverview {
    pub fn unavailable() -> Self {
        let reason = ManagedAuthReasonCode::NativeProjectionUnavailable;
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            checked_at: now_timestamp(),
            providers: vec![
                ManagedAuthProviderSummary {
                    provider: ManagedAuthProvider::Openai,
                    available: false,
                    login_methods: vec![],
                    consumers: vec![
                        ManagedAuthConsumer::Codex,
                        ManagedAuthConsumer::Opencode,
                        ManagedAuthConsumer::FyagentProxy,
                    ],
                    reason_codes: vec![reason],
                },
                ManagedAuthProviderSummary {
                    provider: ManagedAuthProvider::Xai,
                    available: false,
                    login_methods: vec![],
                    consumers: vec![
                        ManagedAuthConsumer::Grokbuild,
                        ManagedAuthConsumer::Opencode,
                        ManagedAuthConsumer::FyagentProxy,
                    ],
                    reason_codes: vec![reason],
                },
                ManagedAuthProviderSummary {
                    provider: ManagedAuthProvider::GithubCopilot,
                    available: false,
                    login_methods: vec![],
                    consumers: vec![
                        ManagedAuthConsumer::Opencode,
                        ManagedAuthConsumer::FyagentProxy,
                    ],
                    reason_codes: vec![reason],
                },
            ],
            accounts: vec![],
            connections: vec![],
            active_sessions: vec![],
            reason_codes: vec![reason],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthAccountRemovalImpact {
    pub consumer: ManagedAuthConsumer,
    pub target_label: Option<String>,
    pub request_mode: ManagedAuthRequestMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthAccountRemovalPreview {
    pub contract_version: u8,
    pub preview_id: String,
    pub account_id: String,
    pub expected_revision: String,
    pub disconnects: Vec<ManagedAuthAccountRemovalImpact>,
    pub preserved: Vec<ManagedAuthAccountRemovalImpact>,
    pub can_apply: bool,
    pub reason_codes: Vec<ManagedAuthReasonCode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthMutationResult {
    pub contract_version: u8,
    pub operation_id: String,
    pub outcome: ManagedAuthMutationOutcome,
    pub overview: ManagedAuthOverview,
    pub pending_restart_consumers: Vec<ManagedAuthConsumer>,
    pub reason_code: Option<ManagedAuthReasonCode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartManagedAuthLoginRequest {
    pub provider: ManagedAuthProvider,
    pub purpose: ManagedAuthLoginPurpose,
    pub consumer: Option<ManagedAuthConsumer>,
    pub method: ManagedAuthLoginMethod,
    pub account_id: Option<String>,
}

impl StartManagedAuthLoginRequest {
    pub fn validate(&self) -> Result<(), ManagedAuthErrorDto> {
        if self.provider != ManagedAuthProvider::Openai
            && self.method == ManagedAuthLoginMethod::BrowserLoopback
        {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        match self.purpose {
            ManagedAuthLoginPurpose::SaveOnly if self.consumer.is_some() => {
                Err(ManagedAuthErrorDto::invalid_request())
            }
            ManagedAuthLoginPurpose::ConnectConsumer
                if self.consumer.is_none() || self.account_id.is_some() =>
            {
                Err(ManagedAuthErrorDto::invalid_request())
            }
            ManagedAuthLoginPurpose::Reauthenticate
                if self
                    .account_id
                    .as_deref()
                    .is_none_or(|id| !valid_account_id(id)) =>
            {
                Err(ManagedAuthErrorDto::invalid_request())
            }
            _ if self
                .account_id
                .as_deref()
                .is_some_and(|id| !valid_account_id(id)) =>
            {
                Err(ManagedAuthErrorDto::invalid_request())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedAuthAccountMutationRequest {
    pub account_id: String,
    pub expected_revision: String,
}

impl ManagedAuthAccountMutationRequest {
    pub fn validate(&self) -> Result<(), ManagedAuthErrorDto> {
        if valid_account_id(&self.account_id) && valid_revision(&self.expected_revision) {
            Ok(())
        } else {
            Err(ManagedAuthErrorDto::invalid_request())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedAuthAccountRemovalRequest {
    pub preview_id: String,
    pub account_id: String,
    pub expected_revision: String,
}

impl ManagedAuthAccountRemovalRequest {
    pub fn validate(&self) -> Result<(), ManagedAuthErrorDto> {
        if valid_prefixed_hex(&self.preview_id, "mp1:", 32)
            && valid_account_id(&self.account_id)
            && valid_revision(&self.expected_revision)
        {
            Ok(())
        } else {
            Err(ManagedAuthErrorDto::invalid_request())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedAuthConnectionActionRequest {
    pub connection_id: String,
    pub expected_revision: String,
    pub action: ManagedAuthConnectionAction,
    pub account_id: Option<String>,
}

impl ManagedAuthConnectionActionRequest {
    pub fn validate(&self) -> Result<(), ManagedAuthErrorDto> {
        let needs_account = matches!(
            self.action,
            ManagedAuthConnectionAction::ConnectAccount
                | ManagedAuthConnectionAction::SwitchAccount
        );
        if !valid_prefixed_hex(&self.connection_id, "mc1:", 32)
            || !valid_revision(&self.expected_revision)
            || needs_account != self.account_id.is_some()
            || self
                .account_id
                .as_deref()
                .is_some_and(|id| !valid_account_id(id))
        {
            return Err(ManagedAuthErrorDto::invalid_request());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAuthErrorDto {
    pub contract_version: u8,
    pub reason_code: ManagedAuthReasonCode,
}

impl ManagedAuthErrorDto {
    pub fn unavailable() -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            reason_code: ManagedAuthReasonCode::NativeProjectionUnavailable,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            reason_code: ManagedAuthReasonCode::InvalidResponse,
        }
    }

    pub(crate) fn with_reason(reason_code: ManagedAuthReasonCode) -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            reason_code,
        }
    }

    pub(crate) fn from_core(error: ManagedAuthCoreError) -> Self {
        Self {
            contract_version: MANAGED_AUTH_CONTRACT_VERSION,
            reason_code: error.reason_code(),
        }
    }
}

pub fn validate_session_id(value: &str) -> Result<(), ManagedAuthErrorDto> {
    let id = Uuid::parse_str(value).map_err(|_| ManagedAuthErrorDto::invalid_request())?;
    if id.get_version_num() == 0 {
        return Err(ManagedAuthErrorDto::invalid_request());
    }
    Ok(())
}

fn valid_account_id(value: &str) -> bool {
    valid_prefixed_hex(value, "ma1:", 32)
}

fn valid_revision(value: &str) -> bool {
    valid_prefixed_hex(value, "mr1:", 64)
}

fn valid_prefixed_hex(value: &str, prefix: &str, length: usize) -> bool {
    value.strip_prefix(prefix).is_some_and(|tail| {
        tail.len() == length
            && tail
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub(crate) fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_overview_is_closed_and_credential_free() {
        let value = serde_json::to_value(ManagedAuthOverview::unavailable()).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(
            object
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            [
                "accounts",
                "activeSessions",
                "checkedAt",
                "connections",
                "contractVersion",
                "providers",
                "reasonCodes",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
        let text = value.to_string().to_ascii_lowercase();
        for forbidden in [
            "access_token",
            "refresh_token",
            "id_token",
            "authorization_code",
            "device_code",
            "secretref",
            "verifier",
            "secret_ref",
        ] {
            assert!(!text.contains(forbidden));
        }
    }

    #[test]
    fn request_validation_is_closed_before_any_future_side_effect() {
        let valid = StartManagedAuthLoginRequest {
            provider: ManagedAuthProvider::Openai,
            purpose: ManagedAuthLoginPurpose::ConnectConsumer,
            consumer: Some(ManagedAuthConsumer::Codex),
            method: ManagedAuthLoginMethod::BrowserLoopback,
            account_id: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = StartManagedAuthLoginRequest {
            provider: ManagedAuthProvider::Xai,
            purpose: ManagedAuthLoginPurpose::ConnectConsumer,
            consumer: Some(ManagedAuthConsumer::Grokbuild),
            method: ManagedAuthLoginMethod::BrowserLoopback,
            account_id: None,
        };
        assert!(invalid.validate().is_err());
    }
}
