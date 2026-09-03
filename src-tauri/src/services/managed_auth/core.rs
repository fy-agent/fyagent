use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::services::secret::{SecretErrorCode, SecretHandle, SecretServiceError};

use super::{ManagedAuthConsumer, ManagedAuthProvider, ManagedAuthReasonCode};

pub(crate) const ACCOUNT_ID_PREFIX: &str = "ma1:";
pub(crate) const CREDENTIAL_ID_PREFIX: &str = "mcred1:";
pub(crate) const CONNECTION_ID_PREFIX: &str = "mc1:";
pub(crate) const REVISION_PREFIX: &str = "mr1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CredentialPurpose {
    ProxyUpstream,
    CodexNative,
    GrokNative,
    OpencodeProvider,
    Copilot,
}

impl CredentialPurpose {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ProxyUpstream => "proxy_upstream",
            Self::CodexNative => "codex_native",
            Self::GrokNative => "grok_native",
            Self::OpencodeProvider => "opencode_provider",
            Self::Copilot => "copilot",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "proxy_upstream" => Ok(Self::ProxyUpstream),
            "codex_native" => Ok(Self::CodexNative),
            "grok_native" => Ok(Self::GrokNative),
            "opencode_provider" => Ok(Self::OpencodeProvider),
            "copilot" => Ok(Self::Copilot),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefreshOwner {
    Fyagent,
    CodexNative,
    GrokNative,
    Opencode,
    Unavailable,
}

impl RefreshOwner {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Fyagent => "fyagent",
            Self::CodexNative => "codex_native",
            Self::GrokNative => "grok_native",
            Self::Opencode => "opencode",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "fyagent" => Ok(Self::Fyagent),
            "codex_native" => Ok(Self::CodexNative),
            "grok_native" => Ok(Self::GrokNative),
            "opencode" => Ok(Self::Opencode),
            "unavailable" => Ok(Self::Unavailable),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CredentialStatus {
    Provisioning,
    Ready,
    RequiresReauth,
    SecretMissing,
    MigrationBlocked,
    Revoked,
}

impl CredentialStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::RequiresReauth => "requires_reauth",
            Self::SecretMissing => "secret_missing",
            Self::MigrationBlocked => "migration_blocked",
            Self::Revoked => "revoked",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "provisioning" => Ok(Self::Provisioning),
            "ready" => Ok(Self::Ready),
            "requires_reauth" => Ok(Self::RequiresReauth),
            "secret_missing" => Ok(Self::SecretMissing),
            "migration_blocked" => Ok(Self::MigrationBlocked),
            "revoked" => Ok(Self::Revoked),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MigrationStatus {
    Copying,
    Prepared,
    Completed,
    Blocked,
}

impl MigrationStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Copying => "copying",
            Self::Prepared => "prepared",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "copying" => Ok(Self::Copying),
            "prepared" => Ok(Self::Prepared),
            "completed" => Ok(Self::Completed),
            "blocked" => Ok(Self::Blocked),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IdentityRecord {
    pub(crate) identity_id: String,
    pub(crate) provider: ManagedAuthProvider,
    pub(crate) provider_subject: String,
    pub(crate) provider_tenant: String,
    pub(crate) login: String,
    pub(crate) display_name: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct CredentialRecord {
    pub(crate) credential_id: String,
    pub(crate) identity_id: String,
    pub(crate) provider: ManagedAuthProvider,
    pub(crate) purpose: CredentialPurpose,
    pub(crate) consumer: Option<ManagedAuthConsumer>,
    pub(crate) legacy_account_id: String,
    pub(crate) secret_handle: SecretHandle,
    pub(crate) refresh_owner: RefreshOwner,
    pub(crate) generation: u64,
    pub(crate) access_expires_at: Option<i64>,
    pub(crate) status: CredentialStatus,
    pub(crate) authenticated_at: i64,
    pub(crate) refreshed_at: Option<i64>,
    pub(crate) migration_id: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct CredentialWithIdentity {
    pub(crate) credential: CredentialRecord,
    pub(crate) identity: IdentityRecord,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MigrationRecord {
    pub(crate) migration_id: String,
    pub(crate) source_kind: String,
    pub(crate) source_hash: String,
    pub(crate) status: MigrationStatus,
    pub(crate) reason_code: Option<String>,
    pub(crate) backup_name: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) completed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct NewCredential {
    pub(crate) identity: IdentityRecord,
    pub(crate) credential: CredentialRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionStatus {
    Disconnected,
    Connected,
    Checking,
    RequiresReauth,
    PendingRestart,
    ExternalChangeDetected,
    RecoveryRequired,
    Unavailable,
}

impl ConnectionStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connected => "connected",
            Self::Checking => "checking",
            Self::RequiresReauth => "requires_reauth",
            Self::PendingRestart => "pending_restart",
            Self::ExternalChangeDetected => "external_change_detected",
            Self::RecoveryRequired => "recovery_required",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "disconnected" => Ok(Self::Disconnected),
            "connected" => Ok(Self::Connected),
            "checking" => Ok(Self::Checking),
            "requires_reauth" => Ok(Self::RequiresReauth),
            "pending_restart" => Ok(Self::PendingRestart),
            "external_change_detected" => Ok(Self::ExternalChangeDetected),
            "recovery_required" => Ok(Self::RecoveryRequired),
            "unavailable" => Ok(Self::Unavailable),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConnectionRecord {
    pub(crate) connection_id: String,
    pub(crate) consumer: ManagedAuthConsumer,
    pub(crate) target_id: String,
    pub(crate) provider_slot: String,
    pub(crate) credential_id: Option<String>,
    pub(crate) desired_revision: String,
    pub(crate) observed_revision: Option<String>,
    pub(crate) status: ConnectionStatus,
    pub(crate) request_mode: super::ManagedAuthRequestMode,
    pub(crate) request_provider_label: Option<String>,
    pub(crate) official_session_preserved: Option<bool>,
    pub(crate) pending_restart: bool,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ManagedAuthCoreError {
    #[error("managed auth database operation failed")]
    Database,
    #[error("managed auth secret store is unavailable")]
    SecretUnavailable,
    #[error("managed auth secret is missing")]
    SecretMissing,
    #[error("managed auth data is invalid")]
    InvalidData,
    #[error("managed auth state is stale")]
    Stale,
    #[error("managed auth record was not found")]
    NotFound,
    #[error("managed auth operation conflicts with current state")]
    Conflict,
    #[error("managed auth migration is blocked")]
    MigrationBlocked,
    #[error("managed auth filesystem operation failed")]
    Io,
}

impl ManagedAuthCoreError {
    pub(crate) const fn reason_code(&self) -> ManagedAuthReasonCode {
        match self {
            Self::SecretUnavailable | Self::SecretMissing => {
                ManagedAuthReasonCode::SecretUnavailable
            }
            Self::Stale => ManagedAuthReasonCode::ExternalChangeDetected,
            Self::Conflict => ManagedAuthReasonCode::OperationConflict,
            Self::MigrationBlocked => ManagedAuthReasonCode::MigrationBlocked,
            Self::Database | Self::InvalidData | Self::NotFound | Self::Io => {
                ManagedAuthReasonCode::InvalidResponse
            }
        }
    }
}

impl From<AppError> for ManagedAuthCoreError {
    fn from(_: AppError) -> Self {
        Self::Database
    }
}

impl From<SecretServiceError> for ManagedAuthCoreError {
    fn from(error: SecretServiceError) -> Self {
        match error.code() {
            SecretErrorCode::Missing => Self::SecretMissing,
            SecretErrorCode::Locked
            | SecretErrorCode::PermissionDenied
            | SecretErrorCode::BackendUnavailable => Self::SecretUnavailable,
            SecretErrorCode::RefInvalid
            | SecretErrorCode::InputInvalid
            | SecretErrorCode::AlreadyExists
            | SecretErrorCode::WriteFailed
            | SecretErrorCode::ReadFailed
            | SecretErrorCode::DeleteFailed
            | SecretErrorCode::VerifyFailed
            | SecretErrorCode::Internal => Self::InvalidData,
        }
    }
}

impl ManagedAuthProvider {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Xai => "xai",
            Self::GithubCopilot => "github_copilot",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "openai" => Ok(Self::Openai),
            "xai" => Ok(Self::Xai),
            "github_copilot" => Ok(Self::GithubCopilot),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

impl super::ManagedAuthRequestMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::OfficialSubscription => "official_subscription",
            Self::ThirdPartyApi => "third_party_api",
            Self::ProviderConnections => "provider_connections",
            Self::None => "none",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, ManagedAuthCoreError> {
        match value {
            "official_subscription" => Ok(Self::OfficialSubscription),
            "third_party_api" => Ok(Self::ThirdPartyApi),
            "provider_connections" => Ok(Self::ProviderConnections),
            "none" => Ok(Self::None),
            "unknown" => Ok(Self::Unknown),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

impl ManagedAuthConsumer {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Grokbuild => "grokbuild",
            Self::Opencode => "opencode",
            Self::FyagentProxy => "fyagent_proxy",
        }
    }

    pub(crate) fn parse_optional(value: &str) -> Result<Option<Self>, ManagedAuthCoreError> {
        match value {
            "" => Ok(None),
            "codex" => Ok(Some(Self::Codex)),
            "grokbuild" => Ok(Some(Self::Grokbuild)),
            "opencode" => Ok(Some(Self::Opencode)),
            "fyagent_proxy" => Ok(Some(Self::FyagentProxy)),
            _ => Err(ManagedAuthCoreError::InvalidData),
        }
    }
}

pub(crate) fn stable_identity_id(
    provider: ManagedAuthProvider,
    subject: &str,
    tenant: &str,
) -> String {
    stable_id(
        ACCOUNT_ID_PREFIX,
        &["identity-v1", provider.as_str(), subject, tenant],
        16,
    )
}

pub(crate) fn stable_credential_id(
    provider: ManagedAuthProvider,
    purpose: CredentialPurpose,
    consumer: Option<ManagedAuthConsumer>,
    legacy_account_id: &str,
) -> String {
    stable_id(
        CREDENTIAL_ID_PREFIX,
        &[
            "credential-v1",
            provider.as_str(),
            purpose.as_str(),
            consumer.map(ManagedAuthConsumer::as_str).unwrap_or(""),
            legacy_account_id,
        ],
        16,
    )
}

pub(crate) fn stable_connection_id(
    consumer: ManagedAuthConsumer,
    target_id: &str,
    provider_slot: &str,
) -> String {
    stable_id(
        CONNECTION_ID_PREFIX,
        &["connection-v1", consumer.as_str(), target_id, provider_slot],
        16,
    )
}

pub(crate) fn stable_revision(parts: &[&str]) -> String {
    stable_id(REVISION_PREFIX, parts, 32)
}

fn stable_id(prefix: &str, parts: &[&str], bytes: usize) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    let digest = digest.finalize();
    let mut result = String::with_capacity(prefix.len() + bytes * 2);
    result.push_str(prefix);
    for byte in &digest[..bytes] {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_are_domain_separated_and_contract_shaped() {
        let account = stable_identity_id(ManagedAuthProvider::Openai, "subject", "tenant");
        let credential = stable_credential_id(
            ManagedAuthProvider::Openai,
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
            "legacy",
        );
        let connection = stable_connection_id(ManagedAuthConsumer::Codex, "", "official");
        let revision = stable_revision(&["account", &account]);

        assert!(account.starts_with(ACCOUNT_ID_PREFIX));
        assert_eq!(account.len(), ACCOUNT_ID_PREFIX.len() + 32);
        assert!(credential.starts_with(CREDENTIAL_ID_PREFIX));
        assert!(connection.starts_with(CONNECTION_ID_PREFIX));
        assert_eq!(connection.len(), CONNECTION_ID_PREFIX.len() + 32);
        assert!(revision.starts_with(REVISION_PREFIX));
        assert_eq!(revision.len(), REVISION_PREFIX.len() + 64);
        assert_ne!(account, credential);
    }

    #[test]
    fn refresh_owner_rejects_shared() {
        assert!(RefreshOwner::parse("shared").is_err());
        assert!(CredentialPurpose::parse("shared").is_err());
        assert_eq!(RefreshOwner::GrokNative.as_str(), "grok_native");
        assert_eq!(CredentialPurpose::GrokNative.as_str(), "grok_native");
        assert_eq!(CredentialPurpose::ProxyUpstream.as_str(), "proxy_upstream");
    }
}
