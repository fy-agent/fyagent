use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::services::secret::{SecretMaterial, SecretPurpose, SecretVersion};

use super::{ManagedAuthCoreError, ManagedAuthProvider};

const SECRET_BUNDLE_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManagedSecretKind {
    RefreshToken,
    AccessToken,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawSecretBundle {
    schema_version: u8,
    credential_id: String,
    provider: ManagedAuthProvider,
    generation: u64,
    secret_version: String,
    secret_kind: ManagedSecretKind,
    token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawSecretBundleRef<'a> {
    schema_version: u8,
    credential_id: &'a str,
    provider: ManagedAuthProvider,
    generation: u64,
    secret_version: &'a str,
    secret_kind: ManagedSecretKind,
    token: &'a str,
}

/// Versioned native-store payload. The token is zeroized on drop and this
/// type intentionally has no `Debug`, `Clone`, or serde implementation.
pub(crate) struct ManagedAuthSecretBundle {
    credential_id: String,
    provider: ManagedAuthProvider,
    generation: u64,
    secret_version: SecretVersion,
    secret_kind: ManagedSecretKind,
    token: Zeroizing<String>,
}

impl ManagedAuthSecretBundle {
    pub(crate) fn new(
        credential_id: String,
        provider: ManagedAuthProvider,
        generation: u64,
        secret_version: SecretVersion,
        secret_kind: ManagedSecretKind,
        token: String,
    ) -> Result<Self, ManagedAuthCoreError> {
        if !valid_credential_id(&credential_id)
            || generation == 0
            || token.is_empty()
            || token.as_bytes().contains(&0)
        {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        Ok(Self {
            credential_id,
            provider,
            generation,
            secret_version,
            secret_kind,
            token: Zeroizing::new(token),
        })
    }

    pub(crate) fn credential_id(&self) -> &str {
        &self.credential_id
    }

    pub(crate) const fn provider(&self) -> ManagedAuthProvider {
        self.provider
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn secret_version(&self) -> &SecretVersion {
        &self.secret_version
    }

    pub(crate) const fn secret_kind(&self) -> ManagedSecretKind {
        self.secret_kind
    }

    pub(crate) fn token(&self) -> &str {
        self.token.as_str()
    }

    pub(crate) fn encode(mut self) -> Result<SecretMaterial, ManagedAuthCoreError> {
        let raw = RawSecretBundleRef {
            schema_version: SECRET_BUNDLE_SCHEMA_VERSION,
            credential_id: &self.credential_id,
            provider: self.provider,
            generation: self.generation,
            secret_version: self.secret_version.as_str(),
            secret_kind: self.secret_kind,
            token: self.token.as_str(),
        };
        let encoded = serde_json::to_vec(&raw).map_err(|_| ManagedAuthCoreError::InvalidData)?;
        self.token.zeroize();
        SecretMaterial::from_native_input(encoded, SecretPurpose::ManagedOAuthCredential)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, ManagedAuthCoreError> {
        let raw: RawSecretBundle =
            serde_json::from_slice(bytes).map_err(|_| ManagedAuthCoreError::InvalidData)?;
        if raw.schema_version != SECRET_BUNDLE_SCHEMA_VERSION {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        Self::new(
            raw.credential_id,
            raw.provider,
            raw.generation,
            SecretVersion::parse(raw.secret_version)
                .map_err(|_| ManagedAuthCoreError::InvalidData)?,
            raw.secret_kind,
            raw.token,
        )
    }
}

fn valid_credential_id(value: &str) -> bool {
    value.strip_prefix("mcred1:").is_some_and(|tail| {
        tail.len() == 32
            && tail
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use crate::services::secret::{DecodeSecret, MemorySecretBackend, SecretService};

    use super::*;

    #[test]
    fn bundle_round_trip_is_typed_and_debug_free() {
        let service = SecretService::new(MemorySecretBackend::new());
        let bundle = ManagedAuthSecretBundle::new(
            "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            ManagedAuthProvider::Openai,
            1,
            SecretVersion::generate(),
            ManagedSecretKind::RefreshToken,
            "test-refresh-value".to_string(),
        )
        .expect("bundle");
        let summary = service
            .create(
                bundle.encode().expect("encode"),
                SecretPurpose::ManagedOAuthCredential,
            )
            .expect("create");
        let decoded = service
            .with_material(
                &summary.handle(),
                SecretPurpose::ManagedOAuthCredential,
                DecodeSecret::new(ManagedAuthSecretBundle::decode),
            )
            .expect("read")
            .expect("decode");
        assert_eq!(decoded.provider(), ManagedAuthProvider::Openai);
        assert_eq!(decoded.generation(), 1);
        assert_eq!(decoded.secret_kind(), ManagedSecretKind::RefreshToken);
        assert_eq!(decoded.token(), "test-refresh-value");
    }

    #[test]
    fn bundle_rejects_wrong_schema_and_oversized_material() {
        let wrong = br#"{"schemaVersion":2,"credentialId":"mcred1:0123456789abcdef0123456789abcdef","provider":"openai","generation":1,"secretVersion":"sv_00000000000040008000000000000000","secretKind":"refresh_token","token":"x"}"#;
        assert!(ManagedAuthSecretBundle::decode(wrong).is_err());

        let oversized = "x".repeat(2_560);
        let bundle = ManagedAuthSecretBundle::new(
            "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            ManagedAuthProvider::Openai,
            1,
            SecretVersion::generate(),
            ManagedSecretKind::RefreshToken,
            oversized,
        )
        .expect("shape is valid before encoding");
        assert!(bundle.encode().is_err());
    }
}
