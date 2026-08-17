//! Windows Credential Manager compile-gate for Phase 2B.
//!
//! Documented mapping (not executed on macOS hosts):
//! - CRED_TYPE_GENERIC
//! - TargetName = `"FyAgent/secret/v1/" + SecretRef`
//! - Persist = LOCAL_MACHINE
//! - UserName = "FyAgent"
//!
//! Methods return unavailable behind `cfg(target_os = "windows")`. This
//! module exists so `WindowsSecretStore` can be sealed in backend.rs.

use crate::secret::{
    SecretBackendUnavailableReason, SecretInternalError, SecretMaterial, SecretRef,
    SecretSourceFreeErrorCode, SecretTerminalOperationContext,
};

pub(crate) struct WindowsSecretStore;

impl WindowsSecretStore {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self
    }

    #[cfg(target_os = "windows")]
    fn unavailable() -> SecretInternalError {
        SecretInternalError::backend_unavailable(
            SecretTerminalOperationContext::Summary,
            SecretBackendUnavailableReason::OsStoreUnavailable,
        )
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn create_new(
        &self,
        _secret_ref: &SecretRef,
        _material: SecretMaterial,
    ) -> Result<(), SecretInternalError> {
        Err(Self::unavailable())
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn read(
        &self,
        _secret_ref: &SecretRef,
    ) -> Result<SecretMaterial, SecretInternalError> {
        Err(Self::unavailable())
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn delete(&self, _secret_ref: &SecretRef) -> Result<(), SecretInternalError> {
        Err(Self::unavailable())
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn validate_missing(
        &self,
        _secret_ref: &SecretRef,
    ) -> Result<(), SecretInternalError> {
        Err(Self::unavailable())
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    fn _todo_cred_ui_is_not_a_create_path() -> SecretInternalError {
        let _ = SecretSourceFreeErrorCode::WriteFailed;
        Self::unavailable()
    }
}
