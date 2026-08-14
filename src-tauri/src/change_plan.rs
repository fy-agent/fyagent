use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::services::provider::{build_effective_settings_with_common_config, read_live_settings};
use crate::store::AppState;
use crate::AppError;

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
    pub target_projection_digest: String,
    pub contract_digest: String,
}

#[derive(Debug, Clone, Serialize)]
struct BaselineDigestInput<'a> {
    contract: &'a str,
    db_current_provider_id: &'a Option<String>,
    device_current_provider_id: &'a Option<String>,
    effective_current_provider_id: &'a Option<String>,
    current_definition_digest: &'a Option<String>,
    target_provider_id: &'a str,
    target_definition_digest: &'a str,
    live_projection_digest: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) struct CodexSwitchInspection {
    pub db_current_provider_id: Option<String>,
    pub device_current_provider_id: Option<String>,
    pub effective_current_provider_id: Option<String>,
    pub current_definition_digest: Option<String>,
    pub target: Provider,
    pub target_definition_digest: String,
    pub live_projection_digest: String,
    pub live_projection_available: bool,
    pub target_projection_digest: String,
    pub baseline_digest: String,
}

fn change_plan_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn provider_definition_digest(provider: &Provider) -> String {
    let value = serde_json::to_value(provider).unwrap_or(Value::Null);
    digest_json("fyagent.change-plan.provider-definition.v1", &value)
}

fn codex_projection_for_digest(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    let Some(config) = object.get("config").and_then(Value::as_str) else {
        return value;
    };
    let Ok(mut config_value) = config.parse::<toml::Value>() else {
        return value;
    };
    if let Some(table) = config_value.as_table_mut() {
        // MCP projection is independently managed after the provider write and
        // therefore is not part of Provider switch identity.
        table.remove("mcp_servers");
    }
    if let Ok(normalized) = toml::to_string(&config_value) {
        object.insert("config".to_string(), Value::String(normalized));
    }
    value
}

fn live_projection_digest(result: Result<Value, AppError>) -> String {
    match result {
        Ok(value) => digest_json(
            "fyagent.change-plan.codex-live.v1",
            &codex_projection_for_digest(value),
        ),
        Err(_) => digest_bytes(
            "fyagent.change-plan.codex-live.v1",
            b"projection_unavailable",
        ),
    }
}

pub(crate) fn inspect_codex_switch(
    state: &AppState,
    target_provider_id: &str,
) -> Result<CodexSwitchInspection, ChangePlanErrorCode> {
    let target = state
        .db
        .get_provider_by_id(target_provider_id, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?
        .ok_or(ChangePlanErrorCode::TargetNotFound)?;
    let db_current_provider_id = state
        .db
        .get_current_provider(AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    let device_current_provider_id = crate::settings::get_current_provider(&AppType::Codex);
    let effective_current_provider_id = device_current_provider_id
        .as_ref()
        .filter(|id| {
            state
                .db
                .get_provider_by_id(id, AppType::Codex.as_str())
                .ok()
                .flatten()
                .is_some()
        })
        .cloned()
        .or_else(|| db_current_provider_id.clone());
    let current_definition_digest = effective_current_provider_id
        .as_ref()
        .and_then(|id| {
            state
                .db
                .get_provider_by_id(id, AppType::Codex.as_str())
                .ok()
                .flatten()
        })
        .map(|provider| provider_definition_digest(&provider));
    let target_definition_digest = provider_definition_digest(&target);
    let live_projection = read_live_settings(AppType::Codex);
    let live_projection_available = live_projection.is_ok();
    let live_projection_digest = live_projection_digest(live_projection);
    let effective_target =
        build_effective_settings_with_common_config(state.db.as_ref(), &AppType::Codex, &target)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
    let target_projection_digest = digest_json(
        "fyagent.change-plan.codex-live.v1",
        &codex_projection_for_digest(effective_target),
    );
    let baseline_value = serde_json::to_value(BaselineDigestInput {
        contract: CHANGE_PLAN_CONTRACT_VERSION,
        db_current_provider_id: &db_current_provider_id,
        device_current_provider_id: &device_current_provider_id,
        effective_current_provider_id: &effective_current_provider_id,
        current_definition_digest: &current_definition_digest,
        target_provider_id,
        target_definition_digest: &target_definition_digest,
        live_projection_digest: &live_projection_digest,
    })
    .map_err(|_| ChangePlanErrorCode::Internal)?;
    let baseline_digest = digest_json("fyagent.change-plan.baseline.v1", &baseline_value);
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        effective_current_provider_id,
        current_definition_digest,
        target,
        target_definition_digest,
        live_projection_digest,
        live_projection_available,
        target_projection_digest,
        baseline_digest,
    })
}

pub struct ChangePlanService;

impl ChangePlanService {
    pub fn plan_codex_switch(
        state: &AppState,
        target_provider_id: &str,
    ) -> Result<ChangePlan, ChangePlanErrorCode> {
        Self::plan_codex_switch_at(state, target_provider_id, chrono::Utc::now().timestamp())
    }

    pub(crate) fn plan_codex_switch_at(
        state: &AppState,
        target_provider_id: &str,
        now: i64,
    ) -> Result<ChangePlan, ChangePlanErrorCode> {
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let inspection = inspect_codex_switch(state, target_provider_id)?;
        if inspection.effective_current_provider_id.as_deref() == Some(target_provider_id) {
            return Err(ChangePlanErrorCode::TargetAlreadyCurrent);
        }
        let semantic = serde_json::json!({
            "operation": "codex_provider_switch",
            "targetProviderId": target_provider_id,
            "baselineDigest": inspection.baseline_digest,
            "contract": CHANGE_PLAN_CONTRACT_VERSION,
        });
        let plan_digest = digest_json("fyagent.change-plan.plan.v1", &semantic);
        let public = ChangePlan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: target_provider_id.to_string(),
            target_provider_name: inspection.target.name.clone(),
            plan_digest,
            baseline_digest: inspection.baseline_digest,
            created_at: now,
            expires_at: now + CHANGE_PLAN_TTL_SECONDS,
            status: ChangePlanStatus::Ready,
            current_provider_code: if inspection.effective_current_provider_id.is_some() {
                "current_configured".to_string()
            } else {
                "current_unconfigured".to_string()
            },
            target_provider_code: "existing_provider".to_string(),
            restart_expectation: RestartRequirement::Recommended,
            risks: vec![ChangePlanRisk {
                code: "local_configuration_write".to_string(),
                severity: "notice".to_string(),
            }],
            evidence_note: "usage_not_observed".to_string(),
        };
        state
            .db
            .insert_change_plan(&StoredChangePlan {
                public: public.clone(),
                current_provider_id: inspection.effective_current_provider_id,
                current_definition_digest: inspection.current_definition_digest,
                target_definition_digest: inspection.target_definition_digest,
                live_projection_digest: inspection.live_projection_digest,
                target_projection_digest: inspection.target_projection_digest,
                contract_digest: CHANGE_PLAN_CONTRACT_VERSION.to_string(),
            })
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        Ok(public)
    }

    pub fn apply_codex_switch(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode> {
        let now = chrono::Utc::now().timestamp();
        Self::apply_codex_switch_at_with_writer(state, plan_id, plan_digest, now, || {
            crate::services::ProviderService::with_live_config_result(AppType::Codex, || {
                crate::services::ProviderService::switch(
                    state,
                    AppType::Codex,
                    &state
                        .db
                        .get_stored_change_plan(plan_id)
                        .map_err(|_| AppError::Message("change plan unavailable".to_string()))?
                        .ok_or_else(|| AppError::Message("change plan unavailable".to_string()))?
                        .public
                        .target_provider_id,
                )
            })
            .map(|result| result.live_config_changed)
            .map_err(|_| ())
        })
    }

    pub(crate) fn apply_codex_switch_at_with_writer<F>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        writer: F,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce() -> Result<bool, ()>,
    {
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let stored = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let Some(stored) = stored else {
            return Ok(ApplyChangePlanOutcome {
                kind: ChangeApplyOutcomeKind::Rejected,
                job: None,
                error_code: Some(ChangePlanErrorCode::PlanNotFound),
            });
        };
        let observed = inspect_codex_switch(state, &stored.public.target_provider_id)?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let admitted = state
            .db
            .admit_change_plan(
                plan_id,
                plan_digest,
                &observed.baseline_digest,
                &job_id,
                now,
            )
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if admitted.kind == ChangeApplyOutcomeKind::Rejected {
            return Ok(admitted);
        }
        let mut job = admitted.job.ok_or(ChangePlanErrorCode::Internal)?;
        set_step(
            &mut job,
            ChangeStepKind::Precheck,
            ChangeStepStatus::Succeeded,
            "baseline_matched",
        );
        set_step(
            &mut job,
            ChangeStepKind::Apply,
            ChangeStepStatus::Running,
            "writer_started",
        );
        advance_job(&mut job, now, "running");
        state
            .db
            .save_change_job(&job, "writer_started")
            .map_err(|_| ChangePlanErrorCode::Internal)?;

        let writer_result = writer();
        set_step(
            &mut job,
            ChangeStepKind::Apply,
            if writer_result.is_ok() {
                ChangeStepStatus::Succeeded
            } else {
                ChangeStepStatus::Failed
            },
            if writer_result.is_ok() {
                "writer_returned"
            } else {
                "writer_failed"
            },
        );
        set_step(
            &mut job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Running,
            "readback_started",
        );
        advance_job(&mut job, now, "readback");
        state
            .db
            .save_change_job(&job, "readback_started")
            .map_err(|_| ChangePlanErrorCode::Internal)?;

        let readback = inspect_codex_switch(state, &stored.public.target_provider_id);
        classify_job(&stored, &mut job, writer_result, readback, now);
        state
            .db
            .save_change_job(&job, "terminal")
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        Ok(ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Admitted,
            job: Some(job),
            error_code: None,
        })
    }

    pub fn get_job(
        state: &AppState,
        job_id: &str,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
        let job = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        if job.status.is_terminal() {
            return Ok(job);
        }
        Self::reconcile_job(state, job)
    }

    fn reconcile_job(
        state: &AppState,
        mut job: ChangeJobSnapshot,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let stored = state
            .db
            .get_stored_change_plan(&job.plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::PlanNotFound)?;
        let now = chrono::Utc::now().timestamp();
        set_step(
            &mut job,
            ChangeStepKind::Reconcile,
            ChangeStepStatus::Running,
            "reconcile_started",
        );
        let readback = inspect_codex_switch(state, &stored.public.target_provider_id);
        classify_job(&stored, &mut job, Err(()), readback, now);
        let reconcile_status = if job.recovery_state == RecoveryState::RecoveryRequired {
            ChangeStepStatus::Failed
        } else {
            ChangeStepStatus::Succeeded
        };
        set_step(
            &mut job,
            ChangeStepKind::Reconcile,
            reconcile_status,
            "reconciled_without_replay",
        );
        state
            .db
            .save_change_job(&job, "reconciled")
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        Ok(job)
    }

    pub fn list_recoverable_jobs(
        state: &AppState,
    ) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode> {
        let jobs = state
            .db
            .list_recoverable_change_jobs()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        jobs.into_iter()
            .map(|job| Self::reconcile_job(state, job))
            .collect()
    }
}

fn set_step(
    job: &mut ChangeJobSnapshot,
    kind: ChangeStepKind,
    status: ChangeStepStatus,
    code: &str,
) {
    if let Some(step) = job.steps.iter_mut().find(|step| step.kind == kind) {
        step.status = status;
        step.code = code.to_string();
    }
}

fn set_resource(
    job: &mut ChangeJobSnapshot,
    kind: ChangeResourceKind,
    status: ChangeResourceStatus,
    code: &str,
) {
    if let Some(resource) = job.resources.iter_mut().find(|item| item.kind == kind) {
        resource.status = status;
        resource.code = code.to_string();
    }
}

fn advance_job(job: &mut ChangeJobSnapshot, now: i64, code: &str) {
    job.revision += 1;
    job.event_seq += 1;
    job.status = ChangeJobStatus::Running;
    job.result_code = ChangeResultCode::Running;
    job.diagnostic_code = Some(code.to_string());
    job.updated_at = now;
}

fn classify_job(
    stored: &StoredChangePlan,
    job: &mut ChangeJobSnapshot,
    writer_result: Result<bool, ()>,
    readback: Result<CodexSwitchInspection, ChangePlanErrorCode>,
    now: i64,
) {
    job.revision += 1;
    job.event_seq += 1;
    job.updated_at = now;
    let Ok(readback) = readback else {
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::ReadbackUnavailable;
        job.recovery_state = RecoveryState::RecoveryRequired;
        job.restart_requirement = RestartRequirement::Unknown;
        job.diagnostic_code = Some("readback_unavailable".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "readback_unavailable",
        );
        return;
    };

    let target_id = &stored.public.target_provider_id;
    let db_target = readback.db_current_provider_id.as_ref() == Some(target_id);
    let device_target = readback.device_current_provider_id.as_ref() == Some(target_id);
    let definition_target = readback.target_definition_digest == stored.target_definition_digest;
    let live_target = readback.live_projection_available
        && readback.live_projection_digest == stored.target_projection_digest;
    let baseline_db = readback.db_current_provider_id == stored.current_provider_id;
    let baseline_device = readback.device_current_provider_id == stored.current_provider_id;
    let baseline_live = readback.live_projection_digest == stored.live_projection_digest;

    set_resource(
        job,
        ChangeResourceKind::ProviderDbCurrent,
        if db_target {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if db_target {
            "target_current"
        } else {
            "target_not_current"
        },
    );
    set_resource(
        job,
        ChangeResourceKind::DeviceCurrent,
        if device_target {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if device_target {
            "target_current"
        } else {
            "target_not_current"
        },
    );
    set_resource(
        job,
        ChangeResourceKind::TargetDefinition,
        if definition_target {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if definition_target {
            "definition_matched"
        } else {
            "definition_drifted"
        },
    );
    set_resource(
        job,
        ChangeResourceKind::CodexLiveProjection,
        if !readback.live_projection_available {
            ChangeResourceStatus::Unavailable
        } else if live_target {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if !readback.live_projection_available {
            "live_unavailable"
        } else if live_target {
            "live_matched"
        } else {
            "live_mismatched"
        },
    );

    if db_target && device_target && definition_target && live_target {
        job.live_config_changed = writer_result.unwrap_or(false);
        job.restart_requirement = if job.live_config_changed {
            RestartRequirement::Recommended
        } else {
            RestartRequirement::NotRequired
        };
        job.status = if writer_result.is_ok() {
            ChangeJobStatus::Succeeded
        } else {
            ChangeJobStatus::Warning
        };
        job.result_code = if writer_result.is_err() {
            ChangeResultCode::WriterErrorTargetReached
        } else if job.live_config_changed {
            ChangeResultCode::AppliedRestartRecommended
        } else {
            ChangeResultCode::Applied
        };
        job.recovery_state = RecoveryState::NotNeeded;
        job.diagnostic_code = Some("target_readback_matched".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "target_matched",
        );
    } else if baseline_db && baseline_device && baseline_live {
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::WriterFailedBaselineRestored;
        job.restart_requirement = RestartRequirement::NotRequired;
        job.recovery_state = RecoveryState::Succeeded;
        job.diagnostic_code = Some("baseline_restored".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "baseline_restored",
        );
    } else {
        job.status = ChangeJobStatus::Failed;
        job.result_code = if readback.live_projection_available {
            ChangeResultCode::PostWriteMismatch
        } else {
            ChangeResultCode::ReadbackUnavailable
        };
        job.restart_requirement = RestartRequirement::Unknown;
        job.recovery_state = RecoveryState::RecoveryRequired;
        job.diagnostic_code = Some("recovery_required".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "state_mixed",
        );
    }
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
    use crate::database::Database;
    use crate::services::provider::write_live_with_common_config;
    use crate::store::AppState;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    struct TestHome(Option<std::ffi::OsString>);

    impl TestHome {
        fn set(path: &std::path::Path) -> Self {
            let previous = std::env::var_os("FYAGENT_TEST_HOME");
            std::env::set_var("FYAGENT_TEST_HOME", path);
            Self(previous)
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
        }
    }

    fn provider(id: &str, name: &str, model: &str) -> Provider {
        Provider::with_id(
            id.to_string(),
            name.to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": format!("sentinel-{id}")},
                "config": format!("model = \"{model}\"\n")
            }),
            None,
        )
    }

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

    #[test]
    #[serial]
    fn codex_provider_switch_plan_is_semantically_stable_unique_and_side_effect_free() {
        let home = tempfile::tempdir().expect("test home");
        let _home = TestHome::set(home.path());
        let db = Arc::new(Database::memory().expect("database"));
        let current = provider("current", "Current", "gpt-current");
        let target = provider("target", "Target", "gpt-target");
        db.save_provider(AppType::Codex.as_str(), &current).unwrap();
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        db.set_current_provider(AppType::Codex.as_str(), &current.id)
            .unwrap();
        crate::settings::set_current_provider(&AppType::Codex, Some(&current.id)).unwrap();
        write_live_with_common_config(db.as_ref(), &AppType::Codex, &current).unwrap();
        let state = AppState::new(db.clone());

        let before_db_current = db.get_current_provider(AppType::Codex.as_str()).unwrap();
        let before_device = crate::settings::get_current_provider(&AppType::Codex);
        let before_current = provider_definition_digest(
            &db.get_provider_by_id(&current.id, AppType::Codex.as_str())
                .unwrap()
                .unwrap(),
        );
        let before_target = provider_definition_digest(
            &db.get_provider_by_id(&target.id, AppType::Codex.as_str())
                .unwrap()
                .unwrap(),
        );
        let before_live = read_live_settings(AppType::Codex).unwrap();

        let first = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let second = ChangePlanService::plan_codex_switch_at(&state, &target.id, 101).unwrap();

        assert_ne!(first.plan_id, second.plan_id);
        assert_eq!(first.plan_digest, second.plan_digest);
        assert_eq!(first.baseline_digest, second.baseline_digest);
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            before_db_current
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Codex),
            before_device
        );
        assert_eq!(
            provider_definition_digest(
                &db.get_provider_by_id(&current.id, AppType::Codex.as_str())
                    .unwrap()
                    .unwrap(),
            ),
            before_current
        );
        assert_eq!(
            provider_definition_digest(
                &db.get_provider_by_id(&target.id, AppType::Codex.as_str())
                    .unwrap()
                    .unwrap(),
            ),
            before_target
        );
        assert_eq!(read_live_settings(AppType::Codex).unwrap(), before_live);
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
        let conn = db.conn.lock().unwrap();
        let plan_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(plan_count, 2, "only immutable plan rows may be written");
    }

    #[test]
    #[serial]
    fn change_plan_no_side_effects_rejects_missing_and_current_targets() {
        let home = tempfile::tempdir().expect("test home");
        let _home = TestHome::set(home.path());
        let db = Arc::new(Database::memory().unwrap());
        let current = provider("current", "Current", "gpt-current");
        db.save_provider(AppType::Codex.as_str(), &current).unwrap();
        db.set_current_provider(AppType::Codex.as_str(), &current.id)
            .unwrap();
        crate::settings::set_current_provider(&AppType::Codex, Some(&current.id)).unwrap();
        write_live_with_common_config(db.as_ref(), &AppType::Codex, &current).unwrap();
        let state = AppState::new(db.clone());
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, "missing", 100),
            Err(ChangePlanErrorCode::TargetNotFound)
        );
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &current.id, 100),
            Err(ChangePlanErrorCode::TargetAlreadyCurrent)
        );
        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    fn setup_switch_state() -> (
        tempfile::TempDir,
        TestHome,
        Arc<Database>,
        AppState,
        Provider,
        Provider,
    ) {
        let home = tempfile::tempdir().expect("test home");
        let home_guard = TestHome::set(home.path());
        let db = Arc::new(Database::memory().expect("database"));
        let current = provider("current", "Current", "gpt-current");
        let target = provider("target", "Target", "gpt-target");
        db.save_provider(AppType::Codex.as_str(), &current).unwrap();
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        db.set_current_provider(AppType::Codex.as_str(), &current.id)
            .unwrap();
        crate::settings::set_current_provider(&AppType::Codex, Some(&current.id)).unwrap();
        write_live_with_common_config(db.as_ref(), &AppType::Codex, &current).unwrap();
        let state = AppState::new(db.clone());
        (home, home_guard, db, state, current, target)
    }

    #[test]
    #[serial]
    fn codex_provider_change_job_calls_existing_writer_once_and_rejects_replay() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                crate::services::ProviderService::with_live_config_result(AppType::Codex, || {
                    crate::services::ProviderService::switch(&state, AppType::Codex, &target.id)
                })
                .map(|result| result.live_config_changed)
                .map_err(|_| ())
            },
        )
        .unwrap();
        let job = outcome.job.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(job.status, ChangeJobStatus::Succeeded);
        assert!(matches!(
            job.result_code,
            ChangeResultCode::Applied | ChangeResultCode::AppliedRestartRecommended
        ));
        assert_eq!(job.usage_evidence, UsageEvidence::NotObserved);
        assert!(job
            .resources
            .iter()
            .all(|resource| resource.status == ChangeResourceStatus::Matched));

        let replay = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            102,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(replay.error_code, Some(ChangePlanErrorCode::Consumed));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "replay must not call writer"
        );
    }

    #[test]
    #[serial]
    fn codex_provider_change_job_stale_plan_performs_zero_writes() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        target.settings_config["config"] = Value::String("model = \"drifted\"\n".into());
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str())
                .unwrap()
                .as_deref(),
            Some("current")
        );
    }

    #[test]
    #[serial]
    fn change_plan_reconciliation_reads_state_without_replaying_writer() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let inspected = inspect_codex_switch(&state, &target.id).unwrap();
        let admitted = db
            .admit_change_plan(
                &plan.plan_id,
                &plan.plan_digest,
                &inspected.baseline_digest,
                "interrupted-job",
                101,
            )
            .unwrap();
        assert_eq!(admitted.kind, ChangeApplyOutcomeKind::Admitted);
        crate::services::ProviderService::switch(&state, AppType::Codex, &target.id).unwrap();
        let before = db.get_current_provider(AppType::Codex.as_str()).unwrap();
        let reconciled = ChangePlanService::get_job(&state, "interrupted-job").unwrap();
        assert_eq!(reconciled.status, ChangeJobStatus::Warning);
        assert_eq!(
            reconciled.result_code,
            ChangeResultCode::WriterErrorTargetReached
        );
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            before
        );
        assert_eq!(
            reconciled
                .steps
                .iter()
                .find(|step| step.kind == ChangeStepKind::Reconcile)
                .unwrap()
                .code,
            "reconciled_without_replay"
        );
    }

    #[test]
    #[serial]
    fn codex_provider_change_job_classifies_recovery_states_from_readback() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let baseline_plan =
            ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let baseline = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &baseline_plan.plan_id,
            &baseline_plan.plan_digest,
            101,
            || Err(()),
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(
            baseline.result_code,
            ChangeResultCode::WriterFailedBaselineRestored
        );
        assert_eq!(baseline.recovery_state, RecoveryState::Succeeded);

        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
        let mixed = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            201,
            || {
                db.set_current_provider(AppType::Codex.as_str(), &target.id)
                    .unwrap();
                Err(())
            },
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(mixed.status, ChangeJobStatus::Failed);
        assert_eq!(mixed.recovery_state, RecoveryState::RecoveryRequired);
        assert_eq!(mixed.result_code, ChangeResultCode::PostWriteMismatch);
    }
}
