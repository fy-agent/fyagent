use std::{fmt, str::FromStr};

use serde::{de, Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::error::SecretServiceError;

const SECRET_REF_PREFIX: &str = "sec_";
const SECRET_VERSION_PREFIX: &str = "sv_";

fn parse_uuid_v4_simple<'a>(value: &'a str, prefix: &str) -> Result<&'a str, SecretServiceError> {
    let Some(simple) = value.strip_prefix(prefix) else {
        return Err(SecretServiceError::invalid_ref());
    };
    if simple.len() != 32
        || !simple
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || simple.as_bytes().get(12) != Some(&b'4')
        || !matches!(simple.as_bytes().get(16), Some(b'8' | b'9' | b'a' | b'b'))
    {
        return Err(SecretServiceError::invalid_ref());
    }
    Uuid::parse_str(simple).map_err(|_| SecretServiceError::invalid_ref())?;
    Ok(simple)
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub(crate) struct SecretRef(String);

impl SecretRef {
    pub(crate) fn generate() -> Self {
        Self(format!("{SECRET_REF_PREFIX}{}", Uuid::new_v4().simple()))
    }

    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SecretServiceError> {
        let value = value.into();
        parse_uuid_v4_simple(&value, SECRET_REF_PREFIX)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn display_ref(&self) -> String {
        format!("sec_…{}", &self.0[self.0.len() - 4..])
    }
}

impl fmt::Debug for SecretRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SecretRef")
            .field(&self.display_ref())
            .finish()
    }
}

impl FromStr for SecretRef {
    type Err = SecretServiceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for SecretRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub(crate) struct SecretVersion(String);

impl SecretVersion {
    pub(crate) fn generate() -> Self {
        Self(format!(
            "{SECRET_VERSION_PREFIX}{}",
            Uuid::new_v4().simple()
        ))
    }

    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SecretServiceError> {
        let value = value.into();
        parse_uuid_v4_simple(&value, SECRET_VERSION_PREFIX)?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretVersion(<opaque>)")
    }
}

impl<'de> Deserialize<'de> for SecretVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SecretHandle {
    secret_ref: SecretRef,
    version: SecretVersion,
}

impl SecretHandle {
    pub(crate) fn new(secret_ref: SecretRef, version: SecretVersion) -> Self {
        Self {
            secret_ref,
            version,
        }
    }

    pub(crate) fn secret_ref(&self) -> &SecretRef {
        &self.secret_ref
    }

    pub(crate) fn version(&self) -> &SecretVersion {
        &self.version
    }

    pub(crate) fn rotate(&self) -> Self {
        Self::new(self.secret_ref.clone(), SecretVersion::generate())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SecretPurpose {
    CodexApiKey,
    ManagedOAuthCredential,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SecretBackendKind {
    OsKeyring,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SecretPresence {
    Present,
    Missing,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SecretAvailability {
    Ready,
    Missing,
    Locked,
    Denied,
    Stale,
    Revoked,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BackendProbe {
    pub(crate) presence: SecretPresence,
    pub(crate) availability: SecretAvailability,
}

impl BackendProbe {
    pub(crate) const fn ready() -> Self {
        Self {
            presence: SecretPresence::Present,
            availability: SecretAvailability::Ready,
        }
    }

    pub(crate) const fn missing() -> Self {
        Self {
            presence: SecretPresence::Missing,
            availability: SecretAvailability::Missing,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretSummaryDto {
    schema_version: u8,
    secret_ref: SecretRef,
    secret_ref_display: String,
    version: SecretVersion,
    purpose: SecretPurpose,
    backend_kind: SecretBackendKind,
    presence: SecretPresence,
    availability: SecretAvailability,
}

impl SecretSummaryDto {
    pub(crate) fn from_probe(
        handle: &SecretHandle,
        purpose: SecretPurpose,
        backend_kind: SecretBackendKind,
        probe: BackendProbe,
    ) -> Self {
        Self {
            schema_version: 1,
            secret_ref: handle.secret_ref.clone(),
            secret_ref_display: handle.secret_ref.display_ref(),
            version: handle.version.clone(),
            purpose,
            backend_kind,
            presence: probe.presence,
            availability: probe.availability,
        }
    }

    pub(crate) fn handle(&self) -> SecretHandle {
        SecretHandle::new(self.secret_ref.clone(), self.version.clone())
    }

    pub(crate) fn presence(&self) -> SecretPresence {
        self.presence
    }

    pub(crate) fn availability(&self) -> SecretAvailability {
        self.availability
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretDeleteReceiptDto {
    schema_version: u8,
    secret_ref: SecretRef,
    deleted: bool,
}

impl SecretDeleteReceiptDto {
    pub(crate) fn deleted(secret_ref: SecretRef) -> Self {
        Self {
            schema_version: 1,
            secret_ref,
            deleted: true,
        }
    }
}
