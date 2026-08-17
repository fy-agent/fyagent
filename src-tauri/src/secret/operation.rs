// Process-local, single-use capture-flow registry. The public id is lookup
// text only; the complete owner/binding/legacy/backend snapshot is private.
struct SecretCaptureLegacyExpectation {
    coverage: LegacySourceCoverageReceipt,
    expected_hidden_binding: Option<OwnerBindingExpectation>,
}

struct SecretCaptureBackendSelectionExpectation {
    instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    registered_backend: RegisteredBackendHandleBinding,
}

struct SecretCaptureIntentRegistration {
    owner: ExistingSecretOwnerToken,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    owner_binding: OwnerBindingExpectation,
    legacy: SecretCaptureLegacyExpectation,
    selectable_backends: Vec<SecretCaptureBackendSelectionExpectation>,
    expires_at: UtcTimestamp,
}

pub(crate) struct ClaimedSecretCaptureIntent {
    registration: SecretCaptureIntentRegistration,
    selected_backend: SecretCaptureBackendSelectionExpectation,
    claim_id: [u8; 16],
}

pub(crate) trait SecretCaptureIntentRegistry: Send + Sync {
    // Called only through BackendOperationBroker after list_secret_backend_options has
    // atomically read owner identity, purpose, requested intent, current
    // owner-binding, current-scrubbable/adjacent-blocked coverage receipt,
    // hidden binding and the exact
    // registered backend option set. It mints a fresh short-lived id and
    // returns the output-only view plus options derived from that same row.
    fn mint_from_atomic_snapshot(
        &self,
        registration: SecretCaptureIntentRegistration,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError>;

    // Atomic Ready -> Claimed and single use. begin_secret_capture supplies
    // only the public id and selected instance id; the registry resolves the
    // exact registered handle, then revalidates the whole snapshot before any
    // native material prompt, candidate mint or backend write.
    fn claim_once(
        &self,
        capture_intent_id: SecretCaptureIntentId,
        backend_instance_id: &SecretBackendInstanceId,
        now: &UtcTimestamp,
    ) -> Result<ClaimedSecretCaptureIntent, SecretInternalError>;

    fn consume(
        &self,
        claim: ClaimedSecretCaptureIntent,
    ) -> Result<(), SecretInternalError>;

    fn terminalize(
        &self,
        claim: ClaimedSecretCaptureIntent,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

// The registration/claim/backend-binding fields are private; none implements
// Clone/Serialize/Deserialize/Debug. legacyReconcile requires a non-empty
// current-scrubbable coverage and the exact hidden binding expectation. New and
// replacement intents enforce their matching current binding state. Expiry,
// replay or any snapshot drift is zero-write and cannot reuse a candidate.

struct SecretCapabilityId([u8; 16]);

struct SecretCapabilityClaim {
    capability_id: SecretCapabilityId,
    claim_id: [u8; 16],
}

pub(crate) trait SecretCapabilityRegistry: Send + Sync {
    // Registration is called only by BackendOperationBroker after operation has
    // constructed the private, fully-bound registration row. The registry
    // mints the id and returns the registered consuming capability; callers
    // never submit or reconstruct an id.
    fn register_prepared(
        &self,
        registration: PreparedCapabilityRegistration,
    ) -> Result<PreparedSecretCapability, SecretInternalError>;

    // Atomic prepared -> revalidating. Any other state returns
    // SECRET_CAPABILITY_CONSUMED without exposing registry state.
    fn claim_prepared(
        &self,
        capability: &PreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<SecretCapabilityClaim, SecretInternalError>;

    // Atomic revalidating -> consumed after successful revalidation.
    fn mark_consumed(
        &self,
        claim: SecretCapabilityClaim,
    ) -> Result<(), SecretInternalError>;

    // Any failed revalidation is terminal; it cannot return to prepared.
    fn invalidate(
        &self,
        claim: SecretCapabilityClaim,
        code: SecretErrorCode,
    );

    // Atomic prepared -> discarded; used for unused roles/cancel/expiry.
    fn terminalize_prepared(
        &self,
        capability: &PreparedSecretCapability,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError>;
}

pub(in crate::secret) struct SecretReadinessId([u8; 16]);

enum SecretReadinessKindRepr {
    Delete {
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
    },
    Recovery {
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        pending_steps: NonEmptySortedRecoverySteps,
    },
}

pub(in crate::secret) struct SecretReadinessKind(SecretReadinessKindRepr);

impl SecretReadinessKind {
    fn delete(
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
    ) -> Self {
        Self(SecretReadinessKindRepr::Delete {
            secret_ref,
            record_revision,
            store_revision,
            binding_set_cas,
            backend_instance_id,
            backend_generation,
            device_binding_generation,
            capability_revision,
        })
    }

    fn recovery(
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        pending_steps: NonEmptySortedRecoverySteps,
    ) -> Self {
        Self(SecretReadinessKindRepr::Recovery {
            recovery_id,
            recovery_kind,
            recovery_cas,
            pending_steps,
        })
    }
}

pub(in crate::secret) struct SecretReadinessRegistration {
    operation_id: SecretOperationId,
    kind: SecretReadinessKind,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct SecretReadinessHandle {
    readiness_id: SecretReadinessId,
    operation_id: SecretOperationId,
}

pub(in crate::secret) struct SecretReadinessClaim {
    readiness_id: SecretReadinessId,
    operation_id: SecretOperationId,
    kind: SecretReadinessKind,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) trait SecretReadinessRegistry: Send + Sync {
    // Process-local only. crate::secret::operation is the sole registration
    // caller and provides a freshly native-minted operation id. The registry
    // returns an opaque handle; only the textual operation id enters a DTO.
    fn mint(
        &self,
        registration: SecretReadinessRegistration,
    ) -> Result<SecretReadinessHandle, SecretInternalError>;

    // Atomic Ready -> Claimed. The lookup id is never authority by itself:
    // the closed expected kind/CAS and expiry are compared before claiming.
    // Missing/claimed/consumed map to SECRET_CONFIRMATION_REPLAYED; an expired
    // ready row maps to SECRET_CONFIRMATION_EXPIRED after becoming terminal.
    // Delete identity drift maps DEPENDENCY_CHANGED and recovery
    // kind/CAS/pending-step drift maps RECOVERY_CHANGED, also terminal. None
    // can re-open authorization.
    fn claim_once(
        &self,
        operation_id: &SecretOperationId,
        expected: &SecretReadinessKind,
        now: &UtcTimestamp,
    ) -> Result<SecretReadinessClaim, SecretInternalError>;

    fn consume(
        &self,
        claim: SecretReadinessClaim,
    ) -> Result<(), SecretInternalError>;

    fn expire(
        &self,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;

    fn terminate(
        &self,
        claim: SecretReadinessClaim,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

impl SecretReadinessRegistration {
    // The type is visible to the registry impl, but fields are private and this
    // checked factory is private to crate::secret::operation.
    fn checked(
        operation_id: SecretOperationId,
        kind: SecretReadinessKind,
        expires_at: UtcTimestamp,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate future expiry and exact delete/recovery identity")
    }
}

// SecretReadinessId/Handle/Claim/Registration are non-Serialize,
// non-Deserialize, non-Clone and non-Debug. The registry keeps terminal
// tombstones through the maximum replay window; operationId is lookup only.

pub(crate) struct PreparedSecretCapability {
    capability_id: SecretCapabilityId,
    operation_id: SecretOperationId,
    plan_identity: OwnedAdmittedSecretChangePlanIdentity,
    role: SecretApplyRole,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(in crate::secret) struct PreparedCapabilityRegistration {
    // Same complete identity as PreparedSecretCapability, without an id.
    prepared: PreparedSecretCapabilityWithoutId,
}

pub(in crate::secret) struct PreparedSecretCapabilityWithoutId {
    operation_id: SecretOperationId,
    plan_identity: OwnedAdmittedSecretChangePlanIdentity,
    role: SecretApplyRole,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

impl PreparedCapabilityRegistration {
    // Private to crate::secret::operation; the complete value is assembled
    // only after projection/admission/backend authorization validation.
    fn from_prepared(prepared: PreparedSecretCapabilityWithoutId) -> Self {
        Self { prepared }
    }
}

// PreparedSecretCapability has private fields and no
// Serialize, Deserialize, Clone or Debug implementation.

pub(crate) struct PreparedSecretCapabilityBundle {
    admitted_plan: AdmittedSecretChangePlan,
    operation_id: SecretOperationId,
    projection: SecretApplyPlanProjection,
    target: PreparedCapabilityRoleSlot,
    rollback: Option<PreparedCapabilityRoleSlot>,
}

enum PreparedCapabilityRoleSlot {
    Prepared(PreparedSecretCapability),
    Consumed,
    Discarded,
}

impl PreparedCapabilityRoleSlot {
    fn prepared_ref(&self) -> Result<&PreparedSecretCapability, SecretInternalError> {
        match self {
            Self::Prepared(capability) => Ok(capability),
            Self::Consumed | Self::Discarded => {
                Err(SecretInternalError::capability_consumed())
            }
        }
    }

    fn take_prepared(
        &mut self,
    ) -> Result<PreparedSecretCapability, SecretInternalError> {
        match std::mem::replace(self, Self::Consumed) {
            Self::Prepared(capability) => Ok(capability),
            Self::Consumed => Err(SecretInternalError::capability_consumed()),
            Self::Discarded => {
                *self = Self::Discarded;
                Err(SecretInternalError::capability_consumed())
            }
        }
    }

    fn discard(
        &mut self,
    ) -> Result<Option<PreparedSecretCapability>, SecretInternalError> {
        match std::mem::replace(self, Self::Discarded) {
            Self::Prepared(capability) => Ok(Some(capability)),
            Self::Consumed | Self::Discarded => Ok(None),
        }
    }
}

pub(crate) struct ClaimedPreparedSecretCapability {
    capability: PreparedSecretCapability,
    claim: SecretCapabilityClaim,
}

impl PreparedSecretCapabilityBundle {
    // The only role extraction path. It changes the role slot before the
    // capability is returned, so a writer error/panic cannot make it prepared
    // again and safe Rust cannot borrow one role while moving the other.
    pub(in crate::secret) fn claim_role_for_revalidation(
        &mut self,
        role: SecretApplyRole,
        broker: &BackendOperationBroker,
        now: &UtcTimestamp,
    ) -> Result<ClaimedPreparedSecretCapability, SecretInternalError> {
        let slot = match role {
            SecretApplyRole::Target => &mut self.target,
            SecretApplyRole::Rollback => self.rollback.as_mut()
                .ok_or_else(SecretInternalError::capability_consumed)?,
        };
        // Atomic registry claim happens while the exact role remains in its
        // Prepared slot. Only a successful claim permits the subsequent move.
        let claim = broker.claim_prepared_capability(slot.prepared_ref()?, now)?;
        let capability = slot.take_prepared()?;
        Ok(ClaimedPreparedSecretCapability { capability, claim })
    }

    pub(in crate::secret) fn terminalize_remaining(
        &mut self,
        broker: &BackendOperationBroker,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError> {
        if let Ok(target) = self.target.prepared_ref() {
            broker.terminalize_prepared_capability(target, code)?;
        }
        let _ = self.target.discard()?;
        if let Some(rollback) = self.rollback.as_mut() {
            if let Ok(capability) = rollback.prepared_ref() {
                broker.terminalize_prepared_capability(capability, code)?;
            }
            let _ = rollback.discard()?;
        }
        Ok(())
    }

    pub(in crate::secret) fn projection(&self) -> &SecretApplyPlanProjection {
        &self.projection
    }

    pub(in crate::secret) fn admitted_plan(&self) -> &AdmittedSecretChangePlan {
        &self.admitted_plan
    }

    pub(in crate::secret) fn into_finish_parts(
        self,
    ) -> (AdmittedSecretChangePlan, SecretOperationId) {
        (self.admitted_plan, self.operation_id)
    }
}

// The bundle, role slots and both capabilities are
// non-Serialize/non-Deserialize/non-Clone/non-Debug.

pub(crate) struct PendingSecretConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    plan: AdmittedSecretChangePlan,
    projection: SecretApplyPlanProjection,
    prepared_target: Option<PreparedSecretCapability>,
    prepared_rollback: Option<PreparedSecretCapability>,
    pending_role: SecretApplyRole,
    step: SecretApplyHardwareConfirmStep,
    pending: BackendPendingConfirmation,
}

// PendingSecretConfirmation is also non-Serialize/non-Clone/non-Debug.

pub(crate) struct PendingSecretConfirmationId([u8; 16]);

pub(crate) trait PendingSecretConfirmationRegistry: Send + Sync {
    // BackendOperationBroker is the only caller. The registry mints the id
    // and atomically records the opaque state before a step is returned.
    fn register_pending(
        &self,
        registration: PendingConfirmationRegistration,
    ) -> Result<RegisteredPendingConfirmation, SecretInternalError>;

    // Each operation is atomic and terminal. Missing/terminal ids map to replayed.
    fn claim_confirm(
        &self,
        id: &PendingSecretConfirmationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;

    fn mark_confirmed(
        &self,
        id: PendingSecretConfirmationId,
    ) -> Result<(), SecretInternalError>;

    fn terminate(
        &self,
        id: PendingSecretConfirmationId,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

pub(in crate::secret) struct PendingConfirmationRegistration {
    operation_id: SecretOperationId,
    expires_at: UtcTimestamp,
    backend_pending: BackendPendingConfirmation,
    kind: PendingConfirmationKind,
}

pub(in crate::secret) struct RegisteredPendingConfirmation {
    id: PendingSecretConfirmationId,
    backend_pending: BackendPendingConfirmation,
}

impl RegisteredPendingConfirmation {
    fn into_parts(
        self,
    ) -> (PendingSecretConfirmationId, BackendPendingConfirmation) {
        (self.id, self.backend_pending)
    }
}

pub(in crate::secret) enum PendingConfirmationKind {
    Apply(SecretApplyRole),
    CandidateDiscard(CandidateDiscardConfirmationSlot),
    Activation(ActivationConfirmationSlot),
    Recovery(RecoveryConfirmationSlot),
    StagedImport(StagedImportConfirmationSlot),
}

impl PendingConfirmationRegistration {
    // Private to crate::secret::operation; registry ids are never caller input.
    fn from_backend_pending(
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
        backend_pending: BackendPendingConfirmation,
        kind: PendingConfirmationKind,
    ) -> Self {
        Self {
            operation_id,
            expires_at,
            backend_pending,
            kind,
        }
    }
}

pub(crate) struct SecretApplyPreparationView {
    _private: (),
}

pub(crate) enum PrepareForApply {

    Prepared {
        public: SecretApplyPreparationView,
        capabilities: PreparedSecretCapabilityBundle,
    },
    ConfirmationRequired {
        public: SecretApplyPreparationView,
        pending: PendingSecretConfirmation,
    },
}

pub(crate) struct PreparedCandidateDiscardRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedCandidateDiscardRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedCandidateDiscardBundle {
    operation_id: SecretOperationId,
    journal: CandidateDeleteJournalRow,
    record_delete: PreparedCandidateDiscardRecordDelete,
    record_missing_readback: PreparedCandidateDiscardRecordMissingReadback,
    expires_at: UtcTimestamp,
}

impl PreparedCandidateDiscardBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        SecretOperationId,
        CandidateDeleteJournalRow,
        PreparedCandidateDiscardRecordDelete,
        PreparedCandidateDiscardRecordMissingReadback,
    ) {
        (
            self.operation_id,
            self.journal,
            self.record_delete,
            self.record_missing_readback,
        )
    }
}

pub(crate) enum PendingCandidateDiscardConfirmation {
    RecordDelete {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        journal: CandidateDeleteJournalRow,
        step: SecretCandidateDiscardHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    RecordMissingReadback {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        journal: CandidateDeleteJournalRow,
        prepared_record_delete: PreparedCandidateDiscardRecordDelete,
        step: SecretCandidateDiscardHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) enum PrepareCandidateDiscard {
    AlreadyTerminal(DiscardSecretCandidateResult),
    Prepared {
        public: SecretCandidateDiscardPreparationView,
        bundle: PreparedCandidateDiscardBundle,
    },
    ConfirmationRequired {
        public: SecretCandidateDiscardPreparationView,
        pending: PendingCandidateDiscardConfirmation,
    },
}

// Both slots are prepared/confirmed before the first backend mutation. The
// missing slot may therefore be pre-confirmed, but its authorization remains
// unusable until its operation-owned reservation is fulfilled by the exact
// durable CandidateDiscardDeleteCheckpoint minted after delete.

pub(crate) struct PreparedActivationCandidateRead {
    operation_id: SecretOperationId,
    candidate_record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedActivationOldRecordDelete {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_pre_activation_binding_set: SecretBindingSetCas,
    required_post_activation_binding_state: ActivationOldRecordPostBindingState,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedActivationOldRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedActivationOldRecordDelete),
}

pub(crate) struct PreparedActivationOldRecordMissingReadback {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_pre_activation_binding_set: SecretBindingSetCas,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedActivationOldRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedActivationOldRecordMissingReadback),
}

pub(crate) struct PreparedCandidateActivationBundle {
    admitted_plan: AdmittedSecretChangePlan,
    operation_id: SecretOperationId,
    projection: SecretCandidateActivationProjection,
    candidate_read: PreparedActivationCandidateRead,
    old_record_delete: PreparedActivationOldRecordDeleteSlot,
    old_record_missing_readback: PreparedActivationOldRecordMissingReadbackSlot,
}

impl PreparedCandidateActivationBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        AdmittedSecretChangePlan,
        SecretOperationId,
        SecretCandidateActivationProjection,
        PreparedActivationCandidateRead,
        PreparedActivationOldRecordDeleteSlot,
        PreparedActivationOldRecordMissingReadbackSlot,
    ) {
        (
            self.admitted_plan,
            self.operation_id,
            self.projection,
            self.candidate_read,
            self.old_record_delete,
            self.old_record_missing_readback,
        )
    }
}

pub(crate) enum ActivationConfirmationSlot {
    CandidateRead,
    OldRecordDelete,
    OldRecordMissingReadback,
}

pub(crate) struct PendingCandidateActivationConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    plan: AdmittedSecretChangePlan,
    projection: SecretCandidateActivationProjection,
    prepared_candidate_read: Option<PreparedActivationCandidateRead>,
    prepared_old_record_delete: Option<PreparedActivationOldRecordDeleteSlot>,
    prepared_old_record_missing_readback:
        Option<PreparedActivationOldRecordMissingReadbackSlot>,
    pending_slot: ActivationConfirmationSlot,
    step: SecretActivationHardwareConfirmStep,
    pending: BackendPendingConfirmation,
}

pub(crate) enum PrepareCandidateActivation {
    Prepared {
        public: SecretActivationPreparationView,
        bundle: PreparedCandidateActivationBundle,
    },
    ConfirmationRequired {
        public: SecretActivationPreparationView,
        pending: PendingCandidateActivationConfirmation,
    },
}

// These three token types are defined in crate::commands::import_export.
// Only ImportCoordinator::scan_temp_database_structure can mint them, from one open temp
// Database object. The live-object identity is process-opaque, non-Clone and
// has no path/string/serde representation.
struct TempDatabaseProcessNonce([u8; 16]);

struct TempDatabaseAuthorityIdentity {
    durable_object_id: TempDatabaseDurableObjectId,
    process_nonce: TempDatabaseProcessNonce,
}

pub(crate) struct TempDatabaseLiveObjectIdentity {
    authority: std::sync::Arc<TempDatabaseAuthorityIdentity>,
}

// Random opaque id persisted by the import coordinator in its stage registry.
// It is neither a path nor a content/value digest and has no public serde/text
// conversion. A reopened temp DB proves this id from its own durable stage row
// before a fresh live-object identity may be minted.
pub(crate) struct TempDatabaseDurableObjectId([u8; 16]);
pub(crate) struct ImportCutoverReceiptId([u8; 16]);
pub(crate) struct StagedImportAdmissionId([u8; 16]);

pub(crate) struct StagedSecretOwnerToken {
    stage_id: ImportStageId,
    temp_database: TempDatabaseLiveObjectIdentity,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
}

pub(crate) struct StagedSecretOwnerIdentity<'a> {
    stage_id: &'a ImportStageId,
    temp_database: &'a TempDatabaseLiveObjectIdentity,
    owner: &'a SecretOwner,
    staged_row_revision: StagedRowRevision,
}

impl StagedSecretOwnerToken {
    // This implementation resides in crate::commands::import_export; #35 sees
    // only this immutable view and cannot construct/replay a staged token.
    pub(crate) fn identity(&self) -> StagedSecretOwnerIdentity<'_> {
        StagedSecretOwnerIdentity {
            stage_id: &self.stage_id,
            temp_database: &self.temp_database,
            owner: &self.owner,
            staged_row_revision: self.staged_row_revision,
        }
    }
}

impl StagedSecretOwnerIdentity<'_> {
    pub(crate) fn stage_id(&self) -> &ImportStageId { self.stage_id }
    pub(crate) fn temp_database(&self) -> &TempDatabaseLiveObjectIdentity {
        self.temp_database
    }
    pub(crate) fn owner(&self) -> &SecretOwner { self.owner }
    pub(crate) fn staged_row_revision(&self) -> StagedRowRevision {
        self.staged_row_revision
    }
}

// The import owner privately creates this binding from the same process-live
// temp DB authority as StagedSecretOwnerToken. #55 may retain it opaquely but
// cannot construct, inspect bytes or substitute another temp DB.
pub(crate) struct StagedImportAdmissionAuthority {
    temp_database: std::sync::Arc<TempDatabaseAuthorityIdentity>,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
}

// Defined and privately minted by crate::change_plan::secret_admission (#55),
// never by the import coordinator or #35.
pub(crate) struct AdmittedStagedSecretImportPlan {
    operation: StagedSecretImportActivationOperation,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    authority: StagedImportAdmissionAuthority,
    admission_id: StagedImportAdmissionId,
}

pub(crate) struct AdmittedStagedSecretImportIdentity<'a> {
    operation: &'a StagedSecretImportActivationOperation,
    plan_id: &'a ChangePlanId,
    plan_digest: &'a ChangePlanDigest,
    projection_digest: &'a SecretProjectionDigest,
    authority: &'a StagedImportAdmissionAuthority,
    admission_id: &'a StagedImportAdmissionId,
}

// Durable journal identity deliberately omits the process nonce. It records
// the old admission and temp object identity so restart can terminate/reconcile
// it, but a fresh process must mint a new live authority and #55 admission.
pub(in crate::secret) struct OwnedAdmittedStagedSecretImportIdentity {
    operation: StagedSecretImportActivationOperation,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    durable_object_id: TempDatabaseDurableObjectId,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    admission_id: StagedImportAdmissionId,
}

impl AdmittedStagedSecretImportPlan {
    pub(crate) fn identity(&self) -> AdmittedStagedSecretImportIdentity<'_> {
        AdmittedStagedSecretImportIdentity {
            operation: &self.operation,
            plan_id: &self.plan_id,
            plan_digest: &self.plan_digest,
            projection_digest: &self.projection_digest,
            authority: &self.authority,
            admission_id: &self.admission_id,
        }
    }
}

// This exact scope is built only by crate::commands::import_export after the
// equality port proves staged token + #55 admission share one live object.
pub(crate) struct StagedImportBackendAuthorityScope {
    temp_database: std::sync::Arc<TempDatabaseAuthorityIdentity>,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    admission_id: StagedImportAdmissionId,
}

pub(crate) struct StagedImportAuthorityMatchReceipt {
    backend_scope: StagedImportBackendAuthorityScope,
    _private: (),
}

pub(crate) struct ImportStagedAuthorityComparator {
    _private: (),
}

mod staged_import_authority_equality_sealed {
    pub(super) trait Sealed {}
    impl Sealed for super::ImportStagedAuthorityComparator {}
}

pub(crate) trait StagedImportAuthorityEqualityPort:
    staged_import_authority_equality_sealed::Sealed + Send + Sync
{
    fn assert_same_live_authority(
        &self,
        staged_owner: StagedSecretOwnerIdentity<'_>,
        admission: AdmittedStagedSecretImportIdentity<'_>,
    ) -> Result<StagedImportAuthorityMatchReceipt, SecretInternalError>;
}

pub(crate) struct ReopenedStagedImportAuthority {
    durable_object_id: TempDatabaseDurableObjectId,
    resume_cas: StagedImportResumeCas,
    _private: (),
}

pub(crate) struct PriorStagedAdmissionTerminalReceipt {
    terminal: StagedPriorAdmissionTerminal,
    _private: (),
}

pub(crate) struct FreshStagedLiveAuthority {
    staged_owner: StagedSecretOwnerToken,
    admission_authority: StagedImportAdmissionAuthority,
    _private: (),
}

pub(crate) trait StagedImportResumeAuthorityPort:
    staged_import_authority_equality_sealed::Sealed + Send + Sync
{
    fn reopen_durable_stage(
        &self,
        request: &ResumeStagedImportCutoverRequest,
    ) -> Result<ReopenedStagedImportAuthority, SecretInternalError>;

    fn reconcile_prior_admission(
        &self,
        reopened: &ReopenedStagedImportAuthority,
        prior: &OwnedAdmittedStagedSecretImportIdentity,
    ) -> Result<PriorStagedAdmissionTerminalReceipt, SecretInternalError>;

    fn mint_fresh_live_authority(
        &self,
        reopened: ReopenedStagedImportAuthority,
        prior_terminal: PriorStagedAdmissionTerminalReceipt,
    ) -> Result<FreshStagedLiveAuthority, SecretInternalError>;
}

pub(crate) struct PreparedStagedImportCandidateRead {
    operation_id: SecretOperationId,
    candidate_record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedStagedImportBundle {
    admitted_plan: AdmittedStagedSecretImportPlan,
    staged_owner: StagedSecretOwnerToken,
    projection: StagedSecretImportActivationProjection,
    candidate_read: PreparedStagedImportCandidateRead,
}

impl PreparedStagedImportBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        AdmittedStagedSecretImportPlan,
        StagedSecretOwnerToken,
        StagedSecretImportActivationProjection,
        PreparedStagedImportCandidateRead,
    ) {
        (
            self.admitted_plan,
            self.staged_owner,
            self.projection,
            self.candidate_read,
        )
    }
}

pub(crate) enum StagedImportConfirmationSlot {
    CandidateRead,
}

pub(crate) struct PendingStagedImportConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    admitted_plan: AdmittedStagedSecretImportPlan,
    staged_owner: StagedSecretOwnerToken,
    projection: StagedSecretImportActivationProjection,
    pending_slot: StagedImportConfirmationSlot,
    pending: BackendPendingConfirmation,
}

pub(crate) enum PrepareStagedImport {
    Prepared(PreparedStagedImportBundle),
    ConfirmationRequired(PendingStagedImportConfirmation),
}

pub(crate) struct StagedImportSourceValidationReceipt {
    _private: (),
}

// Exact device-store operation journal. The common envelope owns operationId,
// durable DeviceInstanceId and timestamps; the process-local
// DeviceSecretStoreInstanceId is never encoded. Each payload below contains authority
// fields unique to one of the eight operation kinds and one independent phase
// algebra. No payload/phase uses Option, flatten or a generic checkpoint bag.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::secret) enum CandidateTerminalState { Discarded, Expired }

pub(in crate::secret) struct JournalAttempt(u32);
pub(in crate::secret) struct DeviceSecretStoreInstanceId([u8; 16]);
pub(in crate::secret) struct BackendVerifyReceiptId([u8; 16]);
pub(in crate::secret) struct DeleteAdmissionId([u8; 16]);
pub(in crate::secret) struct ProviderDetachTransactionId([u8; 16]);

impl JournalAttempt {
    pub(super) fn checked(value: u32) -> Result<Self, SecretInternalError> {
        (value >= 1)
            .then_some(Self(value))
            .ok_or_else(SecretInternalError::input_invalid)
    }
}

pub(in crate::secret) struct JournalBackendIdentity {
    device_instance_id: DeviceInstanceId,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
}

pub(in crate::secret) struct JournalCandidateIdentity {
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    candidate_kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
}

pub(in crate::secret) struct JournalPlanIdentity {
    operation: SecretCandidateActivationOperation,
    admission_id: [u8; 16],
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
}

pub(in crate::secret) struct StagedImportJournalPlanIdentity {
    operation: StagedSecretImportActivationOperation,
    admission_id: StagedImportAdmissionId,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
}

pub(in crate::secret) struct DeleteJournalAdmissionIdentity {
    admission_id: DeleteAdmissionId,
    readiness_operation_id: SecretOperationId,
    admitted_at: UtcTimestamp,
}

pub(in crate::secret) struct NonEmptySortedJournalTargetOwners(Vec<SecretOwner>);
pub(in crate::secret) struct NonEmptySortedJournalBindingExpectations(
    Vec<OwnerBindingExpectation>,
);
pub(in crate::secret) struct NonEmptyCurrentLegacySourceExpectations(
    CurrentLegacySourceExpectations,
);
pub(in crate::secret) struct NonEmptySortedOwnerBindingRevisions(
    Vec<SecretOwnerBindingRevision>,
);

pub(in crate::secret) enum CaptureCandidateSourceAuthority {
    None,
    CurrentExplicitReplacement {
        source_expectations: NonEmptyCurrentLegacySourceExpectations,
    },
}

pub(in crate::secret) enum DurableCandidateSourceAuthority {
    NoLegacySources,
    Current { expectations: CurrentLegacySourceExpectations },
}

pub(in crate::secret) struct DurableSecretCandidateRecord {
    candidate: JournalCandidateIdentity,
    state: SecretCandidateState,
    pending_terminal_disposition: Option<CandidateTerminalState>,
    store_revision: SecretStoreRevision,
    target_owners: NonEmptySortedJournalTargetOwners,
    expected_bindings: NonEmptySortedJournalBindingExpectations,
    source_authority: DurableCandidateSourceAuthority,
    backend: JournalBackendIdentity,
    created_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

impl DurableSecretCandidateRecord {
    pub(super) fn checked(
        record: DurableSecretCandidateRecord,
    ) -> Result<Self, SecretInternalError> {
        todo!("policy/impact/kind/source/owner/backend/store/state/expiry plus pending disposition iff matching nonterminal discard journal invariant")
    }
}

pub(in crate::secret) enum DetachProviderOwnerBindingExpectation {
    Bound {
        secret_ref: SecretRef,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
        remaining_owners: SortedSecretOwners,
    },
    Unbound { remaining_owners: [SecretOwner; 0] },
}

wire_enum!(NoBlockingLegacySourcesState { Clear });
wire_enum!(CandidateEqualityOnly { CandidateEquality });
wire_enum!(ExplicitReplacementOnly { ExplicitReplacement });
wire_enum!(JournalCandidateTerminalOutcome { CandidateStaged, Compensated });
wire_enum!(JournalActivationTerminalOutcome { Activated });
wire_enum!(UserDeleteRevocationSource { UserDelete });
wire_enum!(NoBindingsRequired { NoBindings });
wire_enum!(ImportStageKind { SqlImport, BinaryRestore, SyncDownload });

pub(in crate::secret) struct StagedTempDatabaseJournalIdentity {
    stage_id: ImportStageId,
    stage_kind: ImportStageKind,
    durable_object_id: TempDatabaseDurableObjectId,
    process_nonce: TempDatabaseProcessNonce,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    staged_source_set_cas: StagedSourceSetCas,
}

pub(in crate::secret) struct PromotedLiveOwnerCheckpoint {
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    provider_row_revision: ProviderRowRevision,
}

// This is the complete resume-CAS phase algebra, not a best-effort checkpoint
// bag. Every later arm repeats all receipts from the earlier completed arms so
// omission cannot be confused with an earlier phase. Every
// staged_source_set_cas_after_scrub has source_count=0.
pub(in crate::secret) enum StagedImportResumePhase {
    Intent,
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
}

// Credential-free internal preimage for public {revision,digest}. None of
// these fields is part of ResumeStagedImportCutoverRequest or any resume result arm.
pub(in crate::secret) struct StagedImportResumePreimageIdentity {
    operation_id: SecretOperationId,
    expected_store_revision: SecretStoreRevision,
    stage_authority: StagedTempDatabaseJournalIdentity,
    source_expectations: StagedLegacySourceExpectations,
    candidate: JournalCandidateIdentity,
    admission: StagedImportJournalPlanIdentity,
    record: JournalBackendIdentity,
    expected_live_binding: OwnerBindingExpectation,
}

pub(in crate::secret) struct StagedImportResumePreimage {
    identity: StagedImportResumePreimageIdentity,
    phase: StagedImportResumePhase,
}

impl StagedImportResumeCas {
    pub(super) fn checked_from_internal_preimage(
        revision: StagedImportResumeRevision,
        preimage: &StagedImportResumePreimage,
    ) -> Result<Self, SecretInternalError> {
        todo!("hash only the exact canonical rows above, never raw struct/debug serialization: immutable journal operation id plus the closed stage/source/plan/candidate/comparison/record/backend/live-binding/five-arm cumulative phase fields; every after-scrub CAS has count zero; every phase/nonce/admission/source/CAS/receipt/owner change first increments revision, then recomputes digest; output only revision+digest")
    }
}

pub(in crate::secret) enum StagedPriorAdmissionTerminal {
    Consumed,
    Terminated,
    AlreadyTerminal,
}

pub(in crate::secret) struct ActivationCleanupRecoveryLink {
    kind: ActivationCleanupRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct CaptureCompensationRecoveryLink {
    kind: CaptureCompensationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct DeleteFinalizationRecoveryLink {
    kind: DeleteFinalizationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct OwnerDetachFinalizationRecoveryLink {
    kind: OwnerDetachFinalizationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}

wire_enum!(ActivationCleanupRecoveryKind { ActivationCleanup });
wire_enum!(CaptureCompensationRecoveryKind { CaptureCompensation });
wire_enum!(DeleteFinalizationRecoveryKind { DeleteFinalization });
wire_enum!(OwnerDetachFinalizationRecoveryKind { OwnerDetachFinalization });

pub(in crate::secret) enum CaptureCandidateJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

pub(in crate::secret) enum MigrateLegacyJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

pub(in crate::secret) enum RotateCandidateJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

// OldRecordDeleteApplied is the crash boundary between the two independent
// backend authorizations. A successful fresh-missing receipt is the final old
// record step, so the authority persists supersession and Terminal atomically;
// it never exposes an empty-suffix missing-verified journal phase. This
// delete-specific durable projection is exactly None or the complete
// three-field applied record; ordinary activation progress stays in its own
// journal phase and cannot masquerade as an old-record checkpoint.
pub(in crate::secret) enum ActivationOldRecordDurableCheckpoint {
    None,
    OldRecordDeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
    },
}

pub(in crate::secret) enum ActivateCandidateJournalPhase {
    Intent,
    StateFinalized,
    ProviderFinalized,
    OldRecordDeleteIntent,
    OldRecordDeleteApplied {
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        checkpoint: ActivationOldRecordDurableCheckpoint,
        recovery: ActivationCleanupRecoveryLink,
    },
    Terminal { outcome: JournalActivationTerminalOutcome },
}

pub(in crate::secret) enum DiscardCandidateRecoveryCheckpoint {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: UtcTimestamp,
    },
}

pub(in crate::secret) enum DiscardCandidateJournalPhase {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: UtcTimestamp,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        checkpoint: DiscardCandidateRecoveryCheckpoint,
    },
    Terminal { terminal_disposition: CandidateTerminalState },
}

pub(in crate::secret) enum DeleteSecretJournalPhase {
    Intent,
    BackendApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
    },
    MissingReadbackVerified { missing_checked_at: UtcTimestamp },
    StateFinalized {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: DeleteFinalizationRecoveryLink,
    },
    Terminal {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
    },
}

pub(in crate::secret) enum DetachProviderOwnerJournalPhase {
    Intent,
    ProviderDetachCommitted { provider_detach_commit_id: ProviderDetachCommitId },
    LocalOwnerCasApplied { provider_detach_commit_id: ProviderDetachCommitId },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        provider_detach_commit_id: ProviderDetachCommitId,
        recovery: OwnerDetachFinalizationRecoveryLink,
    },
    Terminal { provider_detach_commit_id: ProviderDetachCommitId },
}

pub(in crate::secret) enum StagedImportJournalPhase {
    Intent,
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        resume_phase: StagedImportResumePhase,
    },
    Terminal {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
}

pub(in crate::secret) enum JournalOldRecordDeleteExpectation {
    NotApplicable,
    DeleteAfterActivation {
        old_record: JournalBackendIdentity,
        required_binding_state: NoBindingsRequired,
        missing_readback_confirmation: PhysicalConfirmation,
    },
}

pub(in crate::secret) struct CandidateDeleteJournalRow {
    attempt: JournalAttempt,
    expected_store_revision: SecretStoreRevision,
    terminal_disposition: CandidateTerminalState,
    candidate: JournalCandidateIdentity,
    target_owners: NonEmptySortedJournalTargetOwners,
    expected_bindings: NonEmptySortedJournalBindingExpectations,
    record: JournalBackendIdentity,
    delete_slot: CandidateDiscardConfirmationSlot,
    missing_readback_slot: CandidateDiscardConfirmationSlot,
    delete_confirmation: PhysicalConfirmation,
    missing_readback_confirmation: PhysicalConfirmation,
    phase: DiscardCandidateJournalPhase,
}

pub(in crate::secret) struct CandidateDeleteIdentity { _private: () }

impl CandidateDeleteJournalRow {
    fn for_explicit_discard(identity: CandidateDeleteIdentity) -> Self {
        Self::checked(identity, CandidateTerminalState::Discarded)
    }

    fn for_expiry_sweep(identity: CandidateDeleteIdentity) -> Self {
        Self::checked(identity, CandidateTerminalState::Expired)
    }

    fn checked(
        identity: CandidateDeleteIdentity,
        terminal_disposition: CandidateTerminalState,
    ) -> Self {
        todo!("copy exact candidate/owner/store/backend identity plus literal RecordDelete/RecordMissingReadback slots and their independent confirmation policies into discardCandidate intent; strict replay accepts only delete -> durable typed BackendApplied -> fresh Validate missing -> MissingReadbackVerified -> immutable terminal sequence")
    }
}

pub(in crate::secret) enum DurableSecretOperationJournalRepr {
    CaptureCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        owner_expectation: OwnerBindingExpectation,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        candidate: JournalCandidateIdentity,
        source_authority: CaptureCandidateSourceAuthority,
        backend: JournalBackendIdentity,
        phase: CaptureCandidateJournalPhase,
    },
    MigrateLegacy {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        migration_report_id: SecretMigrationReportId,
        owner_expectation: OwnerBindingExpectation,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        candidate: JournalCandidateIdentity,
        comparison_policy: CandidateEqualityOnly,
        source_expectations: NonEmptyCurrentLegacySourceExpectations,
        backend: JournalBackendIdentity,
        phase: MigrateLegacyJournalPhase,
    },
    RotateCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        old_record: JournalBackendIdentity,
        expected_old_binding_set: SecretBindingSetCas,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        candidate: JournalCandidateIdentity,
        comparison_policy: ExplicitReplacementOnly,
        new_record: JournalBackendIdentity,
        phase: RotateCandidateJournalPhase,
    },
    ActivateCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        admission: JournalPlanIdentity,
        candidate: JournalCandidateIdentity,
        active_record: JournalBackendIdentity,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        source_expectations: CurrentLegacySourceExpectations,
        old_record_delete: JournalOldRecordDeleteExpectation,
        phase: ActivateCandidateJournalPhase,
    },
    DiscardCandidate { row: CandidateDeleteJournalRow },
    DeleteSecret {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        delete_admission: DeleteJournalAdmissionIdentity,
        record: JournalBackendIdentity,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        expected_owner_binding_revisions: NonEmptySortedOwnerBindingRevisions,
        revocation_source: UserDeleteRevocationSource,
        phase: DeleteSecretJournalPhase,
    },
    DetachProviderOwner {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        provider_detach_transaction_id: ProviderDetachTransactionId,
        detached_owner: SecretOwner,
        expected_owner_binding_revision: SecretOwnerBindingRevision,
        legacy_source_coverage_state: NoBlockingLegacySourcesState,
        binding_view: DetachProviderOwnerBindingExpectation,
        phase: DetachProviderOwnerJournalPhase,
    },
    StagedImport {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        stage_authority: StagedTempDatabaseJournalIdentity,
        admission: StagedImportJournalPlanIdentity,
        candidate: JournalCandidateIdentity,
        source_expectations: StagedLegacySourceExpectations,
        record: JournalBackendIdentity,
        expected_live_binding: OwnerBindingExpectation,
        resume_cas: StagedImportResumeCas,
        phase: StagedImportJournalPhase,
    },
}

pub(in crate::secret) struct DurableSecretOperationJournal {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    device_instance_id: DeviceInstanceId,
    created_at: UtcTimestamp,
    updated_at: UtcTimestamp,
    payload: DurableSecretOperationJournalRepr,
}

impl DurableSecretOperationJournal {
    pub(super) fn checked(
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        device_instance_id: DeviceInstanceId,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
        payload: DurableSecretOperationJournalRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate common envelope plus variant-specific candidate/owner/backend/plan/stage/CAS/phase invariants; discard retains full delete checkpoint and staged resume CAS hashes this operation_id plus the exact cumulative five-arm phase")
    }
}

// Normative codec/replay rules:
// - operationKind is exactly captureCandidate|migrateLegacy|rotateCandidate|
//   activateCandidate|discardCandidate|deleteSecret|detachProviderOwner|
//   stagedImport. There is no ninth generic recovery operation. Each variant
//   uses only its named phase enum and every declared required field is encoded
//   in canonical order; there is no optional property bag.
// - CaptureCandidate preserves the complete sorted target-owner set, a
//   one-to-one sorted OwnerBindingExpectation set, candidate policy+impact,
//   exact source expectations and backend identity. NewBinding may have no
//   sources; ExplicitReplacement requires the exact nonempty admitted source
//   set and replaceExistingCredential impact. Drift is not reconstructible.
// - MigrateLegacy has one owner/binding expectation/current source set;
//   RotateCandidate has the original binding-set CAS, complete affected rows
//   and both backend identities; DeleteSecret aligns
//   every state-finalization revision with the sorted affected owners;
//   ActivateCandidate repeats the opaque #55 admission identity, candidate
//   policy+impact, affected rows, current sources, old-delete expectation and
//   active backend. Its OldRecordDeleteApplied arm embeds the complete
//   ActivationOldRecordDeleteCheckpoint; RecoveryRequired embeds the exact
//   None|OldRecordDeleteApplied durable projection without side state.
// - DiscardCandidate is exactly CandidateDeleteJournalRow. Its generated
//   operation id, candidate/ref/revisions, zero-binding-set CAS and complete
//   backend/device/capability tuple are required. Its terminal state and
//   Intent -> BackendApplied{deleteDisposition,backendCompletedAt,
//   deleteAppliedCas} -> MissingReadbackVerified{the same three fields,
//   missingCheckedAt} -> Terminal sequence cannot be relabelled on replay;
//   there is no candidate-discard StateFinalized arm. RecoveryRequired retains
//   exactly the last complete checkpoint. The RecordMissingReadback authorization is independently
//   prepared/confirmed with Validate policy and remains unusable until the
//   durable BackendApplied CAS reservation is fulfilled.
// - DetachProviderOwner.legacy_source_coverage_state is the required literal
//   Clear; any current-scrubbable or adjacent-blocked occurrence invalidates preview before journal
//   creation. binding is mandatory and only Bound|Unbound. Bound carries
//   ref/per-owner binding revision/binding-set CAS and canonical
//   sorted-unique remaining owners. Unbound carries none and requires the
//   empty array. A current legacy source prevents journal creation entirely.
//   Every arm carries Provider-row + owner-binding revisions and the exact
//   Provider detach transaction id; committed phases add the commit id.
// - StagedImport.admission is the sole staged #55 plan/admission identity;
//   no ordinary activation-plan identity is also present. stage_authority
//   binds stage kind, opaque durable object id, fresh process nonce, owner,
//   staged-row revision and staged-source-set CAS, while the resume preimage
//   additionally binds the fresh operation id. Phase ordering is Intent ->
//   SourcesScrubbed(source-set CAS) -> CutoverCommitted(source-set CAS,
//   receipt) -> LiveOwnerMinted(source-set CAS, receipt, promoted
//   owner/Provider-row/owner-binding checkpoint) -> LocalBindingFinalized(the
//   same three cumulative fields) -> Terminal. RecoveryRequired contains one
//   exact StagedImportResumePhase arm with no optional field bag. A phase,
//   process nonce, admission, receipt or promoted-owner change increments the
//   resume revision before digest recomputation. Terminal currentResumeCas is
//   the exact LocalBindingFinalized projection with all three cumulative
//   fields. ImportCoordinator may reopen only by proving
//   that opaque id from the stage row, then minting a new process live-object
//   identity and rechecking CAS/receipt; no path/snapshot/digest is authority.
// - RecoveryRequired phases contain exactly one typed link to the separately
//   stored activationCleanup|captureCompensation|deleteFinalization|
//   ownerDetachFinalization row. StagedImport instead carries its exact
//   five-arm resume phase. A recovery row is never itself a journal operation
//   variant.
// - Unknown tags/fields, illegal phase payloads, unsorted/duplicate/disjoint
//   sets or candidate/backend/plan/stage/CAS mismatch reject before replay.
//   Only typed structural digests are permitted; material/value digests are
//   forbidden. Startup reconciliation and explicit retry share this decoder.

// Activation bundles/pending state are material-free, non-Serialize,
// non-Deserialize, non-Clone and non-Debug. Preparation authorizes the
// candidate read/compare, planned old delete and fresh old-missing readback as
// three independent slots; confirm may return the next slot. All are ready before #41 may
// acquire its lease. The old-delete authorization is bound to the exact
// expectation already hashed into the activation projection.

wire_enum!(CleanupActiveRecordReadOperation { ResolveForApply });
wire_enum!(CleanupActiveRecordReadScope { CleanupActiveRecordCompare });
wire_enum!(CleanupOldRecordDeleteOperation { Delete });
wire_enum!(CleanupOldRecordDeleteScope { CleanupOldRecordDelete });
wire_enum!(CleanupOldRecordMissingReadbackOperation { Validate });
wire_enum!(CleanupOldRecordMissingReadbackScope {
    CleanupOldRecordMissingReadback
});
wire_enum!(CaptureCompensationDeleteOperation { Delete });
wire_enum!(CaptureCompensationDeleteScope { CaptureCompensationDelete });
wire_enum!(CaptureCompensationMissingReadbackOperation { Validate });
wire_enum!(CaptureCompensationMissingReadbackScope {
    CaptureCompensationMissingReadback
});
wire_enum!(DeleteFinalizationDeleteOperation { Delete });
wire_enum!(DeleteFinalizationDeleteScope { DeleteFinalizationDelete });
wire_enum!(DeleteFinalizationMissingReadbackOperation { Validate });
wire_enum!(DeleteFinalizationMissingReadbackScope {
    DeleteFinalizationMissingReadback
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryReadHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupActiveRecordReadOperation,
    pub scope: CleanupActiveRecordReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupOldRecordDeleteOperation,
    pub scope: CleanupOldRecordDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryOldRecordMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupOldRecordMissingReadbackOperation,
    pub scope: CleanupOldRecordMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureCompensationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CaptureCompensationDeleteOperation,
    pub scope: CaptureCompensationDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureCompensationMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CaptureCompensationMissingReadbackOperation,
    pub scope: CaptureCompensationMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretCaptureCompensationHardwareConfirmStep {
    UncommittedRecordDelete(SecretCaptureCompensationDeleteHardwareConfirmStep),
    UncommittedRecordMissingReadback(SecretCaptureCompensationMissingHardwareConfirmStep),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteFinalizationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: DeleteFinalizationDeleteOperation,
    pub scope: DeleteFinalizationDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteFinalizationMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: DeleteFinalizationMissingReadbackOperation,
    pub scope: DeleteFinalizationMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretDeleteFinalizationHardwareConfirmStep {
    AdmittedRecordDelete(SecretDeleteFinalizationDeleteHardwareConfirmStep),
    AdmittedRecordMissingReadback(
        SecretDeleteFinalizationMissingHardwareConfirmStep,
    ),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretActivationCleanupHardwareConfirmStep {
    ActiveRecordRead(SecretRecoveryReadHardwareConfirmStep),
    OldRecordDelete(SecretRecoveryDeleteHardwareConfirmStep),
    OldRecordMissingReadback(SecretRecoveryOldRecordMissingHardwareConfirmStep),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretRecoveryHardwareConfirmStep {
    ActivationCleanup(SecretActivationCleanupHardwareConfirmStep),
    CaptureCompensation(SecretCaptureCompensationHardwareConfirmStep),
    DeleteFinalization(SecretDeleteFinalizationHardwareConfirmStep),
}

pub(crate) struct PreparedCleanupActiveRecordRead {
    operation_id: SecretOperationId,
    active_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_binding_set: SecretBindingSetCas,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupActiveRecordReadSlot {
    NotApplicable,
    Prepared(PreparedCleanupActiveRecordRead),
}

pub(crate) struct PreparedCleanupOldRecordDelete {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupOldRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedCleanupOldRecordDelete),
}

pub(crate) struct PreparedCleanupOldRecordMissingReadback {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupOldRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedCleanupOldRecordMissingReadback),
}

pub(crate) struct PreparedRecoveryUncommittedRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryUncommittedRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedRecoveryUncommittedRecordDelete),
}

pub(crate) struct PreparedRecoveryUncommittedRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryUncommittedRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedRecoveryUncommittedRecordMissingReadback),
}

pub(crate) struct PreparedRecoveryAdmittedRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryAdmittedRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedRecoveryAdmittedRecordDelete),
}

pub(crate) struct PreparedRecoveryAdmittedRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryAdmittedRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedRecoveryAdmittedRecordMissingReadback),
}

enum PreparedSecretRecoveryBundleRepr {
    ActivationCleanup {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        active_record_read: PreparedCleanupActiveRecordReadSlot,
        old_record_delete: PreparedCleanupOldRecordDeleteSlot,
        old_record_missing_readback: PreparedCleanupOldRecordMissingReadbackSlot,
    },
    CaptureCompensation {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        uncommitted_record_delete: PreparedRecoveryUncommittedRecordDeleteSlot,
        uncommitted_record_missing_readback:
            PreparedRecoveryUncommittedRecordMissingReadbackSlot,
    },
    DeleteFinalization {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        admitted_record_delete: PreparedRecoveryAdmittedRecordDeleteSlot,
        admitted_record_missing_readback:
            PreparedRecoveryAdmittedRecordMissingReadbackSlot,
    },
    OwnerDetachFinalization {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
    },
}

pub(crate) struct PreparedSecretRecoveryBundle(
    PreparedSecretRecoveryBundleRepr,
);

pub(in crate::secret) enum PreparedSecretRecoveryParts {
    ActivationCleanup(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedCleanupActiveRecordReadSlot,
        PreparedCleanupOldRecordDeleteSlot,
        PreparedCleanupOldRecordMissingReadbackSlot,
    ),
    CaptureCompensation(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedRecoveryUncommittedRecordDeleteSlot,
        PreparedRecoveryUncommittedRecordMissingReadbackSlot,
    ),
    DeleteFinalization(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedRecoveryAdmittedRecordDeleteSlot,
        PreparedRecoveryAdmittedRecordMissingReadbackSlot,
    ),
    OwnerDetachFinalization(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
    ),
}

impl PreparedSecretRecoveryBundle {
    fn checked(
        repr: PreparedSecretRecoveryBundleRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate recovery kind/CAS and phase-derived independent slots: activation read/delete/old-missing, capture delete/uncommitted-missing, delete admitted-delete/admitted-missing, detach none")
    }

    pub(in crate::secret) fn recovery_kind(&self) -> SecretRecoveryKind {
        match &self.0 {
            PreparedSecretRecoveryBundleRepr::ActivationCleanup { .. } => {
                SecretRecoveryKind::ActivationCleanup
            }
            PreparedSecretRecoveryBundleRepr::CaptureCompensation { .. } => {
                SecretRecoveryKind::CaptureCompensation
            }
            PreparedSecretRecoveryBundleRepr::DeleteFinalization { .. } => {
                SecretRecoveryKind::DeleteFinalization
            }
            PreparedSecretRecoveryBundleRepr::OwnerDetachFinalization { .. } => {
                SecretRecoveryKind::OwnerDetachFinalization
            }
        }
    }

    pub(in crate::secret) fn into_parts(self) -> PreparedSecretRecoveryParts {
        match self.0 {
            PreparedSecretRecoveryBundleRepr::ActivationCleanup {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                active_record_read,
                old_record_delete,
                old_record_missing_readback,
            } => PreparedSecretRecoveryParts::ActivationCleanup(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                active_record_read,
                old_record_delete,
                old_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::CaptureCompensation {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                uncommitted_record_delete,
                uncommitted_record_missing_readback,
            } => PreparedSecretRecoveryParts::CaptureCompensation(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                uncommitted_record_delete,
                uncommitted_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::DeleteFinalization {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                admitted_record_delete,
                admitted_record_missing_readback,
            } => PreparedSecretRecoveryParts::DeleteFinalization(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                admitted_record_delete,
                admitted_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::OwnerDetachFinalization {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
            } => PreparedSecretRecoveryParts::OwnerDetachFinalization(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
            ),
        }
    }
}

pub(crate) enum RecoveryConfirmationSlot {
    ActiveRecordRead,
    OldRecordDelete,
    OldRecordMissingReadback,
    UncommittedRecordDelete,
    UncommittedRecordMissingReadback,
    AdmittedRecordDelete,
    AdmittedRecordMissingReadback,
}

pub(crate) enum ActivationCleanupConfirmationSlot {
    ActiveRecordRead,
    OldRecordDelete,
    OldRecordMissingReadback,
}

pub(crate) enum CaptureCompensationConfirmationSlot {
    UncommittedRecordDelete,
    UncommittedRecordMissingReadback,
}

pub(crate) enum DeleteFinalizationConfirmationSlot {
    AdmittedRecordDelete,
    AdmittedRecordMissingReadback,
}

pub(crate) enum PendingSecretRecoveryConfirmation {
    ActivationCleanup {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_active_record_read: Option<PreparedCleanupActiveRecordReadSlot>,
        prepared_old_record_delete: Option<PreparedCleanupOldRecordDeleteSlot>,
        prepared_old_record_missing_readback:
            Option<PreparedCleanupOldRecordMissingReadbackSlot>,
        pending_slot: ActivationCleanupConfirmationSlot,
        step: SecretActivationCleanupHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    CaptureCompensation {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_uncommitted_record_delete: Option<PreparedRecoveryUncommittedRecordDeleteSlot>,
        prepared_uncommitted_record_missing_readback:
            Option<PreparedRecoveryUncommittedRecordMissingReadbackSlot>,
        pending_slot: CaptureCompensationConfirmationSlot,
        step: SecretCaptureCompensationHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    DeleteFinalization {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_admitted_record_delete:
            Option<PreparedRecoveryAdmittedRecordDeleteSlot>,
        prepared_admitted_record_missing_readback:
            Option<PreparedRecoveryAdmittedRecordMissingReadbackSlot>,
        pending_slot: DeleteFinalizationConfirmationSlot,
        step: SecretDeleteFinalizationHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) enum PrepareSecretRecovery {
    Prepared(PreparedSecretRecoveryBundle),
    ConfirmationRequired {
        step: SecretRecoveryHardwareConfirmStep,
        pending: PendingSecretRecoveryConfirmation,
    },
}

// Recovery preparation is consuming and material-free. Every pending/read/delete
// platform session is registered before its hardware step can be shown. Cancel,
// expiry and discard terminate the backend session and registry row; Drop is
// not relied on for recovery. Bundle/pending/slot types implement no Clone,
// Serialize, Deserialize or Debug. Only activationCleanup later takes #41's
// lease, and no hardware prompt is legal after that lease is held.

// Actual definition/factory live in
// crate::change_plan::secret_admission. #35 imports this opaque type;
// it cannot construct it or read fields directly.
pub(crate) struct AdmittedSecretChangePlan {
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    admission_id: [u8; 16],
}

pub(crate) struct AdmittedSecretChangePlanIdentity<'a> {
    plan_id: &'a ChangePlanId,
    plan_digest: &'a ChangePlanDigest,
    projection_digest: &'a SecretProjectionDigest,
    admission_id: &'a [u8; 16],
}

pub(crate) struct OwnedAdmittedSecretChangePlanIdentity {
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    admission_id: [u8; 16],
}

impl AdmittedSecretChangePlan {
    // This impl and the sole constructor live in
    // crate::change_plan::secret_admission. The view is immutable and has no
    // constructor/serde; #35 can inspect identity but cannot mint admission.
    pub(crate) fn identity(&self) -> AdmittedSecretChangePlanIdentity<'_> {
        AdmittedSecretChangePlanIdentity {
            plan_id: &self.plan_id,
            plan_digest: &self.plan_digest,
            projection_digest: &self.projection_digest,
            admission_id: &self.admission_id,
        }
    }
}

impl AdmittedSecretChangePlanIdentity<'_> {
    pub(crate) fn plan_id(&self) -> &ChangePlanId {
        self.plan_id
    }

    pub(crate) fn plan_digest(&self) -> &ChangePlanDigest {
        self.plan_digest
    }

    pub(crate) fn projection_digest(&self) -> &SecretProjectionDigest {
        self.projection_digest
    }

    pub(crate) fn into_owned(
        self,
    ) -> OwnedAdmittedSecretChangePlanIdentity {
        OwnedAdmittedSecretChangePlanIdentity {
            plan_id: self.plan_id.clone(),
            plan_digest: self.plan_digest.clone(),
            projection_digest: self.projection_digest.clone(),
            admission_id: *self.admission_id,
        }
    }
}

impl OwnedAdmittedSecretChangePlanIdentity {
    pub(crate) fn matches(&self, admitted: &AdmittedSecretChangePlan) -> bool {
        let current = admitted.identity();
        &self.plan_id == current.plan_id
            && &self.plan_digest == current.plan_digest
            && &self.projection_digest == current.projection_digest
            && &self.admission_id == current.admission_id
    }
}

pub(crate) trait SecretChangePlanAuthority: Send + Sync {
    // #35 receives an already-minted admission. Creation is not exposed on
    // this port; only #55's private owner-module factory may mint one.
    fn assert_still_admitted(
        &self,
        admitted: &AdmittedSecretChangePlan,
    ) -> Result<(), SecretInternalError>;

    fn consume(
        &self,
        admitted: AdmittedSecretChangePlan,
    ) -> Result<(), SecretInternalError>;

    // Consumes a still-admitted plan without applying it.
    fn terminate(
        &self,
        admitted: AdmittedSecretChangePlan,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError>;

    fn assert_staged_still_admitted(
        &self,
        admitted: &AdmittedStagedSecretImportPlan,
        projection: &StagedSecretImportActivationProjection,
        authority_match: &StagedImportAuthorityMatchReceipt,
    ) -> Result<(), SecretInternalError>;

    fn staged_durable_identity(
        &self,
        admitted: &AdmittedStagedSecretImportPlan,
    ) -> Result<OwnedAdmittedStagedSecretImportIdentity, SecretInternalError>;

    fn consume_staged(
        &self,
        admitted: AdmittedStagedSecretImportPlan,
    ) -> Result<(), SecretInternalError>;

    fn terminate_staged(
        &self,
        admitted: AdmittedStagedSecretImportPlan,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError>;
}

// Phase 2A: #41 configuration_apply adapters are unpublished. Local
// placeholders keep the writer seal/route compiling without those modules.
pub(crate) struct CodexTargetLiveConfigWriterAdapter {
    _private: (),
}
pub(crate) struct CodexRollbackLiveConfigWriterAdapter {
    _private: (),
}

impl CodexTargetLiveConfigWriterAdapter {
    fn bound_live_sink_id(&self) -> CodexLiveSecretSinkId {
        CodexLiveSecretSinkId::CodexAuthJsonOpenAiApiKey
    }
    fn write_and_readback_once(&mut self, _material: &[u8]) -> SecretWriterReceiptDto {
        todo!("#41 target writer adapter is unpublished")
    }
}

impl CodexRollbackLiveConfigWriterAdapter {
    fn bound_live_sink_id(&self) -> CodexLiveSecretSinkId {
        CodexLiveSecretSinkId::CodexAuthJsonOpenAiApiKey
    }
    fn write_and_readback_once(&mut self, _material: &[u8]) -> SecretWriterReceiptDto {
        todo!("#41 rollback writer adapter is unpublished")
    }
}

mod secret_apply_writer_sealed {
    pub(super) trait Sealed {}
    impl Sealed for super::CodexTargetLiveConfigWriterAdapter {}
    impl Sealed for super::CodexRollbackLiveConfigWriterAdapter {}
}

pub(crate) trait SecretApplyWriter:
    secret_apply_writer_sealed::Sealed
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId;

    // Synchronous and fixed-result: no await while material is borrowed and no
    // generic return type through which material can escape.
    fn write_and_readback(
        &mut self,
        material: &[u8],
    ) -> SecretWriterReceiptDto;
}

impl SecretApplyWriter for
    CodexTargetLiveConfigWriterAdapter
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        self.bound_live_sink_id()
    }

    fn write_and_readback(&mut self, material: &[u8]) -> SecretWriterReceiptDto {
        self.write_and_readback_once(material)
    }
}

impl SecretApplyWriter for
    CodexRollbackLiveConfigWriterAdapter
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        self.bound_live_sink_id()
    }

    fn write_and_readback(&mut self, material: &[u8]) -> SecretWriterReceiptDto {
        self.write_and_readback_once(material)
    }
}

pub(crate) enum SecretApplyWriterInvocation<'a> {
    Target(
        &'a mut CodexTargetLiveConfigWriterAdapter,
    ),
    Rollback(
        &'a mut CodexRollbackLiveConfigWriterAdapter,
    ),
}

impl SecretApplyWriterInvocation<'_> {
    pub(crate) fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        match self {
            Self::Target(writer) => writer.bound_live_sink_id(),
            Self::Rollback(writer) => writer.bound_live_sink_id(),
        }
    }

    // Called only by crate::secret::backend's sealed callback impl.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> SecretWriterReceiptDto {
        match self {
            Self::Target(writer) => writer.write_and_readback_once(material),
            Self::Rollback(writer) => writer.write_and_readback_once(material),
        }
    }
}

// Both concrete adapter types and their private constructors live in
// crate::services::configuration_apply::provider. Only that module's
// target/rollback job
// factories can construct them. SecretApplyWriterInvocation is the closed
// role-to-writer pairing accepted by #35. Each private adapter constructor
// requires one CodexLiveSecretSinkId and binds its exact #41 final-baseline
// projection/readback target; it never accepts or exposes a filesystem path.
// This is the complete implementer
// allowlist; there is no closure/function-pointer adapter constructor.

pub(crate) struct ExistingSecretOwnerToken {
    owner: SecretOwner,
}

impl ExistingSecretOwnerToken {
    // Credential-free inspection only. Construction/existence authority stays
    // private to crate::database::dao::providers.
    pub(crate) fn owner(&self) -> &SecretOwner {
        &self.owner
    }
}
pub(crate) struct SecretApplyAuthoritySnapshot {
    _private: (),
}
pub(crate) struct SecretCandidateAuthoritySnapshot {
    _private: (),
}

impl SecretCandidateAuthoritySnapshot {
    fn validate_activation_result_identity(
        &self,
        result: &SecretActivationResultDtoRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match candidate/plan/ref and exact affected owner set")
    }
}

// Durable, tagged device-local recovery schema. It is never a public command
// DTO and has no material/material-derived field. Custom device-store encoding
// is owned by crate::secret::device_store::recovery; the private fields prevent
// unchecked construction even inside the wider crate.
pub(in crate::secret) struct RecoveryAffectedOwner {
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    secret_ref: SecretRef,
    binding_revision: SecretBindingRevision,
}

pub(in crate::secret) struct NonEmptySortedRecoveryAffectedOwners(
    Vec<RecoveryAffectedOwner>,
);

impl NonEmptySortedRecoveryAffectedOwners {
    pub(super) fn checked(
        owners: Vec<RecoveryAffectedOwner>,
    ) -> Result<Self, SecretInternalError> {
        todo!("non-empty, strict owner sort, unique owner and active-ref match")
    }
}

pub(in crate::secret) struct NonEmptyRecoverySourceExpectations(
    CurrentLegacySourceExpectations,
);

impl NonEmptyRecoverySourceExpectations {
    pub(super) fn checked(
        values: CurrentLegacySourceExpectations,
    ) -> Result<Self, SecretInternalError> {
        if values.as_slice().is_empty() {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(values))
        }
    }

    fn as_slice(&self) -> &[LegacySourceExpectation] {
        self.0.as_slice()
    }
}

pub(in crate::secret) struct FinalizeLegacyScrubRecoveryStep {
    expected_store_revision: SecretStoreRevision,
    active_secret_ref: SecretRef,
    active_record_revision: SecretRecordRevision,
    active_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    source_expectations: NonEmptyRecoverySourceExpectations,
    read_confirmation: PhysicalConfirmation,
    structure_digest: RecoveryStructureDigest,
}

pub(in crate::secret) struct DeleteOldRecordRecoveryStep {
    expected_store_revision: SecretStoreRevision,
    old_secret_ref: SecretRef,
    old_record_revision: SecretRecordRevision,
    expected_old_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    delete_confirmation: PhysicalConfirmation,
    required_binding_state: NoBindingsRequired,
}

pub(in crate::secret) struct VerifyOldRecordMissingRecoveryStep {
    read_confirmation: PhysicalConfirmation,
}

pub(in crate::secret) enum ActivationCleanupRecoveryStep {
    FinalizeLegacyScrub(FinalizeLegacyScrubRecoveryStep),
    DeleteOldRecord(DeleteOldRecordRecoveryStep),
    VerifyOldRecordMissing(VerifyOldRecordMissingRecoveryStep),
}

pub(in crate::secret) struct NonEmptyActivationRecoverySteps(
    Vec<ActivationCleanupRecoveryStep>,
);

impl NonEmptyActivationRecoverySteps {
    pub(super) fn checked(
        values: Vec<ActivationCleanupRecoveryStep>,
    ) -> Result<Self, SecretInternalError> {
        todo!("nonempty exact suffix in rank finalizeLegacyScrub < deleteOldRecord < verifyOldRecordMissing")
    }
}

pub(crate) enum ActivationCleanupRecoveryPhase {
    StateFinalized,
    ProviderFinalized,
    OldRecordDeleteIntent,
    OldRecordDeleteApplied {
        checkpoint: RecoveryOldRecordDeleteCheckpoint,
    },
    RecoveryRequired {
        checkpoint: ActivationOldRecordDurableCheckpoint,
    },
}

// Old-record missing readback is independently authorized and consumes the
// durable delete-applied CAS. Because it is the final recovery step, its
// receipt and the supersession + Terminal transition are committed in one
// device-authority transaction. There is no standalone nonterminal
// old-record-missing-verified phase with an empty remaining-step suffix.
pub(in crate::secret) enum ActivationCleanupOldRecordTerminal {
    NotApplicable,
    Superseded {
        disposition: BackendDeleteDisposition,
        source: RotationSupersessionSource,
        revoked_at: UtcTimestamp,
    },
}

pub(in crate::secret) enum ActivationCleanupRecoveryState {
    Nonterminal {
        phase: ActivationCleanupRecoveryPhase,
        remaining_steps: NonEmptyActivationRecoverySteps,
    },
    Terminal {
        old_record: ActivationCleanupOldRecordTerminal,
        remaining_steps: [ActivationCleanupRecoveryStep; 0],
    },
}

pub(in crate::secret) struct CaptureDeleteUncommittedRecordStep {
    delete_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct CaptureVerifyMissingStep {
    read_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct CaptureFinalizeCompensationStep {
    required_binding_state: NoBindingsRequired,
    terminal_candidate_state: DiscardedCandidateTerminalState,
    required_record_state: AbsentRecordState,
}
pub(in crate::secret) enum CaptureCompensationRecoveryStep {
    DeleteUncommittedRecord(CaptureDeleteUncommittedRecordStep),
    VerifyUncommittedRecordMissing(CaptureVerifyMissingStep),
    FinalizeCaptureCompensation(CaptureFinalizeCompensationStep),
}
pub(in crate::secret) struct CaptureDeleteIntentSteps(
    CaptureDeleteUncommittedRecordStep,
    CaptureVerifyMissingStep,
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) struct CaptureDeleteAppliedSteps(
    CaptureVerifyMissingStep,
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) struct CaptureMissingVerifiedSteps(
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) enum CaptureCompensationRecoveryCheckpointAndSuffix {
    None { remaining_steps: CaptureDeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: CaptureDeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: CaptureMissingVerifiedSteps,
    },
}
pub(in crate::secret) enum CaptureCompensationRecoveryState {
    DeleteIntent { remaining_steps: CaptureDeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: CaptureDeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: CaptureMissingVerifiedSteps,
    },
    RecoveryRequired {
        checkpoint_and_suffix: CaptureCompensationRecoveryCheckpointAndSuffix,
    },
    StateFinalized {
        terminal_candidate_state: DiscardedCandidateTerminalState,
        remaining_steps: [CaptureCompensationRecoveryStep; 0],
    },
    Terminal {
        terminal_candidate_state: DiscardedCandidateTerminalState,
        remaining_steps: [CaptureCompensationRecoveryStep; 0],
    },
}

pub(in crate::secret) struct DeleteAdmittedRecordRecoveryStep {
    delete_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct DeleteVerifyMissingRecoveryStep {
    read_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct DeleteFinalizeStateRecoveryStep {
    required_binding_state: RetainedTombstonesBindingState,
    revocation_source: UserDeleteRevocationSource,
}
pub(in crate::secret) enum DeleteFinalizationRecoveryStep {
    DeleteAdmittedRecord(DeleteAdmittedRecordRecoveryStep),
    VerifyDeletedRecordMissing(DeleteVerifyMissingRecoveryStep),
    FinalizeDeletedRecord(DeleteFinalizeStateRecoveryStep),
}
pub(in crate::secret) struct DeleteIntentSteps(
    DeleteAdmittedRecordRecoveryStep,
    DeleteVerifyMissingRecoveryStep,
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) struct DeleteAppliedSteps(
    DeleteVerifyMissingRecoveryStep,
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) struct DeleteMissingVerifiedSteps(
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) enum DeleteFinalizationRecoveryCheckpointAndSuffix {
    None { remaining_steps: DeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: DeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: DeleteMissingVerifiedSteps,
    },
}
pub(in crate::secret) enum DeleteFinalizationRecoveryState {
    DeleteIntent { remaining_steps: DeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: DeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: DeleteMissingVerifiedSteps,
    },
    RecoveryRequired {
        checkpoint_and_suffix: DeleteFinalizationRecoveryCheckpointAndSuffix,
    },
    StateFinalized {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
        remaining_steps: [DeleteFinalizationRecoveryStep; 0],
    },
    Terminal {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
        remaining_steps: [DeleteFinalizationRecoveryStep; 0],
    },
}

wire_enum!(AbsentRecordState { Absent });
wire_enum!(RetainedTombstonesBindingState { RetainedTombstones });
wire_enum!(ForbiddenBackendMutation { Forbidden });

pub(in crate::secret) struct OwnerDetachFinalizeLocalStateStep {
    confirmation: NeverPhysicalConfirmation,
    backend_mutation: ForbiddenBackendMutation,
}
pub(in crate::secret) enum OwnerDetachFinalizationNonterminalPhase {
    ProviderDetachCommitted,
    LocalOwnerCasIntent,
    RecoveryRequired,
}
pub(in crate::secret) enum OwnerDetachFinalizationCompletedPhase {
    LocalOwnerCasApplied,
    Terminal,
}
pub(in crate::secret) enum OwnerDetachFinalizationRecoveryState {
    Nonterminal {
        phase: OwnerDetachFinalizationNonterminalPhase,
        remaining_steps: OwnerDetachFinalizeLocalStateStep,
    },
    Completed {
        phase: OwnerDetachFinalizationCompletedPhase,
        remaining_steps: [OwnerDetachFinalizeLocalStateStep; 0],
    },
}

pub(in crate::secret) enum DurableSecretRecoveryRecord {
    ActivationCleanup {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        active_secret_ref: SecretRef,
        active_record_revision: SecretRecordRevision,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        state: ActivationCleanupRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    CaptureCompensation {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        expected_store_revision: SecretStoreRevision,
        expected_binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        state: CaptureCompensationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    DeleteFinalization {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        delete_admission: DeleteJournalAdmissionIdentity,
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        expected_store_revision: SecretStoreRevision,
        expected_binding_set_cas: SecretBindingSetCas,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        revocation_source: UserDeleteRevocationSource,
        state: DeleteFinalizationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    OwnerDetachFinalization {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        provider_detach_transaction_id: ProviderDetachTransactionId,
        provider_detach_commit_id: ProviderDetachCommitId,
        detached_owner: SecretOwner,
        expected_owner_binding_revision: SecretOwnerBindingRevision,
        expected_store_revision: SecretStoreRevision,
        legacy_source_coverage_state: NoBlockingLegacySourcesState,
        binding_view: DetachProviderOwnerBindingExpectation,
        state: OwnerDetachFinalizationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
}

impl DurableSecretRecoveryRecord {
    // The private custom codec emits the device-store wire algebra, not these
    // Rust implementation field names. RecoveryRequired encodes
    // phase=recoveryRequired, flattens checkpoint_and_suffix into the exact
    // checkpoint object plus sibling remainingSteps, and never exposes the
    // internal pairing key. StateFinalized/Terminal omit
    // intermediate receipts. Activation Terminal and owner-detach Completed
    // encode their explicit phase plus an empty array. Checked construction
    // rejects every phase/suffix pair not listed by the device-store schema.
    // Activation OldRecordDeleteApplied and its RecoveryRequired checkpoint
    // always encode the indivisible deleteDisposition/backendCompletedAt/
    // deleteAppliedCas triple. The subsequent missing receipt is a commit gate:
    // it is consumed in the same transaction that writes supersession and
    // Terminal, whose revokedAt is exactly backendCompletedAt.
    pub(super) fn checked(
        record: DurableSecretRecoveryRecord,
    ) -> Result<Self, SecretInternalError> {
        todo!("custom strict codec: exact four-arm fields, zero-count/no-legacy literals, phase receipt suffix, sorted owners/steps, full activation delete checkpoint in normal/recovery-required arms, CAS and timestamps; supersession revokedAt equals backendCompletedAt")
    }
}

pub(crate) struct RecoveryProviderProjection {
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    phase: ActivationCleanupRecoveryPhase,
    active_secret_ref: SecretRef,
    active_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    active_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    read_confirmation: PhysicalConfirmation,
    structure_digest: RecoveryStructureDigest,
    source_expectations: NonEmptyRecoverySourceExpectations,
}

impl RecoveryProviderProjection {
    // Private checked factory in crate::secret::device_store::recovery. It can
    // be created only from a FinalizeLegacyScrub expectation whose full row and
    // RecoveryCas were re-read under the recovery mutation permit.
    fn checked_from_recovery(
        recovery: &DurableSecretRecoveryRecord,
        step: &ActivationCleanupRecoveryStep,
    ) -> Result<Self, SecretInternalError> {
        todo!("accept only FinalizeLegacyScrub from the current nonterminal remaining suffix; copy exact fields and reject changed CAS")
    }

    pub(crate) fn recovery_id(&self) -> &SecretRecoveryId { &self.recovery_id }
    pub(crate) fn recovery_cas(&self) -> &SecretRecoveryCas { &self.recovery_cas }
    pub(crate) fn candidate_id(&self) -> &SecretCandidateId { &self.candidate_id }
    pub(crate) fn phase(&self) -> &ActivationCleanupRecoveryPhase {
        &self.phase
    }
    pub(crate) fn active_ref(&self) -> &SecretRef { &self.active_secret_ref }
    pub(crate) fn record_revision(&self) -> SecretRecordRevision {
        self.active_record_revision
    }
    pub(crate) fn store_revision(&self) -> SecretStoreRevision {
        self.expected_store_revision
    }
    pub(crate) fn binding_set_cas(&self) -> &SecretBindingSetCas {
        &self.active_binding_set_cas
    }
    pub(crate) fn backend_instance_id(&self) -> &SecretBackendInstanceId {
        &self.backend_instance_id
    }
    pub(crate) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }
    pub(crate) fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.device_binding_generation
    }
    pub(crate) fn capability_revision(&self) -> CapabilityRevision {
        self.capability_revision
    }
    pub(crate) fn confirmation(&self) -> PhysicalConfirmation {
        self.read_confirmation
    }
    pub(crate) fn structure_digest(&self) -> &RecoveryStructureDigest {
        &self.structure_digest
    }
    pub(crate) fn source_expectations(&self) -> &[LegacySourceExpectation] {
        self.source_expectations.as_slice()
    }
}

pub(crate) struct SecretRecoveryAuthoritySnapshot {
    _private: (),
}

#[derive(Clone)]
pub(crate) struct CandidateDiscardDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

pub(crate) struct CandidateDiscardDeleteApplied {
    journal: CandidateDeleteJournalRow,
    checkpoint: CandidateDiscardDeleteCheckpoint,
}

pub(crate) struct AuthorizedCandidateDiscardRecordDelete {
    backend: AuthorizedBackendDelete,
    journal: CandidateDeleteJournalRow,
}

impl AuthorizedCandidateDiscardRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<CandidateDiscardDeleteApplied, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at);
        todo!("atomically persist DiscardCandidate BackendApplied with the exact three-field checkpoint and mint its operation-bound deleteAppliedCas")
    }
}

pub(crate) struct CandidateDiscardMissingReadbackCheckpoint {
    journal: CandidateDeleteJournalRow,
    delete: CandidateDiscardDeleteCheckpoint,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedCandidateDiscardRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    applied: CandidateDiscardDeleteApplied,
}

impl AuthorizedCandidateDiscardRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<CandidateDiscardMissingReadbackCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.applied.checkpoint.delete_applied_cas,
            now,
        )?;
        let checkpoint = CandidateDiscardMissingReadbackCheckpoint {
            journal: self.applied.journal,
            delete: self.applied.checkpoint,
            missing,
        };
        let _ = checkpoint;
        todo!("durably persist the independent MissingReadbackVerified phase before returning the checkpoint; terminal state is still forbidden")
    }
}

pub(crate) struct CaptureCompensationDeleteCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    delete_applied_cas: BackendDeleteAppliedCas,
}
pub(crate) struct AuthorizedCaptureCompensationDelete {
    backend: AuthorizedBackendDelete,
    snapshot: SecretRecoveryAuthoritySnapshot,
}

impl AuthorizedCaptureCompensationDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<CaptureCompensationDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        todo!("atomically persist durable backendApplied before returning snapshot + delete receipt + new delete-applied CAS")
    }
}

pub(crate) struct CaptureCompensationMissingCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedCaptureCompensationMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: CaptureCompensationDeleteCheckpoint,
}

impl AuthorizedCaptureCompensationMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<CaptureCompensationMissingCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        todo!("persist MissingReadbackVerified separately; delete and probe are never one call")
    }
}

pub(crate) struct DeleteFinalizationDeleteCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    delete_applied_cas: BackendDeleteAppliedCas,
}

pub(crate) struct AuthorizedDeleteFinalizationDelete {
    backend: AuthorizedBackendDelete,
    snapshot: SecretRecoveryAuthoritySnapshot,
}

impl AuthorizedDeleteFinalizationDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<DeleteFinalizationDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        todo!("persist deleteFinalization backendApplied and a new delete-applied CAS before any missing readback")
    }
}

pub(crate) struct DeleteFinalizationMissingCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedDeleteFinalizationMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: DeleteFinalizationDeleteCheckpoint,
}

impl AuthorizedDeleteFinalizationMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<DeleteFinalizationMissingCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        todo!("persist deleteFinalization MissingReadbackVerified independently")
    }
}

impl SecretRecoveryAuthoritySnapshot {
    fn validate_recovery_impact_identity(
        &self,
        impact: &SecretRecoveryImpactRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match recovery/candidate/ref, pending steps and affected owners")
    }

    fn validate_recovery_result_identity(
        &self,
        result: &SecretRecoveryResultRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match recovery/candidate/ref and the exact affected owner set")
    }
}
pub(crate) struct ActivationBindingCheckpoint {
    _private: (),
}
pub(crate) struct ProviderFinalizedActivationCheckpoint {
    _private: (),
}
pub(crate) struct ActivationOldRecordDeletePostconditionReceipt {
    _private: (),
}
#[derive(Clone)]
pub(crate) struct ActivationOldRecordDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

impl ActivationOldRecordDeleteCheckpoint {
    fn into_durable_failure_checkpoint(
        self,
    ) -> ActivationOldRecordDurableCheckpoint {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition: self.delete_disposition,
            backend_completed_at: self.backend_completed_at,
            delete_applied_cas: self.delete_applied_cas,
        }
    }
}

pub(crate) struct ActivationOldRecordDeleteApplied {
    postcondition: ActivationOldRecordDeletePostconditionReceipt,
    checkpoint: ActivationOldRecordDeleteCheckpoint,
}
pub(crate) struct AuthorizedActivationOldRecordDelete {
    backend: AuthorizedBackendDelete,
    postcondition: ActivationOldRecordDeletePostconditionReceipt,
}

impl AuthorizedActivationOldRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<ActivationOldRecordDeleteApplied, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at, self.postcondition);
        todo!("persist activation OldRecordDeleteApplied with exact disposition/completion/CAS and return postcondition + checkpoint; no supersession yet")
    }
}

pub(crate) struct AuthorizedActivationOldRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    applied: ActivationOldRecordDeleteApplied,
}

impl AuthorizedActivationOldRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<ActivationOldRecordDeleteCompletion, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.applied.checkpoint.delete_applied_cas,
            now,
        )?;
        let revoked_at =
            self.applied.checkpoint.backend_completed_at.clone();
        let supersession = RotationSupersessionReceipt {
            source: RotationSupersessionSource::SupersededByRotation,
            revoked_at,
        };
        Ok(ActivationOldRecordDeleteCompletion::Completed {
            postcondition: self.applied.postcondition,
            delete: self.applied.checkpoint,
            missing,
            supersession,
        })
    }
}
wire_enum!(RotationSupersessionSource { SupersededByRotation });
pub(crate) struct RotationSupersessionReceipt {
    source: RotationSupersessionSource,
    revoked_at: UtcTimestamp,
}
pub(crate) enum ActivationOldRecordDeleteCompletion {
    NotApplicable,
    Completed {
        postcondition: ActivationOldRecordDeletePostconditionReceipt,
        delete: ActivationOldRecordDeleteCheckpoint,
        missing: BackendMissingReadbackReceipt,
        supersession: RotationSupersessionReceipt,
    },
}
pub(crate) enum ActivationRecoveryCheckpoint {
    ProviderScrubPending(ActivationBindingCheckpoint),
    OldRecordDeletePending(ProviderFinalizedActivationCheckpoint),
    OldRecordMissingReadbackPending(ActivationOldRecordDeleteApplied),
}
// The two Provider receipts are actually defined in
// crate::services::configuration_apply::provider; only its lease-bound port
// implementation
// can construct them. #35 can consume but never inspect or mint them.
pub(crate) struct ProviderScrubReadbackReceipt {
    _private: (),
}
pub(crate) struct RecoveryProviderFinalizedCheckpoint {
    _private: (),
}
pub(crate) struct ProviderLegacySourceMatchReceipt {
    _private: (),
}
pub(crate) struct ProviderReplacementSourceValidationReceipt {
    _private: (),
}
pub(crate) enum ProviderActivationSourceValidationReceipt {
    CandidateEquality(ProviderLegacySourceMatchReceipt),
    ExplicitReplacement(ProviderReplacementSourceValidationReceipt),
}
pub(crate) enum RecoveryStepCheckpoint {
    Initial(SecretRecoveryAuthoritySnapshot),
    ProviderFinalized(RecoveryProviderFinalizedCheckpoint),
}
pub(crate) struct AuthorizedRecoveryOldRecordDelete {
    backend: AuthorizedBackendDelete,
}

#[derive(Clone)]
pub(crate) struct RecoveryOldRecordDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

impl RecoveryOldRecordDeleteCheckpoint {
    fn checked_from_durable_failure_checkpoint(
        checkpoint: ActivationOldRecordDurableCheckpoint,
    ) -> Result<Self, SecretInternalError> {
        match checkpoint {
            ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
                delete_disposition,
                backend_completed_at,
                delete_applied_cas,
            } => Ok(Self {
                delete_disposition,
                backend_completed_at,
                delete_applied_cas,
            }),
            ActivationOldRecordDurableCheckpoint::None => {
                Err(SecretInternalError::dependency_changed())
            }
        }
    }

    fn into_recovery_required_checkpoint(
        self,
    ) -> ActivationOldRecordDurableCheckpoint {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition: self.delete_disposition,
            backend_completed_at: self.backend_completed_at,
            delete_applied_cas: self.delete_applied_cas,
        }
    }
}

impl AuthorizedRecoveryOldRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<RecoveryOldRecordDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at);
        todo!("persist recovery old-record exact disposition/completion/CAS checkpoint before any probe")
    }
}

pub(crate) struct AuthorizedRecoveryOldRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: RecoveryOldRecordDeleteCheckpoint,
}

impl AuthorizedRecoveryOldRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<RecoveryOldRecordDeleteCompletion, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        let revoked_at = self.checkpoint.backend_completed_at.clone();
        let supersession = RotationSupersessionReceipt {
            source: RotationSupersessionSource::SupersededByRotation,
            revoked_at,
        };
        Ok(RecoveryOldRecordDeleteCompletion::Completed {
            delete: self.checkpoint,
            missing,
            supersession,
        })
    }
}
pub(crate) enum RecoveryOldRecordDeleteCompletion {
    NotPending,
    Completed {
        delete: RecoveryOldRecordDeleteCheckpoint,
        missing: BackendMissingReadbackReceipt,
        supersession: RotationSupersessionReceipt,
    },
}

pub(crate) enum SecretMutationScope<'a> {
    ApplyOwner(&'a ExistingSecretOwnerToken),
    Candidate(&'a SecretCandidateId),
    Recovery(&'a SecretRecoveryId),
    RuntimeOwner(&'a ExistingSecretOwnerToken),
}

pub(crate) struct SecretMutationPermit<'a> {
    // A real keyed std::sync::Mutex guard; never a marker/boolean lease.
    _held_guard: std::sync::MutexGuard<'a, ()>,
}

pub(crate) trait SecretMutationGate: Send + Sync {
    fn acquire<'a>(
        &'a self,
        scope: SecretMutationScope<'_>,
    ) -> Result<SecretMutationPermit<'a>, SecretInternalError>;
}

struct SecretOwnerSummaryAuthorityRow {
    owner: ExistingSecretOwnerToken,
    summary: SecretOwnerCredentialSummary,
}

struct SecretSummaryAuthoritySnapshot {
    owners: Vec<SecretOwnerSummaryAuthorityRow>,
    refs: Vec<SecretRefAggregate>,
    next_cursor: Option<SecretSummaryCursor>,
}

pub(crate) trait DeviceLocalSecretAuthority: Send + Sync {
    fn read_secret_summary_snapshot(
        &self,
        request: &ListSecretSummariesRequest,
    ) -> Result<SecretSummaryAuthoritySnapshot, SecretInternalError>;

    fn revalidate_claimed_capture_intent(
        &self,
        claim: &ClaimedSecretCaptureIntent,
        current_legacy_source_coverage: LegacySourceCoverageReceipt,
        backends: &dyn SecretBackendRegistry,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;
    // Freshly compares owner/purpose/intent/binding/coverage/hidden-binding,
    // expiry and the exact selected registered Arc/device/backend tuple.

    fn capture_intent_registration_from_atomic_snapshot(
        &self,
        owner: ExistingSecretOwnerToken,
        request: ListSecretBackendOptionsRequest,
        legacy_source_coverage: LegacySourceCoverageReceipt,
        backends: &dyn SecretBackendRegistry,
        now: &UtcTimestamp,
    ) -> Result<SecretCaptureIntentRegistration, SecretInternalError>;
    // Both receipts above are newly minted by
    // CodexLegacySourceInventoryBridge::fresh_capture_coverage. The authority
    // can consume and compare them but has no legacy-inventory method and
    // cannot construct a coverage receipt.

    fn read_apply_snapshot(
        &self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<SecretApplyAuthoritySnapshot, SecretInternalError>;

    fn read_candidate_snapshot(
        &self,
        candidate_id: &SecretCandidateId,
    ) -> Result<SecretCandidateAuthoritySnapshot, SecretInternalError>;

    fn authorize_candidate_discard_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        journal: CandidateDeleteJournalRow,
        backend: BackendInstanceHandle,
        prepared: PreparedCandidateDiscardRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCandidateDiscardRecordDelete, SecretInternalError>;

    fn authorize_candidate_discard_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        applied: CandidateDiscardDeleteApplied,
        backend: BackendInstanceHandle,
        prepared: PreparedCandidateDiscardRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCandidateDiscardRecordMissingReadback, SecretInternalError>;
    // This method must consume delete_applied_cas_reservation with the exact
    // operation id + CandidateDiscardDeleteCheckpoint.delete_applied_cas
    // before BackendInstanceHandle::authorize_missing_readback_once is legal.

    fn finalize_candidate_discard(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CandidateDiscardMissingReadbackCheckpoint,
    ) -> Result<DiscardSecretCandidateResult, SecretInternalError>;
    // Atomically removes the unbound record, writes the candidate/audit state
    // and Terminal with the journal's immutable discarded|expired target; no
    // intermediate StateFinalized or general recovery row is created.

    fn read_recovery_snapshot(
        &self,
        recovery_id: &SecretRecoveryId,
        expected: &SecretRecoveryCas,
    ) -> Result<SecretRecoveryAuthoritySnapshot, SecretInternalError>;

    fn recovery_provider_projection(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<RecoveryProviderProjection, SecretInternalError>;

    fn mint_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
        consumer: FixedRuntimeConsumer,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError>;
    // Implementation must fresh-check that the bound record's validated
    // allowedConsumers contains consumer.required_record_consumer(); a named
    // fixed consumer can never borrow another consumer's capability bit.

    fn authorize_apply_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretApplyAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: ClaimedPreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedApplyRead, SecretInternalError>;

    fn authorize_runtime_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        binding: &AuthorityMintedRuntimeBinding,
        backend: BackendInstanceHandle,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRuntimeRead, SecretInternalError>;

    fn authorize_migration_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        backend: BackendInstanceHandle,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedMigrationRead, SecretInternalError>;

    fn authorize_staged_import_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        staged_owner: &StagedSecretOwnerToken,
        backend: BackendInstanceHandle,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedStagedImportRead, SecretInternalError>;

    fn persist_backend_revocation_observation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        observation: BackendRevocationObservation,
    ) -> Result<SecretRevocationView, SecretInternalError>;
    // The implementation destructures the consuming receipt only inside the
    // mutation permit, fresh-revalidates its ref/store/record/binding-set/
    // registered-backend/device/capability tuple, then persists source/time.
    // There is no caller-supplied ref or transplantable observation payload.

    fn commit_activation_binding(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretCandidateAuthoritySnapshot,
        projection: &SecretCandidateActivationProjection,
        provider_sources: ProviderActivationSourceValidationReceipt,
    ) -> Result<ActivationBindingCheckpoint, SecretInternalError>;

    fn authorize_activation_candidate_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretCandidateAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationCandidateRead,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationRead, SecretInternalError>;

    fn record_activation_provider_finalized(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ActivationBindingCheckpoint,
        provider: ProviderScrubReadbackReceipt,
    ) -> Result<ProviderFinalizedActivationCheckpoint, SecretInternalError>;

    fn authorize_activation_old_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: &ProviderFinalizedActivationCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationOldRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationOldRecordDelete, SecretInternalError>;

    fn authorize_activation_old_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        applied: ActivationOldRecordDeleteApplied,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationOldRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationOldRecordMissingReadback, SecretInternalError>;
    // Consumes the prepared reservation against
    // applied.checkpoint.delete_applied_cas; pre-confirmation alone never
    // authorizes the missing readback.

    fn finalize_activation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ProviderFinalizedActivationCheckpoint,
        old_record: ActivationOldRecordDeleteCompletion,
    ) -> Result<SecretActivationResultDto, SecretInternalError>;

    fn record_activation_recovery(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ActivationRecoveryCheckpoint,
        failure: SecretInternalError,
    ) -> Result<SecretActivationResultDto, SecretInternalError>;

    fn record_recovery_provider_finalized(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        provider: ProviderScrubReadbackReceipt,
    ) -> Result<RecoveryProviderFinalizedCheckpoint, SecretInternalError>;

    fn authorize_recovery_active_record_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupActiveRecordRead,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryRead, SecretInternalError>;

    fn authorize_recovery_old_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: &RecoveryStepCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupOldRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryOldRecordDelete, SecretInternalError>;

    fn authorize_recovery_old_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryOldRecordDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupOldRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryOldRecordMissingReadback, SecretInternalError>;
    // RecoveryRequired must reconstruct this exact three-field checkpoint;
    // the missing authorization consumes its reservation against the retained
    // CAS and terminal supersession uses retained backend_completed_at.

    fn finalize_recovery(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryStepCheckpoint,
        old_record: RecoveryOldRecordDeleteCompletion,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn record_recovery_failure(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryProviderFinalizedCheckpoint,
        failure: SecretInternalError,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn authorize_capture_compensation_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryUncommittedRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCaptureCompensationDelete, SecretInternalError>;

    fn authorize_capture_compensation_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CaptureCompensationDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryUncommittedRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCaptureCompensationMissingReadback, SecretInternalError>;

    fn finalize_capture_compensation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CaptureCompensationMissingCheckpoint,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn authorize_delete_finalization_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryAdmittedRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedDeleteFinalizationDelete, SecretInternalError>;

    fn authorize_delete_finalization_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: DeleteFinalizationDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryAdmittedRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedDeleteFinalizationMissingReadback, SecretInternalError>;

    fn finalize_deleted_record(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: DeleteFinalizationMissingCheckpoint,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn finalize_owner_detach(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        provider: ProviderDetachCommitReceipt,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;
}

// This object-safe trait and its concrete implementation live in
// crate::secret::device_store. It has no generic method and no Provider/DB
// accessor. Every authorize_* method revalidates the complete authority scope,
// invokes only BackendInstanceHandle's matching consuming read/delete wrapper,
// and returns an unforgeable route-specific Authorized*Read/Delete object.

pub(crate) struct ProviderDetachCommitReceipt {
    _private: (),
}

trait ProviderLeaseBoundPort {
    fn assert_apply_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretApplyPlanProjection,
    ) -> Result<(), SecretInternalError>;

    fn assert_activation_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<(), SecretInternalError>;

    fn assert_cleanup_final_baseline(
        &mut self,
        projection: &RecoveryProviderProjection,
    ) -> Result<(), SecretInternalError>;

    // CandidateEquality only: resolve the complete exact Provider occurrence
    // set under the held lease, validate every structural revision, compare
    // every value with `expected` through ConstantTimeEq, return no material.
    fn compare_candidate_equality_activation_sources(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        expected: &[u8],
    ) -> Result<ProviderLegacySourceMatchReceipt, SecretInternalError>;

    // ExplicitReplacement only: resolve the same complete exact occurrence
    // set/revisions and validate the admitted replacement impact. It receives
    // only a candidate-read receipt and MUST NOT require or compare old values.
    fn validate_explicit_replacement_sources(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        candidate: CandidateReadVerifiedReceipt,
    ) -> Result<ProviderReplacementSourceValidationReceipt, SecretInternalError>;

    fn scrub_activation_and_readback(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        binding: &ActivationBindingCheckpoint,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;

    fn compare_and_scrub_recovery_equality_sources(
        &mut self,
        projection: &RecoveryProviderProjection,
        expected: &[u8],
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;
}

pub(crate) struct ActivationCandidateEqualityCompareCallback<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    projection: &'a SecretCandidateActivationProjection,
}

impl ActivationCandidateEqualityCompareCallback<'_> {
    fn new<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        projection: &'a SecretCandidateActivationProjection,
    ) -> ActivationCandidateEqualityCompareCallback<'a> {
        ActivationCandidateEqualityCompareCallback { port, projection }
    }

    // Visible crate-wide only so crate::secret::backend can host the sealed
    // trait impl. The type's sole constructor remains owner-private.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<ProviderLegacySourceMatchReceipt, SecretInternalError> {
        self.port
            .compare_candidate_equality_activation_sources(self.projection, material)
    }
}

pub(crate) struct RecoveryCandidateEqualityScrubCallback<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    projection: &'a RecoveryProviderProjection,
}

impl RecoveryCandidateEqualityScrubCallback<'_> {
    fn new<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        projection: &'a RecoveryProviderProjection,
    ) -> RecoveryCandidateEqualityScrubCallback<'a> {
        RecoveryCandidateEqualityScrubCallback { port, projection }
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port
            .compare_and_scrub_recovery_equality_sources(self.projection, material)
    }
}

// Actual definition/private constructor live in
// crate::commands::import_export; backend.rs owns only its sealed callback impl.
pub(crate) struct StagedImportCandidateEqualityCompareCallback<'a> {
    port: &'a mut dyn ImportCutoverPort,
    projection: &'a StagedSecretImportActivationProjection,
}

impl StagedImportCandidateEqualityCompareCallback<'_> {
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError> {
        self.port
            .compare_candidate_equality_staged_sources(self.projection, material)
    }
}

// These opaque contexts live in crate::services::configuration_apply::provider.
// Their
// constructors are private to that owner module and require its live Provider
// lease plus a #55 final-baseline receipt.
pub(crate) struct SecretApplyCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
}
pub(crate) struct SecretActivationCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
}
pub(crate) struct ActivationCleanupCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    expected_recovery_cas: SecretRecoveryCas,
}

// Local contexts are minted only by crate::secret::operation after readiness
// claim; they carry no Provider/DB capability.
pub(crate) struct CaptureCompensationCoordinatorContext {
    _private: (),
}
pub(crate) struct DeleteFinalizationCoordinatorContext {
    _private: (),
}

// Defined in crate::commands::provider. Constructor requires the already-held
// Provider delete/detach transaction plus the consumed preview registry row.
pub(crate) struct ProviderDetachCommitId([u8; 16]);

pub(crate) struct OwnerDetachCoordinatorContext<'a> {
    port: &'a mut dyn OwnerDetachCoordinatorPort,
    expected_provider_detach_commit_id: ProviderDetachCommitId,
}

pub(crate) trait OwnerDetachCoordinatorPort {
    fn assert_provider_detach_committed(
        &mut self,
        expected_commit_id: &ProviderDetachCommitId,
        recovery_id: &SecretRecoveryId,
        recovery_cas: &SecretRecoveryCas,
    ) -> Result<ProviderDetachCommitReceipt, SecretInternalError>;

    fn finalize_detach_transaction(
        &mut self,
        receipt: ProviderDetachCommitReceipt,
    ) -> Result<(), SecretInternalError>;
}

impl OwnerDetachCoordinatorContext<'_> {
    pub(crate) fn assert_provider_detach_committed(
        &mut self,
        recovery_id: &SecretRecoveryId,
        recovery_cas: &SecretRecoveryCas,
    ) -> Result<ProviderDetachCommitReceipt, SecretInternalError> {
        self.port.assert_provider_detach_committed(
            &self.expected_provider_detach_commit_id,
            recovery_id,
            recovery_cas,
        )
    }

    pub(crate) fn finalize_detach_transaction(
        &mut self,
        receipt: ProviderDetachCommitReceipt,
    ) -> Result<(), SecretInternalError> {
        self.port.finalize_detach_transaction(receipt)
    }
}

pub(crate) enum SecretRecoveryCoordinatorContext<'a> {
    ActivationCleanup(ActivationCleanupCoordinatorContext<'a>),
    CaptureCompensation(CaptureCompensationCoordinatorContext),
    DeleteFinalization(DeleteFinalizationCoordinatorContext),
    OwnerDetachFinalization(OwnerDetachCoordinatorContext<'a>),
}

// Defined in crate::commands::import_export; this is the sole main-integration
// cutover capability. Its constructor requires the same temp Database live
// object as StagedSecretOwnerToken and a still-admitted #55 staged plan.
pub(crate) struct ImportCutoverCoordinatorContext<'a> {
    port: &'a mut dyn ImportCutoverPort,
}

pub(crate) struct ImportCutoverReceipt {
    receipt_id: ImportCutoverReceiptId,
    durable_temp_database: TempDatabaseDurableObjectId,
    stage_id: ImportStageId,
    provider_row_revision: ProviderRowRevision,
}
pub(crate) struct StagedSourcesScrubReadbackReceipt {
    staged_source_set_cas_after_scrub: StagedSourceSetCas,
    _private: (),
}
trait ImportCutoverPort {
    fn assert_staged_final_baseline(
        &mut self,
        plan: &AdmittedStagedSecretImportPlan,
        staged_owner: &StagedSecretOwnerToken,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<(), SecretInternalError>;

    fn compare_candidate_equality_staged_sources(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        expected: &[u8],
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError>;

    fn validate_staged_explicit_replacement(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        candidate: CandidateReadVerifiedReceipt,
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError>;

    fn scrub_staged_sources_and_readback(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        validated: StagedImportSourceValidationReceipt,
    ) -> Result<StagedSourcesScrubReadbackReceipt, SecretInternalError>;

    fn cutover_sanitized_temp_database(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        scrubbed: StagedSourcesScrubReadbackReceipt,
    ) -> Result<ImportCutoverReceipt, SecretInternalError>;

    fn mint_live_owner_after_cutover(
        &mut self,
        receipt: &ImportCutoverReceipt,
        owner: &SecretOwner,
    ) -> Result<ExistingSecretOwnerToken, SecretInternalError>;
}

// Scanner allowlist: every ImportCutoverPort value-bearing method call occurs
// only inside the ImportCutoverCoordinatorContext impl below. The pre-context
// structural scanner cannot name the port/callback and has no staged-value API.

impl ImportCutoverCoordinatorContext<'_> {
    pub(crate) fn assert_staged_final_baseline(
        &mut self,
        plan: &AdmittedStagedSecretImportPlan,
        staged_owner: &StagedSecretOwnerToken,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_staged_final_baseline(plan, staged_owner, projection)
    }

    pub(crate) fn validate_staged_sources(
        &mut self,
        read: AuthorizedStagedImportRead,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError> {
        match projection.comparison_policy() {
            LegacyActivationComparisonPolicy::CandidateEquality => read
                .compare_candidate_equality_once(StagedImportCandidateEqualityCompareCallback {
                    port: self.port,
                    projection,
                }),
            LegacyActivationComparisonPolicy::ExplicitReplacement => {
                let candidate = read.verify_explicit_replacement_once()?;
                self.port
                    .validate_staged_explicit_replacement(projection, candidate)
            }
        }
    }

    pub(crate) fn scrub_staged_sources_and_readback(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        validated: StagedImportSourceValidationReceipt,
    ) -> Result<StagedSourcesScrubReadbackReceipt, SecretInternalError> {
        self.port
            .scrub_staged_sources_and_readback(projection, validated)
    }

    pub(crate) fn cutover_sanitized_temp_database(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        scrubbed: StagedSourcesScrubReadbackReceipt,
    ) -> Result<ImportCutoverReceipt, SecretInternalError> {
        self.port
            .cutover_sanitized_temp_database(projection, scrubbed)
    }

    pub(crate) fn mint_live_owner_after_cutover(
        &mut self,
        receipt: &ImportCutoverReceipt,
        owner: &SecretOwner,
    ) -> Result<ExistingSecretOwnerToken, SecretInternalError> {
        self.port.mint_live_owner_after_cutover(receipt, owner)
    }
}

impl SecretApplyCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
    ) -> SecretApplyCoordinatorContext<'a> {
        SecretApplyCoordinatorContext { port }
    }

    pub(crate) fn assert_apply_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretApplyPlanProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_apply_final_baseline(plan, projection)
    }
}

impl SecretActivationCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
    ) -> SecretActivationCoordinatorContext<'a> {
        SecretActivationCoordinatorContext { port }
    }

    pub(crate) fn assert_activation_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_activation_final_baseline(plan, projection)
    }

    pub(crate) fn validate_activation_sources(
        &mut self,
        read: AuthorizedActivationRead,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<ProviderActivationSourceValidationReceipt, SecretInternalError> {
        match projection.comparison_policy() {
            LegacyActivationComparisonPolicy::CandidateEquality => read
                .compare_candidate_equality_once(ActivationCandidateEqualityCompareCallback::new(
                    self.port,
                    projection,
                ))
                .map(ProviderActivationSourceValidationReceipt::CandidateEquality),
            LegacyActivationComparisonPolicy::ExplicitReplacement => {
                let candidate = read.verify_explicit_replacement_once()?;
                self.port
                    .validate_explicit_replacement_sources(projection, candidate)
                    .map(ProviderActivationSourceValidationReceipt::ExplicitReplacement)
            }
        }
    }

    pub(crate) fn scrub_activation_and_readback(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        binding: &ActivationBindingCheckpoint,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port.scrub_activation_and_readback(projection, binding)
    }
}

impl ActivationCleanupCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        expected_recovery_cas: SecretRecoveryCas,
    ) -> ActivationCleanupCoordinatorContext<'a> {
        ActivationCleanupCoordinatorContext {
            port,
            expected_recovery_cas,
        }
    }

    pub(crate) fn assert_cleanup_final_baseline(
        &mut self,
        projection: &RecoveryProviderProjection,
    ) -> Result<(), SecretInternalError> {
        if &self.expected_recovery_cas != projection.recovery_cas() {
            return Err(SecretInternalError::recovery_changed());
        }
        self.port.assert_cleanup_final_baseline(projection)
    }

    pub(crate) fn scrub_recovery_with_active_record(
        &mut self,
        read: AuthorizedRecoveryRead,
        projection: &RecoveryProviderProjection,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        read.compare_recovery_source_once(RecoveryCandidateEqualityScrubCallback::new(
            self.port,
            projection,
        ))
    }
}

pub(crate) trait NativeSecretCapture: Send + Sync {
    fn capture_once(
        &self,
        purpose: SecretPurpose,
    ) -> Result<SecretMaterial, SecretInternalError>;
}

pub(crate) trait SecretClock: Send + Sync {
    fn now(&self) -> UtcTimestamp;
}

pub(crate) trait SecretIdSource: Send + Sync {
    fn operation_id(&self) -> SecretOperationId;
    fn candidate_id(&self) -> SecretCandidateId;
    fn secret_ref(&self) -> SecretRef;
    fn audit_event_id(&self) -> SecretAuditEventId;
    fn confirmation_step_id(&self) -> SecretConfirmationStepId;
    fn recovery_id(&self) -> SecretRecoveryId;
}

pub(crate) struct SecretServiceDeps {
    pub(in crate::secret) store_lifetime: SecretStoreLifetime,
    pub(in crate::secret) authority: std::sync::Arc<dyn DeviceLocalSecretAuthority>,
    pub(in crate::secret) backends: std::sync::Arc<dyn SecretBackendRegistry>,
    pub(in crate::secret) broker: std::sync::Arc<BackendOperationBroker>,
    pub(in crate::secret) readiness: std::sync::Arc<dyn SecretReadinessRegistry>,
    pub(in crate::secret) startup_gate: std::sync::Arc<dyn SecretStartupGateRegistry>,
    pub(in crate::secret) change_plans: std::sync::Arc<dyn SecretChangePlanAuthority>,
    pub(in crate::secret) gate: std::sync::Arc<dyn SecretMutationGate>,
    pub(in crate::secret) capture: std::sync::Arc<dyn NativeSecretCapture>,
    pub(in crate::secret) clock: std::sync::Arc<dyn SecretClock>,
    pub(in crate::secret) id: std::sync::Arc<dyn SecretIdSource>,
}
// SecretServiceDeps is an internal move-only assembly row, not a dependency-
// injection API. Its sole literals are scanner-bound to the production and
// fixture factories; neither AppStateBuilder nor any caller accepts/replaces/
// extracts a broker or one of its registry traits.

// Defined in crate::store. Fields and mint functions are private to that
// module; passing the value is possible, constructing one elsewhere is not.
pub(crate) struct SecretServiceConstructionToken {
    _private: (),
}

// All types in this block live in crate::secret::device_store. The opened
// handle is non-Clone/non-serde, owns the exclusive lifetime lock and embeds
// the one bootstrap token. Only SecretBootstrap::open may derive the private
// root from AppHandle; no API accepts PathBuf/String or reopens by root.
struct DeviceLocalSecretRoot(std::path::PathBuf);
pub(crate) struct SecretBootstrapToken {
    _private: (),
}
struct DeviceLocalStoreLifetimeLock {
    // Owns DeviceLocalSecretStore (and its exclusive store.lock) for the
    // process lifetime of this opened handle.
    store: device_store::DeviceLocalSecretStore,
}
pub(crate) struct OpenedDeviceLocalSecretStore {
    root: DeviceLocalSecretRoot,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    bootstrap: SecretBootstrapToken,
    lifetime_lock: DeviceLocalStoreLifetimeLock,
}
pub(crate) struct SecretBootstrap;

const DEVICE_LOCAL_SECRET_SUBDIR: &str = "device-local-secrets";

fn open_device_local_secret_store(
    root: std::path::PathBuf,
) -> Result<OpenedDeviceLocalSecretStore, SecretInternalError> {
    let store = device_store::DeviceLocalSecretStore::open(root.clone())?;
    let device_instance_id = store.device_instance_id().clone();
    Ok(OpenedDeviceLocalSecretStore {
        root: DeviceLocalSecretRoot(root),
        device_instance_id,
        device_store_instance_id: std::sync::Arc::new(DeviceSecretStoreInstanceId(
            uuid::Uuid::new_v4().into_bytes(),
        )),
        bootstrap: SecretBootstrapToken { _private: () },
        lifetime_lock: DeviceLocalStoreLifetimeLock { store },
    })
}

impl SecretBootstrap {
    pub(crate) fn open(
        app_handle: &tauri::AppHandle,
    ) -> Result<OpenedDeviceLocalSecretStore, SecretInternalError> {
        use tauri::Manager;
        let base = app_handle
            .path()
            .app_local_data_dir()
            .or_else(|_| app_handle.path().app_data_dir())
            .map_err(|_| SecretInternalError::input_invalid())?;
        open_device_local_secret_store(base.join(DEVICE_LOCAL_SECRET_SUBDIR))
    }

    #[cfg(test)]
    pub(crate) fn open_for_test(
        root: std::path::PathBuf,
    ) -> Result<OpenedDeviceLocalSecretStore, SecretInternalError> {
        open_device_local_secret_store(root)
    }
}

impl OpenedDeviceLocalSecretStore {
    pub(crate) fn database_preflight_token(&self) -> &SecretBootstrapToken {
        &self.bootstrap
    }

    pub(crate) fn store(&self) -> &device_store::DeviceLocalSecretStore {
        &self.lifetime_lock.store
    }
}

// crate::store owns this non-secret DB path/config authority. It is produced
// by the existing application path resolver; callers cannot pass a raw path.
pub(crate) struct DatabaseOpenAuthority {
    _private: (),
}

impl crate::database::Database {
    pub(crate) fn open_preflight_without_backup(
        authority: &DatabaseOpenAuthority,
        bootstrap: &SecretBootstrapToken,
    ) -> Result<std::sync::Arc<Self>, crate::error::AppError> {
        let _ = (authority, bootstrap);
        todo!("open DB/WAL with automatic/raw backup path disabled")
    }
}

pub(crate) struct SecretBootstrapCleanReceipt {
    legacy_source_coverage: LegacySourceCoverageReceipt,
    _private: (),
}

pub(crate) struct SecretStartupBlockedState {
    issue: SecretIssueView,
    legacy_source_coverage: LegacySourceCoverageReceipt,
    checked_at: UtcTimestamp,
    _private: (),
}

impl SecretBootstrapCleanReceipt {
    pub(crate) fn checked_from_clear_coverage(
        legacy_source_coverage: LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        legacy_source_coverage.assert_complete_clear()?;
        Ok(Self {
            legacy_source_coverage,
            _private: (),
        })
    }
}

impl SecretStartupBlockedState {
    pub(crate) fn checked_from_coverage_and_issue(
        issue: SecretIssueView,
        legacy_source_coverage: LegacySourceCoverageReceipt,
        checked_at: UtcTimestamp,
    ) -> Result<Self, SecretInternalError> {
        legacy_source_coverage.assert_complete()?;
        let _ = &issue;
        // A legacy-source blocker additionally requires
        // assert_complete_blocking(); lock/permission/recovery blockers may
        // retain a complete clear receipt but still cannot yield Clean.
        Ok(Self {
            issue,
            legacy_source_coverage,
            checked_at,
            _private: (),
        })
    }
}

pub(crate) enum SecretStartupGateOutcome {
    Clean(SecretBootstrapCleanReceipt),
    Blocked(SecretStartupBlockedState),
}

// Defined in crate::store. It borrows the already-open preflight Database and
// exposes only exact legacy structural inventory/scrub transaction methods to
// the same SecretService; it cannot construct a second secret authority.
pub(crate) struct StartupSecretReconcileContext<'a> {
    port: Box<dyn StartupSecretReconcilePort + 'a>,
    legacy_sources: CodexLegacySourceInventoryBridge<'a>,
}

pub(crate) trait StartupSecretReconcilePort {
    fn reconcile_exact_journaled_provider_step(
        &mut self,
        projection: &RecoveryProviderProjection,
        read: AuthorizedRecoveryRead,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;
}

impl<'a> StartupSecretReconcileContext<'a> {
    // The constructor is private to crate::store. #35 receives only the
    // already-open Database-backed port and cannot open/clone a Database or
    // acquire a Provider lease itself.
    fn from_open_database_port(
        port: Box<dyn StartupSecretReconcilePort + 'a>,
        legacy_sources: CodexLegacySourceInventoryBridge<'a>,
    ) -> Self {
        StartupSecretReconcileContext {
            port,
            legacy_sources,
        }
    }

    pub(crate) fn inventory_legacy_source_coverage(
        &mut self,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.legacy_sources.fresh_startup_coverage()
    }

    pub(crate) fn reconcile_exact_journaled_provider_step(
        &mut self,
        projection: &RecoveryProviderProjection,
        read: AuthorizedRecoveryRead,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port
            .reconcile_exact_journaled_provider_step(projection, read)
    }
}

pub(crate) trait SecretStartupGateRegistry: Send + Sync {
    fn arm_managed_runtime(
        &self,
        receipt: SecretCommandRegistrationReceipt,
    ) -> Result<(), SecretInternalError>;

    fn assert_managed_runtime_armed(&self) -> Result<(), SecretInternalError>;

    fn publish_clean(
        &self,
        receipt: &SecretBootstrapCleanReceipt,
    ) -> Result<(), SecretInternalError>;

    fn publish_blocked(
        &self,
        blocked: &SecretStartupBlockedState,
    ) -> Result<(), SecretInternalError>;

    fn assert_consumer_allowed(&self) -> Result<(), SecretInternalError>;
}

// crate::store is the sole owner of the port factory. It creates one
// transaction/lease adapter over the exact Arc<Database> already stored in
// AppState. It never opens a Database, resolves a path or creates secret deps.
pub(crate) fn startup_secret_reconcile_context(
    state: &AppState,
) -> Result<StartupSecretReconcileContext<'_>, crate::error::AppError> {
    let _ = state;
    Err(crate::error::AppError::Message(
        "main-integration startup reconcile port is unpublished".to_string(),
    ))
}

// The preparation function lives in crate::store. Its order is exact: one
// device-store open/lock, backup-suppressed DB preflight, construct
// AppState/SecretService from that same handle, then ask that same service and
// authority to reconcile through an external DB context. It does not publish a
// gate state, create a backup or start a worker. The private envelope forces
// crate-root setup to retain the exact outcome while it first manages AppState
// and completes the static command-handler registration.
pub(crate) struct PreparedProductionAppState {
    state: AppState,
    startup: SecretStartupGateOutcome,
}

impl PreparedProductionAppState {
    // Scanner-allowlisted only at the sole src-tauri/src/lib.rs setup callsite.
    pub(in crate) fn into_managed_parts(
        self,
    ) -> (AppState, SecretStartupGateOutcome) {
        (self.state, self.startup)
    }
}

pub(crate) fn open_production_app_state(
    app_handle: tauri::AppHandle,
    database_authority: DatabaseOpenAuthority,
) -> Result<PreparedProductionAppState, crate::error::AppError> {
    let opened_store = SecretBootstrap::open(&app_handle)?;
    let db = crate::database::Database::open_preflight_without_backup(
        &database_authority,
        opened_store.database_preflight_token(),
    )?;
    let state = AppState::new_production(db, app_handle, opened_store)?;
    let _ = &state;
    return Err(crate::error::AppError::Message(
        "main-integration startup reconcile is unpublished".to_string(),
    ));
    #[allow(unreachable_code)]
    let outcome = SecretStartupGateOutcome::Blocked(todo!("unpublished"));
    Ok(PreparedProductionAppState {
        state,
        startup: outcome,
    })
}

// This declaration lives at crate root in src-tauri/src/lib.rs. Its only
// constructor follows app.manage(AppState) at the setup callsite after the
// statically declared invoke_handler list is installed. The two private rows
// prove exactly the 15 #35 handlers and the independent main-integration
// resume handler; resume is deliberately not a SecretCommandName variant.
pub(crate) struct ResumeStagedImportCutoverHandlerRegistration {
    command: SecretMainIntegrationCommandName,
    _private: (),
}

impl ResumeStagedImportCutoverHandlerRegistration {
    pub(crate) fn checked_after_handler_registration(
        command: SecretMainIntegrationCommandName,
    ) -> Result<Self, SecretInternalError> {
        if command != SecretMainIntegrationCommandName::ResumeStagedImportCutover {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self { command, _private: () })
    }
}

pub(crate) struct SecretCommandRegistrationReceipt {
    secret_commands: [SecretCommandName; 15],
    resume_staged_import_cutover: ResumeStagedImportCutoverHandlerRegistration,
}

impl SecretCommandRegistrationReceipt {
    pub(crate) fn checked_after_static_registration(
        secret_commands: [SecretCommandName; 15],
        resume_staged_import_cutover: ResumeStagedImportCutoverHandlerRegistration,
    ) -> Result<Self, SecretInternalError> {
        let expected = [
            SecretCommandName::ListSecretSummaries,
            SecretCommandName::ListSecretBackendOptions,
            SecretCommandName::BeginSecretCapture,
            SecretCommandName::RotateSecret,
            SecretCommandName::ListSecretCandidates,
            SecretCommandName::DiscardSecretCandidate,
            SecretCommandName::SetSecretLocked,
            SecretCommandName::GetSecretDeleteImpact,
            SecretCommandName::DeleteSecret,
            SecretCommandName::GetSecretCleanupImpact,
            SecretCommandName::RetrySecretCleanup,
            SecretCommandName::ValidateSecret,
            SecretCommandName::CheckSecretApplyReadiness,
            SecretCommandName::MigrateLegacyCodexSecrets,
            SecretCommandName::ListSecretAudit,
        ];
        if secret_commands != expected
            || resume_staged_import_cutover.command
                != SecretMainIntegrationCommandName::ResumeStagedImportCutover
        {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self {
            secret_commands,
            resume_staged_import_cutover,
        })
    }
}

// Called from that same setup callsite only after AppState is retrievable via
// app.state::<AppState>(). Clean authorizes one sanitized backup, then gate
// publication, then worker start. Blocked publishes the scrubbed issue but
// starts no backup/worker/consumer. The managed AppState survives both arms.
pub(crate) fn finish_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
    commands_registered: SecretCommandRegistrationReceipt,
) -> Result<(), crate::error::AppError> {
    state
        .secret_service()
        .arm_managed_runtime(commands_registered)?;
    advance_managed_production_secret_startup(state, app_handle, outcome)
}

// Existing repair command handlers call this only after their durable state
// is terminal and a fresh same-service reconcile returns an outcome. The gate
// registry proves the initial manage/registration receipt was already consumed;
// no second receipt or setup call is possible.
pub(crate) fn resume_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
) -> Result<(), crate::error::AppError> {
    state
        .secret_service()
        .assert_managed_runtime_armed()?;
    advance_managed_production_secret_startup(state, app_handle, outcome)
}

fn advance_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
) -> Result<(), crate::error::AppError> {
    match outcome {
        SecretStartupGateOutcome::Clean(clean) => {
            let _ = (state, app_handle, clean);
            return Err(crate::error::AppError::Message(
                "main-integration sanitized backup/worker start is unpublished".to_string(),
            ));
        }
        SecretStartupGateOutcome::Blocked(blocked) => {
            state.secret_service().publish_startup_blocked(&blocked)?;
        }
    }
    Ok(())
}

// Exact sole src-tauri/src/lib.rs setup shape (the receipt constructor is
// private in that module and scanner-bound to the line after app.manage):
// let prepared = crate::store::open_production_app_state(...)?;
// let (state, startup) = prepared.into_managed_parts();
// app.manage(state);
// let resume_handler = ResumeStagedImportCutoverHandlerRegistration::
//     checked_after_handler_registration(
//         SecretMainIntegrationCommandName::ResumeStagedImportCutover,
//     )?;
// let commands = SecretCommandRegistrationReceipt::
//     checked_after_static_registration(REGISTERED_SECRET_COMMANDS_15, resume_handler)?;
// let managed = app.state::<crate::store::AppState>();
// crate::store::finish_managed_production_secret_startup(
//     &managed, app.handle(), startup, commands,
// )?;
// REGISTERED_SECRET_COMMANDS_15 is the literal §9 array in the same order as
// the 15 #35 invoke handlers. The independently registered resume handler is
// adjacent in the Tauri handler list but can never enter that array/type.

// A lock/permission/backend-unavailable observation, any unresolved current
// legacy source state, any adjacent-blocked supplemental observation, or any
// non-terminal durable recovery is a reachable
// security outcome, never a construction error: reconcile_startup returns
// Ok(Blocked(...)), the store publishes that blocker, and AppState reaches the
// scrubbed summary plus the existing capture/migrate/discard/recovery repair
// routes. Those routes do not call assert_consumer_allowed. After a successful
// repair the store invokes resume_managed_production_secret_startup with
// a fresh outcome from this same service and the same
// AppState, SecretService, authority and lifetime lock. A new Clean receipt is
// the sole authority for the first sanitized backup and worker start. Only
// unrecoverable device-store/journal corruption or loss of the already-held
// lifetime lock may leave reconcile_startup as Err and abort construction.
// Clean consumes and retains the exact fresh clear LegacySourceCoverageReceipt;
// Blocked retains the exact blocking receipt beside its checked issue. Neither
// result can be minted from a count/category projection or divergent scan.
// Runtime reads and live apply call assert_consumer_allowed both when minting
// authority and immediately before consume; a Blocked state therefore cannot
// race into material exposure, writer mutation or network construction.

pub(in crate::secret) enum SecretStoreLifetime {
    Production(OpenedDeviceLocalSecretStore),
    #[cfg(any(test, feature = "test-hooks"))]
    Test,
}

pub(crate) struct SecretService {
    store_lifetime: SecretStoreLifetime,
    authority: std::sync::Arc<dyn DeviceLocalSecretAuthority>,
    backends: std::sync::Arc<dyn SecretBackendRegistry>,
    broker: std::sync::Arc<BackendOperationBroker>,
    readiness: std::sync::Arc<dyn SecretReadinessRegistry>,
    startup_gate: std::sync::Arc<dyn SecretStartupGateRegistry>,
    change_plans: std::sync::Arc<dyn SecretChangePlanAuthority>,
    gate: std::sync::Arc<dyn SecretMutationGate>,
    capture: std::sync::Arc<dyn NativeSecretCapture>,
    clock: std::sync::Arc<dyn SecretClock>,
    id: std::sync::Arc<dyn SecretIdSource>,
}

// Existing crate::store::AppState retains every existing field. The only
// additive field is the independently owned secret_service Arc; SecretService has no
// Database, Provider DAO, Provider lease or transaction field.
pub struct AppState {
    pub db: std::sync::Arc<crate::database::Database>,
    pub proxy_service: crate::services::ProxyService,
    pub usage_cache: std::sync::Arc<crate::services::UsageCache>,
    pub codex_desktop_service: std::sync::Arc<crate::services::CodexDesktopService>,
    secret_service: std::sync::Arc<SecretService>,
}

impl AppState {
    pub(crate) fn new_production(
        db: std::sync::Arc<crate::database::Database>,
        app_handle: tauri::AppHandle,
        opened_store: OpenedDeviceLocalSecretStore,
    ) -> Result<Self, SecretInternalError> {
        let construction = SecretServiceConstructionToken { _private: () };
        let secret_service = crate::secret::device_store::new_production_service(
            construction,
            app_handle,
            opened_store,
        )?;
        todo!("construct existing AppState fields unchanged")
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn new_with_secret_test_mode(
        db: std::sync::Arc<crate::database::Database>,
        mode: test_support::SecretTestFixtureMode,
    ) -> Self {
        let construction = SecretServiceConstructionToken { _private: () };
        let secret_service = crate::secret::device_store::new_test_service(
            construction,
            mode,
        );
        todo!("construct existing AppState test fields unchanged")
    }

    pub(crate) fn secret_service(&self) -> &std::sync::Arc<SecretService> {
        &self.secret_service
    }

}

// These two narrow functions live in crate::secret::device_store. Their deps
// builder is private to that module; no caller can submit production authority
// or backend implementations.
fn production_service_deps(
    _app_handle: tauri::AppHandle,
    opened_store: OpenedDeviceLocalSecretStore,
) -> Result<SecretServiceDeps, SecretInternalError> {
    let broker = BackendOperationBroker::from_production_store(&opened_store)?;
    let _ = (opened_store, broker);
    todo!("construct fixed authority/backends/readiness/startup gate and move this exact broker Arc into SecretServiceDeps")
}

pub(crate) fn new_production_service(
    construction: SecretServiceConstructionToken,
    app_handle: tauri::AppHandle,
    opened_store: OpenedDeviceLocalSecretStore,
) -> Result<std::sync::Arc<SecretService>, SecretInternalError> {
    let deps = production_service_deps(app_handle, opened_store)?;
    Ok(std::sync::Arc::new(SecretService::from_deps(
        construction,
        deps,
    )))
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn new_test_service(
    construction: SecretServiceConstructionToken,
    mode: test_support::SecretTestFixtureMode,
) -> std::sync::Arc<SecretService> {
    let deps = secret_test_support::for_mode(mode);
    std::sync::Arc::new(SecretService::from_deps(
        construction,
        deps,
    ))
}

#[cfg(any(test, feature = "test-hooks"))]
mod secret_test_support {
    // Private fixed support factory. No raw dependency/service factory is
    // exported through the test-hooks feature.
    pub(super) fn for_mode(
        mode: super::test_support::SecretTestFixtureMode,
    ) -> super::SecretServiceDeps {
        let broker = super::BackendOperationBroker::from_fixture_mode(mode);
        let _ = (mode, broker);
        todo!("construct fixed test deps and move this exact broker Arc into SecretServiceDeps")
    }
}

#[cfg(any(test, feature = "test-hooks"))]
fn test_database(
) -> Result<std::sync::Arc<crate::database::Database>, crate::error::AppError> {
    todo!("fixed in-memory database fixture")
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn build_test_app_state(
    mode: test_support::SecretTestFixtureMode,
    database: Option<std::sync::Arc<crate::database::Database>>,
) -> Result<AppState, crate::error::AppError> {
    let db = match database {
        Some(database) => database,
        None => test_database()?,
    };
    Ok(AppState::new_with_secret_test_mode(db, mode))
}

#[cfg(any(test, feature = "test-hooks"))]
pub mod test_support {
    // Re-exported as fyagent_lib::test_support::AppStateBuilder. Fields and
    // support/dependency types stay private; integration crates can only choose
    // a closed fault mode and optionally preserve one caller-owned non-secret
    // Arc<Database> through named methods.
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum SecretTestFixtureMode {
        InMemory,
        LockedRead,
        DeniedRead,
        BackendUnavailable,
        VerifyMismatchOnce,
        OldDeleteFailOnce,
    }

    pub struct AppStateBuilder {
        mode: SecretTestFixtureMode,
        database: Option<std::sync::Arc<crate::database::Database>>,
    }

    impl AppStateBuilder {
        pub fn new() -> Self {
            Self {
                mode: SecretTestFixtureMode::InMemory,
                database: None,
            }
        }

        pub fn fixture_mode(mut self, mode: SecretTestFixtureMode) -> Self {
            self.mode = mode;
            self
        }

        pub fn with_database(
            mut self,
            database: std::sync::Arc<crate::database::Database>,
        ) -> Self {
            self.database = Some(database);
            self
        }

        pub fn build(self) -> Result<super::AppState, crate::error::AppError> {
            super::build_test_app_state(self.mode, self.database)
        }
    }
}

impl SecretService {
    // Unique SecretService constructor. Static ownership permits calls only
    // from device_store::new_production_service/new_test_service; those two
    // narrow functions are themselves called only by the two AppState
    // constructors above. Struct literals are forbidden outside this impl.
    pub(in crate::secret) fn from_deps(
        _construction: SecretServiceConstructionToken,
        deps: SecretServiceDeps,
    ) -> Self {
        Self {
            store_lifetime: deps.store_lifetime,
            authority: deps.authority,
            backends: deps.backends,
            broker: deps.broker,
            readiness: deps.readiness,
            startup_gate: deps.startup_gate,
            change_plans: deps.change_plans,
            gate: deps.gate,
            capture: deps.capture,
            clock: deps.clock,
            id: deps.id,
        }
    }

    pub(crate) fn list_secret_summaries(
        &self,
        request: ListSecretSummariesRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<ListSecretSummariesResult, SecretInternalError> {
        let snapshot = self.authority.read_secret_summary_snapshot(&request)?;
        let mut owners = Vec::with_capacity(snapshot.owners.len());
        for row in snapshot.owners {
            let coverage = legacy_sources
                .fresh_owner_summary_coverage(&row.owner)?;
            owners.push(SecretOwnerCredentialSummary::checked_from_authority(
                row.summary,
                &coverage,
            )?);
        }
        ListSecretSummariesResult::checked_from_authority(
            ListSecretSummariesResult {
                owners,
                refs: snapshot.refs,
                next_cursor: snapshot.next_cursor,
            },
        )
    }

    pub(crate) fn list_secret_backend_options(
        &self,
        owner: ExistingSecretOwnerToken,
        request: ListSecretBackendOptionsRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError> {
        let now = self.clock.now();
        let legacy_source_coverage = legacy_sources
            .fresh_capture_coverage(&owner)?;
        let registration = self.authority
            .capture_intent_registration_from_atomic_snapshot(
                owner,
                request,
                legacy_source_coverage,
                self.backends.as_ref(),
                &now,
            )?;
        self.broker
            .mint_capture_intent_from_atomic_snapshot(registration)
    }

    pub(crate) fn begin_secret_capture(
        &self,
        request: BeginSecretCaptureRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<StageSecretCandidateResult, SecretInternalError> {
        let now = self.clock.now();
        let claim = self.broker.claim_capture_intent_and_fresh_revalidate(
            request.capture_intent_id,
            &request.backend_instance_id,
            &now,
            legacy_sources,
            self.authority.as_ref(),
            self.backends.as_ref(),
        )?;
        match self.stage_claimed_capture(&claim, &now) {
            Ok(result) => {
                self.broker.consume_capture_intent(claim)?;
                Ok(result)
            }
            Err(error) => {
                self.broker.terminalize_capture_intent(
                    claim,
                    PendingConfirmationTermination::Failed,
                )?;
                Err(error)
            }
        }
    }

    fn stage_claimed_capture(
        &self,
        claim: &ClaimedSecretCaptureIntent,
        now: &UtcTimestamp,
    ) -> Result<StageSecretCandidateResult, SecretInternalError> {
        let _ = (claim, now);
        todo!("single native capture/write/verify/journal flow; native input cancellation is an Err consumed by begin_secret_capture terminalization")
    }

    pub(crate) fn check_apply_readiness(
        &self,
        owner: ExistingSecretOwnerToken,
        request: CheckSecretApplyReadinessRequest,
    ) -> Result<SecretApplyReadiness, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Called before the Provider lease. It never reads material.
    pub(crate) fn prepare_for_apply(
        &self,
        plan: AdmittedSecretChangePlan,
        projection: SecretApplyPlanProjection,
    ) -> Result<PrepareForApply, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Called before the Provider lease. Consumes pending native state.
    pub(crate) fn confirm_for_apply(
        &self,
        pending: PendingSecretConfirmation,
    ) -> Result<PrepareForApply, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Consumes pending native state, terminates its backend session, invalidates
    // every already-prepared role, terminates the admission and registry row.
    pub(crate) fn cancel_for_apply(
        &self,
        pending: PendingSecretConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Expiry calls cancel_for_apply(..., Expired); renderer cancellation calls
    // UserCancelled; job/baseline discard calls Discarded. No Drop-only cleanup.
    pub(crate) fn discard_prepared(
        &self,
        capabilities: PreparedSecretCapabilityBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // The registered discard_secret_candidate handler calls this native-only
    // preparation entry. It creates/loads the immutable disposition journal,
    // generates a fresh operation id and prepares exactly RecordDelete then
    // reservation-bound RecordMissingReadback before any backend mutation.
    pub(crate) fn prepare_candidate_discard(
        &self,
        request: DiscardSecretCandidateRequest,
    ) -> Result<PrepareCandidateDiscard, SecretInternalError> {
        todo!("closed two-slot candidate-discard preparation; terminal replay returns AlreadyTerminal without a slot")
    }

    pub(crate) fn confirm_candidate_discard(
        &self,
        pending: PendingCandidateDiscardConfirmation,
    ) -> Result<PrepareCandidateDiscard, SecretInternalError> {
        todo!("consume only the pending variant's fixed slot; after RecordDelete confirmation prepare/confirm RecordMissingReadback before returning a bundle")
    }

    pub(crate) fn cancel_candidate_discard(
        &self,
        pending: PendingCandidateDiscardConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminalize the fresh operation/pending backend session while preserving the durable journal target and candidate reachability")
    }

    pub(crate) fn discard_prepared_candidate_discard(
        &self,
        bundle: PreparedCandidateDiscardBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("invalidate both one-shot authorizations; preserve immutable nonterminal candidate journal")
    }

    pub(crate) fn execute_candidate_discard(
        &self,
        bundle: PreparedCandidateDiscardBundle,
    ) -> Result<DiscardSecretCandidateResult, SecretInternalError> {
        todo!("under one candidate mutation permit consume delete, persist three-field checkpoint, unlock/consume Validate missing, persist MissingReadbackVerified, then finalize exact disposition")
    }

    // Activation is prepared separately from live apply. Before #41 takes a
    // lease it prepares the mandatory candidate-read/compare authorization
    // and, when projected, the old-record delete authorization. It never reads
    // material during prepare/confirm.
    pub(crate) fn prepare_candidate_activation(
        &self,
        plan: AdmittedSecretChangePlan,
        projection: SecretCandidateActivationProjection,
    ) -> Result<PrepareCandidateActivation, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn confirm_candidate_activation(
        &self,
        pending: PendingCandidateActivationConfirmation,
    ) -> Result<PrepareCandidateActivation, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn cancel_candidate_activation(
        &self,
        pending: PendingCandidateActivationConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn discard_prepared_activation(
        &self,
        bundle: PreparedCandidateActivationBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Called after Provider lease + #55 baseline recheck + backup.
    pub(crate) fn resolve_for_apply(
        &self,
        coordinator: &mut SecretApplyCoordinatorContext<'_>,
        capabilities: &mut PreparedSecretCapabilityBundle,
        invocation: SecretApplyWriterInvocation<'_>,
    ) -> Result<SecretApplyResultDto, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Invalidates every still-prepared role and consumes the plan admission.
    pub(crate) fn finish_apply(
        &self,
        capabilities: PreparedSecretCapabilityBundle,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // #35 never acquires a Provider lease. #41 passes the already-held lease +
    // final baseline token, and this call performs fresh compare, local CAS,
    // Provider scrub and journal finalization before that lease is released.
    pub(crate) fn activate_candidate_from_change_plan(
        &self,
        coordinator: &mut SecretActivationCoordinatorContext<'_>,
        bundle: PreparedCandidateActivationBundle,
    ) -> Result<SecretActivationResultDto, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Main integration, not #41, prepares/optionally confirms this bundle
    // before it acquires its opaque temp-DB cutover context.
    pub(crate) fn prepare_staged_import(
        &self,
        plan: AdmittedStagedSecretImportPlan,
        staged_owner: StagedSecretOwnerToken,
        authority_match: StagedImportAuthorityMatchReceipt,
        projection: StagedSecretImportActivationProjection,
    ) -> Result<PrepareStagedImport, SecretInternalError> {
        todo!("closed staged-only preparation")
    }

    pub(crate) fn confirm_staged_import(
        &self,
        pending: PendingStagedImportConfirmation,
    ) -> Result<PrepareStagedImport, SecretInternalError> {
        todo!("consume exact staged pending confirmation")
    }

    pub(crate) fn cancel_staged_import(
        &self,
        pending: PendingStagedImportConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminate staged pending/admission state")
    }

    pub(crate) fn discard_prepared_staged_import(
        &self,
        bundle: PreparedStagedImportBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("consume candidate authorization, terminate staged admission and registry state")
    }

    pub(crate) fn activate_staged_import(
        &self,
        coordinator: &mut ImportCutoverCoordinatorContext<'_>,
        bundle: PreparedStagedImportBundle,
    ) -> Result<StagedSecretImportActivationResultDto, SecretInternalError> {
        todo!("validate/scrub/cutover/mint live owner/finalize local binding")
    }

    pub(crate) fn get_recovery_impact(
        &self,
        request: GetSecretCleanupImpactRequest,
    ) -> Result<SecretRecoveryImpact, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Called by retry_secret_cleanup before selecting the kind-specific
    // coordinator. It prepares exactly that row's hardware/backend slots;
    // only a completed activationCleanup bundle later asks #41 for a lease.
    pub(crate) fn prepare_recovery(
        &self,
        request: RetrySecretCleanupRequest,
    ) -> Result<PrepareSecretRecovery, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn confirm_recovery(
        &self,
        pending: PendingSecretRecoveryConfirmation,
    ) -> Result<PrepareSecretRecovery, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn cancel_recovery(
        &self,
        pending: PendingSecretRecoveryConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminate backend pending session and pending registry row")
    }

    pub(crate) fn discard_prepared_recovery(
        &self,
        bundle: PreparedSecretRecoveryBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("invalidate every prepared cleanup authorization")
    }

    pub(crate) fn retry_recovery(
        &self,
        coordinator: &mut SecretRecoveryCoordinatorContext<'_>,
        bundle: PreparedSecretRecoveryBundle,
    ) -> Result<SecretRecoveryResult, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // This is the only startup reconciliation entry. It uses the production
    // authority already retained by this service and the port over AppState's
    // already-open Database. It never constructs a temporary authority,
    // reopens the device store or starts a consumer.
    pub(crate) fn reconcile_startup(
        &self,
        context: &mut StartupSecretReconcileContext<'_>,
    ) -> Result<SecretStartupGateOutcome, SecretInternalError> {
        let legacy_source_coverage =
            context.inventory_legacy_source_coverage()?;
        let _ = legacy_source_coverage;
        todo!(
            "consume this fresh complete eleven-domain receipt into Clean only when both retained sets are empty, otherwise retain it in Blocked; map lock/recovery blockers to Ok(Blocked), reserve Err for fatal store corruption or inability to retain the lifetime lock"
        )
    }

    pub(crate) fn publish_startup_clean(
        &self,
        clean: &SecretBootstrapCleanReceipt,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.publish_clean(clean)
    }

    pub(crate) fn arm_managed_runtime(
        &self,
        receipt: SecretCommandRegistrationReceipt,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.arm_managed_runtime(receipt)
    }

    pub(crate) fn assert_managed_runtime_armed(
        &self,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.assert_managed_runtime_armed()
    }

    pub(crate) fn publish_startup_blocked(
        &self,
        blocked: &SecretStartupBlockedState,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.publish_blocked(blocked)
    }
}

// Owned only by crate::secret::device_store. The authority mints it from one
// fresh device-local binding snapshot; no DAO/runtime module can assemble it.
struct RuntimeSecretBindingIdentityOwned {
    owner: ExistingSecretOwnerToken,
    owner_binding_revision: SecretOwnerBindingRevision,
    secret_ref: SecretRef,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
}

pub(crate) struct AuthorityMintedRuntimeBinding {
    consumer: FixedRuntimeConsumer,
    identity: RuntimeSecretBindingIdentityOwned,
    authority_nonce: [u8; 16],
}

// Borrow-only, non-authorizing identity view. Only
// AuthorityMintedRuntimeBinding::identity can construct it.
pub(crate) struct RuntimeSecretBindingIdentity<'a> {
    owner: &'a ExistingSecretOwnerToken,
    owner_binding_revision: &'a SecretOwnerBindingRevision,
    secret_ref: &'a SecretRef,
    binding_revision: &'a SecretBindingRevision,
    record_revision: &'a SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: &'a SecretBindingSetCas,
}

impl AuthorityMintedRuntimeBinding {
    // This factory is private in crate::secret::device_store and is called only
    // by its DeviceLocalSecretAuthority implementation after a fresh read.
    fn mint(
        consumer: FixedRuntimeConsumer,
        identity: RuntimeSecretBindingIdentityOwned,
        authority_nonce: [u8; 16],
    ) -> Self {
        Self { consumer, identity, authority_nonce }
    }

    pub(crate) fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        RuntimeSecretBindingIdentity {
            owner: &self.identity.owner,
            owner_binding_revision: &self.identity.owner_binding_revision,
            secret_ref: &self.identity.secret_ref,
            binding_revision: &self.identity.binding_revision,
            record_revision: &self.identity.record_revision,
            store_revision: self.identity.store_revision,
            binding_set_cas: &self.identity.binding_set_cas,
        }
    }

    pub(crate) fn require_consumer(
        &self,
        expected: FixedRuntimeConsumer,
    ) -> Result<(), SecretInternalError> {
        if std::mem::discriminant(&self.consumer) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                SecretTerminalOperationContext::Runtime(expected),
            ))
        }
    }
}

// These wrappers live in their exact runtime owner modules. Their private
// constructors accept only an authority-minted token; no constructor accepts
// owner/ref/revision scalar fields.
pub(crate) struct ProxyRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct UsageRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct CodingPlanRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct ModelFetchRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}

impl ProxyRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::ProxyRequest)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl UsageRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::UsageProbe)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl CodingPlanRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::CodingPlanUsageProbe)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl ModelFetchRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::ModelFetch)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl RuntimeSecretBindingIdentity<'_> {
    pub(crate) fn owner(&self) -> &ExistingSecretOwnerToken {
        self.owner
    }

    pub(crate) fn owner_binding_revision(&self) -> &SecretOwnerBindingRevision {
        self.owner_binding_revision
    }

    pub(crate) fn secret_ref(&self) -> &SecretRef {
        self.secret_ref
    }

    pub(crate) fn binding_revision(&self) -> &SecretBindingRevision {
        self.binding_revision
    }

    pub(crate) fn record_revision(&self) -> &SecretRecordRevision {
        self.record_revision
    }

    pub(crate) fn store_revision(&self) -> SecretStoreRevision {
        self.store_revision
    }

    pub(crate) fn binding_set_cas(&self) -> &SecretBindingSetCas {
        self.binding_set_cas
    }
}

pub(crate) struct ProxyRequestSecretExecution {
    binding: ProxyRuntimeSecretBinding,
    metadata: ProxyRequestMetadata,
    request: ProxySingleSendRequestHandle,
}

wire_enum!(ProxyHttpMethod { Get, Post });
wire_enum!(CodexProxyRoute { Responses, ChatCompletions });

pub(crate) struct NoRedirectPolicy;

impl NoRedirectPolicy {
    // Owner-private HTTP client factories are required to call this exact
    // function; no caller-supplied redirect policy or default client is legal.
    fn reqwest_policy(&self) -> reqwest::redirect::Policy {
        reqwest::redirect::Policy::none()
    }
}

pub(crate) struct ProxyRequestMetadata {
    operation_id: SecretOperationId,
    method: ProxyHttpMethod,
    route: CodexProxyRoute,
    upstream: ValidatedUrl,
    content_length: u64,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct ProxySingleSendRequestHandle {
    _private: (),
}

pub(crate) struct ProxyRequestExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedProxyRequest {
    metadata: ProxyRequestMetadata,
    request: ProxySingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl ProxyRequestSecretExecution {
    fn new(
        binding: ProxyRuntimeSecretBinding,
        metadata: ProxyRequestMetadata,
        request: ProxySingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    // Called only by crate::secret::backend's sealed callback impl; the sole
    // constructor remains private to crate::proxy::forwarder.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedProxyRequest, SecretInternalError> {
        Ok(PreparedProxyRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedProxyRequest {
    // Consumes metadata, request body/route and authorization in one transport
    // await. There is no retry/clone/get-header API.
    pub(crate) async fn send_once(
        self,
    ) -> Result<ProxyRequestExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

pub(crate) enum UsageProbeKind {
    Usage,
    Balance,
}

pub(crate) struct UsageProbeMetadata {
    operation_id: SecretOperationId,
    probe: UsageProbeKind,
    upstream: ValidatedUrl,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct UsageProbeSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct UsageProbeSecretExecution {
    binding: UsageRuntimeSecretBinding,
    metadata: UsageProbeMetadata,
    request: UsageProbeSingleSendRequestHandle,
}
pub(crate) struct UsageProbeExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedUsageProbeRequest {
    metadata: UsageProbeMetadata,
    request: UsageProbeSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl UsageProbeSecretExecution {
    fn new(
        binding: UsageRuntimeSecretBinding,
        metadata: UsageProbeMetadata,
        request: UsageProbeSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedUsageProbeRequest, SecretInternalError> {
        Ok(PreparedUsageProbeRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedUsageProbeRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<UsageProbeExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

pub(crate) enum CodingPlanPrimaryAdapter {
    Kimi,
    Zhipu,
    MiniMax,
}

pub(crate) struct CodingPlanMetadata {
    operation_id: SecretOperationId,
    adapter: CodingPlanPrimaryAdapter,
    upstream: ValidatedUrl,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct CodingPlanSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct CodingPlanSecretExecution {
    binding: CodingPlanRuntimeSecretBinding,
    metadata: CodingPlanMetadata,
    request: CodingPlanSingleSendRequestHandle,
}
pub(crate) struct CodingPlanExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedCodingPlanRequest {
    metadata: CodingPlanMetadata,
    request: CodingPlanSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl CodingPlanSecretExecution {
    fn new(
        binding: CodingPlanRuntimeSecretBinding,
        metadata: CodingPlanMetadata,
        request: CodingPlanSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedCodingPlanRequest, SecretInternalError> {
        Ok(PreparedCodingPlanRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedCodingPlanRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<CodingPlanExecutionReceipt, SecretInternalError> {
        todo!("fixed primary-key adapter; one await; redirect policy none")
    }
}

pub(crate) struct ModelFetchMetadata {
    operation_id: SecretOperationId,
    upstream: ValidatedUrl,
    model_provider_id: CodexModelProviderId,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct ModelFetchSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct ModelFetchSecretExecution {
    binding: ModelFetchRuntimeSecretBinding,
    metadata: ModelFetchMetadata,
    request: ModelFetchSingleSendRequestHandle,
}
pub(crate) struct ModelFetchExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedModelFetchRequest {
    metadata: ModelFetchMetadata,
    request: ModelFetchSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl ModelFetchSecretExecution {
    fn new(
        binding: ModelFetchRuntimeSecretBinding,
        metadata: ModelFetchMetadata,
        request: ModelFetchSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedModelFetchRequest, SecretInternalError> {
        Ok(PreparedModelFetchRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedModelFetchRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<ModelFetchExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

// The type blocks above live in their exact owner modules, not crate::secret:
// proxy types: crate::proxy::forwarder; the Codex adapter supplies only closed
// route metadata and cannot construct or retain the execution token
// usage types: crate::services::provider::usage
// primary-key coding-plan types: crate::services::coding_plan
// model-fetch types: crate::services::model_fetch
// Each execution token has a private owner-module constructor and exactly one
// scanner-allowlisted factory callsite. #35 receives it opaquely and can only
// ask for binding_identity, then pass the token only to the backend-owned
// sealed callback. No crate-wide adapter constructor exists.

impl SecretService {
    pub(crate) fn mint_proxy_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::ProxyRequest)
    }

    pub(crate) fn mint_usage_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::UsageProbe)
    }

    pub(crate) fn mint_model_fetch_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::ModelFetch)
    }

    pub(crate) fn mint_coding_plan_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority.mint_runtime_binding(
            owner,
            FixedRuntimeConsumer::CodingPlanUsageProbe,
        )
    }

    pub(crate) async fn execute_proxy_request(
        self: &std::sync::Arc<Self>,
        request: ProxyRequestSecretExecution,
    ) -> Result<ProxyRequestExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    pub(crate) async fn execute_usage_probe(
        self: &std::sync::Arc<Self>,
        request: UsageProbeSecretExecution,
    ) -> Result<UsageProbeExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    pub(crate) async fn execute_coding_plan_usage_probe(
        self: &std::sync::Arc<Self>,
        request: CodingPlanSecretExecution,
    ) -> Result<CodingPlanExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("spawn_blocking resolve, then one redirect-none send_once await")
    }

    pub(crate) async fn execute_model_fetch(
        self: &std::sync::Arc<Self>,
        request: ModelFetchSecretExecution,
    ) -> Result<ModelFetchExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }
}

wire_enum!(SecretDiscardReason {
    PlanStale, BaselineChanged, BackupFailed, JobCancelled, Shutdown
});
