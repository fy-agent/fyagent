//! Production capture UI is contract §10.2 (NSAlert + NSSecureTextField).
//!
//! objc2-app-kit currently only enables the `NSColor` feature and Cargo.toml
//! is frozen for Phase 2B, so this prompt is a documented stub. It is not
//! used by default tests. Do not treat this as source-freeze evidence that
//! the dialog shipped.

use crate::secret::{
    BeginCaptureIntent, SecretInternalError, SecretMaterial, SecretSourceFreeErrorCode,
    SecretTerminalOperationContext,
};

use super::CapturePrompt;

/// Production macOS secure prompt. Not wired to NSAlert in this phase.
pub(crate) struct MacOsSecurePrompt;

impl CapturePrompt for MacOsSecurePrompt {
    fn prompt_once(&self) -> Result<SecretMaterial, SecretInternalError> {
        Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::InputCancelled,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding),
        ))
    }
}
