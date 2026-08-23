use super::{
    inspect_codex_switch, inspect_codex_upsert_precheck, inspect_codex_upsert_readback,
    safe_provider_display_name, ChangeAdapterDescriptor, ChangeCancelMode, ChangeCompensationMode,
    ChangeFaultPoint, ChangeIdempotencyScope, ChangeOperation, ChangePlanErrorCode, ChangePlanRisk,
    ChangeResourceKind, ChangeStepKind, CodexSwitchInspection, PrivateCodexCredentialPlan,
    PrivateProjectionProof, RestartRequirement,
};
use crate::store::AppState;
use std::sync::Arc;

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
        ChangeOperation::CodexProviderUpsertAndSwitch => ChangeAdapterDescriptor {
            adapter_id: "codex_provider_upsert_switch".to_string(),
            adapter_version: "1".to_string(),
            operation_type: ChangeOperation::CodexProviderUpsertAndSwitch,
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
                ChangeResourceKind::TargetDefinition,
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

pub(super) struct CodexProviderUpsertAdapter<'a> {
    state: &'a AppState,
    proof_id: &'a str,
    credential: Arc<PrivateCodexCredentialPlan>,
    writer: Option<Box<dyn FnOnce() -> Result<bool, ()> + 'a>>,
}

impl<'a> CodexProviderUpsertAdapter<'a> {
    pub(super) fn for_plan(
        state: &'a AppState,
        proof_id: &'a str,
        credential: Arc<PrivateCodexCredentialPlan>,
    ) -> Self {
        Self {
            state,
            proof_id,
            credential,
            writer: None,
        }
    }

    pub(super) fn for_execution<F>(
        state: &'a AppState,
        proof_id: &'a str,
        credential: Arc<PrivateCodexCredentialPlan>,
        writer: F,
    ) -> Self
    where
        F: FnOnce() -> Result<bool, ()> + 'a,
    {
        Self {
            state,
            proof_id,
            credential,
            writer: Some(Box::new(writer)),
        }
    }
}

impl ChangeAdapter for CodexProviderUpsertAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = PrivateProjectionProof;
    type WriteReceipt = bool;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        descriptor_for_operation(ChangeOperation::CodexProviderUpsertAndSwitch)
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        inspect_codex_upsert_precheck(self.state, &self.credential, self.proof_id)
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        ChangeAdapterPlanFields {
            target_provider_name: safe_provider_display_name(&inspection.target),
            current_provider_code: if inspection.effective_current_provider_id.is_some() {
                "current_configured".to_string()
            } else {
                "current_unconfigured".to_string()
            },
            target_provider_code: "provider_upsert".to_string(),
            restart_expectation: RestartRequirement::Recommended,
            risks: vec![
                ChangePlanRisk {
                    code: "os_keyring_write".to_string(),
                    severity: "notice".to_string(),
                },
                ChangePlanRisk {
                    code: "local_configuration_write".to_string(),
                    severity: "notice".to_string(),
                },
            ],
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
        inspect_codex_upsert_readback(self.state, &self.credential, self.proof_id)
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        ChangeCompensationMode::WriterOwnedRollback
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

pub(super) enum RegisteredCodexAdapter<'a> {
    Switch(CodexProviderSwitchAdapter<'a>),
    Upsert(CodexProviderUpsertAdapter<'a>),
}

impl<'a> RegisteredCodexAdapter<'a> {
    pub(super) fn for_execution<F>(
        state: &'a AppState,
        operation: ChangeOperation,
        target_provider_id: &'a str,
        proof_id: &'a str,
        credential: Option<Arc<PrivateCodexCredentialPlan>>,
        writer: F,
    ) -> Result<Self, ChangePlanErrorCode>
    where
        F: FnOnce() -> Result<bool, ()> + 'a,
    {
        Ok(match operation {
            ChangeOperation::CodexProviderSwitch => {
                Self::Switch(CodexProviderSwitchAdapter::for_execution(
                    state,
                    target_provider_id,
                    proof_id,
                    writer,
                ))
            }
            ChangeOperation::CodexProviderUpsertAndSwitch => {
                Self::Upsert(CodexProviderUpsertAdapter::for_execution(
                    state,
                    proof_id,
                    credential.ok_or(ChangePlanErrorCode::Stale)?,
                    writer,
                ))
            }
        })
    }

    pub(super) fn for_readback(
        state: &'a AppState,
        operation: ChangeOperation,
        target_provider_id: &'a str,
        proof_id: &'a str,
        credential: Option<Arc<PrivateCodexCredentialPlan>>,
    ) -> Result<Self, ChangePlanErrorCode> {
        Ok(match operation {
            ChangeOperation::CodexProviderSwitch => Self::Switch(
                CodexProviderSwitchAdapter::for_plan(state, target_provider_id, proof_id),
            ),
            ChangeOperation::CodexProviderUpsertAndSwitch => {
                Self::Upsert(CodexProviderUpsertAdapter::for_plan(
                    state,
                    proof_id,
                    credential.ok_or(ChangePlanErrorCode::Stale)?,
                ))
            }
        })
    }
}

impl ChangeAdapter for RegisteredCodexAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = PrivateProjectionProof;
    type WriteReceipt = bool;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        match self {
            Self::Switch(adapter) => adapter.descriptor(),
            Self::Upsert(adapter) => adapter.descriptor(),
        }
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        match self {
            Self::Switch(adapter) => adapter.inspect(),
            Self::Upsert(adapter) => adapter.inspect(),
        }
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        match self {
            Self::Switch(adapter) => adapter.plan(inspection),
            Self::Upsert(adapter) => adapter.plan(inspection),
        }
    }

    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        match self {
            Self::Switch(adapter) => adapter.precheck(),
            Self::Upsert(adapter) => adapter.precheck(),
        }
    }

    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot {
        match self {
            Self::Switch(adapter) => adapter.snapshot(inspection),
            Self::Upsert(adapter) => adapter.snapshot(inspection),
        }
    }

    fn managed_write(&mut self) -> Result<Self::WriteReceipt, ()> {
        match self {
            Self::Switch(adapter) => adapter.managed_write(),
            Self::Upsert(adapter) => adapter.managed_write(),
        }
    }

    fn readback(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        match self {
            Self::Switch(adapter) => adapter.readback(),
            Self::Upsert(adapter) => adapter.readback(),
        }
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        match self {
            Self::Switch(adapter) => adapter.compensation_capability(),
            Self::Upsert(adapter) => adapter.compensation_capability(),
        }
    }
}
