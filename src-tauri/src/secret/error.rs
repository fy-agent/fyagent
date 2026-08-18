
pub(crate) struct SecretInternalError {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    effect: SecretEffect,
    condition: SecretActionCondition,
    lock_source: Option<SecretLockSource>,
    revocation_source: Option<SecretRevocationSource>,
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    recovery: Option<SecretRecoveryPointer>,
}

struct SecretErrorSources {
    lock_source: Option<SecretLockSource>,
    revocation_source: Option<SecretRevocationSource>,
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    recovery: Option<SecretRecoveryPointer>,
}

#[derive(Debug)]
struct SecretErrorFactoryViolation;

// Closed input for source-free terminal failures. Codes that require a lock
// source, revocation source, backend-unavailable reason, or recovery pointer
// are intentionally unrepresentable here and have dedicated factories below.
pub(in crate::secret) enum SecretSourceFreeErrorCode {
    RequestInvalid,
    RefInvalid,
    OwnerKindUnsupported,
    OwnerNamespaceUnsupported,
    OwnerNotFound,
    OwnerConflict,
    OperationBusy,
    UnsupportedPurpose,
    ConsumerUnsupported,
    InputCancelled,
    InputInvalid,
    CandidateNotFound,
    CandidateExpired,
    CandidateConsumed,
    ChangePlanRequired,
    ChangePlanInvalid,
    ChangePlanStale,
    MigrationRequired,
    LegacySourceInvalid,
    LegacyConflict,
    LegacyComparisonPending,
    MigrationFailed,
    Missing,
    PermissionDenied,
    Stale,
    ConfirmationRequired,
    ConfirmationCancelled,
    ConfirmationExpired,
    ConfirmationReplayed,
    DeviceMismatch,
    WriteFailed,
    ReadFailed,
    DeleteFailed,
    VerifyFailed,
    ProjectionForbidden,
    DependencyChanged,
    RecordChanged,
    BackendChanged,
    CapabilityExpired,
    CapabilityConsumed,
    RecoveryNotFound,
    RecoveryChanged,
    Internal,
}

impl SecretSourceFreeErrorCode {
    fn stable_code(self) -> SecretErrorCode {
        match self {
            Self::RequestInvalid => SecretErrorCode::SecretRequestInvalid,
            Self::RefInvalid => SecretErrorCode::SecretRefInvalid,
            Self::OwnerKindUnsupported => SecretErrorCode::SecretOwnerKindUnsupported,
            Self::OwnerNamespaceUnsupported => SecretErrorCode::SecretOwnerNamespaceUnsupported,
            Self::OwnerNotFound => SecretErrorCode::SecretOwnerNotFound,
            Self::OwnerConflict => SecretErrorCode::SecretOwnerConflict,
            Self::OperationBusy => SecretErrorCode::SecretOperationBusy,
            Self::UnsupportedPurpose => SecretErrorCode::SecretUnsupportedPurpose,
            Self::ConsumerUnsupported => SecretErrorCode::SecretConsumerUnsupported,
            Self::InputCancelled => SecretErrorCode::SecretInputCancelled,
            Self::InputInvalid => SecretErrorCode::SecretInputInvalid,
            Self::CandidateNotFound => SecretErrorCode::SecretCandidateNotFound,
            Self::CandidateExpired => SecretErrorCode::SecretCandidateExpired,
            Self::CandidateConsumed => SecretErrorCode::SecretCandidateConsumed,
            Self::ChangePlanRequired => SecretErrorCode::SecretChangePlanRequired,
            Self::ChangePlanInvalid => SecretErrorCode::SecretChangePlanInvalid,
            Self::ChangePlanStale => SecretErrorCode::SecretChangePlanStale,
            Self::MigrationRequired => SecretErrorCode::SecretMigrationRequired,
            Self::LegacySourceInvalid => SecretErrorCode::SecretLegacySourceInvalid,
            Self::LegacyConflict => SecretErrorCode::SecretLegacyConflict,
            Self::LegacyComparisonPending => SecretErrorCode::SecretLegacyComparisonPending,
            Self::MigrationFailed => SecretErrorCode::SecretMigrationFailed,
            Self::Missing => SecretErrorCode::SecretMissing,
            Self::PermissionDenied => SecretErrorCode::SecretPermissionDenied,
            Self::Stale => SecretErrorCode::SecretStale,
            Self::ConfirmationRequired => SecretErrorCode::SecretConfirmationRequired,
            Self::ConfirmationCancelled => SecretErrorCode::SecretConfirmationCancelled,
            Self::ConfirmationExpired => SecretErrorCode::SecretConfirmationExpired,
            Self::ConfirmationReplayed => SecretErrorCode::SecretConfirmationReplayed,
            Self::DeviceMismatch => SecretErrorCode::SecretDeviceMismatch,
            Self::WriteFailed => SecretErrorCode::SecretWriteFailed,
            Self::ReadFailed => SecretErrorCode::SecretReadFailed,
            Self::DeleteFailed => SecretErrorCode::SecretDeleteFailed,
            Self::VerifyFailed => SecretErrorCode::SecretVerifyFailed,
            Self::ProjectionForbidden => SecretErrorCode::SecretProjectionForbidden,
            Self::DependencyChanged => SecretErrorCode::SecretDependencyChanged,
            Self::RecordChanged => SecretErrorCode::SecretRecordChanged,
            Self::BackendChanged => SecretErrorCode::SecretBackendChanged,
            Self::CapabilityExpired => SecretErrorCode::SecretCapabilityExpired,
            Self::CapabilityConsumed => SecretErrorCode::SecretCapabilityConsumed,
            Self::RecoveryNotFound => SecretErrorCode::SecretRecoveryNotFound,
            Self::RecoveryChanged => SecretErrorCode::SecretRecoveryChanged,
            Self::Internal => SecretErrorCode::SecretInternal,
        }
    }
}

impl SecretErrorSources {
    fn none() -> Self {
        Self {
            lock_source: None,
            revocation_source: None,
            backend_unavailable_reason: None,
            recovery: None,
        }
    }

    fn locked(source: SecretLockSource) -> Self {
        Self { lock_source: Some(source), ..Self::none() }
    }

    fn revoked(source: SecretRevocationSource) -> Self {
        Self { revocation_source: Some(source), ..Self::none() }
    }

    fn backend_unavailable(reason: SecretBackendUnavailableReason) -> Self {
        Self { backend_unavailable_reason: Some(reason), ..Self::none() }
    }

    fn recovery(pointer: SecretRecoveryPointer) -> Self {
        Self { recovery: Some(pointer), ..Self::none() }
    }
}

impl SecretTerminalOperationContext {
    fn fresh_action_and_condition(&self) -> (SecretUserAction, SecretActionCondition) {
        match self {
            Self::Summary => (
                SecretUserAction::RefreshSummary,
                SecretActionCondition::General,
            ),
            Self::Capture(BeginCaptureIntent::NewBinding) => (
                SecretUserAction::RetryCapture,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Capture(BeginCaptureIntent::ReplaceBinding) => (
                SecretUserAction::CaptureReplacement,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Capture(BeginCaptureIntent::LegacyReconcile) => (
                SecretUserAction::ResolveLegacyConflict,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Rotation => (SecretUserAction::RetryRotation, SecretActionCondition::RotationFreshOperation),
            Self::CandidateDiscard => (SecretUserAction::DiscardCandidate, SecretActionCondition::CandidateDiscardFreshOperation),
            Self::CandidateTerminalCleanupPending => (SecretUserAction::DiscardCandidate, SecretActionCondition::CandidateTerminalCleanupPending),
            Self::Delete => (SecretUserAction::RefreshDeleteImpact, SecretActionCondition::DeleteReadiness),
            Self::Recovery => (SecretUserAction::RefreshRecoveryImpact, SecretActionCondition::RecoveryReadiness),
            Self::ApplyOrActivation => (SecretUserAction::ReopenChangePlan, SecretActionCondition::ApplyOrActivationPlan),
            Self::StagedImport => (SecretUserAction::ResumeStagedImportCutover, SecretActionCondition::StagedImportResume),
            Self::Validation => (SecretUserAction::RefreshSummary, SecretActionCondition::ValidationFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::ProxyRequest) => (SecretUserAction::RetryProxyRequest, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::UsageProbe) => (SecretUserAction::RetryUsageProbe, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::CodingPlanUsageProbe) => (SecretUserAction::RetryCodingPlanUsageProbe, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::ModelFetch) => (SecretUserAction::RetryModelFetch, SecretActionCondition::RuntimeFreshOperation),
        }
    }
}

impl SecretInternalError {
    // Sole constructor. The code match is exhaustive with no wildcard; adding
    // a stable code cannot compile until retry/action/effect handling is added.
    fn checked(
        code: SecretErrorCode,
        context: SecretTerminalOperationContext,
        sources: SecretErrorSources,
    ) -> Result<Self, SecretErrorFactoryViolation> {
        let mut retryable = match code {
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretMissing
            | SecretErrorCode::SecretRevoked
            | SecretErrorCode::SecretDeviceMismatch
            | SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretProjectionForbidden => false,
            SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacyComparisonPending
            | SecretErrorCode::SecretMigrationFailed
            | SecretErrorCode::SecretLocked
            | SecretErrorCode::SecretPermissionDenied
            | SecretErrorCode::SecretBackendUnavailable
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretConfirmationRequired
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretRecoveryChanged
            | SecretErrorCode::SecretOperationRecoveryRequired
            | SecretErrorCode::SecretInternal => true,
        };
        let capture_selection = matches!(
            &context,
            SecretTerminalOperationContext::Capture(_)
                | SecretTerminalOperationContext::Rotation
        );
        let backend_selection_action = match &context {
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding) => SecretUserAction::ChooseBackend,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::ReplaceBinding) => SecretUserAction::CaptureReplacement,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::LegacyReconcile) => SecretUserAction::ResolveLegacyConflict,
            SecretTerminalOperationContext::Rotation => SecretUserAction::RetryRotation,
            SecretTerminalOperationContext::Summary
            | SecretTerminalOperationContext::CandidateDiscard
            | SecretTerminalOperationContext::CandidateTerminalCleanupPending
            | SecretTerminalOperationContext::Delete
            | SecretTerminalOperationContext::Recovery
            | SecretTerminalOperationContext::ApplyOrActivation
            | SecretTerminalOperationContext::StagedImport
            | SecretTerminalOperationContext::Validation
            | SecretTerminalOperationContext::Runtime(_) => SecretUserAction::RefreshSummary,
        };
        let (fresh_action, mut condition) = context.fresh_action_and_condition();
        let action = match code {
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported => SecretUserAction::None,
            SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretLegacyComparisonPending => SecretUserAction::RefreshSummary,
            SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretProjectionForbidden => SecretUserAction::ReopenChangePlan,
            SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretMigrationFailed => SecretUserAction::ResolveLegacyConflict,
            SecretErrorCode::SecretMissing => SecretUserAction::CaptureReplacement,
            SecretErrorCode::SecretRevoked => match sources.revocation_source {
                Some(SecretRevocationSource::UserDelete) => SecretUserAction::CaptureReplacement,
                Some(SecretRevocationSource::SupersededByRotation) => SecretUserAction::None,
                Some(SecretRevocationSource::CentralBackend) => SecretUserAction::ContactAdministrator,
                Some(SecretRevocationSource::DeviceAdministration) => SecretUserAction::OpenBackendSettings,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretLocked => match sources.lock_source {
                Some(SecretLockSource::FyAgentPolicy) => SecretUserAction::UnlockFyAgent,
                Some(SecretLockSource::Backend) => SecretUserAction::UnlockBackend,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretPermissionDenied => SecretUserAction::RequestPermission,
            SecretErrorCode::SecretBackendUnavailable => match sources.backend_unavailable_reason {
                Some(SecretBackendUnavailableReason::HardwareUnregistered) if capture_selection => backend_selection_action,
                Some(SecretBackendUnavailableReason::HardwareUnregistered) => SecretUserAction::OpenBackendSettings,
                Some(SecretBackendUnavailableReason::HardwareDisconnected) => SecretUserAction::ReconnectDevice,
                Some(SecretBackendUnavailableReason::OsStoreUnavailable) => SecretUserAction::OpenBackendSettings,
                Some(SecretBackendUnavailableReason::CentralServiceUnavailable) => SecretUserAction::ContactAdministrator,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretConfirmationRequired => SecretUserAction::ConfirmDevice,
            SecretErrorCode::SecretDeviceMismatch => SecretUserAction::ReconnectDevice,
            SecretErrorCode::SecretBackendChanged if capture_selection => backend_selection_action,
            SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretRecoveryChanged => SecretUserAction::RefreshRecoveryImpact,
            SecretErrorCode::SecretOperationRecoveryRequired => {
                if sources.recovery.is_some() { SecretUserAction::CompleteRecovery } else { fresh_action }
            }
            SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretInternal => fresh_action,
        };
        let effect = if code == SecretErrorCode::SecretOperationRecoveryRequired
            && sources.recovery.is_some()
        {
            SecretEffect::CleanupPending
        } else {
            SecretEffect::None
        };
        if code == SecretErrorCode::SecretBackendUnavailable {
            retryable = match sources.backend_unavailable_reason {
                Some(SecretBackendUnavailableReason::HardwareUnregistered) => capture_selection,
                Some(SecretBackendUnavailableReason::HardwareDisconnected)
                | Some(SecretBackendUnavailableReason::OsStoreUnavailable) => true,
                Some(SecretBackendUnavailableReason::CentralServiceUnavailable) => false,
                None => return Err(SecretErrorFactoryViolation),
            };
        }
        if capture_selection
            && (code == SecretErrorCode::SecretBackendChanged
                || (code == SecretErrorCode::SecretBackendUnavailable
                    && sources.backend_unavailable_reason
                        == Some(SecretBackendUnavailableReason::HardwareUnregistered)))
        {
            condition = SecretActionCondition::CaptureBackendSelection;
        }
        let source_shape_valid = match code {
            SecretErrorCode::SecretLocked => sources.lock_source.is_some()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretRevoked => sources.lock_source.is_none()
                && sources.revocation_source.is_some()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretBackendUnavailable => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_some()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretOperationRecoveryRequired => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && match (&context, &sources.recovery) {
                    (
                        SecretTerminalOperationContext::CandidateTerminalCleanupPending,
                        None,
                    ) => true,
                    (
                        SecretTerminalOperationContext::CandidateTerminalCleanupPending,
                        Some(_),
                    ) => false,
                    (_, Some(_)) => true,
                    (_, None) => false,
                },
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretLegacyComparisonPending
            | SecretErrorCode::SecretMigrationFailed
            | SecretErrorCode::SecretMissing
            | SecretErrorCode::SecretPermissionDenied
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretConfirmationRequired
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretDeviceMismatch
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretProjectionForbidden
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretRecoveryChanged
            | SecretErrorCode::SecretInternal => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
        };
        if !source_shape_valid { return Err(SecretErrorFactoryViolation); }
        Ok(Self {
            code,
            retryable,
            action,
            effect,
            condition,
            lock_source: sources.lock_source,
            revocation_source: sources.revocation_source,
            backend_unavailable_reason: sources.backend_unavailable_reason,
            recovery: sources.recovery,
        })
    }

    fn known(code: SecretSourceFreeErrorCode, context: SecretTerminalOperationContext) -> Self {
        Self::checked(code.stable_code(), context, SecretErrorSources::none())
            .expect("closed factory tuple")
    }

    pub(in crate::secret) fn input_invalid() -> Self {
        Self::known(SecretSourceFreeErrorCode::InputInvalid, SecretTerminalOperationContext::Summary)
    }
    pub(in crate::secret) fn recovery_changed() -> Self {
        Self::known(SecretSourceFreeErrorCode::RecoveryChanged, SecretTerminalOperationContext::Recovery)
    }
    pub(in crate::secret) fn dependency_changed() -> Self {
        Self::known(SecretSourceFreeErrorCode::DependencyChanged, SecretTerminalOperationContext::Summary)
    }
    pub(in crate::secret) fn capability_consumed() -> Self {
        Self::known(SecretSourceFreeErrorCode::CapabilityConsumed, SecretTerminalOperationContext::ApplyOrActivation)
    }

    pub(in crate::secret) fn terminal_operation_failure(
        code: SecretSourceFreeErrorCode,
        context: SecretTerminalOperationContext,
    ) -> Self {
        Self::known(code, context)
    }

    pub(in crate::secret) fn locked(
        context: SecretTerminalOperationContext,
        source: SecretLockSource,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretLocked,
            context,
            SecretErrorSources::locked(source),
        )
        .expect("lock source is required and exact")
    }

    pub(in crate::secret) fn revoked(
        context: SecretTerminalOperationContext,
        source: SecretRevocationSource,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretRevoked,
            context,
            SecretErrorSources::revoked(source),
        )
        .expect("revocation source is required and exact")
    }

    pub(in crate::secret) fn backend_unavailable(
        context: SecretTerminalOperationContext,
        reason: SecretBackendUnavailableReason,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretBackendUnavailable,
            context,
            SecretErrorSources::backend_unavailable(reason),
        )
        .expect("backend-unavailable reason is required and exact")
    }

    pub(in crate::secret) fn operation_recovery_required(
        pointer: SecretRecoveryPointer,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretOperationRecoveryRequired,
            SecretTerminalOperationContext::Recovery,
            SecretErrorSources::recovery(pointer),
        )
        .expect("general recovery requires exactly one typed pointer")
    }

    pub(in crate::secret) fn candidate_terminal_cleanup_pending() -> Self {
        Self::checked(
            SecretErrorCode::SecretOperationRecoveryRequired,
            SecretTerminalOperationContext::CandidateTerminalCleanupPending,
            SecretErrorSources::none(),
        )
        .expect("candidate terminal cleanup is the sole pointer-free recovery issue")
    }
}

// Compile-shape scanner rule: a `SecretInternalError` struct literal is allowed
// exactly once inside `SecretInternalError::checked`; fields have no getters and no module
// may re-export the type or create a literal. Error-to-wire conversion reads it
// only in the owner module and projects the §11-validated tuple without accepting
// any replacement code/action/source fields.

impl std::fmt::Debug for SecretInternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretInternalError(stable-code-only)")
    }
}

impl std::fmt::Display for SecretInternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("secret operation failed")
    }
}

impl std::error::Error for SecretInternalError {}
