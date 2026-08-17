use chrono::{DateTime, SecondsFormat, Utc};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, hash::Hash};
use uuid::{Variant, Version, Uuid};

const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;

// Pair with #[serde(default, deserialize_with = "deserialize_absent_only")].
// Missing field -> None through default; a present JSON null is passed here
// and rejected because T (not Option<T>) must deserialize.
fn deserialize_absent_only<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WireValidationError(&'static str);

impl fmt::Display for WireValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl fmt::Debug for WireValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WireValidationError(redacted)")
    }
}

impl std::error::Error for WireValidationError {}

fn valid_prefixed_uuid_v4(value: &str, prefix: &str) -> bool {
    let Some(raw) = value.strip_prefix(prefix) else {
        return false;
    };
    raw.len() == 32
        && raw.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && Uuid::parse_str(raw).is_ok_and(|uuid| {
            uuid.get_version() == Some(Version::Random)
                && uuid.get_variant() == Variant::RFC4122
                && uuid.simple().to_string() == raw
        })
}


fn valid_secret_ref_nibbles(raw: &str) -> bool {
    raw.as_bytes().get(12) == Some(&b'4')
        && raw
            .as_bytes()
            .get(16)
            .is_some_and(|b| matches!(*b, b'8' | b'9' | b'a' | b'b'))
}

fn valid_secret_ref(value: &str) -> bool {
    valid_prefixed_uuid_v4(value, "sec_")
        && value
            .strip_prefix("sec_")
            .is_some_and(valid_secret_ref_nibbles)
}

fn valid_change_plan_id(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|uuid| {
        uuid.get_version() == Some(Version::Random)
            && uuid.get_variant() == Variant::RFC4122
            && uuid.hyphenated().to_string() == value
    })
}

fn valid_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn contains_token_boundary_marker(value: &str, marker: &str) -> bool {
    value.match_indices(marker).any(|(index, _)| {
        index == 0
            || !value.as_bytes()[index - 1].is_ascii_alphanumeric()
    })
}

// Separator-insensitive canonical semantic keys. This is the sole Rust set;
// §12.3 is the sole source-spelling list from which it is generated.
const FORBIDDEN_SEMANTIC_FIELDS_V1: &[&str] = &[
    "secret",
    "secretvalue",
    "value",
    "apikey",
    "openaiapikey",
    "experimentalbearertoken",
    "token",
    "accesstoken",
    "refreshtoken",
    "authorization",
    "accesskey",
    "secretkey",
    "password",
    "credential",
    "privatekey",
    "credentialblob",
    "backendlocator",
    "rawerror",
    "rawmessage",
    "rawconfig",
    "providersettings",
    "livesettings",
    "absolutepath",
    "materialdigest",
];

fn canonical_semantic_key_ascii(value: &str) -> Option<String> {
    value.is_ascii().then(|| {
        value
            .bytes()
            .filter(|byte| byte.is_ascii_alphanumeric())
            .map(|byte| byte.to_ascii_lowercase() as char)
            .collect()
    })
}

// Exact mirror of CREDENTIAL_SEPARATOR_CODE_POINTS_V1. Do not replace this
// with char::is_whitespace/is_ascii_whitespace or a regex character class.
const CREDENTIAL_SEPARATOR_CODE_POINTS_V1: [char; 19] = [
    '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '#', '&', ',', '.', '/',
    ':', ';', '=', '?', '@', '\\', '\u{00a0}', '\u{2003}',
];

fn is_credential_separator_v1(value: char) -> bool {
    CREDENTIAL_SEPARATOR_CODE_POINTS_V1.contains(&value)
}

fn credential_shaped_token_stream(value: &str, unicode_boundary: bool) -> bool {
    let lower = value.to_ascii_lowercase();
    let has_forbidden_key = lower
        .split(|ch| {
            is_credential_separator_v1(ch)
                || (unicode_boundary && !ch.is_ascii())
        })
        .any(|part| {
            canonical_semantic_key_ascii(part).is_some_and(|compact| {
                FORBIDDEN_SEMANTIC_FIELDS_V1.contains(&compact.as_str())
                    || compact == "bearer"
            })
        });
    const MARKERS: &[&str] = &[
        "sk-",
        "ghp_",
        "github_pat_",
        "glpat-",
        "akia",
        "aiza",
        "ya29.",
        "npm_",
        "pypi-",
        "hf_",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "eyj",
        "bearer ",
        "bearer%20",
    ];
    has_forbidden_key
        || MARKERS
            .iter()
            .any(|marker| contains_token_boundary_marker(&lower, marker))
}

pub(in crate::secret) fn credential_shaped_ascii(value: &str) -> bool {
    !value.is_ascii() || credential_shaped_token_stream(value, false)
}

pub(in crate::secret) fn credential_shaped_display(value: &str) -> bool {
    credential_shaped_token_stream(value, true)
}

fn valid_owner_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|b| {
            b.is_ascii_alphanumeric() || matches!(*b, b'.' | b'_' | b':' | b'-')
        })
        && !credential_shaped_ascii(value)
}

fn valid_owner_namespace(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

fn valid_opaque_cursor(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|raw| {
        raw.len() == 32
            && raw
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn valid_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok_and(|parsed| {
        parsed.offset().local_minus_utc() == 0
            && parsed
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true)
                == value
    })
}

fn valid_safe_display(value: &str) -> bool {
    let count = value.chars().count();
    (1..=80).contains(&count)
        && value.trim() == value
        && !value.chars().any(char::is_control)
        && !value.starts_with('/')
        && !value.starts_with("\\\\")
        && !value.as_bytes().get(1).is_some_and(|b| *b == b':')
        && !credential_shaped_display(value)
}

fn valid_url(value: &str) -> bool {
    (1..=2048).contains(&value.len())
        && value.trim() == value
        && !value.chars().any(char::is_control)
        && url::Url::parse(value).is_ok_and(|parsed| {
            let path = parsed.path();
            let safe_path = path.len() <= 512
                && path.is_ascii()
                && !path.contains('%')
                && path.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'/' | b'.' | b'_' | b'~' | b'-')
                })
                && path
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .all(|segment| !credential_shaped_ascii(segment));
            matches!(parsed.scheme(), "http" | "https")
                && parsed.username().is_empty()
                && parsed.password().is_none()
                && parsed.query().is_none()
                && parsed.fragment().is_none()
                && parsed
                    .host_str()
                    .is_some_and(|host| {
                        host.split('.')
                            .all(|label| !credential_shaped_ascii(label))
                    })
                && safe_path
                && parsed.as_str() == value
        })
}

fn valid_codex_model_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && value.trim() == value
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
        && !credential_shaped_ascii(value)
}

macro_rules! validated_string_newtype {
    ($name:ident, $validate:expr, $label:literal) => {
        #[derive(Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: String) -> Result<Self, WireValidationError> {
                if ($validate)(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValidationError($label))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

macro_rules! revision_newtype {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub fn parse(value: u64) -> Result<Self, WireValidationError> {
                if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValidationError("invalid revision"))
                }
            }

            pub fn get(self) -> u64 {
                self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(u64::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

validated_string_newtype!(SecretRef, valid_secret_ref, "invalid secret ref");
validated_string_newtype!(SecretCandidateId, |v: &str| valid_prefixed_uuid_v4(v, "scd_"), "invalid candidate id");
validated_string_newtype!(SecretOperationId, |v: &str| valid_prefixed_uuid_v4(v, "sop_"), "invalid operation id");
validated_string_newtype!(SecretCommandId, |v: &str| valid_prefixed_uuid_v4(v, "scm_"), "invalid command id");
validated_string_newtype!(SecretAuditEventId, |v: &str| valid_prefixed_uuid_v4(v, "sae_"), "invalid audit id");
validated_string_newtype!(SecretConfirmationStepId, |v: &str| valid_prefixed_uuid_v4(v, "scs_"), "invalid step id");
validated_string_newtype!(SecretBackendInstanceId, |v: &str| valid_prefixed_uuid_v4(v, "sbi_"), "invalid backend instance id");
validated_string_newtype!(DeviceInstanceId, |v: &str| valid_prefixed_uuid_v4(v, "dev_"), "invalid durable device instance id");
validated_string_newtype!(SecretRecoveryId, |v: &str| valid_prefixed_uuid_v4(v, "src_"), "invalid recovery id");
validated_string_newtype!(SecretCaptureIntentId, |v: &str| valid_prefixed_uuid_v4(v, "sci_"), "invalid capture intent id");
validated_string_newtype!(ImportStageId, |v: &str| valid_prefixed_uuid_v4(v, "ist_"), "invalid import stage id");
validated_string_newtype!(ProviderDeleteImpactId, |v: &str| valid_prefixed_uuid_v4(v, "pdi_"), "invalid Provider delete impact id");
validated_string_newtype!(SecretMigrationReportId, |v: &str| valid_prefixed_uuid_v4(v, "smr_"), "invalid report id");
validated_string_newtype!(LegacySourceLocationId, |v: &str| valid_opaque_cursor(v, "lsl_"), "invalid legacy source location id");
validated_string_newtype!(SecretSummaryCursor, |v: &str| valid_opaque_cursor(v, "ssc_"), "invalid summary cursor");
validated_string_newtype!(SecretAuditCursor, |v: &str| valid_opaque_cursor(v, "sac_"), "invalid audit cursor");
validated_string_newtype!(ChangePlanId, valid_change_plan_id, "invalid change plan id");
validated_string_newtype!(ChangePlanDigest, valid_hex_64, "invalid change plan digest");
validated_string_newtype!(BindingSetDigest, valid_hex_64, "invalid binding set digest");
validated_string_newtype!(SecretRecoveryDigest, valid_hex_64, "invalid recovery digest");
validated_string_newtype!(SecretProjectionDigest, valid_hex_64, "invalid projection digest");
validated_string_newtype!(RecoveryStructureDigest, valid_hex_64, "invalid recovery structure digest");
validated_string_newtype!(StagedImportResumeDigest, valid_hex_64, "invalid staged import resume digest");
validated_string_newtype!(OwnerId, valid_owner_id, "invalid owner id");
validated_string_newtype!(SecretOwnerNamespace, valid_owner_namespace, "invalid owner namespace");
validated_string_newtype!(SafeDisplayText, valid_safe_display, "invalid display text");
validated_string_newtype!(UtcTimestamp, valid_timestamp, "invalid UTC timestamp");
validated_string_newtype!(ValidatedUrl, valid_url, "invalid URL");
validated_string_newtype!(CodexModelId, valid_codex_model_id, "invalid Codex model id");
validated_string_newtype!(CodexModelProviderId, valid_codex_model_id, "invalid Codex model provider id");

revision_newtype!(SecretRecordRevision);
revision_newtype!(SecretCandidateRevision);
revision_newtype!(SecretBindingRevision);
revision_newtype!(SecretOwnerBindingRevision);
revision_newtype!(SecretBindingSetRevision);
revision_newtype!(SecretRecoveryRevision);
revision_newtype!(StagedImportResumeRevision);
revision_newtype!(StagedRowRevision);
revision_newtype!(ProviderRowRevision);
revision_newtype!(LegacySourceStructuralRevision);
revision_newtype!(CodexLiveStructuralRevision);
revision_newtype!(SecretBackendGeneration);
revision_newtype!(DeviceBindingGeneration);
revision_newtype!(CapabilityRevision);

macro_rules! generate_prefixed_uuid_v4 {
    ($name:ident, $prefix:literal) => {
        impl $name {
            pub fn generate() -> Self {
                Self(format!("{}{}", $prefix, Uuid::new_v4().simple()))
            }
        }
    };
}

generate_prefixed_uuid_v4!(SecretRef, "sec_");
generate_prefixed_uuid_v4!(SecretCandidateId, "scd_");
generate_prefixed_uuid_v4!(SecretOperationId, "sop_");
generate_prefixed_uuid_v4!(SecretCommandId, "scm_");
generate_prefixed_uuid_v4!(SecretAuditEventId, "sae_");
generate_prefixed_uuid_v4!(SecretConfirmationStepId, "scs_");
generate_prefixed_uuid_v4!(SecretBackendInstanceId, "sbi_");
generate_prefixed_uuid_v4!(DeviceInstanceId, "dev_");
generate_prefixed_uuid_v4!(SecretRecoveryId, "src_");
generate_prefixed_uuid_v4!(SecretCaptureIntentId, "sci_");

impl TryFrom<&str> for SecretRef {
    type Error = WireValidationError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl TryFrom<String> for SecretRef {
    type Error = WireValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for DeviceInstanceId {
    type Error = WireValidationError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl TryFrom<String> for DeviceInstanceId {
    type Error = WireValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}


#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PageLimit(u8);

impl PageLimit {
    pub fn parse(value: u8) -> Result<Self, WireValidationError> {
        if (1..=100).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WireValidationError("page limit must be 1..=100"))
        }
    }
}

impl<'de> Deserialize<'de> for PageLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(u8::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ConfirmationTimeoutSeconds(u16);

impl ConfirmationTimeoutSeconds {
    pub fn parse(value: u16) -> Result<Self, WireValidationError> {
        if (1..=300).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WireValidationError("confirmation timeout must be 1..=300"))
        }
    }
}

impl<'de> Deserialize<'de> for ConfirmationTimeoutSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(u16::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersionV1;

impl Serialize for SchemaVersionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(1)
    }
}

impl<'de> Deserialize<'de> for SchemaVersionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(Self),
            _ => Err(de::Error::custom("schemaVersion must be 1")),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SecretRefDisplay(String);

impl SecretRefDisplay {
    pub(crate) fn derive_from(secret_ref: &SecretRef) -> Self {
        let value = secret_ref.as_str();
        Self(format!("sec_…{}", &value[value.len() - 4..]))
    }
}

// Output-only: no Deserialize/FromStr/TryFrom<String> implementation exists.
// Request DTOs accept SecretRef and derive this display only after authority
// lookup; response decoders verify it against the adjacent authoritative ref.

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AlwaysFalse;

impl Serialize for AlwaysFalse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for AlwaysFalse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            false => Ok(Self),
            true => Err(de::Error::custom("must be false")),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AlwaysTrue;

impl Serialize for AlwaysTrue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for AlwaysTrue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            true => Ok(Self),
            false => Err(de::Error::custom("must be true")),
        }
    }
}

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum $name { $($variant),+ }
    };
}

wire_enum!(SecretOwnerKind { Provider, Agent });
wire_enum!(SecretPurpose { CodexApiKey });
wire_enum!(SecretSlot { PrimaryApiKey });
wire_enum!(SecretBackendKind { OsKeyring, Hardware });
wire_enum!(SecretBackendAvailability { Available, Unavailable });
wire_enum!(SecretPresence { Present, Missing, Unknown });
wire_enum!(SecretStableAvailability {
    Ready, Missing, Locked, Denied, Stale, Revoked, Unavailable
});
wire_enum!(SecretLockSource { FyAgentPolicy, Backend });
wire_enum!(SecretRevocationSource {
    UserDelete, CentralBackend, DeviceAdministration, SupersededByRotation
});
wire_enum!(BackendObservedRevocationSource { CentralBackend, DeviceAdministration });
wire_enum!(SecretBackendUnavailableReason {
    HardwareUnregistered, HardwareDisconnected, OsStoreUnavailable,
    CentralServiceUnavailable
});
wire_enum!(SecretRecoveryKind {
    ActivationCleanup, CaptureCompensation, DeleteFinalization,
    OwnerDetachFinalization
});
wire_enum!(DeviceBinding { HostUser, HardwareDevice });
wire_enum!(PhysicalConfirmation { Never, Optional, Required });
wire_enum!(StorageResidency { OsProtectedStore, HardwareOnly });
wire_enum!(SecretConsumer {
    ChangePlanApply, ProxyRequest, UsageProbe, CodingPlanUsageProbe,
    ModelFetch, ProviderTerminal
});
wire_enum!(ApplyTargetSink {
    ProcessMemory, ExternalConfigFile, ChildProcessEnvironment
});
wire_enum!(SecretRuntimeConsumer {
    ChangePlanApply, ProxyRequest, UsageProbe, CodingPlanUsageProbe, ModelFetch
});
wire_enum!(SecretRuntimeSink { ProcessMemory, ExternalConfigFile });
wire_enum!(SecretChangePlanApplyConsumer { ChangePlanApply });
wire_enum!(SecretChangePlanApplySink { ExternalConfigFile });
wire_enum!(CodexLiveSecretSinkId {
    CodexAuthJsonOpenAiApiKey,
    CodexConfigTomlExperimentalBearerToken
});
wire_enum!(SecretBackendOperation {
    CaptureVerify, Validate, ResolveForApply, Delete, Revoke
});
// There is no MissingReadback operation variant. All five typed missing-
// readback scopes map to Validate while retaining distinct slots, consuming
// authorizations and durable checkpoints.
wire_enum!(SecretCandidateKind {
    NewBinding, ReplaceBinding, RotateBindingSet, LegacyReconcile,
    LegacyScrubExistingBinding
});
wire_enum!(SecretCandidateState {
    VerifiedPendingPlan, Activated, Discarded, CleanupRequired, Expired
});
wire_enum!(LegacyActivationComparisonPolicy { CandidateEquality, ExplicitReplacement });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "policy",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LegacyActivationComparisonImpact {
    CandidateEquality {
        user_meaning: VerifySameValueMigrationMeaning,
    },
    ExplicitReplacement {
        user_meaning: ReplaceExistingCredentialMeaning,
        affected_source_count: u32,
        replaces_bound_binding: bool,
    },
}

wire_enum!(VerifySameValueMigrationMeaning { VerifySameValueMigration });
wire_enum!(ReplaceExistingCredentialMeaning { ReplaceExistingCredential });
wire_enum!(LegacySourceCategory {
    ProviderAuthJson, ProviderConfigTomlTopLevel,
    ProviderConfigTomlActiveTable, ProviderConfigTomlInactiveTable,
    ProviderConfigTomlInlineTable, ProviderUsageScriptApiKey,
    ProviderNonCanonicalProxyAlias
});
wire_enum!(LegacySourceOrigin {
    ProviderRow, LiveAuth, LiveConfig, SqlImportStaging,
    DbRestoreStaging, SyncDownloadStaging
});
wire_enum!(LegacyOwnerState {
    SingleValuePending, SourcesConflict, SourceInvalid, BindingComparisonPending,
    BindingConflict, ApprovalRequired
});

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySourceRef {
    pub location_id: LegacySourceLocationId,
    pub category: LegacySourceCategory,
    pub origin: LegacySourceOrigin,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SupplementalLegacySourceCategory {
    ProcessEnvironment,
    WindowsRegistryCurrentUser,
    WindowsRegistryLocalMachine,
    ShellStartupFile,
    CommonConfigJson,
    CommonConfigBackup,
    CommonConfigMigrated,
    CommonConfigSqlite,
    RendererLocalStorage,
    LiveConfigMerge,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum AdjacentBlockedLegacySourceObservationState {
    AdjacentBlocked,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentBlockedLegacySourceObservation {
    state: AdjacentBlockedLegacySourceObservationState,
    category: SupplementalLegacySourceCategory,
}

impl AdjacentBlockedLegacySourceObservation {
    pub(crate) fn checked_from_codex_inventory_bridge(
        category: SupplementalLegacySourceCategory,
    ) -> Self {
        Self {
            state: AdjacentBlockedLegacySourceObservationState::AdjacentBlocked,
            category,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySourceExpectation {
    pub source: LegacySourceRef,
    pub structural_revision: LegacySourceStructuralRevision,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CurrentLegacySourceExpectations(Vec<LegacySourceExpectation>);

impl CurrentLegacySourceExpectations {
    fn validate(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        let current_only = values.iter().all(|expectation| {
            matches!(
                expectation.source.origin,
                LegacySourceOrigin::ProviderRow
                    | LegacySourceOrigin::LiveAuth
                    | LegacySourceOrigin::LiveConfig
            )
        });
        let sorted_unique = values.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0].source)
                < legacy_source_sort_key(&pair[1].source)
        });
        if current_only && sorted_unique {
            Ok(Self(values))
        } else {
            Err(WireValidationError(
                "legacy scrub expectations must be current/sorted/unique",
            ))
        }
    }

    pub(in crate::secret) fn as_slice(&self) -> &[LegacySourceExpectation] {
        &self.0
    }

    pub(crate) fn checked_from_codex_inventory_bridge(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        Self::validate(values)
    }
}

impl<'de> Deserialize<'de> for CurrentLegacySourceExpectations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate(Vec::<LegacySourceExpectation>::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CurrentScrubbableLegacySourceCoverageView {
    None {
        source_count: u32,
        categories: [LegacySourceCategory; 0],
    },
    CurrentSourcesPresent {
        source_count: u32,
        categories: Vec<LegacySourceCategory>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum AdjacentBlockedLegacySourceCoverageView {
    None {
        observation_count: u32,
        observations: [AdjacentBlockedLegacySourceObservation; 0],
    },
    AdjacentBlockedSourcesPresent {
        observation_count: u32,
        observations: Vec<AdjacentBlockedLegacySourceObservation>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum LegacySourceCoverageRepr {
    Clear {
        current_scrubbable: CurrentScrubbableLegacySourceCoverageView,
        adjacent_blocked: AdjacentBlockedLegacySourceCoverageView,
    },
    BlockingSourcesPresent {
        current_scrubbable: CurrentScrubbableLegacySourceCoverageView,
        adjacent_blocked: AdjacentBlockedLegacySourceCoverageView,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct LegacySourceCoverageView(LegacySourceCoverageRepr);

impl Serialize for LegacySourceCoverageView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

// The following bridge/identity/authority types live in the crate-root
// main-integration module `crate::legacy_source_inventory`. The module is
// declared `pub(crate)` so `crate::store`, `crate::commands::provider` and the
// #35 secret module can name its opaque types. All fields and all authority /
// identity constructors remain private to that module.
#[derive(PartialEq, Eq)]
pub(crate) struct LegacySourceInventoryRevision(u64);

impl LegacySourceInventoryRevision {
    fn checked_from_structural_generation(
        revision: u64,
    ) -> Result<Self, SecretInternalError> {
        if revision == 0 || revision > 9_007_199_254_740_991 {
            return Err(todo!("checked internal invariant error"));
        }
        // Scanner permits the call only on structural-generation metadata and
        // rejects source bytes, values or digests as the argument origin.
        Ok(Self(revision))
    }
}

#[derive(PartialEq, Eq)]
enum LegacySourceDomainPresence {
    Absent,
    Present,
}

#[derive(PartialEq, Eq)]
struct LegacySourceDomainCoverageIdentity {
    structural_revision: LegacySourceInventoryRevision,
    presence: LegacySourceDomainPresence,
    source_count: u32,
}

impl LegacySourceDomainCoverageIdentity {
    fn checked_from_structural_inventory(
        structural_revision: LegacySourceInventoryRevision,
        presence: LegacySourceDomainPresence,
        source_count: u32,
    ) -> Result<Self, SecretInternalError> {
        let coherent = match presence {
            LegacySourceDomainPresence::Absent => source_count == 0,
            LegacySourceDomainPresence::Present => source_count > 0,
        };
        if !coherent {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(Self {
            structural_revision,
            presence,
            source_count,
        })
    }
}

// Fixed named fields make omission, duplication, extension and category
// substitution unrepresentable. This identity never stores a LegacySourceRef,
// location id, path, source value, value-derived revision or value digest.
#[derive(PartialEq, Eq)]
pub(crate) struct CompleteLegacySourceCoverageIdentity {
    current_provider_live_scrubbable: LegacySourceDomainCoverageIdentity,
    process_environment: LegacySourceDomainCoverageIdentity,
    windows_registry_current_user: LegacySourceDomainCoverageIdentity,
    windows_registry_local_machine: LegacySourceDomainCoverageIdentity,
    shell_startup_file: LegacySourceDomainCoverageIdentity,
    common_config_json: LegacySourceDomainCoverageIdentity,
    common_config_backup: LegacySourceDomainCoverageIdentity,
    common_config_migrated: LegacySourceDomainCoverageIdentity,
    common_config_sqlite: LegacySourceDomainCoverageIdentity,
    renderer_local_storage: LegacySourceDomainCoverageIdentity,
    live_config_merge: LegacySourceDomainCoverageIdentity,
}

impl CompleteLegacySourceCoverageIdentity {
    fn checked_exact_eleven_domains(
        current_provider_live_scrubbable: LegacySourceDomainCoverageIdentity,
        process_environment: LegacySourceDomainCoverageIdentity,
        windows_registry_current_user: LegacySourceDomainCoverageIdentity,
        windows_registry_local_machine: LegacySourceDomainCoverageIdentity,
        shell_startup_file: LegacySourceDomainCoverageIdentity,
        common_config_json: LegacySourceDomainCoverageIdentity,
        common_config_backup: LegacySourceDomainCoverageIdentity,
        common_config_migrated: LegacySourceDomainCoverageIdentity,
        common_config_sqlite: LegacySourceDomainCoverageIdentity,
        renderer_local_storage: LegacySourceDomainCoverageIdentity,
        live_config_merge: LegacySourceDomainCoverageIdentity,
    ) -> Result<Self, SecretInternalError> {
        Ok(Self {
            current_provider_live_scrubbable,
            process_environment,
            windows_registry_current_user,
            windows_registry_local_machine,
            shell_startup_file,
            common_config_json,
            common_config_backup,
            common_config_migrated,
            common_config_sqlite,
            renderer_local_storage,
            live_config_merge,
        })
    }

    pub(crate) fn all_domains_absent(&self) -> bool {
        [
            &self.current_provider_live_scrubbable,
            &self.process_environment,
            &self.windows_registry_current_user,
            &self.windows_registry_local_machine,
            &self.shell_startup_file,
            &self.common_config_json,
            &self.common_config_backup,
            &self.common_config_migrated,
            &self.common_config_sqlite,
            &self.renderer_local_storage,
            &self.live_config_merge,
        ]
        .into_iter()
        .all(|domain| {
            matches!(domain.presence, LegacySourceDomainPresence::Absent)
                && domain.source_count == 0
        })
    }
}

pub(crate) struct CompleteLegacySourceInventoryAuthority {
    inventory_revision: LegacySourceInventoryRevision,
    coverage_identity: CompleteLegacySourceCoverageIdentity,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

impl CompleteLegacySourceInventoryAuthority {
    fn checked_from_bridge(
        inventory_revision: LegacySourceInventoryRevision,
        coverage_identity: CompleteLegacySourceCoverageIdentity,
        current_scrubbable: CurrentLegacySourceExpectations,
        adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
    ) -> Result<Self, SecretInternalError> {
        Ok(Self {
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        })
    }

    // Scanner-allowlisted only from
    // LegacySourceCoverageReceipt::checked_from_complete_inventory_authority.
    // The bridge never returns this authority to any sibling module.
    pub(crate) fn into_secret_checked_parts(
        self,
    ) -> (
        LegacySourceInventoryRevision,
        CompleteLegacySourceCoverageIdentity,
        CurrentLegacySourceExpectations,
        Vec<AdjacentBlockedLegacySourceObservation>,
    ) {
        (
            self.inventory_revision,
            self.coverage_identity,
            self.current_scrubbable,
            self.adjacent_blocked,
        )
    }
}

enum FreshLegacySourceInventoryTarget<'a> {
    Startup,
    OwnerSummary(&'a ExistingSecretOwnerToken),
    Capture(&'a ExistingSecretOwnerToken),
    ProviderDelete {
        owner: &'a ExistingSecretOwnerToken,
        provider_row_revision: &'a ProviderRowRevision,
    },
}

struct CodexLegacySourceStructuralInventoryPorts<'a> {
    _borrowed_main_integration_authorities: std::marker::PhantomData<&'a mut ()>,
}

struct FreshLegacySourceDomainInventory {
    structural_generation: u64,
    source_count: u32,
}

impl FreshLegacySourceDomainInventory {
    fn into_coverage_identity(
        self,
    ) -> Result<LegacySourceDomainCoverageIdentity, SecretInternalError> {
        let presence = if self.source_count == 0 {
            LegacySourceDomainPresence::Absent
        } else {
            LegacySourceDomainPresence::Present
        };
        LegacySourceDomainCoverageIdentity::checked_from_structural_inventory(
            LegacySourceInventoryRevision::checked_from_structural_generation(
                self.structural_generation,
            )?,
            presence,
            self.source_count,
        )
    }
}

struct FreshCompleteLegacySourceInventory {
    inventory_generation: u64,
    current_provider_live_scrubbable: FreshLegacySourceDomainInventory,
    process_environment: FreshLegacySourceDomainInventory,
    windows_registry_current_user: FreshLegacySourceDomainInventory,
    windows_registry_local_machine: FreshLegacySourceDomainInventory,
    shell_startup_file: FreshLegacySourceDomainInventory,
    common_config_json: FreshLegacySourceDomainInventory,
    common_config_backup: FreshLegacySourceDomainInventory,
    common_config_migrated: FreshLegacySourceDomainInventory,
    common_config_sqlite: FreshLegacySourceDomainInventory,
    renderer_local_storage: FreshLegacySourceDomainInventory,
    live_config_merge: FreshLegacySourceDomainInventory,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

pub(crate) struct CodexLegacySourceInventoryBridge<'a> {
    ports: CodexLegacySourceStructuralInventoryPorts<'a>,
}

impl<'a> CodexLegacySourceInventoryBridge<'a> {
    // The factory binds the existing AppState/DB, Provider/live configuration,
    // process/OS/file/common-config and renderer-storage structural adapters.
    // No caller may inject a source list, path, locator, value or digest.
    pub(crate) fn from_app_state(
        state: &'a crate::store::AppState,
    ) -> Result<Self, SecretInternalError> {
        let _ = state;
        Err(SecretInternalError::input_invalid())
    }

    pub(crate) fn fresh_startup_coverage(
        &mut self,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(FreshLegacySourceInventoryTarget::Startup)
    }

    pub(crate) fn fresh_owner_summary_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::OwnerSummary(owner),
        )
    }

    pub(crate) fn fresh_capture_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::Capture(owner),
        )
    }

    pub(crate) fn fresh_provider_delete_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
        provider_row_revision: &ProviderRowRevision,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::ProviderDelete {
                owner,
                provider_row_revision,
            },
        )
    }

    fn fresh_complete_coverage(
        &mut self,
        target: FreshLegacySourceInventoryTarget<'_>,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        // `collect_complete_inventory_authority` performs one fresh read of
        // all eleven fixed domains. It privately constructs the complete
        // identity and authority; no partial Vec/map is an accepted input.
        let authority = self.collect_complete_inventory_authority(target)?;
        LegacySourceCoverageReceipt::checked_from_complete_inventory_authority(
            authority,
        )
    }

    fn collect_complete_inventory_authority(
        &mut self,
        target: FreshLegacySourceInventoryTarget<'_>,
    ) -> Result<CompleteLegacySourceInventoryAuthority, SecretInternalError> {
        let inventory: FreshCompleteLegacySourceInventory = {
            let _ = (&mut self.ports, target);
            todo!("one fresh complete pass over the fixed eleven structural adapters with before/after generation fencing and drift rejection; output has only structural generations/counts, typed current expectations and category-only adjacent observations")
        };
        let FreshCompleteLegacySourceInventory {
            inventory_generation,
            current_provider_live_scrubbable,
            process_environment,
            windows_registry_current_user,
            windows_registry_local_machine,
            shell_startup_file,
            common_config_json,
            common_config_backup,
            common_config_migrated,
            common_config_sqlite,
            renderer_local_storage,
            live_config_merge,
            current_scrubbable,
            adjacent_blocked,
        } = inventory;
        let inventory_revision =
            LegacySourceInventoryRevision::checked_from_structural_generation(
                inventory_generation,
            )?;
        let coverage_identity =
            CompleteLegacySourceCoverageIdentity::checked_exact_eleven_domains(
                current_provider_live_scrubbable.into_coverage_identity()?,
                process_environment.into_coverage_identity()?,
                windows_registry_current_user.into_coverage_identity()?,
                windows_registry_local_machine.into_coverage_identity()?,
                shell_startup_file.into_coverage_identity()?,
                common_config_json.into_coverage_identity()?,
                common_config_backup.into_coverage_identity()?,
                common_config_migrated.into_coverage_identity()?,
                common_config_sqlite.into_coverage_identity()?,
                renderer_local_storage.into_coverage_identity()?,
                live_config_merge.into_coverage_identity()?,
            )?;
        CompleteLegacySourceInventoryAuthority::checked_from_bridge(
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        )
    }
}

// Opaque, no-value inventory receipt owned by the private child module
// crate::secret::legacy_source_coverage and re-exported pub(crate) by
// crate::secret. Store/Provider/other secret siblings can name, move and
// consume it but cannot access its fields. It implements no
// Clone/Serialize/Deserialize/Debug/Default.
// The exact fields are one non-value-derived inventory revision, one complete
// eleven-domain identity, current expectations and adjacent observations.
// Only current_scrubbable may retain exact current LegacySourceRef
// expectations, including its typed non-value-derived LegacySourceLocationId;
// no raw locator/path/value/value-derived digest is retained. adjacent_blocked
// retains category/state observations only and can never authorize
// parse/read/compare/scrub.
pub(crate) struct LegacySourceCoverageReceipt {
    inventory_revision: LegacySourceInventoryRevision,
    coverage_identity: CompleteLegacySourceCoverageIdentity,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

impl LegacySourceCoverageReceipt {
    fn validate_complete_parts(
        inventory_revision: &LegacySourceInventoryRevision,
        coverage_identity: &CompleteLegacySourceCoverageIdentity,
        current_scrubbable: &CurrentLegacySourceExpectations,
        adjacent_blocked: &[AdjacentBlockedLegacySourceObservation],
    ) -> Result<(), SecretInternalError> {
        let _ = (
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        );
        todo!("current count/presence equals exact current expectations; each supplemental count/presence equals its canonical category-only observations; inventory/domain revisions are positive structural generations")
    }

    pub(crate) fn checked_from_complete_inventory_authority(
        authority: CompleteLegacySourceInventoryAuthority,
    ) -> Result<Self, SecretInternalError> {
        let (
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        ) = authority.into_secret_checked_parts();
        Self::validate_complete_parts(
            &inventory_revision,
            &coverage_identity,
            &current_scrubbable,
            &adjacent_blocked,
        )?;
        Ok(Self {
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        })
    }

    pub(crate) fn assert_complete_clear(
        &self,
    ) -> Result<(), SecretInternalError> {
        if !self.coverage_identity.all_domains_absent()
            || !self.current_scrubbable.as_slice().is_empty()
            || !self.adjacent_blocked.is_empty()
        {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(())
    }

    pub(crate) fn assert_complete(
        &self,
    ) -> Result<(), SecretInternalError> {
        Self::validate_complete_parts(
            &self.inventory_revision,
            &self.coverage_identity,
            &self.current_scrubbable,
            &self.adjacent_blocked,
        )
    }

    pub(crate) fn assert_complete_blocking(
        &self,
    ) -> Result<(), SecretInternalError> {
        if self.coverage_identity.all_domains_absent()
            || (self.current_scrubbable.as_slice().is_empty()
                && self.adjacent_blocked.is_empty())
        {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(())
    }

    pub(crate) fn assert_same_complete_coverage_as(
        &self,
        expected: &LegacySourceCoverageReceipt,
    ) -> Result<(), SecretInternalError> {
        if self.inventory_revision != expected.inventory_revision
            || self.coverage_identity != expected.coverage_identity
            || self.current_scrubbable != expected.current_scrubbable
            || self.adjacent_blocked != expected.adjacent_blocked
        {
            return Err(todo!("checked stale-coverage error"));
        }
        Ok(())
    }

}

impl LegacySourceCoverageView {
    pub(crate) fn checked_from_coverage_receipt(
        receipt: &LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        let current_empty = receipt.current_scrubbable.as_slice().is_empty();
        let adjacent_empty = receipt.adjacent_blocked.is_empty();
        let all_absent = receipt.coverage_identity.all_domains_absent();
        let current_scrubbable = if current_empty {
            CurrentScrubbableLegacySourceCoverageView::None {
                source_count: 0,
                categories: [],
            }
        } else {
            let categories: Vec<LegacySourceCategory> = receipt
                .current_scrubbable
                .as_slice()
                .iter()
                .map(|expectation| expectation.source.category)
                .collect();
            CurrentScrubbableLegacySourceCoverageView::CurrentSourcesPresent {
                source_count: u32::try_from(categories.len())
                    .map_err(|_| SecretInternalError::input_invalid())?,
                categories,
            }
        };
        let adjacent_blocked = if adjacent_empty {
            AdjacentBlockedLegacySourceCoverageView::None {
                observation_count: 0,
                observations: [],
            }
        } else {
            AdjacentBlockedLegacySourceCoverageView::AdjacentBlockedSourcesPresent {
                observation_count: u32::try_from(receipt.adjacent_blocked.len())
                    .map_err(|_| SecretInternalError::input_invalid())?,
                observations: receipt.adjacent_blocked.clone(),
            }
        };
        let repr = match (current_empty, adjacent_empty, all_absent) {
            (true, true, true) => LegacySourceCoverageRepr::Clear {
                current_scrubbable,
                adjacent_blocked,
            },
            (false, _, false) | (_, false, false) => {
                LegacySourceCoverageRepr::BlockingSourcesPresent {
                    current_scrubbable,
                    adjacent_blocked,
                }
            }
            _ => return Err(SecretInternalError::input_invalid()),
        };
        Ok(Self(repr))
    }
}

// LegacySourceInventoryRevision, CompleteLegacySourceCoverageIdentity,
// CompleteLegacySourceInventoryAuthority, CodexLegacySourceInventoryBridge
// and LegacySourceCoverageReceipt implement no Clone/Serialize/Deserialize/
// Debug/Default. The revision constructors accept structural-generation
// counters only and reject zero; source bytes and value-derived hashes are not
// in their type surface. A LegacySourceRef is current-scrubbable; an
// AdjacentBlockedLegacySourceObservation is supplemental and never one.
wire_enum!(SecretAuditAction {
    CaptureCandidate, DiscardCandidate, ActivateCandidate, Validate,
    RotateCandidate, Lock, Unlock, Delete, Revoke, CheckReadiness,
    PrepareApply, ConfirmHardware, ResolveApply, MigrateLegacy,
    ReconcileLegacy, ReconcileRecovery, RetryCleanup, CancelConfirmation
});
wire_enum!(SecretAuditOutcome { Success, Blocked, Failed, Partial, Recovered });
wire_enum!(SecretEffect {
    None, CandidateStaged, BindingChanged, PolicyChanged, RecordRevoked,
    TargetWriterInvoked, CleanupPending
});
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretUserAction {
    None,
    RetryCapture,
    RetryRotation,
    RetryProxyRequest,
    RetryUsageProbe,
    RetryCodingPlanUsageProbe,
    RetryModelFetch,
    UnlockFyAgent,
    UnlockBackend,
    RequestPermission,
    CaptureReplacement,
    ChooseBackend,
    ConfirmDevice,
    RefreshSummary,
    RefreshDeleteImpact,
    RefreshRecoveryImpact,
    ReopenChangePlan,
    ResolveLegacyConflict,
    DiscardCandidate,
    CompleteRecovery,
    ResumeStagedImportCutover,
    ReconnectDevice,
    OpenBackendSettings,
    ContactAdministrator,
}

// Internal-only discriminator used by the checked error/issue factory when
// one stable code has more than one executable remediation. It is never wire
// data; the renderer receives the already-derived exact SecretUserAction.
pub(in crate::secret) enum SecretActionCondition {
    General,
    DeleteReadiness,
    RecoveryReadiness,
    CaptureFreshOperation,
    RotationFreshOperation,
    CandidateDiscardFreshOperation,
    ApplyOrActivationPlan,
    StagedImportResume,
    ValidationFreshOperation,
    RuntimeFreshOperation,
    CaptureBackendSelection,
    CandidateTerminalCleanupPending,
}

pub(in crate::secret) enum SecretDeleteReadinessDrift {
    Dependency,
    Record,
}

pub(in crate::secret) enum SecretTerminalOperationContext {
    Summary,
    Capture(BeginCaptureIntent),
    Rotation,
    CandidateDiscard,
    CandidateTerminalCleanupPending,
    Delete,
    Recovery,
    ApplyOrActivation,
    StagedImport,
    Validation,
    Runtime(FixedRuntimeConsumer),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SecretCommandName {
    ListSecretSummaries,
    ListSecretBackendOptions,
    BeginSecretCapture,
    RotateSecret,
    ListSecretCandidates,
    DiscardSecretCandidate,
    SetSecretLocked,
    GetSecretDeleteImpact,
    DeleteSecret,
    GetSecretCleanupImpact,
    RetrySecretCleanup,
    ValidateSecret,
    CheckSecretApplyReadiness,
    MigrateLegacyCodexSecrets,
    ListSecretAudit,
}

pub enum SecretCaptureFlowSelection {
    RegisteredBackendOption,
}

pub enum SecretFixedRuntimeEntry {
    ProxyRequest,
    UsageProbe,
    CodingPlanUsageProbe,
    ModelFetch,
}

pub enum SecretOperationIdPolicy {
    ServerGeneratedNew,
}

pub enum SecretExternalGuidance {
    UnlockBackend,
    GrantPermission,
    ReconnectDevice,
    OpenBackendSettings,
    OpenChangePlan,
    ContactAdministrator,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SecretMainIntegrationCommandName {
    ResumeStagedImportCutover,
}

pub enum SecretPostGuidanceDestination {
    None,
    RefreshSummary(SecretCommandName),
}

pub enum SecretActionDestination {
    None,
    SecretCommand(SecretCommandName),
    FreshSecretCommand {
        command: SecretCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    SecretCaptureFlow {
        intent: BeginCaptureIntent,
        list_options: SecretCommandName,
        selection: SecretCaptureFlowSelection,
        begin_capture: SecretCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    FixedRuntimeFlow {
        entry: SecretFixedRuntimeEntry,
        operation_id_policy: SecretOperationIdPolicy,
    },
    MainIntegrationCommand {
        command: SecretMainIntegrationCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    SecretCommandFlow {
        commands: [SecretCommandName; 2],
        operation_id_policy: SecretOperationIdPolicy,
    },
    NativeConfirmationContinuation,
    ExternalGuidance {
        guidance: SecretExternalGuidance,
        after: SecretPostGuidanceDestination,
    },
}

pub fn secret_action_destination(action: SecretUserAction) -> SecretActionDestination {
    use SecretActionDestination as Destination;
    use SecretCommandName as Command;
    use SecretExternalGuidance as Guidance;
    use SecretFixedRuntimeEntry as Runtime;
    use SecretMainIntegrationCommandName as MainCommand;
    use SecretPostGuidanceDestination as After;
    match action {
        SecretUserAction::None => Destination::None,
        SecretUserAction::RetryCapture => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::NewBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::RetryRotation => {
            Destination::FreshSecretCommand {
                command: Command::RotateSecret,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryProxyRequest => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::ProxyRequest,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryUsageProbe => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::UsageProbe,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryCodingPlanUsageProbe => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::CodingPlanUsageProbe,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryModelFetch => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::ModelFetch,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::UnlockFyAgent => {
            Destination::SecretCommand(Command::SetSecretLocked)
        }
        SecretUserAction::UnlockBackend => {
            Destination::ExternalGuidance {
                guidance: Guidance::UnlockBackend,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::RequestPermission => {
            Destination::ExternalGuidance {
                guidance: Guidance::GrantPermission,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::CaptureReplacement => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::ReplaceBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ChooseBackend => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::NewBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ConfirmDevice => Destination::NativeConfirmationContinuation,
        SecretUserAction::RefreshSummary => {
            Destination::SecretCommand(Command::ListSecretSummaries)
        }
        SecretUserAction::RefreshDeleteImpact => {
            Destination::FreshSecretCommand {
                command: Command::GetSecretDeleteImpact,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RefreshRecoveryImpact => {
            Destination::FreshSecretCommand {
                command: Command::GetSecretCleanupImpact,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::ReopenChangePlan => {
            Destination::ExternalGuidance {
                guidance: Guidance::OpenChangePlan,
                after: After::None,
            }
        }
        SecretUserAction::ResolveLegacyConflict => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::LegacyReconcile,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::DiscardCandidate => {
            Destination::FreshSecretCommand {
                command: Command::DiscardSecretCandidate,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::CompleteRecovery => Destination::SecretCommandFlow {
            commands: [
                Command::GetSecretCleanupImpact,
                Command::RetrySecretCleanup,
            ],
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ResumeStagedImportCutover => {
            Destination::MainIntegrationCommand {
                command: MainCommand::ResumeStagedImportCutover,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::ReconnectDevice => {
            Destination::ExternalGuidance {
                guidance: Guidance::ReconnectDevice,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::OpenBackendSettings => {
            Destination::ExternalGuidance {
                guidance: Guidance::OpenBackendSettings,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::ContactAdministrator => {
            Destination::ExternalGuidance {
                guidance: Guidance::ContactAdministrator,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecretErrorCode {
    SecretRequestInvalid,
    SecretRefInvalid,
    SecretOwnerKindUnsupported,
    SecretOwnerNamespaceUnsupported,
    SecretOwnerNotFound,
    SecretOwnerConflict,
    SecretOperationBusy,
    SecretUnsupportedPurpose,
    SecretConsumerUnsupported,
    SecretInputCancelled,
    SecretInputInvalid,
    SecretCandidateNotFound,
    SecretCandidateExpired,
    SecretCandidateConsumed,
    SecretChangePlanRequired,
    SecretChangePlanInvalid,
    SecretChangePlanStale,
    SecretMigrationRequired,
    SecretLegacySourceInvalid,
    SecretLegacyConflict,
    SecretLegacyComparisonPending,
    SecretMigrationFailed,
    SecretMissing,
    SecretLocked,
    SecretPermissionDenied,
    SecretBackendUnavailable,
    SecretStale,
    SecretRevoked,
    SecretConfirmationRequired,
    SecretConfirmationCancelled,
    SecretConfirmationExpired,
    SecretConfirmationReplayed,
    SecretDeviceMismatch,
    SecretWriteFailed,
    SecretReadFailed,
    SecretDeleteFailed,
    SecretVerifyFailed,
    SecretProjectionForbidden,
    SecretDependencyChanged,
    SecretRecordChanged,
    SecretBackendChanged,
    SecretCapabilityExpired,
    SecretCapabilityConsumed,
    SecretRecoveryNotFound,
    SecretRecoveryChanged,
    SecretOperationRecoveryRequired,
    SecretInternal,
}

impl TryFrom<SecretConsumer> for SecretRuntimeConsumer {
    type Error = SecretErrorCode;

    fn try_from(value: SecretConsumer) -> Result<Self, Self::Error> {
        match value {
            SecretConsumer::ChangePlanApply => Ok(Self::ChangePlanApply),
            SecretConsumer::ProxyRequest => Ok(Self::ProxyRequest),
            SecretConsumer::UsageProbe => Ok(Self::UsageProbe),
            SecretConsumer::CodingPlanUsageProbe => Ok(Self::CodingPlanUsageProbe),
            SecretConsumer::ModelFetch => Ok(Self::ModelFetch),
            SecretConsumer::ProviderTerminal => {
                Err(SecretErrorCode::SecretConsumerUnsupported)
            }
        }
    }
}

impl TryFrom<ApplyTargetSink> for SecretRuntimeSink {
    type Error = SecretErrorCode;

    fn try_from(value: ApplyTargetSink) -> Result<Self, Self::Error> {
        match value {
            ApplyTargetSink::ProcessMemory => Ok(Self::ProcessMemory),
            ApplyTargetSink::ExternalConfigFile => Ok(Self::ExternalConfigFile),
            ApplyTargetSink::ChildProcessEnvironment => {
                Err(SecretErrorCode::SecretProjectionForbidden)
            }
        }
    }
}

fn validate_change_plan_apply_route(
    consumer: SecretConsumer,
    sink: ApplyTargetSink,
) -> Result<
    (SecretChangePlanApplyConsumer, SecretChangePlanApplySink),
    SecretErrorCode,
> {
    match (
        SecretRuntimeConsumer::try_from(consumer)?,
        SecretRuntimeSink::try_from(sink)?,
    ) {
        (
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeSink::ExternalConfigFile,
        ) => Ok((
            SecretChangePlanApplyConsumer::ChangePlanApply,
            SecretChangePlanApplySink::ExternalConfigFile,
        )),
        (SecretRuntimeConsumer::ChangePlanApply, _) => {
            Err(SecretErrorCode::SecretProjectionForbidden)
        }
        _ => Err(SecretErrorCode::SecretConsumerUnsupported),
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwner {
    pub kind: SecretOwnerKind,
    pub namespace: SecretOwnerNamespace,
    pub owner_id: OwnerId,
    pub slot: SecretSlot,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeviceDisplay {
    pub display_name: SafeDisplayText,
    pub device_class: SecretDeviceClass,
    pub transport: SecretDeviceTransport,
}

wire_enum!(SecretDeviceClass { OsAccount, SecurityKey, SecureElement, Unknown });
wire_enum!(SecretDeviceTransport { Platform, Usb, Nfc, Ble, Unknown });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretBackendInstanceViewRepr {
    kind: SecretBackendKind,
    instance_id: SecretBackendInstanceId,
    generation: SecretBackendGeneration,
    availability: SecretBackendAvailability,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_absent_only"
    )]
    device: Option<SecretDeviceDisplay>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretBackendInstanceView(SecretBackendInstanceViewRepr);

impl SecretBackendInstanceView {
    fn validate_repr(
        repr: SecretBackendInstanceViewRepr,
    ) -> Result<Self, WireValidationError> {
        let valid_device = match (&repr.kind, repr.device.as_ref()) {
            (SecretBackendKind::Hardware, Some(device)) => {
                device.device_class != SecretDeviceClass::OsAccount
                    && device.transport != SecretDeviceTransport::Platform
            }
            (SecretBackendKind::Hardware, None) => false,
            (SecretBackendKind::OsKeyring, Some(device)) => {
                device.device_class == SecretDeviceClass::OsAccount
                    && device.transport == SecretDeviceTransport::Platform
            }
            (SecretBackendKind::OsKeyring, None) => true,
        };
        if !valid_device {
            return Err(WireValidationError("invalid backend device tuple"));
        }
        Ok(Self(repr))
    }

    // Only crate::secret::backend's registered-instance factory calls this.
    // Callers cannot submit or construct an instance identity tuple.
    pub(in crate::secret) fn try_registered(
        kind: SecretBackendKind,
        instance_id: SecretBackendInstanceId,
        generation: SecretBackendGeneration,
        availability: SecretBackendAvailability,
        device: Option<SecretDeviceDisplay>,
    ) -> Result<Self, SecretInternalError> {
        Self::validate_repr(SecretBackendInstanceViewRepr {
            kind,
            instance_id,
            generation,
            availability,
            device,
        })
        .map_err(|_| SecretInternalError::input_invalid())
    }

    pub fn kind(&self) -> SecretBackendKind { self.0.kind }
    pub fn instance_id(&self) -> &SecretBackendInstanceId { &self.0.instance_id }
    pub fn generation(&self) -> SecretBackendGeneration { self.0.generation }
    pub fn availability(&self) -> SecretBackendAvailability { self.0.availability }
    pub fn device(&self) -> Option<&SecretDeviceDisplay> { self.0.device.as_ref() }
}

impl Serialize for SecretBackendInstanceView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretBackendInstanceView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(SecretBackendInstanceViewRepr::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOperationConfirmationCapabilities {
    pub capture_verify: PhysicalConfirmation,
    pub validate: PhysicalConfirmation,
    pub resolve_for_apply: PhysicalConfirmation,
    pub delete: PhysicalConfirmation,
    pub revoke: PhysicalConfirmation,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretRecordCapabilitiesRepr {
    schema_version: SchemaVersionV1,
    capability_revision: CapabilityRevision,
    backend_kind: SecretBackendKind,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    device_binding: DeviceBinding,
    storage_residency: StorageResidency,
    operation_confirmation: SecretOperationConfirmationCapabilities,
    allowed_consumers: Vec<SecretRuntimeConsumer>,
    allowed_sinks: Vec<SecretRuntimeSink>,
    persistent_target_projection: bool,
    central_revocation: bool,
    revocation_observation: BackendRevocationObservationCapability,
    silent_fallback: AlwaysFalse,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecordCapabilities(SecretRecordCapabilitiesRepr);

wire_enum!(BackendRevocationObservationCapability {
    Unsupported, SourceAndTime
});

fn runtime_consumer_rank(value: SecretRuntimeConsumer) -> u8 {
    match value {
        SecretRuntimeConsumer::ChangePlanApply => 0,
        SecretRuntimeConsumer::ProxyRequest => 1,
        SecretRuntimeConsumer::UsageProbe => 2,
        SecretRuntimeConsumer::CodingPlanUsageProbe => 3,
        SecretRuntimeConsumer::ModelFetch => 4,
    }
}

fn runtime_sink_rank(value: SecretRuntimeSink) -> u8 {
    match value {
        SecretRuntimeSink::ProcessMemory => 0,
        SecretRuntimeSink::ExternalConfigFile => 1,
    }
}

impl SecretRecordCapabilities {
    fn validate_repr(
        repr: SecretRecordCapabilitiesRepr,
    ) -> Result<Self, WireValidationError> {
        let consumers_sorted = repr.allowed_consumers.windows(2).all(|pair| {
            runtime_consumer_rank(pair[0]) < runtime_consumer_rank(pair[1])
        });
        let sinks_sorted = repr.allowed_sinks.windows(2).all(|pair| {
            runtime_sink_rank(pair[0]) < runtime_sink_rank(pair[1])
        });
        let change_plan = repr
            .allowed_consumers
            .contains(&SecretRuntimeConsumer::ChangePlanApply);
        let memory_consumers = repr.allowed_consumers.iter().any(|consumer| {
            matches!(
                consumer,
                SecretRuntimeConsumer::ProxyRequest
                    | SecretRuntimeConsumer::UsageProbe
                    | SecretRuntimeConsumer::CodingPlanUsageProbe
                    | SecretRuntimeConsumer::ModelFetch
            )
        });
        let process_memory = repr
            .allowed_sinks
            .contains(&SecretRuntimeSink::ProcessMemory);
        let external_config = repr
            .allowed_sinks
            .contains(&SecretRuntimeSink::ExternalConfigFile);
        let os_all_confirmations_never = [
            repr.operation_confirmation.capture_verify,
            repr.operation_confirmation.validate,
            repr.operation_confirmation.resolve_for_apply,
            repr.operation_confirmation.delete,
            repr.operation_confirmation.revoke,
        ]
        .into_iter()
        .all(|confirmation| confirmation == PhysicalConfirmation::Never);
        let os_all_consumers = repr.allowed_consumers.as_slice() == [
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeConsumer::ProxyRequest,
            SecretRuntimeConsumer::UsageProbe,
            SecretRuntimeConsumer::CodingPlanUsageProbe,
            SecretRuntimeConsumer::ModelFetch,
        ];
        let os_all_sinks = repr.allowed_sinks.as_slice() == [
            SecretRuntimeSink::ProcessMemory,
            SecretRuntimeSink::ExternalConfigFile,
        ];
        let backend_matrix = match repr.backend_kind {
            SecretBackendKind::OsKeyring => {
                repr.device_binding == DeviceBinding::HostUser
                    && repr.storage_residency == StorageResidency::OsProtectedStore
                    && !repr.central_revocation
                    && repr.revocation_observation
                        == BackendRevocationObservationCapability::Unsupported
                    && os_all_confirmations_never
                    && os_all_consumers
                    && os_all_sinks
                    && repr.persistent_target_projection
            }
            SecretBackendKind::Hardware => {
                repr.device_binding == DeviceBinding::HardwareDevice
                    && repr.storage_residency == StorageResidency::HardwareOnly
            }
        };
        if !consumers_sorted
            || !sinks_sorted
            || repr.central_revocation
                != (repr.revocation_observation
                    == BackendRevocationObservationCapability::SourceAndTime)
            || change_plan != external_config
            || change_plan != repr.persistent_target_projection
            || memory_consumers != process_memory
            || !backend_matrix
        {
            return Err(WireValidationError("invalid record capability matrix"));
        }
        Ok(Self(repr))
    }

    // Only crate::secret::backend calls this constructor. It copies identity
    // from the registered backend instead of accepting caller-supplied ids.
    pub(in crate::secret) fn try_new(
        backend: &SecretBackendInstanceView,
        capability_revision: CapabilityRevision,
        device_binding_generation: DeviceBindingGeneration,
        device_binding: DeviceBinding,
        storage_residency: StorageResidency,
        operation_confirmation: SecretOperationConfirmationCapabilities,
        allowed_consumers: Vec<SecretRuntimeConsumer>,
        allowed_sinks: Vec<SecretRuntimeSink>,
        persistent_target_projection: bool,
        central_revocation: bool,
        revocation_observation: BackendRevocationObservationCapability,
    ) -> Result<Self, SecretInternalError> {
        let observation_matches = central_revocation
            == matches!(
                revocation_observation,
                BackendRevocationObservationCapability::SourceAndTime
            );
        if !observation_matches
            || (backend.kind() == SecretBackendKind::OsKeyring
                && central_revocation)
        {
            return Err(SecretInternalError::input_invalid());
        }
        Self::validate_repr(SecretRecordCapabilitiesRepr {
            schema_version: SchemaVersionV1,
            capability_revision,
            backend_kind: backend.kind(),
            backend_instance_id: backend.instance_id().clone(),
            backend_generation: backend.generation(),
            device_binding_generation,
            device_binding,
            storage_residency,
            operation_confirmation,
            allowed_consumers,
            allowed_sinks,
            persistent_target_projection,
            central_revocation,
            revocation_observation,
            silent_fallback: AlwaysFalse,
        })
        .map_err(|_| SecretInternalError::input_invalid())
    }

    pub fn backend_identity(
        &self,
    ) -> (&SecretBackendInstanceId, SecretBackendGeneration) {
        (&self.0.backend_instance_id, self.0.backend_generation)
    }

    pub fn allowed_consumers(&self) -> &[SecretRuntimeConsumer] {
        &self.0.allowed_consumers
    }

    pub fn allowed_sinks(&self) -> &[SecretRuntimeSink] {
        &self.0.allowed_sinks
    }

    pub fn operation_confirmation(
        &self,
    ) -> &SecretOperationConfirmationCapabilities {
        &self.0.operation_confirmation
    }

    pub(in crate::secret) fn central_revocation(&self) -> bool {
        self.0.central_revocation
    }

    pub fn capability_revision(&self) -> CapabilityRevision {
        self.0.capability_revision
    }

    pub fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.0.device_binding_generation
    }

    pub fn persistent_target_projection(&self) -> bool {
        self.0.persistent_target_projection
    }
}

impl Serialize for SecretRecordCapabilities {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretRecordCapabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = SecretRecordCapabilitiesRepr::deserialize(deserializer)?;
        Self::validate_repr(repr).map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretBindingSetCas {
    pub revision: SecretBindingSetRevision,
    pub digest: BindingSetDigest,
    pub count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRecoveryCas {
    pub revision: SecretRecoveryRevision,
    pub digest: SecretRecoveryDigest,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwnerBindingSummary {
    owner: SecretOwner,
    purpose: SecretPurpose,
    binding_revision: SecretBindingRevision,
    created_at: UtcTimestamp,
    updated_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretLockView {
    source: SecretLockSource,
    locked_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRevocationView {
    source: SecretRevocationSource,
    revoked_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformRevocationObservation {
    source: BackendObservedRevocationSource,
    revoked_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformBackendRevocationHint {
    _private: (),
}

// Ordinary read/probe can surface this non-Clone, non-serde, non-persistable
// hint only. It has no source/time/ref getter and is not accepted by authority.
pub(crate) struct BackendRevocationHint {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    _private: (),
}

struct BackendRevocationObservationScope {
    authorization_scope: BackendAuthorizationScope,
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    store_revision: SecretStoreRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

// Consuming native receipt: no Clone/Serialize/Deserialize/Debug. The raw
// platform observation cannot be persisted until the registered wrapper has
// proven SourceAndTime support plus the record's centralRevocation capability.
pub(crate) struct BackendRevocationObservation {
    scope: BackendRevocationObservationScope,
    source: BackendObservedRevocationSource,
    revoked_at: UtcTimestamp,
}

impl BackendRevocationObservation {
    fn checked_from_platform(
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        capabilities: &SecretRecordCapabilities,
        authorization: ConsumedBackendAuthorization,
        raw: PlatformRevocationObservationResult,
    ) -> Result<Self, SecretInternalError> {
        backend.assert_record_identity(record)?;
        authorization.scope.require_revoke_observation()?;
        let source_and_time = matches!(
            backend.registered.platform.revocation_observation_capability(),
            BackendRevocationObservationCapability::SourceAndTime
        );
        let capability_matches = capabilities.central_revocation()
            && capabilities.backend_identity()
                == (
                    backend.registered.instance.instance_id(),
                    backend.registered.instance.generation(),
                )
            && capabilities.capability_revision() == record.capability_revision
            && capabilities.device_binding_generation()
                == record.device_binding_generation;
        let returned_generations_match = raw.backend_generation
            == record.backend_generation
            && raw.device_binding_generation
                == record.device_binding_generation;
        if !source_and_time || !capability_matches || !returned_generations_match {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                authorization.scope.into_terminal_error_context(),
            ));
        }
        Ok(Self {
            scope: BackendRevocationObservationScope {
                authorization_scope: authorization.scope,
                registered_backend: RegisteredBackendHandleBinding::from_handle(backend),
                device_store_instance_id:
                    record.device_store_instance_id.clone(),
                secret_ref: record.secret_ref.clone(),
                store_revision: record.store_revision,
                record_revision: record.record_revision,
                binding_set_cas: record.binding_set_cas.clone(),
                backend_instance_id: record.instance_id.clone(),
                backend_generation: record.backend_generation,
                device_binding_generation: record.device_binding_generation,
                capability_revision: record.capability_revision,
            },
            source: raw.observation.source,
            revoked_at: raw.observation.revoked_at,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRecoveryPointer {
    recovery_id: SecretRecoveryId,
    kind: SecretRecoveryKind,
    recovery_cas: SecretRecoveryCas,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretIssueView {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock_source: Option<SecretLockSource>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation_source: Option<SecretRevocationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    recovery: Option<SecretRecoveryPointer>,
}

impl SecretIssueView {
    // Sole constructor in crate::secret::device_store::result. Arbitrary
    // code/action/source tuples are not accepted: the view can only project a
    // tuple already minted by SecretInternalError::checked.
    pub(super) fn checked_from_internal(error: &SecretInternalError) -> Self {
        Self {
            code: error.code,
            retryable: error.retryable,
            action: error.action,
            lock_source: error.lock_source,
            revocation_source: error.revocation_source,
            backend_unavailable_reason: error.backend_unavailable_reason,
            recovery: error.recovery.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRefAggregate {
    schema_version: SchemaVersionV1,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend: SecretBackendInstanceView,
    capabilities: SecretRecordCapabilities,
    bindings: Vec<SecretOwnerBindingSummary>,
    presence: SecretPresence,
    availability: SecretStableAvailability,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock: Option<SecretLockView>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation: Option<SecretRevocationView>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
    created_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    rotated_at: Option<UtcTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    last_validated_at: Option<UtcTimestamp>,
}

impl SecretRefAggregate {
    pub(super) fn checked_from_authority(
        aggregate: SecretRefAggregate,
    ) -> Result<Self, SecretInternalError> {
        let display_matches = aggregate.secret_ref_display
            == SecretRefDisplay::derive_from(&aggregate.secret_ref);
        let binding_count_matches = u32::try_from(aggregate.bindings.len())
            .ok()
            .is_some_and(|count| count == aggregate.binding_set_cas.count);
        let bindings_sorted_unique = aggregate.bindings.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0].owner) < secret_owner_sort_key(&pair[1].owner)
        });
        let backend_capability_matches = aggregate.capabilities.0.backend_kind
            == aggregate.backend.kind()
            && aggregate.capabilities.backend_identity()
                == (
                    aggregate.backend.instance_id(),
                    aggregate.backend.generation(),
                );
        let ready_has_no_issue = aggregate.availability != SecretStableAvailability::Ready
            || aggregate.issue.is_none();
        let ready_is_present = aggregate.availability != SecretStableAvailability::Ready
            || aggregate.presence == SecretPresence::Present;
        let revoked_has_revocation =
            aggregate.availability != SecretStableAvailability::Revoked
                || aggregate.revocation.is_some();
        let locked_has_lock = aggregate.availability != SecretStableAvailability::Locked
            || aggregate.lock.is_some();
        let issue_matrix = match aggregate.issue.as_ref() {
            None => true,
            Some(issue) => {
                let allowed = matches!(
                    issue.code,
                    SecretErrorCode::SecretMissing
                        | SecretErrorCode::SecretLocked
                        | SecretErrorCode::SecretPermissionDenied
                        | SecretErrorCode::SecretBackendUnavailable
                        | SecretErrorCode::SecretStale
                        | SecretErrorCode::SecretRevoked
                        | SecretErrorCode::SecretDeviceMismatch
                        | SecretErrorCode::SecretOperationRecoveryRequired
                );
                let locked_ok = issue.code != SecretErrorCode::SecretLocked
                    || (issue.lock_source.is_some()
                        && aggregate.lock.as_ref().is_some_and(|lock| {
                            issue.lock_source == Some(lock.source)
                        }));
                let revoked_ok = issue.code != SecretErrorCode::SecretRevoked
                    || (issue.revocation_source.is_some()
                        && aggregate.revocation.as_ref().is_some_and(|revocation| {
                            issue.revocation_source == Some(revocation.source)
                        }));
                let unavailable_ok = issue.code != SecretErrorCode::SecretBackendUnavailable
                    || issue.backend_unavailable_reason.is_some();
                let recovery_ok = issue.code
                    != SecretErrorCode::SecretOperationRecoveryRequired
                    || issue.recovery.is_some();
                allowed && locked_ok && revoked_ok && unavailable_ok && recovery_ok
            }
        };
        if display_matches
            && binding_count_matches
            && bindings_sorted_unique
            && backend_capability_matches
            && ready_has_no_issue
            && ready_is_present
            && revoked_has_revocation
            && locked_has_lock
            && issue_matrix
        {
            Ok(aggregate)
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum OwnerBindingStateRepr {
    Bound {
        secret_ref: SecretRef,
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
    },
    Legacy {
        legacy_state: LegacyOwnerState,
        sources: Vec<LegacySourceRef>,
        source_count: u32,
        action: SecretUserAction,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_id: Option<SecretCandidateId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_error: Option<SecretIssueView>,
    },
    Unbound,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OwnerBindingState(OwnerBindingStateRepr);

impl OwnerBindingState {
    pub(super) fn checked_from_authority(
        repr: OwnerBindingStateRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("bound identity or exact legacy state/source/count/candidate/error/action mapping; cached state never emits Retry")
    }
}

impl Serialize for OwnerBindingState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwnerCredentialSummary {
    schema_version: SchemaVersionV1,
    owner: SecretOwner,
    purpose: SecretPurpose,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_state: OwnerBindingState,
    legacy_source_coverage: LegacySourceCoverageView,
}

impl SecretOwnerCredentialSummary {
    pub(super) fn checked_from_authority(
        mut summary: SecretOwnerCredentialSummary,
        coverage: &LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        summary.legacy_source_coverage =
            LegacySourceCoverageView::checked_from_coverage_receipt(coverage)?;
        let _ = summary;
        todo!("owner/purpose/tombstone revision, checked binding-state identity and LegacySourceCoverageView derived from this exact opaque coverage receipt")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OwnerBindingExpectation {
    Unbound {
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
    },
    Bound {
        owner: SecretOwner,
        secret_ref: SecretRef,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
        source_binding_set: SecretBindingSetCas,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretCandidateSummary {
    schema_version: SchemaVersionV1,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    state: SecretCandidateState,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    backend: SecretBackendInstanceView,
    capabilities: SecretRecordCapabilities,
    target_owners: Vec<SecretOwner>,
    expected_bindings: Vec<OwnerBindingExpectation>,
    legacy_sources_to_scrub: CurrentLegacySourceExpectations,
    created_at: UtcTimestamp,
    expires_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_terminal_disposition: Option<CandidateTerminalState>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
}

impl SecretCandidateSummary {
    pub(super) fn checked_from_candidate_authority(
        summary: SecretCandidateSummary,
        journal: Option<&CandidateDeleteJournalRow>,
    ) -> Result<Self, SecretInternalError> {
        todo!(
            "pending disposition iff verifiedPendingPlan + nonterminal matching discard journal + OPERATION_RECOVERY_REQUIRED/discardCandidate; terminal forbids both fields"
        )
    }
}

wire_enum!(ActivationOldRecordDeleteOperation { Delete });
wire_enum!(ActivationOldRecordPostBindingState { NoBindings });
wire_enum!(ActivationOldRecordMissingReadbackOperation { Validate });
wire_enum!(ActivationOldRecordMissingReadbackScope {
    ActivationOldRecordMissingReadback
});

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretActivationOldRecordDeleteExpectation {
    NotApplicable,
    DeleteAfterActivation {
        operation: ActivationOldRecordDeleteOperation,
        old_secret_ref: SecretRef,
        expected_record_revision: SecretRecordRevision,
        expected_pre_activation_binding_set: SecretBindingSetCas,
        required_post_activation_binding_state:
            ActivationOldRecordPostBindingState,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        delete_confirmation: PhysicalConfirmation,
        missing_readback_operation: ActivationOldRecordMissingReadbackOperation,
        missing_readback_scope: ActivationOldRecordMissingReadbackScope,
        missing_readback_confirmation: PhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretActivationCandidateReadExpectation {
    pub operation: ActivationCandidateReadOperation,
    pub scope: ActivationCandidateReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub backend_generation: SecretBackendGeneration,
    pub device_binding_generation: DeviceBindingGeneration,
    pub capability_revision: CapabilityRevision,
    pub confirmation: PhysicalConfirmation,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretCandidateActivationProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: SecretCandidateActivationOperation,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    secret_ref: SecretRef,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    target_owners: Vec<SecretOwner>,
    expected_bindings: Vec<OwnerBindingExpectation>,
    legacy_sources_to_scrub: CurrentLegacySourceExpectations,
    candidate_read: SecretActivationCandidateReadExpectation,
    old_record_delete: SecretActivationOldRecordDeleteExpectation,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretCandidateActivationProjection(
    SecretCandidateActivationProjectionRepr,
);

impl SecretCandidateActivationProjection {
    fn validate_repr(
        repr: SecretCandidateActivationProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        let owner_sets_match = !repr.target_owners.is_empty()
            && repr.target_owners.len() == repr.expected_bindings.len()
            && repr.target_owners.windows(2).all(|pair| {
                secret_owner_sort_key(&pair[0]) < secret_owner_sort_key(&pair[1])
            })
            && repr
                .target_owners
                .iter()
                .zip(repr.expected_bindings.iter())
                .all(|(owner, expectation)| {
                    owner == match expectation {
                        OwnerBindingExpectation::Unbound { owner, .. }
                        | OwnerBindingExpectation::Bound { owner, .. } => owner,
                    }
                });
        let policy_matches = match repr.comparison_policy {
            LegacyActivationComparisonPolicy::CandidateEquality => {
                !repr.legacy_sources_to_scrub.as_slice().is_empty()
            }
            LegacyActivationComparisonPolicy::ExplicitReplacement => true,
        };
        let impact_matches = matches!(
            (&repr.comparison_policy, &repr.comparison_impact),
            (
                LegacyActivationComparisonPolicy::CandidateEquality,
                LegacyActivationComparisonImpact::CandidateEquality { .. },
            ) | (
                LegacyActivationComparisonPolicy::ExplicitReplacement,
                LegacyActivationComparisonImpact::ExplicitReplacement { .. },
            )
        );
        let fixed_scrub_policy = repr.kind
            != SecretCandidateKind::LegacyScrubExistingBinding
            || repr.comparison_policy
                == LegacyActivationComparisonPolicy::CandidateEquality;
        if owner_sets_match && policy_matches && impact_matches && fixed_scrub_policy {
            Ok(Self(repr))
        } else {
            Err(WireValidationError("invalid activation projection"))
        }
    }

    pub(in crate::secret) fn candidate_id(&self) -> &SecretCandidateId {
        &self.0.candidate_id
    }

    pub(in crate::secret) fn comparison_policy(
        &self,
    ) -> LegacyActivationComparisonPolicy {
        self.0.comparison_policy
    }

    pub(in crate::secret) fn projection_digest(&self) -> &SecretProjectionDigest {
        &self.0.projection_digest
    }

    pub(in crate::secret) fn legacy_sources(
        &self,
    ) -> &[LegacySourceExpectation] {
        self.0.legacy_sources_to_scrub.as_slice()
    }
}

impl Serialize for SecretCandidateActivationProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretCandidateActivationProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(
            SecretCandidateActivationProjectionRepr::deserialize(deserializer)?,
        )
        .map_err(de::Error::custom)
    }
}

wire_enum!(SecretCandidateActivationOperation { SecretCandidateActivation });
wire_enum!(StagedSecretImportActivationOperation { StagedSecretImportActivation });
wire_enum!(CodexProviderApplyOperation { CodexProviderApply });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct StagedLegacySourceExpectations(Vec<LegacySourceExpectation>);

impl StagedLegacySourceExpectations {
    fn validate(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        let staging_only = values.iter().all(|expectation| {
            matches!(
                expectation.source.origin,
                LegacySourceOrigin::SqlImportStaging
                    | LegacySourceOrigin::DbRestoreStaging
                    | LegacySourceOrigin::SyncDownloadStaging
            )
        });
        let sorted_unique = !values.is_empty() && values.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0].source)
                < legacy_source_sort_key(&pair[1].source)
        });
        if staging_only && sorted_unique {
            Ok(Self(values))
        } else {
            Err(WireValidationError(
                "staged import sources must be non-empty/staging/sorted/unique",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for StagedLegacySourceExpectations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate(Vec::<LegacySourceExpectation>::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StagedSourceSetCas {
    staged_row_revision: StagedRowRevision,
    structure_digest: RecoveryStructureDigest,
    source_count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StagedSecretImportActivationProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: StagedSecretImportActivationOperation,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_source_set_cas: StagedSourceSetCas,
    source_expectations: StagedLegacySourceExpectations,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expected_live_binding: OwnerBindingExpectation,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StagedSecretImportActivationProjection(
    StagedSecretImportActivationProjectionRepr,
);

impl StagedSecretImportActivationProjection {
    pub(in crate::secret) fn validate_repr(
        repr: StagedSecretImportActivationProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        let exact_count = repr.staged_source_set_cas.source_count as usize
            == repr.source_expectations.0.len();
        let owner_matches = match &repr.expected_live_binding {
            OwnerBindingExpectation::Unbound { owner, .. }
            | OwnerBindingExpectation::Bound { owner, .. } => owner == &repr.owner,
        };
        let impact_matches = matches!(
            (&repr.comparison_policy, &repr.comparison_impact),
            (
                LegacyActivationComparisonPolicy::CandidateEquality,
                LegacyActivationComparisonImpact::CandidateEquality { .. },
            ) | (
                LegacyActivationComparisonPolicy::ExplicitReplacement,
                LegacyActivationComparisonImpact::ExplicitReplacement { .. },
            )
        );
        if exact_count && owner_matches && impact_matches {
            Ok(Self(repr))
        } else {
            Err(WireValidationError("invalid staged import activation projection"))
        }
    }

    pub(in crate::secret) fn stage_id(&self) -> &ImportStageId {
        &self.0.stage_id
    }

    pub(in crate::secret) fn projection_digest(&self) -> &SecretProjectionDigest {
        &self.0.projection_digest
    }

    pub(crate) fn comparison_policy(&self) -> LegacyActivationComparisonPolicy {
        self.0.comparison_policy
    }
}

impl Serialize for StagedSecretImportActivationProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StagedSecretImportActivationProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(
            StagedSecretImportActivationProjectionRepr::deserialize(deserializer)?,
        )
        .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagedImportResumeCas {
    revision: StagedImportResumeRevision,
    digest: StagedImportResumeDigest,
}

wire_enum!(ResumeStagedImportCutoverAction { ResumeStagedImportCutover });

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeStagedImportCutoverRequest {
    stage_id: ImportStageId,
    expected_resume_cas: StagedImportResumeCas,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StagedSecretImportActivationResultRepr {
    Activated {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        candidate_id: SecretCandidateId,
        owner_summary: SecretOwnerCredentialSummary,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyActivated {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        candidate_id: SecretCandidateId,
        owner_summary: SecretOwnerCredentialSummary,
        audit_event_id: SecretAuditEventId,
    },
    CutoverRecoveryRequired {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        action: ResumeStagedImportCutoverAction,
        current_resume_cas: StagedImportResumeCas,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct StagedSecretImportActivationResultDto(
    StagedSecretImportActivationResultRepr,
);

impl StagedSecretImportActivationResultDto {
    fn checked_from_cutover_journal(
        repr: StagedSecretImportActivationResultRepr,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        todo!("initial terminal arm may project the verified candidate/owner summary; recovery arm exposes only stage/action/current CAS/audit while candidate/owner/checkpoint remain in the journal preimage")
    }
}

impl Serialize for StagedSecretImportActivationResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ResumeStagedImportCutoverResultRepr {
    Activated {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
    AlreadyActivated {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
    CutoverRecoveryRequired {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResumeStagedImportCutoverResultDto(
    ResumeStagedImportCutoverResultRepr,
);

impl ResumeStagedImportCutoverResultDto {
    fn checked_from_resume_journal(
        repr: ResumeStagedImportCutoverResultRepr,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        let _ = journal;
        todo!("exact five fields in every arm: stageId/currentResumeCas/status/action/issue; terminal arms require action=none + issue=None serialized as null, recovery requires action=resumeStagedImportCutover + Some(checked issue); schema/audit/candidate/owner/ref/summary are structurally impossible")
    }
}

impl Serialize for ResumeStagedImportCutoverResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretContractVersionV1 {
    #[serde(rename = "secret-contract/v1")]
    V1,
}

impl SecretContractVersionV1 {
    pub const WIRE: &'static str = "secret-contract/v1";
}

wire_enum!(SecretApplyRole { Target, Rollback });
wire_enum!(SecretApplyTargetRole { Target });
wire_enum!(SecretApplyRollbackRole { Rollback });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyTargetProjectionRepr {
    role: SecretApplyTargetRole,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyTargetProjection(SecretApplyTargetProjectionRepr);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyRollbackProjectionRepr {
    role: SecretApplyRollbackRole,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyRollbackProjection(SecretApplyRollbackProjectionRepr);

fn validate_apply_projection_identity(
    owner: &SecretOwner,
    binding_set: &SecretBindingSetCas,
) -> Result<(), WireValidationError> {
    let provider_codex = owner.kind == SecretOwnerKind::Provider
        && owner.namespace.as_str() == "codex";
    let nonzero_exact = binding_set.count > 0;
    if provider_codex && nonzero_exact {
        Ok(())
    } else {
        Err(WireValidationError("invalid apply projection identity"))
    }
}

macro_rules! impl_apply_role_projection {
    ($public:ident, $repr:ident) => {
        impl $public {
            fn validate_repr(repr: $repr) -> Result<Self, WireValidationError> {
                validate_apply_projection_identity(&repr.owner, &repr.binding_set_cas)?;
                Ok(Self(repr))
            }

            pub(in crate::secret) fn live_sink_id(&self) -> CodexLiveSecretSinkId {
                self.0.live_sink_id
            }
        }

        impl Serialize for $public {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $public {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                Self::validate_repr($repr::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

impl_apply_role_projection!(SecretApplyTargetProjection, SecretApplyTargetProjectionRepr);
impl_apply_role_projection!(SecretApplyRollbackProjection, SecretApplyRollbackProjectionRepr);

// Readiness output only. Each wrapped projection already serializes its
// single-value role; untagged here cannot swallow fields because there is no
// Deserialize implementation for this enum.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretApplyCredentialProjection {
    Target(SecretApplyTargetProjection),
    Rollback(SecretApplyRollbackProjection),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyPlanProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: CodexProviderApplyOperation,
    target: SecretApplyTargetProjection,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_absent_only"
    )]
    rollback: Option<SecretApplyRollbackProjection>,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyPlanProjection(SecretApplyPlanProjectionRepr);

impl SecretApplyPlanProjection {
    fn validate_repr(
        repr: SecretApplyPlanProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        let _operation = CodexProviderApplyOperation::CodexProviderApply;
        if let Some(rollback) = &repr.rollback {
            let _ = rollback.0.role;
        }
        let _ = repr.target.0.role;
        Ok(Self(repr))
    }
}

impl Serialize for SecretApplyPlanProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretApplyPlanProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Self::validate_repr(SecretApplyPlanProjectionRepr::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyReadinessContext {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    projection: SecretApplyCredentialProjection,
    checked_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum SecretApplyReadinessRepr {
    Ready {
        context: SecretApplyReadinessContext,
    },
    ConfirmationRequired {
        context: SecretApplyReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretApplyReadinessContext,
        error: SecretIssueView,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyReadiness(SecretApplyReadinessRepr);

impl SecretApplyReadiness {
    fn checked_from_authority(
        repr: SecretApplyReadinessRepr,
    ) -> Result<Self, SecretInternalError> {
        let context = match &repr {
            SecretApplyReadinessRepr::Ready { context } => context,
            SecretApplyReadinessRepr::ConfirmationRequired { context, .. } => context,
            SecretApplyReadinessRepr::Blocked { context, .. } => context,
        };
        if context.expires_at.as_str() < context.checked_at.as_str() {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretApplyReadiness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretConfirmationRequirementView {
    operation: SecretBackendOperation,
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwarePromptKey {
    #[serde(rename = "secret.hardware.confirmTouch")]
    ConfirmTouch,
}

wire_enum!(ResolveForApplyOperation { ResolveForApply });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ResolveForApplyOperation,
    pub role: SecretApplyRole,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HardwareConfirmStep {
    ResolveForApply {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        role: SecretApplyRole,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    CaptureVerify {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Validate {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Delete {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Revoke {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
}

wire_enum!(WriterReadbackMatchedCode { ReadbackMatched });
wire_enum!(WriterFailedCode { WriterFailed });
wire_enum!(WriterReadbackMismatchCode { ReadbackMismatch });
wire_enum!(WriterReadbackUnavailableCode { ReadbackUnavailable });
wire_enum!(WriterTargetChanged { Changed });
wire_enum!(WriterTargetNone { None });
wire_enum!(WriterTargetChangedUnknown { ChangedUnknown });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretWriterReceiptDto {
    Succeeded {
        writer_code: WriterReadbackMatchedCode,
        target_effect: WriterTargetChanged,
    },
    FailedBeforeMutation {
        writer_code: WriterFailedCode,
        target_effect: WriterTargetNone,
    },
    FailedAfterMutation {
        writer_code: WriterFailedCode,
        target_effect: WriterTargetChangedUnknown,
    },
    ReadbackMismatch {
        writer_code: WriterReadbackMismatchCode,
        target_effect: WriterTargetChanged,
    },
    ReadbackUnavailable {
        writer_code: WriterReadbackUnavailableCode,
        target_effect: WriterTargetChangedUnknown,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyResultDto {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    role: SecretApplyRole,
    status: SecretApplyResultStatus,
    writer: SecretWriterReceiptDto,
    consumed_record_revision: SecretRecordRevision,
    consumed_binding_set_revision: SecretBindingSetRevision,
    consumed_backend_generation: SecretBackendGeneration,
    audit_event_id: SecretAuditEventId,
}

pub(crate) struct ConsumedPreparedSecretCapabilityIdentity {
    _private: (),
}

impl SecretApplyResultDto {
    fn checked_from_consumed_capability(
        result: SecretApplyResultDto,
        capability: &ConsumedPreparedSecretCapabilityIdentity,
    ) -> Result<Self, SecretInternalError> {
        todo!("role/operation/revision/generation/writer/audit identity")
    }
}

wire_enum!(SecretApplyResultStatus { WriterReturned });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMutationImpact {
    schema_version: SchemaVersionV1,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    affected_owners: Vec<SecretOwnerBindingSummary>,
    effect: SecretImpactEffect,
    no_fallback: AlwaysTrue,
}

impl SecretMutationImpact {
    fn checked_from_candidate_snapshot(
        impact: SecretMutationImpact,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let owners_match = impact.affected_owners.iter().all(|row| {
            snapshot
                .projection
                .0
                .target_owners
                .iter()
                .any(|owner| owner == &row.owner)
                || snapshot
                    .affected_owners
                    .iter()
                    .any(|existing| existing.owner == row.owner)
        });
        if impact.secret_ref == snapshot.secret_ref
            && impact.record_revision == snapshot.record_revision
            && owners_match
        {
            Ok(impact)
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

wire_enum!(SecretImpactEffect { AllBindingsAffected, OneBindingAffected });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeleteReadinessContext {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub operation: SecretDeleteOperation,
    pub secret_ref: SecretRef,
    pub record_revision: SecretRecordRevision,
    pub binding_set_cas: SecretBindingSetCas,
    pub checked_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

wire_enum!(SecretDeleteOperation { Delete });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretDeleteReadiness {
    Ready {
        context: SecretDeleteReadinessContext,
    },
    ConfirmationRequired {
        context: SecretDeleteReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretDeleteReadinessContext,
        error: SecretIssueView,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeleteImpact {
    pub impact: SecretMutationImpact,
    pub readiness: SecretDeleteReadiness,
}

wire_enum!(ActivationCleanupStepKind {
    FinalizeLegacyScrub, DeleteOldRecord, VerifyOldRecordMissing
});
wire_enum!(SecretRecoveryOperation { Recovery });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationSupersessionView {
    pub source: RotationSupersessionSource,
    pub revoked_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivationCleanupStepImpact {
    FinalizeLegacyScrub {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteOldRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyOldRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryReadinessContext {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub operation: SecretRecoveryOperation,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
    pub recovery_cas: SecretRecoveryCas,
    pub checked_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretRecoveryReadiness {
    Ready {
        context: SecretRecoveryReadinessContext,
    },
    ConfirmationRequired {
        context: SecretRecoveryReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretRecoveryReadinessContext,
        error: SecretIssueView,
    },
}

fn secret_owner_sort_key(
    owner: &SecretOwner,
) -> (&'static str, &str, &str, &'static str) {
    let kind = match owner.kind {
        SecretOwnerKind::Provider => "provider",
        SecretOwnerKind::Agent => "agent",
    };
    let slot = match owner.slot {
        SecretSlot::PrimaryApiKey => "primaryApiKey",
    };
    (
        kind,
        owner.namespace.as_str(),
        owner.owner_id.as_str(),
        slot,
    )
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedAffectedOwners(Vec<SecretOwnerBindingSummary>);

impl SortedAffectedOwners {
    pub(in crate::secret) fn try_from_sorted_nonempty(
        owners: Vec<SecretOwnerBindingSummary>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0].owner)
                < secret_owner_sort_key(&pair[1].owner)
        });
        if owners.is_empty() || !ordered {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(owners))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedOwnerSummaries(Vec<SecretOwnerCredentialSummary>);

impl SortedOwnerSummaries {
    pub(in crate::secret) fn try_from_sorted_nonempty(
        owners: Vec<SecretOwnerCredentialSummary>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0].owner)
                < secret_owner_sort_key(&pair[1].owner)
        });
        if owners.is_empty() || !ordered {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(owners))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedSecretOwners(Vec<SecretOwner>);

impl SortedSecretOwners {
    pub(in crate::secret) fn try_from_sorted_unique(
        owners: Vec<SecretOwner>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0]) < secret_owner_sort_key(&pair[1])
        });
        if ordered {
            Ok(Self(owners))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedActivationCleanupSteps(Vec<ActivationCleanupStepKind>);

impl SortedActivationCleanupSteps {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepKind>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            []
            | [ActivationCleanupStepKind::FinalizeLegacyScrub]
            | [ActivationCleanupStepKind::DeleteOldRecord]
            | [ActivationCleanupStepKind::VerifyOldRecordMissing]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn contains(&self, step: &ActivationCleanupStepKind) -> bool {
        self.0.contains(step)
    }

    fn iter(&self) -> impl Iterator<Item = &ActivationCleanupStepKind> {
        self.0.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptyActivationCleanupSteps(Vec<ActivationCleanupStepKind>);

impl NonEmptyActivationCleanupSteps {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepKind>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            [ActivationCleanupStepKind::FinalizeLegacyScrub]
            | [ActivationCleanupStepKind::DeleteOldRecord]
            | [ActivationCleanupStepKind::VerifyOldRecordMissing]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn is_disjoint_from(&self, completed: &SortedActivationCleanupSteps) -> bool {
        self.0.iter().all(|step| !completed.contains(step))
    }


    fn iter(&self) -> impl Iterator<Item = &ActivationCleanupStepKind> {
        self.0.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptySortedActivationCleanupStepImpacts(
    Vec<ActivationCleanupStepImpact>,
);

impl NonEmptySortedActivationCleanupStepImpacts {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepImpact>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            [ActivationCleanupStepImpact::FinalizeLegacyScrub { .. }]
            | [ActivationCleanupStepImpact::DeleteOldRecord { .. }]
            | [ActivationCleanupStepImpact::VerifyOldRecordMissing { .. }]
            | [
                ActivationCleanupStepImpact::FinalizeLegacyScrub { .. },
                ActivationCleanupStepImpact::DeleteOldRecord { .. },
                ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
            ]
            | [
                ActivationCleanupStepImpact::DeleteOldRecord { .. },
                ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }


    fn contains_kind(&self, expected: &ActivationCleanupStepKind) -> bool {
        self.0.iter().any(|impact| {
            matches!(
                (impact, expected),
                (
                    ActivationCleanupStepImpact::FinalizeLegacyScrub { .. },
                    ActivationCleanupStepKind::FinalizeLegacyScrub,
                ) | (
                    ActivationCleanupStepImpact::DeleteOldRecord { .. },
                    ActivationCleanupStepKind::DeleteOldRecord,
                ) | (
                    ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
                    ActivationCleanupStepKind::VerifyOldRecordMissing,
                )
            )
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivationCleanupImpactRepr {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    affected_owners: SortedAffectedOwners,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedActivationCleanupStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq)]
struct ActivationCleanupImpact(ActivationCleanupImpactRepr);

impl ActivationCleanupImpact {
    // Private device-authority factory; no public/product constructor.
    fn from_recovery_snapshot(
        repr: ActivationCleanupImpactRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let _ = snapshot;
        todo!("validate activation-cleanup impact against recovery snapshot");
        let context = match &repr.readiness {
            SecretRecoveryReadiness::Ready { context }
            | SecretRecoveryReadiness::ConfirmationRequired { context, .. }
            | SecretRecoveryReadiness::Blocked { context, .. } => context,
        };
        if context.recovery_id != repr.recovery_id
            || context.recovery_cas != repr.recovery_cas
        {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self(repr))
    }
}

impl Serialize for ActivationCleanupImpact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ActivationCleanupResultRepr {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: [ActivationCleanupStepKind; 0],
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: [ActivationCleanupStepKind; 0],
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: NonEmptyActivationCleanupSteps,
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
struct ActivationCleanupResult(ActivationCleanupResultRepr);

impl ActivationCleanupResult {
    // The three private owner-module factories populate one repr variant and
    // all call this gate before construction.
    fn validate_sets(
        completed: &SortedActivationCleanupSteps,
        remaining: Option<&NonEmptyActivationCleanupSteps>,
        issue: Option<&SecretIssueView>,
    ) -> Result<(), SecretInternalError> {
        match (remaining, issue) {
            (None, None) => Ok(()),
            (Some(remaining), Some(issue))
                if remaining.is_disjoint_from(completed)
                    && issue.code
                        == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn from_authority_snapshot(
        repr: ActivationCleanupResultRepr,
        admitted_pending: &NonEmptySortedActivationCleanupStepImpacts,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let _ = snapshot;
        todo!("validate activation-cleanup result against recovery snapshot");
        match &repr {
            ActivationCleanupResultRepr::Complete {
                completed_steps, ..
            }
            | ActivationCleanupResultRepr::AlreadyComplete {
                completed_steps, ..
            } => {
                Self::validate_sets(completed_steps, None, None)?;
                if !completed_steps
                    .iter()
                    .all(|step| admitted_pending.contains_kind(step))
                {
                    return Err(SecretInternalError::input_invalid());
                }
            }
            ActivationCleanupResultRepr::RecoveryRequired {
                recovery_id,
                recovery_cas,
                completed_steps,
                remaining_steps,
                issue,
                ..
            } => {
                Self::validate_sets(
                    completed_steps,
                    Some(remaining_steps),
                    Some(issue),
                )?;
                if !issue.recovery.as_ref().is_some_and(|pointer| {
                    &pointer.recovery_id == recovery_id
                        && &pointer.recovery_cas == recovery_cas
                }) {
                    return Err(SecretInternalError::input_invalid());
                }
                if !completed_steps
                    .iter()
                    .chain(remaining_steps.iter())
                    .all(|step| admitted_pending.contains_kind(step))
                {
                    return Err(SecretInternalError::input_invalid());
                }
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for ActivationCleanupResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretRecoveryStepKind {
    FinalizeLegacyScrub,
    DeleteOldRecord,
    VerifyOldRecordMissing,
    DeleteUncommittedRecord,
    VerifyUncommittedRecordMissing,
    FinalizeCaptureCompensation,
    DeleteAdmittedRecord,
    VerifyDeletedRecordMissing,
    FinalizeDeletedRecord,
    FinalizeOwnerDetach,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NeverPhysicalConfirmation { Never }

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct SortedRecoverySteps(Vec<SecretRecoveryStepKind>);

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct NonEmptySortedRecoverySteps(Vec<SecretRecoveryStepKind>);

impl SortedRecoverySteps {
    pub(in crate::secret) fn checked(
        values: Vec<SecretRecoveryStepKind>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique completed subset of exact kind allowlist")
    }
}

impl NonEmptySortedRecoverySteps {
    pub(in crate::secret) fn checked(
        values: Vec<SecretRecoveryStepKind>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("nonempty sorted unique remaining subset of exact kind allowlist")
    }

    pub(in crate::secret) fn disjoint_from(&self, completed: &SortedRecoverySteps) -> bool {
        todo!("exact disjointness")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretRecoveryStepImpact {
    FinalizeLegacyScrub {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteOldRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyOldRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteUncommittedRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyUncommittedRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    FinalizeCaptureCompensation {
        confirmation: NeverPhysicalConfirmation,
    },
    DeleteAdmittedRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyDeletedRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    FinalizeDeletedRecord {
        confirmation: NeverPhysicalConfirmation,
    },
    FinalizeOwnerDetach {
        confirmation: NeverPhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct NonEmptySortedRecoveryStepImpacts(Vec<SecretRecoveryStepImpact>);

impl NonEmptySortedRecoveryStepImpacts {
    fn checked(
        values: Vec<SecretRecoveryStepImpact>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("non-empty strict rank/unique and exact recovery-kind step allowlist")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureCompensationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteFinalizationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    affected_owners: SortedAffectedOwners,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum OwnerDetachRecoveryBindingState {
    Bound {
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
    },
    Unbound,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnerDetachFinalizationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    detached_owner: SecretOwner,
    remaining_owners: SortedSecretOwners,
    binding_state: OwnerDetachRecoveryBindingState,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    content = "impact",
    rename_all = "camelCase"
)]
enum SecretRecoveryImpactRepr {
    ActivationCleanup(ActivationCleanupImpactRepr),
    CaptureCompensation(CaptureCompensationImpact),
    DeleteFinalization(DeleteFinalizationImpact),
    OwnerDetachFinalization(OwnerDetachFinalizationImpact),
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecoveryImpact(SecretRecoveryImpactRepr);

impl SecretRecoveryImpact {
    fn from_authority_snapshot(
        repr: SecretRecoveryImpactRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_recovery_impact_identity(&repr)?;
        todo!("validate outer kind equals readiness kind/CAS and exact step algebra")
    }
}

impl Serialize for SecretRecoveryImpact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum LocalRecoveryOutcome {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: NonEmptySortedRecoverySteps,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CaptureCompensationRecoveryResult {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        terminal_candidate_state: DiscardedCandidateTerminalState,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        terminal_candidate_state: DiscardedCandidateTerminalState,
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: NonEmptySortedRecoverySteps,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteFinalizationRecoveryResult {
    owner_summaries: SortedOwnerSummaries,
    aggregate: SecretRefAggregate,
    outcome: LocalRecoveryOutcome,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnerDetachRecoveryResult {
    detached_owner: SecretOwner,
    remaining_owners: SortedSecretOwners,
    outcome: LocalRecoveryOutcome,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "camelCase"
)]
enum SecretRecoveryResultRepr {
    ActivationCleanup(ActivationCleanupResultRepr),
    CaptureCompensation(CaptureCompensationRecoveryResult),
    DeleteFinalization(DeleteFinalizationRecoveryResult),
    OwnerDetachFinalization(OwnerDetachRecoveryResult),
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecoveryResult(SecretRecoveryResultRepr);

impl SecretRecoveryResult {
    fn from_authority_snapshot(
        repr: SecretRecoveryResultRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_recovery_result_identity(&repr)?;
        todo!("validate kind-specific terminal/pending rows and disjoint step sets")
    }
}

impl Serialize for SecretRecoveryResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NullableSecretMutationImpact(
    Option<SecretMutationImpact>,
);

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageSecretCandidateResult {
    status: SecretCandidateStageStatus,
    candidate: SecretCandidateSummary,
    activation_projection: SecretCandidateActivationProjection,
    // Required field: serializes as object or explicit null; omission fails.
    impact: NullableSecretMutationImpact,
    audit_event_id: SecretAuditEventId,
}

impl StageSecretCandidateResult {
    fn checked_from_candidate_snapshot(
        result: StageSecretCandidateResult,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        if result.status != SecretCandidateStageStatus::Staged {
            return Err(SecretInternalError::input_invalid());
        }
        SecretCandidateWithProjection::checked_from_candidate_snapshot(
            SecretCandidateWithProjection {
                candidate: result.candidate.clone(),
                activation_projection: result.activation_projection.clone(),
            },
            snapshot,
        )?;
        if let Some(impact) = result.impact.0.clone() {
            SecretMutationImpact::checked_from_candidate_snapshot(impact, snapshot)?;
        }
        Ok(result)
    }
}

wire_enum!(SecretCandidateStageStatus { Staged });

pub(in crate::secret) fn legacy_source_sort_key(
    source: &LegacySourceRef,
) -> (u8, u8, &str) {
    let origin = match source.origin {
        LegacySourceOrigin::ProviderRow => 0,
        LegacySourceOrigin::LiveAuth => 1,
        LegacySourceOrigin::LiveConfig => 2,
        LegacySourceOrigin::SqlImportStaging => 3,
        LegacySourceOrigin::DbRestoreStaging => 4,
        LegacySourceOrigin::SyncDownloadStaging => 5,
    };
    let category = match source.category {
        LegacySourceCategory::ProviderAuthJson => 0,
        LegacySourceCategory::ProviderConfigTomlTopLevel => 1,
        LegacySourceCategory::ProviderConfigTomlActiveTable => 2,
        LegacySourceCategory::ProviderConfigTomlInactiveTable => 3,
        LegacySourceCategory::ProviderConfigTomlInlineTable => 4,
        LegacySourceCategory::ProviderUsageScriptApiKey => 5,
        LegacySourceCategory::ProviderNonCanonicalProxyAlias => 6,
    };
    (origin, category, source.location_id.as_str())
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedLegacySourceRefs(Vec<LegacySourceRef>);

impl SortedLegacySourceRefs {
    fn try_from_sorted_unique(
        sources: Vec<LegacySourceRef>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = sources.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0]) < legacy_source_sort_key(&pair[1])
        });
        if ordered {
            Ok(Self(sources))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }

    fn is_disjoint_from(&self, retained: &NonEmptySortedLegacySourceRefs) -> bool {
        self.0.iter().all(|source| !retained.0.contains(source))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptySortedLegacySourceRefs(Vec<LegacySourceRef>);

impl NonEmptySortedLegacySourceRefs {
    fn try_from_sorted_unique(
        sources: Vec<LegacySourceRef>,
    ) -> Result<Self, SecretInternalError> {
        if sources.is_empty() {
            return Err(SecretInternalError::input_invalid());
        }
        SortedLegacySourceRefs::try_from_sorted_unique(sources.clone())?;
        Ok(Self(sources))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretLegacyCleanupTerminal {
    NotApplicable,
    Complete {
        scrubbed_sources: SortedLegacySourceRefs,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretLegacyCleanupPending {
    Partial {
        scrubbed_sources: SortedLegacySourceRefs,
        retained_sources: NonEmptySortedLegacySourceRefs,
        issue: SecretIssueView,
    },
    Blocked {
        retained_sources: NonEmptySortedLegacySourceRefs,
        issue: SecretIssueView,
    },
}

impl SecretLegacyCleanupPending {
    fn validate(&self) -> Result<(), SecretInternalError> {
        match self {
            Self::Partial {
                scrubbed_sources,
                retained_sources,
                issue,
            } if scrubbed_sources.is_disjoint_from(retained_sources)
                && issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            Self::Blocked { issue, .. }
                if issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn issue(&self) -> &SecretIssueView {
        match self {
            Self::Partial { issue, .. } | Self::Blocked { issue, .. } => issue,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretOldRecordCleanupTerminal {
    NotApplicable,
    Deleted {
        old_secret_ref_display: SecretRefDisplay,
        supersession: RotationSupersessionView,
    },
    AlreadyMissing {
        old_secret_ref_display: SecretRefDisplay,
        supersession: RotationSupersessionView,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretOldRecordCleanupPending {
    pub status: SecretOldRecordCleanupPendingStatus,
    pub old_secret_ref_display: SecretRefDisplay,
    pub issue: SecretIssueView,
}

wire_enum!(SecretOldRecordCleanupPendingStatus { CleanupRequired });
wire_enum!(SecretActivationCompleteKind { Complete });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationCompleteCleanup {
    pub kind: SecretActivationCompleteKind,
    pub legacy: SecretLegacyCleanupTerminal,
    pub old_record: SecretOldRecordCleanupTerminal,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretActivationPendingCleanup {
    LegacyScrubPending {
        legacy: SecretLegacyCleanupPending,
        old_record: SecretOldRecordNotAttempted,
        recovery: SecretRecoveryPointer,
    },
    OldRecordDeletePending {
        legacy: SecretLegacyCleanupTerminal,
        old_record: SecretOldRecordCleanupPending,
        recovery: SecretRecoveryPointer,
    },
}

impl SecretActivationPendingCleanup {
    fn validate(&self) -> Result<(), SecretInternalError> {
        match self {
            Self::LegacyScrubPending {
                legacy,
                recovery,
                ..
            } => {
                legacy.validate()?;
                if legacy.issue().recovery.as_ref() == Some(recovery) {
                    Ok(())
                } else {
                    Err(SecretInternalError::input_invalid())
                }
            }
            Self::OldRecordDeletePending {
                old_record,
                recovery,
                ..
            }
                if old_record.issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired
                    && old_record.issue.recovery.as_ref() == Some(recovery) =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }
}

wire_enum!(SecretOldRecordNotAttemptedStatus { NotAttempted });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretOldRecordNotAttempted {
    pub status: SecretOldRecordNotAttemptedStatus,
}

wire_enum!(ActivationOldRecordDeleteScope { ActivationOldRecordDelete });
wire_enum!(ActivationCandidateReadOperation { ResolveForApply });
wire_enum!(ActivationCandidateReadScope { ActivationCandidateCompare });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationReadHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationCandidateReadOperation,
    pub scope: ActivationCandidateReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationOldRecordDeleteOperation,
    pub scope: ActivationOldRecordDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationOldRecordMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationOldRecordMissingReadbackOperation,
    pub scope: ActivationOldRecordMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretActivationHardwareConfirmStep {
    CandidateRead(SecretActivationReadHardwareConfirmStep),
    OldRecordDelete(SecretActivationDeleteHardwareConfirmStep),
    OldRecordMissingReadback(SecretActivationOldRecordMissingHardwareConfirmStep),
}

impl SecretActivationHardwareConfirmStep {
    fn operation_id(&self) -> &SecretOperationId {
        match self {
            Self::CandidateRead(step) => &step.operation_id,
            Self::OldRecordDelete(step) => &step.operation_id,
            Self::OldRecordMissingReadback(step) => &step.operation_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretActivationPreparationViewRepr {
    Prepared {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
    },
    ConfirmationRequired {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        step: SecretActivationHardwareConfirmStep,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretActivationPreparationView(SecretActivationPreparationViewRepr);

impl SecretActivationPreparationView {
    // Private to crate::secret::device_store::result.
    fn from_prepared(
        repr: SecretActivationPreparationViewRepr,
    ) -> Result<Self, SecretInternalError> {
        if let SecretActivationPreparationViewRepr::ConfirmationRequired {
            operation_id,
            step,
            ..
        } = &repr
        {
            if step.operation_id() != operation_id {
                return Err(SecretInternalError::input_invalid());
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretActivationPreparationView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretActivationResultDtoRepr {
    Activated {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationCompleteCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyActivated {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationCompleteCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
    ActivatedCleanupPending {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationPendingCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretActivationResultDto(SecretActivationResultDtoRepr);

impl SecretActivationResultDto {
    // Private to crate::secret::device_store::result after identity/recovery
    // cross-checks against the committed authority snapshot.
    fn from_authority_snapshot(
        repr: SecretActivationResultDtoRepr,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_activation_result_identity(&repr)?;
        if let SecretActivationResultDtoRepr::ActivatedCleanupPending {
            cleanup,
            ..
        } = &repr
        {
            cleanup.validate()?;
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretActivationResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

wire_enum!(SecretTargetProjectionStatus { NotPerformedByActivation });
wire_enum!(LegacyMigrationOwnerStatus {
    NoCredential, AlreadyMigrated, CandidateStaged, CleanupCandidateStaged,
    Conflict, SourceInvalid, ComparisonPending, Blocked, Failed
});
wire_enum!(HistoricalArtifactCategory {
    HistoricalProviderSnapshot, AppPrivateCache, ManagedDiagnostic,
    ManagedBackup, UserOwnedExport
});
wire_enum!(ArtifactScanStatus { NotRun, Complete, Partial, Blocked });
wire_enum!(SecretMigrationStatus {
    NoChanges, Staged, ApprovalRequired, Partial, Blocked
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyMigrationOwnerResult {
    owner: SecretOwner,
    status: LegacyMigrationOwnerStatus,
    sources: Vec<LegacySourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    candidate_id: Option<SecretCandidateId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    activation_projection: Option<SecretCandidateActivationProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    plan_id: Option<ChangePlanId>,
    action: SecretUserAction,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretArtifactScanReport {
    status: ArtifactScanStatus,
    enumerated_categories: Vec<HistoricalArtifactCategory>,
    scanned_count: u32,
    finding_count: u32,
    report_only_count: u32,
    unreadable_count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMigrationReport {
    schema_version: SchemaVersionV1,
    report_id: SecretMigrationReportId,
    status: SecretMigrationStatus,
    owners: Vec<LegacyMigrationOwnerResult>,
    artifact_scan: SecretArtifactScanReport,
    started_at: UtcTimestamp,
    completed_at: UtcTimestamp,
}

impl SecretMigrationReport {
    pub(super) fn checked_from_inventory(
        report: SecretMigrationReport,
    ) -> Result<Self, SecretInternalError> {
        todo!("owner status/candidate/projection/plan/action/issue and aggregate status matrix")
    }
}

wire_enum!(SecretApplyAuditAction {
    PrepareApply, ConfirmHardware, ResolveApply
});
wire_enum!(SecretGeneralAuditAction {
    CaptureCandidate, DiscardCandidate, ActivateCandidate, Validate,
    RotateCandidate, Lock, Unlock, Delete, Revoke, CheckReadiness,
    MigrateLegacy, ReconcileLegacy, ReconcileRecovery, RetryCleanup,
    CancelConfirmation
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretAuditScope {
    General {
        action: SecretGeneralAuditAction,
    },
    Apply {
        action: SecretApplyAuditAction,
        role: SecretApplyRole,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretAuditEvent {
    schema_version: SchemaVersionV1,
    event_id: SecretAuditEventId,
    occurred_at: UtcTimestamp,
    operation_id: SecretOperationId,
    scope: SecretAuditScope,
    outcome: SecretAuditOutcome,
    effect: SecretEffect,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    owner: Option<SecretOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    secret_ref_display: Option<SecretRefDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    backend_kind: Option<SecretBackendKind>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    backend_instance_id: Option<SecretBackendInstanceId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    error_code: Option<SecretErrorCode>,
}

impl SecretAuditEvent {
    // Sole device-store audit factory. It enforces §11.1 action/scope/role,
    // outcome/effect/error and optional owner/backend tuple constraints.
    pub(super) fn checked_from_operation(
        event: SecretAuditEvent,
    ) -> Result<Self, SecretInternalError> {
        todo!("complete audit matrix and material-free optional-field allowlist")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretErrorView {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    effect: SecretEffect,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    audit_event_id: Option<SecretAuditEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    owner: Option<SecretOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    secret_ref_display: Option<SecretRefDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock_source: Option<SecretLockSource>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation_source: Option<SecretRevocationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recovery: Option<SecretRecoveryPointer>,
}

impl SecretErrorView {
    fn checked_from_internal(
        error: SecretInternalError,
        audit_event_id: Option<SecretAuditEventId>,
        owner: Option<SecretOwner>,
        secret_ref_display: Option<SecretRefDisplay>,
    ) -> Self {
        Self {
            code: error.code,
            retryable: error.retryable,
            action: error.action,
            effect: error.effect,
            audit_event_id,
            owner,
            secret_ref_display,
            lock_source: error.lock_source,
            revocation_source: error.revocation_source,
            backend_unavailable_reason: error.backend_unavailable_reason,
            recovery: error.recovery,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCommandSuccess<T> {
    pub contract_version: SecretContractVersionV1,
    pub schema_version: SchemaVersionV1,
    pub command_id: SecretCommandId,
    pub data: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCommandError {
    pub contract_version: SecretContractVersionV1,
    pub schema_version: SchemaVersionV1,
    pub command_id: SecretCommandId,
    pub error: SecretErrorView,
}

pub type SecretCommandResult<T> =
    Result<SecretCommandSuccess<T>, SecretCommandError>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretSummariesRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub secret_ref: Option<SecretRef>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub availability: Option<Vec<SecretStableAvailability>>,
    pub include_unbound_owners: bool,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub cursor: Option<SecretSummaryCursor>,
    pub limit: PageLimit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretSummariesResult {
    owners: Vec<SecretOwnerCredentialSummary>,
    refs: Vec<SecretRefAggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<SecretSummaryCursor>,
}

impl ListSecretSummariesResult {
    fn checked_from_authority(
        result: ListSecretSummariesResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique owners/refs, binding joins, cursor/page invariants")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretBackendOptionsRequest {
    pub schema_version: SchemaVersionV1,
    pub owner: SecretOwner,
    pub purpose: SecretPurpose,
    pub intent: BeginCaptureIntent,
}

wire_enum!(BeginCaptureIntent { NewBinding, ReplaceBinding, LegacyReconcile });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretBackendOption {
    backend: SecretBackendInstanceView,
    capabilities_for_new_record: SecretRecordCapabilities,
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretCaptureBindingView {
    Unbound,
    Bound {
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
    },
    Legacy {
        legacy_state: LegacyOwnerState,
        source_count: u32,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureIntentView {
    schema_version: SchemaVersionV1,
    capture_intent_id: SecretCaptureIntentId,
    owner: SecretOwner,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    current_binding: SecretCaptureBindingView,
    legacy_source_coverage: LegacySourceCoverageView,
    expires_at: UtcTimestamp,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretBackendOptionsResult {
    capture_intent: SecretCaptureIntentView,
    options: Vec<SecretBackendOption>,
}

impl ListSecretBackendOptionsResult {
    fn checked_from_registry(
        result: ListSecretBackendOptionsResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("output-only intent view is derived from the exact atomic owner/binding/coverage receipt; options are sorted unique registered instances with matching validated capabilities")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginSecretCaptureRequest {
    pub schema_version: SchemaVersionV1,
    pub capture_intent_id: SecretCaptureIntentId,
    pub backend_instance_id: SecretBackendInstanceId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotateSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub backend_instance_id: SecretBackendInstanceId,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretCandidatesRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    pub include_terminal: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretCandidatesResult {
    candidates: Vec<SecretCandidateWithProjection>,
}

impl ListSecretCandidatesResult {
    fn checked_from_authority(
        result: ListSecretCandidatesResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique candidate rows and exact projection pairing")
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateWithProjection {
    candidate: SecretCandidateSummary,
    activation_projection: SecretCandidateActivationProjection,
}

impl SecretCandidateWithProjection {
    fn checked_from_candidate_snapshot(
        value: SecretCandidateWithProjection,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let candidate = &value.candidate;
        let projection = &value.activation_projection.0;
        let matched = candidate.candidate_id == snapshot.candidate_id
            && candidate.candidate_revision == snapshot.candidate_revision
            && candidate.comparison_policy == snapshot.comparison_policy
            && candidate.secret_ref == snapshot.secret_ref
            && candidate.record_revision == snapshot.record_revision
            && candidate.target_owners == snapshot.projection.0.target_owners
            && projection.candidate_id == snapshot.candidate_id
            && projection.candidate_revision == snapshot.candidate_revision
            && projection.comparison_policy == snapshot.comparison_policy
            && projection.secret_ref == snapshot.secret_ref
            && projection.record_revision == snapshot.record_revision
            && projection.projection_digest == snapshot.projection.0.projection_digest
            && projection.target_owners == snapshot.projection.0.target_owners;
        if matched {
            Ok(value)
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscardSecretCandidateRequest {
    pub schema_version: SchemaVersionV1,
    pub candidate_id: SecretCandidateId,
    pub expected_candidate_revision: SecretCandidateRevision,
}

wire_enum!(CandidateDiscardDeleteOperation { Delete });
wire_enum!(CandidateDiscardDeleteScope { CandidateDiscardRecordDelete });
wire_enum!(CandidateDiscardMissingReadbackOperation { Validate });
wire_enum!(CandidateDiscardMissingReadbackScope {
    CandidateDiscardRecordMissingReadback
});

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CandidateDiscardConfirmationSlot {
    RecordDelete,
    RecordMissingReadback,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateDiscardDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CandidateDiscardDeleteOperation,
    pub scope: CandidateDiscardDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateDiscardMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CandidateDiscardMissingReadbackOperation,
    pub scope: CandidateDiscardMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretCandidateDiscardHardwareConfirmStep {
    RecordDelete(SecretCandidateDiscardDeleteHardwareConfirmStep),
    RecordMissingReadback(SecretCandidateDiscardMissingHardwareConfirmStep),
}

impl SecretCandidateDiscardHardwareConfirmStep {
    fn operation_id(&self) -> &SecretOperationId {
        match self {
            Self::RecordDelete(step) => &step.operation_id,
            Self::RecordMissingReadback(step) => &step.operation_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretCandidateDiscardPreparationViewRepr {
    Prepared {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
    },
    ConfirmationRequired {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        step: SecretCandidateDiscardHardwareConfirmStep,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretCandidateDiscardPreparationView(
    SecretCandidateDiscardPreparationViewRepr,
);

impl SecretCandidateDiscardPreparationView {
    fn checked(
        repr: SecretCandidateDiscardPreparationViewRepr,
    ) -> Result<Self, SecretInternalError> {
        if let SecretCandidateDiscardPreparationViewRepr::ConfirmationRequired {
            operation_id,
            step,
            ..
        } = &repr
        {
            if step.operation_id() != operation_id {
                return Err(SecretInternalError::input_invalid());
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretCandidateDiscardPreparationView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

wire_enum!(DiscardedCandidateTerminalState { Discarded });
wire_enum!(ExpiredCandidateTerminalState { Expired });
wire_enum!(RefreshSummaryAction { RefreshSummary });

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum DiscardSecretCandidateResultRepr {
    Discarded {
        terminal_state: DiscardedCandidateTerminalState,
        candidate_id: SecretCandidateId,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyDiscarded {
        terminal_state: DiscardedCandidateTerminalState,
        candidate_id: SecretCandidateId,
        audit_event_id: SecretAuditEventId,
    },
    Expired {
        terminal_state: ExpiredCandidateTerminalState,
        candidate_id: SecretCandidateId,
        action: RefreshSummaryAction,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyExpired {
        terminal_state: ExpiredCandidateTerminalState,
        candidate_id: SecretCandidateId,
        action: RefreshSummaryAction,
        audit_event_id: SecretAuditEventId,
    },
}

pub struct DiscardSecretCandidateResult(DiscardSecretCandidateResultRepr);

impl DiscardSecretCandidateResult {
    fn checked_from_candidate_journal(
        repr: DiscardSecretCandidateResultRepr,
        journal: &CandidateDeleteJournalRow,
    ) -> Result<Self, SecretInternalError> {
        let DiscardCandidateJournalPhase::Terminal {
            terminal_disposition,
        } = &journal.phase
        else {
            return Err(SecretInternalError::input_invalid());
        };
        if *terminal_disposition != journal.terminal_disposition {
            return Err(SecretInternalError::input_invalid());
        }
        let matched = match (&repr, journal.terminal_disposition) {
            (
                DiscardSecretCandidateResultRepr::Discarded {
                    terminal_state: DiscardedCandidateTerminalState::Discarded,
                    candidate_id,
                    ..
                }
                | DiscardSecretCandidateResultRepr::AlreadyDiscarded {
                    terminal_state: DiscardedCandidateTerminalState::Discarded,
                    candidate_id,
                    ..
                },
                CandidateTerminalState::Discarded,
            )
            | (
                DiscardSecretCandidateResultRepr::Expired {
                    terminal_state: ExpiredCandidateTerminalState::Expired,
                    action: RefreshSummaryAction::RefreshSummary,
                    candidate_id,
                    ..
                }
                | DiscardSecretCandidateResultRepr::AlreadyExpired {
                    terminal_state: ExpiredCandidateTerminalState::Expired,
                    action: RefreshSummaryAction::RefreshSummary,
                    candidate_id,
                    ..
                },
                CandidateTerminalState::Expired,
            ) => candidate_id == &journal.candidate.candidate_id,
            _ => false,
        };
        if matched {
            Ok(Self(repr))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

impl Serialize for DiscardSecretCandidateResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSecretLockedRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub locked: bool,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSecretDeleteImpactRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub secret_ref: SecretRef,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub expected_record_revision: SecretRecordRevision,
}

#[derive(Deserialize)]
#[serde(
    tag = "role",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CheckSecretApplyReadinessRequest {
    Target {
        schema_version: SchemaVersionV1,
        owner: SecretOwner,
        consumer: SecretConsumer,
        target_sink: ApplyTargetSink,
        live_sink_id: CodexLiveSecretSinkId,
    },
    Rollback {
        schema_version: SchemaVersionV1,
        owner: SecretOwner,
        consumer: SecretConsumer,
        target_sink: ApplyTargetSink,
        live_sink_id: CodexLiveSecretSinkId,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSecretCleanupImpactRequest {
    pub schema_version: SchemaVersionV1,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetrySecretCleanupRequest {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
    pub expected_recovery_cas: SecretRecoveryCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrateLegacyCodexSecretsRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretAuditRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub secret_ref: Option<SecretRef>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub actions: Option<Vec<SecretAuditAction>>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub outcomes: Option<Vec<SecretAuditOutcome>>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub cursor: Option<SecretAuditCursor>,
    pub limit: PageLimit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretAuditPage {
    events: Vec<SecretAuditEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<SecretAuditCursor>,
}

wire_enum!(SecretValidationOutcome { Valid, Missing, Blocked });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretValidationResult {
    outcome: SecretValidationOutcome,
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretMutationResult {
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

wire_enum!(SecretDeleteStatus { Revoked, AlreadyRevoked });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteResult {
    status: SecretDeleteStatus,
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

impl SecretAuditPage {
    fn checked_from_audit_store(page: SecretAuditPage) -> Result<Self, SecretInternalError> {
        todo!("ordered page, valid events and cursor")
    }
}

impl SecretValidationResult {
    fn checked_from_authority(
        result: SecretValidationResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("outcome/aggregate/audit matrix")
    }
}

impl SecretMutationResult {
    fn checked_from_authority(
        result: SecretMutationResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("aggregate/audit identity")
    }
}

impl SecretDeleteResult {
    fn checked_from_authority(
        result: SecretDeleteResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("status/revocation source/aggregate/audit identity")
    }
}

#[cfg(test)]
fn knife4_absent_domain() -> LegacySourceDomainCoverageIdentity {
    LegacySourceDomainCoverageIdentity::checked_from_structural_inventory(
        LegacySourceInventoryRevision::checked_from_structural_generation(1)
            .expect("revision"),
        LegacySourceDomainPresence::Absent,
        0,
    )
    .expect("absent")
}

#[cfg(test)]
fn knife4_present_domain(count: u32) -> LegacySourceDomainCoverageIdentity {
    LegacySourceDomainCoverageIdentity::checked_from_structural_inventory(
        LegacySourceInventoryRevision::checked_from_structural_generation(1)
            .expect("revision"),
        LegacySourceDomainPresence::Present,
        count,
    )
    .expect("present")
}

#[cfg(test)]
fn knife4_clear_receipt() -> LegacySourceCoverageReceipt {
    LegacySourceCoverageReceipt {
        inventory_revision: LegacySourceInventoryRevision::checked_from_structural_generation(1)
            .expect("revision"),
        coverage_identity: CompleteLegacySourceCoverageIdentity::checked_exact_eleven_domains(
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
        )
        .expect("identity"),
        current_scrubbable: CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(
            Vec::new(),
        )
        .expect("empty current"),
        adjacent_blocked: Vec::new(),
    }
}

#[cfg(test)]
fn knife4_blocking_adjacent_receipt() -> LegacySourceCoverageReceipt {
    LegacySourceCoverageReceipt {
        inventory_revision: LegacySourceInventoryRevision::checked_from_structural_generation(1)
            .expect("revision"),
        coverage_identity: CompleteLegacySourceCoverageIdentity::checked_exact_eleven_domains(
            knife4_absent_domain(),
            knife4_present_domain(1),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
            knife4_absent_domain(),
        )
        .expect("identity"),
        current_scrubbable: CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(
            Vec::new(),
        )
        .expect("empty current"),
        adjacent_blocked: vec![
            AdjacentBlockedLegacySourceObservation::checked_from_codex_inventory_bridge(
                SupplementalLegacySourceCategory::ProcessEnvironment,
            ),
        ],
    }
}

#[test]
fn secret_coverage_view_clear_from_empty_receipt() {
    let view = LegacySourceCoverageView::checked_from_coverage_receipt(&knife4_clear_receipt())
        .expect("clear view");
    let json = serde_json::to_value(&view).expect("json");
    assert_eq!(json["state"], "clear");
    assert_eq!(json["currentScrubbable"]["state"], "none");
    assert_eq!(json["currentScrubbable"]["sourceCount"], 0);
    assert_eq!(json["adjacentBlocked"]["state"], "none");
    assert_eq!(json["adjacentBlocked"]["observationCount"], 0);
}

#[test]
fn secret_coverage_view_blocking_from_adjacent_receipt() {
    let view = LegacySourceCoverageView::checked_from_coverage_receipt(
        &knife4_blocking_adjacent_receipt(),
    )
    .expect("blocking view");
    let json = serde_json::to_value(&view).expect("json");
    assert_eq!(json["state"], "blockingSourcesPresent");
    assert_eq!(json["currentScrubbable"]["state"], "none");
    assert_eq!(json["adjacentBlocked"]["state"], "adjacentBlockedSourcesPresent");
    assert_eq!(json["adjacentBlocked"]["observationCount"], 1);
}

#[cfg(test)]
fn knife4_os_backend() -> SecretBackendInstanceView {
    SecretBackendInstanceView::try_registered(
        SecretBackendKind::OsKeyring,
        SecretBackendInstanceId::generate(),
        SecretBackendGeneration::parse(1).expect("generation"),
        SecretBackendAvailability::Available,
        None,
    )
    .expect("backend")
}

#[cfg(test)]
fn knife4_os_capabilities(
    backend: &SecretBackendInstanceView,
) -> SecretRecordCapabilities {
    SecretRecordCapabilities::try_new(
        backend,
        CapabilityRevision::parse(1).expect("capability"),
        DeviceBindingGeneration::parse(1).expect("device binding"),
        DeviceBinding::HostUser,
        StorageResidency::OsProtectedStore,
        SecretOperationConfirmationCapabilities {
            capture_verify: PhysicalConfirmation::Never,
            validate: PhysicalConfirmation::Never,
            resolve_for_apply: PhysicalConfirmation::Never,
            delete: PhysicalConfirmation::Never,
            revoke: PhysicalConfirmation::Never,
        },
        vec![
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeConsumer::ProxyRequest,
            SecretRuntimeConsumer::UsageProbe,
            SecretRuntimeConsumer::CodingPlanUsageProbe,
            SecretRuntimeConsumer::ModelFetch,
        ],
        vec![
            SecretRuntimeSink::ProcessMemory,
            SecretRuntimeSink::ExternalConfigFile,
        ],
        true,
        false,
        BackendRevocationObservationCapability::Unsupported,
    )
    .expect("capabilities")
}

#[cfg(test)]
fn knife4_ready_aggregate() -> SecretRefAggregate {
    let secret_ref = SecretRef::generate();
    let backend = knife4_os_backend();
    let capabilities = knife4_os_capabilities(&backend);
    SecretRefAggregate {
        schema_version: SchemaVersionV1,
        secret_ref: secret_ref.clone(),
        secret_ref_display: SecretRefDisplay::derive_from(&secret_ref),
        purpose: SecretPurpose::CodexApiKey,
        record_revision: SecretRecordRevision::parse(1).expect("record"),
        binding_set_cas: SecretBindingSetCas {
            revision: SecretBindingSetRevision::parse(1).expect("binding set"),
            digest: BindingSetDigest::parse("ab".repeat(32)).expect("digest"),
            count: 0,
        },
        backend,
        capabilities,
        bindings: Vec::new(),
        presence: SecretPresence::Present,
        availability: SecretStableAvailability::Ready,
        lock: None,
        revocation: None,
        issue: None,
        created_at: UtcTimestamp::parse("2026-08-17T17:00:00.000Z".to_string())
            .expect("timestamp"),
        rotated_at: None,
        last_validated_at: None,
    }
}

#[test]
fn secret_ref_aggregate_accepts_consistent_matrix() {
    SecretRefAggregate::checked_from_authority(knife4_ready_aggregate())
        .expect("consistent matrix");
}

#[test]
fn secret_ref_aggregate_rejects_display_ref_mismatch() {
    let mut aggregate = knife4_ready_aggregate();
    aggregate.secret_ref_display = SecretRefDisplay::derive_from(&SecretRef::generate());
    assert!(SecretRefAggregate::checked_from_authority(aggregate).is_err());
}

#[cfg(test)]
fn knife4_owner(owner_id: &str) -> SecretOwner {
    SecretOwner {
        kind: SecretOwnerKind::Provider,
        namespace: SecretOwnerNamespace::parse("codex".to_string()).expect("namespace"),
        owner_id: OwnerId::parse(owner_id.to_string()).expect("owner"),
        slot: SecretSlot::PrimaryApiKey,
    }
}

#[cfg(test)]
fn knife4_unbound_expectation(owner: SecretOwner) -> OwnerBindingExpectation {
    OwnerBindingExpectation::Unbound {
        owner,
        owner_binding_revision: SecretOwnerBindingRevision::parse(1).expect("revision"),
    }
}

#[cfg(test)]
fn knife4_activation_repr(
    owners: Vec<SecretOwner>,
    bindings: Vec<OwnerBindingExpectation>,
) -> SecretCandidateActivationProjectionRepr {
    let backend_instance_id = SecretBackendInstanceId::generate();
    SecretCandidateActivationProjectionRepr {
        contract_version: SecretContractVersionV1::V1,
        operation: SecretCandidateActivationOperation::SecretCandidateActivation,
        candidate_id: SecretCandidateId::generate(),
        candidate_revision: SecretCandidateRevision::parse(1).expect("candidate"),
        kind: SecretCandidateKind::NewBinding,
        comparison_policy: LegacyActivationComparisonPolicy::ExplicitReplacement,
        comparison_impact: LegacyActivationComparisonImpact::ExplicitReplacement {
            user_meaning: ReplaceExistingCredentialMeaning::ReplaceExistingCredential,
            affected_source_count: 0,
            replaces_bound_binding: false,
        },
        secret_ref: SecretRef::generate(),
        purpose: SecretPurpose::CodexApiKey,
        record_revision: SecretRecordRevision::parse(1).expect("record"),
        backend_instance_id: backend_instance_id.clone(),
        backend_generation: SecretBackendGeneration::parse(1).expect("generation"),
        device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
        capability_revision: CapabilityRevision::parse(1).expect("capability"),
        target_owners: owners,
        expected_bindings: bindings,
        legacy_sources_to_scrub: CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(
            Vec::new(),
        )
        .expect("empty scrub"),
        candidate_read: SecretActivationCandidateReadExpectation {
            operation: ActivationCandidateReadOperation::ResolveForApply,
            scope: ActivationCandidateReadScope::ActivationCandidateCompare,
            backend_instance_id,
            backend_generation: SecretBackendGeneration::parse(1).expect("generation"),
            device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
            capability_revision: CapabilityRevision::parse(1).expect("capability"),
            confirmation: PhysicalConfirmation::Never,
        },
        old_record_delete: SecretActivationOldRecordDeleteExpectation::NotApplicable,
        projection_digest: SecretProjectionDigest::parse("cd".repeat(32)).expect("digest"),
    }
}

#[test]
fn secret_activation_projection_validate_repr_owners_match() {
    let owner = knife4_owner("owner-1");
    SecretCandidateActivationProjection::validate_repr(knife4_activation_repr(
        vec![owner.clone()],
        vec![knife4_unbound_expectation(owner)],
    ))
    .expect("owners match");
}

#[test]
fn secret_activation_projection_validate_repr_owners_mismatch() {
    let err = SecretCandidateActivationProjection::validate_repr(knife4_activation_repr(
        vec![knife4_owner("owner-1")],
        vec![knife4_unbound_expectation(knife4_owner("owner-2"))],
    ));
    assert!(err.is_err());
}

fn knife8_snapshot(
    projection: &SecretCandidateActivationProjection,
    candidate: &SecretCandidateSummary,
) -> SecretCandidateAuthoritySnapshot {
    SecretCandidateAuthoritySnapshot::from_staged(
        candidate.candidate_id.clone(),
        candidate.candidate_revision,
        candidate.kind,
        candidate.comparison_policy,
        candidate.secret_ref.clone(),
        candidate.record_revision,
        projection.clone(),
        SecretBindingSetCas {
            revision: SecretBindingSetRevision::parse(1).expect("set"),
            digest: BindingSetDigest::parse("ab".repeat(32)).expect("digest"),
            count: 0,
        },
        Vec::new(),
    )
    .expect("snapshot")
}

fn knife8_summary_from_projection(
    projection: &SecretCandidateActivationProjection,
) -> SecretCandidateSummary {
    let repr = &projection.0;
    let backend = SecretBackendInstanceView::try_registered(
        SecretBackendKind::OsKeyring,
        repr.backend_instance_id.clone(),
        repr.backend_generation,
        SecretBackendAvailability::Available,
        None,
    )
    .expect("backend");
    let capabilities = SecretRecordCapabilities::try_new(
        &backend,
        repr.capability_revision,
        repr.device_binding_generation,
        DeviceBinding::HostUser,
        StorageResidency::OsProtectedStore,
        SecretOperationConfirmationCapabilities {
            capture_verify: PhysicalConfirmation::Never,
            validate: PhysicalConfirmation::Never,
            resolve_for_apply: PhysicalConfirmation::Never,
            delete: PhysicalConfirmation::Never,
            revoke: PhysicalConfirmation::Never,
        },
        vec![
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeConsumer::ProxyRequest,
            SecretRuntimeConsumer::UsageProbe,
            SecretRuntimeConsumer::CodingPlanUsageProbe,
            SecretRuntimeConsumer::ModelFetch,
        ],
        vec![
            SecretRuntimeSink::ProcessMemory,
            SecretRuntimeSink::ExternalConfigFile,
        ],
        true,
        false,
        BackendRevocationObservationCapability::Unsupported,
    )
    .expect("capabilities");
    SecretCandidateSummary {
        schema_version: SchemaVersionV1,
        candidate_id: repr.candidate_id.clone(),
        candidate_revision: repr.candidate_revision,
        kind: repr.kind,
        comparison_policy: repr.comparison_policy,
        comparison_impact: repr.comparison_impact.clone(),
        state: SecretCandidateState::VerifiedPendingPlan,
        secret_ref: repr.secret_ref.clone(),
        secret_ref_display: SecretRefDisplay::derive_from(&repr.secret_ref),
        purpose: repr.purpose,
        record_revision: repr.record_revision,
        backend,
        capabilities,
        target_owners: repr.target_owners.clone(),
        expected_bindings: repr.expected_bindings.clone(),
        legacy_sources_to_scrub: repr.legacy_sources_to_scrub.clone(),
        created_at: UtcTimestamp::parse("2026-01-01T00:00:00.000Z".to_string()).expect("ts"),
        expires_at: UtcTimestamp::parse("2026-01-01T00:00:00.000Z".to_string()).expect("ts"),
        pending_terminal_disposition: None,
        issue: None,
    }
}

#[test]
fn secret_candidate_with_projection_checked_from_matching_snapshot() {
    let owner = knife4_owner("owner-1");
    let projection = SecretCandidateActivationProjection::validate_repr(knife4_activation_repr(
        vec![owner.clone()],
        vec![knife4_unbound_expectation(owner)],
    ))
    .expect("projection");
    let candidate = knife8_summary_from_projection(&projection);
    let snapshot = knife8_snapshot(&projection, &candidate);
    SecretCandidateWithProjection::checked_from_candidate_snapshot(
        SecretCandidateWithProjection {
            candidate,
            activation_projection: projection,
        },
        &snapshot,
    )
    .expect("matching snapshot");
}

#[test]
fn secret_candidate_with_projection_checked_from_mismatched_snapshot_fails_closed() {
    let owner = knife4_owner("owner-1");
    let projection = SecretCandidateActivationProjection::validate_repr(knife4_activation_repr(
        vec![owner.clone()],
        vec![knife4_unbound_expectation(owner)],
    ))
    .expect("projection");
    let mut candidate = knife8_summary_from_projection(&projection);
    candidate.candidate_id = SecretCandidateId::generate();
    let snapshot = knife8_snapshot(&projection, &knife8_summary_from_projection(&projection));
    assert!(
        SecretCandidateWithProjection::checked_from_candidate_snapshot(
            SecretCandidateWithProjection {
                candidate,
                activation_projection: projection,
            },
            &snapshot,
        )
        .is_err()
    );
}

#[test]
fn secret_stage_result_checked_from_matching_snapshot_with_null_impact() {
    let owner = knife4_owner("owner-1");
    let projection = SecretCandidateActivationProjection::validate_repr(knife4_activation_repr(
        vec![owner.clone()],
        vec![knife4_unbound_expectation(owner)],
    ))
    .expect("projection");
    let candidate = knife8_summary_from_projection(&projection);
    let snapshot = knife8_snapshot(&projection, &candidate);
    StageSecretCandidateResult::checked_from_candidate_snapshot(
        StageSecretCandidateResult {
            status: SecretCandidateStageStatus::Staged,
            candidate,
            activation_projection: projection,
            impact: NullableSecretMutationImpact(None),
            audit_event_id: SecretAuditEventId::generate(),
        },
        &snapshot,
    )
    .expect("null impact is allowed");
}

fn knife9_apply_target_repr(count: u32) -> SecretApplyTargetProjectionRepr {
    let owner = knife4_owner("owner-apply-1");
    SecretApplyTargetProjectionRepr {
        role: SecretApplyTargetRole::Target,
        consumer: SecretChangePlanApplyConsumer::ChangePlanApply,
        target_sink: SecretChangePlanApplySink::ExternalConfigFile,
        live_sink_id: CodexLiveSecretSinkId::CodexAuthJsonOpenAiApiKey,
        owner,
        secret_ref: SecretRef::generate(),
        owner_binding_revision: SecretOwnerBindingRevision::parse(1).expect("owner rev"),
        binding_revision: SecretBindingRevision::parse(1).expect("binding"),
        record_revision: SecretRecordRevision::parse(1).expect("record"),
        binding_set_cas: SecretBindingSetCas {
            revision: SecretBindingSetRevision::parse(1).expect("set"),
            digest: BindingSetDigest::parse("ab".repeat(32)).expect("digest"),
            count,
        },
        backend_instance_id: SecretBackendInstanceId::generate(),
        backend_generation: SecretBackendGeneration::parse(1).expect("gen"),
        device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
        capability_revision: CapabilityRevision::parse(1).expect("cap"),
    }
}

#[test]
fn secret_apply_target_validate_repr_nonzero_provider_codex() {
    SecretApplyTargetProjection::validate_repr(knife9_apply_target_repr(1)).expect("ok");
}

#[test]
fn secret_apply_target_validate_repr_zero_count_fails_closed() {
    assert!(SecretApplyTargetProjection::validate_repr(knife9_apply_target_repr(0)).is_err());
}

#[test]
fn secret_apply_plan_validate_repr_from_d2_target() {
    let target = SecretApplyTargetProjection::validate_repr(knife9_apply_target_repr(1)).expect("target");
    SecretApplyPlanProjection::validate_repr(SecretApplyPlanProjectionRepr {
        contract_version: SecretContractVersionV1::V1,
        operation: CodexProviderApplyOperation::CodexProviderApply,
        target,
        rollback: None,
        projection_digest: SecretProjectionDigest::parse("cd".repeat(32)).expect("digest"),
    })
    .expect("plan");
}

#[test]
fn secret_apply_readiness_ready_when_expiry_holds() {
    let target = SecretApplyTargetProjection::validate_repr(knife9_apply_target_repr(1)).expect("target");
    SecretApplyReadiness::checked_from_authority(SecretApplyReadinessRepr::Ready {
        context: SecretApplyReadinessContext {
            schema_version: SchemaVersionV1,
            operation_id: SecretOperationId::generate(),
            projection: SecretApplyCredentialProjection::Target(target),
            checked_at: UtcTimestamp::parse("2026-01-01T00:00:00.000Z".to_string()).expect("checked"),
            expires_at: UtcTimestamp::parse("2026-01-01T00:01:00.000Z".to_string()).expect("expires"),
        },
    })
    .expect("ready");
}

#[test]
fn secret_apply_readiness_expiry_inversion_fails_closed() {
    let target = SecretApplyTargetProjection::validate_repr(knife9_apply_target_repr(1)).expect("target");
    assert!(
        SecretApplyReadiness::checked_from_authority(SecretApplyReadinessRepr::Ready {
            context: SecretApplyReadinessContext {
                schema_version: SchemaVersionV1,
                operation_id: SecretOperationId::generate(),
                projection: SecretApplyCredentialProjection::Target(target),
                checked_at: UtcTimestamp::parse("2026-01-01T00:01:00.000Z".to_string()).expect("checked"),
                expires_at: UtcTimestamp::parse("2026-01-01T00:00:00.000Z".to_string()).expect("expires"),
            },
        })
        .is_err()
    );
}

#[cfg(test)]
fn knife4_terminal_journal(
    disposition: CandidateTerminalState,
    candidate_id: SecretCandidateId,
) -> CandidateDeleteJournalRow {
    let owner = knife4_owner("owner-1");
    CandidateDeleteJournalRow {
        attempt: JournalAttempt::checked(1).expect("attempt"),
        expected_store_revision: SecretStoreRevision::parse(1).expect("store"),
        terminal_disposition: disposition,
        candidate: JournalCandidateIdentity {
            candidate_id,
            candidate_revision: SecretCandidateRevision::parse(1).expect("candidate"),
            candidate_kind: SecretCandidateKind::NewBinding,
            comparison_policy: LegacyActivationComparisonPolicy::ExplicitReplacement,
            comparison_impact: LegacyActivationComparisonImpact::ExplicitReplacement {
                user_meaning: ReplaceExistingCredentialMeaning::ReplaceExistingCredential,
                affected_source_count: 0,
                replaces_bound_binding: false,
            },
        },
        target_owners: NonEmptySortedJournalTargetOwners(vec![owner.clone()]),
        expected_bindings: NonEmptySortedJournalBindingExpectations(vec![
            knife4_unbound_expectation(owner),
        ]),
        record: JournalBackendIdentity {
            device_instance_id: DeviceInstanceId::generate(),
            secret_ref: SecretRef::generate(),
            record_revision: SecretRecordRevision::parse(1).expect("record"),
            binding_set_cas: SecretBindingSetCas {
                revision: SecretBindingSetRevision::parse(1).expect("binding set"),
                digest: BindingSetDigest::parse("ab".repeat(32)).expect("digest"),
                count: 0,
            },
            backend_instance_id: SecretBackendInstanceId::generate(),
            backend_generation: SecretBackendGeneration::parse(1).expect("generation"),
            device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
            capability_revision: CapabilityRevision::parse(1).expect("capability"),
            confirmation: PhysicalConfirmation::Never,
        },
        delete_slot: CandidateDiscardConfirmationSlot::RecordDelete,
        missing_readback_slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
        delete_confirmation: PhysicalConfirmation::Never,
        missing_readback_confirmation: PhysicalConfirmation::Never,
        phase: DiscardCandidateJournalPhase::Terminal {
            terminal_disposition: disposition,
        },
    }
}

#[test]
fn secret_discard_result_matches_discarded_journal() {
    let candidate_id = SecretCandidateId::generate();
    let journal = knife4_terminal_journal(CandidateTerminalState::Discarded, candidate_id.clone());
    DiscardSecretCandidateResult::checked_from_candidate_journal(
        DiscardSecretCandidateResultRepr::Discarded {
            terminal_state: DiscardedCandidateTerminalState::Discarded,
            candidate_id,
            audit_event_id: SecretAuditEventId::generate(),
        },
        &journal,
    )
    .expect("discarded matches terminal journal");
}

#[test]
fn secret_discard_result_expired_requires_refresh_summary_and_terminal() {
    let candidate_id = SecretCandidateId::generate();
    let journal = knife4_terminal_journal(CandidateTerminalState::Expired, candidate_id.clone());
    DiscardSecretCandidateResult::checked_from_candidate_journal(
        DiscardSecretCandidateResultRepr::Expired {
            terminal_state: ExpiredCandidateTerminalState::Expired,
            candidate_id,
            action: RefreshSummaryAction::RefreshSummary,
            audit_event_id: SecretAuditEventId::generate(),
        },
        &journal,
    )
    .expect("expired matches terminal journal with refreshSummary");
}

#[test]
fn secret_discard_result_mismatch_fail_closed() {
    let candidate_id = SecretCandidateId::generate();
    let expired_journal =
        knife4_terminal_journal(CandidateTerminalState::Expired, candidate_id.clone());
    assert!(
        DiscardSecretCandidateResult::checked_from_candidate_journal(
            DiscardSecretCandidateResultRepr::Discarded {
                terminal_state: DiscardedCandidateTerminalState::Discarded,
                candidate_id: candidate_id.clone(),
                audit_event_id: SecretAuditEventId::generate(),
            },
            &expired_journal,
        )
        .is_err(),
        "discarded arm must not match expired journal"
    );

    let mut pending = knife4_terminal_journal(
        CandidateTerminalState::Discarded,
        candidate_id.clone(),
    );
    pending.phase = DiscardCandidateJournalPhase::Intent;
    assert!(
        DiscardSecretCandidateResult::checked_from_candidate_journal(
            DiscardSecretCandidateResultRepr::Discarded {
                terminal_state: DiscardedCandidateTerminalState::Discarded,
                candidate_id: candidate_id.clone(),
                audit_event_id: SecretAuditEventId::generate(),
            },
            &pending,
        )
        .is_err(),
        "pending journal must fail closed"
    );

    let other = SecretCandidateId::generate();
    let journal = knife4_terminal_journal(CandidateTerminalState::Discarded, other);
    assert!(
        DiscardSecretCandidateResult::checked_from_candidate_journal(
            DiscardSecretCandidateResultRepr::Discarded {
                terminal_state: DiscardedCandidateTerminalState::Discarded,
                candidate_id,
                audit_event_id: SecretAuditEventId::generate(),
            },
            &journal,
        )
        .is_err(),
        "candidate mismatch must fail closed"
    );
}

#[cfg(test)]
fn knife10_import_repr(owner: SecretOwner, binding_owner: SecretOwner, source_count: u32) -> StagedSecretImportActivationProjectionRepr {
    let source = LegacySourceExpectation {
        source: LegacySourceRef {
            location_id: LegacySourceLocationId::parse(format!("lsl_{}", "11".repeat(16))).expect("loc"),
            category: LegacySourceCategory::ProviderAuthJson,
            origin: LegacySourceOrigin::SqlImportStaging,
        },
        structural_revision: LegacySourceStructuralRevision::parse(1).expect("src rev"),
    };
    let sources = StagedLegacySourceExpectations::validate(vec![source]).expect("staging sources");
    StagedSecretImportActivationProjectionRepr {
        contract_version: SecretContractVersionV1::V1,
        operation: StagedSecretImportActivationOperation::StagedSecretImportActivation,
        stage_id: ImportStageId::parse(format!("ist_{}", Uuid::new_v4().simple())).expect("stage"),
        owner,
        staged_source_set_cas: StagedSourceSetCas {
            staged_row_revision: StagedRowRevision::parse(1).expect("row"),
            structure_digest: RecoveryStructureDigest::parse("ab".repeat(32)).expect("struct"),
            source_count,
        },
        source_expectations: sources,
        candidate_id: SecretCandidateId::generate(),
        candidate_revision: SecretCandidateRevision::parse(1).expect("cand"),
        comparison_policy: LegacyActivationComparisonPolicy::CandidateEquality,
        comparison_impact: LegacyActivationComparisonImpact::CandidateEquality {
            user_meaning: VerifySameValueMigrationMeaning::VerifySameValueMigration,
        },
        secret_ref: SecretRef::generate(),
        record_revision: SecretRecordRevision::parse(1).expect("record"),
        backend_instance_id: SecretBackendInstanceId::generate(),
        backend_generation: SecretBackendGeneration::parse(1).expect("gen"),
        device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
        capability_revision: CapabilityRevision::parse(1).expect("cap"),
        expected_live_binding: knife4_unbound_expectation(binding_owner),
        projection_digest: SecretProjectionDigest::parse("cd".repeat(32)).expect("digest"),
    }
}

#[test]
fn secret_staged_import_projection_validate_repr_owner_matches() {
    let owner = knife4_owner("owner-import-1");
    StagedSecretImportActivationProjection::validate_repr(knife10_import_repr(
        owner.clone(),
        owner,
        1,
    ))
    .expect("matching owner");
}

#[test]
fn secret_staged_import_projection_validate_repr_owner_mismatch_fails_closed() {
    let owner = knife4_owner("owner-import-1");
    let other = knife4_owner("owner-import-2");
    assert!(
        StagedSecretImportActivationProjection::validate_repr(knife10_import_repr(owner, other, 1))
            .is_err()
    );
}

#[test]
fn secret_staged_import_projection_validate_repr_count_mismatch_fails_closed() {
    let owner = knife4_owner("owner-import-1");
    assert!(
        StagedSecretImportActivationProjection::validate_repr(knife10_import_repr(
            owner.clone(),
            owner,
            2,
        ))
        .is_err()
    );
}
