use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use crate::app_config::AppType;
use crate::provider::{Provider, ProviderMeta};
use crate::services::provider::{
    build_effective_settings_with_common_config, codex_secret_ref_handle,
    materialize_codex_secret_ref_provider, materialize_codex_secret_ref_provider_from_keyring,
    read_live_settings,
};
use crate::services::secret::{
    NativeSecretBackend, SecretBackendKind, SecretHandle, SecretMaterial, SecretPurpose,
    SecretService,
};
use crate::services::workbuddy::{
    self, types::SaveWorkBuddyModelsRequest, WorkBuddyChangeSnapshot,
};
use crate::store::AppState;
use crate::AppError;

mod adapter;

use adapter::{
    ChangeAdapter, CodexProviderSwitchAdapter, CodexProviderUpsertAdapter, RegisteredCodexAdapter,
    WorkBuddyModelsAdapter,
};

pub(crate) const CHANGE_PLAN_CONTRACT_VERSION: &str = "fyagent-change-plan-v2-schema20";
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
    CodexProviderSwitch,
    CodexProviderUpsertAndSwitch,
    WorkBuddyModelsUpdate
});
string_enum!(ChangeBusinessStepKind {
    SaveProvider,
    SetCurrentProvider,
    SaveWorkBuddyModels
});
string_enum!(ChangeSecretBackend { OsKeyring });
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
    Snapshot,
    ManagedWrite,
    Readback,
    Finalize
});
string_enum!(ChangeStepStatus {
    NotStarted,
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
    CodexLiveProjection,
    WorkBuddyModelsConfig,
    WorkBuddyBackup
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
    InvalidRequest,
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
    pub remaining_effects: Vec<String>,
    pub manual_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeActor {
    #[serde(rename = "type")]
    pub actor_type: ChangeActorType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCredentialProjection {
    pub secret_ref_display: String,
    pub backend: ChangeSecretBackend,
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
    pub business_steps: Vec<ChangeBusinessStepKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<ChangeCredentialProjection>,
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
    pub idempotency_key: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_error_code: Option<ChangeAdapterErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result: Option<ChangePartialResult>,
    pub diagnostic_code: Option<String>,
    pub live_config_changed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ChangeJobSnapshot {
    pub(crate) fn planned(
        job_id: String,
        plan_id: String,
        target_id: String,
        resources: Vec<ChangeResourceKind>,
        now: i64,
    ) -> Self {
        Self {
            execution_id: job_id.clone(),
            idempotency_key: plan_id.clone(),
            job_id,
            plan_id,
            target_provider_id: target_id,
            revision: 1,
            event_seq: 1,
            status: ChangeJobStatus::Planned,
            result_code: ChangeResultCode::Planned,
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
                status: ChangeStepStatus::NotStarted,
                code: "not_started".to_string(),
            })
            .collect(),
            resources: resources
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
            adapter_error_code: None,
            partial_result: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelChangeJobOutcome {
    pub accepted: bool,
    pub code: ChangeCancelCode,
    pub job_id: String,
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
    business_steps: &'a [ChangeBusinessStepKind],
    credential: &'a Option<ChangeCredentialProjection>,
    current_provider_code: &'a str,
    target_provider_code: &'a str,
    restart_expectation: RestartRequirement,
    risks: &'a [ChangePlanRisk],
    evidence_note: &'a str,
    adapter: &'a ChangeAdapterDescriptor,
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
    baseline_target_definition: [u8; 32],
    target_definition: [u8; 32],
    live_projection: [u8; 32],
    target_projection: [u8; 32],
}

#[derive(Clone)]
struct PrivatePlanProof {
    projection: PrivateProjectionProof,
    expires_at: i64,
    credential: Option<Arc<PrivateCodexCredentialPlan>>,
    workbuddy: Option<Arc<PrivateWorkBuddyPlan>>,
}

struct PrivateCodexCredentialPlan {
    handle: SecretHandle,
    material: SecretMaterial,
    persisted_provider: Provider,
    previous_handle: Option<SecretHandle>,
    cleanup_state: AtomicU8,
}

struct PrivateWorkBuddyPlan {
    request: SaveWorkBuddyModelsRequest,
    baseline: WorkBuddyChangeSnapshot,
    target_model_count: usize,
    existing_target_count: usize,
}

#[derive(Clone)]
enum WorkBuddyWriteReceipt {
    Applied { revision: String },
    NoWrite,
    UnknownOutcome,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexProviderUpsertPlanRequest {
    name: String,
    base_url: String,
    api_key: String,
    model_id: String,
    #[serde(default)]
    codex_features: Option<crate::codex_config::CodexProviderFeatureIntent>,
}

impl Drop for CodexProviderUpsertPlanRequest {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.api_key.zeroize();
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkBuddyModelsPlanRequest {
    base_url: String,
    api_key: String,
    allow_no_api_key: bool,
    #[serde(default)]
    selected_model_ids: Vec<String>,
    #[serde(default)]
    manual_model_ids: Vec<String>,
    #[serde(default)]
    removed_model_ids: Vec<String>,
    #[serde(default)]
    clear_existing_api_keys: bool,
    expected_revision: Option<String>,
}

impl WorkBuddyModelsPlanRequest {
    fn into_save_request(mut self) -> SaveWorkBuddyModelsRequest {
        SaveWorkBuddyModelsRequest {
            base_url: std::mem::take(&mut self.base_url),
            api_key: std::mem::take(&mut self.api_key),
            allow_no_api_key: self.allow_no_api_key,
            selected_model_ids: std::mem::take(&mut self.selected_model_ids),
            manual_model_ids: std::mem::take(&mut self.manual_model_ids),
            removed_model_ids: std::mem::take(&mut self.removed_model_ids),
            clear_existing_api_keys: self.clear_existing_api_keys,
            expected_revision: self.expected_revision.take(),
            overwrite_token: None,
        }
    }
}

impl Drop for WorkBuddyModelsPlanRequest {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.api_key.zeroize();
    }
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

const UCP_CODEX_PROVIDER_ID: &str = "fyagent-v2-quick-setup-codex";

fn missing_target_definition_proof(proof_id: &str) -> [u8; 32] {
    private_revision_bytes(
        "fyagent.change-plan.provider-definition.v1",
        proof_id,
        b"provider_missing",
    )
}

fn provider_secret_handle(
    provider: &Provider,
) -> Result<Option<SecretHandle>, ChangePlanErrorCode> {
    codex_secret_ref_handle(provider).map_err(|_| ChangePlanErrorCode::InvalidRequest)
}

fn build_codex_credential_plan(
    state: &AppState,
    mut request: CodexProviderUpsertPlanRequest,
) -> Result<PrivateCodexCredentialPlan, ChangePlanErrorCode> {
    use zeroize::Zeroize;

    let name = request.name.trim().to_string();
    let base_url = request.base_url.trim().to_string();
    let model_id = request.model_id.trim().to_string();
    let mut raw_input = std::mem::take(&mut request.api_key);
    let api_key = raw_input.trim();
    let valid_public_fields = !name.is_empty()
        && !model_id.is_empty()
        && !api_key.is_empty()
        && !name.contains(api_key)
        && !model_id.contains(api_key)
        && !base_url.contains(api_key);
    let parsed_url = url::Url::parse(&base_url).ok();
    let valid_url = parsed_url.as_ref().is_some_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
    });
    if !valid_public_fields || !valid_url {
        raw_input.zeroize();
        return Err(ChangePlanErrorCode::InvalidRequest);
    }
    let material = match SecretMaterial::from_native_input(
        api_key.as_bytes().to_vec(),
        SecretPurpose::CodexApiKey,
    ) {
        Ok(material) => material,
        Err(_) => {
            raw_input.zeroize();
            return Err(ChangePlanErrorCode::InvalidRequest);
        }
    };
    raw_input.zeroize();

    let features = request.codex_features.take().unwrap_or_default();
    let image_extension = features.image_extension.unwrap_or(false);
    let websockets = features.websockets.unwrap_or(false);
    let quote =
        |value: &str| serde_json::to_string(value).expect("serializing a Rust string cannot fail");
    let mut config = format!(
        "model_provider = \"custom\"\nmodel = {}\ndisable_response_storage = true\n\n[model_providers.custom]\nname = {}\nbase_url = {}\nwire_api = \"responses\"\nrequires_openai_auth = {}",
        quote(&model_id),
        quote(&name),
        quote(&base_url),
        !image_extension,
    );
    if image_extension {
        config.push_str(&format!(
            "\nhttp_headers = {{ \"{}\" = \"{}\" }}",
            crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER,
            crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE,
        ));
    }
    if websockets {
        config.push_str("\nsupports_websockets = true");
    }

    let handle = SecretHandle::generate();
    let mut persisted_provider = Provider::with_id(
        UCP_CODEX_PROVIDER_ID.to_string(),
        name,
        serde_json::json!({
            "auth": {
                "secretRef": handle.secret_ref().as_str(),
                "secretVersion": handle.version().as_str(),
                "backend": "osKeyring",
            },
            "config": config,
        }),
        None,
    );
    persisted_provider.category = Some("custom".to_string());
    persisted_provider.notes = Some("Created by FyAgent V2 quick setup".to_string());
    persisted_provider.meta = Some(ProviderMeta {
        image_extension_configured: features.image_extension.map(|_| true),
        ..ProviderMeta::default()
    });
    let previous_handle = state
        .db
        .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?
        .as_ref()
        .map(provider_secret_handle)
        .transpose()?
        .flatten();

    Ok(PrivateCodexCredentialPlan {
        handle,
        material,
        persisted_provider,
        previous_handle,
        cleanup_state: AtomicU8::new(0),
    })
}

type CurrentCodexProviderIds = (Option<String>, Option<String>, Option<String>);

fn current_codex_provider_ids(
    state: &AppState,
) -> Result<CurrentCodexProviderIds, ChangePlanErrorCode> {
    let db_current = state
        .db
        .get_current_provider(AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    let device_current = crate::settings::get_current_provider(&AppType::Codex);
    let effective = device_current
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
        .or_else(|| db_current.clone());
    Ok((db_current, device_current, effective))
}

fn intended_codex_projection_proof_for_live_provider(
    state: &AppState,
    live: &crate::services::provider::EphemeralProvider,
    proof_id: &str,
) -> Result<[u8; 32], ChangePlanErrorCode> {
    let mut effective =
        build_effective_settings_with_common_config(state.db.as_ref(), &AppType::Codex, live)
            .map_err(|_| ChangePlanErrorCode::InvalidRequest)?;
    let auth = effective
        .get("auth")
        .cloned()
        .ok_or(ChangePlanErrorCode::InvalidRequest)?;
    let config = effective
        .get("config")
        .and_then(Value::as_str)
        .ok_or(ChangePlanErrorCode::InvalidRequest)?;
    // The upsert contract never carries a model catalog. This makes catalog
    // preparation a pure normalization step here while still matching the
    // established Codex writer's final config.toml projection exactly.
    if effective.get("modelCatalog").is_some() {
        return Err(ChangePlanErrorCode::InvalidRequest);
    }
    let profile = crate::proxy::providers::resolve_codex_catalog_tool_profile(live);
    let catalog_prepared = crate::codex_config::prepare_codex_config_text_with_model_catalog(
        &effective, config, profile,
    )
    .map_err(|_| ChangePlanErrorCode::InvalidRequest)?;
    let preserve_official_auth = crate::settings::preserve_codex_official_auth_on_switch();
    let projected_config = if preserve_official_auth {
        crate::codex_config::prepare_codex_provider_live_config(&auth, &catalog_prepared)
    } else {
        crate::codex_config::project_codex_live_config_when_openai_auth_disabled(
            &auth,
            &catalog_prepared,
        )
    }
    .map_err(|_| ChangePlanErrorCode::InvalidRequest)?;
    let projected_auth = if preserve_official_auth {
        read_live_settings(AppType::Codex)
            .map_err(|_| ChangePlanErrorCode::BaselineUnavailable)?
            .get("auth")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        auth
    };
    let object = effective
        .as_object_mut()
        .ok_or(ChangePlanErrorCode::InvalidRequest)?;
    object.insert("auth".to_string(), projected_auth);
    object.insert("config".to_string(), Value::String(projected_config));
    Ok(private_revision_json(
        "fyagent.change-plan.codex-live.v1",
        proof_id,
        &codex_projection_for_digest(effective),
    ))
}

fn intended_codex_projection_proof(
    state: &AppState,
    credential: &PrivateCodexCredentialPlan,
    proof_id: &str,
) -> Result<[u8; 32], ChangePlanErrorCode> {
    let live =
        materialize_codex_secret_ref_provider(&credential.persisted_provider, &credential.material)
            .map_err(|_| ChangePlanErrorCode::InvalidRequest)?;
    intended_codex_projection_proof_for_live_provider(state, &live, proof_id)
}

fn inspect_codex_upsert_precheck(
    state: &AppState,
    credential: &PrivateCodexCredentialPlan,
    proof_id: &str,
) -> Result<CodexSwitchInspection, ChangePlanErrorCode> {
    let (db_current_provider_id, device_current_provider_id, effective_current_provider_id) =
        current_codex_provider_ids(state)?;
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
    let stored_target = state
        .db
        .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    let baseline_target_definition = stored_target
        .as_ref()
        .map(|provider| provider_definition_proof(provider, proof_id))
        .unwrap_or_else(|| missing_target_definition_proof(proof_id));
    let target_definition = provider_definition_proof(&credential.persisted_provider, proof_id);
    let (live_projection_state, live_projection) =
        live_projection_proof(read_live_settings(AppType::Codex), proof_id);
    let target_projection = intended_codex_projection_proof(state, credential, proof_id)?;
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        effective_current_provider_id,
        target: credential.persisted_provider.clone(),
        live_projection_state,
        proxy_takeover_active: codex_proxy_takeover_active(state)?,
        private_proof: PrivateProjectionProof {
            current_definition,
            baseline_target_definition,
            target_definition,
            live_projection,
            target_projection,
        },
    })
}

fn inspect_codex_upsert_readback(
    state: &AppState,
    credential: &PrivateCodexCredentialPlan,
    proof_id: &str,
) -> Result<CodexSwitchInspection, ChangePlanErrorCode> {
    let mut inspection = inspect_codex_upsert_precheck(state, credential, proof_id)?;
    let actual = state
        .db
        .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    inspection.target = actual
        .clone()
        .unwrap_or_else(|| credential.persisted_provider.clone());
    inspection.private_proof.target_definition = actual
        .as_ref()
        .map(|provider| provider_definition_proof(provider, proof_id))
        .unwrap_or_else(|| missing_target_definition_proof(proof_id));
    Ok(inspection)
}

fn inspect_without_private_proof(
    state: &AppState,
    target_provider_id: &str,
) -> Result<CodexSwitchInspection, ChangePlanErrorCode> {
    let (db_current_provider_id, device_current_provider_id, effective_current_provider_id) =
        current_codex_provider_ids(state)?;
    let target = state
        .db
        .get_provider_by_id(target_provider_id, AppType::Codex.as_str())
        .map_err(|_| ChangePlanErrorCode::Internal)?
        .unwrap_or_else(|| {
            Provider::with_id(
                target_provider_id.to_string(),
                "Provider".to_string(),
                Value::Null,
                None,
            )
        });
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        effective_current_provider_id,
        target,
        live_projection_state: CodexLiveBaselineState::Unavailable,
        proxy_takeover_active: false,
        private_proof: PrivateProjectionProof {
            current_definition: None,
            baseline_target_definition: [0; 32],
            target_definition: [0; 32],
            live_projection: [0; 32],
            target_projection: [0; 32],
        },
    })
}

pub(crate) fn registered_adapter_descriptor(operation: ChangeOperation) -> ChangeAdapterDescriptor {
    adapter::descriptor_for_operation(operation)
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

const EXECUTION_CANCEL_SAFE: u8 = 0;
const EXECUTION_WRITE_CLAIMED: u8 = 1;
const EXECUTION_CANCELLED: u8 = 2;
const EXECUTION_TERMINAL: u8 = 3;
const SECRET_CLEANUP_OK: u8 = 0;
const OLD_SECRET_CLEANUP_WARNING: u8 = 1;
const NEW_SECRET_CLEANUP_FAILED: u8 = 2;

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
            Ok(_) => ChangeCancelCode::Accepted,
            Err(EXECUTION_CANCELLED) => ChangeCancelCode::Accepted,
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

fn lock_active_executions() -> std::sync::MutexGuard<'static, HashMap<String, Arc<ExecutionGate>>> {
    active_executions()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct ActiveExecutionRegistration {
    job_id: String,
    gate: Arc<ExecutionGate>,
}

impl ActiveExecutionRegistration {
    fn register(job_id: &str) -> Result<Self, ChangePlanErrorCode> {
        let mut executions = lock_active_executions();
        if executions.contains_key(job_id) {
            return Err(ChangePlanErrorCode::Internal);
        }
        let gate = Arc::new(ExecutionGate::new());
        executions.insert(job_id.to_string(), gate.clone());
        Ok(Self {
            job_id: job_id.to_string(),
            gate,
        })
    }
}

impl Drop for ActiveExecutionRegistration {
    fn drop(&mut self) {
        lock_active_executions().remove(&self.job_id);
    }
}

fn active_execution_gate(job_id: &str) -> Option<Arc<ExecutionGate>> {
    lock_active_executions().get(job_id).cloned()
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
    let target_projection = match materialize_codex_secret_ref_provider_from_keyring(&target)
        .map_err(|_| ChangePlanErrorCode::BaselineUnavailable)?
    {
        Some(live) => intended_codex_projection_proof_for_live_provider(state, &live, proof_id)?,
        None => {
            let effective_target = build_effective_settings_with_common_config(
                state.db.as_ref(),
                &AppType::Codex,
                &target,
            )
            .map_err(|_| ChangePlanErrorCode::Internal)?;
            private_revision_json(
                "fyagent.change-plan.codex-live.v1",
                proof_id,
                &codex_projection_for_digest(effective_target),
            )
        }
    };
    Ok(CodexSwitchInspection {
        db_current_provider_id,
        device_current_provider_id,
        effective_current_provider_id,
        target,
        live_projection_state,
        proxy_takeover_active,
        private_proof: PrivateProjectionProof {
            current_definition,
            baseline_target_definition: target_definition,
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

fn workbuddy_baseline_binding_digest(
    plan_id: &str,
    proof_id: &str,
    epoch_id: &str,
    snapshot: &WorkBuddyChangeSnapshot,
) -> Result<String, ChangePlanErrorCode> {
    let value = serde_json::json!({
        "contract": CHANGE_PLAN_CONTRACT_VERSION,
        "proofId": proof_id,
        "processEpochId": epoch_id,
        "exists": snapshot.status.exists,
        "modelCount": snapshot.status.model_count,
        "backupExists": snapshot.status.backup_exists,
        "format": snapshot.status.format,
    });
    Ok(opaque_revision_json(
        "fyagent.change-plan.workbuddy-baseline-binding.v1",
        plan_id,
        &value,
    ))
}

fn workbuddy_snapshot_matches(
    left: &WorkBuddyChangeSnapshot,
    right: &WorkBuddyChangeSnapshot,
) -> bool {
    workbuddy_primary_matches(left, right)
        && left.status.backup_exists == right.status.backup_exists
}

fn workbuddy_primary_matches(
    left: &WorkBuddyChangeSnapshot,
    right: &WorkBuddyChangeSnapshot,
) -> bool {
    left.status.exists == right.status.exists
        && left.status.model_count == right.status.model_count
        && left.status.format == right.status.format
        && left.model_ids == right.model_ids
        && optional_text_matches(
            left.status.revision.as_deref(),
            right.status.revision.as_deref(),
        )
}

fn optional_text_matches(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => constant_time_text_matches(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn map_workbuddy_plan_error(
    error: crate::services::workbuddy::error::WorkBuddyError,
) -> ChangePlanErrorCode {
    use crate::services::workbuddy::error::WorkBuddyErrorCode;

    match error.code() {
        WorkBuddyErrorCode::ConfigConcurrentModification => ChangePlanErrorCode::Stale,
        WorkBuddyErrorCode::InvalidUrl
        | WorkBuddyErrorCode::ApiKeyRequired
        | WorkBuddyErrorCode::ConfigInvalidEntry
        | WorkBuddyErrorCode::ConfigNoTargetModels
        | WorkBuddyErrorCode::OverwriteTokenInvalid
        | WorkBuddyErrorCode::OverwriteTokenExpired => ChangePlanErrorCode::InvalidRequest,
        WorkBuddyErrorCode::ConfigReadFailed
        | WorkBuddyErrorCode::ConfigInvalidJson
        | WorkBuddyErrorCode::ConfigRootUnsupported
        | WorkBuddyErrorCode::ConfigModelsNotArray => ChangePlanErrorCode::BaselineUnavailable,
        _ => ChangePlanErrorCode::Internal,
    }
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
        business_steps: &plan.business_steps,
        credential: &plan.credential,
        current_provider_code: &plan.current_provider_code,
        target_provider_code: &plan.target_provider_code,
        restart_expectation: plan.restart_expectation,
        risks: &plan.risks,
        evidence_note: &plan.evidence_note,
        adapter: &plan.adapter,
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
    ) && constant_time_revision_matches(
        &left.baseline_target_definition,
        &right.baseline_target_definition,
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

fn apply_codex_provider_upsert_writer(
    state: &AppState,
    credential: &Arc<PrivateCodexCredentialPlan>,
) -> Result<bool, ()> {
    credential
        .cleanup_state
        .store(SECRET_CLEANUP_OK, Ordering::SeqCst);
    let secrets = SecretService::new(NativeSecretBackend::new());
    let created = secrets
        .create_reserved(
            &credential.handle,
            &credential.material,
            SecretPurpose::CodexApiKey,
        )
        .map_err(|error| {
            log::error!("Codex SecretRef create failed: {:?}", error.code());
        })?;
    if created.backend_kind() != SecretBackendKind::OsKeyring {
        let _ = secrets.delete(&credential.handle);
        return Err(());
    }

    let live_provider = match materialize_codex_secret_ref_provider(
        &credential.persisted_provider,
        &credential.material,
    ) {
        Ok(provider) => provider,
        Err(_) => {
            if secrets.delete(&credential.handle).is_err() {
                credential
                    .cleanup_state
                    .store(NEW_SECRET_CLEANUP_FAILED, Ordering::SeqCst);
            }
            return Err(());
        }
    };
    let result = crate::services::ProviderService::apply_quick_setup_with_secret_ref_lock_held(
        state,
        credential.persisted_provider.clone(),
        live_provider,
    );
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            log::error!("Codex UCP Provider writer failed: {error}");
            if secrets.delete(&credential.handle).is_err() {
                credential
                    .cleanup_state
                    .store(NEW_SECRET_CLEANUP_FAILED, Ordering::SeqCst);
            }
            return Err(());
        }
    };

    if let Some(previous) = credential.previous_handle.as_ref() {
        if previous.secret_ref() != credential.handle.secret_ref()
            && secrets.delete(previous).is_err()
        {
            credential
                .cleanup_state
                .store(OLD_SECRET_CLEANUP_WARNING, Ordering::SeqCst);
            log::warn!("Codex previous SecretRef cleanup failed after verified Provider commit");
        }
    }
    Ok(result.live_config_changed)
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
        let adapter = CodexProviderSwitchAdapter::for_plan(state, target_provider_id, &proof_id);
        let inspection = adapter.inspect()?;
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
        let plan_fields = adapter.plan(&inspection);
        let mut public = ChangePlan {
            plan_id,
            operation: ChangeOperation::CodexProviderSwitch,
            target_provider_id: target_provider_id.to_string(),
            target_provider_name: plan_fields.target_provider_name,
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
            business_steps: vec![ChangeBusinessStepKind::SetCurrentProvider],
            credential: None,
            adapter: adapter.descriptor(),
            current_provider_code: plan_fields.current_provider_code,
            target_provider_code: plan_fields.target_provider_code,
            restart_expectation: plan_fields.restart_expectation,
            risks: plan_fields.risks,
            evidence_note: plan_fields.evidence_note,
        };
        public.plan_digest = plan_approval_binding_digest(
            &public,
            &proof_id,
            &epoch_id,
            CHANGE_PLAN_CONTRACT_VERSION,
        )?;
        drop(adapter);
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
                credential: None,
                workbuddy: None,
            },
            now,
        );
        Ok(public)
    }

    pub fn plan_codex_provider_upsert(
        state: &AppState,
        request: CodexProviderUpsertPlanRequest,
    ) -> Result<ChangePlan, ChangePlanErrorCode> {
        Self::plan_codex_provider_upsert_at(state, request, chrono::Utc::now().timestamp())
    }

    fn plan_codex_provider_upsert_at(
        state: &AppState,
        request: CodexProviderUpsertPlanRequest,
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
        let credential = Arc::new(build_codex_credential_plan(state, request)?);
        let adapter = CodexProviderUpsertAdapter::for_plan(state, &proof_id, credential.clone());
        let inspection = adapter.inspect()?;
        if inspection.proxy_takeover_active {
            return Err(ChangePlanErrorCode::UnsupportedOperation);
        }
        if inspection.live_projection_state == CodexLiveBaselineState::Unavailable {
            return Err(ChangePlanErrorCode::BaselineUnavailable);
        }
        let baseline_digest = baseline_binding_digest(&plan_id, &proof_id, &epoch_id, &inspection)?;
        let plan_fields = adapter.plan(&inspection);
        let mut public = ChangePlan {
            plan_id,
            operation: ChangeOperation::CodexProviderUpsertAndSwitch,
            target_provider_id: UCP_CODEX_PROVIDER_ID.to_string(),
            target_provider_name: plan_fields.target_provider_name,
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
            business_steps: vec![
                ChangeBusinessStepKind::SaveProvider,
                ChangeBusinessStepKind::SetCurrentProvider,
            ],
            credential: Some(ChangeCredentialProjection {
                secret_ref_display: credential.handle.secret_ref().display_ref(),
                backend: ChangeSecretBackend::OsKeyring,
            }),
            adapter: adapter.descriptor(),
            current_provider_code: plan_fields.current_provider_code,
            target_provider_code: plan_fields.target_provider_code,
            restart_expectation: plan_fields.restart_expectation,
            risks: plan_fields.risks,
            evidence_note: plan_fields.evidence_note,
        };
        public.plan_digest = plan_approval_binding_digest(
            &public,
            &proof_id,
            &epoch_id,
            CHANGE_PLAN_CONTRACT_VERSION,
        )?;
        drop(adapter);
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
                credential: Some(credential),
                workbuddy: None,
            },
            now,
        );
        Ok(public)
    }

    pub fn plan_workbuddy_models(
        state: &AppState,
        request: WorkBuddyModelsPlanRequest,
    ) -> Result<ChangePlan, ChangePlanErrorCode> {
        Self::plan_workbuddy_models_at(state, request, chrono::Utc::now().timestamp())
    }

    fn plan_workbuddy_models_at(
        state: &AppState,
        request: WorkBuddyModelsPlanRequest,
        now: i64,
    ) -> Result<ChangePlan, ChangePlanErrorCode> {
        let workbuddy_guard = workbuddy::lock_workbuddy_mutation();
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        let request = request.into_save_request();
        let preview = workbuddy::preview_workbuddy_change_locked(&workbuddy_guard, &request)
            .map_err(map_workbuddy_plan_error)?;
        let plan_id = uuid::Uuid::new_v4().to_string();
        let proof_id = uuid::Uuid::new_v4().to_string();
        let epoch_id = process_epoch_id().to_string();
        let private = Arc::new(PrivateWorkBuddyPlan {
            request,
            baseline: preview.baseline,
            target_model_count: preview.target_model_count,
            existing_target_count: preview.existing_target_count,
        });
        let adapter = WorkBuddyModelsAdapter::for_plan(&workbuddy_guard, private.clone());
        let inspection = private.baseline.clone();
        let baseline_digest =
            workbuddy_baseline_binding_digest(&plan_id, &proof_id, &epoch_id, &inspection)?;
        let plan_fields = adapter.plan(&inspection);
        let mut public = ChangePlan {
            plan_id,
            operation: ChangeOperation::WorkBuddyModelsUpdate,
            target_provider_id: "workbuddy-models".to_string(),
            target_provider_name: plan_fields.target_provider_name,
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
            business_steps: vec![ChangeBusinessStepKind::SaveWorkBuddyModels],
            credential: None,
            adapter: adapter.descriptor(),
            current_provider_code: plan_fields.current_provider_code,
            target_provider_code: plan_fields.target_provider_code,
            restart_expectation: plan_fields.restart_expectation,
            risks: plan_fields.risks,
            evidence_note: plan_fields.evidence_note,
        };
        public.plan_digest = plan_approval_binding_digest(
            &public,
            &proof_id,
            &epoch_id,
            CHANGE_PLAN_CONTRACT_VERSION,
        )?;
        drop(adapter);
        state
            .db
            .insert_change_plan(&StoredChangePlan {
                public: public.clone(),
                proof_id: proof_id.clone(),
                process_epoch_id: epoch_id,
                current_provider_id: None,
                contract_digest: CHANGE_PLAN_CONTRACT_VERSION.to_string(),
            })
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        register_private_proof(
            proof_id,
            PrivatePlanProof {
                projection: PrivateProjectionProof {
                    current_definition: None,
                    baseline_target_definition: [0; 32],
                    target_definition: [0; 32],
                    live_projection: [0; 32],
                    target_projection: [0; 32],
                },
                expires_at: public.expires_at,
                credential: None,
                workbuddy: Some(private),
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
        Self::apply_codex_switch_with_observer(state, plan_id, plan_digest, |_| {})
    }

    pub fn apply_change_plan_with_observer<O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        observer: O,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let Some(stored) = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
        else {
            return Ok(rejected(ChangePlanErrorCode::PlanNotFound));
        };
        match stored.public.operation {
            ChangeOperation::CodexProviderSwitch => {
                Self::apply_codex_switch_with_observer(state, plan_id, plan_digest, observer)
            }
            ChangeOperation::CodexProviderUpsertAndSwitch => {
                let Some(credential) =
                    get_private_proof(&stored.proof_id).and_then(|proof| proof.credential)
                else {
                    return Ok(rejected(ChangePlanErrorCode::Stale));
                };
                let writer_credential = credential.clone();
                Self::apply_codex_switch_at_with_writer_observer_and_fault(
                    state,
                    plan_id,
                    plan_digest,
                    chrono::Utc::now().timestamp(),
                    move || apply_codex_provider_upsert_writer(state, &writer_credential),
                    observer,
                    None,
                )
            }
            ChangeOperation::WorkBuddyModelsUpdate => {
                Self::apply_workbuddy_with_observer_and_fault(
                    state,
                    plan_id,
                    plan_digest,
                    chrono::Utc::now().timestamp(),
                    observer,
                    None,
                )
            }
        }
    }

    pub fn apply_codex_switch_with_observer<O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        observer: O,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        let now = chrono::Utc::now().timestamp();
        Self::apply_codex_switch_at_with_writer_observer_and_fault(
            state,
            plan_id,
            plan_digest,
            now,
            || {
                crate::services::ProviderService::with_live_config_result(AppType::Codex, || {
                    crate::services::ProviderService::switch_with_lock_held(
                        state,
                        AppType::Codex,
                        &state
                            .db
                            .get_stored_change_plan(plan_id)
                            .map_err(|_| AppError::Message("change plan unavailable".to_string()))?
                            .ok_or_else(|| {
                                AppError::Message("change plan unavailable".to_string())
                            })?
                            .public
                            .target_provider_id,
                    )
                })
                .map(|result| result.live_config_changed)
                .map_err(|_| ())
            },
            observer,
            None,
        )
    }

    #[cfg(test)]
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
            return Ok(Some(rejected(ChangePlanErrorCode::PlanNotFound)));
        };
        if !constant_time_text_matches(&stored.public.plan_digest, plan_digest) {
            return Ok(Some(rejected(ChangePlanErrorCode::InvalidDigest)));
        }
        if stored.public.status != ChangePlanStatus::Consumed {
            return Ok(None);
        }
        if stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION
            || stored.public.adapter != registered_adapter_descriptor(stored.public.operation)
        {
            return Ok(Some(rejected(ChangePlanErrorCode::Stale)));
        }
        let rebound_digest = plan_approval_binding_digest(
            &stored.public,
            &stored.proof_id,
            &stored.process_epoch_id,
            &stored.contract_digest,
        )?;
        if !constant_time_text_matches(&stored.public.plan_digest, &rebound_digest) {
            return Ok(Some(rejected(ChangePlanErrorCode::Stale)));
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

    pub(crate) fn apply_codex_switch_at_with_writer_observer_and_fault<F, O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        writer: F,
        observer: O,
        injected_fault: Option<ChangeFaultPoint>,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        F: FnOnce() -> Result<bool, ()>,
        O: Fn(&ChangeJobSnapshot),
    {
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let _provider_guard =
            crate::services::ProviderService::lock_provider_mutation(state, &AppType::Codex);
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
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
        if now >= stored.public.expires_at {
            return Ok(rejected(ChangePlanErrorCode::Expired));
        }
        if stored.process_epoch_id != process_epoch_id() {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        if stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        if stored.public.adapter != registered_adapter_descriptor(stored.public.operation) {
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
        let mut adapter = RegisteredCodexAdapter::for_execution(
            state,
            stored.public.operation,
            &stored.public.target_provider_id,
            &stored.proof_id,
            expected_private.credential.clone(),
            writer,
        )?;
        if adapter.descriptor() != stored.public.adapter
            || adapter.compensation_capability() != stored.public.adapter.compensation_mode
        {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let observed = adapter.precheck()?;
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
        let execution = ActiveExecutionRegistration::register(&job_id)?;
        let admitted = state
            .db
            .admit_change_plan(plan_id, plan_digest, &observed_baseline, &job_id, now)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if admitted.kind == ChangeApplyOutcomeKind::Rejected {
            return Ok(admitted);
        }
        let mut job = admitted.job.ok_or(ChangePlanErrorCode::Internal)?;
        normalize_job_projection(&mut job);
        observer(&job);

        set_step(
            &mut job,
            ChangeStepKind::Precheck,
            ChangeStepStatus::Succeeded,
            "baseline_matched",
        );
        advance_job(&mut job, now, "precheck_succeeded");
        persist_job(state, &mut job, "precheck_succeeded", &observer)?;

        let captured_snapshot = adapter.snapshot(&observed);
        if !private_proof_matches(&captured_snapshot, &expected_private.projection) {
            return Err(ChangePlanErrorCode::Internal);
        }
        set_step(
            &mut job,
            ChangeStepKind::Snapshot,
            ChangeStepStatus::Succeeded,
            "snapshot_bound",
        );
        advance_job(&mut job, now, "snapshot_succeeded");
        persist_job(state, &mut job, "snapshot_succeeded", &observer)?;

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
            terminal_job(
                &mut job,
                now,
                ChangeJobStatus::Cancelled,
                ChangeResultCode::CancelledBeforeWrite,
                RestartRequirement::NotRequired,
                RecoveryState::NotNeeded,
                "cancelled_before_write",
            );
            persist_job(state, &mut job, "cancelled_before_write", &observer)?;
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
        advance_job(&mut job, now, "managed_write_started");
        persist_job(state, &mut job, "managed_write_started", &observer)?;

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
        advance_job(&mut job, now, "readback_started");
        persist_job(state, &mut job, "readback_started", &observer)?;

        let readback = adapter.readback();
        finalize_readback(
            state,
            &stored,
            Some(&expected_private.projection),
            expected_private
                .credential
                .as_ref()
                .map(|credential| credential.cleanup_state.load(Ordering::SeqCst))
                .unwrap_or(SECRET_CLEANUP_OK),
            &mut job,
            WriterObservation::Returned(writer_result),
            readback,
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

    fn apply_workbuddy_with_observer_and_fault<O>(
        state: &AppState,
        plan_id: &str,
        plan_digest: &str,
        now: i64,
        observer: O,
        injected_fault: Option<ChangeFaultPoint>,
    ) -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let workbuddy_guard = workbuddy::lock_workbuddy_mutation();
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if let Some(existing) = Self::idempotent_replay_if_consumed(state, plan_id, plan_digest)? {
            return Ok(existing);
        }
        let Some(stored) = state
            .db
            .get_stored_change_plan(plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
        else {
            return Ok(rejected(ChangePlanErrorCode::PlanNotFound));
        };
        if stored.public.operation != ChangeOperation::WorkBuddyModelsUpdate {
            return Ok(rejected(ChangePlanErrorCode::UnsupportedOperation));
        }
        if !constant_time_text_matches(&stored.public.plan_digest, plan_digest) {
            return Ok(rejected(ChangePlanErrorCode::InvalidDigest));
        }
        if now >= stored.public.expires_at {
            return Ok(rejected(ChangePlanErrorCode::Expired));
        }
        if stored.process_epoch_id != process_epoch_id()
            || stored.contract_digest != CHANGE_PLAN_CONTRACT_VERSION
            || stored.public.adapter != registered_adapter_descriptor(stored.public.operation)
        {
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
        let Some(private) = get_private_proof(&stored.proof_id).and_then(|proof| proof.workbuddy)
        else {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        };
        let writer_plan = private.clone();
        let guard_for_writer = &workbuddy_guard;
        let mut adapter =
            WorkBuddyModelsAdapter::for_execution(&workbuddy_guard, private.clone(), move || {
                Ok(match workbuddy::apply_workbuddy_change_locked(
                    guard_for_writer,
                    &writer_plan.request,
                ) {
                    Ok(workbuddy::types::SaveWorkBuddyModelsOutcome::Saved {
                        revision, ..
                    }) => WorkBuddyWriteReceipt::Applied { revision },
                    Ok(workbuddy::types::SaveWorkBuddyModelsOutcome::ConcurrentModification) => {
                        WorkBuddyWriteReceipt::NoWrite
                    }
                    Ok(workbuddy::types::SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired {
                        ..
                    })
                    | Err(_) => WorkBuddyWriteReceipt::UnknownOutcome,
                })
            });
        if adapter.descriptor() != stored.public.adapter
            || adapter.compensation_capability() != stored.public.adapter.compensation_mode
        {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let observed = adapter.precheck()?;
        if !workbuddy_snapshot_matches(&private.baseline, &observed) {
            return Ok(rejected(ChangePlanErrorCode::Stale));
        }
        let observed_baseline = workbuddy_baseline_binding_digest(
            &stored.public.plan_id,
            &stored.proof_id,
            &stored.process_epoch_id,
            &observed,
        )?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let execution = ActiveExecutionRegistration::register(&job_id)?;
        let admitted = state
            .db
            .admit_change_plan(plan_id, plan_digest, &observed_baseline, &job_id, now)
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        if admitted.kind == ChangeApplyOutcomeKind::Rejected {
            return Ok(admitted);
        }
        let mut job = admitted.job.ok_or(ChangePlanErrorCode::Internal)?;
        normalize_job_projection(&mut job);
        observer(&job);

        set_step(
            &mut job,
            ChangeStepKind::Precheck,
            ChangeStepStatus::Succeeded,
            "baseline_matched",
        );
        advance_job(&mut job, now, "precheck_succeeded");
        persist_job(state, &mut job, "precheck_succeeded", &observer)?;

        let captured_snapshot = adapter.snapshot(&observed);
        if !workbuddy_snapshot_matches(&captured_snapshot, &private.baseline) {
            return Err(ChangePlanErrorCode::Internal);
        }
        set_step(
            &mut job,
            ChangeStepKind::Snapshot,
            ChangeStepStatus::Succeeded,
            "snapshot_bound",
        );
        advance_job(&mut job, now, "snapshot_succeeded");
        persist_job(state, &mut job, "snapshot_succeeded", &observer)?;

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
            terminal_job(
                &mut job,
                now,
                ChangeJobStatus::Cancelled,
                ChangeResultCode::CancelledBeforeWrite,
                RestartRequirement::NotRequired,
                RecoveryState::NotNeeded,
                "cancelled_before_write",
            );
            persist_job(state, &mut job, "cancelled_before_write", &observer)?;
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
        advance_job(&mut job, now, "managed_write_started");
        persist_job(state, &mut job, "managed_write_started", &observer)?;
        let writer_result = adapter.managed_write();
        if injected_fault == Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord) {
            return Err(ChangePlanErrorCode::Internal);
        }
        let writer_applied = matches!(&writer_result, Ok(WorkBuddyWriteReceipt::Applied { .. }));
        set_step(
            &mut job,
            ChangeStepKind::ManagedWrite,
            if writer_applied {
                ChangeStepStatus::Succeeded
            } else {
                ChangeStepStatus::Failed
            },
            if writer_applied {
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
        advance_job(&mut job, now, "readback_started");
        persist_job(state, &mut job, "readback_started", &observer)?;

        let mut readback = adapter.readback();
        let mut target_matches =
            workbuddy::workbuddy_target_matches_locked(&workbuddy_guard, &private.request)
                .unwrap_or(false);
        let baseline_matches = readback
            .as_ref()
            .is_ok_and(|snapshot| workbuddy_primary_matches(&private.baseline, snapshot));
        let may_have_written = !matches!(&writer_result, Ok(WorkBuddyWriteReceipt::NoWrite));
        let exact_target = match (&writer_result, &readback) {
            (Ok(WorkBuddyWriteReceipt::Applied { revision }), Ok(snapshot)) => snapshot
                .status
                .revision
                .as_deref()
                .is_some_and(|actual| constant_time_text_matches(actual, revision)),
            _ => false,
        };
        let mut restored = false;
        if may_have_written && !exact_target && !target_matches && !baseline_matches {
            set_step(
                &mut job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Compensating,
                "restoring_baseline",
            );
            advance_job(&mut job, now, "compensation_started");
            persist_job(state, &mut job, "compensation_started", &observer)?;
            restored = workbuddy::restore_workbuddy_change_snapshot_locked(
                &workbuddy_guard,
                &private.baseline,
            )
            .unwrap_or(false);
            readback = adapter.readback();
            target_matches = false;
        }
        finalize_workbuddy_readback(
            state,
            &private,
            &mut job,
            writer_result,
            readback,
            target_matches,
            restored,
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

    pub fn get_job(
        state: &AppState,
        job_id: &str,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode> {
        Self::get_job_with_observer(state, job_id, |_| {})
    }

    pub fn get_job_with_observer<O>(
        state: &AppState,
        job_id: &str,
        observer: O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        let mut job = state
            .db
            .get_change_job(job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        normalize_job_projection(&mut job);
        if job.status.is_terminal() || active_execution_gate(job_id).is_some() {
            return Ok(job);
        }
        Self::reconcile_job(state, job, &observer)
    }

    fn reconcile_job<O>(
        state: &AppState,
        job: ChangeJobSnapshot,
        observer: &O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        let stored = state
            .db
            .get_stored_change_plan(&job.plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::PlanNotFound)?;
        match stored.public.operation {
            ChangeOperation::WorkBuddyModelsUpdate => {
                Self::reconcile_workbuddy_job(state, job, observer)
            }
            ChangeOperation::CodexProviderSwitch
            | ChangeOperation::CodexProviderUpsertAndSwitch => {
                Self::reconcile_codex_job(state, job, observer)
            }
        }
    }

    fn reconcile_codex_job<O>(
        state: &AppState,
        mut job: ChangeJobSnapshot,
        observer: &O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
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
            normalize_job_projection(&mut job);
            return Ok(job);
        }
        let stored = state
            .db
            .get_stored_change_plan(&job.plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::PlanNotFound)?;
        let now = chrono::Utc::now().timestamp();
        let managed_write_started = job.steps.iter().any(|step| {
            step.kind == ChangeStepKind::ManagedWrite
                && step.status != ChangeStepStatus::NotStarted
                && step.status != ChangeStepStatus::Skipped
        });
        if !managed_write_started {
            set_step(
                &mut job,
                ChangeStepKind::Readback,
                ChangeStepStatus::Skipped,
                "write_never_started",
            );
            set_step(
                &mut job,
                ChangeStepKind::Finalize,
                ChangeStepStatus::Succeeded,
                "interrupted_before_write",
            );
            terminal_job(
                &mut job,
                now,
                ChangeJobStatus::Failed,
                ChangeResultCode::InterruptedBeforeWrite,
                RestartRequirement::NotRequired,
                RecoveryState::NotNeeded,
                "interrupted_before_write",
            );
            persist_job(state, &mut job, "interrupted_before_write", observer)?;
            return Ok(job);
        }

        set_step(
            &mut job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Running,
            "reconcile_readback_started",
        );
        advance_job(&mut job, now, "reconcile_readback_started");
        persist_job(state, &mut job, "reconcile_readback_started", observer)?;
        let private_proof = (stored.process_epoch_id == process_epoch_id())
            .then(|| get_private_proof(&stored.proof_id))
            .flatten();
        let readback = match private_proof.as_ref() {
            Some(proof) => RegisteredCodexAdapter::for_readback(
                state,
                stored.public.operation,
                &stored.public.target_provider_id,
                &stored.proof_id,
                proof.credential.clone(),
            )?
            .readback(),
            None => inspect_without_private_proof(state, &stored.public.target_provider_id),
        };
        finalize_readback(
            state,
            &stored,
            private_proof.as_ref().map(|proof| &proof.projection),
            private_proof
                .as_ref()
                .and_then(|proof| proof.credential.as_ref())
                .map(|credential| credential.cleanup_state.load(Ordering::SeqCst))
                .unwrap_or(SECRET_CLEANUP_OK),
            &mut job,
            WriterObservation::Unknown,
            readback,
            now,
            observer,
        )?;
        Ok(job)
    }

    fn reconcile_workbuddy_job<O>(
        state: &AppState,
        mut job: ChangeJobSnapshot,
        observer: &O,
    ) -> Result<ChangeJobSnapshot, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        let workbuddy_guard = workbuddy::lock_workbuddy_mutation();
        let _guard = change_plan_lock()
            .lock()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        job = state
            .db
            .get_change_job(&job.job_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::JobNotFound)?;
        if job.status.is_terminal() {
            normalize_job_projection(&mut job);
            return Ok(job);
        }
        let stored = state
            .db
            .get_stored_change_plan(&job.plan_id)
            .map_err(|_| ChangePlanErrorCode::Internal)?
            .ok_or(ChangePlanErrorCode::PlanNotFound)?;
        let now = chrono::Utc::now().timestamp();
        let managed_write_started = job.steps.iter().any(|step| {
            step.kind == ChangeStepKind::ManagedWrite
                && step.status != ChangeStepStatus::NotStarted
                && step.status != ChangeStepStatus::Skipped
        });
        if !managed_write_started {
            set_step(
                &mut job,
                ChangeStepKind::Readback,
                ChangeStepStatus::Skipped,
                "write_never_started",
            );
            set_step(
                &mut job,
                ChangeStepKind::Finalize,
                ChangeStepStatus::Succeeded,
                "interrupted_before_write",
            );
            terminal_job(
                &mut job,
                now,
                ChangeJobStatus::Failed,
                ChangeResultCode::InterruptedBeforeWrite,
                RestartRequirement::NotRequired,
                RecoveryState::NotNeeded,
                "interrupted_before_write",
            );
            persist_job(state, &mut job, "interrupted_before_write", observer)?;
            return Ok(job);
        }

        set_step(
            &mut job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Running,
            "reconcile_readback_started",
        );
        advance_job(&mut job, now, "reconcile_readback_started");
        persist_job(state, &mut job, "reconcile_readback_started", observer)?;
        let private = (stored.process_epoch_id == process_epoch_id())
            .then(|| get_private_proof(&stored.proof_id))
            .flatten()
            .and_then(|proof| proof.workbuddy);
        let Some(private) = private else {
            set_resource(
                &mut job,
                ChangeResourceKind::WorkBuddyModelsConfig,
                ChangeResourceStatus::Unavailable,
                "private_proof_unavailable",
            );
            set_resource(
                &mut job,
                ChangeResourceKind::WorkBuddyBackup,
                ChangeResourceStatus::Unavailable,
                "private_proof_unavailable",
            );
            set_step(
                &mut job,
                ChangeStepKind::Readback,
                ChangeStepStatus::Failed,
                "private_proof_unavailable",
            );
            set_step(
                &mut job,
                ChangeStepKind::Finalize,
                ChangeStepStatus::Succeeded,
                "recovery_required",
            );
            terminal_job(
                &mut job,
                now,
                ChangeJobStatus::Failed,
                ChangeResultCode::RecoveryRequired,
                RestartRequirement::Unknown,
                RecoveryState::RecoveryRequired,
                "private_proof_unavailable",
            );
            persist_job(state, &mut job, "recovery_required", observer)?;
            return Ok(job);
        };

        let adapter = WorkBuddyModelsAdapter::for_plan(&workbuddy_guard, private.clone());
        let mut readback = adapter.readback();
        let baseline_matches = readback
            .as_ref()
            .is_ok_and(|snapshot| workbuddy_primary_matches(&private.baseline, snapshot));
        let mut target_matches =
            workbuddy::workbuddy_target_matches_locked(&workbuddy_guard, &private.request)
                .unwrap_or(false);
        let mut restored = false;
        if !baseline_matches && !target_matches {
            set_step(
                &mut job,
                ChangeStepKind::ManagedWrite,
                ChangeStepStatus::Compensating,
                "reconcile_restoring_baseline",
            );
            advance_job(&mut job, now, "compensation_started");
            persist_job(state, &mut job, "compensation_started", observer)?;
            restored = workbuddy::restore_workbuddy_change_snapshot_locked(
                &workbuddy_guard,
                &private.baseline,
            )
            .unwrap_or(false);
            readback = adapter.readback();
            target_matches = false;
        }
        finalize_workbuddy_readback(
            state,
            &private,
            &mut job,
            Ok(WorkBuddyWriteReceipt::UnknownOutcome),
            readback,
            target_matches,
            restored,
            now,
            observer,
        )?;
        Ok(job)
    }

    pub fn list_recoverable_jobs(
        state: &AppState,
    ) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode> {
        Self::list_recoverable_jobs_with_observer(state, |_| {})
    }

    pub fn list_recoverable_jobs_with_observer<O>(
        state: &AppState,
        observer: O,
    ) -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode>
    where
        O: Fn(&ChangeJobSnapshot),
    {
        let jobs = state
            .db
            .list_recoverable_change_jobs()
            .map_err(|_| ChangePlanErrorCode::Internal)?;
        jobs.into_iter()
            .map(|mut job| {
                normalize_job_projection(&mut job);
                if active_execution_gate(&job.job_id).is_some() {
                    Ok(job)
                } else {
                    Self::reconcile_job(state, job, &observer)
                }
            })
            .collect()
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
}

#[derive(Clone, Copy)]
enum WriterObservation {
    Returned(Result<bool, ()>),
    Unknown,
}

fn persist_job<O>(
    state: &AppState,
    job: &mut ChangeJobSnapshot,
    event_code: &str,
    observer: &O,
) -> Result<(), ChangePlanErrorCode>
where
    O: Fn(&ChangeJobSnapshot),
{
    normalize_job_projection(job);
    state
        .db
        .save_change_job(job, event_code)
        .map_err(|_| ChangePlanErrorCode::Internal)?;
    observer(job);
    Ok(())
}

fn terminal_job(
    job: &mut ChangeJobSnapshot,
    now: i64,
    status: ChangeJobStatus,
    result_code: ChangeResultCode,
    restart_requirement: RestartRequirement,
    recovery_state: RecoveryState,
    diagnostic_code: &str,
) {
    job.revision += 1;
    job.event_seq += 1;
    job.status = status;
    job.result_code = result_code;
    job.restart_requirement = restart_requirement;
    job.recovery_state = recovery_state;
    job.diagnostic_code = Some(diagnostic_code.to_string());
    job.updated_at = now;
    normalize_job_projection(job);
}

#[allow(clippy::too_many_arguments)]
fn finalize_readback<O>(
    state: &AppState,
    stored: &StoredChangePlan,
    expected_private: Option<&PrivateProjectionProof>,
    secret_cleanup_state: u8,
    job: &mut ChangeJobSnapshot,
    writer_observation: WriterObservation,
    readback: Result<CodexSwitchInspection, ChangePlanErrorCode>,
    now: i64,
    observer: &O,
) -> Result<(), ChangePlanErrorCode>
where
    O: Fn(&ChangeJobSnapshot),
{
    classify_job(
        stored,
        expected_private,
        secret_cleanup_state,
        job,
        writer_observation,
        readback,
    );
    let final_status = job.status;
    let final_result = job.result_code;
    let final_restart = job.restart_requirement;
    let final_recovery = job.recovery_state;
    let final_diagnostic = job.diagnostic_code.clone();
    let final_live_changed = job.live_config_changed;

    set_step(
        job,
        ChangeStepKind::Finalize,
        ChangeStepStatus::Running,
        "finalize_started",
    );
    advance_job(job, now, "finalize_started");
    persist_job(state, job, "finalize_started", observer)?;

    job.revision += 1;
    job.event_seq += 1;
    job.status = final_status;
    job.result_code = final_result;
    job.restart_requirement = final_restart;
    job.recovery_state = final_recovery;
    job.diagnostic_code = final_diagnostic;
    job.live_config_changed = final_live_changed;
    job.updated_at = now;
    set_step(
        job,
        ChangeStepKind::Finalize,
        ChangeStepStatus::Succeeded,
        "finalized",
    );
    persist_job(state, job, "terminal", observer)
}

#[allow(clippy::too_many_arguments)]
fn finalize_workbuddy_readback<O>(
    state: &AppState,
    private: &PrivateWorkBuddyPlan,
    job: &mut ChangeJobSnapshot,
    writer_result: Result<WorkBuddyWriteReceipt, ()>,
    readback: Result<WorkBuddyChangeSnapshot, ChangePlanErrorCode>,
    semantic_target_matches: bool,
    restored: bool,
    now: i64,
    observer: &O,
) -> Result<(), ChangePlanErrorCode>
where
    O: Fn(&ChangeJobSnapshot),
{
    classify_workbuddy_job(
        private,
        job,
        writer_result,
        readback,
        semantic_target_matches,
        restored,
    );
    let final_status = job.status;
    let final_result = job.result_code;
    let final_restart = job.restart_requirement;
    let final_recovery = job.recovery_state;
    let final_diagnostic = job.diagnostic_code.clone();

    set_step(
        job,
        ChangeStepKind::Finalize,
        ChangeStepStatus::Running,
        "finalize_started",
    );
    advance_job(job, now, "finalize_started");
    persist_job(state, job, "finalize_started", observer)?;

    job.revision += 1;
    job.event_seq += 1;
    job.status = final_status;
    job.result_code = final_result;
    job.restart_requirement = final_restart;
    job.recovery_state = final_recovery;
    job.diagnostic_code = final_diagnostic;
    job.live_config_changed = false;
    job.updated_at = now;
    set_step(
        job,
        ChangeStepKind::Finalize,
        ChangeStepStatus::Succeeded,
        "finalized",
    );
    persist_job(state, job, "terminal", observer)
}

fn classify_workbuddy_job(
    private: &PrivateWorkBuddyPlan,
    job: &mut ChangeJobSnapshot,
    writer_result: Result<WorkBuddyWriteReceipt, ()>,
    readback: Result<WorkBuddyChangeSnapshot, ChangePlanErrorCode>,
    semantic_target_matches: bool,
    restored: bool,
) {
    let Ok(readback) = readback else {
        set_resource(
            job,
            ChangeResourceKind::WorkBuddyModelsConfig,
            ChangeResourceStatus::Unavailable,
            "workbuddy_readback_unavailable",
        );
        set_resource(
            job,
            ChangeResourceKind::WorkBuddyBackup,
            ChangeResourceStatus::Unavailable,
            "workbuddy_backup_unavailable",
        );
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::ReadbackUnavailable;
        job.restart_requirement = RestartRequirement::Unknown;
        job.recovery_state = RecoveryState::RecoveryRequired;
        job.diagnostic_code = Some("workbuddy_readback_unavailable".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "readback_unavailable",
        );
        return;
    };

    let exact_target = match &writer_result {
        Ok(WorkBuddyWriteReceipt::Applied { revision }) => readback
            .status
            .revision
            .as_deref()
            .is_some_and(|actual| constant_time_text_matches(actual, revision)),
        _ => false,
    };
    let baseline = workbuddy_primary_matches(&private.baseline, &readback);
    let target = exact_target || semantic_target_matches;
    set_resource(
        job,
        ChangeResourceKind::WorkBuddyModelsConfig,
        if target || baseline {
            ChangeResourceStatus::Matched
        } else {
            ChangeResourceStatus::Mismatched
        },
        if target {
            "workbuddy_target_matched"
        } else if baseline {
            "workbuddy_baseline_restored"
        } else {
            "workbuddy_config_mismatched"
        },
    );
    set_resource(
        job,
        ChangeResourceKind::WorkBuddyBackup,
        if target || baseline {
            ChangeResourceStatus::Matched
        } else if readback.status.backup_exists {
            ChangeResourceStatus::Mismatched
        } else {
            ChangeResourceStatus::Unavailable
        },
        if target {
            if private.baseline.status.exists {
                "workbuddy_backup_observed"
            } else {
                "workbuddy_backup_not_required"
            }
        } else if baseline && restored {
            "workbuddy_recovery_snapshot_applied"
        } else if baseline {
            "workbuddy_baseline_backup_state_observed"
        } else if readback.status.backup_exists {
            "workbuddy_backup_requires_inspection"
        } else {
            "workbuddy_backup_unavailable"
        },
    );

    if exact_target {
        job.status = ChangeJobStatus::Succeeded;
        job.result_code = ChangeResultCode::Applied;
        job.restart_requirement = RestartRequirement::NotRequired;
        job.recovery_state = RecoveryState::NotNeeded;
        job.diagnostic_code = Some("workbuddy_target_readback_matched".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "target_matched",
        );
    } else if target {
        job.status = ChangeJobStatus::Warning;
        job.result_code = ChangeResultCode::RecoveredTargetReached;
        job.restart_requirement = RestartRequirement::NotRequired;
        job.recovery_state = RecoveryState::NotNeeded;
        job.diagnostic_code = Some("workbuddy_target_recovered_by_readback".to_string());
        set_step(
            job,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Succeeded,
            "target_reached_after_unknown_outcome",
        );
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "target_matched",
        );
    } else if baseline {
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::WriterFailedBaselineRestored;
        job.restart_requirement = RestartRequirement::NotRequired;
        job.recovery_state = RecoveryState::Succeeded;
        job.diagnostic_code = Some("workbuddy_baseline_restored".to_string());
        set_step(
            job,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Compensated,
            "writer_rollback_observed",
        );
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Succeeded,
            "baseline_restored",
        );
    } else {
        job.status = ChangeJobStatus::Failed;
        job.result_code = ChangeResultCode::PostWriteMismatch;
        job.restart_requirement = RestartRequirement::Unknown;
        job.recovery_state = RecoveryState::RecoveryRequired;
        job.diagnostic_code = Some("workbuddy_recovery_required".to_string());
        set_step(
            job,
            ChangeStepKind::Readback,
            ChangeStepStatus::Failed,
            "state_mixed",
        );
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

fn adapter_error_for_result(result: ChangeResultCode) -> Option<ChangeAdapterErrorCode> {
    match result {
        ChangeResultCode::WriterFailedBaselineRestored => Some(ChangeAdapterErrorCode::Permanent),
        ChangeResultCode::WriterErrorTargetReached => Some(ChangeAdapterErrorCode::Transient),
        ChangeResultCode::PostWriteMismatch => Some(ChangeAdapterErrorCode::VerifyFailed),
        ChangeResultCode::ReadbackUnavailable | ChangeResultCode::RecoveryRequired => {
            Some(ChangeAdapterErrorCode::UnknownOutcome)
        }
        ChangeResultCode::InterruptedBeforeWrite => Some(ChangeAdapterErrorCode::Transient),
        _ => None,
    }
}

fn normalize_job_projection(job: &mut ChangeJobSnapshot) {
    job.execution_id.clone_from(&job.job_id);
    job.idempotency_key.clone_from(&job.plan_id);
    job.adapter_error_code = adapter_error_for_result(job.result_code);

    let needs_partial = matches!(
        job.status,
        ChangeJobStatus::Running | ChangeJobStatus::Warning | ChangeJobStatus::Failed
    ) || job.recovery_state == RecoveryState::RecoveryRequired;
    if !needs_partial {
        job.partial_result = None;
        return;
    }

    let succeeded_steps = job
        .steps
        .iter()
        .filter(|step| step.status == ChangeStepStatus::Succeeded)
        .map(|step| step.kind)
        .collect();
    let compensated_steps = job
        .steps
        .iter()
        .filter(|step| step.status == ChangeStepStatus::Compensated)
        .map(|step| step.kind)
        .collect();
    let mut unverified_steps = Vec::new();
    let managed_write = job
        .steps
        .iter()
        .find(|step| step.kind == ChangeStepKind::ManagedWrite);
    let readback_succeeded = job.steps.iter().any(|step| {
        step.kind == ChangeStepKind::Readback && step.status == ChangeStepStatus::Succeeded
    });
    if managed_write.is_some_and(|step| {
        matches!(
            step.status,
            ChangeStepStatus::Running | ChangeStepStatus::Succeeded | ChangeStepStatus::Failed
        )
    }) && !readback_succeeded
    {
        unverified_steps.push(ChangeStepKind::ManagedWrite);
    }
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
            .map(|resource| resource.code.clone())
            .collect()
    };
    let mut manual_actions = Vec::new();
    if job.recovery_state == RecoveryState::RecoveryRequired {
        manual_actions.push("inspect_and_resolve".to_string());
    }
    if job.result_code == ChangeResultCode::ReadbackUnavailable {
        manual_actions.push("restore_readback_authority".to_string());
    }
    if job.diagnostic_code.as_deref() == Some("old_secret_cleanup_failed") {
        manual_actions.push("delete_previous_secret_ref".to_string());
    }
    if job.diagnostic_code.as_deref() == Some("new_secret_cleanup_failed") {
        manual_actions.push("inspect_secret_store".to_string());
    }
    job.partial_result = Some(ChangePartialResult {
        succeeded_steps,
        compensated_steps,
        unverified_steps,
        remaining_effects,
        manual_actions,
    });
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
    secret_cleanup_state: u8,
    job: &mut ChangeJobSnapshot,
    writer_observation: WriterObservation,
    readback: Result<CodexSwitchInspection, ChangePlanErrorCode>,
) {
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
    let baseline_target_definition = optional_revision_matches(
        Some(&readback.private_proof.target_definition),
        Some(&expected_private.baseline_target_definition),
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
        match writer_observation {
            WriterObservation::Returned(Ok(live_config_changed)) => {
                job.live_config_changed = live_config_changed;
                job.restart_requirement = if live_config_changed {
                    RestartRequirement::Recommended
                } else {
                    RestartRequirement::NotRequired
                };
                job.status = ChangeJobStatus::Succeeded;
                job.result_code = if live_config_changed {
                    ChangeResultCode::AppliedRestartRecommended
                } else {
                    ChangeResultCode::Applied
                };
            }
            WriterObservation::Returned(Err(())) => {
                job.live_config_changed = false;
                job.restart_requirement = RestartRequirement::Recommended;
                job.status = ChangeJobStatus::Warning;
                job.result_code = ChangeResultCode::WriterErrorTargetReached;
            }
            WriterObservation::Unknown => {
                job.live_config_changed = false;
                job.restart_requirement = RestartRequirement::Recommended;
                job.status = ChangeJobStatus::Warning;
                job.result_code = ChangeResultCode::RecoveredTargetReached;
                set_step(
                    job,
                    ChangeStepKind::ManagedWrite,
                    ChangeStepStatus::Succeeded,
                    "target_reached_after_unknown_outcome",
                );
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
        if secret_cleanup_state == OLD_SECRET_CLEANUP_WARNING {
            job.status = ChangeJobStatus::Warning;
            job.result_code = ChangeResultCode::AppliedWithWarning;
            job.diagnostic_code = Some("old_secret_cleanup_failed".to_string());
        }
    } else if baseline_db
        && baseline_device
        && baseline_current_definition
        && baseline_live
        && baseline_target_definition
    {
        set_step(
            job,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Compensated,
            "writer_rollback_observed",
        );
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
        if secret_cleanup_state == NEW_SECRET_CLEANUP_FAILED {
            job.recovery_state = RecoveryState::RecoveryRequired;
            job.result_code = ChangeResultCode::RecoveryRequired;
            job.diagnostic_code = Some("new_secret_cleanup_failed".to_string());
        }
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

    #[cfg(any(target_os = "macos", windows))]
    struct NativeSecretCleanup(Option<SecretHandle>);

    #[cfg(any(target_os = "macos", windows))]
    impl Drop for NativeSecretCleanup {
        fn drop(&mut self) {
            if let Some(handle) = self.0.take() {
                let _ = SecretService::new(NativeSecretBackend::new()).delete(&handle);
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
            business_steps: vec![ChangeBusinessStepKind::SetCurrentProvider],
            credential: None,
            adapter: registered_adapter_descriptor(ChangeOperation::CodexProviderSwitch),
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
            business_steps: vec![ChangeBusinessStepKind::SetCurrentProvider],
            credential: None,
            adapter: registered_adapter_descriptor(ChangeOperation::CodexProviderSwitch),
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
            execution_id: "job-contract".into(),
            idempotency_key: "plan-contract".into(),
            plan_id: "plan-contract".into(),
            target_provider_id: "provider-target".into(),
            revision: 7,
            event_seq: 7,
            status: ChangeJobStatus::Succeeded,
            result_code: ChangeResultCode::AppliedRestartRecommended,
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
                    status: ChangeStepStatus::Succeeded,
                    code: "writer_returned".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Readback,
                    status: ChangeStepStatus::Succeeded,
                    code: "target_matched".into(),
                },
                ChangeJobStep {
                    kind: ChangeStepKind::Finalize,
                    status: ChangeStepStatus::Succeeded,
                    code: "finalized".into(),
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
            adapter_error_code: None,
            partial_result: None,
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
        let cancel_outcome = CancelChangeJobOutcome {
            accepted: true,
            code: ChangeCancelCode::Accepted,
            job_id: "job-contract".into(),
        };
        assert_eq!(
            serde_json::to_value(cancel_outcome).unwrap(),
            fixture["cancelOutcome"]
        );
        let partial = ChangePartialResult {
            succeeded_steps: vec![ChangeStepKind::Precheck, ChangeStepKind::Snapshot],
            compensated_steps: vec![ChangeStepKind::ManagedWrite],
            ..ChangePartialResult::default()
        };
        assert_eq!(
            serde_json::to_value(partial).unwrap(),
            fixture["partialResult"]
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
        assert_eq!(
            serde_json::to_value(ChangeStepStatus::Compensating).unwrap(),
            fixture["reservedStatuses"]["step"][1]
        );
        assert_eq!(
            serde_json::to_value(ChangeStepStatus::Compensated).unwrap(),
            fixture["reservedStatuses"]["step"][2]
        );
        for (index, code) in [
            ChangeAdapterErrorCode::PreconditionFailed,
            ChangeAdapterErrorCode::Transient,
            ChangeAdapterErrorCode::Permanent,
            ChangeAdapterErrorCode::UnknownOutcome,
            ChangeAdapterErrorCode::VerifyFailed,
            ChangeAdapterErrorCode::CompensationFailed,
            ChangeAdapterErrorCode::Unsupported,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                serde_json::to_value(code).unwrap(),
                fixture["reservedStatuses"]["adapterError"][index]
            );
        }
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

    fn codex_upsert_request(api_key: &str) -> CodexProviderUpsertPlanRequest {
        CodexProviderUpsertPlanRequest {
            name: "Planned Codex".to_string(),
            base_url: "https://gateway.example.test/v1".to_string(),
            api_key: api_key.to_string(),
            model_id: "gpt-planned".to_string(),
            codex_features: Some(crate::codex_config::CodexProviderFeatureIntent {
                image_extension: Some(true),
                websockets: Some(true),
            }),
        }
    }

    #[test]
    #[serial]
    fn codex_provider_upsert_preview_writes_only_one_safe_ledger_row() {
        let (_home, _guard, db, state, current, _target) = setup_switch_state();
        let before_live = read_live_settings(AppType::Codex).unwrap();
        let before_provider_count = db.get_all_providers(AppType::Codex.as_str()).unwrap().len();
        let canary = "ucp-preview-secret-canary";

        let plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request(canary),
            100,
        )
        .unwrap();
        assert_eq!(
            plan.operation,
            ChangeOperation::CodexProviderUpsertAndSwitch
        );
        assert_eq!(
            plan.business_steps,
            vec![
                ChangeBusinessStepKind::SaveProvider,
                ChangeBusinessStepKind::SetCurrentProvider,
            ]
        );
        let credential = plan.credential.as_ref().expect("credential projection");
        assert!(credential.secret_ref_display.starts_with("sec_…"));
        assert_eq!(credential.backend, ChangeSecretBackend::OsKeyring);
        let public_json = serde_json::to_string(&plan).unwrap();
        assert!(!public_json.contains(canary));
        assert!(!public_json.contains("OPENAI_API_KEY"));
        assert!(!public_json.contains("secretVersion"));

        assert_eq!(
            db.get_all_providers(AppType::Codex.as_str()).unwrap().len(),
            before_provider_count,
            "preview must not save a Provider"
        );
        assert!(db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .is_none());
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            Some(current.id.clone())
        );
        assert_eq!(read_live_settings(AppType::Codex).unwrap(), before_live);
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());

        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let private = get_private_proof(&stored.proof_id).unwrap();
        let private_credential = private.credential.expect("private credential");
        assert!(private_credential.material.ct_eq_slice(canary.as_bytes()));
        let conn = db.conn.lock().unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM change_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1, "preview may insert exactly one UCP ledger row");
        let persisted_public: String = conn
            .query_row(
                "SELECT operation || target_provider_name || plan_digest || baseline_digest || business_steps_json || COALESCE(credential_json, '') || risks_json FROM change_plans WHERE plan_id=?1",
                [plan.plan_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!persisted_public.contains(canary));
        assert!(!persisted_public.contains(private_credential.handle.secret_ref().as_str()));
    }

    #[test]
    #[serial]
    fn codex_provider_upsert_executes_one_existing_writer_and_rereads_safe_target() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, db, state, _current, _target) = setup_switch_state();
        let canary = "ucp-upsert-execution-secret-canary";
        let plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request(canary),
            100,
        )
        .unwrap();
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let credential = get_private_proof(&stored.proof_id)
            .and_then(|proof| proof.credential)
            .expect("private credential");
        let calls = AtomicUsize::new(0);
        let observed_event_seq = Mutex::new(Vec::new());

        let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                let live = materialize_codex_secret_ref_provider(
                    &credential.persisted_provider,
                    &credential.material,
                )
                .map_err(|_| ())?;
                crate::services::ProviderService::apply_quick_setup_with_secret_ref_lock_held(
                    &state,
                    credential.persisted_provider.clone(),
                    live,
                )
                .map(|result| result.live_config_changed)
                .map_err(|_| ())
            },
            |snapshot| observed_event_seq.lock().unwrap().push(snapshot.event_seq),
            None,
        )
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let job = outcome.job.expect("admitted job");
        assert_eq!(job.status, ChangeJobStatus::Succeeded, "{job:#?}");
        assert!(matches!(
            job.result_code,
            ChangeResultCode::Applied | ChangeResultCode::AppliedRestartRecommended
        ));
        assert_eq!(job.usage_evidence, UsageEvidence::NotObserved);
        assert_eq!(
            *observed_event_seq.lock().unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7]
        );
        assert!(job
            .steps
            .iter()
            .all(|step| step.status == ChangeStepStatus::Succeeded));
        assert!(job
            .resources
            .iter()
            .all(|resource| resource.status == ChangeResourceStatus::Matched));

        let persisted = db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .expect("safe Provider");
        let persisted_json = serde_json::to_string(&persisted).unwrap();
        assert!(!persisted_json.contains(canary));
        assert!(!persisted_json.contains("OPENAI_API_KEY"));
        assert!(persisted_json.contains("secretRef"));
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str())
                .unwrap()
                .as_deref(),
            Some(UCP_CODEX_PROVIDER_ID)
        );
        let live_json =
            serde_json::to_string(&read_live_settings(AppType::Codex).unwrap()).unwrap();
        assert!(live_json.contains(canary));

        clear_private_proofs_for_test();
        let replay = ChangePlanService::apply_change_plan_with_observer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            |_| {},
        )
        .unwrap();
        assert_eq!(replay.kind, ChangeApplyOutcomeKind::IdempotentReplay);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[serial]
    fn interrupted_upsert_without_private_proof_requires_recovery_without_replay() {
        let (_home, _guard, db, state, _current, _target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request("ucp-interrupted-secret-canary"),
            100,
        )
        .unwrap();
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let credential = get_private_proof(&stored.proof_id)
            .and_then(|proof| proof.credential)
            .expect("private credential before simulated restart");
        let inspected = inspect_codex_upsert_precheck(&state, &credential, &stored.proof_id)
            .expect("upsert baseline");
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
                "interrupted-upsert-job",
                101,
            )
            .unwrap();
        let mut interrupted = admitted.job.expect("admitted job");
        set_step(
            &mut interrupted,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Running,
            "managed_write_started",
        );
        advance_job(&mut interrupted, 101, "managed_write_started");
        db.save_change_job(&interrupted, "managed_write_started")
            .unwrap();
        let before_provider_count = db.get_all_providers(AppType::Codex.as_str()).unwrap().len();

        clear_private_proofs_for_test();
        let reconciled = ChangePlanService::get_job(&state, "interrupted-upsert-job").unwrap();

        assert_eq!(reconciled.status, ChangeJobStatus::Failed);
        assert_eq!(reconciled.result_code, ChangeResultCode::RecoveryRequired);
        assert_eq!(reconciled.recovery_state, RecoveryState::RecoveryRequired);
        assert_eq!(
            reconciled.diagnostic_code.as_deref(),
            Some("private_proof_unavailable")
        );
        assert_eq!(
            db.get_all_providers(AppType::Codex.as_str()).unwrap().len(),
            before_provider_count,
            "reconcile must never replay the Provider writer"
        );
        assert!(reconciled.resources.iter().any(|resource| {
            resource.kind == ChangeResourceKind::TargetDefinition
                && resource.status == ChangeResourceStatus::Unavailable
        }));
    }

    #[test]
    #[serial]
    fn codex_provider_upsert_dispatcher_returns_safe_rejections_before_any_writer() {
        let (_home, _guard, db, state, _current, _target) = setup_switch_state();
        let missing = ChangePlanService::apply_change_plan_with_observer(
            &state,
            "missing-plan",
            "missing-digest",
            |_| {},
        )
        .unwrap();
        assert_eq!(missing.error_code, Some(ChangePlanErrorCode::PlanNotFound));

        let plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request("ucp-dispatcher-secret-canary"),
            100,
        )
        .unwrap();
        let before_provider_count = db.get_all_providers(AppType::Codex.as_str()).unwrap().len();
        clear_private_proofs_for_test();
        let stale = ChangePlanService::apply_change_plan_with_observer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            |_| {},
        )
        .unwrap();
        assert_eq!(stale.error_code, Some(ChangePlanErrorCode::Stale));
        assert_eq!(
            db.get_all_providers(AppType::Codex.as_str()).unwrap().len(),
            before_provider_count
        );
        assert!(db.list_recoverable_change_jobs().unwrap().is_empty());
    }

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    #[ignore = "native OS keyring HIL; run explicitly on a real macOS or Windows host"]
    #[serial]
    fn codex_provider_upsert_native_keyring_hil() {
        use crate::services::secret::{MaterialMatches, SecretAvailability, SecretPresence};

        let (_home, _guard, db, state, _current, _target) = setup_switch_state();
        let canary = format!("fyagent-issue63-hil-{}", uuid::Uuid::new_v4().simple());
        let plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request(&canary),
            chrono::Utc::now().timestamp(),
        )
        .expect("native upsert plan");
        let stored = db.get_stored_change_plan(&plan.plan_id).unwrap().unwrap();
        let handle = get_private_proof(&stored.proof_id)
            .and_then(|proof| proof.credential)
            .map(|credential| credential.handle.clone())
            .expect("private keyring handle");
        let mut cleanup = NativeSecretCleanup(Some(handle.clone()));
        let observed_event_seq = Mutex::new(Vec::new());

        let outcome = ChangePlanService::apply_change_plan_with_observer(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            |snapshot| observed_event_seq.lock().unwrap().push(snapshot.event_seq),
        )
        .expect("native upsert apply");
        let job = outcome.job.expect("admitted native job");
        assert_eq!(job.status, ChangeJobStatus::Succeeded, "{job:#?}");
        assert_eq!(
            *observed_event_seq.lock().unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(job.usage_evidence, UsageEvidence::NotObserved);
        assert!(job
            .steps
            .iter()
            .all(|step| step.status == ChangeStepStatus::Succeeded));
        assert!(job
            .resources
            .iter()
            .all(|resource| resource.status == ChangeResourceStatus::Matched));

        let secrets = SecretService::new(NativeSecretBackend::new());
        let probe = secrets
            .probe(&handle, SecretPurpose::CodexApiKey)
            .expect("probe native secret");
        assert_eq!(probe.presence(), SecretPresence::Present);
        assert_eq!(probe.availability(), SecretAvailability::Ready);
        assert!(secrets
            .with_material(&handle, MaterialMatches::new(canary.as_bytes()))
            .expect("read native secret through sealed callback"));

        let persisted = db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .expect("safe persisted Provider");
        let persisted_json = serde_json::to_string(&persisted).unwrap();
        assert!(!persisted_json.contains(&canary));
        assert!(!persisted_json.contains("OPENAI_API_KEY"));
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str())
                .unwrap()
                .as_deref(),
            Some(UCP_CODEX_PROVIDER_ID)
        );
        assert!(
            serde_json::to_string(&read_live_settings(AppType::Codex).unwrap())
                .unwrap()
                .contains(&canary)
        );

        let failed_canary = format!(
            "fyagent-issue63-failed-edit-hil-{}",
            uuid::Uuid::new_v4().simple()
        );
        let failed_plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request(&failed_canary),
            chrono::Utc::now().timestamp(),
        )
        .expect("native failed-edit plan");
        let failed_stored = db
            .get_stored_change_plan(&failed_plan.plan_id)
            .unwrap()
            .unwrap();
        let failed_handle = get_private_proof(&failed_stored.proof_id)
            .and_then(|proof| proof.credential)
            .map(|credential| credential.handle.clone())
            .expect("failed-edit private keyring handle");
        let mut failed_cleanup = NativeSecretCleanup(Some(failed_handle.clone()));
        db.conn
            .lock()
            .expect("lock database")
            .execute_batch(&format!(
                "CREATE TRIGGER fail_ucp_codex_provider_update\n\
                 BEFORE UPDATE ON providers\n\
                 WHEN NEW.id = '{UCP_CODEX_PROVIDER_ID}'\n\
                   AND NEW.app_type = 'codex'\n\
                   AND NEW.settings_config LIKE '%{}%'\n\
                 BEGIN\n\
                   SELECT RAISE(ABORT, 'injected UCP Codex Provider update failure');\n\
                 END;",
                failed_handle.secret_ref().as_str()
            ))
            .expect("install failed-edit trigger");

        let failed_edit = ChangePlanService::apply_change_plan_with_observer(
            &state,
            &failed_plan.plan_id,
            &failed_plan.plan_digest,
            |_| {},
        )
        .expect("failed native edit must still return a terminal snapshot")
        .job
        .expect("admitted failed native edit job");
        assert_eq!(
            failed_edit.status,
            ChangeJobStatus::Failed,
            "{failed_edit:#?}"
        );
        assert_eq!(
            failed_edit.result_code,
            ChangeResultCode::WriterFailedBaselineRestored
        );
        assert_eq!(failed_edit.recovery_state, RecoveryState::Succeeded);
        assert_eq!(
            failed_edit.diagnostic_code.as_deref(),
            Some("baseline_restored")
        );
        assert!(secrets
            .with_material(&handle, MaterialMatches::new(canary.as_bytes()))
            .expect("original secret must survive a failed edit"));
        let failed_probe = secrets
            .probe(&failed_handle, SecretPurpose::CodexApiKey)
            .expect("probe failed-edit native secret");
        assert_eq!(failed_probe.presence(), SecretPresence::Missing);
        assert_eq!(failed_probe.availability(), SecretAvailability::Missing);
        failed_cleanup.0 = None;
        let after_failed_persisted = db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .expect("baseline Provider after failed edit");
        assert_eq!(
            after_failed_persisted.settings_config["auth"]["secretRef"].as_str(),
            Some(handle.secret_ref().as_str())
        );
        let after_failed_live =
            serde_json::to_string(&read_live_settings(AppType::Codex).unwrap()).unwrap();
        assert!(after_failed_live.contains(&canary));
        assert!(!after_failed_live.contains(&failed_canary));
        db.conn
            .lock()
            .expect("lock database")
            .execute_batch("DROP TRIGGER fail_ucp_codex_provider_update;")
            .expect("remove failed-edit trigger");

        let rotated_canary = format!("fyagent-issue63-edit-hil-{}", uuid::Uuid::new_v4().simple());
        let edit_plan = ChangePlanService::plan_codex_provider_upsert_at(
            &state,
            codex_upsert_request(&rotated_canary),
            chrono::Utc::now().timestamp(),
        )
        .expect("native edit plan");
        let edit_stored = db
            .get_stored_change_plan(&edit_plan.plan_id)
            .unwrap()
            .unwrap();
        let rotated_handle = get_private_proof(&edit_stored.proof_id)
            .and_then(|proof| proof.credential)
            .map(|credential| credential.handle.clone())
            .expect("rotated private keyring handle");
        let mut rotated_cleanup = NativeSecretCleanup(Some(rotated_handle.clone()));
        let edit = ChangePlanService::apply_change_plan_with_observer(
            &state,
            &edit_plan.plan_id,
            &edit_plan.plan_digest,
            |_| {},
        )
        .expect("native edit apply")
        .job
        .expect("admitted native edit job");
        assert_eq!(edit.status, ChangeJobStatus::Succeeded, "{edit:#?}");

        let deleted = secrets
            .probe(&handle, SecretPurpose::CodexApiKey)
            .expect("probe rotated old native secret");
        assert_eq!(deleted.presence(), SecretPresence::Missing);
        assert_eq!(deleted.availability(), SecretAvailability::Missing);
        cleanup.0 = None;
        assert!(secrets
            .with_material(
                &rotated_handle,
                MaterialMatches::new(rotated_canary.as_bytes()),
            )
            .expect("read rotated native secret through sealed callback"));
        let rotated_persisted = db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .expect("rotated safe Provider");
        assert_eq!(
            rotated_persisted.settings_config["auth"]["secretRef"].as_str(),
            Some(rotated_handle.secret_ref().as_str())
        );
        let rotated_live =
            serde_json::to_string(&read_live_settings(AppType::Codex).unwrap()).unwrap();
        assert!(rotated_live.contains(&rotated_canary));
        assert!(!rotated_live.contains(&canary));

        crate::services::ProviderService::switch(&state, AppType::Codex, "target")
            .expect("switch away from SecretRef Provider");
        let backfilled_safe = db
            .get_provider_by_id(UCP_CODEX_PROVIDER_ID, AppType::Codex.as_str())
            .unwrap()
            .expect("backfilled safe Provider");
        let backfilled_json = serde_json::to_string(&backfilled_safe).unwrap();
        assert!(!backfilled_json.contains(&rotated_canary));
        assert!(!backfilled_json.contains("OPENAI_API_KEY"));
        assert_eq!(
            backfilled_safe.settings_config["auth"]["secretRef"].as_str(),
            Some(rotated_handle.secret_ref().as_str())
        );
        let switch_back = ChangePlanService::plan_codex_switch_at(
            &state,
            UCP_CODEX_PROVIDER_ID,
            chrono::Utc::now().timestamp(),
        )
        .expect("plan switch back to SecretRef Provider");
        let switched = ChangePlanService::apply_codex_switch(
            &state,
            &switch_back.plan_id,
            &switch_back.plan_digest,
        )
        .expect("apply existing Provider switch")
        .job
        .expect("admitted existing Provider switch");
        assert_eq!(switched.status, ChangeJobStatus::Succeeded, "{switched:#?}");
        let switched_live =
            serde_json::to_string(&read_live_settings(AppType::Codex).unwrap()).unwrap();
        assert!(switched_live.contains(&rotated_canary));
        assert!(!switched_live.contains(&canary));

        let receipt = secrets
            .delete(&rotated_handle)
            .expect("delete rotated native HIL secret");
        assert_eq!(
            serde_json::to_value(receipt).unwrap()["deleted"].as_bool(),
            Some(true)
        );
        rotated_cleanup.0 = None;
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

    fn workbuddy_request(
        expected_revision: Option<String>,
        api_key: &str,
    ) -> WorkBuddyModelsPlanRequest {
        WorkBuddyModelsPlanRequest {
            base_url: "https://new.example.test/v1".to_string(),
            api_key: api_key.to_string(),
            allow_no_api_key: false,
            selected_model_ids: vec!["model-a".to_string()],
            manual_model_ids: Vec::new(),
            removed_model_ids: Vec::new(),
            clear_existing_api_keys: false,
            expected_revision,
        }
    }

    fn setup_workbuddy_state() -> (
        tempfile::TempDir,
        TestHome,
        Arc<Database>,
        AppState,
        std::path::PathBuf,
        std::path::PathBuf,
        Vec<u8>,
        String,
    ) {
        let home = tempfile::tempdir().expect("test home");
        let home_guard = TestHome::set(home.path());
        let directory = home.path().join(".workbuddy");
        let models = directory.join("models.json");
        let backup = directory.join("models.json.backup");
        let original = br#"{"models":[{"id":"model-a","url":"https://old.example.test/v1","apiKey":"BASELINE-SECRET-CANARY"}],"unknown":{"kept":true}}"#.to_vec();
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&models, &original).unwrap();
        let workbuddy_guard = workbuddy::lock_workbuddy_mutation();
        let revision = workbuddy::inspect_workbuddy_change_locked(&workbuddy_guard)
            .unwrap()
            .status
            .revision
            .clone()
            .expect("existing WorkBuddy document has a revision");
        drop(workbuddy_guard);
        let db = Arc::new(Database::memory().expect("database"));
        let state = AppState::new(db.clone());
        (
            home, home_guard, db, state, models, backup, original, revision,
        )
    }

    fn change_ledger_counts(db: &Database) -> (i64, i64, i64) {
        db.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM change_plans),
                    (SELECT COUNT(*) FROM change_jobs),
                    (SELECT COUNT(*) FROM change_job_events)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }

    fn persisted_plan_text(db: &Database, plan_id: &str) -> String {
        db.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT operation, target_provider_id, target_provider_name, plan_digest,
                        baseline_digest, actor_code, source_version, proof_id,
                        process_epoch_id, COALESCE(current_provider_id, ''),
                        current_provider_code, target_provider_code, business_steps_json,
                        COALESCE(credential_json, ''), risks_json, restart_requirement,
                        contract_digest, status
                 FROM change_plans WHERE plan_id = ?1",
                [plan_id],
                |row| {
                    let mut fields = Vec::new();
                    for index in 0..18 {
                        fields.push(row.get::<_, String>(index)?);
                    }
                    Ok(fields.join("|"))
                },
            )
            .unwrap()
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
        assert_eq!(replay.kind, ChangeApplyOutcomeKind::IdempotentReplay);
        assert_eq!(
            replay.job.as_ref().map(|job| &job.job_id),
            Some(&job.job_id)
        );
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
        let mut interrupted = admitted.job.unwrap();
        set_step(
            &mut interrupted,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Running,
            "managed_write_started",
        );
        advance_job(&mut interrupted, 101, "managed_write_started");
        db.save_change_job(&interrupted, "managed_write_started")
            .unwrap();

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
        let mut interrupted = admitted.job.unwrap();
        set_step(
            &mut interrupted,
            ChangeStepKind::ManagedWrite,
            ChangeStepStatus::Running,
            "managed_write_started",
        );
        advance_job(&mut interrupted, 101, "managed_write_started");
        db.save_change_job(&interrupted, "managed_write_started")
            .unwrap();
        crate::services::ProviderService::switch(&state, AppType::Codex, &target.id).unwrap();
        let before = db.get_current_provider(AppType::Codex.as_str()).unwrap();
        let reconciled = ChangePlanService::get_job(&state, "interrupted-job").unwrap();
        assert_eq!(reconciled.status, ChangeJobStatus::Warning);
        assert_eq!(
            reconciled.result_code,
            ChangeResultCode::RecoveredTargetReached
        );
        assert_eq!(
            db.get_current_provider(AppType::Codex.as_str()).unwrap(),
            before
        );
        assert_eq!(
            reconciled
                .steps
                .iter()
                .find(|step| step.kind == ChangeStepKind::Finalize)
                .unwrap()
                .code,
            "finalized"
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

        let observed = ChangePlanService::reconcile_job(&state, stale, &|_| {}).unwrap();
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
        let baseline_partial = baseline.partial_result.as_ref().unwrap();
        assert_eq!(
            baseline_partial.compensated_steps,
            vec![ChangeStepKind::ManagedWrite]
        );
        assert!(baseline_partial.remaining_effects.is_empty());

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
        assert_eq!(
            mixed.adapter_error_code,
            Some(ChangeAdapterErrorCode::VerifyFailed)
        );
        assert_eq!(
            mixed.partial_result.unwrap().manual_actions,
            vec!["inspect_and_resolve"]
        );
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

    #[test]
    fn registered_adapter_contract_is_closed_and_has_no_dynamic_command_surface() {
        let descriptor = registered_adapter_descriptor(ChangeOperation::CodexProviderSwitch);
        assert_eq!(
            descriptor.phases,
            vec![
                ChangeStepKind::Precheck,
                ChangeStepKind::Snapshot,
                ChangeStepKind::ManagedWrite,
                ChangeStepKind::Readback,
                ChangeStepKind::Finalize,
            ]
        );
        assert_eq!(
            descriptor.compensation_mode,
            ChangeCompensationMode::WriterOwnedRollback
        );
        let serialized = serde_json::to_string(&descriptor).unwrap();
        for forbidden in ["shell", "script", "command", "argv", "settingsConfig"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    #[serial]
    fn concurrent_duplicate_apply_returns_one_execution_and_one_writer_call() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);
        let outcomes = std::thread::scope(|scope| {
            let handles = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        ChangePlanService::apply_codex_switch_at_with_writer(
                            &state,
                            &plan.plan_id,
                            &plan.plan_digest,
                            101,
                            || {
                                calls.fetch_add(1, Ordering::SeqCst);
                                crate::services::ProviderService::with_live_config_result(
                                    AppType::Codex,
                                    || {
                                        crate::services::ProviderService::switch_with_lock_held(
                                            &state,
                                            AppType::Codex,
                                            &target.id,
                                        )
                                    },
                                )
                                .map(|result| result.live_config_changed)
                                .map_err(|_| ())
                            },
                        )
                        .unwrap()
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });

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
            7
        );
        let job_ids = outcomes
            .iter()
            .map(|outcome| outcome.job.as_ref().unwrap().job_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(job_ids.len(), 1);
    }

    #[test]
    #[serial]
    fn cancellation_wins_only_before_the_managed_write_commit_point() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let cancellation_sent = AtomicBool::new(false);
        let calls = AtomicUsize::new(0);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            },
            |job| {
                let snapshot_ready = job.steps.iter().any(|step| {
                    step.kind == ChangeStepKind::Snapshot
                        && step.status == ChangeStepStatus::Succeeded
                });
                if snapshot_ready && !cancellation_sent.swap(true, Ordering::SeqCst) {
                    let cancelled = ChangePlanService::cancel_job(&state, &job.job_id).unwrap();
                    assert!(cancelled.accepted);
                    assert_eq!(cancelled.code, ChangeCancelCode::Accepted);
                }
            },
            None,
        )
        .unwrap();
        let job = outcome.job.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(job.status, ChangeJobStatus::Cancelled);
        assert_eq!(job.result_code, ChangeResultCode::CancelledBeforeWrite);
        assert_eq!(job.execution_id, job.job_id);
        assert_eq!(job.idempotency_key, plan.plan_id);

        let (_home, _guard, _db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
        let cancel_code = std::sync::Mutex::new(None);
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            201,
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
            |job| {
                let write_started = job.steps.iter().any(|step| {
                    step.kind == ChangeStepKind::ManagedWrite
                        && step.status == ChangeStepStatus::Running
                });
                if write_started && cancel_code.lock().unwrap().is_none() {
                    let cancelled = ChangePlanService::cancel_job(&state, &job.job_id).unwrap();
                    *cancel_code.lock().unwrap() = Some(cancelled.code);
                }
            },
            None,
        )
        .unwrap();
        assert!(outcome.job.unwrap().status.is_terminal());
        assert_eq!(
            *cancel_code.lock().unwrap(),
            Some(ChangeCancelCode::CommitPointPassed)
        );
    }

    #[test]
    #[serial]
    fn phase_events_follow_committed_snapshots_and_polling_returns_full_state() {
        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let observed = std::sync::Mutex::new(Vec::new());
        let outcome = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
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
            |job| {
                let persisted = db.get_change_job(&job.job_id).unwrap().unwrap();
                assert_eq!(persisted.event_seq, job.event_seq);
                observed.lock().unwrap().push(job.event_seq);
            },
            None,
        )
        .unwrap();
        let job = outcome.job.unwrap();
        assert_eq!(*observed.lock().unwrap(), vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            ChangePlanService::get_job(&state, &job.job_id).unwrap(),
            job
        );
    }

    #[test]
    #[serial]
    fn workbuddy_plan_is_one_safe_ledger_insert_and_zero_external_writes() {
        const REQUEST_KEY: &str = "REQUEST-SECRET-CANARY-WORKBUDDY";
        let (_home, _guard, db, state, models, backup, original, revision) =
            setup_workbuddy_state();

        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision.clone()), REQUEST_KEY),
            100,
        )
        .unwrap();

        assert_eq!(plan.operation, ChangeOperation::WorkBuddyModelsUpdate);
        assert_eq!(
            plan.business_steps,
            vec![ChangeBusinessStepKind::SaveWorkBuddyModels]
        );
        assert_eq!(
            plan.adapter.read_set,
            vec![
                ChangeResourceKind::WorkBuddyModelsConfig,
                ChangeResourceKind::WorkBuddyBackup,
            ]
        );
        assert_eq!(change_ledger_counts(&db), (1, 0, 0));
        assert_eq!(std::fs::read(&models).unwrap(), original);
        assert!(!backup.exists());

        let public = serde_json::to_string(&plan).unwrap();
        let persisted = persisted_plan_text(&db, &plan.plan_id);
        for forbidden in [
            REQUEST_KEY,
            "BASELINE-SECRET-CANARY",
            "https://new.example.test/v1",
            revision.as_str(),
            models.to_string_lossy().as_ref(),
            "overwriteToken",
        ] {
            assert!(
                !public.contains(forbidden),
                "public plan leaked {forbidden}"
            );
            assert!(
                !persisted.contains(forbidden),
                "persisted plan leaked {forbidden}"
            );
        }
        clear_private_proofs_for_test();
    }

    #[test]
    #[serial]
    fn workbuddy_apply_consumes_one_confirmation_and_is_idempotent() {
        const REQUEST_KEY: &str = "REQUEST-SECRET-CANARY-WORKBUDDY";
        let (_home, _guard, db, state, models, backup, original, revision) =
            setup_workbuddy_state();
        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision), REQUEST_KEY),
            100,
        )
        .unwrap();

        let outcome = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {},
            None,
        )
        .unwrap();
        let job = outcome.job.expect("admitted WorkBuddy job");
        assert_eq!(job.status, ChangeJobStatus::Succeeded);
        assert_eq!(job.result_code, ChangeResultCode::Applied);
        assert_eq!(job.restart_requirement, RestartRequirement::NotRequired);
        assert_eq!(job.usage_evidence, UsageEvidence::NotObserved);
        assert!(job
            .steps
            .iter()
            .all(|step| step.status == ChangeStepStatus::Succeeded));
        assert!(job
            .resources
            .iter()
            .all(|resource| resource.status == ChangeResourceStatus::Matched));
        assert_eq!(change_ledger_counts(&db).0, 1);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        let written = std::fs::read(&models).unwrap();
        let document: Value = serde_json::from_slice(&written).unwrap();
        assert_eq!(document["models"][0]["id"], "model-a");
        assert_eq!(document["models"][0]["url"], "https://new.example.test/v1");
        assert_eq!(document["models"][0]["apiKey"], REQUEST_KEY);
        assert_eq!(document["unknown"]["kept"], true);

        let replay = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            102,
            |_| {},
            None,
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(replay.job_id, job.job_id);
        assert_eq!(replay.event_seq, job.event_seq);
        assert_eq!(std::fs::read(&models).unwrap(), written);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        clear_private_proofs_for_test();
    }

    #[test]
    #[serial]
    fn workbuddy_api_key_only_drift_is_stale_and_writer_zero() {
        let (_home, _guard, db, state, models, backup, _original, revision) =
            setup_workbuddy_state();
        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision), "REQUEST-SECRET-CANARY-WORKBUDDY"),
            100,
        )
        .unwrap();
        let external = br#"{"models":[{"id":"model-a","url":"https://old.example.test/v1","apiKey":"EXTERNAL-SECRET-ONLY-DRIFT"}],"unknown":{"kept":true}}"#;
        std::fs::write(&models, external).unwrap();

        let outcome = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {},
            None,
        )
        .unwrap();
        assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
        assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
        assert!(outcome.job.is_none());
        assert_eq!(change_ledger_counts(&db), (1, 0, 0));
        assert_eq!(std::fs::read(&models).unwrap(), external);
        assert!(!backup.exists());
        clear_private_proofs_for_test();
    }

    #[test]
    #[serial]
    #[cfg(target_os = "macos")]
    fn workbuddy_postwrite_mismatch_restores_the_exact_baseline() {
        let (_home, _guard, _db, state, models, backup, original, revision) =
            setup_workbuddy_state();
        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision), "REQUEST-SECRET-CANARY-WORKBUDDY"),
            100,
        )
        .unwrap();
        crate::services::workbuddy::config::replace_after_commit_for_test(
            br#"{"models":[{"id":"mismatch","apiKey":"POSTWRITE-MISMATCH-CANARY"}]}"#.to_vec(),
        );

        let job = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {},
            None,
        )
        .unwrap()
        .job
        .unwrap();
        assert_eq!(job.status, ChangeJobStatus::Failed);
        assert_eq!(
            job.result_code,
            ChangeResultCode::WriterFailedBaselineRestored
        );
        assert_eq!(job.recovery_state, RecoveryState::Succeeded);
        assert_eq!(std::fs::read(&models).unwrap(), original);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert!(!String::from_utf8_lossy(&std::fs::read(&models).unwrap())
            .contains("POSTWRITE-MISMATCH-CANARY"));
        clear_private_proofs_for_test();
    }

    #[test]
    #[serial]
    fn workbuddy_crash_reconcile_never_replays_the_writer() {
        let (_home, _guard, db, state, models, backup, original, revision) =
            setup_workbuddy_state();
        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision), "REQUEST-SECRET-CANARY-WORKBUDDY"),
            100,
        )
        .unwrap();
        let interrupted = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            |_| {},
            Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord),
        );
        assert_eq!(interrupted.unwrap_err(), ChangePlanErrorCode::Internal);
        let pending = db
            .get_change_job_by_plan_id(&plan.plan_id)
            .unwrap()
            .unwrap();
        let written = std::fs::read(&models).unwrap();
        let recovered = ChangePlanService::get_job(&state, &pending.job_id).unwrap();
        assert_eq!(recovered.status, ChangeJobStatus::Warning);
        assert_eq!(
            recovered.result_code,
            ChangeResultCode::RecoveredTargetReached
        );
        assert_eq!(std::fs::read(&models).unwrap(), written);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert_eq!(
            ChangePlanService::get_job(&state, &pending.job_id)
                .unwrap()
                .event_seq,
            recovered.event_seq
        );
        assert_eq!(std::fs::read(&models).unwrap(), written);

        let (_home, _guard, db, state, models, backup, original, revision) =
            setup_workbuddy_state();
        let plan = ChangePlanService::plan_workbuddy_models_at(
            &state,
            workbuddy_request(Some(revision), "REQUEST-SECRET-CANARY-WORKBUDDY-RESTART"),
            200,
        )
        .unwrap();
        let interrupted = ChangePlanService::apply_workbuddy_with_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            201,
            |_| {},
            Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord),
        );
        assert_eq!(interrupted.unwrap_err(), ChangePlanErrorCode::Internal);
        let pending = db
            .get_change_job_by_plan_id(&plan.plan_id)
            .unwrap()
            .unwrap();
        let written = std::fs::read(&models).unwrap();
        clear_private_proofs_for_test();
        let recovery_required = ChangePlanService::get_job(&state, &pending.job_id).unwrap();
        assert_eq!(recovery_required.status, ChangeJobStatus::Failed);
        assert_eq!(
            recovery_required.result_code,
            ChangeResultCode::RecoveryRequired
        );
        assert_eq!(
            recovery_required.recovery_state,
            RecoveryState::RecoveryRequired
        );
        assert_eq!(std::fs::read(&models).unwrap(), written);
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert_eq!(
            ChangePlanService::get_job(&state, &pending.job_id)
                .unwrap()
                .event_seq,
            recovery_required.event_seq
        );
        assert_eq!(std::fs::read(&models).unwrap(), written);
    }

    #[test]
    #[serial]
    fn executor_fault_points_reconcile_without_replaying_the_writer() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 100).unwrap();
        let calls = AtomicUsize::new(0);
        let interrupted = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            101,
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(false)
            },
            |_| {},
            Some(ChangeFaultPoint::BeforeManagedWrite),
        );
        assert_eq!(interrupted.unwrap_err(), ChangePlanErrorCode::Internal);
        let pending = db
            .get_change_job_by_plan_id(&plan.plan_id)
            .unwrap()
            .unwrap();
        let recovered = ChangePlanService::get_job(&state, &pending.job_id).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            recovered.result_code,
            ChangeResultCode::InterruptedBeforeWrite
        );

        let (_home, _guard, db, state, _current, target) = setup_switch_state();
        let plan = ChangePlanService::plan_codex_switch_at(&state, &target.id, 200).unwrap();
        let calls = AtomicUsize::new(0);
        let interrupted = ChangePlanService::apply_codex_switch_at_with_writer_observer_and_fault(
            &state,
            &plan.plan_id,
            &plan.plan_digest,
            201,
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
            |_| {},
            Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord),
        );
        assert_eq!(interrupted.unwrap_err(), ChangePlanErrorCode::Internal);
        let pending = db
            .get_change_job_by_plan_id(&plan.plan_id)
            .unwrap()
            .unwrap();
        let recovered = ChangePlanService::get_job(&state, &pending.job_id).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(recovered.status, ChangeJobStatus::Warning);
        assert_eq!(
            recovered.result_code,
            ChangeResultCode::RecoveredTargetReached
        );
        assert_eq!(
            ChangePlanService::get_job(&state, &pending.job_id)
                .unwrap()
                .job_id,
            recovered.job_id
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
