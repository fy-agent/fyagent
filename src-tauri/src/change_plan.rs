use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) const CHANGE_PLAN_CONTRACT_VERSION: &str = "fyagent-change-plan-v1-schema16";
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
    TargetNotFound,
    TargetAlreadyCurrent,
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
    pub restart_requirement: RestartRequirement,
    pub usage_evidence: UsageEvidence,
    pub recovery_state: RecoveryState,
    pub diagnostic_code: Option<String>,
    pub live_config_changed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ChangeJobSnapshot {
    pub(crate) fn planned(job_id: String, plan_id: String, target_id: String, now: i64) -> Self {
        Self {
            job_id,
            plan_id,
            target_provider_id: target_id,
            revision: 1,
            event_seq: 1,
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

#[derive(Debug, Clone)]
pub(crate) struct StoredChangePlan {
    pub public: ChangePlan,
    pub current_provider_id: Option<String>,
    pub current_definition_digest: Option<String>,
    pub target_definition_digest: String,
    pub live_projection_digest: String,
    pub contract_digest: String,
}

pub(crate) fn enum_json<T: Serialize>(value: T) -> Result<String, crate::AppError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| crate::AppError::Database("invalid change-plan enum".to_string()))
}

pub(crate) fn digest_bytes(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            let mut result = serde_json::Map::new();
            for (key, value) in entries {
                result.insert(key.clone(), canonical_json(value));
            }
            Value::Object(result)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

pub(crate) fn digest_json(domain: &str, value: &Value) -> String {
    let bytes = serde_json::to_vec(&canonical_json(value)).unwrap_or_default();
    digest_bytes(domain, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn change_plan_contract_is_closed_camel_case_and_redacted() {
        let plan = ChangePlan {
            plan_id: "plan-safe".into(),
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: "provider-safe".into(),
            target_provider_name: "Safe Provider".into(),
            plan_digest: "a".repeat(64),
            baseline_digest: "b".repeat(64),
            created_at: 1,
            expires_at: 2,
            status: ChangePlanStatus::Ready,
            current_provider_code: "current_configured".into(),
            target_provider_code: "existing_provider".into(),
            restart_expectation: RestartRequirement::Recommended,
            risks: vec![ChangePlanRisk {
                code: "local_configuration_write".into(),
                severity: "notice".into(),
            }],
            evidence_note: "usage_not_observed".into(),
        };
        let serialized = serde_json::to_string(&plan).expect("serialize contract");
        assert!(serialized.contains("\"planId\""));
        assert!(serialized.contains("\"codex_provider_switch\""));
        for forbidden in ["settingsConfig", "api_key", "/Users/", "rawError"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn change_plan_contract_digest_is_stable_for_object_key_order() {
        let left = json!({"b": 2, "a": {"y": 1, "x": 0}});
        let right = json!({"a": {"x": 0, "y": 1}, "b": 2});
        assert_eq!(
            digest_json("change-plan-test", &left),
            digest_json("change-plan-test", &right)
        );
    }
}
