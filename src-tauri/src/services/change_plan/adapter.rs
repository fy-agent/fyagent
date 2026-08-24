use crate::app_config::AppType;
use crate::provider::Provider;
use crate::services::workbuddy::types::{SaveWorkBuddyModelsRequest, WorkBuddyConfigFormat};
use crate::store::AppState;

use super::domain::{
    resources_for_operation, ChangeAdapterDescriptor, ChangeCancelMode, ChangeCompensationMode,
    ChangeFaultPoint, ChangeIdempotencyScope, ChangeOperation, ChangePlanErrorCode, ChangePlanRisk,
    ChangeResourceKind, ChangeStepKind, RestartRequirement, WriterReceipt,
};
use super::sanitize::sanitize_display_name;
use super::service::{
    inspect_codex_intended_provider, inspect_codex_switch, inspect_workbuddy_save,
    CodexSwitchInspection, WorkBuddySaveInspection,
};

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
    let operation_type = operation;
    let adapter_id = match operation {
        ChangeOperation::CodexProviderSwitch => "codex_provider_switch",
        ChangeOperation::CodexProviderUpsertAndSwitch => "codex_provider_upsert_and_switch",
        ChangeOperation::WorkbuddyModelsSave => "workbuddy_models_save",
    };
    let (read_set, write_set) = match operation {
        ChangeOperation::CodexProviderSwitch | ChangeOperation::CodexProviderUpsertAndSwitch => (
            resources_for_operation(operation),
            vec![
                ChangeResourceKind::ProviderDbCurrent,
                ChangeResourceKind::DeviceCurrent,
                ChangeResourceKind::CodexLiveProjection,
            ],
        ),
        ChangeOperation::WorkbuddyModelsSave => {
            let resources = resources_for_operation(operation);
            (resources.clone(), resources)
        }
    };
    ChangeAdapterDescriptor {
        adapter_id: adapter_id.to_string(),
        adapter_version: "1".to_string(),
        operation_type,
        phases: vec![
            ChangeStepKind::Precheck,
            ChangeStepKind::Snapshot,
            ChangeStepKind::ManagedWrite,
            ChangeStepKind::Readback,
            ChangeStepKind::Finalize,
        ],
        read_set,
        write_set,
        idempotency_scope: ChangeIdempotencyScope::Plan,
        cancel_mode: ChangeCancelMode::BeforeManagedWrite,
        compensation_mode: ChangeCompensationMode::WriterOwnedRollback,
        fault_points: vec![
            ChangeFaultPoint::BeforeManagedWrite,
            ChangeFaultPoint::AfterManagedWriteBeforeRecord,
        ],
    }
}

type CodexSwitchWriter<'a> = Box<dyn FnOnce(&str) -> Result<WriterReceipt, ()> + 'a>;
type CodexUpsertWriter<'a> = Box<dyn FnOnce(Provider) -> Result<WriterReceipt, ()> + 'a>;
type WorkBuddySaveWriter<'a> =
    Box<dyn FnOnce(SaveWorkBuddyModelsRequest) -> Result<WriterReceipt, ()> + 'a>;

pub(super) struct CodexProviderSwitchAdapter<'a> {
    state: &'a AppState,
    target_provider_id: &'a str,
    writer: Option<CodexSwitchWriter<'a>>,
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
        switch_plan_fields(inspection)
    }

    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot {
        snapshot_from_inspection(inspection)
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

pub(super) struct CodexProviderUpsertAdapter<'a> {
    state: &'a AppState,
    provider: Provider,
    existing_reserved_row: bool,
    writer: Option<CodexUpsertWriter<'a>>,
}

impl<'a> CodexProviderUpsertAdapter<'a> {
    pub(super) fn for_plan(
        state: &'a AppState,
        provider: Provider,
        existing_reserved_row: bool,
    ) -> Self {
        Self {
            state,
            provider,
            existing_reserved_row,
            writer: None,
        }
    }

    pub(super) fn for_execution<F>(
        state: &'a AppState,
        provider: Provider,
        existing_reserved_row: bool,
        writer: F,
    ) -> Self
    where
        F: FnOnce(Provider) -> Result<WriterReceipt, ()> + 'a,
    {
        Self {
            state,
            provider,
            existing_reserved_row,
            writer: Some(Box::new(writer)),
        }
    }
}

impl ChangeAdapter for CodexProviderUpsertAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = CodexSwitchSnapshot;
    type WriteReceipt = WriterReceipt;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        descriptor_for_operation(ChangeOperation::CodexProviderUpsertAndSwitch)
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        inspect_codex_intended_provider(self.state, self.provider.clone())
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        let mut risks = vec![
            ChangePlanRisk {
                code: "local_configuration_write".to_string(),
                severity: "notice".to_string(),
            },
            ChangePlanRisk {
                code: "save_provider_then_set_current".to_string(),
                severity: "notice".to_string(),
            },
        ];
        let proxy_takeover_active = self
            .state
            .proxy_service
            .detect_takeover_in_live_config_for_app(&AppType::Codex);
        for code in crate::codex_config::codex_provider_save_warning_codes(
            &self.provider,
            proxy_takeover_active,
        ) {
            risks.push(ChangePlanRisk {
                code,
                severity: "warning".to_string(),
            });
        }
        ChangeAdapterPlanFields {
            target_provider_name: sanitize_display_name(&inspection.target.name),
            current_provider_code: current_provider_code(
                &inspection.db_current_provider_id,
                &inspection.device_current_provider_id,
            ),
            target_provider_code: if self.existing_reserved_row {
                "quick_setup_update"
            } else {
                "quick_setup_create"
            }
            .to_string(),
            restart_expectation: RestartRequirement::Recommended,
            risks,
            evidence_note: "usage_not_observed".to_string(),
        }
    }

    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot {
        snapshot_from_inspection(inspection)
    }

    fn managed_write(&mut self) -> Result<WriterReceipt, ()> {
        self.writer.take().ok_or(())?(self.provider.clone())
    }

    fn verify(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        ChangeCompensationMode::WriterOwnedRollback
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkBuddySaveSnapshot {
    pub config_digest: String,
    pub backup_digest: String,
    pub revision: Option<String>,
}

pub(super) struct WorkBuddySaveAdapter<'a> {
    request: SaveWorkBuddyModelsRequest,
    writer: Option<WorkBuddySaveWriter<'a>>,
}

impl<'a> WorkBuddySaveAdapter<'a> {
    pub(super) fn for_plan(request: SaveWorkBuddyModelsRequest) -> Self {
        Self {
            request,
            writer: None,
        }
    }

    pub(super) fn for_execution<F>(request: SaveWorkBuddyModelsRequest, writer: F) -> Self
    where
        F: FnOnce(SaveWorkBuddyModelsRequest) -> Result<WriterReceipt, ()> + 'a,
    {
        Self {
            request,
            writer: Some(Box::new(writer)),
        }
    }
}

impl ChangeAdapter for WorkBuddySaveAdapter<'_> {
    type Inspection = WorkBuddySaveInspection;
    type Snapshot = WorkBuddySaveSnapshot;
    type WriteReceipt = WriterReceipt;

    fn descriptor(&self) -> ChangeAdapterDescriptor {
        descriptor_for_operation(ChangeOperation::WorkbuddyModelsSave)
    }

    fn inspect(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        inspect_workbuddy_save(&self.request)
    }

    fn plan(&self, inspection: &Self::Inspection) -> ChangeAdapterPlanFields {
        let mut risks = vec![ChangePlanRisk {
            code: "local_configuration_write".to_string(),
            severity: "notice".to_string(),
        }];
        if !inspection.existing_update_ids.is_empty() {
            risks.push(ChangePlanRisk {
                code: "existing_model_ids_will_be_updated".to_string(),
                severity: "warning".to_string(),
            });
        }
        ChangeAdapterPlanFields {
            target_provider_name: workbuddy_display_name(&inspection.canonical_base_url),
            current_provider_code: workbuddy_format_code(inspection.format).to_string(),
            target_provider_code: if inspection.format == WorkBuddyConfigFormat::Missing {
                "object_root".to_string()
            } else {
                workbuddy_format_code(inspection.format).to_string()
            },
            restart_expectation: RestartRequirement::NotRequired,
            risks,
            evidence_note: "usage_not_observed".to_string(),
        }
    }

    fn precheck(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn snapshot(&self, inspection: &Self::Inspection) -> Self::Snapshot {
        WorkBuddySaveSnapshot {
            config_digest: inspection.config_digest.clone(),
            backup_digest: inspection.backup_digest.clone(),
            revision: inspection.revision.clone(),
        }
    }

    fn managed_write(&mut self) -> Result<WriterReceipt, ()> {
        self.writer.take().ok_or(())?(self.request.clone())
    }

    fn verify(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        self.inspect()
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        ChangeCompensationMode::WriterOwnedRollback
    }
}

pub(super) enum CodexExecutionAdapter<'a> {
    Switch(CodexProviderSwitchAdapter<'a>),
    Upsert(Box<CodexProviderUpsertAdapter<'a>>),
}

impl ChangeAdapter for CodexExecutionAdapter<'_> {
    type Inspection = CodexSwitchInspection;
    type Snapshot = CodexSwitchSnapshot;
    type WriteReceipt = WriterReceipt;

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

    fn verify(&self) -> Result<Self::Inspection, ChangePlanErrorCode> {
        match self {
            Self::Switch(adapter) => adapter.verify(),
            Self::Upsert(adapter) => adapter.verify(),
        }
    }

    fn compensation_capability(&self) -> ChangeCompensationMode {
        match self {
            Self::Switch(adapter) => adapter.compensation_capability(),
            Self::Upsert(adapter) => adapter.compensation_capability(),
        }
    }
}

fn switch_plan_fields(inspection: &CodexSwitchInspection) -> ChangeAdapterPlanFields {
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

fn snapshot_from_inspection(inspection: &CodexSwitchInspection) -> CodexSwitchSnapshot {
    CodexSwitchSnapshot {
        baseline_digest: inspection.baseline_digest.clone(),
        target_definition_digest: inspection.target_definition_digest.clone(),
        target_projection_digest: inspection.target_projection_digest.clone(),
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

fn workbuddy_format_code(format: WorkBuddyConfigFormat) -> &'static str {
    match format {
        WorkBuddyConfigFormat::Missing => "missing",
        WorkBuddyConfigFormat::LegacyArray => "legacy_array",
        WorkBuddyConfigFormat::ObjectRoot => "object_root",
    }
}

fn workbuddy_display_name(canonical_base_url: &str) -> String {
    let trimmed = canonical_base_url.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return "WorkBuddy".to_string();
    }
    trimmed.chars().take(80).collect()
}
