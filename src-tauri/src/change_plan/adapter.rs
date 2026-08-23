use super::{
    inspect_codex_switch, safe_provider_display_name, ChangeAdapterDescriptor, ChangeCancelMode,
    ChangeCompensationMode, ChangeFaultPoint, ChangeIdempotencyScope, ChangeOperation,
    ChangePlanErrorCode, ChangePlanRisk, ChangeResourceKind, ChangeStepKind, CodexSwitchInspection,
    PrivateProjectionProof, RestartRequirement,
};
use crate::store::AppState;

pub(super) struct ChangeAdapterPlanFields {
    pub target_provider_name: String,
    pub current_provider_code: String,
    pub target_provider_code: String,
    pub restart_expectation: RestartRequirement,
    pub risks: Vec<ChangePlanRisk>,
    pub evidence_note: String,
}

/// A closed operation implementation. The executor never receives a shell
/// command or dynamic write target; every side effect comes from one registered
/// adapter implementation and its compile-time types.
pub(super) trait ChangeAdapter {
    type Inspection;
    type Snapshot;
    type WriteReceipt;

    fn descriptor(&self) -> ChangeAdapterDescriptor;
    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode>;
    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields;
    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode>;
    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot;
    fn managed_write(&mut self) -> Result<Self::WriteReceipt, ()>;
    fn readback(&self) -> Result<Self::Inspection, ChangePlanErrorCode>;
    fn compensation_capability(&self) -> ChangeCompensationMode;
}

pub(super) fn descriptor_for_operation(operation: ChangeOperation) -> ChangeAdapterDescriptor {
    match operation {
        ChangeOperation::CodexProviderSwitch => ChangeAdapterDescriptor {
            adapter_id: "codex_provider_switch".to_string(),
            adapter_version: "1".to_string(),
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
        },
    }
}

pub(super) struct CodexProviderSwitchAdapter<'a> {
    state: &'a AppState,
    target_provider_id: &'a str,
    proof_id: &'a str,
    writer: Option<Box<dyn FnOnce() -> Result<bool, ()> + 'a>>,
}

impl<'a> CodexProviderSwitchAdapter<'a> {
    pub(super) fn for_plan(
        state: &'a AppState,
        target_provider_id: &'a str,
        proof_id: &'a str,
    ) -> Self {
        Self {
            state,
            target_provider_id,
            proof_id,
            writer: None,
        }
    }

    pub(super) fn for_execution<F>(
        state: &'a AppState,
        target_provider_id: &'a str,
        proof_id: &'a str,
        writer: F,
    ) -> Self
    where
        F: FnOnce() -> Result<bool, ()> + 'a,
    {
        Self {
            state,
            target_provider_id,
            proof_id,
            writer: Some(Box::new(writer)),
        }
    }
}

impl ChangeAdapter for CodexProviderSwitchAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = PrivateProjectionProof;
    type WriteReceipt = bool;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        descriptor_for_operation(ChangeOperation::CodexProviderSwitch)
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        inspect_codex_switch(self.state, self.target_provider_id, self.proof_id)
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        ChangeAdapterPlanFields {
            target_provider_name: safe_provider_display_name(&inspection.target),
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
        }
    }

    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot {
        inspection.private_proof.clone()
    }

    fn managed_write(&mut self) -> Result<Self::WriteReceipt, ()> {
        self.writer.take().ok_or(())?()
    }

    fn readback(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        ChangeCompensationMode::WriterOwnedRollback
    }
}
