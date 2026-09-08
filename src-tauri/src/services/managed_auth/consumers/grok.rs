//! Grok Build consumer observation. Native projection and helper stay gated.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::services::managed_auth::{
    stable_connection_id, stable_revision, ConnectionRecord, CredentialPurpose,
    CredentialWithIdentity, ManagedAuthConnectionAction, ManagedAuthConnectionState,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthCredentialManager,
    ManagedAuthProvider, ManagedAuthReasonCode, ManagedAuthRequestMode,
};

/// Production `auth_provider_command` stays disabled until macOS/Windows HIL.
pub(crate) const GROK_AUTH_PROVIDER_COMMAND_ENABLED: bool = false;
/// Production `auth.json` writes stay disabled until lock/HIL evidence exists.
pub(crate) const GROK_FILE_PROJECTION_PRODUCTION_ENABLED: bool = false;

#[allow(dead_code)]
const MAX_AUTH_JSON_BYTES: usize = 256 * 1024;
#[allow(dead_code)]
const AUTH_JSON_NAME: &str = "auth.json";
#[allow(dead_code)]
const AUTH_LOCK_NAME: &str = "auth.json.lock";

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub(crate) enum GrokStoreError {
    #[error("grok native projection is unsupported")]
    Unsupported,
    #[error("grok auth store is invalid")]
    Invalid,
    #[error("grok auth store io failed")]
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum GrokAuthMode {
    Oidc,
    External,
    ApiKey,
    #[serde(rename = "grok")]
    WebLogin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct GrokAuthEntry {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<GrokAuthMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oidc_issuer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oidc_client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[allow(dead_code)]
pub(crate) type GrokAuthStore = BTreeMap<String, GrokAuthEntry>;

pub(crate) fn auth_provider_command_enabled() -> bool {
    GROK_AUTH_PROVIDER_COMMAND_ENABLED
}

pub(crate) fn file_projection_enabled() -> bool {
    GROK_FILE_PROJECTION_PRODUCTION_ENABLED
}

#[allow(dead_code)]
pub(crate) fn parse_grok_auth_store(bytes: &[u8]) -> Result<GrokAuthStore, GrokStoreError> {
    if bytes.len() > MAX_AUTH_JSON_BYTES {
        return Err(GrokStoreError::Invalid);
    }
    if bytes.contains(&0) {
        return Err(GrokStoreError::Invalid);
    }
    let root: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| GrokStoreError::Invalid)?;
    let object = root.as_object().ok_or(GrokStoreError::Invalid)?;
    let mut store = GrokAuthStore::new();
    for (scope, value) in object {
        if scope.is_empty() || scope.len() > 512 {
            return Err(GrokStoreError::Invalid);
        }
        let entry: GrokAuthEntry =
            serde_json::from_value(value.clone()).map_err(|_| GrokStoreError::Invalid)?;
        if entry.key.trim().is_empty() {
            return Err(GrokStoreError::Invalid);
        }
        store.insert(scope.clone(), entry);
    }
    Ok(store)
}

#[allow(dead_code)]
pub(crate) fn merge_grok_auth_scope(store: &mut GrokAuthStore, scope: &str, entry: GrokAuthEntry) {
    store.insert(scope.to_string(), entry);
}

#[allow(dead_code)]
pub(crate) fn project_grok_native(
    home: &Path,
    store: &GrokAuthStore,
) -> Result<(), GrokStoreError> {
    if !file_projection_enabled() {
        return Err(GrokStoreError::Unsupported);
    }
    write_grok_auth_store(home, store)
}

#[allow(dead_code)]
pub(crate) fn write_grok_auth_store(
    home: &Path,
    store: &GrokAuthStore,
) -> Result<(), GrokStoreError> {
    #[cfg(not(test))]
    if !file_projection_enabled() {
        return Err(GrokStoreError::Unsupported);
    }
    fs::create_dir_all(home).map_err(|_| GrokStoreError::Io)?;
    let lock = GrokAuthLock::acquire(&home.join(AUTH_LOCK_NAME))?;
    let path = home.join(AUTH_JSON_NAME);
    let encoded = serde_json::to_vec_pretty(store).map_err(|_| GrokStoreError::Invalid)?;
    write_auth_json_atomic(&path, &encoded)?;
    drop(lock);
    Ok(())
}

#[allow(dead_code)]
pub(crate) struct GrokAuthLock {
    _file: fs::File,
}

impl GrokAuthLock {
    #[allow(dead_code)]
    pub(crate) fn acquire(lock_path: &Path) -> Result<Self, GrokStoreError> {
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|_| GrokStoreError::Io)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| GrokStoreError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
            if rc != 0 {
                return Err(GrokStoreError::Io);
            }
        }
        #[cfg(windows)]
        {
            let _ = &file;
        }
        Ok(Self { _file: file })
    }

    #[allow(dead_code)]
    pub(crate) fn try_acquire(lock_path: &Path) -> Result<Option<Self>, GrokStoreError> {
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|_| GrokStoreError::Io)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| GrokStoreError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if rc != 0 {
                return Ok(None);
            }
        }
        Ok(Some(Self { _file: file }))
    }
}

impl Drop for GrokAuthLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let _ = unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
        }
        // Official Grok never unlinks auth.json.lock.
    }
}

#[allow(dead_code)]
fn write_auth_json_atomic(path: &Path, content: &[u8]) -> Result<(), GrokStoreError> {
    let parent = path.parent().ok_or(GrokStoreError::Io)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let temporary = parent.join(format!("auth.json.{pid}.{nonce}.tmp"));
    let result = (|| -> Result<(), std::io::Error> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(content)?;
            file.flush()?;
            fs::rename(&temporary, path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        #[cfg(windows)]
        {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(content)?;
            file.flush()?;
            if path.exists() {
                fs::remove_file(path)?;
            }
            fs::rename(&temporary, path)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|_| GrokStoreError::Io)
}

pub(crate) fn connection_summary(
    account: Option<&CredentialWithIdentity>,
    connection: Option<&ConnectionRecord>,
    checked_at: String,
) -> ManagedAuthConnectionSummary {
    let projection_ready = file_projection_enabled() || auth_provider_command_enabled();
    let pending_restart = connection.is_some_and(|row| row.pending_restart);
    let auth_status = if pending_restart {
        ManagedAuthConnectionState::PendingRestart
    } else if projection_ready && account_connectable(account) {
        ManagedAuthConnectionState::Connected
    } else if account.is_some() {
        ManagedAuthConnectionState::Unavailable
    } else {
        ManagedAuthConnectionState::Disconnected
    };
    let reason_codes = vec![ManagedAuthReasonCode::NativeProjectionUnavailable];
    let mut allowed_actions = vec![ManagedAuthConnectionAction::Refresh];
    if account.is_some() {
        allowed_actions.push(ManagedAuthConnectionAction::Disconnect);
    } else if account_connectable(account) {
        allowed_actions.push(ManagedAuthConnectionAction::ConnectAccount);
    }
    let connection_id = connection
        .map(|row| row.connection_id.clone())
        .unwrap_or_else(|| stable_connection_id(ManagedAuthConsumer::Grokbuild, "", "xai"));
    let revision = connection
        .and_then(|row| row.observed_revision.clone())
        .unwrap_or_else(|| stable_revision(&["grok-observation", "unsupported"]));
    ManagedAuthConnectionSummary {
        connection_id,
        revision,
        consumer: ManagedAuthConsumer::Grokbuild,
        target_id: None,
        target_label: None,
        provider: Some(ManagedAuthProvider::Xai),
        account_id: account.map(|row| row.identity.identity_id.clone()),
        auth_status,
        credential_manager: ManagedAuthCredentialManager::Unavailable,
        request_mode: ManagedAuthRequestMode::OfficialSubscription,
        request_provider_label: Some("xai".to_string()),
        official_session_preserved: Some(true),
        pending_restart,
        allowed_actions,
        checked_at,
        reason_codes,
    }
}

fn account_connectable(account: Option<&CredentialWithIdentity>) -> bool {
    account.is_some_and(|row| {
        row.credential.purpose == CredentialPurpose::GrokNative
            && row.credential.status == crate::services::managed_auth::CredentialStatus::Ready
            && file_projection_enabled()
    })
}

#[allow(dead_code)]
pub(crate) fn default_grok_home() -> PathBuf {
    crate::grok_config::get_grok_config_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_entry(key: &str) -> GrokAuthEntry {
        GrokAuthEntry {
            key: key.to_string(),
            auth_mode: Some(GrokAuthMode::Oidc),
            refresh_token: Some("rt-grok".into()),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            oidc_issuer: Some("https://auth.x.ai".into()),
            oidc_client_id: Some("b1a00492-073a-47ea-816f-4c329264a828".into()),
            user_id: Some("user-123".into()),
            extra: BTreeMap::from([(
                "coding_data_retention_opt_out".to_string(),
                serde_json::Value::Bool(true),
            )]),
        }
    }

    #[test]
    fn helper_and_native_projection_stay_disabled() {
        assert!(!auth_provider_command_enabled());
        assert!(!file_projection_enabled());
        let dir = tempdir().unwrap();
        let mut store = GrokAuthStore::new();
        merge_grok_auth_scope(
            &mut store,
            "https://auth.x.ai::b1a00492-073a-47ea-816f-4c329264a828",
            sample_entry("access"),
        );
        assert!(matches!(
            project_grok_native(dir.path(), &store),
            Err(GrokStoreError::Unsupported)
        ));
        assert!(!dir.path().join(AUTH_JSON_NAME).exists());
    }

    #[test]
    fn parser_is_strict_and_preserves_unknown_fields() {
        let json = br#"{
            "https://auth.x.ai::client": {
                "key": "access-one",
                "auth_mode": "oidc",
                "user_id": "user-123",
                "team_name": "atlas"
            },
            "https://accounts.x.ai/sign-in": {
                "key": "legacy-key",
                "auth_mode": "grok"
            }
        }"#;
        let store = parse_grok_auth_store(json).expect("parse");
        assert_eq!(store.len(), 2);
        let official = store.get("https://auth.x.ai::client").expect("official");
        assert_eq!(official.key, "access-one");
        assert_eq!(
            official
                .extra
                .get("team_name")
                .and_then(|value| value.as_str()),
            Some("atlas")
        );
        assert!(parse_grok_auth_store(b"[]").is_err());
        assert!(parse_grok_auth_store(br#"{"scope":"not-an-object"}"#).is_err());
        assert!(parse_grok_auth_store(br#"{"scope":{"key":""}}"#).is_err());
        assert!(parse_grok_auth_store(&vec![0u8; MAX_AUTH_JSON_BYTES + 1]).is_err());
    }

    #[test]
    fn read_modify_write_keeps_sibling_scopes_under_lock() {
        let dir = tempdir().unwrap();
        let mut store = GrokAuthStore::new();
        merge_grok_auth_scope(
            &mut store,
            "https://auth.x.ai::keep",
            sample_entry("keep-access"),
        );
        write_grok_auth_store(dir.path(), &store).expect("write");
        let lock_path = dir.path().join(AUTH_LOCK_NAME);
        assert!(lock_path.exists(), "lock file must remain");
        let mut reloaded =
            parse_grok_auth_store(&fs::read(dir.path().join(AUTH_JSON_NAME)).unwrap()).unwrap();
        merge_grok_auth_scope(
            &mut reloaded,
            "https://auth.x.ai::added",
            sample_entry("added-access"),
        );
        write_grok_auth_store(dir.path(), &reloaded).expect("merge write");
        let final_store =
            parse_grok_auth_store(&fs::read(dir.path().join(AUTH_JSON_NAME)).unwrap()).unwrap();
        assert!(final_store.contains_key("https://auth.x.ai::keep"));
        assert!(final_store.contains_key("https://auth.x.ai::added"));
        assert!(lock_path.exists());
        let held = GrokAuthLock::acquire(&lock_path).expect("first lock");
        #[cfg(unix)]
        {
            assert!(GrokAuthLock::try_acquire(&lock_path)
                .expect("try")
                .is_none());
            drop(held);
            assert!(GrokAuthLock::try_acquire(&lock_path)
                .expect("retry")
                .is_some());
        }
        #[cfg(not(unix))]
        {
            drop(held);
        }
        assert!(lock_path.exists(), "drop must not unlink the lock");
    }

    #[test]
    fn connection_summary_does_not_claim_verified_login() {
        let summary =
            connection_summary(None, None, crate::services::managed_auth::now_timestamp());
        assert_eq!(
            summary.auth_status,
            ManagedAuthConnectionState::Disconnected
        );
        assert_eq!(
            summary.credential_manager,
            ManagedAuthCredentialManager::Unavailable
        );
        assert!(summary
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        let text = serde_json::to_string(&summary)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!text.contains("auth.json"));
        assert!(!text.contains("refresh_token"));
        assert!(!text.contains("auth_provider_command"));
    }
}
