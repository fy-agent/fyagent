use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;
pub const STATE_MAX_BYTES: usize = 4 * 1024 * 1024;
pub const JOURNAL_MAX_BYTES: usize = 64 * 1024;
pub const AUDIT_MAX_BYTES: usize = 32 * 1024;
pub const HASH_ALGORITHM: &str = "sha256";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HashAlgorithm {
    Sha256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateEnvelope {
    pub schema_version: u8,
    pub hash_algorithm: HashAlgorithm,
    pub payload_sha256: String,
    pub payload: StatePayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatePayload {
    pub device_instance_id: String,
    pub store_revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub backend_instances: Vec<serde_json::Value>,
    pub secrets: Vec<serde_json::Value>,
    pub candidates: Vec<serde_json::Value>,
    pub recoveries: Vec<serde_json::Value>,
    pub owner_bindings: Vec<serde_json::Value>,
    pub owner_migrations: Vec<serde_json::Value>,
    pub managed_artifact_scan: Option<serde_json::Value>,
}

impl StatePayload {
    pub fn empty(device_instance_id: String, timestamp: String) -> Self {
        Self {
            device_instance_id,
            store_revision: 1,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            backend_instances: Vec::new(),
            secrets: Vec::new(),
            candidates: Vec::new(),
            recoveries: Vec::new(),
            owner_bindings: Vec::new(),
            owner_migrations: Vec::new(),
            managed_artifact_scan: None,
        }
    }
}

/// Exactly eight durable journal operation kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JournalOperationKind {
    CaptureCandidate,
    MigrateLegacy,
    RotateCandidate,
    ActivateCandidate,
    DiscardCandidate,
    DeleteSecret,
    DetachProviderOwner,
    StagedImport,
}

impl JournalOperationKind {
    pub const ALL: [Self; 8] = [
        Self::CaptureCandidate,
        Self::MigrateLegacy,
        Self::RotateCandidate,
        Self::ActivateCandidate,
        Self::DiscardCandidate,
        Self::DeleteSecret,
        Self::DetachProviderOwner,
        Self::StagedImport,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CaptureCandidate => "captureCandidate",
            Self::MigrateLegacy => "migrateLegacy",
            Self::RotateCandidate => "rotateCandidate",
            Self::ActivateCandidate => "activateCandidate",
            Self::DiscardCandidate => "discardCandidate",
            Self::DeleteSecret => "deleteSecret",
            Self::DetachProviderOwner => "detachProviderOwner",
            Self::StagedImport => "stagedImport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JournalEnvelope {
    pub schema_version: u8,
    pub operation_id: String,
    pub device_instance_id: String,
    pub operation_kind: JournalOperationKind,
    pub created_at: String,
    pub updated_at: String,
    pub attempt: u32,
}

/// Exactly four recovery arms. stagedImport is not a fifth recovery kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryKind {
    ActivationCleanup,
    CaptureCompensation,
    DeleteFinalization,
    OwnerDetachFinalization,
}

impl RecoveryKind {
    pub const ALL: [Self; 4] = [
        Self::ActivationCleanup,
        Self::CaptureCompensation,
        Self::DeleteFinalization,
        Self::OwnerDetachFinalization,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActivationCleanup => "activationCleanup",
            Self::CaptureCompensation => "captureCompensation",
            Self::DeleteFinalization => "deleteFinalization",
            Self::OwnerDetachFinalization => "ownerDetachFinalization",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEnvelope {
    pub schema_version: u8,
    pub audit_event_id: String,
    pub device_instance_id: String,
    pub created_at: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn canonical_payload_bytes(payload: &StatePayload) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(payload)
}

pub fn envelope_from_payload(payload: StatePayload) -> Result<StateEnvelope, serde_json::Error> {
    let bytes = canonical_payload_bytes(&payload)?;
    Ok(StateEnvelope {
        schema_version: SCHEMA_VERSION,
        hash_algorithm: HashAlgorithm::Sha256,
        payload_sha256: sha256_hex(&bytes),
        payload,
    })
}

pub fn verify_envelope(envelope: &StateEnvelope) -> Result<(), String> {
    if envelope.schema_version != SCHEMA_VERSION {
        return Err("schemaVersion must be 1".to_string());
    }
    if envelope.hash_algorithm != HashAlgorithm::Sha256 {
        return Err("hashAlgorithm must be sha256".to_string());
    }
    let bytes = canonical_payload_bytes(&envelope.payload).map_err(|e| e.to_string())?;
    let actual = sha256_hex(&bytes);
    if actual != envelope.payload_sha256 {
        return Err("payloadSha256 mismatch".to_string());
    }
    if envelope.payload.store_revision < 1 {
        return Err("storeRevision must be >= 1".to_string());
    }
    Ok(())
}
