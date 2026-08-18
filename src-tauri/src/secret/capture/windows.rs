//! Windows capture UI compile-gate for CredUIPromptForCredentialsW.
//!
//! Not executed on macOS hosts. Methods return unavailable.

use crate::secret::{
    SecretBackendUnavailableReason, SecretInternalError, SecretMaterial,
    SecretTerminalOperationContext,
};

use super::CapturePrompt;

pub(crate) struct WindowsCredentialPrompt;

impl CapturePrompt for WindowsCredentialPrompt {
    fn prompt_once(&self) -> Result<SecretMaterial, SecretInternalError> {
        Err(SecretInternalError::backend_unavailable(
            SecretTerminalOperationContext::Capture(
                crate::secret::BeginCaptureIntent::NewBinding,
            ),
            SecretBackendUnavailableReason::OsStoreUnavailable,
        ))
    }
}
