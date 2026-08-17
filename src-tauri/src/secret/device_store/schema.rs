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
    pub secrets: Vec<StoredSecretRecord>,
    pub candidates: Vec<StoredCandidateRecord>,
    pub recoveries: Vec<serde_json::Value>,
    pub owner_bindings: Vec<StoredOwnerBindingRecord>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredBindingSetCas {
    pub revision: u64,
    pub digest: String,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoredPolicyState {
    Active,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoredRetirementState {
    Live,
    Stale,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredSecretRecord {
    pub secret_ref: String,
    pub purpose: String,
    pub backend_instance_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_locator: Option<String>,
    pub record_revision: u64,
    pub binding_set_cas: StoredBindingSetCas,
    pub backend_generation: u64,
    pub device_binding_generation: u64,
    pub capability_revision: u64,
    pub policy_state: StoredPolicyState,
    pub retirement_state: StoredRetirementState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoredCandidateKind {
    NewBinding,
    ReplaceBinding,
    RotateBindingSet,
    LegacyReconcile,
    LegacyScrubExistingBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoredCandidateState {
    VerifiedPendingPlan,
    Activated,
    Discarded,
    CleanupRequired,
    Expired,
}

impl StoredCandidateState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Activated | Self::Discarded | Self::Expired
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalDisposition {
    Discarded,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredCandidateRecord {
    pub candidate_id: String,
    pub candidate_revision: u64,
    pub kind: StoredCandidateKind,
    pub state: StoredCandidateState,
    pub secret_ref: String,
    pub record_revision: u64,
    pub backend_instance_id: String,
    pub backend_generation: u64,
    pub device_binding_generation: u64,
    pub capability_revision: u64,
    pub created_at: String,
    pub expires_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_terminal_disposition: Option<TerminalDisposition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoredBindingState {
    Unbound,
    Bound,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredOwner {
    pub kind: String,
    pub namespace: String,
    pub owner_id: String,
    pub slot: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredOwnerBindingRecord {
    pub owner: StoredOwner,
    pub purpose: String,
    pub owner_binding_revision: u64,
    pub state: StoredBindingState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_revision: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalError {
    InvalidInput,
    KindPhaseMismatch,
    SlotReuse,
    SlotSkip,
    SlotSwap,
    MissingCheckpoint,
    RoleMismatch,
    AlreadyTerminal,
    InvalidTransition,
    AfterScrubCountMustBeZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteDisposition {
    Deleted,
    AlreadyMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteAppliedRole {
    DiscardRecordDelete,
    ActivationOldRecordDelete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteAppliedCas {
    pub revision: u64,
    pub digest: String,
}

impl DeleteAppliedCas {
    pub fn checked(revision: u64, digest: String) -> Result<Self, JournalError> {
        if revision < 1 || !valid_hex_n(&digest, 64) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self { revision, digest })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateDiscardDeleteCheckpoint {
    pub delete_disposition: DeleteDisposition,
    pub backend_completed_at: String,
    pub delete_applied_cas: DeleteAppliedCas,
}

impl CandidateDiscardDeleteCheckpoint {
    pub fn checked(
        delete_disposition: DeleteDisposition,
        backend_completed_at: String,
        delete_applied_cas: DeleteAppliedCas,
    ) -> Result<Self, JournalError> {
        if !valid_rfc3339_utc_millis(&backend_completed_at) {
            return Err(JournalError::InvalidInput);
        }
        if delete_applied_cas.revision < 1 || !valid_hex_n(&delete_applied_cas.digest, 64) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self {
            delete_disposition,
            backend_completed_at,
            delete_applied_cas,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivationOldRecordDeleteCheckpoint {
    pub delete_disposition: DeleteDisposition,
    pub backend_completed_at: String,
    pub delete_applied_cas: DeleteAppliedCas,
}

impl ActivationOldRecordDeleteCheckpoint {
    pub fn checked(
        delete_disposition: DeleteDisposition,
        backend_completed_at: String,
        delete_applied_cas: DeleteAppliedCas,
    ) -> Result<Self, JournalError> {
        if !valid_rfc3339_utc_millis(&backend_completed_at) {
            return Err(JournalError::InvalidInput);
        }
        if delete_applied_cas.revision < 1 || !valid_hex_n(&delete_applied_cas.digest, 64) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self {
            delete_disposition,
            backend_completed_at,
            delete_applied_cas,
        })
    }

    pub fn try_from_discard(
        _: &CandidateDiscardDeleteCheckpoint,
    ) -> Result<Self, JournalError> {
        Err(JournalError::RoleMismatch)
    }

    pub fn to_durable(&self) -> ActivationOldRecordDurableCheckpoint {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition: self.delete_disposition,
            backend_completed_at: self.backend_completed_at.clone(),
            delete_applied_cas: self.delete_applied_cas.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ActivationOldRecordDurableCheckpoint {
    None,
    OldRecordDeleteApplied {
        delete_disposition: DeleteDisposition,
        backend_completed_at: String,
        delete_applied_cas: DeleteAppliedCas,
    },
}

impl ActivationOldRecordDurableCheckpoint {
    pub fn from_applied(checkpoint: &ActivationOldRecordDeleteCheckpoint) -> Self {
        checkpoint.to_durable()
    }

    pub fn try_from_discard(
        _: &CandidateDiscardDeleteCheckpoint,
    ) -> Result<Self, JournalError> {
        Err(JournalError::RoleMismatch)
    }

    pub fn three_fields(&self) -> Option<(DeleteDisposition, &str, &DeleteAppliedCas)> {
        match self {
            Self::None => None,
            Self::OldRecordDeleteApplied {
                delete_disposition,
                backend_completed_at,
                delete_applied_cas,
            } => Some((
                *delete_disposition,
                backend_completed_at.as_str(),
                delete_applied_cas,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SlotOccupation {
    Unused,
    Consumed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscardSlot {
    RecordDelete,
    RecordMissingReadback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureTerminalOutcome {
    CandidateStaged,
    Compensated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationTerminalOutcome {
    Activated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCompensationLink {
    pub kind: CaptureCompensationKind,
    pub recovery_id: String,
    pub recovery_cas: DeleteAppliedCas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureCompensationKind {
    CaptureCompensation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivationCleanupLink {
    pub kind: ActivationCleanupKind,
    pub recovery_id: String,
    pub recovery_cas: DeleteAppliedCas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationCleanupKind {
    ActivationCleanup,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteFinalizationLink {
    pub kind: DeleteFinalizationKind,
    pub recovery_id: String,
    pub recovery_cas: DeleteAppliedCas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteFinalizationKind {
    DeleteFinalization,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerDetachFinalizationLink {
    pub kind: OwnerDetachFinalizationKind,
    pub recovery_id: String,
    pub recovery_cas: DeleteAppliedCas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OwnerDetachFinalizationKind {
    OwnerDetachFinalization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserDeleteRevocationSource {
    UserDelete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CaptureLikePhase {
    Intent,
    BackendApplied { verify_receipt_id: String },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: String,
        recovery: CaptureCompensationLink,
    },
    Terminal { outcome: CaptureTerminalOutcome },
}

impl CaptureLikePhase {
    pub fn backend_applied(verify_receipt_id: String) -> Result<Self, JournalError> {
        if !valid_hex_n(&verify_receipt_id, 32) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::BackendApplied { verify_receipt_id })
    }

    pub fn recovery_required(
        last_error_code: String,
        recovery_id: String,
        recovery_cas: DeleteAppliedCas,
    ) -> Result<Self, JournalError> {
        if last_error_code.is_empty() || !valid_prefixed_id(&recovery_id, "src_") {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::RecoveryRequired {
            last_error_code,
            recovery: CaptureCompensationLink {
                kind: CaptureCompensationKind::CaptureCompensation,
                recovery_id,
                recovery_cas,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ActivateCandidatePhase {
    Intent,
    StateFinalized,
    ProviderFinalized,
    OldRecordDeleteIntent,
    OldRecordDeleteApplied {
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    },
    RecoveryRequired {
        last_error_code: String,
        checkpoint: ActivationOldRecordDurableCheckpoint,
        recovery: ActivationCleanupLink,
    },
    Terminal { outcome: ActivationTerminalOutcome },
}

impl ActivateCandidatePhase {
    pub fn old_record_delete_applied(
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    ) -> Self {
        Self::OldRecordDeleteApplied { checkpoint }
    }

    pub fn recovery_required_preserving(
        last_error_code: String,
        applied: &ActivationOldRecordDeleteCheckpoint,
        recovery_id: String,
        recovery_cas: DeleteAppliedCas,
    ) -> Result<Self, JournalError> {
        if last_error_code.is_empty() || !valid_prefixed_id(&recovery_id, "src_") {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::RecoveryRequired {
            last_error_code,
            checkpoint: applied.to_durable(),
            recovery: ActivationCleanupLink {
                kind: ActivationCleanupKind::ActivationCleanup,
                recovery_id,
                recovery_cas,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DiscardRecoveryCheckpoint {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DiscardCandidatePhase {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: String,
    },
    RecoveryRequired {
        last_error_code: String,
        checkpoint: DiscardRecoveryCheckpoint,
    },
    Terminal {
        terminal_disposition: TerminalDisposition,
    },
}

impl DiscardCandidatePhase {
    pub fn backend_applied(
        checkpoint: CandidateDiscardDeleteCheckpoint,
    ) -> Self {
        Self::BackendApplied { checkpoint }
    }

    pub fn missing_readback_verified(
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: String,
    ) -> Result<Self, JournalError> {
        if !valid_rfc3339_utc_millis(&missing_checked_at) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::MissingReadbackVerified {
            checkpoint,
            missing_checked_at,
        })
    }

    pub fn delete_checkpoint(&self) -> Option<&CandidateDiscardDeleteCheckpoint> {
        match self {
            Self::BackendApplied { checkpoint }
            | Self::MissingReadbackVerified { checkpoint, .. } => Some(checkpoint),
            Self::RecoveryRequired {
                checkpoint: DiscardRecoveryCheckpoint::BackendApplied { checkpoint },
                ..
            }
            | Self::RecoveryRequired {
                checkpoint: DiscardRecoveryCheckpoint::MissingReadbackVerified { checkpoint, .. },
                ..
            } => Some(checkpoint),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DeleteSecretPhase {
    Intent,
    BackendApplied {
        delete_disposition: DeleteDisposition,
        backend_completed_at: String,
    },
    MissingReadbackVerified { missing_checked_at: String },
    StateFinalized {
        revoked_at: String,
        revocation_source: UserDeleteRevocationSource,
    },
    RecoveryRequired {
        last_error_code: String,
        recovery: DeleteFinalizationLink,
    },
    Terminal {
        revoked_at: String,
        revocation_source: UserDeleteRevocationSource,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DetachProviderOwnerPhase {
    Intent,
    ProviderDetachCommitted { provider_detach_commit_id: String },
    LocalOwnerCasApplied { provider_detach_commit_id: String },
    RecoveryRequired {
        last_error_code: String,
        provider_detach_commit_id: String,
        recovery: OwnerDetachFinalizationLink,
    },
    Terminal { provider_detach_commit_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagedSourceSetCas {
    pub revision: u64,
    pub digest: String,
    pub count: u64,
}

impl StagedSourceSetCas {
    pub fn after_scrub(revision: u64, digest: String, count: u64) -> Result<Self, JournalError> {
        if revision < 1 || !valid_hex_n(&digest, 64) {
            return Err(JournalError::InvalidInput);
        }
        if count != 0 {
            return Err(JournalError::AfterScrubCountMustBeZero);
        }
        Ok(Self {
            revision,
            digest,
            count,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromotedLiveOwner {
    pub owner: StoredOwner,
    pub owner_binding_revision: u64,
    pub provider_row_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StagedImportResumePhase {
    Intent {},
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    },
}

impl StagedImportResumePhase {
    pub fn sources_scrubbed(cas: StagedSourceSetCas) -> Result<Self, JournalError> {
        if cas.count != 0 {
            return Err(JournalError::AfterScrubCountMustBeZero);
        }
        Ok(Self::SourcesScrubbed {
            staged_source_set_cas_after_scrub: cas,
        })
    }

    pub fn cutover_committed(
        cas: StagedSourceSetCas,
        cutover_receipt_id: String,
    ) -> Result<Self, JournalError> {
        if cas.count != 0 {
            return Err(JournalError::AfterScrubCountMustBeZero);
        }
        if !valid_hex_n(&cutover_receipt_id, 32) {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::CutoverCommitted {
            staged_source_set_cas_after_scrub: cas,
            cutover_receipt_id,
        })
    }

    pub fn live_owner_minted(
        cas: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    ) -> Result<Self, JournalError> {
        if cas.count != 0 {
            return Err(JournalError::AfterScrubCountMustBeZero);
        }
        if !valid_hex_n(&cutover_receipt_id, 32) {
            return Err(JournalError::InvalidInput);
        }
        if promoted_live_owner.owner_binding_revision < 1
            || promoted_live_owner.provider_row_revision < 1
        {
            return Err(JournalError::InvalidInput);
        }
        Ok(Self::LiveOwnerMinted {
            staged_source_set_cas_after_scrub: cas,
            cutover_receipt_id,
            promoted_live_owner,
        })
    }

    pub fn local_binding_finalized(
        cas: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    ) -> Result<Self, JournalError> {
        let minted = Self::live_owner_minted(cas, cutover_receipt_id, promoted_live_owner)?;
        match minted {
            Self::LiveOwnerMinted {
                staged_source_set_cas_after_scrub,
                cutover_receipt_id,
                promoted_live_owner,
            } => Ok(Self::LocalBindingFinalized {
                staged_source_set_cas_after_scrub,
                cutover_receipt_id,
                promoted_live_owner,
            }),
            _ => Err(JournalError::InvalidInput),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StagedImportPhase {
    Intent,
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    },
    RecoveryRequired {
        last_error_code: String,
        resume_phase: StagedImportResumePhase,
    },
    Terminal {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: String,
        promoted_live_owner: PromotedLiveOwner,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operationKind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum JournalEnvelope {
    CaptureCandidate {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: CaptureLikePhase,
    },
    MigrateLegacy {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: CaptureLikePhase,
    },
    RotateCandidate {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: CaptureLikePhase,
    },
    ActivateCandidate {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: ActivateCandidatePhase,
    },
    DiscardCandidate {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        terminal_disposition: TerminalDisposition,
        record_delete_slot: SlotOccupation,
        record_missing_readback_slot: SlotOccupation,
        phase: DiscardCandidatePhase,
    },
    DeleteSecret {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: DeleteSecretPhase,
    },
    DetachProviderOwner {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: DetachProviderOwnerPhase,
    },
    StagedImport {
        schema_version: u8,
        operation_id: String,
        device_instance_id: String,
        created_at: String,
        updated_at: String,
        attempt: u32,
        phase: StagedImportPhase,
    },
}

impl JournalEnvelope {
    pub fn schema_version(&self) -> u8 {
        match self {
            Self::CaptureCandidate { schema_version, .. }
            | Self::MigrateLegacy { schema_version, .. }
            | Self::RotateCandidate { schema_version, .. }
            | Self::ActivateCandidate { schema_version, .. }
            | Self::DiscardCandidate { schema_version, .. }
            | Self::DeleteSecret { schema_version, .. }
            | Self::DetachProviderOwner { schema_version, .. }
            | Self::StagedImport { schema_version, .. } => *schema_version,
        }
    }

    pub fn operation_id(&self) -> &str {
        match self {
            Self::CaptureCandidate { operation_id, .. }
            | Self::MigrateLegacy { operation_id, .. }
            | Self::RotateCandidate { operation_id, .. }
            | Self::ActivateCandidate { operation_id, .. }
            | Self::DiscardCandidate { operation_id, .. }
            | Self::DeleteSecret { operation_id, .. }
            | Self::DetachProviderOwner { operation_id, .. }
            | Self::StagedImport { operation_id, .. } => operation_id,
        }
    }

    pub fn device_instance_id(&self) -> &str {
        match self {
            Self::CaptureCandidate { device_instance_id, .. }
            | Self::MigrateLegacy { device_instance_id, .. }
            | Self::RotateCandidate { device_instance_id, .. }
            | Self::ActivateCandidate { device_instance_id, .. }
            | Self::DiscardCandidate { device_instance_id, .. }
            | Self::DeleteSecret { device_instance_id, .. }
            | Self::DetachProviderOwner { device_instance_id, .. }
            | Self::StagedImport { device_instance_id, .. } => device_instance_id,
        }
    }

    pub fn operation_kind(&self) -> JournalOperationKind {
        match self {
            Self::CaptureCandidate { .. } => JournalOperationKind::CaptureCandidate,
            Self::MigrateLegacy { .. } => JournalOperationKind::MigrateLegacy,
            Self::RotateCandidate { .. } => JournalOperationKind::RotateCandidate,
            Self::ActivateCandidate { .. } => JournalOperationKind::ActivateCandidate,
            Self::DiscardCandidate { .. } => JournalOperationKind::DiscardCandidate,
            Self::DeleteSecret { .. } => JournalOperationKind::DeleteSecret,
            Self::DetachProviderOwner { .. } => JournalOperationKind::DetachProviderOwner,
            Self::StagedImport { .. } => JournalOperationKind::StagedImport,
        }
    }

    pub fn attempt(&self) -> u32 {
        match self {
            Self::CaptureCandidate { attempt, .. }
            | Self::MigrateLegacy { attempt, .. }
            | Self::RotateCandidate { attempt, .. }
            | Self::ActivateCandidate { attempt, .. }
            | Self::DiscardCandidate { attempt, .. }
            | Self::DeleteSecret { attempt, .. }
            | Self::DetachProviderOwner { attempt, .. }
            | Self::StagedImport { attempt, .. } => *attempt,
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self {
            Self::CaptureCandidate {
                phase: CaptureLikePhase::Terminal { .. },
                ..
            }
            | Self::MigrateLegacy {
                phase: CaptureLikePhase::Terminal { .. },
                ..
            }
            | Self::RotateCandidate {
                phase: CaptureLikePhase::Terminal { .. },
                ..
            }
            | Self::ActivateCandidate {
                phase: ActivateCandidatePhase::Terminal { .. },
                ..
            }
            | Self::DiscardCandidate {
                phase: DiscardCandidatePhase::Terminal { .. },
                ..
            }
            | Self::DeleteSecret {
                phase: DeleteSecretPhase::Terminal { .. },
                ..
            }
            | Self::DetachProviderOwner {
                phase: DetachProviderOwnerPhase::Terminal { .. },
                ..
            }
            | Self::StagedImport {
                phase: StagedImportPhase::Terminal { .. },
                ..
            } => true,
            _ => false,
        }
    }
}

pub fn valid_hex_n(value: &str, n: usize) -> bool {
    value.len() == n
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn valid_prefixed_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix) && valid_hex_n(&value[prefix.len()..], 32)
}

pub fn valid_rfc3339_utc_millis(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.offset().local_minus_utc() == 0)
        .unwrap_or(false)
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
