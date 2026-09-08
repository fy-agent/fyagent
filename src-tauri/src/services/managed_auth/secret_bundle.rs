use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::services::secret::{SecretMaterial, SecretPurpose, MAX_SECRET_BYTES};

use super::{ManagedAuthCoreError, ManagedAuthProvider};

const SECRET_BUNDLE_SCHEMA_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawSecretBundle {
    schema_version: u8,
    credential_id: String,
    #[zeroize(skip)]
    provider: ManagedAuthProvider,
    generation: u64,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    token_type: Option<String>,
    #[zeroize(skip)]
    granted_scopes: Vec<String>,
    issued_at: Option<i64>,
    expires_at: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawSecretBundleRef<'a> {
    schema_version: u8,
    credential_id: &'a str,
    provider: ManagedAuthProvider,
    generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type: Option<&'a str>,
    granted_scopes: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    issued_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<i64>,
}

/// Versioned native-store payload. Token fields are zeroized on drop and this
/// type intentionally has no `Debug`, `Clone`, or serde implementation.
pub(crate) struct ManagedAuthSecretBundle {
    credential_id: String,
    provider: ManagedAuthProvider,
    generation: u64,
    access_token: Option<Zeroizing<String>>,
    refresh_token: Option<Zeroizing<String>>,
    id_token: Option<Zeroizing<String>>,
    token_type: Option<String>,
    granted_scopes: Vec<String>,
    issued_at: Option<i64>,
    expires_at: Option<i64>,
}

pub(crate) struct ManagedAuthSecretBundleParts {
    pub(crate) credential_id: String,
    pub(crate) provider: ManagedAuthProvider,
    pub(crate) generation: u64,
    pub(crate) access_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) id_token: Option<String>,
    pub(crate) token_type: Option<String>,
    pub(crate) granted_scopes: Vec<String>,
    pub(crate) issued_at: Option<i64>,
    pub(crate) expires_at: Option<i64>,
}

impl ManagedAuthSecretBundle {
    pub(crate) fn new(parts: ManagedAuthSecretBundleParts) -> Result<Self, ManagedAuthCoreError> {
        if !valid_credential_id(&parts.credential_id) || parts.generation == 0 {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        let access_token = optional_token(parts.access_token)?;
        let refresh_token = optional_token(parts.refresh_token)?;
        let id_token = optional_token(parts.id_token)?;
        if access_token.is_none() && refresh_token.is_none() && id_token.is_none() {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        Ok(Self {
            credential_id: parts.credential_id,
            provider: parts.provider,
            generation: parts.generation,
            access_token,
            refresh_token,
            id_token,
            token_type: parts.token_type.filter(|value| !value.is_empty()),
            granted_scopes: parts.granted_scopes,
            issued_at: parts.issued_at,
            expires_at: parts.expires_at,
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

    pub(crate) fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref().map(String::as_str)
    }

    pub(crate) fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref().map(String::as_str)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn id_token(&self) -> Option<&str> {
        self.id_token.as_deref().map(String::as_str)
    }

    pub(crate) fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_generation(mut self, generation: u64) -> Result<Self, ManagedAuthCoreError> {
        if generation == 0 {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        self.generation = generation;
        Ok(self)
    }

    fn encoded_bytes(&self) -> Result<Vec<u8>, ManagedAuthCoreError> {
        let raw = RawSecretBundleRef {
            schema_version: SECRET_BUNDLE_SCHEMA_VERSION,
            credential_id: &self.credential_id,
            provider: self.provider,
            generation: self.generation,
            access_token: self.access_token.as_deref().map(String::as_str),
            refresh_token: self.refresh_token.as_deref().map(String::as_str),
            id_token: self.id_token.as_deref().map(String::as_str),
            token_type: self.token_type.as_deref(),
            granted_scopes: &self.granted_scopes,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
        };
        serde_json::to_vec(&raw).map_err(|_| ManagedAuthCoreError::InvalidData)
    }

    pub(crate) fn encode(self) -> Result<SecretMaterial, ManagedAuthCoreError> {
        let mut bundle = self;
        let mut encoded = bundle.encoded_bytes()?;
        if encoded.len() > MAX_SECRET_BYTES
            && bundle.id_token.take().is_some()
            && (bundle.access_token.is_some() || bundle.refresh_token.is_some())
        {
            encoded = bundle.encoded_bytes()?;
        }
        if encoded.len() > MAX_SECRET_BYTES
            && bundle.access_token.take().is_some()
            && bundle.refresh_token.is_some()
        {
            encoded = bundle.encoded_bytes()?;
        }
        SecretMaterial::from_native_input(encoded, SecretPurpose::ManagedOAuthCredential)
            .map_err(ManagedAuthCoreError::from)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, ManagedAuthCoreError> {
        let raw: RawSecretBundle =
            serde_json::from_slice(bytes).map_err(|_| ManagedAuthCoreError::InvalidData)?;
        if raw.schema_version != SECRET_BUNDLE_SCHEMA_VERSION {
            return Err(ManagedAuthCoreError::InvalidData);
        }
        Self::new(ManagedAuthSecretBundleParts {
            credential_id: raw.credential_id.clone(),
            provider: raw.provider,
            generation: raw.generation,
            access_token: raw.access_token.clone(),
            refresh_token: raw.refresh_token.clone(),
            id_token: raw.id_token.clone(),
            token_type: raw.token_type.clone(),
            granted_scopes: raw.granted_scopes.clone(),
            issued_at: raw.issued_at,
            expires_at: raw.expires_at,
        })
    }
}

fn optional_token(
    value: Option<String>,
) -> Result<Option<Zeroizing<String>>, ManagedAuthCoreError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_empty() || value.as_bytes().contains(&0) {
        return Err(ManagedAuthCoreError::InvalidData);
    }
    Ok(Some(Zeroizing::new(value)))
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
        let production = include_str!("secret_bundle.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        assert!(
            !production.contains("#[derive(Debug"),
            "secret bundle types must not derive Debug"
        );
        assert!(
            !production.contains("#[derive(Clone"),
            "secret bundle types must not derive Clone"
        );
        let service = SecretService::new(MemorySecretBackend::new());
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            provider: ManagedAuthProvider::Openai,
            generation: 1,
            access_token: None,
            refresh_token: Some("test-refresh-value".to_string()),
            id_token: Some("id-token-value".to_string()),
            token_type: Some("Bearer".to_string()),
            granted_scopes: vec!["openid".to_string()],
            issued_at: Some(1_700_000_000),
            expires_at: None,
        })
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
        assert_eq!(decoded.refresh_token(), Some("test-refresh-value"));
        assert_eq!(decoded.id_token(), Some("id-token-value"));
        assert!(decoded.access_token().is_none());
        let rotated = decoded.with_generation(2).expect("generation");
        assert_eq!(rotated.generation(), 2);
        assert_eq!(rotated.id_token(), Some("id-token-value"));
    }

    #[test]
    fn bundle_rejects_wrong_schema_and_oversized_material() {
        let wrong = br#"{"schemaVersion":2,"credentialId":"mcred1:0123456789abcdef0123456789abcdef","provider":"openai","generation":1,"refreshToken":"x","grantedScopes":[]}"#;
        assert!(ManagedAuthSecretBundle::decode(wrong).is_err());

        let oversized = "x".repeat(2_560);
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            provider: ManagedAuthProvider::Openai,
            generation: 1,
            access_token: None,
            refresh_token: Some(oversized),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("shape is valid before encoding");
        assert!(bundle.encode().is_err());
    }

    #[test]
    fn encode_omits_id_token_when_openai_sized_grant_exceeds_limit() {
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            provider: ManagedAuthProvider::Openai,
            generation: 1,
            access_token: Some("a".repeat(1884)),
            refresh_token: Some("r".repeat(196)),
            id_token: Some("i".repeat(1858)),
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: Some(1_700_000_000),
            expires_at: None,
        })
        .expect("shape is valid before encoding");
        let encoded = bundle
            .encode()
            .expect("omit id token to fit Windows blob cap");
        assert!(encoded.as_bytes().len() <= MAX_SECRET_BYTES);
        let decoded = ManagedAuthSecretBundle::decode(encoded.as_bytes()).expect("decode");
        assert_eq!(decoded.access_token().map(str::len), Some(1884));
        assert_eq!(decoded.refresh_token().map(str::len), Some(196));
        assert!(decoded.id_token().is_none());
    }

    #[test]
    fn encode_omits_access_token_when_refresh_is_the_only_fitting_field() {
        let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            provider: ManagedAuthProvider::Openai,
            generation: 1,
            access_token: Some("a".repeat(2400)),
            refresh_token: Some("r".repeat(196)),
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .expect("shape is valid before encoding");
        let encoded = bundle
            .encode()
            .expect("omit access token when refresh still fits");
        assert!(encoded.as_bytes().len() <= MAX_SECRET_BYTES);
        let decoded = ManagedAuthSecretBundle::decode(encoded.as_bytes()).expect("decode");
        assert!(decoded.access_token().is_none());
        assert_eq!(decoded.refresh_token().map(str::len), Some(196));
    }

    #[test]
    fn bundle_requires_at_least_one_token() {
        assert!(ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
            credential_id: "mcred1:0123456789abcdef0123456789abcdef".to_string(),
            provider: ManagedAuthProvider::Xai,
            generation: 1,
            access_token: None,
            refresh_token: None,
            id_token: None,
            token_type: None,
            granted_scopes: Vec::new(),
            issued_at: None,
            expires_at: None,
        })
        .is_err());
    }
}
