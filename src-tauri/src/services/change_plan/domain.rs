use serde::{Deserialize, Serialize};

pub(crate) const CHANGE_PLAN_CONTRACT_VERSION: &str = "fyagent-change-plan/v2";
pub(crate) const CHANGE_PLAN_TTL_SECONDS: i64 = 15 * 60;

macro_rules! string_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

string_enum!(ChangeOperation {
    CodexProviderSwitch
});
string_enum!(ChangePlanStatus { Ready, Consumed });
string_enum!(SecretCapabilityResult {
    NoNewCredentialMaterial,
    SecretDependencyUnavailable
});
string_enum!(ChangeJobStatus {
    Planned,
    Running,
    Succeeded,
    Warning,
    Failed,
    Cancelled
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStepKind {
    Precheck,
    Snapshot,
    ManagedWrite,
    Readback,
    Finalize,
    /// Legacy v1 persistence-only decode variant. New jobs never emit it.
    Apply,
    /// Legacy v1 persistence-only decode variant. New jobs never emit it.
    Reconcile,
}
string_enum!(ChangeStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Compensating,
    Compensated,
    Skipped
});
string_enum!(ChangeResourceKind {
    ProviderDbCurrent,
    DeviceCurrent,
    TargetDefinition,
    CodexLiveProjection
});
string_enum!(ChangeResourceStatus {
    Pending,
    Matched,
    Mismatched,
    Unavailable
});
string_enum!(RestartRequirement {
    NotRequired,
    Recommended,
    Unknown
});
string_enum!(UsageEvidence { NotObserved });
string_enum!(RecoveryState {
    NotNeeded,
    Succeeded,
    RecoveryRequired
});
string_enum!(ChangeIdempotencyScope { Plan });
string_enum!(ChangeCancelMode { BeforeManagedWrite });
string_enum!(ChangeCompensationMode {
    WriterOwnedRollback
});
string_enum!(ChangeFaultPoint {
    BeforeManagedWrite,
    AfterManagedWriteBeforeRecord
});
string_enum!(ChangeAdapterErrorCode {
    PreconditionFailed,
    Transient,
    Permanent,
    UnknownOutcome,
    VerifyFailed,
    CompensationFailed,
    Unsupported
});
string_enum!(ChangeResultCode {
    Planned,
    Running,
    Applied,
    AppliedRestartRecommended,
    AppliedWithWarning,
    CancelledBeforeWrite,
    InterruptedBeforeWrite,
    RecoveredTargetReached,
    WriterFailedBaselineRestored,
    WriterErrorTargetReached,
    PostWriteMismatch,
    ReadbackUnavailable,
    RecoveryRequired
});
string_enum!(ChangePlanErrorCode {
    UnsupportedOperation,
    InvalidTarget,
    TargetNotFound,
    TargetAlreadyCurrent,
    SecretDependencyUnavailable,
    InvalidDigest,
    Expired,
    Consumed,
    Stale,
    PlanNotFound,
    JobNotFound,
    Internal
});
string_enum!(ChangeApplyOutcomeKind {
    Admitted,
    IdempotentReplay,
    Rejected
});
string_enum!(ChangeCancelCode {
    Accepted,
    CommitPointPassed,
    AlreadyTerminal,
    NotActive,
    JobNotFound
});
string_enum!(ChangeManualActionCode {
    RetryReadback,
    ReviewConfiguration
});

impl ChangeJobStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Warning | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeJobStep {
    pub kind: ChangeStepKind,
    pub status: ChangeStepStatus,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeResourceResult {
    pub kind: ChangeResourceKind,
    pub status: ChangeResourceStatus,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeJobEvent {
    pub sequence: i64,
    pub phase: ChangeStepKind,
    pub reason_code: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlanRisk {
    pub code: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAdapterDescriptor {
    pub adapter_id: String,
    pub adapter_version: String,
    pub operation_type: ChangeOperation,
    pub phases: Vec<ChangeStepKind>,
    pub read_set: Vec<ChangeResourceKind>,
    pub write_set: Vec<ChangeResourceKind>,
    pub idempotency_scope: ChangeIdempotencyScope,
    pub cancel_mode: ChangeCancelMode,
    pub compensation_mode: ChangeCompensationMode,
    pub fault_points: Vec<ChangeFaultPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChangePartialResult {
    pub succeeded_steps: Vec<ChangeStepKind>,
    pub compensated_steps: Vec<ChangeStepKind>,
    pub unverified_steps: Vec<ChangeStepKind>,
    pub remaining_effects: Vec<ChangeResourceKind>,
    pub manual_actions: Vec<ChangeManualActionCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlan {
    pub plan_id: String,
    pub operation: ChangeOperation,
    pub target_provider_id: String,
    pub target_provider_name: String,
    pub plan_digest: String,
    pub baseline_digest: String,
    pub db_baseline_provider_id: Option<String>,
    pub device_baseline_provider_id: Option<String>,
    pub secret_capability: SecretCapabilityResult,
    pub created_at: i64,
    pub expires_at: i64,
    pub status: ChangePlanStatus,
    pub adapter: ChangeAdapterDescriptor,
    pub current_provider_code: String,
    pub target_provider_code: String,
    pub restart_expectation: RestartRequirement,
    pub risks: Vec<ChangePlanRisk>,
    pub evidence_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeJobSnapshot {
    pub job_id: String,
    pub execution_id: String,
    pub plan_id: String,
    pub idempotency_key: String,
    pub target_provider_id: String,
    pub revision: i64,
    pub event_seq: i64,
    pub status: ChangeJobStatus,
    pub result_code: ChangeResultCode,
    pub adapter_error_code: Option<ChangeAdapterErrorCode>,
    pub steps: Vec<ChangeJobStep>,
    pub resources: Vec<ChangeResourceResult>,
    pub partial_result: Option<ChangePartialResult>,
    #[serde(default)]
    pub events: Vec<ChangeJobEvent>,
    pub restart_requirement: RestartRequirement,
    pub usage_evidence: UsageEvidence,
    pub recovery_state: RecoveryState,
    pub diagnostic_code: Option<String>,
    pub live_config_changed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ChangeJobSnapshot {
    pub(crate) fn needs_reconcile(&self) -> bool {
        !self.status.is_terminal() || self.recovery_state == RecoveryState::RecoveryRequired
    }

    pub(crate) fn planned(job_id: String, plan_id: String, target_id: String, now: i64) -> Self {
        let first_event = ChangeJobEvent {
            sequence: 1,
            phase: ChangeStepKind::Precheck,
            reason_code: "planned".to_string(),
            created_at: now,
        };
        Self {
            execution_id: job_id.clone(),
            job_id,
            idempotency_key: plan_id.clone(),
            plan_id,
            target_provider_id: target_id,
            revision: 1,
            event_seq: first_event.sequence,
            status: ChangeJobStatus::Planned,
            result_code: ChangeResultCode::Planned,
            adapter_error_code: None,
            steps: [
                ChangeStepKind::Precheck,
                ChangeStepKind::Snapshot,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Readback,
                ChangeStepKind::Finalize,
            ]
            .into_iter()
            .map(|kind| ChangeJobStep {
                kind,
                status: ChangeStepStatus::Pending,
                code: "pending".to_string(),
            })
            .collect(),
            resources: [
                ChangeResourceKind::ProviderDbCurrent,
                ChangeResourceKind::DeviceCurrent,
                ChangeResourceKind::TargetDefinition,
                ChangeResourceKind::CodexLiveProjection,
            ]
            .into_iter()
            .map(|kind| ChangeResourceResult {
                kind,
                status: ChangeResourceStatus::Pending,
                code: "pending".to_string(),
            })
            .collect(),
            partial_result: None,
            events: vec![first_event],
            restart_requirement: RestartRequirement::Unknown,
            usage_evidence: UsageEvidence::NotObserved,
            recovery_state: RecoveryState::NotNeeded,
            diagnostic_code: None,
            live_config_changed: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyChangePlanOutcome {
    pub kind: ChangeApplyOutcomeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<ChangeJobSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ChangePlanErrorCode>,
}

impl ApplyChangePlanOutcome {
    pub(crate) fn rejected(error_code: ChangePlanErrorCode) -> Self {
        Self {
            kind: ChangeApplyOutcomeKind::Rejected,
            job: None,
            error_code: Some(error_code),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelChangeJobOutcome {
    pub accepted: bool,
    pub code: ChangeCancelCode,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeJobEventHint {
    pub job_id: String,
    pub event_seq: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredChangePlan {
    pub public: ChangePlan,
    pub target_definition_digest: String,
    pub live_baseline_digest: String,
    /// Internal Change Plan readback baseline. This is a digest of the
    /// credential-neutral routing/model projection in `projection.rs`; it is
    /// not #112's future RFC8785 `projectionDigest` contract.
    pub target_projection_digest: String,
    pub contract_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WriterReceipt {
    pub live_config_changed: bool,
}

pub(crate) fn enum_json<T: Serialize>(value: T) -> Result<String, crate::AppError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| crate::AppError::Database("invalid change-plan enum".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DTO_CONTRACT_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/changePlanDtoContract.v2.json"
    ));

    fn descriptor() -> ChangeAdapterDescriptor {
        ChangeAdapterDescriptor {
            adapter_id: "codex_provider_switch".into(),
            adapter_version: "2".into(),
            operation_type: ChangeOperation::CodexProviderSwitch,
            phases: vec![
                ChangeStepKind::Precheck,
                ChangeStepKind::Snapshot,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Readback,
                ChangeStepKind::Finalize,
            ],
            read_set: vec![
                ChangeResourceKind::ProviderDbCurrent,
                ChangeResourceKind::DeviceCurrent,
                ChangeResourceKind::TargetDefinition,
                ChangeResourceKind::CodexLiveProjection,
            ],
            write_set: vec![
                ChangeResourceKind::ProviderDbCurrent,
                ChangeResourceKind::DeviceCurrent,
                ChangeResourceKind::CodexLiveProjection,
            ],
            idempotency_scope: ChangeIdempotencyScope::Plan,
            cancel_mode: ChangeCancelMode::BeforeManagedWrite,
            compensation_mode: ChangeCompensationMode::WriterOwnedRollback,
            fault_points: vec![
                ChangeFaultPoint::BeforeManagedWrite,
                ChangeFaultPoint::AfterManagedWriteBeforeRecord,
            ],
        }
    }

    #[test]
    fn v2_dto_serialization_matches_the_shared_renderer_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(DTO_CONTRACT_FIXTURE).unwrap();
        assert_eq!(fixture["contractVersion"], CHANGE_PLAN_CONTRACT_VERSION);

        let plan = ChangePlan {
            plan_id: "plan-1".into(),
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: "provider-1".into(),
            target_provider_name: "Provider One".into(),
            plan_digest: "a".repeat(64),
            baseline_digest: "b".repeat(64),
            db_baseline_provider_id: None,
            device_baseline_provider_id: Some("provider-before".into()),
            secret_capability: SecretCapabilityResult::NoNewCredentialMaterial,
            created_at: 1_800_000_000,
            expires_at: 1_800_000_900,
            status: ChangePlanStatus::Ready,
            adapter: descriptor(),
            current_provider_code: "current_mixed".into(),
            target_provider_code: "existing_provider".into(),
            restart_expectation: RestartRequirement::Recommended,
            risks: vec![ChangePlanRisk {
                code: "local_configuration_write".into(),
                severity: "notice".into(),
            }],
            evidence_note: "usage_not_observed".into(),
        };
        assert_eq!(serde_json::to_value(plan).unwrap(), fixture["plan"]);

        let job = ChangeJobSnapshot {
            job_id: "job-1".into(),
            execution_id: "job-1".into(),
            plan_id: "plan-1".into(),
            idempotency_key: "plan-1".into(),
            target_provider_id: "provider-1".into(),
            revision: 4,
            event_seq: 4,
            status: ChangeJobStatus::Running,
            result_code: ChangeResultCode::Running,
            adapter_error_code: None,
            steps: vec![
                ChangeJobStep {
                    kind: ChangeStepKind::Precheck,
                    status: ChangeStepStatus::Succeeded,
                    code: "baseline_matched".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Snapshot,
                    status: ChangeStepStatus::Succeeded,
                    code: "snapshot_bound".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::ManagedWrite,
                    status: ChangeStepStatus::Running,
                    code: "managed_write_started".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Readback,
                    status: ChangeStepStatus::Pending,
                    code: "pending".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Finalize,
                    status: ChangeStepStatus::Pending,
                    code: "pending".into(),
                },
            ],
            resources: [
                ChangeResourceKind::ProviderDbCurrent,
                ChangeResourceKind::DeviceCurrent,
                ChangeResourceKind::TargetDefinition,
                ChangeResourceKind::CodexLiveProjection,
            ]
            .into_iter()
            .map(|kind| ChangeResourceResult {
                kind,
                status: ChangeResourceStatus::Pending,
                code: "pending".into(),
            })
            .collect(),
            partial_result: Some(ChangePartialResult {
                succeeded_steps: vec![ChangeStepKind::Precheck, ChangeStepKind::Snapshot],
                ..ChangePartialResult::default()
            }),
            events: vec![
                ChangeJobEvent {
                    sequence: 1,
                    phase: ChangeStepKind::Precheck,
                    reason_code: "planned".into(),
                    created_at: 1_800_000_001,
                },
                ChangeJobEvent {
                    sequence: 2,
                    phase: ChangeStepKind::Precheck,
                    reason_code: "precheck_succeeded".into(),
                    created_at: 1_800_000_002,
                },
                ChangeJobEvent {
                    sequence: 3,
                    phase: ChangeStepKind::Snapshot,
                    reason_code: "snapshot_succeeded".into(),
                    created_at: 1_800_000_003,
                },
                ChangeJobEvent {
                    sequence: 4,
                    phase: ChangeStepKind::ManagedWrite,
                    reason_code: "managed_write_started".into(),
                    created_at: 1_800_000_004,
                },
            ],
            restart_requirement: RestartRequirement::Unknown,
            usage_evidence: UsageEvidence::NotObserved,
            recovery_state: RecoveryState::NotNeeded,
            diagnostic_code: None,
            live_config_changed: false,
            created_at: 1_800_000_001,
            updated_at: 1_800_000_004,
        };
        assert_eq!(serde_json::to_value(job).unwrap(), fixture["job"]);

        let cancel = CancelChangeJobOutcome {
            accepted: true,
            code: ChangeCancelCode::Accepted,
            job_id: "job-1".into(),
        };
        assert_eq!(
            serde_json::to_value(cancel).unwrap(),
            fixture["cancelOutcome"]
        );

        let hint = ChangeJobEventHint {
            job_id: "job-1".into(),
            event_seq: 4,
        };
        assert_eq!(serde_json::to_value(hint).unwrap(), fixture["eventHint"]);
    }
}
