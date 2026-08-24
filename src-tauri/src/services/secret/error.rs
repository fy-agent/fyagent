use std::fmt;

use serde::Serialize;

use super::types::{BackendProbe, SecretAvailability, SecretPresence};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum SecretErrorCode {
    #[serde(rename = "SECRET_REF_INVALID")]
    RefInvalid,
    #[serde(rename = "SECRET_INPUT_INVALID")]
    InputInvalid,
    #[serde(rename = "SECRET_ALREADY_EXISTS")]
    AlreadyExists,
    #[serde(rename = "SECRET_MISSING")]
    Missing,
    #[serde(rename = "SECRET_LOCKED")]
    Locked,
    #[serde(rename = "SECRET_PERMISSION_DENIED")]
    PermissionDenied,
    #[serde(rename = "SECRET_BACKEND_UNAVAILABLE")]
    BackendUnavailable,
    #[serde(rename = "SECRET_WRITE_FAILED")]
    WriteFailed,
    #[serde(rename = "SECRET_READ_FAILED")]
    ReadFailed,
    #[serde(rename = "SECRET_DELETE_FAILED")]
    DeleteFailed,
    #[serde(rename = "SECRET_VERIFY_FAILED")]
    VerifyFailed,
    #[serde(rename = "SECRET_INTERNAL")]
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SecretRecoveryAction {
    None,
    Recapture,
    UnlockSystemStore,
    ReviewSystemPermissions,
    ReopenSystemStore,
    RefreshSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretServiceError {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretRecoveryAction,
}

impl SecretServiceError {
    pub(crate) const fn invalid_ref() -> Self {
        Self::new(
            SecretErrorCode::RefInvalid,
            false,
            SecretRecoveryAction::None,
        )
    }

    pub(crate) const fn invalid_input() -> Self {
        Self::new(
            SecretErrorCode::InputInvalid,
            false,
            SecretRecoveryAction::Recapture,
        )
    }

    pub(crate) const fn already_exists() -> Self {
        Self::new(
            SecretErrorCode::AlreadyExists,
            false,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    pub(crate) const fn missing() -> Self {
        Self::new(
            SecretErrorCode::Missing,
            false,
            SecretRecoveryAction::Recapture,
        )
    }

    pub(crate) const fn locked() -> Self {
        Self::new(
            SecretErrorCode::Locked,
            true,
            SecretRecoveryAction::UnlockSystemStore,
        )
    }

    pub(crate) const fn permission_denied() -> Self {
        Self::new(
            SecretErrorCode::PermissionDenied,
            true,
            SecretRecoveryAction::ReviewSystemPermissions,
        )
    }

    pub(crate) const fn backend_unavailable() -> Self {
        Self::new(
            SecretErrorCode::BackendUnavailable,
            true,
            SecretRecoveryAction::ReopenSystemStore,
        )
    }

    pub(crate) const fn write_failed() -> Self {
        Self::new(
            SecretErrorCode::WriteFailed,
            true,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    pub(crate) const fn read_failed() -> Self {
        Self::new(
            SecretErrorCode::ReadFailed,
            true,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    pub(crate) const fn delete_failed() -> Self {
        Self::new(
            SecretErrorCode::DeleteFailed,
            true,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    pub(crate) const fn verify_failed() -> Self {
        Self::new(
            SecretErrorCode::VerifyFailed,
            false,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    pub(crate) const fn internal() -> Self {
        Self::new(
            SecretErrorCode::Internal,
            true,
            SecretRecoveryAction::RefreshSummary,
        )
    }

    const fn new(code: SecretErrorCode, retryable: bool, action: SecretRecoveryAction) -> Self {
        Self {
            code,
            retryable,
            action,
        }
    }

    pub(crate) fn code(&self) -> SecretErrorCode {
        self.code
    }

    pub(crate) const fn as_probe(&self) -> Option<BackendProbe> {
        match self.code {
            SecretErrorCode::Missing => Some(BackendProbe::missing()),
            SecretErrorCode::Locked => Some(BackendProbe {
                presence: SecretPresence::Unknown,
                availability: SecretAvailability::Locked,
            }),
            SecretErrorCode::PermissionDenied => Some(BackendProbe {
                presence: SecretPresence::Unknown,
                availability: SecretAvailability::Denied,
            }),
            SecretErrorCode::BackendUnavailable => Some(BackendProbe {
                presence: SecretPresence::Unknown,
                availability: SecretAvailability::Unavailable,
            }),
            _ => None,
        }
    }
}

impl fmt::Display for SecretServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.code)
    }
}

impl std::error::Error for SecretServiceError {}
