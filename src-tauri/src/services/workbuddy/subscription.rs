//! Explicit WorkBuddy subscription binding and listener lifecycle.
//!
//! The binding names one admitted account and one model. It is not a provider
//! row, does not select a default account, and does not rewrite another agent.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::database::Database;
use crate::provider::{AuthBinding, AuthBindingSource, Provider, ProviderMeta};
use crate::services::managed_auth::{
    ManagedAuthProvider, ManagedAuthService, WorkBuddyAccountAdmission,
};
use crate::services::secret::SecretBackend;
use crate::store::AppState;

use super::error::{WorkBuddySubscriptionError, WorkBuddySubscriptionErrorCode};
use super::subscription_config_locator;
use super::types::WorkBuddySubscriptionSnapshot;

const SCHEMA_VERSION: u8 = 1;
const ROUTE_ID: &str = "workbuddy";
const LOCAL_BASE_PATH: &str = "/workbuddy/v1";
const STATUS_ACTIVE: &str = "active";
// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
const STATUS_INACTIVE: &str = "inactive";
const STATUS_ACCOUNT_UNAVAILABLE: &str = "account_unavailable";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkBuddySubscriptionRecord {
    schema_version: u8,
    route_id: String,
    account_id: String,
    legacy_account_id: String,
    provider: String,
    model_id: String,
    credential_id: String,
    generation: u64,
    provider_id: String,
    status: String,
    desired: bool,
    connected: bool,
    version: u64,
    listen_address: String,
    listen_port: u16,
    config_locator: String,
}

impl WorkBuddySubscriptionRecord {
    fn auth_kind(&self) -> &'static str {
        match self.provider.as_str() {
            "openai" => "codex_oauth",
            "xai" => "xai_oauth",
            _ => "",
        }
    }

    pub(crate) fn model_id(&self) -> &str {
        &self.model_id
    }

    pub(crate) fn is_servable(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && self.route_id == ROUTE_ID
            && self.desired
            && self.connected
            && self.status == STATUS_ACTIVE
            && provider_id_is_closed(&self.provider, &self.provider_id)
    }

    fn snapshot(&self) -> WorkBuddySubscriptionSnapshot {
        WorkBuddySubscriptionSnapshot {
            route_id: self.route_id.clone(),
            account_id: self.account_id.clone(),
            provider: self.provider.clone(),
            model_id: self.model_id.clone(),
            status: self.status.clone(),
            version: self.version,
            connected: self.connected,
            config_locator: self.config_locator.clone(),
            local_base_path: LOCAL_BASE_PATH.to_string(),
            provider_id: self.provider_id.clone(),
        }
    }
}

pub(crate) fn load_record(
    db: &Database,
) -> Result<Option<WorkBuddySubscriptionRecord>, WorkBuddySubscriptionError> {
    let Some(raw) = db
        .get_workbuddy_subscription_binding()
        .map_err(|_| internal())?
    else {
        return Ok(None);
    };
    parse_record(&raw).map(Some)
}

fn parse_record(raw: &str) -> Result<WorkBuddySubscriptionRecord, WorkBuddySubscriptionError> {
    let record: WorkBuddySubscriptionRecord =
        serde_json::from_str(raw).map_err(|_| unavailable())?;
    if record.schema_version != SCHEMA_VERSION || record.route_id != ROUTE_ID {
        return Err(unavailable());
    }
    if record.config_locator != subscription_config_locator() {
        return Err(unavailable());
    }
    Ok(record)
}

fn save_record(
    db: &Database,
    record: &WorkBuddySubscriptionRecord,
) -> Result<(), WorkBuddySubscriptionError> {
    let raw = serde_json::to_string(record).map_err(|_| internal())?;
    db.set_workbuddy_subscription_binding(&raw)
        .map_err(|_| internal())
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
fn delete_record(db: &Database) -> Result<(), WorkBuddySubscriptionError> {
    db.delete_workbuddy_subscription_binding()
        .map_err(|_| internal())
}

/// Drop the connected flag when the shared listener stops.
/// Desired and status stay as they were. A missing or unreadable row is left alone.
pub(crate) fn mark_listener_down(db: &Database) {
    let Ok(Some(raw)) = db.get_workbuddy_subscription_binding() else {
        return;
    };
    let Ok(mut record) = parse_record(&raw) else {
        return;
    };
    if !record.connected {
        return;
    }
    record.connected = false;
    if let Ok(encoded) = serde_json::to_string(&record) {
        let _ = db.set_workbuddy_subscription_binding(&encoded);
    }
}

pub(crate) fn mark_account_unavailable(db: &Database) {
    let Ok(Some(mut record)) = load_record(db) else {
        return;
    };
    record.status = STATUS_ACCOUNT_UNAVAILABLE.to_string();
    record.connected = false;
    let _ = save_record(db, &record);
}

/// `Ok(Some(true))` only when this binding is the connected route for the account.
/// `Ok(None)` means there is no binding. A settings read error is `Err`.
pub(crate) fn observe_connected_binding(
    db: &Database,
    auth_kind: &str,
    legacy_account_id: &str,
    listener_port: u16,
) -> Result<Option<bool>, ()> {
    let raw = db.get_workbuddy_subscription_binding().map_err(|_| ())?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let record = parse_record(&raw).map_err(|_| ())?;
    let address_is_loopback = record
        .listen_address
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    let matched = record.is_servable()
        && record.auth_kind() == auth_kind
        && record.legacy_account_id == legacy_account_id
        && record.listen_port == listener_port
        && record.listen_port != 0
        && address_is_loopback;
    Ok(Some(matched))
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn publish_workbuddy_subscription<B: SecretBackend>(
    state: &AppState,
    auth: &ManagedAuthService<B>,
    account_id: &str,
    model_id: &str,
) -> Result<WorkBuddySubscriptionSnapshot, WorkBuddySubscriptionError> {
    validate_account_id(account_id)?;
    validate_model_id(model_id)?;
    let admitted = admit(auth, account_id)?;
    let provider_id =
        workbuddy_provider_id(admitted.provider, &admitted.legacy_account_id, model_id);
    let global = state
        .db
        .get_global_proxy_config()
        .await
        .map_err(|_| internal())?;
    if !global
        .listen_address
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback())
    {
        return Err(invalid_request());
    }
    state
        .proxy_service
        .sync_workbuddy_proxy_credential()
        .await
        .map_err(|_| internal())?;
    let previous_enabled = global.proxy_enabled;
    let was_running = state.proxy_service.is_running().await;
    let mut record = WorkBuddySubscriptionRecord {
        schema_version: SCHEMA_VERSION,
        route_id: ROUTE_ID.to_string(),
        account_id: admitted.identity_id,
        legacy_account_id: admitted.legacy_account_id,
        provider: admitted.provider.as_str().to_string(),
        model_id: model_id.to_string(),
        credential_id: admitted.credential_id,
        generation: admitted.generation,
        provider_id,
        status: STATUS_ACTIVE.to_string(),
        desired: true,
        connected: false,
        version: 1,
        listen_address: global.listen_address,
        listen_port: 0,
        config_locator: subscription_config_locator().to_string(),
    };
    save_record(&state.db, &record)?;

    let started = state.proxy_service.start().await;
    let info = match started {
        Ok(info) => info,
        Err(_) => {
            let _ = delete_record(&state.db);
            revert_proxy_enabled_if_idle(state, previous_enabled, was_running).await;
            log::warn!("WorkBuddy subscription listener was not started");
            return Err(port_conflict());
        }
    };
    let loopback = info
        .address
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    if !loopback || info.port == 0 {
        let _ = delete_record(&state.db);
        if !was_running {
            let _ = state.proxy_service.stop().await;
        }
        return Err(invalid_request());
    }
    record.connected = true;
    record.listen_address = info.address;
    record.listen_port = info.port;
    record.version = record.version.saturating_add(1);
    save_record(&state.db, &record)?;
    Ok(record.snapshot())
}

pub(crate) async fn restore_workbuddy_subscription<B: SecretBackend>(
    state: &AppState,
    auth: &ManagedAuthService<B>,
) -> Result<Option<WorkBuddySubscriptionSnapshot>, WorkBuddySubscriptionError> {
    let Some(mut record) = load_record(&state.db)? else {
        return Ok(None);
    };
    if !record.desired || record.status != STATUS_ACTIVE {
        record.connected = false;
        save_record(&state.db, &record)?;
        return Ok(Some(record.snapshot()));
    }
    let admitted = match admit(auth, &record.account_id) {
        Ok(admitted) => admitted,
        Err(error) => {
            record.status = STATUS_ACCOUNT_UNAVAILABLE.to_string();
            record.connected = false;
            save_record(&state.db, &record)?;
            return Err(error);
        }
    };
    if admitted.provider.as_str() != record.provider
        || admitted.legacy_account_id != record.legacy_account_id
        || !provider_id_is_closed(&record.provider, &record.provider_id)
    {
        record.status = STATUS_ACCOUNT_UNAVAILABLE.to_string();
        record.connected = false;
        save_record(&state.db, &record)?;
        return Err(account_unavailable());
    }
    record.generation = admitted.generation;
    record.credential_id = admitted.credential_id;
    record.connected = false;
    save_record(&state.db, &record)?;

    let global = state
        .db
        .get_global_proxy_config()
        .await
        .map_err(|_| internal())?;
    if !global
        .listen_address
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback())
    {
        return Err(invalid_request());
    }
    state
        .proxy_service
        .sync_workbuddy_proxy_credential()
        .await
        .map_err(|_| internal())?;
    let was_running = state.proxy_service.is_running().await;
    let previous_enabled = global.proxy_enabled;
    let info = match state.proxy_service.start().await {
        Ok(info) => info,
        Err(_) => {
            revert_proxy_enabled_if_idle(state, previous_enabled, was_running).await;
            log::warn!("WorkBuddy subscription listener was not restored");
            return Err(port_conflict());
        }
    };
    if !info
        .address
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback())
        || info.port == 0
    {
        if !was_running {
            let _ = state.proxy_service.stop().await;
        }
        return Err(invalid_request());
    }
    record.connected = true;
    record.listen_address = info.address;
    record.listen_port = info.port;
    save_record(&state.db, &record)?;
    Ok(Some(record.snapshot()))
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn deactivate_workbuddy_subscription(
    state: &AppState,
) -> Result<Option<WorkBuddySubscriptionSnapshot>, WorkBuddySubscriptionError> {
    let Some(mut record) = load_record(&state.db)? else {
        return Ok(None);
    };
    record.status = STATUS_INACTIVE.to_string();
    record.desired = false;
    record.connected = false;
    save_record(&state.db, &record)?;
    if !other_agent_proxy_enabled(&state.db).await && state.proxy_service.is_running().await {
        let _ = state.proxy_service.stop().await;
    }
    Ok(Some(record.snapshot()))
}

pub(crate) fn build_provider(record: &WorkBuddySubscriptionRecord) -> Option<Provider> {
    if !provider_id_is_closed(&record.provider, &record.provider_id) {
        return None;
    }
    let (auth_kind, slot, label_name, base_url, website) = match record.provider.as_str() {
        "openai" => (
            "codex_oauth",
            "fyagent_chatgpt",
            "ChatGPT",
            crate::proxy::providers::CHATGPT_CODEX_BASE_URL,
            "https://chatgpt.com",
        ),
        "xai" => (
            "xai_oauth",
            "xai",
            "Grok",
            crate::proxy::providers::XAI_SUBSCRIPTION_BASE_URL,
            "https://x.ai/grok",
        ),
        _ => return None,
    };
    let config = format!(
        "model_provider = \"{slot}\"\nmodel = \"{model}\"\n\n[model_providers.{slot}]\nname = \"{label_name} subscription\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n",
        model = record.model_id
    );
    let mut provider = Provider::with_id(
        record.provider_id.clone(),
        "WorkBuddy subscription".to_string(),
        serde_json::json!({
            "auth": {},
            "config": config,
            "model": record.model_id,
        }),
        Some(website.to_string()),
    );
    provider.meta = Some(ProviderMeta {
        provider_type: Some(auth_kind.to_string()),
        auth_binding: Some(AuthBinding {
            source: AuthBindingSource::ManagedAccount,
            auth_provider: Some(auth_kind.to_string()),
            account_id: Some(record.legacy_account_id.clone()),
        }),
        ..Default::default()
    });
    Some(provider)
}

pub(crate) fn admit_record<B: SecretBackend>(
    auth: &ManagedAuthService<B>,
    record: &WorkBuddySubscriptionRecord,
) -> Result<WorkBuddyAccountAdmission, WorkBuddySubscriptionError> {
    if !record.is_servable() {
        return Err(unavailable());
    }
    let admitted = admit(auth, &record.account_id)?;
    if admitted.provider.as_str() != record.provider
        || admitted.legacy_account_id != record.legacy_account_id
        || admitted.identity_id != record.account_id
    {
        return Err(account_unavailable());
    }
    Ok(admitted)
}

fn admit<B: SecretBackend>(
    auth: &ManagedAuthService<B>,
    account_id: &str,
) -> Result<WorkBuddyAccountAdmission, WorkBuddySubscriptionError> {
    auth.admit_explicit_workbuddy_account(account_id)
        .map_err(|_| account_unavailable())
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
fn workbuddy_provider_id(
    provider: ManagedAuthProvider,
    legacy_account_id: &str,
    model: &str,
) -> String {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{legacy_account_id}\0{model}").as_bytes())
    );
    let source = match provider {
        ManagedAuthProvider::Openai => "openai",
        ManagedAuthProvider::Xai => "xai",
        ManagedAuthProvider::GithubCopilot => "copilot",
    };
    format!("fyagent-{source}-workbuddy-{}", &digest[..24])
}

fn provider_id_is_closed(provider: &str, provider_id: &str) -> bool {
    let prefix = match provider {
        "openai" => "fyagent-openai-workbuddy-",
        "xai" => "fyagent-xai-workbuddy-",
        _ => return false,
    };
    provider_id.starts_with(prefix) && provider_id.len() > prefix.len()
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
fn validate_account_id(account_id: &str) -> Result<(), WorkBuddySubscriptionError> {
    if account_id.is_empty()
        || account_id.len() > 200
        || account_id.chars().any(char::is_whitespace)
    {
        return Err(invalid_request());
    }
    Ok(())
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
fn validate_model_id(model_id: &str) -> Result<(), WorkBuddySubscriptionError> {
    let model = model_id.as_bytes();
    if model.is_empty()
        || model.len() > 128
        || !model[0].is_ascii_alphanumeric()
        || !model
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(byte))
    {
        return Err(invalid_request());
    }
    Ok(())
}

// 第一刀仅测试覆盖，第二刀再挂命令。
#[cfg_attr(not(test), allow(dead_code))]
async fn other_agent_proxy_enabled(db: &Database) -> bool {
    for app in ["claude", "codex", "gemini", "grokbuild", "opencode"] {
        match db.get_proxy_config_for_app(app).await {
            Ok(config) if config.enabled => return true,
            Ok(_) => {}
            Err(_) => return true,
        }
    }
    false
}

async fn revert_proxy_enabled_if_idle(state: &AppState, previous_enabled: bool, was_running: bool) {
    if was_running || previous_enabled || state.proxy_service.is_running().await {
        return;
    }
    let Ok(mut global) = state.db.get_global_proxy_config().await else {
        return;
    };
    if !global.proxy_enabled {
        return;
    }
    global.proxy_enabled = false;
    let _ = state.db.update_global_proxy_config(global).await;
}

fn invalid_request() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::InvalidRequest)
}

fn account_unavailable() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::AccountUnavailable)
}

fn unavailable() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::SubscriptionUnavailable)
}

fn port_conflict() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::PortConflict)
}

fn internal() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::Internal)
}

#[cfg(test)]
fn test_auth_slot() -> &'static std::sync::Mutex<
    Option<std::sync::Arc<ManagedAuthService<crate::services::secret::MemorySecretBackend>>>,
> {
    static SLOT: std::sync::OnceLock<
        std::sync::Mutex<
            Option<
                std::sync::Arc<ManagedAuthService<crate::services::secret::MemorySecretBackend>>,
            >,
        >,
    > = std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_workbuddy_subscription_auth_for_test(
    auth: Option<std::sync::Arc<ManagedAuthService<crate::services::secret::MemorySecretBackend>>>,
) {
    *test_auth_slot().lock().expect("workbuddy auth fixture") = auth;
}

#[cfg(test)]
pub(crate) fn workbuddy_subscription_auth_for_test(
) -> Option<std::sync::Arc<ManagedAuthService<crate::services::secret::MemorySecretBackend>>> {
    test_auth_slot()
        .lock()
        .expect("workbuddy auth fixture")
        .clone()
}

/// Process-private plan material. It is deliberately not Debug/Serialize:
/// file credentials and preimages must never enter public plans or logs.
#[derive(Clone)]
pub(crate) struct SubscriptionDraft {
    pub(crate) request: super::types::SaveWorkBuddySubscriptionRequest,
    previous: Option<WorkBuddySubscriptionRecord>,
    previous_raw: Option<String>,
    admitted: WorkBuddyAccountAdmission,
    record: WorkBuddySubscriptionRecord,
    file_request: super::types::SaveWorkBuddyModelsRequest,
    before: super::config::LoadedConfig,
    backup_before: Option<Vec<u8>>,
    published: Vec<u8>,
}

impl SubscriptionDraft {
    pub(crate) fn target_source(&self) -> super::types::WorkBuddySubscriptionSource {
        source_for(&self.record)
    }

    pub(crate) fn baseline_digest(&self) -> String {
        let value = serde_json::json!({
            "previous": self.previous,
            "previousRaw": self.previous_raw,
            "record": self.record,
            "revision": self.before.revision,
            "credentialId": self.admitted.credential_id,
            "generation": self.admitted.generation,
        });
        format!("{:x}", Sha256::digest(value.to_string().as_bytes()))
    }
}

fn source_for(record: &WorkBuddySubscriptionRecord) -> super::types::WorkBuddySubscriptionSource {
    let address: IpAddr = record
        .listen_address
        .parse()
        .expect("admitted loopback address");
    let socket = std::net::SocketAddr::new(address, record.listen_port);
    super::types::WorkBuddySubscriptionSource {
        account_id: record.account_id.clone(),
        provider: record.provider.clone(),
        model_id: record.model_id.clone(),
        binding_version: record.version,
        local_base_url: url::Url::parse(&format!("http://{socket}{LOCAL_BASE_PATH}"))
            .expect("native loopback URL")
            .to_string(),
    }
}

fn admit_from_state(
    state: &AppState,
    account_id: &str,
) -> Result<WorkBuddyAccountAdmission, WorkBuddySubscriptionError> {
    #[cfg(test)]
    if let Some(auth) = workbuddy_subscription_auth_for_test() {
        return admit(auth.as_ref(), account_id);
    }
    let auth = crate::services::managed_auth::NativeManagedAuthService::new(
        state.db.clone(),
        crate::services::secret::SecretService::new(
            crate::services::secret::NativeSecretBackend::new(),
        ),
        crate::config::get_app_config_dir(),
    );
    admit(&auth, account_id)
}

pub(crate) fn prepare_subscription(
    state: &AppState,
    request: super::types::SaveWorkBuddySubscriptionRequest,
) -> Result<SubscriptionDraft, WorkBuddySubscriptionError> {
    validate_account_id(&request.account_id)?;
    validate_model_id(&request.model_id)?;
    let admitted = admit_from_state(state, &request.account_id)?;
    let previous_raw = state
        .db
        .get_workbuddy_subscription_binding()
        .map_err(|_| internal())?;
    let previous = previous_raw.as_deref().map(parse_record).transpose()?;
    if previous.as_ref().map(|record| record.version) != request.expected_binding_version {
        return Err(stale());
    }
    let global = tauri::async_runtime::block_on(state.db.get_global_proxy_config())
        .map_err(|_| internal())?;
    if global.listen_port == 0
        || !global
            .listen_address
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return Err(invalid_request());
    }
    let record = WorkBuddySubscriptionRecord {
        schema_version: SCHEMA_VERSION,
        route_id: ROUTE_ID.to_string(),
        account_id: admitted.identity_id.clone(),
        legacy_account_id: admitted.legacy_account_id.clone(),
        provider: admitted.provider.as_str().to_string(),
        model_id: request.model_id.clone(),
        credential_id: admitted.credential_id.clone(),
        generation: admitted.generation,
        provider_id: workbuddy_provider_id(
            admitted.provider,
            &admitted.legacy_account_id,
            &request.model_id,
        ),
        status: STATUS_ACTIVE.to_string(),
        desired: true,
        connected: true,
        version: previous
            .as_ref()
            .map_or(Some(1), |record| record.version.checked_add(1))
            .ok_or_else(internal)?,
        listen_address: global.listen_address,
        listen_port: global.listen_port,
        config_locator: subscription_config_locator().to_string(),
    };
    let (before, backup_before) =
        super::config::load_workbuddy_files(&super::config::current_paths())
            .map_err(|_| internal())?;
    if before.revision != request.expected_revision {
        return Err(stale());
    }
    let key = state.db.ensure_proxy_access_key().map_err(|_| internal())?;
    let file_request = super::types::SaveWorkBuddyModelsRequest {
        base_url: source_for(&record).local_base_url,
        api_key: key,
        allow_no_api_key: false,
        selected_model_ids: vec![request.model_id.clone()],
        manual_model_ids: vec![],
        removed_model_ids: vec![],
        clear_existing_api_keys: false,
        expected_revision: request.expected_revision.clone(),
        overwrite_token: None,
    };
    let published = super::config::project_subscription_file(&before, &file_request)
        .map_err(|_| invalid_request())?;
    Ok(SubscriptionDraft {
        request,
        previous,
        previous_raw,
        admitted,
        record,
        file_request,
        before,
        backup_before,
        published,
    })
}

pub(crate) fn validate_subscription_draft(
    state: &AppState,
    draft: &SubscriptionDraft,
) -> Result<(), WorkBuddySubscriptionError> {
    if state
        .db
        .get_workbuddy_subscription_binding()
        .map_err(|_| internal())?
        != draft.previous_raw
        || admit_from_state(state, &draft.request.account_id)? != draft.admitted
    {
        return Err(stale());
    }
    let global = tauri::async_runtime::block_on(state.db.get_global_proxy_config())
        .map_err(|_| internal())?;
    let (current, backup) = super::config::load_workbuddy_files(&super::config::current_paths())
        .map_err(|_| internal())?;
    if global.listen_address != draft.record.listen_address
        || global.listen_port != draft.record.listen_port
        || current.revision != draft.before.revision
        || backup != draft.backup_before
        || state
            .db
            .get_setting("proxy_access_key")
            .map_err(|_| internal())?
            .as_deref()
            != Some(draft.file_request.api_key.as_str())
    {
        return Err(stale());
    }
    Ok(())
}

pub(crate) fn read_subscription_source(
    state: &AppState,
) -> Result<super::types::WorkBuddySubscriptionSourceResult, WorkBuddySubscriptionError> {
    let loaded = super::config::load_readonly_config_at(&super::config::current_paths())
        .map_err(|_| internal())?;
    let mut result = super::types::WorkBuddySubscriptionSourceResult {
        revision: loaded.revision,
        binding_version: None,
        source: None,
    };
    let Some(record) = load_record(&state.db)? else {
        return Ok(result);
    };
    result.binding_version = Some(record.version);
    if !record.is_servable() {
        return Ok(result);
    }
    if record.listen_port == 0
        || !record
            .listen_address
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return Err(unavailable());
    }
    let source = source_for(&record);
    let key = state
        .db
        .get_setting("proxy_access_key")
        .map_err(|_| internal())?
        .ok_or_else(unavailable)?;
    let models: Vec<_> = loaded
        .document
        .models()
        .iter()
        .filter(|entry| {
            entry["id"]
                .as_str()
                .is_some_and(|id| id.trim() == record.model_id)
        })
        .collect();
    if models.is_empty()
        || models.iter().any(|entry| {
            entry["url"].as_str() != Some(source.local_base_url.as_str())
                || entry["apiKey"].as_str() != Some(key.as_str())
        })
    {
        return Ok(result);
    }
    let admitted = admit_from_state(state, &record.account_id)?;
    if admitted.provider.as_str() != record.provider
        || admitted.legacy_account_id != record.legacy_account_id
        || admitted.credential_id != record.credential_id
    {
        return Err(account_unavailable());
    }
    let status =
        tauri::async_runtime::block_on(state.proxy_service.get_status()).map_err(|_| internal())?;
    if !status.running
        || status.address != record.listen_address
        || status.port != record.listen_port
    {
        return Err(unavailable());
    }
    result.source = Some(source);
    Ok(result)
}

/// Caller owns the WorkBuddy write lock for validation, commit and compensation.
pub(crate) fn commit_subscription_locked(
    state: &AppState,
    draft: &SubscriptionDraft,
) -> Result<(), WorkBuddySubscriptionError> {
    validate_subscription_draft(state, draft)?;
    let was_running = tauri::async_runtime::block_on(state.proxy_service.is_running());
    let previous_enabled = tauri::async_runtime::block_on(state.db.get_global_proxy_config())
        .map_err(|_| internal())?
        .proxy_enabled;
    let mut file_attempted = false;
    let operation = (|| {
        tauri::async_runtime::block_on(state.proxy_service.sync_workbuddy_proxy_credential())
            .map_err(|_| internal())?;
        let info = tauri::async_runtime::block_on(state.proxy_service.start())
            .map_err(|_| port_conflict())?;
        if info.address != draft.record.listen_address || info.port != draft.record.listen_port {
            return Err(stale());
        }
        validate_subscription_draft(state, draft)?;
        subscription_fault(SubscriptionFaultPoint::FileCommit)?;
        let paths = super::config::current_paths();
        let mut request = draft.file_request.clone();
        file_attempted = true;
        let mut outcome = super::config::save_workbuddy_models_at_locked(&paths, &request)
            .map_err(|_| internal())?;
        if let super::types::SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired {
            token,
            ..
        } = outcome
        {
            request.overwrite_token = Some(token);
            outcome = super::config::save_workbuddy_models_at_locked(&paths, &request)
                .map_err(|_| internal())?;
        }
        if !matches!(
            outcome,
            super::types::SaveWorkBuddyModelsOutcome::Saved { .. }
        ) {
            return Err(stale());
        }
        subscription_fault(SubscriptionFaultPoint::BindingCommit)?;
        if load_record(&state.db)? != draft.previous
            || admit_from_state(state, &draft.request.account_id)? != draft.admitted
        {
            return Err(stale());
        }
        let replacement = serde_json::to_string(&draft.record).map_err(|_| internal())?;
        #[cfg(test)]
        if SUBSCRIPTION_FAULT
            .with(|slot| *slot.borrow() == Some(SubscriptionFaultPoint::BindingConcurrent))
        {
            SUBSCRIPTION_FAULT.with(|slot| slot.borrow_mut().take());
            let mut external = draft.record.clone();
            external.status = STATUS_ACCOUNT_UNAVAILABLE.to_string();
            external.connected = false;
            external.version += 1;
            save_record(&state.db, &external)?;
        }
        if !state
            .db
            .compare_exchange_workbuddy_subscription_binding(
                draft.previous_raw.as_deref(),
                Some(&replacement),
            )
            .map_err(|_| internal())?
        {
            return Err(stale());
        }
        subscription_fault(SubscriptionFaultPoint::Readback)?;
        let result = read_subscription_source(state)?;
        let current = super::config::load_readonly_config_at(&paths).map_err(|_| internal())?;
        if result.source != Some(draft.target_source()) || current.original_bytes != draft.published
        {
            return Err(unavailable());
        }
        Ok(())
    })();
    if operation.is_ok() {
        return operation;
    }
    let files_restored = !file_attempted
        || super::config::compensate_subscription_files(
            &super::config::current_paths(),
            &draft.before,
            draft.backup_before.as_deref(),
            &draft.published,
        )
        .is_ok();
    let replacement = serde_json::to_string(&draft.record).map_err(|_| recovery_required())?;
    let binding_restored = match state.db.get_workbuddy_subscription_binding() {
        Ok(current) if current.as_deref() == Some(replacement.as_str()) => state
            .db
            .compare_exchange_workbuddy_subscription_binding(
                Some(&replacement),
                draft.previous_raw.as_deref(),
            )
            .unwrap_or(false),
        Ok(current) => current == draft.previous_raw,
        Err(_) => false,
    };
    tauri::async_runtime::block_on(async {
        if !was_running
            && !other_agent_proxy_enabled(&state.db).await
            && load_record(&state.db)
                .is_ok_and(|record| record.is_none_or(|record| !record.is_servable()))
        {
            let _ = state.proxy_service.stop().await;
        }
        revert_proxy_enabled_if_idle(state, previous_enabled, was_running).await;
    });
    if !files_restored || !binding_restored {
        return Err(recovery_required());
    }
    operation
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubscriptionFaultPoint {
    FileCommit,
    BindingCommit,
    Readback,
    #[cfg(test)]
    ExternalEdit,
    #[cfg(test)]
    BindingConcurrent,
}

#[cfg(test)]
thread_local! {
    static SUBSCRIPTION_FAULT: std::cell::RefCell<Option<SubscriptionFaultPoint>> = const { std::cell::RefCell::new(None) };
}

fn subscription_fault(point: SubscriptionFaultPoint) -> Result<(), WorkBuddySubscriptionError> {
    #[cfg(test)]
    if point == SubscriptionFaultPoint::Readback
        && SUBSCRIPTION_FAULT
            .with(|slot| *slot.borrow() == Some(SubscriptionFaultPoint::ExternalEdit))
    {
        SUBSCRIPTION_FAULT.with(|slot| slot.borrow_mut().take());
        std::fs::write(super::config::current_paths().models, b"[]").unwrap();
        return Err(internal());
    }
    #[cfg(test)]
    if SUBSCRIPTION_FAULT.with(|slot| {
        if *slot.borrow() == Some(point) {
            slot.borrow_mut().take();
            true
        } else {
            false
        }
    }) {
        return Err(internal());
    }
    let _ = point;
    Ok(())
}

fn stale() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::Stale)
}
fn recovery_required() -> WorkBuddySubscriptionError {
    WorkBuddySubscriptionError::new(WorkBuddySubscriptionErrorCode::RecoveryRequired)
}

#[cfg(test)]
pub(crate) mod plan_tests {
    use super::*;
    use crate::services::managed_auth::{
        CredentialPurpose, CredentialStatus, LegacyCredentialInput, ManagedAuthConsumer,
        ManagedAuthSecretBundle, ManagedAuthSecretBundleParts, RefreshOwner,
    };
    use crate::services::secret::{MemorySecretBackend, SecretService};
    use std::sync::Arc;

    pub(crate) struct Fixture {
        pub(crate) state: AppState,
        pub(crate) account_id: String,
        pub(crate) home: tempfile::TempDir,
        previous_home: Option<std::ffi::OsString>,
        previous_data_home: Option<std::ffi::OsString>,
    }

    impl Fixture {
        pub(crate) fn new() -> Self {
            #[cfg(target_os = "windows")]
            crate::initialize_windows_user_context().unwrap();
            let home = tempfile::tempdir().unwrap();
            let previous_home = std::env::var_os("FYAGENT_TEST_HOME");
            let previous_data_home = std::env::var_os("XDG_DATA_HOME");
            std::env::set_var("FYAGENT_TEST_HOME", home.path());
            assert_eq!(crate::config::get_home_dir(), home.path());
            std::env::set_var("XDG_DATA_HOME", home.path().join(".local/share"));
            crate::settings::reload_settings().unwrap();
            let db = Arc::new(Database::memory().unwrap());
            let auth = Arc::new(ManagedAuthService::new(
                db.clone(),
                SecretService::new(MemorySecretBackend::new()),
                home.path().join("vault-meta"),
            ));
            let credential = auth
                .provision_legacy_credential(LegacyCredentialInput {
                    migration_id: None,
                    provider: ManagedAuthProvider::Openai,
                    purpose: CredentialPurpose::ProxyUpstream,
                    consumer: Some(ManagedAuthConsumer::FyagentProxy),
                    legacy_account_id: "subscription-plan".into(),
                    provider_subject: "subscription-plan".into(),
                    provider_tenant: String::new(),
                    login: "fixture@example.test".into(),
                    display_name: None,
                    avatar_url: None,
                    access_token: None,
                    refresh_token: Some(zeroize::Zeroizing::new("synthetic-refresh".into())),
                    id_token: None,
                    desired_status: CredentialStatus::Ready,
                    refresh_owner: RefreshOwner::Fyagent,
                    authenticated_at: 1_700_000_000,
                    make_default: false,
                })
                .unwrap();
            let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
                credential_id: credential.credential_id.clone(),
                provider: ManagedAuthProvider::Openai,
                generation: credential.generation + 1,
                access_token: Some("synthetic-access".into()),
                refresh_token: Some("synthetic-refresh".into()),
                id_token: None,
                token_type: Some("Bearer".into()),
                granted_scopes: vec![],
                issued_at: Some(chrono::Utc::now().timestamp()),
                expires_at: Some(chrono::Utc::now().timestamp() + 3600),
            })
            .unwrap();
            assert!(auth
                .replace_bundle_cas(
                    &credential.credential_id,
                    credential.generation,
                    RefreshOwner::Fyagent,
                    bundle
                )
                .unwrap());
            let account_id = credential.identity_id.clone();
            set_workbuddy_subscription_auth_for_test(Some(auth));
            let state = AppState::new(db);
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            tauri::async_runtime::block_on(async {
                let mut global = state.db.get_global_proxy_config().await.unwrap();
                global.listen_address = "127.0.0.1".into();
                global.listen_port = port;
                state.db.update_global_proxy_config(global).await.unwrap();
            });
            Self {
                state,
                account_id,
                home,
                previous_home,
                previous_data_home,
            }
        }

        pub(crate) fn request(&self) -> super::super::types::SaveWorkBuddySubscriptionRequest {
            super::super::types::SaveWorkBuddySubscriptionRequest {
                account_id: self.account_id.clone(),
                model_id: "probe-model".into(),
                expected_revision: None,
                expected_binding_version: None,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            tauri::async_runtime::block_on(self.state.proxy_service.stop()).ok();
            set_workbuddy_subscription_auth_for_test(None);
            if let Some(value) = self.previous_data_home.take() {
                std::env::set_var("XDG_DATA_HOME", value);
            } else {
                std::env::remove_var("XDG_DATA_HOME");
            }
            if let Some(value) = self.previous_home.take() {
                std::env::set_var("FYAGENT_TEST_HOME", value);
            } else {
                std::env::remove_var("FYAGENT_TEST_HOME");
            }
            crate::settings::reload_settings().unwrap();
        }
    }

    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_plan_previews_without_publishing_and_reads_exact_source() {
        let fixture = Fixture::new();
        let draft = prepare_subscription(&fixture.state, fixture.request()).unwrap();
        assert!(load_record(&fixture.state.db).unwrap().is_none());
        assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
        assert!(!tauri::async_runtime::block_on(
            fixture.state.proxy_service.is_running()
        ));
        let _guard = super::super::write_lock().blocking_lock();
        commit_subscription_locked(&fixture.state, &draft).unwrap();
        let readback = read_subscription_source(&fixture.state).unwrap();
        assert_eq!(readback.source, Some(draft.target_source()));
        assert!(readback.revision.is_some());
        assert_eq!(
            commit_subscription_locked(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::Stale
        );
        let path = fixture.home.path().join(".workbuddy/models.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["models"][0]["apiKey"] = serde_json::json!("external-api-key");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let overwritten = read_subscription_source(&fixture.state).unwrap();
        assert!(overwritten.source.is_none());
        assert_eq!(
            overwritten.binding_version,
            Some(draft.target_source().binding_version)
        );
        let mut retry = fixture.request();
        retry.expected_revision = overwritten.revision;
        retry.expected_binding_version = overwritten.binding_version;
        assert!(prepare_subscription(&fixture.state, retry).is_ok());
        value["models"] = serde_json::json!([]);
        std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(read_subscription_source(&fixture.state)
            .unwrap()
            .source
            .is_none());
    }

    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_plan_compensates_each_failure_and_preserves_backup() {
        for point in [
            SubscriptionFaultPoint::FileCommit,
            SubscriptionFaultPoint::BindingCommit,
            SubscriptionFaultPoint::Readback,
        ] {
            let fixture = Fixture::new();
            let paths = super::super::config::current_paths();
            std::fs::create_dir_all(&paths.directory).unwrap();
            let original = br#"{"unknown":true,"models":[{"id":"keep","url":"https://example.test/v1","apiKey":"old"}]}"#;
            std::fs::write(&paths.models, original).unwrap();
            std::fs::write(&paths.backup, b"unrelated old backup").unwrap();
            let mut request = fixture.request();
            request.expected_revision = super::super::config::load_readonly_config_at(&paths)
                .unwrap()
                .revision;
            let draft = prepare_subscription(&fixture.state, request).unwrap();
            SUBSCRIPTION_FAULT.with(|slot| *slot.borrow_mut() = Some(point));
            let _guard = super::super::write_lock().blocking_lock();
            assert_eq!(
                commit_subscription_locked(&fixture.state, &draft)
                    .unwrap_err()
                    .code(),
                WorkBuddySubscriptionErrorCode::Internal
            );
            assert!(!tauri::async_runtime::block_on(
                fixture.state.proxy_service.is_running()
            ));
            assert_eq!(std::fs::read(&paths.models).unwrap(), original);
            assert_eq!(
                std::fs::read(&paths.backup).unwrap(),
                b"unrelated old backup"
            );
            assert!(load_record(&fixture.state.db).unwrap().is_none());
        }
    }

    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_plan_failed_first_publication_restores_absence() {
        let fixture = Fixture::new();
        let draft = prepare_subscription(&fixture.state, fixture.request()).unwrap();
        SUBSCRIPTION_FAULT.with(|slot| *slot.borrow_mut() = Some(SubscriptionFaultPoint::Readback));
        let _guard = super::super::write_lock().blocking_lock();
        assert_eq!(
            commit_subscription_locked(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::Internal
        );
        assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
        assert!(load_record(&fixture.state.db).unwrap().is_none());
    }

    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_plan_rejects_external_edits_and_binding_drift() {
        let fixture = Fixture::new();
        let draft = prepare_subscription(&fixture.state, fixture.request()).unwrap();
        let paths = super::super::config::current_paths();
        std::fs::create_dir_all(&paths.directory).unwrap();
        std::fs::write(&paths.models, b"[]").unwrap();
        assert_eq!(
            validate_subscription_draft(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::Stale
        );
        assert!(super::super::config::compensate_subscription_files(
            &paths,
            &draft.before,
            None,
            &draft.published
        )
        .is_err());
        assert_eq!(std::fs::read(&paths.models).unwrap(), b"[]");
        std::fs::remove_file(&paths.models).unwrap();
        save_record(&fixture.state.db, &draft.record).unwrap();
        assert_eq!(
            validate_subscription_draft(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::Stale
        );
    }
    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_plan_partial_compensation_preserves_external_edit() {
        let fixture = Fixture::new();
        let draft = prepare_subscription(&fixture.state, fixture.request()).unwrap();
        SUBSCRIPTION_FAULT
            .with(|slot| *slot.borrow_mut() = Some(SubscriptionFaultPoint::ExternalEdit));
        let _guard = super::super::write_lock().blocking_lock();
        assert_eq!(
            commit_subscription_locked(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::RecoveryRequired
        );
        assert_eq!(
            std::fs::read(fixture.home.path().join(".workbuddy/models.json")).unwrap(),
            b"[]"
        );
        assert!(load_record(&fixture.state.db).unwrap().is_none());
        assert!(read_subscription_source(&fixture.state)
            .unwrap()
            .source
            .is_none());
    }
    #[test]
    #[cfg(target_os = "macos")]
    #[serial_test::serial]
    fn workbuddy_subscription_restore_precommit_edit_is_preserved() {
        for existing in [false, true] {
            let fixture = Fixture::new();
            let paths = super::super::config::current_paths();
            let mut request = fixture.request();
            if existing {
                std::fs::create_dir_all(&paths.directory).unwrap();
                std::fs::write(&paths.models, b"[]").unwrap();
                request.expected_revision = super::super::config::load_readonly_config_at(&paths)
                    .unwrap()
                    .revision;
            }
            let draft = prepare_subscription(&fixture.state, request).unwrap();
            SUBSCRIPTION_FAULT
                .with(|slot| *slot.borrow_mut() = Some(SubscriptionFaultPoint::Readback));
            super::super::config::arm_subscription_restore_external_edit(
                b"external-before-restore".to_vec(),
            );
            let _guard = super::super::write_lock().blocking_lock();
            assert_eq!(
                commit_subscription_locked(&fixture.state, &draft)
                    .unwrap_err()
                    .code(),
                WorkBuddySubscriptionErrorCode::RecoveryRequired
            );
            assert_eq!(
                std::fs::read(&paths.models).unwrap(),
                b"external-before-restore"
            );
            assert!(load_record(&fixture.state.db).unwrap().is_none());
        }
    }
    #[test]
    #[serial_test::serial]
    fn workbuddy_subscription_binding_cas_preserves_concurrent_lifecycle_change() {
        let fixture = Fixture::new();
        let draft = prepare_subscription(&fixture.state, fixture.request()).unwrap();
        SUBSCRIPTION_FAULT
            .with(|slot| *slot.borrow_mut() = Some(SubscriptionFaultPoint::BindingConcurrent));
        let _guard = super::super::write_lock().blocking_lock();
        assert_eq!(
            commit_subscription_locked(&fixture.state, &draft)
                .unwrap_err()
                .code(),
            WorkBuddySubscriptionErrorCode::RecoveryRequired
        );
        let record = load_record(&fixture.state.db).unwrap().unwrap();
        assert_eq!(record.status, STATUS_ACCOUNT_UNAVAILABLE);
        assert_eq!(record.version, draft.record.version + 1);
        assert!(!record.connected);
        assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    }
}
