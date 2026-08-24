use serde::{Deserialize, Serialize};

pub(crate) const CHANGE_PLAN_CONTRACT_VERSION: &str = "fyagent-change-plan/v1";
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
    Failed
});
string_enum!(ChangeStepKind {
    Precheck,
    Apply,
    Readback,
    Reconcile
});
string_enum!(ChangeStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
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
string_enum!(ChangeResultCode {
    Planned,
    Running,
    Applied,
    AppliedRestartRecommended,
    AppliedWithWarning,
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
string_enum!(ChangeApplyOutcomeKind { Admitted, Rejected });

impl ChangeJobStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Warning | Self::Failed)
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
    pub plan_id: String,
    pub target_provider_id: String,
    pub revision: i64,
    pub event_seq: i64,
    pub status: ChangeJobStatus,
    pub result_code: ChangeResultCode,
    pub steps: Vec<ChangeJobStep>,
    pub resources: Vec<ChangeResourceResult>,
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
            job_id,
            plan_id,
            target_provider_id: target_id,
            revision: 1,
            event_seq: first_event.sequence,
            status: ChangeJobStatus::Planned,
            result_code: ChangeResultCode::Planned,
            steps: [
                ChangeStepKind::Precheck,
                ChangeStepKind::Apply,
                ChangeStepKind::Readback,
                ChangeStepKind::Reconcile,
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
