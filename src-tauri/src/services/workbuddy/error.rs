//! Stable, privacy-safe WorkBuddy error DTOs.
//!
//! The renderer branches on `code` and localizes `messageKey`; neither raw
//! configuration data nor request credentials are serialized across IPC.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum WorkBuddyErrorCode {
    #[serde(rename = "WORKBUDDY_INVALID_URL")]
    InvalidUrl,
    #[serde(rename = "WORKBUDDY_API_KEY_REQUIRED")]
    ApiKeyRequired,
    #[serde(rename = "WORKBUDDY_FETCH_HTTP_ERROR")]
    FetchHttpError,
    #[serde(rename = "WORKBUDDY_FETCH_TIMEOUT")]
    FetchTimeout,
    #[serde(rename = "WORKBUDDY_FETCH_REDIRECT_REJECTED")]
    FetchRedirectRejected,
    #[serde(rename = "WORKBUDDY_FETCH_RESPONSE_TOO_LARGE")]
    FetchResponseTooLarge,
    #[serde(rename = "WORKBUDDY_FETCH_INVALID_SCHEMA")]
    FetchInvalidSchema,
    #[serde(rename = "WORKBUDDY_CONFIG_READ_FAILED")]
    ConfigReadFailed,
    #[serde(rename = "WORKBUDDY_CONFIG_INVALID_JSON")]
    ConfigInvalidJson,
    #[serde(rename = "WORKBUDDY_CONFIG_ROOT_UNSUPPORTED")]
    ConfigRootUnsupported,
    #[serde(rename = "WORKBUDDY_CONFIG_MODELS_NOT_ARRAY")]
    ConfigModelsNotArray,
    #[serde(rename = "WORKBUDDY_CONFIG_INVALID_ENTRY")]
    ConfigInvalidEntry,
    #[serde(rename = "WORKBUDDY_CONFIG_NO_TARGET_MODELS")]
    ConfigNoTargetModels,
    #[serde(rename = "WORKBUDDY_CONFIG_CONCURRENT_MODIFICATION")]
    ConfigConcurrentModification,
    #[serde(rename = "WORKBUDDY_OVERWRITE_TOKEN_INVALID")]
    OverwriteTokenInvalid,
    #[serde(rename = "WORKBUDDY_OVERWRITE_TOKEN_EXPIRED")]
    OverwriteTokenExpired,
    #[serde(rename = "WORKBUDDY_CONFIG_BACKUP_FAILED")]
    ConfigBackupFailed,
    #[serde(rename = "WORKBUDDY_CONFIG_WRITE_FAILED")]
    ConfigWriteFailed,
    #[serde(rename = "WORKBUDDY_INTERNAL_ERROR")]
    InternalError,
}

impl WorkBuddyErrorCode {
    pub const fn message_key(self) -> &'static str {
        match self {
            Self::InvalidUrl => "workbuddy.error.invalidUrl",
            Self::ApiKeyRequired => "workbuddy.error.apiKeyRequired",
            Self::FetchHttpError => "workbuddy.error.fetchHttp",
            Self::FetchTimeout => "workbuddy.error.fetchTimeout",
            Self::FetchRedirectRejected => "workbuddy.error.fetchRedirectRejected",
            Self::FetchResponseTooLarge => "workbuddy.error.fetchResponseTooLarge",
            Self::FetchInvalidSchema => "workbuddy.error.fetchInvalidSchema",
            Self::ConfigReadFailed => "workbuddy.error.configReadFailed",
            Self::ConfigInvalidJson => "workbuddy.error.configInvalidJson",
            Self::ConfigRootUnsupported => "workbuddy.error.configRootUnsupported",
            Self::ConfigModelsNotArray => "workbuddy.error.configModelsNotArray",
            Self::ConfigInvalidEntry => "workbuddy.error.configInvalidEntry",
            Self::ConfigNoTargetModels => "workbuddy.error.configNoTargetModels",
            Self::ConfigConcurrentModification => "workbuddy.error.configConcurrentModification",
            Self::OverwriteTokenInvalid => "workbuddy.error.overwriteTokenInvalid",
            Self::OverwriteTokenExpired => "workbuddy.error.overwriteTokenExpired",
            Self::ConfigBackupFailed => "workbuddy.error.configBackupFailed",
            Self::ConfigWriteFailed => "workbuddy.error.configWriteFailed",
            Self::InternalError => "workbuddy.error.internal",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_entry_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyErrorDto {
    pub code: WorkBuddyErrorCode,
    pub message_key: String,
    pub details: WorkBuddyErrorDetails,
}

#[derive(Debug, Clone, Error)]
#[error("WorkBuddy error: {code:?}")]
pub struct WorkBuddyError {
    code: WorkBuddyErrorCode,
    details: WorkBuddyErrorDetails,
}

impl WorkBuddyError {
    pub fn new(code: WorkBuddyErrorCode) -> Self {
        Self {
            code,
            details: WorkBuddyErrorDetails::default(),
        }
    }

    pub fn with_http_status(mut self, status: u16) -> Self {
        self.details.http_status = Some(status);
        self
    }

    pub fn with_redacted_summary(mut self, summary: impl Into<String>) -> Self {
        self.details.redacted_summary = Some(summary.into());
        self
    }

    pub fn with_invalid_entry_index(mut self, index: usize) -> Self {
        self.details.invalid_entry_index = Some(index);
        self
    }

    pub const fn code(&self) -> WorkBuddyErrorCode {
        self.code
    }

    pub fn to_dto(&self) -> WorkBuddyErrorDto {
        WorkBuddyErrorDto {
            code: self.code,
            message_key: self.code.message_key().to_string(),
            details: self.details.clone(),
        }
    }
}

impl From<WorkBuddyError> for WorkBuddyErrorDto {
    fn from(value: WorkBuddyError) -> Self {
        value.to_dto()
    }
}

/// Redact the bounded server-provided summary before it crosses the command
/// boundary. If the upstream echoed the supplied API key, omit the whole body
/// rather than risk retaining a variant of that credential.
pub(crate) fn redact_response_summary(body: &str, api_key: &str) -> String {
    let summary = crate::codex_desktop::error::redact_diagnostic_text(body);
    if !api_key.is_empty() && summary.contains(api_key) {
        "The server returned an error response.".to_string()
    } else {
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwrite_token_error_serializes_only_stable_non_secret_fields() {
        let dto = WorkBuddyError::new(WorkBuddyErrorCode::OverwriteTokenInvalid).to_dto();

        assert_eq!(dto.code, WorkBuddyErrorCode::OverwriteTokenInvalid);
        assert_eq!(dto.message_key, "workbuddy.error.overwriteTokenInvalid");
        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            serde_json::json!({
                "code": "WORKBUDDY_OVERWRITE_TOKEN_INVALID",
                "messageKey": "workbuddy.error.overwriteTokenInvalid",
                "details": {}
            })
        );
    }

    #[test]
    fn response_summary_redacts_credentials_and_user_paths() {
        let summary = redact_response_summary(
            r"Authorization: Bearer fake-secret https://user:pass@example.test/x?token=nope C:\Users\alice",
            "fake-secret",
        );

        assert!(!summary.contains("fake-secret"));
        assert!(!summary.contains("user:pass"));
        assert!(!summary.contains("token=nope"));
        assert!(!summary.contains("alice"));
    }

    #[test]
    fn response_summary_drops_a_body_that_echoes_the_input_key() {
        let summary = redact_response_summary("upstream saw test-api-key", "test-api-key");
        assert_eq!(summary, "The server returned an error response.");
    }
}

/// Closed failures for the subscription route. Display is the code only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkBuddySubscriptionErrorCode {
    Stale,
    RecoveryRequired,
    InvalidRequest,
    AccountUnavailable,
    SubscriptionUnavailable,
    ModelMismatch,
    UnknownRoute,
    ProxyCredentialRejected,
    Timeout,
    UpstreamError,
    PortConflict,
    Internal,
}

impl WorkBuddySubscriptionErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::RecoveryRequired => "recovery_required",
            Self::InvalidRequest => "invalid_request",
            Self::AccountUnavailable => "account_unavailable",
            Self::SubscriptionUnavailable => "subscription_unavailable",
            Self::ModelMismatch => "model_mismatch",
            Self::UnknownRoute => "unknown_route",
            Self::ProxyCredentialRejected => "proxy_credential_rejected",
            Self::Timeout => "timeout",
            Self::UpstreamError => "upstream_error",
            Self::PortConflict => "port_conflict",
            Self::Internal => "internal",
        }
    }

    pub const fn http_status(self) -> u16 {
        match self {
            Self::Stale => 409,
            Self::RecoveryRequired => 500,
            Self::InvalidRequest | Self::ModelMismatch => 400,
            Self::AccountUnavailable | Self::ProxyCredentialRejected => 401,
            Self::SubscriptionUnavailable | Self::UnknownRoute => 404,
            Self::Timeout => 504,
            Self::UpstreamError | Self::PortConflict | Self::Internal => 502,
        }
    }
}

impl std::fmt::Display for WorkBuddySubscriptionErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkBuddySubscriptionError {
    code: WorkBuddySubscriptionErrorCode,
}

impl WorkBuddySubscriptionError {
    pub const fn new(code: WorkBuddySubscriptionErrorCode) -> Self {
        Self { code }
    }

    // 第一刀仅测试覆盖，第二刀再挂命令。
    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn code(self) -> WorkBuddySubscriptionErrorCode {
        self.code
    }
}

impl std::fmt::Display for WorkBuddySubscriptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl std::error::Error for WorkBuddySubscriptionError {}
