use super::super::{
    BackendProbe, SecretBackend, SecretBackendKind, SecretMaterial, SecretPurpose, SecretRef,
    SecretServiceError,
};

/// Fail-closed development-host implementation for unsupported native hosts.
/// It deliberately provides no file or environment fallback.
pub(crate) struct UnavailableSecretBackend;

impl UnavailableSecretBackend {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl SecretBackend for UnavailableSecretBackend {
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::OsKeyring
    }

    fn create_new(
        &self,
        _secret_ref: &SecretRef,
        _material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        Err(SecretServiceError::backend_unavailable())
    }

    fn replace(
        &self,
        _secret_ref: &SecretRef,
        _material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        Err(SecretServiceError::backend_unavailable())
    }

    fn read(
        &self,
        _secret_ref: &SecretRef,
        _purpose: SecretPurpose,
    ) -> Result<SecretMaterial, SecretServiceError> {
        Err(SecretServiceError::backend_unavailable())
    }

    fn probe(&self, _secret_ref: &SecretRef) -> Result<BackendProbe, SecretServiceError> {
        Err(SecretServiceError::backend_unavailable())
    }

    fn delete(&self, _secret_ref: &SecretRef) -> Result<(), SecretServiceError> {
        Err(SecretServiceError::backend_unavailable())
    }
}
