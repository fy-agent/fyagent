#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SecretStoreRevision(u64);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BackendDeleteAppliedRevision(u64);

impl BackendDeleteAppliedRevision {
    fn checked(value: u64) -> Result<Self, SecretInternalError> {
        if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BackendDeleteAppliedCas {
    revision: BackendDeleteAppliedRevision,
    digest: RecoveryStructureDigest,
}

impl BackendDeleteAppliedCas {
    fn checked_from_durable_backend_applied(
        revision: BackendDeleteAppliedRevision,
        digest: RecoveryStructureDigest,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        let _ = journal;
        todo!("accept only the exact just-persisted backendApplied phase and its credential-free structural preimage")
    }
}

pub(crate) struct BackendDeleteAppliedCasReservation {
    operation_id: SecretOperationId,
    expected_revision: BackendDeleteAppliedRevision,
    _private: (),
}

// The broker reserves only the next operation-bound revision before any
// prompt; it cannot predict a receipt-derived digest. After delete is durably
// journaled, authority mints the actual CAS and the missing-readback authorize
// method must consume this reservation via consume_fulfilled_by before it can
// pass that actual CAS into AuthorizedBackendMissingReadback.

impl BackendDeleteAppliedCasReservation {
    fn consume_fulfilled_by(
        self,
        operation_id: &SecretOperationId,
        actual: &BackendDeleteAppliedCas,
    ) -> Result<(), SecretInternalError> {
        if &self.operation_id == operation_id
            && &self.expected_revision == &actual.revision
        {
            Ok(())
        } else {
            Err(SecretInternalError::dependency_changed())
        }
    }
}

impl SecretStoreRevision {
    pub(in crate::secret) fn parse(value: u64) -> Result<Self, SecretInternalError> {
        if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }


    pub(in crate::secret) fn get(self) -> u64 {
        self.0
    }
}

// Native-only: SecretStoreRevision has no Serialize/Deserialize implementation.

// The locator and handle definitions live in crate::secret::backend. This
// private locator type is not re-exported from that module.
struct BackendRecordLocator(String);

impl BackendRecordLocator {
    fn parse(value: String) -> Result<Self, SecretInternalError> {
        let bytes = value.as_bytes();
        let valid = (1..=128).contains(&bytes.len())
            && bytes[0].is_ascii_alphanumeric()
            && bytes.iter().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(*byte, b'.' | b'_' | b':' | b'@' | b'=' | b'-')
            })
            && !credential_shaped_ascii(&value);
        if valid {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}
// This private repr is the complete authorization algebra. The wrapper has no
// Serialize/Deserialize/Clone/Debug and its sole factory is in this module.
// Consequently an operation owner can request preparation but cannot forge,
// narrow, widen or transplant the scope returned by the registered backend.
enum BackendAuthorizationScopeKind {
    Apply {
        role: SecretApplyRole,
        projection_digest: SecretProjectionDigest,
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
        consumer: SecretChangePlanApplyConsumer,
        target_sink: SecretChangePlanApplySink,
        live_sink_id: CodexLiveSecretSinkId,
    },
    Runtime {
        consumer: FixedRuntimeConsumer,
        sink: FixedRuntimeSink,
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
    },
    Activation {
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        projection_digest: SecretProjectionDigest,
        comparison_policy: LegacyActivationComparisonPolicy,
        slot: ActivationConfirmationSlot,
    },
    Recovery {
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        slot: RecoveryConfirmationSlot,
    },
    Migration {
        report_id: SecretMigrationReportId,
        owner: SecretOwner,
        comparison_policy: LegacyActivationComparisonPolicy,
    },
    StagedImport {
        authority: StagedImportBackendAuthorityScope,
        candidate_id: SecretCandidateId,
        projection_digest: SecretProjectionDigest,
        comparison_policy: LegacyActivationComparisonPolicy,
        slot: StagedImportConfirmationSlot,
    },
    General {
        operation: SecretNonApplyBackendOperation,
        owner: SecretOwner,
    },
}

pub(crate) enum FixedRuntimeConsumer {
    ProxyRequest,
    UsageProbe,
    CodingPlanUsageProbe,
    ModelFetch,
}

impl FixedRuntimeConsumer {
    fn required_record_consumer(&self) -> SecretRuntimeConsumer {
        match self {
            Self::ProxyRequest => SecretRuntimeConsumer::ProxyRequest,
            Self::UsageProbe => SecretRuntimeConsumer::UsageProbe,
            Self::CodingPlanUsageProbe => {
                SecretRuntimeConsumer::CodingPlanUsageProbe
            }
            Self::ModelFetch => SecretRuntimeConsumer::ModelFetch,
        }
    }
}

pub(crate) enum FixedRuntimeSink {
    ProcessMemory,
}

struct BackendAuthorizationScopeRepr {
    registered_backend: RegisteredBackendHandleBinding,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    operation_id: SecretOperationId,
    kind: BackendAuthorizationScopeKind,
    terminal_error_context: SecretTerminalOperationContext,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct BackendAuthorizationScope(
    BackendAuthorizationScopeRepr,
);

impl BackendAuthorizationScope {
    // Private to crate::secret::backend. It copies the record identity and the
    // closed operation context after their equality has been checked.
    fn mint_from_context(
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        context: BrokeredBackendOperationContext,
    ) -> Result<Self, SecretInternalError> {
        todo!("unwrap only inside backend; validate complete brokered context/record/registered-handle/store tuple and its exact closed terminal-error context; staged arm consumes live-authority match; mint sealed scope")
    }

    fn into_terminal_error_context(self) -> SecretTerminalOperationContext {
        self.0.terminal_error_context
    }

    fn matches(
        &self,
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> bool {
        todo!("Arc identity plus instance/generation/record/store/binding/device/capability/operation/route/expiry comparison; never partial")
    }

    fn assert_registered_handle(
        &self,
        backend: &BackendInstanceHandle,
    ) -> Result<(), SecretInternalError> {
        self.0.registered_backend.assert_same(backend)
    }

    fn validate_confirmation_requirement(
        &self,
        backend: &BackendInstanceHandle,
        operation: SecretBackendOperation,
        confirmation: PhysicalConfirmation,
        requirement: &PlatformConfirmationRequirement,
    ) -> Result<(), SecretInternalError> {
        let _ = (backend, operation, confirmation, requirement);
        todo!("exact registered object/device/operation/policy/timeout/prompt/scope-expiry validation")
    }

    fn validate_pending_requirement(
        &self,
        backend: &BackendInstanceHandle,
        requirement: &BackendPendingRequirementIdentity,
        now: &UtcTimestamp,
        termination: Option<&PendingConfirmationTermination>,
    ) -> Result<(), SecretInternalError> {
        let _ = (backend, requirement, now, termination);
        todo!("same registered object/instance/generation/device/operation/confirmation/timeout/prompt; confirm requires unexpired, Expired termination requires elapsed deadline, other termination consumes exact row")
    }

    fn platform_requirement(
        &self,
    ) -> Result<PlatformOperationRequirement<'_>, SecretInternalError> {
        let operation = match &self.0.kind {
            BackendAuthorizationScopeKind::Apply { .. }
            | BackendAuthorizationScopeKind::Runtime { .. } => {
                SecretBackendOperation::ResolveForApply
            }
            BackendAuthorizationScopeKind::Activation { slot, .. } => match slot {
                ActivationConfirmationSlot::CandidateRead => {
                    SecretBackendOperation::ResolveForApply
                }
                ActivationConfirmationSlot::OldRecordDelete => {
                    SecretBackendOperation::Delete
                }
                ActivationConfirmationSlot::OldRecordMissingReadback => {
                    SecretBackendOperation::Validate
                }
            },
            BackendAuthorizationScopeKind::Recovery { slot, .. } => match slot {
                RecoveryConfirmationSlot::ActiveRecordRead => {
                    SecretBackendOperation::ResolveForApply
                }
                RecoveryConfirmationSlot::OldRecordDelete
                | RecoveryConfirmationSlot::UncommittedRecordDelete
                | RecoveryConfirmationSlot::AdmittedRecordDelete => {
                    SecretBackendOperation::Delete
                }
                RecoveryConfirmationSlot::OldRecordMissingReadback
                | RecoveryConfirmationSlot::UncommittedRecordMissingReadback
                | RecoveryConfirmationSlot::AdmittedRecordMissingReadback => {
                    SecretBackendOperation::Validate
                }
            },
            BackendAuthorizationScopeKind::Migration { .. }
            | BackendAuthorizationScopeKind::StagedImport { .. } => {
                SecretBackendOperation::ResolveForApply
            }
            BackendAuthorizationScopeKind::General { operation, .. } => match operation {
                SecretNonApplyBackendOperation::CaptureVerify => {
                    SecretBackendOperation::CaptureVerify
                }
                SecretNonApplyBackendOperation::Validate => SecretBackendOperation::Validate,
                SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordDelete,
                    ..
                }
                | SecretNonApplyBackendOperation::DirectDelete => {
                    SecretBackendOperation::Delete
                }
                SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
                    ..
                } => SecretBackendOperation::Validate,
                SecretNonApplyBackendOperation::Revoke => SecretBackendOperation::Revoke,
            },
        };
        Ok(PlatformOperationRequirement {
            scope: self,
            operation,
            confirmation: self.0.confirmation,
        })
    }

    fn require_route(&self, route: AuthorizedReadRoute) -> Result<(), SecretInternalError> {
        let matches = match (&self.0.kind, route) {
            (BackendAuthorizationScopeKind::Apply { .. }, AuthorizedReadRoute::Apply)
            | (BackendAuthorizationScopeKind::Activation { slot: ActivationConfirmationSlot::CandidateRead, .. }, AuthorizedReadRoute::Activation)
            | (BackendAuthorizationScopeKind::Recovery { slot: RecoveryConfirmationSlot::ActiveRecordRead, .. }, AuthorizedReadRoute::Recovery)
            | (BackendAuthorizationScopeKind::Migration { .. }, AuthorizedReadRoute::Migration)
            | (BackendAuthorizationScopeKind::StagedImport { .. }, AuthorizedReadRoute::StagedImport)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::ProxyRequest, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::Proxy)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::UsageProbe, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::Usage)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::CodingPlanUsageProbe, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::CodingPlan)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::ModelFetch, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::ModelFetch)
            | (BackendAuthorizationScopeKind::General { operation: SecretNonApplyBackendOperation::Validate, .. }, AuthorizedReadRoute::Validation) => true,
            _ => false,
        };
        if matches {
            Ok(())
        } else {
            Err(SecretInternalError::dependency_changed())
        }
    }

    fn require_delete_mode(
        &self,
        mode: BackendDeleteMode,
    ) -> Result<(), SecretInternalError> {
        let _ = mode;
        todo!("exact activation/recovery/general delete-or-revoke scope and mode; CandidateDiscard permits Delete only for RecordDelete and rejects RecordMissingReadback")
    }

    fn require_revoke_observation(&self) -> Result<(), SecretInternalError> {
        match &self.0.kind {
            BackendAuthorizationScopeKind::General {
                operation: SecretNonApplyBackendOperation::Revoke,
                ..
            } => Ok(()),
            _ => Err(SecretInternalError::dependency_changed()),
        }
    }

    fn require_missing_readback(&self) -> Result<(), SecretInternalError> {
        match &self.0.kind {
            BackendAuthorizationScopeKind::Activation {
                slot: ActivationConfirmationSlot::OldRecordMissingReadback,
                ..
            }
            | BackendAuthorizationScopeKind::Recovery {
                slot: RecoveryConfirmationSlot::OldRecordMissingReadback
                    | RecoveryConfirmationSlot::UncommittedRecordMissingReadback
                    | RecoveryConfirmationSlot::AdmittedRecordMissingReadback,
                ..
            }
            | BackendAuthorizationScopeKind::General {
                operation: SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
                    ..
                },
                ..
            } => Ok(()),
            _ => Err(SecretInternalError::dependency_changed()),
        }
    }
}

#[derive(Clone, Copy)]
enum AuthorizedReadRoute {
    Apply,
    Activation,
    Recovery,
    Migration,
    StagedImport,
    Proxy,
    Usage,
    CodingPlan,
    ModelFetch,
    Validation,
}

pub(crate) struct BackendAuthorizationHandle {
    authorization_id: u128,
    scope: BackendAuthorizationScope,
}
pub(crate) struct BackendPendingConfirmation {
    pending_id: u128,
    scope: BackendAuthorizationScope,
    requirement: BackendPendingRequirementIdentity,
}

struct BackendPendingRequirementIdentity {
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    operation: SecretBackendOperation,
    confirmation: PhysicalConfirmation,
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct ConsumedBackendAuthorization {
    authorization_id: u128,
    scope: BackendAuthorizationScope,
}

impl BackendAuthorizationHandle {
    // Private to crate::secret::backend::authorization. Only the registered
    // backend wrapper mints after ready/confirmed platform evidence.
    fn mint(
        authorization_id: u128,
        scope: BackendAuthorizationScope,
    ) -> Self {
        Self {
            authorization_id,
            scope,
        }
    }

    fn consume(
        self,
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<ConsumedBackendAuthorization, SecretInternalError> {
        if !self.scope.matches(backend, record, operation_id, now) {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.scope.into_terminal_error_context(),
            ));
        }
        Ok(ConsumedBackendAuthorization {
            authorization_id: self.authorization_id,
            scope: self.scope,
        })
    }
}

impl BackendPendingConfirmation {
    // Same owner/privacy as BackendAuthorizationHandle::mint.
    fn mint(
        pending_id: u128,
        scope: BackendAuthorizationScope,
        requirement: BackendPendingRequirementIdentity,
    ) -> Self {
        Self {
            pending_id,
            scope,
            requirement,
        }
    }
}

// Locator/auth/pending/consumed types have private fields and no
// Serialize/Deserialize/Clone/Debug. Registries store non-material ids/nonces
// only. Mint/consume calls are scanner-allowlisted to crate::secret::backend.

pub(crate) struct BackendRecordHandle {
    registered_backend: RegisteredBackendHandleBinding,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    locator: BackendRecordLocator,
}

struct RegisteredBackendHandleBinding {
    registered: std::sync::Arc<RegisteredSecretBackend>,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
}

impl RegisteredBackendHandleBinding {
    fn from_handle(handle: &BackendInstanceHandle) -> Self {
        Self {
            registered: std::sync::Arc::clone(&handle.registered),
            device_instance_id: handle.registered.device_instance_id.clone(),
            device_store_instance_id: std::sync::Arc::clone(
                &handle.registered.device_store_instance_id,
            ),
        }
    }

    fn assert_same(
        &self,
        handle: &BackendInstanceHandle,
    ) -> Result<(), SecretInternalError> {
        let same_object = std::sync::Arc::ptr_eq(
            &self.registered,
            &handle.registered,
        );
        let same_instance = self.registered.instance.instance_id()
            == handle.registered.instance.instance_id()
            && self.registered.instance.generation()
                == handle.registered.instance.generation();
        let same_device = self.device_instance_id
            == handle.registered.device_instance_id;
        let same_store = std::sync::Arc::ptr_eq(
            &self.device_store_instance_id,
            &handle.registered.device_store_instance_id,
        );
        (same_object && same_instance && same_device && same_store)
            .then_some(())
            .ok_or_else(SecretInternalError::dependency_changed)
    }
}

impl BackendRecordHandle {
    // Private to crate::secret::backend; callers can receive but not forge it.
    fn from_backend_record(
        backend: &BackendInstanceHandle,
        device_instance_id: DeviceInstanceId,
        device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
        secret_ref: SecretRef,
        purpose: SecretPurpose,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        locator: BackendRecordLocator,
    ) -> Result<Self, SecretInternalError> {
        if device_instance_id != backend.registered.device_instance_id
            || !std::sync::Arc::ptr_eq(
                &device_store_instance_id,
                &backend.registered.device_store_instance_id,
            )
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(Self {
            registered_backend: RegisteredBackendHandleBinding::from_handle(backend),
            device_instance_id,
            device_store_instance_id,
            secret_ref,
            purpose,
            record_revision,
            instance_id: backend.registered.instance.instance_id().clone(),
            backend_generation: backend.registered.instance.generation(),
            store_revision,
            binding_set_cas,
            device_binding_generation,
            capability_revision,
            locator,
        })
    }

    // Read-only view usable only inside the crate::secret subtree. It permits
    // platform adapters to address the native record without a raw/public
    // locator getter or the ability to forge/change record identity.
    pub(in crate::secret) fn view(&self) -> BackendRecordView<'_> {
        BackendRecordView {
            device_instance_id: &self.device_instance_id,
            device_store_instance_id: &self.device_store_instance_id,
            secret_ref: &self.secret_ref,
            instance_id: &self.instance_id,
            backend_generation: self.backend_generation,
            store_revision: self.store_revision,
            locator: &self.locator,
        }
    }
}

pub(in crate::secret) struct BackendRecordView<'a> {
    device_instance_id: &'a DeviceInstanceId,
    device_store_instance_id: &'a DeviceSecretStoreInstanceId,
    secret_ref: &'a SecretRef,
    instance_id: &'a SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    store_revision: SecretStoreRevision,
    locator: &'a BackendRecordLocator,
}

impl BackendRecordView<'_> {
    pub(in crate::secret) fn secret_ref(&self) -> &SecretRef {
        self.secret_ref
    }

    pub(in crate::secret) fn instance_id(&self) -> &SecretBackendInstanceId {
        self.instance_id
    }

    pub(in crate::secret) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }

    pub(in crate::secret) fn store_revision(&self) -> SecretStoreRevision {
        self.store_revision
    }

}

struct BackendApplyOperationContext {
    operation_id: SecretOperationId,
    role: SecretApplyRole,
    projection_digest: SecretProjectionDigest,
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendRuntimeOperationContext {
    operation_id: SecretOperationId,
    consumer: FixedRuntimeConsumer,
    sink: FixedRuntimeSink,
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendActivationOperationContext {
    operation_id: SecretOperationId,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    projection_digest: SecretProjectionDigest,
    comparison_policy: LegacyActivationComparisonPolicy,
    slot: ActivationConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendRecoveryOperationContext {
    operation_id: SecretOperationId,
    recovery_id: SecretRecoveryId,
    recovery_kind: SecretRecoveryKind,
    recovery_cas: SecretRecoveryCas,
    slot: RecoveryConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendMigrationOperationContext {
    operation_id: SecretOperationId,
    report_id: SecretMigrationReportId,
    owner: SecretOwner,
    comparison_policy: LegacyActivationComparisonPolicy,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendStagedImportOperationContext {
    operation_id: SecretOperationId,
    authority: StagedImportAuthorityMatchReceipt,
    candidate_id: SecretCandidateId,
    projection_digest: SecretProjectionDigest,
    comparison_policy: LegacyActivationComparisonPolicy,
    slot: StagedImportConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

enum SecretNonApplyBackendOperation {
    CaptureVerify,
    Validate,
    CandidateDiscard {
        terminal_state: CandidateTerminalState,
        slot: CandidateDiscardConfirmationSlot,
    },
    DirectDelete,
    Revoke,
}

struct BackendNonApplyOperationContext {
    operation_id: SecretOperationId,
    operation: SecretNonApplyBackendOperation,
    terminal_error_context: SecretTerminalOperationContext,
    owner: SecretOwner,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

enum BackendOperationContext {
    Apply(BackendApplyOperationContext),
    Runtime(BackendRuntimeOperationContext),
    Activation(BackendActivationOperationContext),
    Recovery(BackendRecoveryOperationContext),
    Migration(BackendMigrationOperationContext),
    StagedImport(BackendStagedImportOperationContext),
    NonApply(BackendNonApplyOperationContext),
}

pub(in crate::secret) struct OpaqueApplyAdmissionClaim {
    context: BackendApplyOperationContext,
}
pub(in crate::secret) struct OpaqueOperationReadinessClaim {
    operation_id: SecretOperationId,
    _private: (),
}
pub(in crate::secret) struct OpaqueDurableJournalClaim {
    operation_id: SecretOperationId,
    _private: (),
}
pub(in crate::secret) struct OpaqueRuntimeAuthorityClaim {
    context: BackendRuntimeOperationContext,
}
pub(in crate::secret) struct OpaqueActivationAdmissionClaim {
    context: BackendActivationOperationContext,
}
pub(in crate::secret) struct OpaqueRecoveryReadinessClaim {
    context: BackendRecoveryOperationContext,
}
pub(in crate::secret) struct OpaqueMigrationReadinessClaim {
    context: BackendMigrationOperationContext,
}
pub(in crate::secret) struct OpaqueStagedAuthorityClaim {
    context: BackendStagedImportOperationContext,
}
pub(in crate::secret) struct OpaqueNonApplyReadinessClaim {
    context: BackendNonApplyOperationContext,
}

pub(crate) struct BrokeredBackendOperationContext(BackendOperationContext);

pub(crate) struct BackendOperationBroker {
    capture_intents: std::sync::Arc<dyn SecretCaptureIntentRegistry>,
    capabilities: std::sync::Arc<dyn SecretCapabilityRegistry>,
    pending: std::sync::Arc<dyn PendingSecretConfirmationRegistry>,
}

impl BackendOperationBroker {
    // Scanner-allowlisted only from the production and fixed test dependency
    // factories. No caller supplies a registry trait/object/parameter.
    pub(in crate::secret) fn from_production_store(
        opened_store: &OpenedDeviceLocalSecretStore,
    ) -> Result<std::sync::Arc<Self>, SecretInternalError> {
        let _ = opened_store;
        let (capture_intents, capabilities, pending) =
            todo!("construct fixed production registry implementations internally");
        Ok(std::sync::Arc::new(Self {
            capture_intents,
            capabilities,
            pending,
        }))
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(in crate::secret) fn from_fixture_mode(
        mode: test_support::SecretTestFixtureMode,
    ) -> std::sync::Arc<Self> {
        let _ = mode;
        let (capture_intents, capabilities, pending) =
            todo!("construct fixed fixture registry implementations internally");
        std::sync::Arc::new(Self {
            capture_intents,
            capabilities,
            pending,
        })
    }

    pub(in crate::secret) fn mint_capture_intent_from_atomic_snapshot(
        &self,
        registration: SecretCaptureIntentRegistration,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError> {
        self.capture_intents.mint_from_atomic_snapshot(registration)
    }

    pub(in crate::secret) fn claim_capture_intent_and_fresh_revalidate(
        &self,
        capture_intent_id: SecretCaptureIntentId,
        backend_instance_id: &SecretBackendInstanceId,
        now: &UtcTimestamp,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
        authority: &dyn DeviceLocalSecretAuthority,
        backends: &dyn SecretBackendRegistry,
    ) -> Result<ClaimedSecretCaptureIntent, SecretInternalError> {
        let claim = self.capture_intents.claim_once(
            capture_intent_id,
            backend_instance_id,
            now,
        )?;
        let revalidated = (|| {
            let current_legacy_source_coverage = legacy_sources
                .fresh_capture_coverage(&claim.registration.owner)?;
            claim.registration.legacy.coverage
                .assert_same_complete_coverage_as(
                    &current_legacy_source_coverage,
                )?;
            authority.revalidate_claimed_capture_intent(
                &claim,
                current_legacy_source_coverage,
                backends,
                now,
            )
        })();
        if let Err(error) = revalidated {
            self.capture_intents.terminalize(
                claim,
                PendingConfirmationTermination::Failed,
            )?;
            return Err(error);
        }
        Ok(claim)
    }

    pub(in crate::secret) fn consume_capture_intent(
        &self,
        claim: ClaimedSecretCaptureIntent,
    ) -> Result<(), SecretInternalError> {
        self.capture_intents.consume(claim)
    }

    pub(in crate::secret) fn terminalize_capture_intent(
        &self,
        claim: ClaimedSecretCaptureIntent,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        self.capture_intents.terminalize(claim, reason)
    }

    pub(in crate::secret) fn register_prepared_capability(
        &self,
        registration: PreparedCapabilityRegistration,
    ) -> Result<PreparedSecretCapability, SecretInternalError> {
        self.capabilities.register_prepared(registration)
    }

    fn claim_prepared_capability(
        &self,
        capability: &PreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<SecretCapabilityClaim, SecretInternalError> {
        self.capabilities.claim_prepared(capability, now)
    }

    pub(in crate::secret) fn mark_capability_consumed(
        &self,
        claim: SecretCapabilityClaim,
    ) -> Result<(), SecretInternalError> {
        self.capabilities.mark_consumed(claim)
    }

    pub(in crate::secret) fn invalidate_capability(
        &self,
        claim: SecretCapabilityClaim,
        code: SecretErrorCode,
    ) {
        self.capabilities.invalidate(claim, code)
    }

    fn terminalize_prepared_capability(
        &self,
        capability: &PreparedSecretCapability,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError> {
        self.capabilities.terminalize_prepared(capability, code)
    }

    pub(in crate::secret) fn register_pending_confirmation(
        &self,
        registration: PendingConfirmationRegistration,
    ) -> Result<RegisteredPendingConfirmation, SecretInternalError> {
        self.pending.register_pending(registration)
    }

    pub(in crate::secret) fn claim_pending_confirmation(
        &self,
        id: &PendingSecretConfirmationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError> {
        self.pending.claim_confirm(id, now)
    }

    pub(in crate::secret) fn mark_pending_confirmation_confirmed(
        &self,
        id: PendingSecretConfirmationId,
    ) -> Result<(), SecretInternalError> {
        self.pending.mark_confirmed(id)
    }

    pub(in crate::secret) fn terminalize_pending_confirmation(
        &self,
        id: PendingSecretConfirmationId,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        self.pending.terminate(id, reason)
    }

    pub(in crate::secret) fn for_apply(
        &self,
        admission: OpaqueApplyAdmissionClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume and equality-check opaque apply admission + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_runtime(
        &self,
        authority: OpaqueRuntimeAuthorityClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact runtime authority + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_activation(
        &self,
        admission: OpaqueActivationAdmissionClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact activation admission + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_recovery(
        &self,
        readiness: OpaqueRecoveryReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact recovery readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_migration(
        &self,
        readiness: OpaqueMigrationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact migration readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_staged_import(
        &self,
        authority: OpaqueStagedAuthorityClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume the authority-match receipt inside staged authority plus exact readiness and durable-journal claims")
    }

    pub(in crate::secret) fn for_non_apply(
        &self,
        readiness: OpaqueNonApplyReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact operation readiness + durable-journal claims")
    }
}

// Every context, claim and broker field is private and every type is
// non-Clone/non-serde. The scanner allows context literals only inside the
// operation broker, forbids re-export, From/Default and any literal in service
// or command modules, and rejects a direct BackendOperationContext parameter.
// The broker also derives and seals the exact terminal-error context; for a
// non-apply capture it must preserve new/replace/legacy intent or rotation,
// while validate/candidate-discard-record-delete/candidate-discard-record-
// missing/direct-delete/revoke have their fixed validation/discard/delete
// contexts. Candidate discard adds exactly those two operation-specific slots;
// a backend edge cannot choose one later.
// BackendOperationBroker is also the sole owner/caller of the capture-intent,
// capability and pending-confirmation registries. SecretServiceDeps and
// SecretService each retain exactly the same Arc<BackendOperationBroker>; they
// have no parallel registry Arc. Production and test factories construct one
// broker, then move that exact Arc into deps. list -> broker mint; begin ->
// broker atomic claim plus fresh authority/registered-handle revalidation; and
// every cancellation, expiry or later error -> broker terminalization. No
// claimed row can return to Ready and no private registry id crosses the broker.
// SecretService is the long-lived owner of the sole Arc. Private production/
// test assembly may move that same Arc through non-public SecretServiceDeps,
// but there is no caller/test setter, trait-injection parameter, registry
// parameter, broker extractor or AppStateBuilder override.

pub(crate) enum BackendPrepareResult {
    Ready(BackendAuthorizationHandle),
    ConfirmationRequired {
        requirement: BackendConfirmationRequirement,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) struct BackendConfirmationRequirement {
    pub backend_instance_id: SecretBackendInstanceId,
    pub backend_generation: SecretBackendGeneration,
    pub operation: SecretBackendOperation,
    pub confirmation: PhysicalConfirmation,
    pub device: SecretDeviceDisplay,
    pub timeout_seconds: ConfirmationTimeoutSeconds,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformOperationRequirement<'a> {
    scope: &'a BackendAuthorizationScope,
    operation: SecretBackendOperation,
    confirmation: PhysicalConfirmation,
}

pub(crate) enum PendingConfirmationTermination {
    UserCancelled,
    Expired,
    Discarded,
    Failed,
}

pub(crate) enum BackendProbeResult {
    Present {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Missing {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: BackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) enum PlatformProbeResult {
    Present {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Missing {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: PlatformBackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) struct BackendVerifyReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    receipt_id: BackendVerifyReceiptId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

impl BackendVerifyReceipt {
    pub(in crate::secret) fn receipt_id(&self) -> &BackendVerifyReceiptId {
        &self.receipt_id
    }
    pub(in crate::secret) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }
    pub(in crate::secret) fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.device_binding_generation
    }
}

pub(crate) struct BackendDeleteReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    disposition: BackendDeleteDisposition,
    completed_at: UtcTimestamp,
}

impl BackendDeleteReceipt {
    pub(in crate::secret) fn into_durable_outcome(
        self,
    ) -> (BackendDeleteDisposition, UtcTimestamp) {
        (self.disposition, self.completed_at)
    }
}

pub(in crate::secret) struct PlatformDeleteResult {
    disposition: BackendDeleteDisposition,
    completed_at: UtcTimestamp,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

wire_enum!(BackendDeleteDisposition { Deleted, AlreadyMissing });

mod platform_backend_sealed {
    pub(super) trait Sealed {}
    // Phase 2A: platform modules are unpublished. Seal only the in-memory store.
    pub(super) struct UnpublishedPlatformStore;
    impl Sealed for UnpublishedPlatformStore {}
    #[cfg(any(test, feature = "test-hooks"))]
    impl Sealed for crate::secret::testing::InMemorySecretStore {}
}

// This seam is visible only inside crate::secret. Platform modules never
// implement/re-export the public backend contract and never expose raw bytes.
pub(in crate::secret) trait PlatformBackendPort:
    platform_backend_sealed::Sealed + Send + Sync + 'static
{
    fn revocation_observation_capability(
        &self,
    ) -> BackendRevocationObservationCapability;

    fn capabilities_for_record(
        &self,
        record: BackendRecordView<'_>,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError>;

    fn capabilities_for_new_record(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError>;

    fn prepare(
        &self,
        record: BackendRecordView<'_>,
        requirement: PlatformOperationRequirement<'_>,
    ) -> Result<PlatformPrepareResult, SecretInternalError>;

    fn confirm(
        &self,
        pending_id: u128,
    ) -> Result<u128, SecretInternalError>;

    fn cancel(
        &self,
        pending_id: u128,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;

    // Raw borrow exists only at this private platform ABI and is invoked only
    // by the backend-owned sealed callback below; it is not a getter.
    fn write_and_readback_bytes(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
        material: &[u8],
    ) -> Result<PlatformWriteReadbackResult, SecretInternalError>;

    fn read_authorized_material_once(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
    ) -> Result<PlatformAuthorizedReadOutcome, SecretInternalError>;

    fn probe(
        &self,
        record: BackendRecordView<'_>,
    ) -> Result<PlatformProbeResult, SecretInternalError>;

    // The sole raw source/time observation entry. It is reachable only with
    // an authorization prepared for exact General::Revoke scope.
    fn observe_revocation_once(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
    ) -> Result<PlatformRevocationObservationResult, SecretInternalError>;

    fn delete_or_revoke(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
        mode: BackendDeleteMode,
    ) -> Result<PlatformDeleteResult, SecretInternalError>;
}

pub(in crate::secret) enum PlatformPrepareResult {
    Ready { authorization_id: u128 },
    ConfirmationRequired {
        pending_id: u128,
        requirement: PlatformConfirmationRequirement,
    },
}

pub(in crate::secret) enum PlatformAuthorizedReadOutcome {
    Material {
        material: SecretMaterial,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: PlatformBackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) struct PlatformRevocationObservationResult {
    observation: PlatformRevocationObservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

pub(in crate::secret) struct PlatformConfirmationRequirement {
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
}

// Private to crate::secret::backend. The platform implementation returns this
// only to PlatformWriteAndReadbackCallback in the same synchronous call stack.
// Its SecretMaterial is never returned by that callback or stored in a field.
pub(in crate::secret) struct PlatformWriteReadbackResult {
    readback: SecretMaterial,
    verify_receipt_id: BackendVerifyReceiptId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

pub(crate) struct BackendInstanceHandle {
    registered: std::sync::Arc<RegisteredSecretBackend>,
}

struct RegisteredSecretBackend {
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    instance: SecretBackendInstanceView,
    platform: std::sync::Arc<dyn PlatformBackendPort>,
}

struct PlatformWriteAndReadbackCallback<'a> {
    platform: &'a dyn PlatformBackendPort,
    record: BackendRecordView<'a>,
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    expected_backend_generation: SecretBackendGeneration,
    expected_device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    authorization_id: u128,
    terminal_error_context: SecretTerminalOperationContext,
}

impl backend_material_callback_sealed::Sealed
    for PlatformWriteAndReadbackCallback<'_>
{}

impl BackendMaterialWriteCallback for PlatformWriteAndReadbackCallback<'_> {
    type Receipt = Result<BackendVerifyReceipt, SecretInternalError>;

    fn write_once(self, material: &[u8]) -> Self::Receipt {
        let result = self.platform.write_and_readback_bytes(
            self.record,
            self.authorization_id,
            material,
        )?;
        // The original material borrow is still alive here. ConstantTimeEq is
        // executed before either material can be dropped; `result.readback`
        // zeroizes on every return path and cannot cross this callback.
        let failure = if !result.readback.ct_eq_slice(material) {
            Some(SecretSourceFreeErrorCode::VerifyFailed)
        } else if result.backend_generation != self.expected_backend_generation
            || result.device_binding_generation
                != self.expected_device_binding_generation
        {
            Some(SecretSourceFreeErrorCode::DependencyChanged)
        } else {
            None
        };
        if let Some(code) = failure {
            return Err(SecretInternalError::terminal_operation_failure(
                code,
                self.terminal_error_context,
            ));
        }
        Ok(BackendVerifyReceipt {
            registered_backend: self.registered_backend,
            device_store_instance_id: self.device_store_instance_id,
            secret_ref: self.secret_ref,
            record_revision: self.record_revision,
            store_revision: self.store_revision,
            binding_set_cas: self.binding_set_cas,
            backend_instance_id: self.backend_instance_id,
            receipt_id: result.verify_receipt_id,
            backend_generation: result.backend_generation,
            device_binding_generation: result.device_binding_generation,
            capability_revision: self.capability_revision,
        })
    }
}

// Concrete callback impls are intentionally absent from backend.rs. Each lane
// owner adds its impl in the adapter module that already owns the concrete
// callback and external receipt. The static scanner permits exactly the §7.1.1
// type/route/receipt triples and rejects any second marker or core-side path to
// `crate::services::configuration_apply` / `crate::commands::import_export`.

struct ScopedAuthorizedBackendRead {
    material: SecretMaterial,
    scope: BackendAuthorizationScope,
}

pub(in crate::secret) enum BackendAuthorizedReadOutcome<T> {
    Ready(T),
    Revoked(BackendRevocationHint),
}

pub(crate) struct AuthorizedApplyRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedActivationRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedRecoveryRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedMigrationRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedStagedImportRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedProxyRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedUsageRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedCodingPlanRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedModelFetchRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedValidationRead(ScopedAuthorizedBackendRead);

pub(crate) enum AuthorizedRuntimeRead {
    Proxy(AuthorizedProxyRead),
    Usage(AuthorizedUsageRead),
    CodingPlan(AuthorizedCodingPlanRead),
    ModelFetch(AuthorizedModelFetchRead),
}

pub(crate) struct CandidateReadVerifiedReceipt {
    _private: (),
}

impl AuthorizedApplyRead {
    pub(crate) fn write_apply_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ApplyMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedActivationRead {
    pub(crate) fn compare_candidate_equality_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ActivationEqualityMaterialAdapter,
    {
        todo!("require Activation scope + CandidateEquality before exposure");
        self.0.material.write_to_sealed_callback(callback)
    }

    pub(crate) fn verify_explicit_replacement_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require Activation scope + ExplicitReplacement, then consume/drop material")
    }
}

impl AuthorizedRecoveryRead {
    pub(crate) fn compare_recovery_source_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: RecoveryEqualityMaterialAdapter,
    {
        todo!("require Recovery active-record equality slot and exact recovery kind/CAS");
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedMigrationRead {
    pub(crate) fn compare_inventory_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: MigrationEqualityMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedStagedImportRead {
    pub(crate) fn compare_candidate_equality_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: StagedImportEqualityMaterialAdapter,
    {
        todo!("require StagedImport scope + CandidateEquality before exposure");
        self.0.material.write_to_sealed_callback(callback)
    }

    pub(crate) fn verify_explicit_replacement_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require StagedImport scope + ExplicitReplacement, consume/drop material")
    }
}

impl AuthorizedProxyRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ProxyMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedUsageRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: UsageMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedCodingPlanRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: CodingPlanMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedModelFetchRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ModelFetchMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedValidationRead {
    pub(crate) fn validate_present_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require exact General::Validate scope, consume/drop material")
    }
}

wire_enum!(BackendDeleteMode { Delete, Revoke });

pub(crate) struct AuthorizedBackendDelete {
    backend: BackendInstanceHandle,
    record: BackendRecordHandle,
    authorization: ConsumedBackendAuthorization,
    mode: BackendDeleteMode,
}

pub(crate) struct BackendMissingReadbackReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    delete_applied_cas: BackendDeleteAppliedCas,
    checked_at: UtcTimestamp,
}

pub(crate) struct AuthorizedBackendMissingReadback {
    backend: BackendInstanceHandle,
    record: BackendRecordHandle,
    authorization: ConsumedBackendAuthorization,
    expected_delete_applied_cas: BackendDeleteAppliedCas,
}

impl AuthorizedBackendMissingReadback {
    pub(crate) fn readback_missing_once(
        self,
        delete_applied_cas: &BackendDeleteAppliedCas,
        now: UtcTimestamp,
    ) -> Result<BackendMissingReadbackReceipt, SecretInternalError> {
        self.authorization
            .scope
            .assert_registered_handle(&self.backend)?;
        self.backend.assert_record_identity(&self.record)?;
        if delete_applied_cas != &self.expected_delete_applied_cas {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.authorization.scope.into_terminal_error_context(),
            ));
        }
        match self.backend.registered.platform.probe(self.record.view())? {
            PlatformProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            } if backend_generation == self.record.backend_generation
                && device_binding_generation
                    == self.record.device_binding_generation => Ok(BackendMissingReadbackReceipt {
                registered_backend:
                    RegisteredBackendHandleBinding::from_handle(&self.backend),
                device_store_instance_id:
                    self.record.device_store_instance_id.clone(),
                secret_ref: self.record.secret_ref.clone(),
                record_revision: self.record.record_revision,
                store_revision: self.record.store_revision,
                binding_set_cas: self.record.binding_set_cas.clone(),
                backend_instance_id: self.record.instance_id.clone(),
                backend_generation,
                device_binding_generation,
                capability_revision: self.record.capability_revision,
                delete_applied_cas: delete_applied_cas.clone(),
                checked_at: now,
            }),
            PlatformProbeResult::Present { .. }
            | PlatformProbeResult::Revoked { .. }
            | PlatformProbeResult::Missing { .. } => Err(
                SecretInternalError::terminal_operation_failure(
                    SecretSourceFreeErrorCode::DependencyChanged,
                    self.authorization.scope.into_terminal_error_context(),
                ),
            ),
        }
    }
}

impl AuthorizedBackendDelete {
    pub(crate) fn delete_once(self) -> Result<BackendDeleteReceipt, SecretInternalError> {
        self.authorization
            .scope
            .assert_registered_handle(&self.backend)?;
        self.backend.assert_record_identity(&self.record)?;
        let raw = self.backend.registered.platform.delete_or_revoke(
            self.record.view(),
            self.authorization.authorization_id,
            self.mode,
        )?;
        if raw.backend_generation != self.record.backend_generation
            || raw.device_binding_generation
                != self.record.device_binding_generation
        {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.authorization.scope.into_terminal_error_context(),
            ));
        }
        Ok(BackendDeleteReceipt {
            registered_backend:
                RegisteredBackendHandleBinding::from_handle(&self.backend),
            device_store_instance_id:
                self.record.device_store_instance_id.clone(),
            secret_ref: self.record.secret_ref.clone(),
            record_revision: self.record.record_revision,
            store_revision: self.record.store_revision,
            binding_set_cas: self.record.binding_set_cas.clone(),
            backend_instance_id: self.record.instance_id.clone(),
            backend_generation: raw.backend_generation,
            device_binding_generation: raw.device_binding_generation,
            capability_revision: self.record.capability_revision,
            disposition: raw.disposition,
            completed_at: raw.completed_at,
        })
    }
}

impl BackendInstanceHandle {
    pub(crate) fn instance(&self) -> &SecretBackendInstanceView {
        &self.registered.instance
    }

    fn assert_record_identity(
        &self,
        record: &BackendRecordHandle,
    ) -> Result<(), SecretInternalError> {
        (&record.instance_id == self.registered.instance.instance_id()
            && record.backend_generation
                == self.registered.instance.generation()
            && record.device_instance_id
                == self.registered.device_instance_id
            && std::sync::Arc::ptr_eq(
                &record.device_store_instance_id,
                &self.registered.device_store_instance_id,
            )
            && record.registered_backend.assert_same(self).is_ok())
            .then_some(())
            .ok_or_else(SecretInternalError::dependency_changed)
    }

    // Backend wrapper is the only producer of the validated capability type.
    pub(crate) fn capabilities_for_record(
        &self,
        record: &BackendRecordHandle,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        self.assert_record_identity(record)?;
        let capabilities = self
            .registered
            .platform
            .capabilities_for_record(record.view(), purpose)?;
        let (instance_id, generation) = capabilities.backend_identity();
        if instance_id != self.registered.instance.instance_id()
            || generation != self.registered.instance.generation()
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(capabilities)
    }

    pub(crate) fn capabilities_for_new_record(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        let capabilities = self
            .registered
            .platform
            .capabilities_for_new_record(owner, purpose)?;
        let (instance_id, generation) = capabilities.backend_identity();
        if instance_id != self.registered.instance.instance_id()
            || generation != self.registered.instance.generation()
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(capabilities)
    }

    pub(crate) fn prepare_brokered_operation(
        &self,
        record: &BackendRecordHandle,
        context: BrokeredBackendOperationContext,
    ) -> Result<BackendPrepareResult, SecretInternalError> {
        self.assert_record_identity(record)?;
        let scope = BackendAuthorizationScope::mint_from_context(self, record, context)?;
        scope.assert_registered_handle(self)?;
        let platform_requirement = scope.platform_requirement()?;
        let operation = platform_requirement.operation;
        let confirmation = platform_requirement.confirmation;
        match self
            .registered
            .platform
            .prepare(record.view(), platform_requirement)?
        {
            PlatformPrepareResult::Ready { authorization_id } => {
                Ok(BackendPrepareResult::Ready(
                    BackendAuthorizationHandle::mint(authorization_id, scope),
                ))
            }
            PlatformPrepareResult::ConfirmationRequired {
                pending_id,
                requirement,
            } => {
                scope.validate_confirmation_requirement(
                    self,
                    operation,
                    confirmation,
                    &requirement,
                )?;
                let public_requirement = BackendConfirmationRequirement {
                    backend_instance_id: scope.0.backend_instance_id.clone(),
                    backend_generation: scope.0.backend_generation,
                    operation,
                    confirmation,
                    device: requirement.device.clone(),
                    timeout_seconds: requirement.timeout_seconds,
                    prompt_key: requirement.prompt_key,
                    expires_at: scope.0.expires_at.clone(),
                };
                let pending_requirement = BackendPendingRequirementIdentity {
                    backend_instance_id: scope.0.backend_instance_id.clone(),
                    backend_generation: scope.0.backend_generation,
                    operation,
                    confirmation,
                    device: requirement.device,
                    timeout_seconds: requirement.timeout_seconds,
                    prompt_key: requirement.prompt_key,
                    expires_at: scope.0.expires_at.clone(),
                };
                Ok(BackendPrepareResult::ConfirmationRequired {
                    requirement: public_requirement,
                    pending: BackendPendingConfirmation::mint(
                        pending_id,
                        scope,
                        pending_requirement,
                    ),
                })
            }
        }
    }

    pub(crate) fn confirm_operation(
        &self,
        pending: BackendPendingConfirmation,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizationHandle, SecretInternalError> {
        pending.scope.assert_registered_handle(self)?;
        pending.scope.validate_pending_requirement(
            self,
            &pending.requirement,
            now,
            None,
        )?;
        let authorization_id = self.registered.platform.confirm(pending.pending_id)?;
        Ok(BackendAuthorizationHandle::mint(
            authorization_id,
            pending.scope,
        ))
    }

    pub(crate) fn cancel_operation(
        &self,
        pending: BackendPendingConfirmation,
        reason: PendingConfirmationTermination,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError> {
        pending.scope.assert_registered_handle(self)?;
        pending.scope.validate_pending_requirement(
            self,
            &pending.requirement,
            now,
            Some(&reason),
        )?;
        self.registered.platform.cancel(pending.pending_id, reason)
    }

    // Capture-only exact operation: consumes authorization, writes and reads
    // back inside the backend wrapper, ConstantTimeEq compares, zeroizes both
    // materials and returns only a fixed receipt.
    pub(in crate::secret) fn write_and_verify_once(
        &self,
        record: &BackendRecordHandle,
        material: SecretMaterial,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendVerifyReceipt, SecretInternalError> {
        self.assert_record_identity(record)?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        let authorization_id = consumed.authorization_id;
        let terminal_error_context = consumed.scope.into_terminal_error_context();
        material.write_to_sealed_callback(PlatformWriteAndReadbackCallback {
            platform: self.registered.platform.as_ref(),
            record: record.view(),
            registered_backend: RegisteredBackendHandleBinding::from_handle(self),
            device_store_instance_id: record.device_store_instance_id.clone(),
            secret_ref: record.secret_ref.clone(),
            record_revision: record.record_revision,
            store_revision: record.store_revision,
            binding_set_cas: record.binding_set_cas.clone(),
            backend_instance_id: record.instance_id.clone(),
            expected_backend_generation: record.backend_generation,
            expected_device_binding_generation:
                record.device_binding_generation,
            capability_revision: record.capability_revision,
            authorization_id,
            terminal_error_context,
        })
    }

    fn read_scoped_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<ScopedAuthorizedBackendRead>, SecretInternalError> {
        self.assert_record_identity(record)?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        match self.registered.platform.read_authorized_material_once(
            record.view(),
            consumed.authorization_id,
        )? {
            PlatformAuthorizedReadOutcome::Material {
                material,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => {
                Ok(BackendAuthorizedReadOutcome::Ready(
                    ScopedAuthorizedBackendRead {
                        material,
                        scope: consumed.scope,
                    },
                ))
            }
            PlatformAuthorizedReadOutcome::Revoked {
                hint: _,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation =>
                Ok(BackendAuthorizedReadOutcome::Revoked(
                    BackendRevocationHint {
                        registered_backend:
                            RegisteredBackendHandleBinding::from_handle(self),
                        device_store_instance_id:
                            record.device_store_instance_id.clone(),
                        _private: (),
                    },
                )),
            PlatformAuthorizedReadOutcome::Material { .. }
            | PlatformAuthorizedReadOutcome::Revoked { .. } => {
                Err(SecretInternalError::terminal_operation_failure(
                    SecretSourceFreeErrorCode::DependencyChanged,
                    consumed.scope.into_terminal_error_context(),
                ))
            }
        }
    }

    pub(in crate::secret) fn authorize_apply_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedApplyRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Apply)?;
        todo!("read_scoped_once then require complete Apply scope")
    }

    pub(in crate::secret) fn authorize_activation_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedActivationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Activation)?;
        todo!("read_scoped_once then require complete Activation candidate-read scope")
    }

    pub(in crate::secret) fn authorize_recovery_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedRecoveryRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Recovery)?;
        todo!("read_scoped_once then require exact Recovery kind/CAS/read slot")
    }

    pub(in crate::secret) fn authorize_migration_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedMigrationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Migration)?;
        todo!("read_scoped_once then require complete Migration scope")
    }

    pub(in crate::secret) fn authorize_staged_import_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedStagedImportRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::StagedImport)?;
        todo!("read_scoped_once then require complete StagedImport scope")
    }

    pub(in crate::secret) fn authorize_proxy_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedProxyRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Proxy)?;
        todo!("read_scoped_once then require Runtime ProxyRequest/processMemory")
    }

    pub(in crate::secret) fn authorize_usage_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedUsageRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Usage)?;
        todo!("read_scoped_once then require Runtime UsageProbe/processMemory")
    }

    pub(in crate::secret) fn authorize_coding_plan_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedCodingPlanRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::CodingPlan)?;
        todo!("read_scoped_once then require CodingPlanUsageProbe/processMemory")
    }

    pub(in crate::secret) fn authorize_model_fetch_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedModelFetchRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::ModelFetch)?;
        todo!("read_scoped_once then require Runtime ModelFetch/processMemory")
    }

    pub(in crate::secret) fn authorize_validation_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedValidationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Validation)?;
        todo!("read_scoped_once then require General Validate scope")
    }

    pub(in crate::secret) fn authorize_delete_once(
        self,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
        mode: BackendDeleteMode,
    ) -> Result<AuthorizedBackendDelete, SecretInternalError> {
        self.assert_record_identity(&record)?;
        let consumed = authorization.consume(&self, &record, operation_id, now)?;
        consumed.scope.require_delete_mode(mode)?;
        Ok(AuthorizedBackendDelete {
            backend: self,
            record,
            authorization: consumed,
            mode,
        })
    }

    pub(in crate::secret) fn authorize_missing_readback_once(
        self,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        expected_delete_applied_cas: BackendDeleteAppliedCas,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedBackendMissingReadback, SecretInternalError> {
        authorization.scope.require_missing_readback()?;
        self.assert_record_identity(&record)?;
        let consumed = authorization.consume(&self, &record, operation_id, now)?;
        Ok(AuthorizedBackendMissingReadback {
            backend: self,
            record,
            authorization: consumed,
            expected_delete_applied_cas,
        })
    }

    pub(crate) fn probe(
        &self,
        record: &BackendRecordHandle,
    ) -> Result<BackendProbeResult, SecretInternalError> {
        self.assert_record_identity(record)?;
        match self.registered.platform.probe(record.view())? {
            PlatformProbeResult::Present {
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => Ok(BackendProbeResult::Present {
                backend_generation,
                device_binding_generation,
            }),
            PlatformProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => Ok(BackendProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            }),
            PlatformProbeResult::Revoked {
                hint: _,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => {
                Ok(BackendProbeResult::Revoked {
                    hint: BackendRevocationHint {
                        registered_backend:
                            RegisteredBackendHandleBinding::from_handle(self),
                        device_store_instance_id:
                            record.device_store_instance_id.clone(),
                        _private: (),
                    },
                    backend_generation,
                    device_binding_generation,
                })
            }
            PlatformProbeResult::Present { .. }
            | PlatformProbeResult::Missing { .. }
            | PlatformProbeResult::Revoked { .. } => {
                Err(SecretInternalError::dependency_changed())
            }
        }
    }

    pub(in crate::secret) fn observe_revocation_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendRevocationObservation, SecretInternalError> {
        self.assert_record_identity(record)?;
        authorization.scope.require_revoke_observation()?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        let capabilities = self.capabilities_for_record(record, record.purpose)?;
        let raw = self.registered.platform.observe_revocation_once(
            record.view(),
            consumed.authorization_id,
        )?;
        BackendRevocationObservation::checked_from_platform(
            self,
            record,
            &capabilities,
            consumed,
            raw,
        )
    }
}

pub(crate) trait SecretBackendRegistry: Send + Sync {
    // Exact tuple lookup only. There is no iterator/fallback API.
    fn get_exact(
        &self,
        instance_id: &SecretBackendInstanceId,
        generation: SecretBackendGeneration,
    ) -> Result<BackendInstanceHandle, SecretInternalError>;

    fn selectable_instances(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<Vec<SecretBackendOption>, SecretInternalError>;
}

// These two types live in crate::secret::migration. The constructor is private
// there; backend.rs alone implements the sealed callback trait for it.
pub(in crate::secret) struct LegacyInventoryCompareCallback {
    expected: SecretMaterial,
}

pub(in crate::secret) struct LegacyInventoryComparisonReceipt {
    equal: bool,
}

impl LegacyInventoryCompareCallback {
    fn new(expected: SecretMaterial) -> Self {
        Self { expected }
    }

    pub(in crate::secret) fn write_material_once(
        self,
        actual: &[u8],
    ) -> LegacyInventoryComparisonReceipt {
        LegacyInventoryComparisonReceipt {
            equal: self.expected.ct_eq_slice(actual),
        }
    }
}

impl backend_material_callback_sealed::Sealed for ActivationCandidateEqualityCompareCallback<'_> {}
impl BackendMaterialWriteCallback for ActivationCandidateEqualityCompareCallback<'_> {
    type Receipt = Result<ProviderLegacySourceMatchReceipt, SecretInternalError>;
    fn write_once(self, material: &[u8]) -> Self::Receipt {
        self.write_material_once(material)
    }
}
impl ActivationEqualityMaterialAdapter for ActivationCandidateEqualityCompareCallback<'_> {}

impl backend_material_callback_sealed::Sealed for RecoveryCandidateEqualityScrubCallback<'_> {}
impl BackendMaterialWriteCallback for RecoveryCandidateEqualityScrubCallback<'_> {
    type Receipt = Result<ProviderScrubReadbackReceipt, SecretInternalError>;
    fn write_once(self, material: &[u8]) -> Self::Receipt {
        self.write_material_once(material)
    }
}
impl RecoveryEqualityMaterialAdapter for RecoveryCandidateEqualityScrubCallback<'_> {}

impl backend_material_callback_sealed::Sealed for StagedImportCandidateEqualityCompareCallback<'_> {}
impl BackendMaterialWriteCallback for StagedImportCandidateEqualityCompareCallback<'_> {
    type Receipt = Result<StagedImportSourceValidationReceipt, SecretInternalError>;
    fn write_once(self, material: &[u8]) -> Self::Receipt {
        self.write_material_once(material)
    }
}
impl StagedImportEqualityMaterialAdapter for StagedImportCandidateEqualityCompareCallback<'_> {}
