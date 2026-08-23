use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::services::provider::{build_effective_settings_with_common_config, read_live_settings};
use crate::store::AppState;
use crate::AppError;

pub(crate) const CHANGE_PLAN_CONTRACT_VERSION: &str = "fyagent-change-plan-v1-schema20";
pub(crate) const CHANGE_PLAN_TTL_SECONDS: i64 = 15 * 60;
type HmacSha256 = Hmac<Sha256>;

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
string_enum!(ChangeActorType { DirectUser });
string_enum!(ChangePlanStatus { Ready, Consumed });
string_enum!(ChangeJobStatus {
    Planned,
    Running,
    Succeeded,
    Warning,
    Failed,
    Cancelled
});
string_enum!(ChangeStepKind {
    Precheck,
    Apply,
    Readback,
    Reconcile
});
string_enum!(ChangeStepStatus {
    NotStarted,
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
    CancelledBeforeWrite,
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
    BaselineUnavailable,
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
pub struct ChangePlanRisk {
    pub code: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeActor {
    #[serde(rename = "type")]
    pub actor_type: ChangeActorType,
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
    pub actor: ChangeActor,
    pub source_version: String,
    pub revision: i64,
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
                status: ChangeStepStatus::NotStarted,
                code: "not_started".to_string(),
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
    pub proof_id: String,
    pub process_epoch_id: String,
    pub current_provider_id: Option<String>,
    pub contract_digest: String,
}

#[derive(Debug, Clone, Serialize)]
struct BaselineDigestInput<'a> {
    contract: &'a str,
    proof_id: &'a str,
    process_epoch_id: &'a str,
    db_current_provider_id: &'a Option<String>,
    device_current_provider_id: &'a Option<String>,
    effective_current_provider_id: &'a Option<String>,
    target_provider_id: &'a str,
    live_projection_state: CodexLiveBaselineState,
    proxy_takeover_active: bool,
}

#[derive(Serialize)]
struct PlanApprovalBindingInput<'a> {
    contract: &'a str,
    proof_id: &'a str,
    process_epoch_id: &'a str,
    plan_id: &'a str,
    operation: ChangeOperation,
    target_provider_id: &'a str,
    target_provider_name: &'a str,
    baseline_digest: &'a str,
    actor: &'a ChangeActor,
    source_version: &'a str,
    revision: i64,
    created_at: i64,
    expires_at: i64,
    current_provider_code: &'a str,
    target_provider_code: &'a str,
    restart_expectation: RestartRequirement,
    risks: &'a [ChangePlanRisk],
    evidence_note: &'a str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CodexLiveBaselineState {
    Available,
    Missing,
    Unavailable,
}

#[derive(Clone)]
struct PrivateProjectionProof {
    current_definition: Option<[u8; 32]>,
    target_definition: [u8; 32],
    live_projection: [u8; 32],
    target_projection: [u8; 32],
}

#[derive(Clone)]
struct PrivatePlanProof {
    projection: PrivateProjectionProof,
    expires_at: i64,
}

#[derive(Clone)]
pub(crate) struct CodexSwitchInspection {
    pub db_current_provider_id: Option<String>,
    pub device_current_provider_id: Option<String>,
    pub effective_current_provider_id: Option<String>,
    pub target: Provider,
    live_projection_state: CodexLiveBaselineState,
    proxy_takeover_active: bool,
    private_proof: PrivateProjectionProof,
}

fn change_plan_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn safe_provider_display_name(provider: &Provider) -> String {
    let trimmed = provider.name.trim();
    let looks_path_like = trimmed
        .chars()
        .any(|character| matches!(character, '/' | '\\'))
        || trimmed.as_bytes().get(1).is_some_and(|byte| *byte == b':');
    let (_, api_key) = provider.resolve_usage_credentials(&AppType::Codex);
    let contains_credential = !api_key.trim().is_empty() && trimmed.contains(api_key.trim());
    if trimmed.is_empty()
        || looks_path_like
        || contains_credential
        || trimmed.chars().any(char::is_control)
    {
        return "Provider".to_string();
    }
    trimmed.chars().take(80).collect()
}

fn change_plan_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(|| {
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key
    })
}

fn process_epoch_id() -> &'static str {
    static EPOCH: OnceLock<String> = OnceLock::new();
    EPOCH
        .get_or_init(|| uuid::Uuid::new_v4().to_string())
        .as_str()
}

fn private_proofs() -> &'static Mutex<HashMap<String, PrivatePlanProof>> {
    static PROOFS: OnceLock<Mutex<HashMap<String, PrivatePlanProof>>> = OnceLock::new();
    PROOFS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_private_proofs() -> std::sync::MutexGuard<'static, HashMap<String, PrivatePlanProof>> {
    private_proofs()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn register_private_proof(proof_id: String, proof: PrivatePlanProof, now: i64) {
    let mut proofs = lock_private_proofs();
    proofs.retain(|_, existing| existing.expires_at >= now);
    proofs.insert(proof_id, proof);
}

fn get_private_proof(proof_id: &str) -> Option<PrivatePlanProof> {
    lock_private_proofs().get(proof_id).cloned()
}

#[cfg(test)]
fn clear_private_proofs_for_test() {
    lock_private_proofs().clear();
}

fn update_length_prefixed(mac: &mut HmacSha256, bytes: &[u8]) {
    mac.update(&(bytes.len() as u64).to_be_bytes());
    mac.update(bytes);
}

/// Derive a process-private proof. Callers must never serialize this value.
/// The proof ID prevents equality checks across plans, while the process-local
/// key makes a persisted database dump useless for offline secret guessing.
fn private_revision_bytes(domain: &str, proof_id: &str, bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(change_plan_mac_key())
        .expect("the fixed-size change-plan MAC key is always valid");
    update_length_prefixed(&mut mac, domain.as_bytes());
    update_length_prefixed(&mut mac, proof_id.as_bytes());
    update_length_prefixed(&mut mac, bytes);
    let bytes = mac.finalize().into_bytes();
    let mut revision = [0u8; 32];
    revision.copy_from_slice(&bytes);
    revision
}

fn opaque_revision_bytes(domain: &str, plan_id: &str, bytes: &[u8]) -> String {
    let encoded: String = private_revision_bytes(domain, plan_id, bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("mac1:{encoded}")
}

fn opaque_revision_json(domain: &str, plan_id: &str, value: &Value) -> String {
    let bytes = serde_json::to_vec(&canonical_json(value)).unwrap_or_default();
    opaque_revision_bytes(domain, plan_id, &bytes)
}

fn private_revision_json(domain: &str, proof_id: &str, value: &Value) -> [u8; 32] {
    let bytes = serde_json::to_vec(&canonical_json(value)).unwrap_or_default();
    private_revision_bytes(domain, proof_id, &bytes)
}

fn provider_definition_proof(provider: &Provider, proof_id: &str) -> [u8; 32] {
    let value = serde_json::to_value(provider).unwrap_or(Value::Null);
    private_revision_json(
        "fyagent.change-plan.provider-definition.v1",
        proof_id,
        &value,
    )
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

fn path_entry_is_missing(path: &std::path::Path) -> Option<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Some(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(true),
        Err(_) => None,
    }
}

fn live_projection_proof(
    result: Result<Value, AppError>,
    proof_id: &str,
) -> (CodexLiveBaselineState, [u8; 32]) {
    match result {
        Ok(value) => (
            CodexLiveBaselineState::Available,
            private_revision_json(
                "fyagent.change-plan.codex-live.v1",
                proof_id,
                &codex_projection_for_digest(value),
            ),
        ),
        Err(_) => {
            let definitely_missing = matches!(
                (
                    path_entry_is_missing(&crate::codex_config::get_codex_auth_path()),
                    path_entry_is_missing(&crate::codex_config::get_codex_config_path()),
                ),
                (Some(true), Some(true))
            );
            let state = if definitely_missing {
                CodexLiveBaselineState::Missing
            } else {
                CodexLiveBaselineState::Unavailable
            };
            let sentinel = match state {
                CodexLiveBaselineState::Missing => b"projection_missing".as_slice(),
                CodexLiveBaselineState::Unavailable => b"projection_unavailable".as_slice(),
                CodexLiveBaselineState::Available => unreachable!(),
            };
            (
                state,
                private_revision_bytes("fyagent.change-plan.codex-live.v1", proof_id, sentinel),
            )
        }
    }
}

fn codex_proxy_takeover_active(state: &AppState) -> Result<bool, ChangePlanErrorCode> {
    let has_backup = futures::executor::block_on(state.db.get_live_backup(AppType::Codex.as_str()))
        .map_err(|_| ChangePlanErrorCode::Internal)?
        .is_some();
    Ok(has_backup
        || state
            .proxy_service
            .detect_takeover_in_live_config_for_app(&AppType::Codex))
}

pub(crate) fn inspect_codex_switch(
    state: &AppState,
    target_provider_id: &str,
    proof_id: &str,
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
    let current_definition = effective_current_provider_id
        .as_ref()
        .and_then(|id| {
            state
                .db
                .get_provider_by_id(id, AppType::Codex.as_str())
                .ok()
                .flatten()
        })
        .map(|provider| provider_definition_proof(&provider, proof_id));
    let target_definition = provider_definition_proof(&target, proof_id);
    let (live_projection_state, live_projection) =
        live_projection_proof(read_live_settings(AppType::Codex), proof_id);
    let proxy_takeover_active = codex_proxy_takeover_active(state)?;
    let effective_target =
        build_effective_settings_with_common_config(state.db.as_ref(), &AppType::Codex, &target)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
    let target_projection = private_revision_json(
        "fyagent.change-plan.codex-live.v1",
        proof_id,
        &codex_projection_for_digest(effective_target),
    );
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        effective_current_provider_id,
        target,
        live_projection_state,
        proxy_takeover_active,
        private_proof: PrivateProjectionProof {
            current_definition,
            target_definition,
            live_projection,
            target_projection,
        },
    })
}

fn baseline_binding_digest(
    plan_id: &str,
    proof_id: &str,
    epoch_id: &str,
    inspection: &CodexSwitchInspection,
) -> Result<String, ChangePlanErrorCode> {
    let baseline_value = serde_json::to_value(BaselineDigestInput {
        contract: CHANGE_PLAN_CONTRACT_VERSION,
        proof_id,
        process_epoch_id: epoch_id,
        db_current_provider_id: &inspection.db_current_provider_id,
        device_current_provider_id: &inspection.device_current_provider_id,
        effective_current_provider_id: &inspection.effective_current_provider_id,
        target_provider_id: &inspection.target.id,
        live_projection_state: inspection.live_projection_state,
        proxy_takeover_active: inspection.proxy_takeover_active,
    })
    .map_err(|_| ChangePlanErrorCode::Internal)?;
    Ok(opaque_revision_json(
        "fyagent.change-plan.baseline-binding.v1",
        plan_id,
        &baseline_value,
    ))
}

fn plan_approval_binding_digest(
    plan: &ChangePlan,
    proof_id: &str,
    epoch_id: &str,
    contract: &str,
) -> Result<String, ChangePlanErrorCode> {
    let value = serde_json::to_value(PlanApprovalBindingInput {
        contract,
        proof_id,
        process_epoch_id: epoch_id,
        plan_id: &plan.plan_id,
        operation: plan.operation,
        target_provider_id: &plan.target_provider_id,
        target_provider_name: &plan.target_provider_name,
        baseline_digest: &plan.baseline_digest,
        actor: &plan.actor,
        source_version: &plan.source_version,
        revision: plan.revision,
        created_at: plan.created_at,
        expires_at: plan.expires_at,
        current_provider_code: &plan.current_provider_code,
        target_provider_code: &plan.target_provider_code,
        restart_expectation: plan.restart_expectation,
        risks: &plan.risks,
        evidence_note: &plan.evidence_note,
    })
    .map_err(|_| ChangePlanErrorCode::Internal)?;
    Ok(opaque_revision_json(
        "fyagent.change-plan.plan-approval.v1",
        &plan.plan_id,
        &value,
    ))
}

fn private_proof_matches(left: &PrivateProjectionProof, right: &PrivateProjectionProof) -> bool {
    optional_revision_matches(
        left.current_definition.as_ref(),
        right.current_definition.as_ref(),
    ) && constant_time_revision_matches(&left.target_definition, &right.target_definition)
        && constant_time_revision_matches(&left.live_projection, &right.live_projection)
        && constant_time_revision_matches(&left.target_projection, &right.target_projection)
}

fn optional_revision_matches(left: Option<&[u8; 32]>, right: Option<&[u8; 32]>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => constant_time_revision_matches(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn constant_time_revision_matches(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn constant_time_text_matches(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0u8, |difference, (left, right)| difference | (left ^ right))
            == 0
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
        let _provider_guard =
            crate::services::ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let plan_id = uuid::Uuid::new_v4().to_string();
        let proof_id = uuid::Uuid::new_v4().to_string();
        let epoch_id = process_epoch_id().to_string();
        let inspection = inspect_codex_switch(state, target_provider_id, &proof_id)?;
        if inspection.effective_current_provider_id.as_deref() == Some(target_provider_id) {
            return Err(ChangePlanErrorCode::TargetAlreadyCurrent);
        }
        if inspection.proxy_takeover_active {
            return Err(ChangePlanErrorCode::UnsupportedOperation);
        }
        if inspection.live_projection_state == CodexLiveBaselineState::Unavailable {
            return Err(ChangePlanErrorCode::BaselineUnavailable);
        }
        let baseline_digest = baseline_binding_digest(&plan_id, &proof_id, &epoch_id, &inspection)?;
        let mut public = ChangePlan {
            plan_id,
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: target_provider_id.to_string(),
            target_provider_name: safe_provider_display_name(&inspection.target),
            plan_digest: String::new(),
            baseline_digest,
            actor: ChangeActor {
                actor_type: ChangeActorType::DirectUser,
            },
            source_version: env!("CARGO_PKG_VERSION").to_string(),
            revision: 1,
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
        public.plan_digest = plan_approval_binding_digest(
            &public,
            &proof_id,
            &epoch_id,
            CHANGE_PLAN_CONTRACT_VERSION,
        )?;
        state
            .db
            .insert_change_plan(&StoredChangePlan {
                public: public.clone(),
                proof_id: proof_id.clone(),
                process_epoch_id: epoch_id,
                current_provider_id: inspection.effective_current_provider_id,
                contract_digest: CHANGE_PLAN_CONTRACT_VERSION.to_string(),
            })
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        register_private_proof(
            proof_id,
            PrivatePlanProof {
                projection: inspection.private_proof,
                expires_at: public.expires_at,
            },
            now,
        );
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
                crate::services::ProviderService::switch_with_lock_held(
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
        let _provider_guard =
            crate::services::ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let stored = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let Some(stored) = stored else {
            return Ok(rejected(ChangePlanErrorCode::PlanNotFound));
        };
        if !constant_time_text_matches(&stored.public.plan_digest, plan_digest) {
            return Ok(rejected(ChangePlanErrorCode::InvalidDigest));
        }
        if stored.public.status == ChangePlanStatus::Consumed {
            return Ok(rejected(ChangePlanErrorCode::Consumed));
        }
        if now >= stored.public.expires_at {
            return Ok(rejected(ChangePlanErrorCode::Expired));
        }
        if stored.process_epoch_id != process_epoch_id() {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        if stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let rebound_digest = plan_approval_binding_digest(
            &stored.public,
            &stored.proof_id,
            &stored.process_epoch_id,
            &stored.contract_digest,
        )?;
        if !constant_time_text_matches(&stored.public.plan_digest, &rebound_digest) {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let Some(expected_private) = get_private_proof(&stored.proof_id) else {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        };
        let observed =
            inspect_codex_switch(state, &stored.public.target_provider_id, &stored.proof_id)?;
        if observed.proxy_takeover_active
            || observed.live_projection_state == CodexLiveBaselineState::Unavailable
        {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let observed_baseline = baseline_binding_digest(
            &stored.public.plan_id,
            &stored.proof_id,
            &stored.process_epoch_id,
            &observed,
        )?;
        if !private_proof_matches(&expected_private.projection, &observed.private_proof) {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let job_id = uuid::Uuid::new_v4().to_string();
        let admitted = state
            .db
            .admit_change_plan(plan_id, plan_digest, &observed_baseline, &job_id, now)
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

        let readback =
            inspect_codex_switch(state, &stored.public.target_provider_id, &stored.proof_id);
        classify_job(
            &stored,
            Some(&expected_private.projection),
            &mut job,
            writer_result,
            readback,
            now,
        );
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
        let _provider_guard =
            crate::services::ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        job = state
            .db
            .get_change_job(&job.job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        if job.status.is_terminal() {
            return Ok(job);
        }
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
        let private_proof = (stored.process_epoch_id == process_epoch_id())
            .then(|| get_private_proof(&stored.proof_id))
            .flatten();
        let readback =
            inspect_codex_switch(state, &stored.public.target_provider_id, &stored.proof_id);
        classify_job(
            &stored,
            private_proof.as_ref().map(|proof| &proof.projection),
            &mut job,
            Err(()),
            readback,
            now,
        );
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

fn rejected(error_code: ChangePlanErrorCode) -> ApplyChangePlanOutcome {
    ApplyChangePlanOutcome {
        kind: ChangeApplyOutcomeKind::Rejected,
        job: None,
        error_code: Some(error_code),
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
    expected_private: Option<&PrivateProjectionProof>,
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
    let baseline_db = readback.db_current_provider_id == stored.current_provider_id;
    let baseline_device = readback.device_current_provider_id == stored.current_provider_id;

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
    let Some(expected_private) = expected_private else {
        set_resource(
            job,
            ChangeResourceKind::TargetDefinition,
            ChangeResourceStatus::Unavailable,
            "private_proof_unavailable",
        );
        set_resource(
            job,
            ChangeResourceKind::CodexLiveProjection,
            ChangeResourceStatus::Unavailable,
            "private_proof_unavailable",
        );
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::RecoveryRequired;
        job.restart_requirement = RestartRequirement::Unknown;
        job.recovery_state = RecoveryState::RecoveryRequired;
        job.diagnostic_code = Some("private_proof_unavailable".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "private_proof_unavailable",
        );
        return;
    };

    let definition_target = constant_time_revision_matches(
        &readback.private_proof.target_definition,
        &expected_private.target_definition,
    );
    let live_available = readback.live_projection_state == CodexLiveBaselineState::Available;
    let live_unavailable = readback.live_projection_state == CodexLiveBaselineState::Unavailable;
    let live_target = live_available
        && constant_time_revision_matches(
            &readback.private_proof.live_projection,
            &expected_private.target_projection,
        );
    let baseline_current_definition = optional_revision_matches(
        readback.private_proof.current_definition.as_ref(),
        expected_private.current_definition.as_ref(),
    );
    let baseline_live = constant_time_revision_matches(
        &readback.private_proof.live_projection,
        &expected_private.live_projection,
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
        if live_unavailable {
            ChangeResourceStatus::Unavailable
        } else if live_target {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if live_unavailable {
            "live_unavailable"
        } else if live_target {
            "live_matched"
        } else if readback.live_projection_state == CodexLiveBaselineState::Missing {
            "live_missing"
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
    } else if baseline_db
        && baseline_device
        && baseline_current_definition
        && baseline_live
        && definition_target
    {
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
        job.result_code = if live_unavailable {
            ChangeResultCode::ReadbackUnavailable
        } else {
            ChangeResultCode::PostWriteMismatch
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

pub(crate) fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
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
            actor: ChangeActor {
                actor_type: ChangeActorType::DirectUser,
            },
            source_version: "test-version".into(),
            revision: 1,
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
    fn change_plan_opaque_revision_is_canonical_but_approval_scoped() {
        let left = json!({"b": 2, "a": {"y": 1, "x": 0}});
        let right = json!({"a": {"x": 0, "y": 1}, "b": 2});
        assert_eq!(
            opaque_revision_json("change-plan-test", "plan-a", &left),
            opaque_revision_json("change-plan-test", "plan-a", &right)
        );
        assert_ne!(
            opaque_revision_json("change-plan-test", "plan-a", &left),
            opaque_revision_json("change-plan-test", "plan-b", &left)
        );
    }

    #[test]
    fn change_plan_cross_layer_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/changePlanDtoContract.v1.json"
        ))
        .unwrap();
        let plan = ChangePlan {
            plan_id: "plan-contract".into(),
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: "provider-target".into(),
            target_provider_name: "Target Provider".into(),
            plan_digest: "plan-digest".into(),
            baseline_digest: "baseline-digest".into(),
            actor: ChangeActor {
                actor_type: ChangeActorType::DirectUser,
            },
            source_version: "0.4.2".into(),
            revision: 1,
            created_at: 100,
            expires_at: 1000,
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
        let job = ChangeJobSnapshot {
            job_id: "job-contract".into(),
            plan_id: "plan-contract".into(),
            target_provider_id: "provider-target".into(),
            revision: 4,
            event_seq: 4,
            status: ChangeJobStatus::Succeeded,
            result_code: ChangeResultCode::AppliedRestartRecommended,
            steps: vec![
                ChangeJobStep {
                    kind: ChangeStepKind::Precheck,
                    status: ChangeStepStatus::Succeeded,
                    code: "baseline_matched".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Apply,
                    status: ChangeStepStatus::Succeeded,
                    code: "writer_returned".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Readback,
                    status: ChangeStepStatus::Succeeded,
                    code: "target_matched".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Reconcile,
                    status: ChangeStepStatus::NotStarted,
                    code: "not_started".into(),
                },
            ],
            resources: vec![
                ChangeResourceResult {
                    kind: ChangeResourceKind::ProviderDbCurrent,
                    status: ChangeResourceStatus::Matched,
                    code: "target_current".into(),
                },
                ChangeResourceResult {
                    kind: ChangeResourceKind::DeviceCurrent,
                    status: ChangeResourceStatus::Matched,
                    code: "target_current".into(),
                },
                ChangeResourceResult {
                    kind: ChangeResourceKind::TargetDefinition,
                    status: ChangeResourceStatus::Matched,
                    code: "definition_matched".into(),
                },
                ChangeResourceResult {
                    kind: ChangeResourceKind::CodexLiveProjection,
                    status: ChangeResourceStatus::Matched,
                    code: "live_matched".into(),
                },
            ],
            restart_requirement: RestartRequirement::Recommended,
            usage_evidence: UsageEvidence::NotObserved,
            recovery_state: RecoveryState::NotNeeded,
            diagnostic_code: Some("target_readback_matched".into()),
            live_config_changed: true,
            created_at: 100,
            updated_at: 101,
        };
        let outcome = ApplyChangePlanOutcome {
            kind: ChangeApplyOutcomeKind::Admitted,
            job: Some(job),
            error_code: None,
        };
        assert_eq!(serde_json::to_value(plan).unwrap(), fixture["plan"]);
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            fixture["applyOutcome"]
        );
        assert_eq!(
            serde_json::to_value(ChangeJobStatus::Cancelled).unwrap(),
            fixture["reservedStatuses"]["job"][0]
        );
        assert_eq!(
            serde_json::to_value(ChangeJobStatus::Warning).unwrap(),
            fixture["reservedStatuses"]["job"][1]
        );
        assert_eq!(
            serde_json::to_value(ChangeStepStatus::NotStarted).unwrap(),
            fixture["reservedStatuses"]["step"][0]
        );
    }

    #[test]
    #[serial]
    fn codex_provider_switch_plan_is_unique_opaque_and_target_side_effect_free() {
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
        let before_current = serde_json::to_value(
            db.get_provider_by_id(&current.id, AppType::Codex.as_str())
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let before_target = serde_json::to_value(
            db.get_provider_by_id(&target.id, AppType::Codex.as_str())
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let before_live = read_live_settings(AppType::Codex).unwrap();

        let first = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let second = ChangePlanService::plan_codex_switch_at(&state, &target.id, 101).unwrap();

        assert_ne!(first.plan_id, second.plan_id);
        assert_ne!(first.plan_digest, second.plan_digest);
        assert_ne!(first.baseline_digest, second.baseline_digest);
        assert!(first.plan_digest.starts_with("mac1:"));
        assert!(first.baseline_digest.starts_with("mac1:"));
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            before_db_current
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Codex),
            before_device
        );
        assert_eq!(
            serde_json::to_value(
                db.get_provider_by_id(&current.id, AppType::Codex.as_str())
                    .unwrap()
                    .unwrap(),
            )
            .unwrap(),
            before_current
        );
        assert_eq!(
            serde_json::to_value(
                db.get_provider_by_id(&target.id, AppType::Codex.as_str())
                    .unwrap()
                    .unwrap(),
            )
            .unwrap(),
            before_target
        );
        assert_eq!(read_live_settings(AppType::Codex).unwrap(), before_live);
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
        let first_stored = db.get_stored_change_plan(&first.plan_id).unwrap().unwrap();
        let private = get_private_proof(&first_stored.proof_id).unwrap();
        let private_tags: Vec<String> = [
            private.projection.current_definition.unwrap(),
            private.projection.target_definition,
            private.projection.live_projection,
            private.projection.target_projection,
        ]
        .into_iter()
        .map(|tag| tag.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect();
        let conn = db.conn.lock().unwrap();
        let plan_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(plan_count, 2, "only immutable plan rows may be written");
        let mut stmt = conn
            .prepare(
                "SELECT plan_digest, baseline_digest, proof_id, process_epoch_id
                 FROM change_plans ORDER BY created_at, plan_id",
            )
            .unwrap();
        let revisions: Vec<Vec<String>> = stmt
            .query_map([], |row| {
                Ok(vec![row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(revisions.len(), 2);
        assert!(revisions
            .iter()
            .all(|row| row[0].starts_with("mac1:") && row[1].starts_with("mac1:")));
        assert_ne!(revisions[0][0], revisions[1][0]);
        assert_ne!(revisions[0][1], revisions[1][1]);
        assert_ne!(revisions[0][2], revisions[1][2]);
        assert_eq!(revisions[0][3], revisions[1][3]);
        let serialized_revisions = serde_json::to_string(&revisions).unwrap();
        assert!(!serialized_revisions.contains("sentinel-current"));
        assert!(!serialized_revisions.contains("sentinel-target"));
        for private_tag in private_tags {
            assert!(
                !serialized_revisions.contains(&private_tag),
                "private secret-bearing proofs must remain memory-only"
            );
        }
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

    #[test]
    #[serial]
    fn codex_provider_switch_plan_sanitizes_cross_platform_path_display_names() {
        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();

        for path_like_name in [
            "/Users/private/provider",
            r"C:\Users\private\provider",
            r"\\server\private\provider",
            "relative/private/provider",
        ] {
            target.name = path_like_name.to_string();
            db.save_provider(AppType::Codex.as_str(), &target).unwrap();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
            assert_eq!(plan.target_provider_name, "Provider");
            assert!(!serde_json::to_string(&plan)
                .unwrap()
                .contains(path_like_name));
        }

        target.name = "prefix-sentinel-target-suffix".to_string();
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        assert_eq!(plan.target_provider_name, "Provider");

        target.name = "  Safe Provider  ".to_string();
        db.save_provider(AppType::Codex.as_str(), &target).unwrap();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        assert_eq!(plan.target_provider_name, "Safe Provider");
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
        let wrong_digest = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            "mac1:wrong",
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(
            wrong_digest.error_code,
            Some(ChangePlanErrorCode::InvalidDigest)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                crate::services::ProviderService::with_live_config_result(AppType::Codex, || {
                    crate::services::ProviderService::switch_with_lock_held(
                        &state,
                        AppType::Codex,
                        &target.id,
                    )
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
    fn stored_plan_immutable_field_tampering_is_stale_and_zero_writer() {
        use rusqlite::params;
        use std::sync::atomic::{AtomicUsize, Ordering};

        for mutation in [
            "source_version",
            "target_name",
            "contract",
            "extended_expiry",
        ] {
            let (_home, _guard, db, state, _current, target) = setup_switch_state();
            let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
            let now = if mutation == "extended_expiry" {
                db.conn
                    .lock()
                    .unwrap()
                    .execute(
                        "UPDATE change_plans SET expires_at = 2000 WHERE plan_id = ?1",
                        params![plan.plan_id],
                    )
                    .unwrap();
                1000
            } else {
                let statement = match mutation {
                    "source_version" => {
                        "UPDATE change_plans SET source_version = 'tampered' WHERE plan_id = ?1"
                    }
                    "target_name" => {
                        "UPDATE change_plans SET target_provider_name = 'Tampered' WHERE plan_id = ?1"
                    }
                    "contract" => {
                        "UPDATE change_plans SET contract_digest = 'tampered' WHERE plan_id = ?1"
                    }
                    _ => unreachable!(),
                };
                db.conn
                    .lock()
                    .unwrap()
                    .execute(statement, params![plan.plan_id])
                    .unwrap();
                101
            };
            let calls = AtomicUsize::new(0);

            let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
                &state,
                &plan.plan_id,
                &plan.plan_digest,
                now,
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(false)
                },
            )
            .unwrap();

            assert_eq!(
                outcome.error_code,
                Some(ChangePlanErrorCode::Stale),
                "mutation {mutation}"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0, "mutation {mutation}");
            assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
        }
    }

    fn remove_codex_live_entries() {
        for path in [
            crate::codex_config::get_codex_auth_path(),
            crate::codex_config::get_codex_config_path(),
        ] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("remove {}: {error}", path.display()),
            }
        }
    }

    #[test]
    #[serial]
    fn known_missing_live_baseline_is_distinct_and_can_apply() {
        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        remove_codex_live_entries();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                crate::services::ProviderService::with_live_config_result(AppType::Codex, || {
                    crate::services::ProviderService::switch_with_lock_held(
                        &state,
                        AppType::Codex,
                        &target.id,
                    )
                })
                .map(|result| result.live_config_changed)
                .map_err(|_| ())
            },
        )
        .unwrap();

        assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Admitted);
        assert_eq!(outcome.job.unwrap().status, ChangeJobStatus::Succeeded);
    }

    #[test]
    #[serial]
    fn malformed_live_baseline_cannot_create_or_apply_a_plan() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        std::fs::write(crate::codex_config::get_codex_config_path(), "[broken").unwrap();
        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &target.id, 100),
            Err(ChangePlanErrorCode::BaselineUnavailable)
        );
        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        remove_codex_live_entries();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 101).unwrap();
        std::fs::write(crate::codex_config::get_codex_config_path(), "[broken").unwrap();
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
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
        assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    #[serial]
    fn proxy_takeover_is_rejected_before_plan_persistence() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        futures::executor::block_on(state.db.save_live_backup(AppType::Codex.as_str(), "{}"))
            .unwrap();

        assert_eq!(
            ChangePlanService::plan_codex_switch_at(&state, &target.id, 100),
            Err(ChangePlanErrorCode::UnsupportedOperation)
        );
        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
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
    fn codex_provider_change_job_api_key_only_drift_is_stale_and_zero_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        target.settings_config["auth"]["OPENAI_API_KEY"] =
            Value::String("sentinel-target-rotated".into());
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
    }

    #[test]
    #[serial]
    fn codex_provider_change_plan_lost_private_proof_is_stale_and_zero_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        clear_private_proofs_for_test();
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
    }

    #[test]
    #[serial]
    fn interrupted_job_without_private_proof_requires_recovery_without_replay() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let inspected = inspect_codex_switch(&state, &target.id, &stored.proof_id).unwrap();
        let observed_baseline = baseline_binding_digest(
            &plan.plan_id,
            &stored.proof_id,
            &stored.process_epoch_id,
            &inspected,
        )
        .unwrap();
        let admitted = db
            .admit_change_plan(
                &plan.plan_id,
                &plan.plan_digest,
                &observed_baseline,
                "restart-proof-loss-job",
                101,
            )
            .unwrap();
        assert_eq!(admitted.kind, ChangeApplyOutcomeKind::Admitted);

        clear_private_proofs_for_test();
        let reconciled = ChangePlanService::get_job(&state, "restart-proof-loss-job").unwrap();

        assert_eq!(reconciled.status, ChangeJobStatus::Failed);
        assert_eq!(reconciled.result_code, ChangeResultCode::RecoveryRequired);
        assert_eq!(reconciled.recovery_state, RecoveryState::RecoveryRequired);
        assert_eq!(
            reconciled.diagnostic_code.as_deref(),
            Some("private_proof_unavailable")
        );
        assert!(reconciled.resources.iter().any(|resource| {
            resource.kind == ChangeResourceKind::TargetDefinition
                && resource.status == ChangeResourceStatus::Unavailable
        }));
    }

    #[test]
    #[serial]
    fn change_plan_reconciliation_reads_state_without_replaying_writer() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let inspected = inspect_codex_switch(&state, &target.id, &stored.proof_id).unwrap();
        let observed_baseline = baseline_binding_digest(
            &plan.plan_id,
            &stored.proof_id,
            &stored.process_epoch_id,
            &inspected,
        )
        .unwrap();
        let admitted = db
            .admit_change_plan(
                &plan.plan_id,
                &plan.plan_digest,
                &observed_baseline,
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
    fn change_plan_reconciliation_reloads_terminal_snapshot_after_lock_acquisition() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let inspected = inspect_codex_switch(&state, &target.id, &stored.proof_id).unwrap();
        let observed_baseline = baseline_binding_digest(
            &plan.plan_id,
            &stored.proof_id,
            &stored.process_epoch_id,
            &inspected,
        )
        .unwrap();
        let admitted = db
            .admit_change_plan(
                &plan.plan_id,
                &plan.plan_digest,
                &observed_baseline,
                "terminal-race-job",
                101,
            )
            .unwrap();
        let stale = admitted.job.unwrap();
        let mut terminal = stale.clone();
        terminal.status = ChangeJobStatus::Succeeded;
        terminal.result_code = ChangeResultCode::Applied;
        terminal.revision = 2;
        terminal.event_seq = 2;
        db.save_change_job(&terminal, "terminal").unwrap();

        let observed = ChangePlanService::reconcile_job(&state, stale).unwrap();
        assert_eq!(observed.status, ChangeJobStatus::Succeeded);
        assert_eq!(observed.event_seq, 2);
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

    #[test]
    #[serial]
    fn codex_provider_change_job_definition_drift_requires_recovery() {
        let (_home, _guard, db, state, _current, mut target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let job = ChangePlanService::apply_codex_switch_at_with_writer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                target.name = "Drifted target".to_string();
                db.save_provider(AppType::Codex.as_str(), &target).unwrap();
                Err(())
            },
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(job.recovery_state, RecoveryState::RecoveryRequired);
        assert_eq!(job.result_code, ChangeResultCode::PostWriteMismatch);
    }
}
