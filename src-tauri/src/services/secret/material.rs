use std::fmt;

use subtle::ConstantTimeEq;
#[cfg(test)]
use zeroize::Zeroize;
use zeroize::Zeroizing;

use super::{SecretPurpose, SecretServiceError};

/// Windows Credential Manager caps a generic credential blob at
/// `CRED_MAX_CREDENTIAL_BLOB_SIZE` (5 * 512 bytes). Keep one cross-platform
/// ceiling so callers never create a secret that can be persisted on macOS
/// but not on Windows.
pub(crate) const MAX_SECRET_BYTES: usize = 2_560;

pub(crate) struct SecretMaterial {
    bytes: Zeroizing<Vec<u8>>,
    purpose: SecretPurpose,
}

impl SecretMaterial {
    pub(crate) fn from_native_input(
        bytes: Vec<u8>,
        purpose: SecretPurpose,
    ) -> Result<Self, SecretServiceError> {
        let bytes = Zeroizing::new(bytes);
        if !matches!(
            purpose,
            SecretPurpose::CodexApiKey | SecretPurpose::ManagedOAuthCredential
        ) || bytes.is_empty()
            || bytes.len() > MAX_SECRET_BYTES
            || bytes.contains(&0)
            || std::str::from_utf8(bytes.as_slice()).is_err()
        {
            return Err(SecretServiceError::invalid_input());
        }
        Ok(Self { bytes, purpose })
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub(crate) const fn purpose(&self) -> SecretPurpose {
        self.purpose
    }

    pub(crate) fn ct_eq_slice(&self, other: &[u8]) -> bool {
        bool::from(self.bytes.as_slice().ct_eq(other))
    }

    #[cfg(test)]
    pub(crate) fn zeroize_now(&mut self) {
        self.bytes.zeroize();
    }
}

impl fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretMaterial([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_zeroize_clears_material_before_drop() {
        let mut material = SecretMaterial::from_native_input(
            b"runtime-canary-not-a-real-key".to_vec(),
            SecretPurpose::CodexApiKey,
        )
        .expect("valid material");
        material.zeroize_now();
        assert!(material.as_bytes().iter().all(|byte| *byte == 0));
    }
}
