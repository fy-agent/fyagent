//! Codex consumer observation. Native file projection stays gated until HIL.

use std::path::{Path, PathBuf};

use crate::codex_config::is_custom_codex_model_provider_id;
use crate::codex_config::{parse_cli_auth_credentials_store, CodexCredentialStore};
use crate::services::managed_auth::{
    stable_connection_id, stable_revision, ConnectionRecord, CredentialPurpose,
    CredentialWithIdentity, ManagedAuthConnectionAction, ManagedAuthConnectionState,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthCredentialManager,
    ManagedAuthProvider, ManagedAuthReasonCode, ManagedAuthRequestMode,
};

/// Production file-store projection is closed until matching-host HIL exists.
pub(crate) const CODEX_FILE_PROJECTION_PRODUCTION_ENABLED: bool = false;

#[derive(Debug, Clone)]
pub(crate) struct CodexObservation {
    pub request_mode: ManagedAuthRequestMode,
    pub request_provider_label: Option<String>,
    pub store: CodexCredentialStore,
    #[allow(dead_code)]
    pub official_session_preserved: bool,
}

impl Default for CodexObservation {
    fn default() -> Self {
        Self {
            request_mode: ManagedAuthRequestMode::Unknown,
            request_provider_label: None,
            store: CodexCredentialStore::Unset,
            official_session_preserved: true,
        }
    }
}

pub(crate) fn observe_codex_home(codex_home: &Path) -> CodexObservation {
    let config_path = codex_home.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&config_path) else {
        return CodexObservation {
            request_mode: ManagedAuthRequestMode::None,
            request_provider_label: None,
            store: CodexCredentialStore::Unset,
            official_session_preserved: true,
        };
    };
    let store = parse_cli_auth_credentials_store(&text).unwrap_or(CodexCredentialStore::Unknown);
    let provider_id = parse_model_provider(&text);
    let (request_mode, request_provider_label) = match provider_id.as_deref() {
        None => (ManagedAuthRequestMode::None, None),
        Some(id) if !is_custom_codex_model_provider_id(id) => (
            ManagedAuthRequestMode::OfficialSubscription,
            Some(id.to_string()),
        ),
        Some(id) => (ManagedAuthRequestMode::ThirdPartyApi, Some(id.to_string())),
    };
    CodexObservation {
        request_mode,
        request_provider_label,
        store,
        official_session_preserved: true,
    }
}

fn parse_model_provider(config_toml: &str) -> Option<String> {
    let document = config_toml.parse::<toml_edit::DocumentMut>().ok()?;
    document
        .get("model_provider")
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn file_projection_enabled() -> bool {
    CODEX_FILE_PROJECTION_PRODUCTION_ENABLED
}

pub(crate) fn connection_summary(
    observation: &CodexObservation,
    account: Option<&CredentialWithIdentity>,
    connection: Option<&ConnectionRecord>,
    checked_at: String,
) -> ManagedAuthConnectionSummary {
    let projection_ready =
        file_projection_enabled() && observation.store.allows_native_file_projection();
    let connected = account.is_some();
    let pending_restart = connection.is_some_and(|row| row.pending_restart);
    let auth_status = if pending_restart {
        ManagedAuthConnectionState::PendingRestart
    } else if connected {
        ManagedAuthConnectionState::Connected
    } else {
        ManagedAuthConnectionState::Disconnected
    };
    let mut reason_codes = Vec::new();
    if !projection_ready {
        reason_codes.push(ManagedAuthReasonCode::NativeProjectionUnavailable);
    }
    let mut allowed_actions = vec![ManagedAuthConnectionAction::Refresh];
    if connected {
        allowed_actions.push(ManagedAuthConnectionAction::Disconnect);
    } else if account_connectable(account) {
        allowed_actions.push(ManagedAuthConnectionAction::ConnectAccount);
    }
    let connection_id = connection
        .map(|row| row.connection_id.clone())
        .unwrap_or_else(|| stable_connection_id(ManagedAuthConsumer::Codex, "", "openai"));
    let revision = connection
        .and_then(|row| row.observed_revision.clone())
        .unwrap_or_else(|| {
            stable_revision(&[
                "codex-observation",
                observation.request_mode_label(),
                observation.store.as_str(),
            ])
        });
    ManagedAuthConnectionSummary {
        connection_id,
        revision,
        consumer: ManagedAuthConsumer::Codex,
        target_id: None,
        target_label: None,
        provider: Some(ManagedAuthProvider::Openai),
        account_id: account.map(|row| row.identity.identity_id.clone()),
        auth_status,
        credential_manager: if projection_ready {
            ManagedAuthCredentialManager::Codex
        } else {
            ManagedAuthCredentialManager::Unavailable
        },
        request_mode: observation.request_mode,
        request_provider_label: observation.request_provider_label.clone(),
        official_session_preserved: Some(true),
        pending_restart,
        allowed_actions,
        checked_at,
        reason_codes,
    }
}

fn account_connectable(account: Option<&CredentialWithIdentity>) -> bool {
    account.is_some_and(|row| {
        row.credential.purpose == CredentialPurpose::CodexNative
            && row.credential.status == crate::services::managed_auth::CredentialStatus::Ready
    })
}

impl CodexObservation {
    fn request_mode_label(&self) -> &'static str {
        match self.request_mode {
            ManagedAuthRequestMode::OfficialSubscription => "official",
            ManagedAuthRequestMode::ThirdPartyApi => "third_party",
            ManagedAuthRequestMode::ProviderConnections => "providers",
            ManagedAuthRequestMode::None => "none",
            ManagedAuthRequestMode::Unknown => "unknown",
        }
    }
}

#[allow(dead_code)]
pub(crate) fn default_codex_home() -> PathBuf {
    crate::codex_config::get_codex_config_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::managed_auth::now_timestamp;
    use tempfile::tempdir;

    #[test]
    fn third_party_model_provider_does_not_imply_logout() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"
model_provider = "deepseek"
cli_auth_credentials_store = "file"
"#,
        )
        .unwrap();
        let observed = observe_codex_home(dir.path());
        assert_eq!(observed.request_mode, ManagedAuthRequestMode::ThirdPartyApi);
        assert_eq!(observed.request_provider_label.as_deref(), Some("deepseek"));
        assert!(observed.official_session_preserved);
        let summary = connection_summary(&observed, None, None, now_timestamp());
        assert_eq!(summary.request_mode, ManagedAuthRequestMode::ThirdPartyApi);
        assert_eq!(summary.official_session_preserved, Some(true));
        assert_eq!(
            summary.auth_status,
            ManagedAuthConnectionState::Disconnected
        );
        assert!(summary
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        let text = serde_json::to_string(&summary)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!text.contains("auth.json"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn reserved_openai_provider_is_official_subscription() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "model_provider = \"openai\"\n",
        )
        .unwrap();
        let observed = observe_codex_home(dir.path());
        assert_eq!(
            observed.request_mode,
            ManagedAuthRequestMode::OfficialSubscription
        );
    }

    #[test]
    fn missing_config_is_none_not_success() {
        let dir = tempdir().unwrap();
        let observed = observe_codex_home(dir.path());
        assert_eq!(observed.request_mode, ManagedAuthRequestMode::None);
        assert!(observed.official_session_preserved);
    }
}
