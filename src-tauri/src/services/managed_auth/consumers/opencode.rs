//! OpenCode Desktop `auth.json` consumer.
//!
//! Path and schema follow official OpenCode MIT pin `b578b726`
//! (`Global.Path.data/auth.json`). This module does not copy cockpit-tools
//! and does not probe the private Desktop sidecar.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::services::managed_auth::{
    stable_connection_id, stable_revision, ConnectionRecord, CredentialPurpose, CredentialStatus,
    CredentialWithIdentity, ManagedAuthConnectionAction, ManagedAuthConnectionState,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthCoreError,
    ManagedAuthCredentialManager, ManagedAuthProvider, ManagedAuthReasonCode,
    ManagedAuthRequestMode,
};

type AuthJsonMap = Map<String, Value>;
type LoadedAuthJson = (AuthJsonMap, Option<Vec<u8>>);

/// External `auth.json` writes are not proven to hot-reload a live Desktop.
pub(crate) const OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN: bool = false;

#[cfg_attr(not(test), allow(dead_code))]
const AUTH_JSON_NAME: &str = "auth.json";
const MAX_AUTH_JSON_BYTES: usize = 256 * 1024;
const MAX_PROVIDER_KEYS: usize = 64;
const MAX_PROVIDER_KEY_CHARS: usize = 80;
const CLOSED_SLOTS: [ManagedAuthProvider; 3] = [
    ManagedAuthProvider::Openai,
    ManagedAuthProvider::Xai,
    ManagedAuthProvider::GithubCopilot,
];

fn auth_json_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservedEntryKind {
    Oauth,
    Api,
    WellKnown,
    Unknown,
}

#[derive(Debug, Clone)]
pub(crate) struct ObservedProvider {
    official_key: String,
    pub label: String,
    pub kind: ObservedEntryKind,
    pub managed_provider: Option<ManagedAuthProvider>,
}

impl ObservedProvider {
    pub(crate) fn capability_id(&self) -> String {
        capability_id(&self.official_key)
    }

    pub(crate) fn official_key(&self) -> &str {
        &self.official_key
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OpencodeAuthObservation {
    pub file_present: bool,
    pub readable: bool,
    pub revision: String,
    pub providers: Vec<ObservedProvider>,
    #[allow(dead_code)]
    pub unknown_entry_count: usize,
}

impl OpencodeAuthObservation {
    fn empty_missing() -> Self {
        Self {
            file_present: false,
            readable: true,
            revision: missing_revision(),
            providers: Vec::new(),
            unknown_entry_count: 0,
        }
    }

    fn unreadable(file_present: bool) -> Self {
        Self {
            file_present,
            readable: false,
            revision: missing_revision(),
            providers: Vec::new(),
            unknown_entry_count: 0,
        }
    }

    pub(crate) fn closed_kind(&self, provider: ManagedAuthProvider) -> Option<ObservedEntryKind> {
        self.providers
            .iter()
            .find_map(|entry| (entry.managed_provider == Some(provider)).then_some(entry.kind))
    }
}

pub(crate) enum ProjectionEntry {
    Oauth {
        refresh: Zeroizing<String>,
        access: Zeroizing<String>,
        expires: u64,
        account_id: Option<String>,
    },
    Api {
        key: Zeroizing<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct AuthJsonWriteReceipt {
    pub revision: String,
    pub pending_restart: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpencodeAuthError {
    NotFound,
    Invalid,
    Stale,
    Io,
}

impl From<OpencodeAuthError> for ManagedAuthCoreError {
    fn from(error: OpencodeAuthError) -> Self {
        match error {
            OpencodeAuthError::NotFound => Self::NotFound,
            OpencodeAuthError::Invalid => Self::InvalidData,
            OpencodeAuthError::Stale => Self::Stale,
            OpencodeAuthError::Io => Self::Io,
        }
    }
}

pub(crate) fn default_auth_json_path() -> PathBuf {
    crate::opencode_config::get_opencode_auth_json_path()
}

pub(crate) fn slot_connection_id(provider: ManagedAuthProvider) -> String {
    stable_connection_id(ManagedAuthConsumer::Opencode, "", provider.as_str())
}

pub(crate) fn slot_for_connection_id(connection_id: &str) -> Option<ManagedAuthProvider> {
    CLOSED_SLOTS
        .into_iter()
        .find(|provider| slot_connection_id(*provider) == connection_id)
}

pub(crate) fn file_key_for(provider: ManagedAuthProvider) -> &'static str {
    match provider {
        ManagedAuthProvider::Openai => "openai",
        ManagedAuthProvider::Xai => "xai",
        ManagedAuthProvider::GithubCopilot => "github-copilot",
    }
}

pub(crate) fn observe_auth_store(path: &Path) -> OpencodeAuthObservation {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    observe_auth_store_locked(path)
}

pub(crate) fn upsert_projection(
    path: &Path,
    provider: ManagedAuthProvider,
    entry: &ProjectionEntry,
    expected_revision: Option<&str>,
) -> Result<AuthJsonWriteReceipt, OpencodeAuthError> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (mut raw, source_bytes) = load_raw_map(path)?;
    let current = revision_for_source(path.exists(), source_bytes.as_deref());
    if let Some(expected) = expected_revision {
        if expected != current {
            return Err(OpencodeAuthError::Stale);
        }
    }
    raw.insert(file_key_for(provider).to_string(), projection_value(entry)?);
    commit_map(path, &raw, source_bytes.as_deref())
}

pub(crate) fn remove_file_key(
    path: &Path,
    official_key: &str,
    expected_revision: Option<&str>,
) -> Result<AuthJsonWriteReceipt, OpencodeAuthError> {
    let _guard = auth_json_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (mut raw, source_bytes) = load_raw_map(path)?;
    let current = revision_for_source(path.exists(), source_bytes.as_deref());
    if let Some(expected) = expected_revision {
        if expected != current {
            return Err(OpencodeAuthError::Stale);
        }
    }
    if raw.remove(official_key).is_none() {
        return Err(OpencodeAuthError::NotFound);
    }
    commit_map(path, &raw, source_bytes.as_deref())
}

pub(crate) fn remove_capability(
    path: &Path,
    capability_id: &str,
    expected_revision: Option<&str>,
) -> Result<AuthJsonWriteReceipt, OpencodeAuthError> {
    let observed = observe_auth_store(path);
    let official_key = observed
        .providers
        .iter()
        .find(|provider| provider.capability_id() == capability_id)
        .map(|provider| provider.official_key().to_string())
        .ok_or(OpencodeAuthError::NotFound)?;
    remove_file_key(path, &official_key, expected_revision)
}

pub(crate) fn connection_summaries(
    observation: &OpencodeAuthObservation,
    rows: &[CredentialWithIdentity],
    connections: &[ConnectionRecord],
    checked_at: String,
) -> Vec<ManagedAuthConnectionSummary> {
    CLOSED_SLOTS
        .into_iter()
        .map(|provider| slot_summary(provider, observation, rows, connections, checked_at.clone()))
        .collect()
}

pub(crate) fn agent_auth_providers(observation: &OpencodeAuthObservation) -> Vec<(String, String)> {
    observation
        .providers
        .iter()
        .filter(|provider| provider.kind != ObservedEntryKind::Unknown)
        .take(MAX_PROVIDER_KEYS)
        .map(|provider| (provider.capability_id(), provider.label.clone()))
        .collect()
}

pub(crate) fn capability_id(official_key: &str) -> String {
    let digest = Sha256::digest(official_key.to_ascii_lowercase().as_bytes());
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("p1:{suffix}")
}

pub(crate) fn opencode_expires_ms(expires_at: Option<i64>) -> u64 {
    // Official oauth `expires` is a non-negative int; first-party callbacks
    // store epoch milliseconds. Managed Auth bundles use unix seconds.
    match expires_at {
        Some(seconds) if seconds > 0 => u64::try_from(seconds)
            .ok()
            .and_then(|value| value.checked_mul(1000))
            .unwrap_or(0),
        _ => 0,
    }
}

fn slot_summary(
    provider: ManagedAuthProvider,
    observation: &OpencodeAuthObservation,
    rows: &[CredentialWithIdentity],
    connections: &[ConnectionRecord],
    checked_at: String,
) -> ManagedAuthConnectionSummary {
    let file_kind = observation.closed_kind(provider);
    let stored = connections.iter().find(|row| {
        row.consumer == ManagedAuthConsumer::Opencode && row.provider_slot == provider.as_str()
    });
    let independent = rows.iter().find(|row| {
        row.credential.provider == provider
            && row.credential.purpose == CredentialPurpose::OpencodeProvider
            && row.credential.status == CredentialStatus::Ready
    });
    let pending_restart = stored.is_some_and(|row| row.pending_restart);
    let connected = observation.file_present
        && matches!(
            file_kind,
            Some(ObservedEntryKind::Oauth | ObservedEntryKind::Api)
        );
    let auth_status = if !observation.readable {
        ManagedAuthConnectionState::Unavailable
    } else if pending_restart {
        ManagedAuthConnectionState::PendingRestart
    } else if connected {
        ManagedAuthConnectionState::Connected
    } else {
        ManagedAuthConnectionState::Disconnected
    };
    let mut reason_codes = Vec::new();
    if !observation.readable {
        reason_codes.push(ManagedAuthReasonCode::ObserverUnavailable);
    }
    if pending_restart {
        reason_codes.push(ManagedAuthReasonCode::PendingRestart);
    }
    let mut allowed_actions = vec![ManagedAuthConnectionAction::Refresh];
    if connected {
        allowed_actions.push(ManagedAuthConnectionAction::Disconnect);
    }
    if independent.is_some() {
        if connected {
            allowed_actions.push(ManagedAuthConnectionAction::SwitchAccount);
        } else {
            allowed_actions.push(ManagedAuthConnectionAction::ConnectAccount);
        }
    }
    if pending_restart {
        allowed_actions.push(ManagedAuthConnectionAction::Restart);
    }
    let account = stored
        .and_then(|row| row.credential_id.as_ref())
        .and_then(|id| rows.iter().find(|row| row.credential.credential_id == *id))
        .or(independent);
    ManagedAuthConnectionSummary {
        connection_id: stored
            .map(|row| row.connection_id.clone())
            .unwrap_or_else(|| slot_connection_id(provider)),
        revision: stored
            .and_then(|row| row.observed_revision.clone())
            .unwrap_or_else(|| observation.revision.clone()),
        consumer: ManagedAuthConsumer::Opencode,
        target_id: None,
        target_label: None,
        provider: Some(provider),
        account_id: account.map(|row| row.identity.identity_id.clone()),
        auth_status,
        credential_manager: if connected {
            ManagedAuthCredentialManager::Opencode
        } else {
            ManagedAuthCredentialManager::Unavailable
        },
        request_mode: ManagedAuthRequestMode::ProviderConnections,
        request_provider_label: Some(display_label(provider).to_string()),
        official_session_preserved: Some(true),
        pending_restart,
        allowed_actions,
        checked_at,
        reason_codes,
    }
}

fn observe_auth_store_locked(path: &Path) -> OpencodeAuthObservation {
    match read_auth_bytes(path) {
        Ok(None) => OpencodeAuthObservation::empty_missing(),
        Ok(Some(bytes)) => classify_bytes(&bytes),
        Err(_) => OpencodeAuthObservation::unreadable(path.exists()),
    }
}

fn classify_bytes(bytes: &[u8]) -> OpencodeAuthObservation {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return OpencodeAuthObservation::unreadable(true);
    };
    let Some(object) = value.as_object() else {
        return OpencodeAuthObservation::unreadable(true);
    };
    if object.len() > MAX_PROVIDER_KEYS {
        return OpencodeAuthObservation::unreadable(true);
    }
    let mut providers = Vec::new();
    let mut unknown_entry_count = 0;
    for (key, entry) in object {
        match classify_entry(key, entry) {
            Some(provider) => {
                if provider.kind == ObservedEntryKind::Unknown {
                    unknown_entry_count += 1;
                }
                providers.push(provider);
            }
            None => unknown_entry_count += 1,
        }
    }
    OpencodeAuthObservation {
        file_present: true,
        readable: true,
        revision: revision_for_source(true, Some(bytes)),
        providers,
        unknown_entry_count,
    }
}

fn classify_entry(key: &str, value: &Value) -> Option<ObservedProvider> {
    if key.is_empty() || key.chars().count() > MAX_PROVIDER_KEY_CHARS {
        return None;
    }
    let managed_provider = provider_from_file_key(key);
    let kind = parse_entry_kind(value);
    let include = managed_provider.is_some()
        || (is_safe_provider_key(key) && kind != ObservedEntryKind::Unknown);
    if !include {
        return None;
    }
    Some(ObservedProvider {
        official_key: key.to_string(),
        label: managed_provider
            .map(display_label)
            .unwrap_or(key)
            .to_string(),
        kind,
        managed_provider,
    })
}

fn parse_entry_kind(value: &Value) -> ObservedEntryKind {
    let Some(object) = value.as_object() else {
        return ObservedEntryKind::Unknown;
    };
    match object.get("type").and_then(Value::as_str) {
        Some("oauth") if valid_oauth(object) => ObservedEntryKind::Oauth,
        Some("api") if valid_api(object) => ObservedEntryKind::Api,
        Some("wellknown") if valid_wellknown(object) => ObservedEntryKind::WellKnown,
        _ => ObservedEntryKind::Unknown,
    }
}

fn valid_oauth(object: &Map<String, Value>) -> bool {
    non_empty_string(object.get("refresh")).is_some()
        && non_empty_string(object.get("access")).is_some()
        && non_negative_int(object.get("expires")).is_some()
        && optional_string_field(object.get("accountId"))
        && optional_string_field(object.get("enterpriseUrl"))
}

fn valid_api(object: &Map<String, Value>) -> bool {
    non_empty_string(object.get("key")).is_some() && optional_string_map(object.get("metadata"))
}

fn valid_wellknown(object: &Map<String, Value>) -> bool {
    non_empty_string(object.get("key")).is_some() && non_empty_string(object.get("token")).is_some()
}

fn non_empty_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
}

fn optional_string_field(value: Option<&Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(text)) => !text.chars().any(char::is_control),
        Some(_) => false,
    }
}

fn optional_string_map(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Object(map)) => map.values().all(
            |entry| matches!(entry, Value::String(text) if !text.chars().any(char::is_control)),
        ),
        Some(_) => false,
    }
}

fn non_negative_int(value: Option<&Value>) -> Option<u64> {
    let number = value?.as_number()?;
    if number.is_f64() {
        return None;
    }
    number.as_u64().or_else(|| {
        number
            .as_i64()
            .filter(|value| *value >= 0)
            .and_then(|value| u64::try_from(value).ok())
    })
}

fn is_safe_provider_key(key: &str) -> bool {
    let chars = key.chars().count();
    chars > 0
        && chars <= MAX_PROVIDER_KEY_CHARS
        && key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn provider_from_file_key(key: &str) -> Option<ManagedAuthProvider> {
    match key {
        "openai" => Some(ManagedAuthProvider::Openai),
        "xai" => Some(ManagedAuthProvider::Xai),
        "github-copilot" => Some(ManagedAuthProvider::GithubCopilot),
        _ => None,
    }
}

fn display_label(provider: ManagedAuthProvider) -> &'static str {
    match provider {
        ManagedAuthProvider::Openai => "OpenAI",
        ManagedAuthProvider::Xai => "xAI",
        ManagedAuthProvider::GithubCopilot => "GitHub Copilot",
    }
}

fn load_raw_map(path: &Path) -> Result<LoadedAuthJson, OpencodeAuthError> {
    match read_auth_bytes(path) {
        Ok(None) => Ok((Map::new(), None)),
        Ok(Some(bytes)) => {
            let value =
                serde_json::from_slice::<Value>(&bytes).map_err(|_| OpencodeAuthError::Invalid)?;
            let object = value
                .as_object()
                .cloned()
                .ok_or(OpencodeAuthError::Invalid)?;
            if object.len() > MAX_PROVIDER_KEYS {
                return Err(OpencodeAuthError::Invalid);
            }
            Ok((object, Some(bytes)))
        }
        Err(_) => Err(OpencodeAuthError::Io),
    }
}

fn commit_map(
    path: &Path,
    raw: &Map<String, Value>,
    preimage: Option<&[u8]>,
) -> Result<AuthJsonWriteReceipt, OpencodeAuthError> {
    let serialized = serde_json::to_vec_pretty(raw).map_err(|_| OpencodeAuthError::Invalid)?;
    write_auth_json_0600(path, &serialized).map_err(|_| OpencodeAuthError::Io)?;
    match read_auth_bytes(path) {
        Ok(Some(readback)) if readback == serialized => Ok(AuthJsonWriteReceipt {
            revision: revision_for_source(true, Some(&readback)),
            pending_restart: !OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN,
        }),
        Ok(Some(_)) | Ok(None) | Err(_) => {
            if let Some(preimage) = preimage {
                let _ = write_auth_json_0600(path, preimage);
            } else {
                let _ = fs::remove_file(path);
            }
            Err(OpencodeAuthError::Io)
        }
    }
}

fn write_auth_json_0600(path: &Path, bytes: &[u8]) -> Result<(), ManagedAuthCoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ManagedAuthCoreError::Io)?;
    }
    crate::config::atomic_write(path, bytes).map_err(|_| ManagedAuthCoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| ManagedAuthCoreError::Io)?;
    }
    Ok(())
}

fn read_auth_bytes(path: &Path) -> Result<Option<Vec<u8>>, OpencodeAuthError> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(OpencodeAuthError::Io),
    };
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_AUTH_JSON_BYTES.saturating_add(1)).expect("auth.json limit");
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| OpencodeAuthError::Io)?;
    if bytes.len() > MAX_AUTH_JSON_BYTES {
        return Err(OpencodeAuthError::Invalid);
    }
    Ok(Some(bytes))
}

fn projection_value(entry: &ProjectionEntry) -> Result<Value, OpencodeAuthError> {
    match entry {
        ProjectionEntry::Oauth {
            refresh,
            access,
            expires,
            account_id,
        } => {
            if refresh.is_empty() || access.is_empty() {
                return Err(OpencodeAuthError::Invalid);
            }
            let mut object = Map::new();
            object.insert("type".into(), Value::String("oauth".into()));
            object.insert("refresh".into(), Value::String(refresh.to_string()));
            object.insert("access".into(), Value::String(access.to_string()));
            object.insert("expires".into(), Value::Number((*expires).into()));
            if let Some(account_id) = account_id.as_deref().filter(|value| !value.is_empty()) {
                object.insert("accountId".into(), Value::String(account_id.to_string()));
            }
            Ok(Value::Object(object))
        }
        ProjectionEntry::Api { key } => {
            if key.is_empty() {
                return Err(OpencodeAuthError::Invalid);
            }
            Ok(serde_json::json!({
                "type": "api",
                "key": key.as_str(),
            }))
        }
    }
}

fn revision_for_source(present: bool, bytes: Option<&[u8]>) -> String {
    match (present, bytes) {
        (true, Some(bytes)) => stable_revision(&["opencode-auth-json", &sha256_hex(bytes)]),
        _ => missing_revision(),
    }
}

fn missing_revision() -> String {
    stable_revision(&["opencode-auth-json", "missing"])
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::managed_auth::now_timestamp;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_fixture(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }

    fn leak_scan(value: &Value) {
        let text = value.to_string().to_ascii_lowercase();
        for forbidden in [
            "access_token",
            "refresh_token",
            "id_token",
            "authorization_code",
            "device_code",
            "secretref",
            "secret_ref",
            "verifier",
            "rt-secret",
            "sk-secret",
            "auth.json",
        ] {
            assert!(!text.contains(forbidden), "leaked {forbidden}: {text}");
        }
    }

    #[test]
    fn observe_parses_oauth_and_api_without_leaking_secrets() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "openai": {
                    "type": "oauth",
                    "refresh": "rt-secret",
                    "access": "at-secret",
                    "expires": 1_700_000_000_000u64,
                    "accountId": "acct_1"
                },
                "anthropic": {
                    "type": "api",
                    "key": "sk-secret"
                }
            }),
        );
        let observed = observe_auth_store(&path);
        assert!(observed.readable);
        assert_eq!(
            observed.closed_kind(ManagedAuthProvider::Openai),
            Some(ObservedEntryKind::Oauth)
        );
        let labels: Vec<_> = agent_auth_providers(&observed)
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        assert!(labels.contains(&"OpenAI".to_string()));
        assert!(labels.contains(&"anthropic".to_string()));
        let summaries = connection_summaries(&observed, &[], &[], now_timestamp());
        let encoded = serde_json::to_value(&summaries).unwrap();
        leak_scan(&encoded);
        assert_eq!(summaries.len(), 3);
        assert_eq!(
            summaries
                .iter()
                .find(|row| row.provider == Some(ManagedAuthProvider::Openai))
                .map(|row| row.auth_status),
            Some(ManagedAuthConnectionState::Connected)
        );
        assert_eq!(
            summaries
                .iter()
                .find(|row| row.provider == Some(ManagedAuthProvider::Openai))
                .map(|row| row.credential_manager),
            Some(ManagedAuthCredentialManager::Opencode)
        );
    }

    #[test]
    fn default_path_is_auth_json_under_opencode_data_dir() {
        assert_eq!(
            default_auth_json_path(),
            crate::opencode_config::get_opencode_data_dir().join(AUTH_JSON_NAME)
        );
    }

    #[test]
    fn missing_file_is_empty_not_observer_failure() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing").join(AUTH_JSON_NAME);
        let observed = observe_auth_store(&path);
        assert!(observed.readable);
        assert!(!observed.file_present);
        assert!(observed.providers.is_empty());
        let summaries = connection_summaries(&observed, &[], &[], now_timestamp());
        assert!(summaries
            .iter()
            .all(|row| row.auth_status == ManagedAuthConnectionState::Disconnected));
    }

    #[test]
    fn rmw_preserves_unknown_keys_and_does_not_drop_undecodable_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "openai": {
                    "type": "oauth",
                    "refresh": "rt-old",
                    "access": "at-old",
                    "expires": 1
                },
                "future-plugin": {
                    "type": "mystery",
                    "token": "leave-me"
                },
                "broken": 42,
                "https://example.invalid/wellknown/": {
                    "type": "wellknown",
                    "key": "wk-key",
                    "token": "wk-token"
                }
            }),
        );
        let receipt = upsert_projection(
            &path,
            ManagedAuthProvider::Xai,
            &ProjectionEntry::Api {
                key: Zeroizing::new("sk-xai".into()),
            },
            None,
        )
        .expect("project xai");
        assert!(receipt.pending_restart);
        let raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["future-plugin"]["type"], "mystery");
        assert_eq!(raw["broken"], 42);
        assert_eq!(
            raw["https://example.invalid/wellknown/"]["type"],
            "wellknown"
        );
        assert_eq!(raw["openai"]["refresh"], "rt-old");
        assert_eq!(raw["xai"]["type"], "api");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn remove_keeps_foreign_keys_and_env_rows_are_not_invented() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "openai": {
                    "type": "oauth",
                    "refresh": "rt-secret",
                    "access": "at-secret",
                    "expires": 9
                },
                "custom": {
                    "type": "api",
                    "key": "keep"
                }
            }),
        );
        remove_file_key(&path, "openai", None).expect("remove openai");
        let raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(raw.get("openai").is_none());
        assert_eq!(raw["custom"]["key"], "keep");
        assert!(raw.get("OPENAI_API_KEY").is_none());
    }

    #[test]
    fn cas_rejects_stale_revision_and_leaves_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "openai": {
                    "type": "api",
                    "key": "keep"
                }
            }),
        );
        let error = upsert_projection(
            &path,
            ManagedAuthProvider::Openai,
            &ProjectionEntry::Api {
                key: Zeroizing::new("new".into()),
            },
            Some("mr1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        )
        .expect_err("stale");
        assert_eq!(error, OpencodeAuthError::Stale);
        let raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["openai"]["key"], "keep");
    }

    #[test]
    fn url_keys_are_omitted_from_agent_auth_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "https://example.invalid/": {
                    "type": "wellknown",
                    "key": "wk-key",
                    "token": "wk-token"
                }
            }),
        );
        let observed = observe_auth_store(&path);
        assert!(agent_auth_providers(&observed).is_empty());
        assert!(observed.unknown_entry_count >= 1);
        let encoded = serde_json::to_value(agent_auth_providers(&observed)).unwrap();
        leak_scan(&encoded);
        assert!(!serde_json::to_string(&encoded)
            .unwrap()
            .contains("example.invalid"));
    }

    #[test]
    fn malformed_oauth_is_unknown_and_preserved() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "openai": {
                    "type": "oauth",
                    "access": "missing-refresh",
                    "expires": 1
                }
            }),
        );
        let observed = observe_auth_store(&path);
        assert_eq!(
            observed.closed_kind(ManagedAuthProvider::Openai),
            Some(ObservedEntryKind::Unknown)
        );
        upsert_projection(
            &path,
            ManagedAuthProvider::Xai,
            &ProjectionEntry::Api {
                key: Zeroizing::new("sk".into()),
            },
            None,
        )
        .unwrap();
        let raw: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["openai"]["access"], "missing-refresh");
        assert!(raw["openai"].get("refresh").is_none());
    }

    #[test]
    fn no_cli_path_is_used_for_observation() {
        let dir = tempdir().unwrap();
        let path = dir
            .path()
            .join(".local")
            .join("share")
            .join("opencode")
            .join(AUTH_JSON_NAME);
        write_fixture(
            &path,
            &json!({
                "xai": {
                    "type": "api",
                    "key": "sk-secret"
                }
            }),
        );
        let observed = observe_auth_store(&path);
        assert_eq!(
            observed.closed_kind(ManagedAuthProvider::Xai),
            Some(ObservedEntryKind::Api)
        );
        assert!(agent_auth_providers(&observed)
            .iter()
            .any(|(id, label)| { label == "xAI" && id.starts_with("p1:") && id.len() == 35 }));
    }
}
