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

fn validate_account_id(account_id: &str) -> Result<(), WorkBuddySubscriptionError> {
    if account_id.is_empty()
        || account_id.len() > 200
        || account_id.chars().any(char::is_whitespace)
    {
        return Err(invalid_request());
    }
    Ok(())
}

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
