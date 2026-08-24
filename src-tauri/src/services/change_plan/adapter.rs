use crate::store::AppState;

use super::domain::{
    ChangeAdapterDescriptor, ChangeCancelMode, ChangeCompensationMode, ChangeFaultPoint,
    ChangeIdempotencyScope, ChangeOperation, ChangePlanErrorCode, ChangePlanRisk,
    ChangeResourceKind, ChangeStepKind, RestartRequirement, WriterReceipt,
};
use super::sanitize::sanitize_display_name;
use super::service::{inspect_codex_switch, CodexSwitchInspection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexSwitchSnapshot {
    pub baseline_digest: String,
    pub target_definition_digest: String,
    pub target_projection_digest: String,
}

pub(super) struct ChangeAdapterPlanFields {
    pub target_provider_name: String,
    pub current_provider_code: String,
    pub target_provider_code: String,
    pub restart_expectation: RestartRequirement,
    pub risks: Vec<ChangePlanRisk>,
    pub evidence_note: String,
}

/// Closed compile-time execution contract. Implementations cannot supply an
/// arbitrary command, path, or write target; all side effects stay behind a
/// registered operation and its established writer.
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
    fn verify(&self) -> Result<Self::Inspection, ChangePlanErrorCode>;
    fn compensation_capability(&self) -> ChangeCompensationMode;
}

pub(crate) fn descriptor_for_operation(operation: ChangeOperation) -> ChangeAdapterDescriptor {
    match operation {
        ChangeOperation::CodexProviderSwitch => ChangeAdapterDescriptor {
            adapter_id: "codex_provider_switch".to_string(),
            adapter_version: "2".to_string(),
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

type CodexWriter<'a> = Box<dyn FnOnce(&str) -> Result<WriterReceipt, ()> + 'a>;

pub(super) struct CodexProviderSwitchAdapter<'a> {
    state: &'a AppState,
    target_provider_id: &'a str,
    writer: Option<CodexWriter<'a>>,
}

impl<'a> CodexProviderSwitchAdapter<'a> {
    pub(super) fn for_plan(state: &'a AppState, target_provider_id: &'a str) -> Self {
        Self {
            state,
            target_provider_id,
            writer: None,
        }
    }

    pub(super) fn for_execution<F>(
        state: &'a AppState,
        target_provider_id: &'a str,
        writer: F,
    ) -> Self
    where
        F: FnOnce(&str) -> Result<WriterReceipt, ()> + 'a,
    {
        Self {
            state,
            target_provider_id,
            writer: Some(Box::new(writer)),
        }
    }
}

impl ChangeAdapter for CodexProviderSwitchAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = CodexSwitchSnapshot;
    type WriteReceipt = WriterReceipt;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        descriptor_for_operation(ChangeOperation::CodexProviderSwitch)
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        inspect_codex_switch(self.state, self.target_provider_id)
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        ChangeAdapterPlanFields {
            target_provider_name: sanitize_display_name(&inspection.target.name),
            current_provider_code: current_provider_code(
                &inspection.db_current_provider_id,
                &inspection.device_current_provider_id,
            ),
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
        CodexSwitchSnapshot {
            baseline_digest: inspection.baseline_digest.clone(),
            target_definition_digest: inspection.target_definition_digest.clone(),
            target_projection_digest: inspection.target_projection_digest.clone(),
        }
    }

    fn managed_write(&mut self) -> Result<WriterReceipt, ()> {
        self.writer.take().ok_or(())?(self.target_provider_id)
    }

    fn verify(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        ChangeCompensationMode::WriterOwnedRollback
    }
}

fn current_provider_code(db: &Option<String>, device: &Option<String>) -> String {
    match (db, device) {
        (None, None) => "current_unconfigured",
        (Some(db), Some(device)) if db == device => "current_configured",
        _ => "current_mixed",
    }
    .to_string()
}
