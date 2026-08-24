use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::services::provider::{
    build_codex_switch_target_live_projection, inspect_codex_switch_environment,
};
use crate::services::ProviderService;
use crate::store::AppState;

use super::adapter::{
    descriptor_for_operation, ChangeAdapter, CodexProviderSwitchAdapter, CodexSwitchSnapshot,
};
use super::domain::*;
use super::projection::{
    credential_neutral_codex_projection, digest_json, digest_serializable,
    provider_definition_digest,
};
use super::sanitize::is_safe_opaque_id;

#[derive(Debug, Clone, Serialize)]
struct BaselineDigestInput<'a> {
    contract: &'a str,
    db_current_provider_id: &'a Option<String>,
    device_current_provider_id: &'a Option<String>,
    current_definition_digest: &'a Option<String>,
    target_provider_id: &'a str,
    target_definition_digest: &'a str,
    live_projection_digest: &'a str,
    switch_mode_code: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) struct CodexSwitchInspection {
    pub db_current_provider_id: Option<String>,
    pub device_current_provider_id: Option<String>,
    pub target: Provider,
    pub target_definition_digest: String,
    pub live_projection_digest: String,
    pub live_projection_available: bool,
    pub target_projection_digest: String,
    pub baseline_digest: String,
    pub preserved_strict_login: bool,
}

pub struct ChangePlanService;

const EXECUTION_CANCEL_SAFE: u8 = 0;
const EXECUTION_WRITE_CLAIMED: u8 = 1;
const EXECUTION_CANCELLED: u8 = 2;
const EXECUTION_TERMINAL: u8 = 3;

struct ExecutionGate {
    state: AtomicU8,
}

impl ExecutionGate {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(EXECUTION_CANCEL_SAFE),
        }
    }

    fn request_cancel(&self) -> ChangeCancelCode {
        match self.state.compare_exchange(
            EXECUTION_CANCEL_SAFE,
            EXECUTION_CANCELLED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) | Err(EXECUTION_CANCELLED) => ChangeCancelCode::Accepted,
            Err(EXECUTION_TERMINAL) => ChangeCancelCode::AlreadyTerminal,
            Err(_) => ChangeCancelCode::CommitPointPassed,
        }
    }

    fn claim_managed_write(&self) -> bool {
        self.state
            .compare_exchange(
                EXECUTION_CANCEL_SAFE,
                EXECUTION_WRITE_CLAIMED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn mark_terminal(&self) {
        self.state.store(EXECUTION_TERMINAL, Ordering::SeqCst);
    }
}

fn active_executions() -> &'static Mutex<HashMap<String, Arc<ExecutionGate>>> {
    static EXECUTIONS: OnceLock<Mutex<HashMap<String, Arc<ExecutionGate>>>> = OnceLock::new();
    EXECUTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_execution_gate(job_id: &str) -> Option<Arc<ExecutionGate>> {
    active_executions()
        .lock()
        .ok()
        .and_then(|executions| executions.get(job_id).cloned())
}

struct ActiveExecutionRegistration {
    job_id: String,
    gate: Arc<ExecutionGate>,
}

impl ActiveExecutionRegistration {
    fn register(job_id: &str) -> Result<Self, ChangePlanErrorCode> {
        let gate = Arc::new(ExecutionGate::new());
        let mut executions = active_executions()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if executions
            .insert(job_id.to_string(), gate.clone())
            .is_some()
        {
            return Err(ChangePlanErrorCode::Internal);
        }
        Ok(Self {
            job_id: job_id.to_string(),
            gate,
        })
    }
}

impl Drop for ActiveExecutionRegistration {
    fn drop(&mut self) {
        if let Ok(mut executions) = active_executions().lock() {
            if executions
                .get(&self.job_id)
                .is_some_and(|current| Arc::ptr_eq(current, &self.gate))
            {
                executions.remove(&self.job_id);
            }
        }
    }
}

impl ChangePlanService {
    /// Create an immutable, credential-free plan. The existing Provider
    /// mutation guard makes the DB/device/live baseline one stable snapshot;
    /// this path performs no Provider write or network request.
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
        if !is_safe_opaque_id(target_provider_id) {
            return Err(ChangePlanErrorCode::InvalidTarget);
        }

        let _provider_guard = ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let adapter = CodexProviderSwitchAdapter::for_plan(state, target_provider_id);
        let inspection = adapter.inspect()?;
        let secret_capability = prove_codex_target_credential_capability(&inspection);
        if secret_capability != SecretCapabilityResult::NoNewCredentialMaterial {
            return Err(ChangePlanErrorCode::SecretDependencyUnavailable);
        }
        if inspection.is_fully_target() {
            return Err(ChangePlanErrorCode::TargetAlreadyCurrent);
        }

        let descriptor = adapter.descriptor();
        if descriptor.compensation_mode != adapter.compensation_capability() {
            return Err(ChangePlanErrorCode::Internal);
        }
        let plan_fields = adapter.plan(&inspection);
        let public = ChangePlan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: target_provider_id.to_string(),
            target_provider_name: plan_fields.target_provider_name,
            plan_digest: plan_approval_digest(
                ChangeOperation::CodexProviderSwitch,
                target_provider_id,
                &inspection.baseline_digest,
                secret_capability,
                &descriptor,
            ),
            baseline_digest: inspection.baseline_digest,
            db_baseline_provider_id: inspection.db_current_provider_id,
            device_baseline_provider_id: inspection.device_current_provider_id,
            secret_capability,
            created_at: now,
            expires_at: now + CHANGE_PLAN_TTL_SECONDS,
            status: ChangePlanStatus::Ready,
            adapter: descriptor,
            current_provider_code: plan_fields.current_provider_code,
            target_provider_code: plan_fields.target_provider_code,
            restart_expectation: plan_fields.restart_expectation,
            risks: plan_fields.risks,
            evidence_note: plan_fields.evidence_note,
        };
        state
            .db
            .insert_change_plan(&StoredChangePlan {
                public: public.clone(),
                target_definition_digest: inspection.target_definition_digest,
                live_baseline_digest: inspection.live_projection_digest,
                target_projection_digest: inspection.target_projection_digest,
                contract_digest: CHANGE_PLAN_CONTRACT_VERSION.to_string(),
            })
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        Ok(public)
    }

    /// Apply using the canonical Provider lock. The supplied writer MUST be
    /// the ProviderService lock-held switch primitive; the integration worker
    /// owns that shared extraction. `FnOnce` plus admission CAS guarantees at
    /// most one writer call.
    pub(crate) fn apply_codex_switch_with_writer_observer<F, E, O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        writer: F,
        observer: O,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce(&str) -> Result<WriterReceipt, E>,
        O: Fn(ChangeJobEventHint),
    {
        Self::apply_codex_switch_at_with_writer_observer_and_fault(
            state,
            plan_id,
            plan_digest,
            chrono::Utc::now().timestamp(),
            writer,
            observer,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn apply_codex_switch_at_with_writer<F, E>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        writer: F,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce(&str) -> Result<WriterReceipt, E>,
    {
        Self::apply_codex_switch_at_with_writer_observer_and_fault(
            state,
            plan_id,
            plan_digest,
            now,
            writer,
            |_| {},
            None,
        )
    }

    pub(crate) fn apply_codex_switch_at_with_writer_observer_and_fault<F, E, O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        writer: F,
        observer: O,
        injected_fault: Option<ChangeFaultPoint>,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce(&str) -> Result<WriterReceipt, E>,
        O: Fn(ChangeJobEventHint),
    {
        let writer = move |target: &str| writer(target).map_err(|_| ());
        Self::apply_codex_switch_inner(
            state,
            plan_id,
            plan_digest,
            now,
            writer,
            observer,
            injected_fault,
        )
    }

    fn idempotent_replay_if_consumed(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
    ) -> Result<Option<ApplyChangePlanOutcome>, ChangePlanErrorCode> {
        let stored = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let Some(stored) = stored else {
            return Ok(Some(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::PlanNotFound,
            )));
        };
        if stored.public.plan_digest != plan_digest {
            return Ok(Some(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::InvalidDigest,
            )));
        }
        if stored.public.status != ChangePlanStatus::Consumed {
            return Ok(None);
        }
        if stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION
            || stored.public.adapter != descriptor_for_operation(stored.public.operation)
        {
            return Ok(Some(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::Stale,
            )));
        }
        if stored.public.plan_digest
            != plan_approval_digest(
                stored.public.operation,
                &stored.public.target_provider_id,
                &stored.public.baseline_digest,
                stored.public.secret_capability,
                &stored.public.adapter,
            )
        {
            return Ok(Some(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::Stale,
            )));
        }
        let mut job = state
            .db
            .get_change_job_by_plan_id(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::Internal)?;
        normalize_job_projection(&mut job);
        Ok(Some(ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::IdempotentReplay,
            job: Some(job),
            error_code: None,
        }))
    }

    fn apply_codex_switch_inner<F, O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        writer: F,
        observer: O,
        injected_fault: Option<ChangeFaultPoint>,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce(&str) -> Result<WriterReceipt, ()>,
        O: Fn(ChangeJobEventHint),
    {
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let _provider_guard = ProviderService::lock_provider_mutation(state, &AppType::Codex);
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let stored = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let Some(stored) = stored else {
            return Ok(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::PlanNotFound,
            ));
        };
        if stored.public.secret_capability != SecretCapabilityResult::NoNewCredentialMaterial {
            return Ok(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::SecretDependencyUnavailable,
            ));
        }
        if stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION {
            return Ok(ApplyChangePlanOutcome::rejected(ChangePlanErrorCode::Stale));
        }
        if stored.public.adapter != descriptor_for_operation(stored.public.operation) {
            return Ok(ApplyChangePlanOutcome::rejected(ChangePlanErrorCode::Stale));
        }
        if stored.public.plan_digest
            != plan_approval_digest(
                stored.public.operation,
                &stored.public.target_provider_id,
                &stored.public.baseline_digest,
                stored.public.secret_capability,
                &stored.public.adapter,
            )
        {
            return Ok(ApplyChangePlanOutcome::rejected(ChangePlanErrorCode::Stale));
        }

        let mut adapter = CodexProviderSwitchAdapter::for_execution(
            state,
            &stored.public.target_provider_id,
            writer,
        );
        let observed = adapter.precheck()?;
        if prove_codex_target_credential_capability(&observed)
            != SecretCapabilityResult::NoNewCredentialMaterial
        {
            return Ok(ApplyChangePlanOutcome::rejected(
                ChangePlanErrorCode::SecretDependencyUnavailable,
            ));
        }
        let captured = adapter.snapshot(&observed);
        let expected = CodexSwitchSnapshot {
            baseline_digest: stored.public.baseline_digest.clone(),
            target_definition_digest: stored.target_definition_digest.clone(),
            target_projection_digest: stored.target_projection_digest.clone(),
        };
        if captured != expected {
            return Ok(ApplyChangePlanOutcome::rejected(ChangePlanErrorCode::Stale));
        }
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
        normalize_job_projection(&mut job);
        let execution = ActiveExecutionRegistration::register(&job.job_id)?;
        observer(job_hint(&job));

        job.status = ChangeJobStatus::Running;
        job.result_code = ChangeResultCode::Running;
        set_step(
            &mut job,
            ChangeStepKind::Precheck,
            ChangeStepStatus::Succeeded,
            "baseline_matched",
        );
        persist_transition(
            state,
            &mut job,
            ChangeStepKind::Precheck,
            "precheck_succeeded",
            now,
            &observer,
        )?;

        set_step(
            &mut job,
            ChangeStepKind::Snapshot,
            ChangeStepStatus::Succeeded,
            "snapshot_bound",
        );
        persist_transition(
            state,
            &mut job,
            ChangeStepKind::Snapshot,
            "snapshot_succeeded",
            now,
            &observer,
        )?;

        if injected_fault == Some(ChangeFaultPoint::BeforeManagedWrite) {
            return Err(ChangePlanErrorCode::Internal);
        }

        if !execution.gate.claim_managed_write() {
            set_step(
                &mut job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Skipped,
                "cancelled_before_write",
            );
            set_step(
                &mut job,
                ChangeStepKind::Readback,
                ChangeStepStatus::Skipped,
                "cancelled_before_write",
            );
            set_step(
                &mut job,
                ChangeStepKind::Finalize,
                ChangeStepStatus::Succeeded,
                "cancelled_before_write",
            );
            job.status = ChangeJobStatus::Cancelled;
            job.result_code = ChangeResultCode::CancelledBeforeWrite;
            job.restart_requirement = RestartRequirement::NotRequired;
            job.recovery_state = RecoveryState::NotNeeded;
            job.diagnostic_code = Some("cancelled_before_write".to_string());
            persist_transition(
                state,
                &mut job,
                ChangeStepKind::Finalize,
                "cancelled_before_write",
                now,
                &observer,
            )?;
            execution.gate.mark_terminal();
            return Ok(ApplyChangePlanOutcome {
                kind: ChangeApplyOutcomeKind::Admitted,
                job: Some(job),
                error_code: None,
            });
        }

        set_step(
            &mut job,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Running,
            "managed_write_started",
        );
        persist_transition(
            state,
            &mut job,
            ChangeStepKind::ManagedWrite,
            "managed_write_started",
            now,
            &observer,
        )?;

        let writer_result = adapter.managed_write();
        if injected_fault == Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord) {
            return Err(ChangePlanErrorCode::Internal);
        }
        set_step(
            &mut job,
            ChangeStepKind::ManagedWrite,
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
        persist_transition(
            state,
            &mut job,
            ChangeStepKind::Readback,
            "readback_started",
            now,
            &observer,
        )?;

        let readback = adapter.verify();
        classify_job(&stored, &mut job, writer_result, readback, now);
        let terminal_reason = job
            .diagnostic_code
            .clone()
            .unwrap_or_else(|| "recovery_required".to_string());
        set_step(
            &mut job,
            ChangeStepKind::Finalize,
            ChangeStepStatus::Succeeded,
            "finalized",
        );
        persist_transition(
            state,
            &mut job,
            ChangeStepKind::Finalize,
            &terminal_reason,
            now,
            &observer,
        )?;
        execution.gate.mark_terminal();
        Ok(ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Admitted,
            job: Some(job),
            error_code: None,
        })
    }

    #[cfg(test)]
    pub fn get_job(
        state: &AppState,
        job_id: &str,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
        Self::get_job_with_observer(state, job_id, |_| {})
    }

    pub(crate) fn get_job_with_observer<O>(
        state: &AppState,
        job_id: &str,
        observer: O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(ChangeJobEventHint),
    {
        let mut job = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        normalize_job_projection(&mut job);
        if active_execution_gate(job_id).is_some() {
            return Ok(job);
        }
        if !job.needs_reconcile() {
            return Ok(job);
        }
        Self::reconcile_job_at_with_observer(
            state,
            &job.job_id,
            chrono::Utc::now().timestamp(),
            &observer,
        )
    }

    fn reconcile_job_at_with_observer<O>(
        state: &AppState,
        job_id: &str,
        now: i64,
        observer: &O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(ChangeJobEventHint),
    {
        if let Some(mut job) = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
        {
            normalize_job_projection(&mut job);
            if active_execution_gate(job_id).is_some() || !job.needs_reconcile() {
                return Ok(job);
            }
        }
        let _provider_guard = ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let mut job = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        if !job.needs_reconcile() {
            return Ok(job);
        }
        let stored = state
            .db
            .get_stored_change_plan(&job.plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::PlanNotFound)?;
        let managed_write_started = job
            .steps
            .iter()
            .find(|step| step.kind == ChangeStepKind::ManagedWrite)
            .is_some_and(|step| step.status != ChangeStepStatus::Pending);
        let readback = inspect_codex_switch(state, &stored.public.target_provider_id);
        classify_job(&stored, &mut job, Err(()), readback, now);
        if !managed_write_started
            && job.result_code == ChangeResultCode::WriterFailedBaselineRestored
        {
            job.status = ChangeJobStatus::Failed;
            job.result_code = ChangeResultCode::InterruptedBeforeWrite;
            job.recovery_state = RecoveryState::Succeeded;
            job.restart_requirement = RestartRequirement::NotRequired;
            job.diagnostic_code = Some("interrupted_before_write".to_string());
            set_step(
                &mut job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Skipped,
                "interrupted_before_write",
            );
        } else if managed_write_started
            && job.result_code == ChangeResultCode::WriterErrorTargetReached
        {
            job.result_code = ChangeResultCode::RecoveredTargetReached;
            job.restart_requirement = RestartRequirement::Recommended;
            job.diagnostic_code = Some("recovered_target_reached".to_string());
            set_step(
                &mut job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Succeeded,
                "target_reached_after_unknown_outcome",
            );
        }
        let finalize_status = if job.recovery_state == RecoveryState::RecoveryRequired {
            ChangeStepStatus::Failed
        } else {
            ChangeStepStatus::Succeeded
        };
        set_step(
            &mut job,
            ChangeStepKind::Finalize,
            finalize_status,
            "reconciled_without_replay",
        );
        let event = append_event(
            &mut job,
            ChangeStepKind::Finalize,
            "reconciled_without_replay",
            now,
        );
        normalize_job_projection(&mut job);
        state
            .db
            .save_change_job(&job, &event)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        observer(job_hint(&job));
        Ok(job)
    }

    pub fn cancel_job(
        state: &AppState,
        job_id: &str,
    ) -> Result<CancelChangeJobOutcome, ChangePlanErrorCode> {
        if let Some(gate) = active_execution_gate(job_id) {
            let code = gate.request_cancel();
            return Ok(CancelChangeJobOutcome {
                accepted: code == ChangeCancelCode::Accepted,
                code,
                job_id: job_id.to_string(),
            });
        }
        let job = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let code = match job {
            None => ChangeCancelCode::JobNotFound,
            Some(job) if job.status.is_terminal() => ChangeCancelCode::AlreadyTerminal,
            Some(_) => ChangeCancelCode::NotActive,
        };
        Ok(CancelChangeJobOutcome {
            accepted: false,
            code,
            job_id: job_id.to_string(),
        })
    }

    pub(crate) fn list_recoverable_jobs_with_observer<O>(
        state: &AppState,
        observer: O,
    ) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode>
    where
        O: Fn(ChangeJobEventHint),
    {
        let jobs = state
            .db
            .list_recoverable_change_jobs()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        jobs.into_iter()
            .map(|job| {
                Self::reconcile_job_at_with_observer(
                    state,
                    &job.job_id,
                    chrono::Utc::now().timestamp(),
                    &observer,
                )
            })
            .collect()
    }
}

impl CodexSwitchInspection {
    fn is_fully_target(&self) -> bool {
        self.db_current_provider_id.as_ref() == Some(&self.target.id)
            && self.device_current_provider_id.as_ref() == Some(&self.target.id)
            && self.live_projection_available
            && self.live_projection_digest == self.target_projection_digest
    }
}

pub(crate) fn inspect_codex_switch(
    state: &AppState,
    target_provider_id: &str,
) -> Result<CodexSwitchInspection, ChangePlanErrorCode> {
    if !is_safe_opaque_id(target_provider_id) {
        return Err(ChangePlanErrorCode::InvalidTarget);
    }
    let target = state
        .db
        .get_provider_by_id(target_provider_id, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?
        .ok_or(ChangePlanErrorCode::TargetNotFound)?;
    let db_current_provider_id = validate_optional_provider_id(
        state
            .db
            .get_current_provider(AppType::Codex.as_str())
            .map_err(|_| ChangePlanErrorCode::Internal)?,
    )?;
    let device_current_provider_id =
        validate_optional_provider_id(crate::settings::get_current_provider(&AppType::Codex))?;
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
        .map(|id| {
            state
                .db
                .get_provider_by_id(id, AppType::Codex.as_str())
                .map_err(|_| ChangePlanErrorCode::Internal)?
                .ok_or(ChangePlanErrorCode::Internal)
                .and_then(|provider| provider_definition_digest(&provider))
        })
        .transpose()?;
    let target_definition_digest = provider_definition_digest(&target)?;
    let environment = inspect_codex_switch_environment(state)
        .map_err(|_| ChangePlanErrorCode::SecretDependencyUnavailable)?;
    let live_projection = credential_neutral_codex_projection(&environment.live_settings)?;
    let live_projection_digest = digest_json("fyagent.change-plan.codex-live.v2", &live_projection);
    let live_projection_available = true;
    let target_live_projection =
        build_codex_switch_target_live_projection(state, &target, &environment)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
    let target_projection = credential_neutral_codex_projection(&target_live_projection)?;
    let target_projection_digest =
        digest_json("fyagent.change-plan.codex-live.v2", &target_projection);
    let baseline_digest = digest_serializable(
        "fyagent.change-plan.baseline.v2",
        &BaselineDigestInput {
            contract: CHANGE_PLAN_CONTRACT_VERSION,
            db_current_provider_id: &db_current_provider_id,
            device_current_provider_id: &device_current_provider_id,
            current_definition_digest: &current_definition_digest,
            target_provider_id,
            target_definition_digest: &target_definition_digest,
            live_projection_digest: &live_projection_digest,
            switch_mode_code: environment.mode_code(),
        },
    )?;
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        target,
        target_definition_digest,
        live_projection_digest,
        live_projection_available,
        target_projection_digest,
        baseline_digest,
        preserved_strict_login: environment.preserved_strict_login,
    })
}

fn prove_codex_target_credential_capability(
    inspection: &CodexSwitchInspection,
) -> SecretCapabilityResult {
    let provider = &inspection.target;
    if provider
        .meta
        .as_ref()
        .and_then(|meta| meta.auth_binding.as_ref())
        .is_some_and(|binding| binding.source == crate::provider::AuthBindingSource::ManagedAccount)
    {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    }
    if provider
        .meta
        .as_ref()
        .and_then(|meta| meta.provider_type.as_deref())
        .is_some()
    {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    }

    let Some(settings) = provider.settings_config.as_object() else {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    };
    let Some(auth) = settings.get("auth").filter(|value| value.is_object()) else {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    };
    let Some(config_text) = settings.get("config").and_then(Value::as_str) else {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    };
    if config_text.parse::<toml::Table>().is_err() {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    }

    let target_has_key =
        crate::codex_config::extract_codex_api_key(Some(auth), Some(config_text)).is_some();
    if crate::proxy::providers::is_codex_official_provider(provider) {
        let target_has_strict_login =
            crate::codex_config::codex_auth_has_credential_login_material(auth);
        let target_auth_would_replace_preserved_login =
            crate::codex_config::codex_auth_has_login_material(auth);
        return if target_has_key
            || target_has_strict_login
            || (!target_auth_would_replace_preserved_login && inspection.preserved_strict_login)
        {
            SecretCapabilityResult::NoNewCredentialMaterial
        } else {
            SecretCapabilityResult::SecretDependencyUnavailable
        };
    }

    if provider.category.as_deref() == Some("official") || provider.uses_managed_account_auth() {
        return SecretCapabilityResult::SecretDependencyUnavailable;
    }

    if target_has_key {
        SecretCapabilityResult::NoNewCredentialMaterial
    } else {
        SecretCapabilityResult::SecretDependencyUnavailable
    }
}

fn validate_optional_provider_id(
    value: Option<String>,
) -> Result<Option<String>, ChangePlanErrorCode> {
    if value.as_deref().is_some_and(|id| !is_safe_opaque_id(id)) {
        return Err(ChangePlanErrorCode::Internal);
    }
    Ok(value)
}

fn plan_approval_digest(
    operation: ChangeOperation,
    target_provider_id: &str,
    baseline_digest: &str,
    secret_capability: SecretCapabilityResult,
    adapter: &ChangeAdapterDescriptor,
) -> String {
    let semantic = serde_json::json!({
        "operation": operation,
        "targetProviderId": target_provider_id,
        "baselineDigest": baseline_digest,
        "secretCapability": secret_capability,
        "adapter": adapter,
        "contract": CHANGE_PLAN_CONTRACT_VERSION,
    });
    digest_json("fyagent.change-plan.plan.v2", &semantic)
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

fn append_event(
    job: &mut ChangeJobSnapshot,
    phase: ChangeStepKind,
    reason_code: &str,
    now: i64,
) -> ChangeJobEvent {
    job.revision += 1;
    job.event_seq += 1;
    job.updated_at = now;
    let event = ChangeJobEvent {
        sequence: job.event_seq,
        phase,
        reason_code: reason_code.to_string(),
        created_at: now,
    };
    job.events.push(event.clone());
    event
}

fn job_hint(job: &ChangeJobSnapshot) -> ChangeJobEventHint {
    ChangeJobEventHint {
        job_id: job.job_id.clone(),
        event_seq: job.event_seq,
    }
}

fn persist_transition<O>(
    state: &AppState,
    job: &mut ChangeJobSnapshot,
    phase: ChangeStepKind,
    reason_code: &str,
    now: i64,
    observer: &O,
) -> Result<(), ChangePlanErrorCode>
where
    O: Fn(ChangeJobEventHint),
{
    let event = append_event(job, phase, reason_code, now);
    normalize_job_projection(job);
    state
        .db
        .save_change_job(job, &event)
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    observer(job_hint(job));
    Ok(())
}

fn adapter_error_for_result(result: ChangeResultCode) -> Option<ChangeAdapterErrorCode> {
    match result {
        ChangeResultCode::WriterFailedBaselineRestored => Some(ChangeAdapterErrorCode::Permanent),
        ChangeResultCode::WriterErrorTargetReached => Some(ChangeAdapterErrorCode::Transient),
        ChangeResultCode::PostWriteMismatch => Some(ChangeAdapterErrorCode::VerifyFailed),
        ChangeResultCode::ReadbackUnavailable
        | ChangeResultCode::RecoveryRequired
        | ChangeResultCode::RecoveredTargetReached => Some(ChangeAdapterErrorCode::UnknownOutcome),
        ChangeResultCode::InterruptedBeforeWrite => Some(ChangeAdapterErrorCode::Transient),
        _ => None,
    }
}

fn normalize_job_projection(job: &mut ChangeJobSnapshot) {
    job.execution_id.clone_from(&job.job_id);
    job.idempotency_key.clone_from(&job.plan_id);
    if job.result_code == ChangeResultCode::CancelledBeforeWrite {
        job.status = ChangeJobStatus::Cancelled;
    }
    job.adapter_error_code = adapter_error_for_result(job.result_code);

    let succeeded_steps = job
        .steps
        .iter()
        .filter(|step| step.status == ChangeStepStatus::Succeeded)
        .map(|step| step.kind)
        .collect::<Vec<_>>();
    let compensated_steps = job
        .steps
        .iter()
        .filter(|step| step.status == ChangeStepStatus::Compensated)
        .map(|step| step.kind)
        .collect::<Vec<_>>();
    let managed_write_may_have_effect = job
        .steps
        .iter()
        .find(|step| step.kind == ChangeStepKind::ManagedWrite)
        .is_some_and(|step| {
            matches!(
                step.status,
                ChangeStepStatus::Running | ChangeStepStatus::Succeeded | ChangeStepStatus::Failed
            )
        });
    let readback_succeeded = job.steps.iter().any(|step| {
        step.kind == ChangeStepKind::Readback && step.status == ChangeStepStatus::Succeeded
    });
    let unverified_steps = if managed_write_may_have_effect && !readback_succeeded {
        vec![ChangeStepKind::ManagedWrite]
    } else {
        Vec::new()
    };
    let remaining_effects = if job.recovery_state == RecoveryState::Succeeded {
        Vec::new()
    } else {
        job.resources
            .iter()
            .filter(|resource| {
                matches!(
                    resource.status,
                    ChangeResourceStatus::Mismatched | ChangeResourceStatus::Unavailable
                )
            })
            .map(|resource| resource.kind)
            .collect::<Vec<_>>()
    };
    let mut manual_actions = Vec::new();
    if job.result_code == ChangeResultCode::ReadbackUnavailable {
        manual_actions.push(ChangeManualActionCode::RetryReadback);
    }
    if job.recovery_state == RecoveryState::RecoveryRequired || !remaining_effects.is_empty() {
        manual_actions.push(ChangeManualActionCode::ReviewConfiguration);
    }

    let needs_partial = matches!(
        job.status,
        ChangeJobStatus::Running | ChangeJobStatus::Warning | ChangeJobStatus::Failed
    ) || job.recovery_state == RecoveryState::RecoveryRequired;
    job.partial_result = needs_partial.then_some(ChangePartialResult {
        succeeded_steps,
        compensated_steps,
        unverified_steps,
        remaining_effects,
        manual_actions,
    });
}

fn classify_job(
    stored: &StoredChangePlan,
    job: &mut ChangeJobSnapshot,
    writer_result: Result<WriterReceipt, ()>,
    readback: Result<CodexSwitchInspection, ChangePlanErrorCode>,
    now: i64,
) {
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
    let baseline_db = readback.db_current_provider_id == stored.public.db_baseline_provider_id;
    let baseline_device =
        readback.device_current_provider_id == stored.public.device_baseline_provider_id;
    let baseline_live = readback.live_projection_available
        && readback.live_projection_digest == stored.live_baseline_digest;

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
        match writer_result {
            Ok(receipt) => {
                job.live_config_changed = receipt.live_config_changed;
                job.restart_requirement = if job.live_config_changed {
                    RestartRequirement::Recommended
                } else {
                    RestartRequirement::NotRequired
                };
                job.status = ChangeJobStatus::Succeeded;
                job.result_code = if job.live_config_changed {
                    ChangeResultCode::AppliedRestartRecommended
                } else {
                    ChangeResultCode::Applied
                };
            }
            Err(()) => {
                job.live_config_changed = false;
                job.restart_requirement = RestartRequirement::Recommended;
                job.status = ChangeJobStatus::Warning;
                job.result_code = ChangeResultCode::WriterErrorTargetReached;
            }
        }
        job.recovery_state = RecoveryState::NotNeeded;
        job.diagnostic_code = Some("target_readback_matched".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "target_matched",
        );
    } else if baseline_db && baseline_device && baseline_live && definition_target {
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::WriterFailedBaselineRestored;
        job.restart_requirement = RestartRequirement::NotRequired;
        job.recovery_state = RecoveryState::Succeeded;
        job.diagnostic_code = Some("baseline_restored".to_string());
        if job
            .steps
            .iter()
            .find(|step| step.kind == ChangeStepKind::ManagedWrite)
            .is_some_and(|step| step.status != ChangeStepStatus::Pending)
        {
            set_step(
                job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Compensated,
                "writer_owned_rollback_confirmed",
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::provider::{read_live_settings, write_live_with_common_config};
    use serde_json::json;
    use serial_test::serial;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Mutex};

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
                "config": format!(
                    "model_provider = \"custom\"\nmodel = \"{model}\"\n\
                     [model_providers.custom]\nname = \"Custom\"\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\n"
                )
            }),
            None,
        )
    }

    fn setup_switch_state() -> (
        tempfile::TempDir,
        TestHome,
        Arc<Database>,
        Arc<AppState>,
        Provider,
        Provider,
    ) {
        let home = tempfile::tempdir().expect("test home");
        let home_guard = TestHome::set(home.path());
        let db = Arc::new(Database::memory().expect("database"));
        db.create_change_plan_tables_for_tests().unwrap();
        let current = provider("current", "Current", "gpt-current");
        let target = provider("target", "Target", "gpt-target");
        db.save_provider(AppType::Codex.as_str(), &current).unwrap();
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        db.set_current_provider(AppType::Codex.as_str(), &current.id)
            .unwrap();
        crate::settings::set_current_provider(&AppType::Codex, Some(&current.id)).unwrap();
        write_live_with_common_config(db.as_ref(), &AppType::Codex, &current).unwrap();
        let state = Arc::new(AppState::new(db.clone()));
        (home, home_guard, db, state, current, target)
    }

    #[test]
    #[serial]
    fn plan_is_unique_stable_side_effect_free_and_secret_free() {
        let (home, _guard, db, state, current, target) = setup_switch_state();
        let before_live = read_live_settings(AppType::Codex).unwrap();
        let first = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let second = ChangePlanService::plan_codex_switch_at(&state, &target.id, 101).unwrap();
        assert_ne!(first.plan_id, second.plan_id);
        assert_eq!(first.plan_digest, second.plan_digest);
        assert_eq!(first.baseline_digest, second.baseline_digest);
        assert_eq!(first.adapter.adapter_id, "codex_provider_switch");
        assert_eq!(first.adapter.adapter_version, "2");
        assert_eq!(
            first.adapter.phases,
            vec![
                ChangeStepKind::Precheck,
                ChangeStepKind::Snapshot,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Readback,
                ChangeStepKind::Finalize,
            ]
        );
        assert_eq!(
            first.adapter.idempotency_scope,
            ChangeIdempotencyScope::Plan
        );
        assert_eq!(
            first.adapter.cancel_mode,
            ChangeCancelMode::BeforeManagedWrite
        );
        assert_eq!(
            first.adapter.compensation_mode,
            ChangeCompensationMode::WriterOwnedRollback
        );
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            Some(current.id)
        );
        assert_eq!(read_live_settings(AppType::Codex).unwrap(), before_live);
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());

        let conn = db.conn.lock().unwrap();
        let persisted: String = conn
            .query_row(
                "SELECT group_concat(quote(plan_id || operation || target_provider_id || target_provider_name || plan_digest || baseline_digest || coalesce(db_baseline_provider_id,'') || coalesce(device_baseline_provider_id,'') || target_definition_digest || live_baseline_digest || target_projection_digest || contract_digest || secret_capability)) FROM change_plans",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!persisted.contains("sentinel"));
        assert!(!persisted.contains("settingsConfig"));
        assert!(!persisted.contains(home.path().to_string_lossy().as_ref()));
    }

    #[test]
    #[serial]
    fn apply_uses_the_lock_held_provider_writer_once_without_reentrant_locking() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let writer_calls = AtomicUsize::new(0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |target_id| {
                writer_calls.fetch_add(1, Ordering::SeqCst);
                ProviderService::with_live_config_result(AppType::Codex, || {
                    ProviderService::switch_with_lock_held(&state, AppType::Codex, target_id)
                        .map(|_| true)
                })
                .map(|result| WriterReceipt {
                    live_config_changed: result.live_config_changed,
                })
            },
        )
        .unwrap();

        assert_eq!(writer_calls.load(Ordering::SeqCst), 1);
        assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Admitted);
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            Some(target.id)
        );
    }

    #[test]
    #[serial]
    fn quick_setup_target_projection_matches_targeted_live_patch_writer() {
        let (_home, _guard, db, state, current, _target) = setup_switch_state();
        let quick_setup = provider("fyagent-v2-quick-setup-codex", "FyAgent Codex", "gpt-quick");
        db.save_provider(AppType::Codex.as_str(), &quick_setup)
            .unwrap();

        let live_config = "# user-owned-comment\nmodel_provider = \"custom\"\nmodel = \"gpt-current\"\nreview_model = \"gpt-review\"\n\
             [model_providers.custom]\nname = \"Current\"\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\ncustom_user_field = \"keep-me\"\n\
             [features]\nplugins = true\n\n[mcp_servers.user_owned]\ncommand = \"echo\"\nargs = [\"keep\"]\n"
            .to_string();
        crate::codex_config::write_codex_live_atomic(
            &json!({"OPENAI_API_KEY": format!("sentinel-{}", current.id)}),
            Some(&live_config),
        )
        .unwrap();

        let plan = ChangePlanService::plan_codex_switch_at(&state, &quick_setup.id, 100).unwrap();
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |target_id| {
                ProviderService::with_live_config_result(AppType::Codex, || {
                    ProviderService::switch_with_lock_held(&state, AppType::Codex, target_id)
                        .map(|_| true)
                })
                .map(|result| WriterReceipt {
                    live_config_changed: result.live_config_changed,
                })
            },
        )
        .unwrap();
        let job = outcome.job.expect("admitted job");
        assert_eq!(job.status, ChangeJobStatus::Succeeded);
        assert_eq!(job.recovery_state, RecoveryState::NotNeeded);

        let live = crate::codex_config::read_codex_config_text().unwrap();
        assert!(live.contains("# user-owned-comment"));
        let parsed: toml::Value = toml::from_str(&live).unwrap();
        assert_eq!(parsed["review_model"].as_str(), Some("gpt-review"));
        assert_eq!(parsed["features"]["plugins"].as_bool(), Some(true));
        assert_eq!(
            parsed["mcp_servers"]["user_owned"]["command"].as_str(),
            Some("echo")
        );
        assert_eq!(
            parsed["model_providers"]["custom"]["custom_user_field"].as_str(),
            Some("keep-me")
        );
    }

    #[test]
    #[serial]
    fn credential_capability_accepts_only_extractable_unmanaged_target_material() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();

        let mut active_token = target.clone();
        active_token.id = "active-token".to_string();
        active_token.settings_config = json!({
            "auth": {},
            "config": "model_provider = \"active\"\nmodel = \"gpt-target\"\n[model_providers.active]\nname = \"Active\"\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\nexperimental_bearer_token = \"token-active\"\n"
        });
        db.save_provider(AppType::Codex.as_str(), &active_token)
            .unwrap();
        assert!(ChangePlanService::plan_codex_switch_at(&state, &active_token.id, 100).is_ok());

        let mut inactive_token = active_token.clone();
        inactive_token.id = "inactive-token".to_string();
        inactive_token.settings_config = json!({
            "auth": {},
            "config": "model_provider = \"active\"\nmodel = \"gpt-target\"\n[model_providers.active]\nname = \"Active\"\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\n[model_providers.inactive]\nexperimental_bearer_token = \"token-inactive\"\n"
        });
        db.save_provider(AppType::Codex.as_str(), &inactive_token)
            .unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &inactive_token.id, 101),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );

        let mut managed = target.clone();
        managed.id = "managed".to_string();
        managed.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            ..Default::default()
        });
        db.save_provider(AppType::Codex.as_str(), &managed).unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &managed.id, 102),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );

        let mut unknown = target.clone();
        unknown.id = "unknown-type".to_string();
        unknown.meta = Some(crate::provider::ProviderMeta {
            provider_type: Some("future_managed_type".to_string()),
            ..Default::default()
        });
        db.save_provider(AppType::Codex.as_str(), &unknown).unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &unknown.id, 103),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
    }

    #[test]
    #[serial]
    fn malformed_target_and_live_read_error_fail_closed_without_plan() {
        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        target.settings_config["config"] = Value::String("not = [valid".to_string());
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &target.id, 100),
            Err(ChangePlanErrorCode::Internal)
        );

        target.settings_config = provider("fixture", "Fixture", "gpt-target").settings_config;
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        std::fs::write(crate::codex_config::get_codex_auth_path(), b"{invalid-json")
            .expect("corrupt test-only live auth");
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &target.id, 101),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );

        let plans: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(plans, 0);
    }

    #[test]
    #[serial]
    fn apply_revalidates_credentials_before_consuming_plan_or_calling_writer() {
        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        target.settings_config["auth"] = json!({});
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        let calls = AtomicUsize::new(0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();

        assert_eq!(
            outcome.error_code,
            Some(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.get_stored_change_plan(&plan.plan_id)
                .unwrap()
                .unwrap()
                .public
                .status,
            ChangePlanStatus::Ready
        );
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn official_target_requires_own_material_or_strict_preserved_login() {
        let (_home, _guard, db, state, _current, _target) = setup_switch_state();
        let mut official = provider(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID,
            "Codex Official",
            "gpt-official",
        );
        official.category = Some("official".to_string());
        official.settings_config["auth"] = json!({});
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();

        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &official.id, 100),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );

        let live = read_live_settings(AppType::Codex).unwrap();
        crate::codex_config::write_codex_live_atomic(
            &json!({"tokens": {"access_token": "strict-login"}}),
            live.get("config").and_then(Value::as_str),
        )
        .unwrap();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &official.id, 101).unwrap();

        official.settings_config["auth"] =
            json!({"last_refresh": "metadata", "tokens": {"account_id": "metadata-only"}});
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &official.id, 102),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        let metadata_calls = AtomicUsize::new(0);
        let metadata_outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            103,
            |_| {
                metadata_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(
            metadata_outcome.error_code,
            Some(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        assert_eq!(metadata_calls.load(Ordering::SeqCst), 0);

        official.settings_config["auth"] = json!({});
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();
        crate::codex_config::write_codex_live_atomic(
            &json!({}),
            live.get("config").and_then(Value::as_str),
        )
        .unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            102,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(
            outcome.error_code,
            Some(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.get_stored_change_plan(&plan.plan_id)
                .unwrap()
                .unwrap()
                .public
                .status,
            ChangePlanStatus::Ready
        );

        official.settings_config["auth"] = json!({"tokens": {"refresh_token": "target-login"}});
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();
        assert!(ChangePlanService::plan_codex_switch_at(&state, &official.id, 200).is_ok());

        official.settings_config["auth"] = json!({"OPENAI_API_KEY": "target-key"});
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();
        assert!(ChangePlanService::plan_codex_switch_at(&state, &official.id, 201).is_ok());

        official.settings_config["auth"] = json!({});
        official.meta = Some(crate::provider::ProviderMeta {
            auth_binding: Some(crate::provider::AuthBinding {
                source: crate::provider::AuthBindingSource::ManagedAccount,
                auth_provider: Some("managed-codex".to_string()),
                account_id: Some("account".to_string()),
            }),
            ..Default::default()
        });
        db.save_provider(AppType::Codex.as_str(), &official)
            .unwrap();
        crate::codex_config::write_codex_live_atomic(
            &json!({"tokens": {"access_token": "strict-login"}}),
            live.get("config").and_then(Value::as_str),
        )
        .unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &official.id, 202),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
    }

    #[test]
    #[serial]
    fn takeover_mode_drift_is_stale_before_writer_and_does_not_consume_plan() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let live = read_live_settings(AppType::Codex).unwrap();
        futures::executor::block_on(db.save_live_backup(
            AppType::Codex.as_str(),
            &serde_json::to_string(&live).unwrap(),
        ))
        .unwrap();
        let calls = AtomicUsize::new(0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();

        assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            db.get_stored_change_plan(&plan.plan_id)
                .unwrap()
                .unwrap()
                .public
                .status,
            ChangePlanStatus::Ready
        );
    }

    #[tokio::test]
    #[serial]
    async fn takeover_plan_projection_matches_the_real_hot_switch_writer() {
        let (_home, _guard, db, state, current, target) = setup_switch_state();
        let live = read_live_settings(AppType::Codex).unwrap();
        db.save_live_backup(
            AppType::Codex.as_str(),
            &serde_json::to_string(&live).unwrap(),
        )
        .await
        .unwrap();
        state
            .proxy_service
            .sync_codex_live_from_provider_while_proxy_active(&current)
            .await
            .unwrap();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |target_id| {
                calls.fetch_add(1, Ordering::SeqCst);
                ProviderService::with_live_config_result(AppType::Codex, || {
                    ProviderService::switch_with_lock_held(&state, AppType::Codex, target_id)
                        .map(|_| true)
                })
                .map(|result| WriterReceipt {
                    live_config_changed: result.live_config_changed,
                })
            },
        )
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Admitted);
        assert_eq!(outcome.job.unwrap().status, ChangeJobStatus::Succeeded);
    }

    #[tokio::test]
    #[serial]
    async fn backup_only_plan_projection_matches_the_real_hot_switch_writer() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let live = read_live_settings(AppType::Codex).unwrap();
        db.save_live_backup(
            AppType::Codex.as_str(),
            &serde_json::to_string(&live).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            inspect_codex_switch_environment(&state)
                .unwrap()
                .mode_code(),
            "backup_only"
        );
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |target_id| {
                calls.fetch_add(1, Ordering::SeqCst);
                ProviderService::with_live_config_result(AppType::Codex, || {
                    ProviderService::switch_with_lock_held(&state, AppType::Codex, target_id)
                        .map(|_| true)
                })
                .map(|result| WriterReceipt {
                    live_config_changed: result.live_config_changed,
                })
            },
        )
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Admitted);
        assert_eq!(outcome.job.unwrap().status, ChangeJobStatus::Succeeded);
        let readback = inspect_codex_switch(&state, &target.id).unwrap();
        assert_eq!(
            readback.live_projection_digest,
            readback.target_projection_digest
        );
    }

    #[test]
    #[serial]
    fn secret_blocked_plan_and_apply_write_nothing_and_never_call_writer() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let mut blocked = target.clone();
        blocked.id = "blocked".to_string();
        blocked.settings_config["auth"] = json!({});
        db.save_provider(AppType::Codex.as_str(), &blocked).unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &blocked.id, 100),
            Err(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        db.conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE change_plans SET secret_capability='secret_dependency_unavailable' WHERE plan_id=?1",
                [&plan.plan_id],
            )
            .unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(
            outcome.error_code,
            Some(ChangePlanErrorCode::SecretDependencyUnavailable)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn persisted_plan_digest_remains_bound_to_the_current_adapter_descriptor() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let tampered_digest = "0".repeat(64);
        db.conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE change_plans SET plan_digest=?1 WHERE plan_id=?2",
                [&tampered_digest, &plan.plan_id],
            )
            .unwrap();

        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &tampered_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(db
            .get_change_job_by_plan_id(&plan.plan_id)
            .unwrap()
            .is_none());
    }

    #[test]
    #[serial]
    fn normal_apply_calls_writer_once_and_replay_stale_expired_call_zero_times() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                db.set_current_provider(AppType::Codex.as_str(), &target.id)
                    .unwrap();
                crate::settings::set_current_provider(&AppType::Codex, Some(&target.id)).unwrap();
                write_live_with_common_config(db.as_ref(), &AppType::Codex, &target).unwrap();
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: true,
                })
            },
        )
        .unwrap();
        let job = outcome.job.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(job.status, ChangeJobStatus::Succeeded);
        assert_eq!(job.result_code, ChangeResultCode::AppliedRestartRecommended);
        assert_eq!(job.usage_evidence, UsageEvidence::NotObserved);
        assert_eq!(job.events.len(), 6);
        assert!(job
            .events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence));

        let replay = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            102,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(replay.kind, ChangeApplyOutcomeKind::IdempotentReplay);
        assert_eq!(replay.error_code, None);
        assert_eq!(
            replay.job.as_ref().map(|job| job.job_id.as_str()),
            Some(job.job_id.as_str())
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[serial]
    fn concurrent_apply_admits_one_consumer_only() {
        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let state = state.clone();
            let barrier = barrier.clone();
            let calls = calls.clone();
            let plan_id = plan.plan_id.clone();
            let plan_digest = plan.plan_digest.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                ChangePlanService::apply_codex_switch_at_with_writer(
                    &state,
                    &plan_id,
                    &plan_digest,
                    101,
                    |_| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<WriterReceipt, _>(())
                    },
                )
                .unwrap()
            }));
        }
        barrier.wait();
        let outcomes: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| outcome.kind == ChangeApplyOutcomeKind::Admitted)
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| outcome.kind == ChangeApplyOutcomeKind::IdempotentReplay)
                .count(),
            1
        );
    }

    #[test]
    #[serial]
    fn stale_expired_and_invalid_requests_never_call_writer() {
        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        let calls = AtomicUsize::new(0);

        let stale = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        target.settings_config["config"] = Value::String(
            "model_provider = \"custom\"\nmodel = \"drifted\"\n[model_providers.custom]\nname=\"Custom\"\nbase_url=\"https://example.test/v1\"\nwire_api=\"responses\"\n".into(),
        );
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        let stale_outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &stale.plan_id,
            &stale.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(stale_outcome.error_code, Some(ChangePlanErrorCode::Stale));

        let expired = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
        let expired_outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &expired.plan_id,
            &expired.plan_digest,
            expired.expires_at,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(
            expired_outcome.error_code,
            Some(ChangePlanErrorCode::Expired)
        );

        let invalid = ChangePlanService::plan_codex_switch_at(&state, &target.id, 300).unwrap();
        let invalid_outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &invalid.plan_id,
            "wrong-digest",
            301,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(WriterReceipt {
                    live_config_changed: false,
                })
            },
        )
        .unwrap();
        assert_eq!(
            invalid_outcome.error_code,
            Some(ChangePlanErrorCode::InvalidDigest)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    #[serial]
    fn mixed_readback_requires_recovery_and_reconcile_never_replays_writer() {
        let (_home, _guard, db, state, current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);
        let mixed = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                db.set_current_provider(AppType::Codex.as_str(), &target.id)
                    .unwrap();
                Err::<WriterReceipt, _>(())
            },
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(mixed.status, ChangeJobStatus::Failed);
        assert_eq!(mixed.recovery_state, RecoveryState::RecoveryRequired);
        assert_eq!(mixed.result_code, ChangeResultCode::PostWriteMismatch);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(db.list_recoverable_change_jobs().unwrap().len(), 1);

        crate::settings::set_current_provider(&AppType::Codex, Some(&target.id)).unwrap();
        write_live_with_common_config(db.as_ref(), &AppType::Codex, &target).unwrap();
        let converged = ChangePlanService::get_job(&state, &mixed.job_id).unwrap();
        assert_eq!(converged.status, ChangeJobStatus::Warning);
        assert_eq!(
            converged.result_code,
            ChangeResultCode::RecoveredTargetReached
        );
        assert_eq!(converged.recovery_state, RecoveryState::NotNeeded);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "reconcile must not replay writer"
        );
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());

        let second = ChangePlanService::plan_codex_switch_at(&state, &current.id, 200).unwrap();
        let inspected = inspect_codex_switch(&state, &current.id).unwrap();
        db.admit_change_plan(
            &second.plan_id,
            &second.plan_digest,
            &inspected.baseline_digest,
            "interrupted-job",
            201,
        )
        .unwrap();
        let before = db.get_current_provider(AppType::Codex.as_str()).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let state = state.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                ChangePlanService::get_job(&state, "interrupted-job").unwrap()
            }));
        }
        barrier.wait();
        let reconciled = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            before
        );
        for snapshot in &reconciled {
            assert_eq!(
                snapshot
                    .steps
                    .iter()
                    .find(|step| step.kind == ChangeStepKind::Finalize)
                    .unwrap()
                    .code,
                "reconciled_without_replay"
            );
            assert_eq!(
                snapshot
                    .events
                    .iter()
                    .filter(|event| event.phase == ChangeStepKind::Finalize)
                    .count(),
                1
            );
        }
    }

    #[test]
    #[serial]
    fn cancellation_wins_only_before_managed_write_and_observer_reads_committed_snapshots() {
        {
            let (_home, _guard, db, state, _current, target) = setup_switch_state();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
            let writer_calls = AtomicUsize::new(0);
            let cancel_sent = AtomicBool::new(false);
            let cancel_code = Mutex::new(None);
            let hints = Mutex::new(Vec::new());

            let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
                &state,
                &plan.plan_id,
                &plan.plan_digest,
                101,
                |_| {
                    writer_calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(WriterReceipt {
                        live_config_changed: false,
                    })
                },
                |hint| {
                    let persisted = db
                        .get_change_job(&hint.job_id)
                        .unwrap()
                        .expect("observer hint must reference a committed snapshot");
                    assert_eq!(persisted.event_seq, hint.event_seq);
                    hints.lock().unwrap().push(hint.event_seq);

                    let snapshot_done = persisted.steps.iter().any(|step| {
                        step.kind == ChangeStepKind::Snapshot
                            && step.status == ChangeStepStatus::Succeeded
                    });
                    let write_pending = persisted.steps.iter().any(|step| {
                        step.kind == ChangeStepKind::ManagedWrite
                            && step.status == ChangeStepStatus::Pending
                    });
                    if snapshot_done && write_pending && !cancel_sent.swap(true, Ordering::SeqCst) {
                        let cancelled =
                            ChangePlanService::cancel_job(&state, &persisted.job_id).unwrap();
                        *cancel_code.lock().unwrap() = Some(cancelled.code);
                    }
                },
                None,
            )
            .unwrap();

            let job = outcome.job.expect("cancelled job");
            assert_eq!(writer_calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                *cancel_code.lock().unwrap(),
                Some(ChangeCancelCode::Accepted)
            );
            assert_eq!(job.status, ChangeJobStatus::Cancelled);
            assert_eq!(job.result_code, ChangeResultCode::CancelledBeforeWrite);
            assert!(job.partial_result.is_none());
            assert!(hints
                .lock()
                .unwrap()
                .windows(2)
                .all(|pair| pair[0] < pair[1]));

            let (stored_status, stored_result): (String, String) = db
                .conn
                .lock()
                .unwrap()
                .query_row(
                    "SELECT status, result_code FROM change_jobs WHERE job_id=?1",
                    [&job.job_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(stored_status, "failed");
            assert_eq!(stored_result, "cancelled_before_write");
            assert_eq!(
                db.get_change_job(&job.job_id).unwrap().unwrap().status,
                ChangeJobStatus::Cancelled
            );
        }

        {
            let (_home, _guard, db, state, _current, target) = setup_switch_state();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
            let writer_calls = AtomicUsize::new(0);
            let cancel_code = Mutex::new(None);

            let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
                &state,
                &plan.plan_id,
                &plan.plan_digest,
                201,
                |_| {
                    writer_calls.fetch_add(1, Ordering::SeqCst);
                    db.set_current_provider(AppType::Codex.as_str(), &target.id)
                        .unwrap();
                    crate::settings::set_current_provider(&AppType::Codex, Some(&target.id))
                        .unwrap();
                    write_live_with_common_config(db.as_ref(), &AppType::Codex, &target).unwrap();
                    Ok::<_, ()>(WriterReceipt {
                        live_config_changed: true,
                    })
                },
                |hint| {
                    let persisted = db.get_change_job(&hint.job_id).unwrap().unwrap();
                    let write_started = persisted.steps.iter().any(|step| {
                        step.kind == ChangeStepKind::ManagedWrite
                            && step.status == ChangeStepStatus::Running
                    });
                    if write_started && cancel_code.lock().unwrap().is_none() {
                        let cancelled =
                            ChangePlanService::cancel_job(&state, &persisted.job_id).unwrap();
                        *cancel_code.lock().unwrap() = Some(cancelled.code);
                    }
                },
                None,
            )
            .unwrap();

            assert_eq!(writer_calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                *cancel_code.lock().unwrap(),
                Some(ChangeCancelCode::CommitPointPassed)
            );
            assert_eq!(outcome.job.unwrap().status, ChangeJobStatus::Succeeded);
        }
    }

    #[test]
    #[serial]
    fn executor_fault_points_recover_by_readback_without_replaying_writer() {
        {
            let (_home, _guard, _db, state, _current, target) = setup_switch_state();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
            let writer_calls = AtomicUsize::new(0);
            let result = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
                &state,
                &plan.plan_id,
                &plan.plan_digest,
                101,
                |_| {
                    writer_calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(WriterReceipt {
                        live_config_changed: false,
                    })
                },
                |_| {},
                Some(ChangeFaultPoint::BeforeManagedWrite),
            );
            assert_eq!(result, Err(ChangePlanErrorCode::Internal));
            assert_eq!(writer_calls.load(Ordering::SeqCst), 0);

            let recovered = ChangePlanService::get_job(
                &state,
                &state
                    .db
                    .get_change_job_by_plan_id(&plan.plan_id)
                    .unwrap()
                    .unwrap()
                    .job_id,
            )
            .unwrap();
            assert_eq!(recovered.status, ChangeJobStatus::Failed);
            assert_eq!(
                recovered.result_code,
                ChangeResultCode::InterruptedBeforeWrite
            );
            assert_eq!(recovered.recovery_state, RecoveryState::Succeeded);
            assert_eq!(writer_calls.load(Ordering::SeqCst), 0);
            assert!(recovered.steps.iter().any(|step| {
                step.kind == ChangeStepKind::ManagedWrite
                    && step.status == ChangeStepStatus::Skipped
            }));
        }

        {
            let (_home, _guard, db, state, _current, target) = setup_switch_state();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
            let writer_calls = AtomicUsize::new(0);
            let result = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
                &state,
                &plan.plan_id,
                &plan.plan_digest,
                201,
                |_| {
                    writer_calls.fetch_add(1, Ordering::SeqCst);
                    db.set_current_provider(AppType::Codex.as_str(), &target.id)
                        .unwrap();
                    crate::settings::set_current_provider(&AppType::Codex, Some(&target.id))
                        .unwrap();
                    write_live_with_common_config(db.as_ref(), &AppType::Codex, &target).unwrap();
                    Ok::<_, ()>(WriterReceipt {
                        live_config_changed: true,
                    })
                },
                |_| {},
                Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord),
            );
            assert_eq!(result, Err(ChangePlanErrorCode::Internal));
            assert_eq!(writer_calls.load(Ordering::SeqCst), 1);

            let job_id = db
                .get_change_job_by_plan_id(&plan.plan_id)
                .unwrap()
                .unwrap()
                .job_id;
            let recovery_hints = Mutex::new(Vec::new());
            let recovered = ChangePlanService::get_job_with_observer(&state, &job_id, |hint| {
                let persisted = db
                    .get_change_job(&hint.job_id)
                    .unwrap()
                    .expect("recovery hint must reference a committed snapshot");
                assert_eq!(persisted.event_seq, hint.event_seq);
                recovery_hints.lock().unwrap().push(hint.event_seq);
            })
            .unwrap();
            assert_eq!(
                recovery_hints.lock().unwrap().as_slice(),
                &[recovered.event_seq]
            );
            assert_eq!(recovered.status, ChangeJobStatus::Warning);
            assert_eq!(
                recovered.result_code,
                ChangeResultCode::RecoveredTargetReached
            );
            assert_eq!(recovered.recovery_state, RecoveryState::NotNeeded);
            assert_eq!(
                recovered.adapter_error_code,
                Some(ChangeAdapterErrorCode::UnknownOutcome)
            );
            assert_eq!(
                recovered.restart_requirement,
                RestartRequirement::Recommended
            );
            assert!(recovered.steps.iter().any(|step| {
                step.kind == ChangeStepKind::ManagedWrite
                    && step.status == ChangeStepStatus::Succeeded
                    && step.code == "target_reached_after_unknown_outcome"
            }));
            assert!(recovered.partial_result.as_ref().is_some_and(|partial| {
                partial.unverified_steps.is_empty() && partial.remaining_effects.is_empty()
            }));
            assert_eq!(writer_calls.load(Ordering::SeqCst), 1);
            let reread = ChangePlanService::get_job(&state, &job_id).unwrap();
            assert_eq!(reread.event_seq, recovered.event_seq);
            assert_eq!(writer_calls.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn partial_projection_distinguishes_compensated_and_unverified_effects() {
        let mut restored = ChangeJobSnapshot::planned(
            "restored-job".into(),
            "restored-plan".into(),
            "target".into(),
            100,
        );
        restored.status = ChangeJobStatus::Failed;
        restored.result_code = ChangeResultCode::WriterFailedBaselineRestored;
        restored.recovery_state = RecoveryState::Succeeded;
        set_step(
            &mut restored,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Compensated,
            "writer_owned_rollback_confirmed",
        );
        set_step(
            &mut restored,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "baseline_restored",
        );
        for resource in &mut restored.resources {
            resource.status = ChangeResourceStatus::Mismatched;
        }
        normalize_job_projection(&mut restored);
        let partial = restored.partial_result.expect("restored partial truth");
        assert_eq!(
            partial.compensated_steps,
            vec![ChangeStepKind::ManagedWrite]
        );
        assert!(partial.unverified_steps.is_empty());
        assert!(partial.remaining_effects.is_empty());
        assert!(partial.manual_actions.is_empty());

        let mut unknown = ChangeJobSnapshot::planned(
            "unknown-job".into(),
            "unknown-plan".into(),
            "target".into(),
            100,
        );
        unknown.status = ChangeJobStatus::Failed;
        unknown.result_code = ChangeResultCode::ReadbackUnavailable;
        unknown.recovery_state = RecoveryState::RecoveryRequired;
        set_step(
            &mut unknown,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Running,
            "managed_write_started",
        );
        set_step(
            &mut unknown,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "readback_unavailable",
        );
        unknown.resources[0].status = ChangeResourceStatus::Unavailable;
        normalize_job_projection(&mut unknown);
        let partial = unknown.partial_result.expect("unknown partial truth");
        assert_eq!(partial.unverified_steps, vec![ChangeStepKind::ManagedWrite]);
        assert_eq!(
            partial.manual_actions,
            vec![
                ChangeManualActionCode::RetryReadback,
                ChangeManualActionCode::ReviewConfiguration,
            ]
        );
    }
}
