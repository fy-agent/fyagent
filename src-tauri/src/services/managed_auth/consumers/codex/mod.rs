//! Codex Managed Auth consumer: observation, delta, auth swap, and projection.

mod auth_document;
mod delta;
mod observation;
mod project;
mod swap;

pub(crate) use auth_document::{CodexChatGptAuthDocument, CodexNativeAuthState};
pub(crate) use observation::{
    auth_matches_account, live_chatgpt_account_id, observe_managed_auth,
    CodexManagedAuthObservation,
};
pub(crate) use project::{materialize_from_bundle, project_codex_official_account};
pub(crate) use swap::{auth_path_in, capture_auth_preimage};

use crate::services::managed_auth::{
    stable_connection_id, stable_revision, ConnectionRecord, CredentialPurpose, CredentialStatus,
    CredentialWithIdentity, ManagedAuthConnectionAction, ManagedAuthConnectionState,
    ManagedAuthConnectionSummary, ManagedAuthConsumer, ManagedAuthCredentialManager,
    ManagedAuthProvider, ManagedAuthReasonCode,
};

/// Capability is decided by effective store + live evidence, not a blanket HIL gate.
#[deprecated(note = "use effective store and live observation instead")]
#[allow(dead_code)]
pub(crate) const CODEX_FILE_PROJECTION_PRODUCTION_ENABLED: bool = true;

pub(crate) fn file_projection_enabled() -> bool {
    // Retained as a soft helper for call sites that only need "is file projection
    // generally available". Unsupported stores still fail closed at plan time.
    true
}

pub(crate) fn observe_codex_home(codex_home: &std::path::Path) -> CodexManagedAuthObservation {
    observe_managed_auth(codex_home)
}

pub(crate) fn connection_summary(
    observation: &CodexManagedAuthObservation,
    account: Option<&CredentialWithIdentity>,
    connection: Option<&ConnectionRecord>,
    all_accounts: &[CredentialWithIdentity],
    checked_at: String,
) -> ManagedAuthConnectionSummary {
    let store_ready = observation.effective_store.allows_native_file_projection();
    let bound = connection
        .and_then(|row| row.credential_id.as_ref())
        .and_then(|credential_id| {
            all_accounts
                .iter()
                .find(|row| row.credential.credential_id == *credential_id)
        })
        .or(account.filter(|row| {
            connection.is_some_and(|conn| {
                conn.credential_id.as_deref() == Some(row.credential.credential_id.as_str())
            })
        }));

    // Prefer the connection-bound credential; fall back only for action ads when
    // a ready CodexNative credential exists but is not yet projected.
    let ready_saved = all_accounts.iter().find(|row| {
        row.credential.purpose == CredentialPurpose::CodexNative
            && row.credential.status == CredentialStatus::Ready
    });

    let live_matches_bound = bound.is_some_and(|row| {
        auth_matches_account(&observation.auth_state, &row.identity.provider_subject)
    });

    let pending_restart = connection.is_some_and(|row| row.pending_restart);
    let mut reason_codes = Vec::new();

    if !store_ready {
        reason_codes.push(ManagedAuthReasonCode::NativeProjectionUnavailable);
    }

    let auth_status = if pending_restart {
        ManagedAuthConnectionState::PendingRestart
    } else if live_matches_bound {
        ManagedAuthConnectionState::Connected
    } else if bound.is_some() || ready_saved.is_some() {
        // Saved credential exists but live Codex is not using it.
        ManagedAuthConnectionState::Disconnected
    } else {
        ManagedAuthConnectionState::Disconnected
    };

    if matches!(
        observation.auth_state,
        CodexNativeAuthState::ChatGptUnmanaged { .. }
    ) {
        reason_codes.push(ManagedAuthReasonCode::ExternalChangeDetected);
    }
    if matches!(
        &observation.auth_state,
        CodexNativeAuthState::Invalid { .. }
            | CodexNativeAuthState::Unreadable
            | CodexNativeAuthState::Oversized
    ) {
        reason_codes.push(ManagedAuthReasonCode::ExternalChangeDetected);
    }

    let official_session_preserved = matches!(
        &observation.auth_state,
        CodexNativeAuthState::ChatGptKnown { .. }
    ) && !observation.provider_route.is_official();

    let mut allowed_actions = vec![ManagedAuthConnectionAction::Refresh];
    if live_matches_bound {
        allowed_actions.push(ManagedAuthConnectionAction::Disconnect);
        if !observation.provider_route.is_official() && store_ready {
            allowed_actions.push(ManagedAuthConnectionAction::SwitchToOfficial);
        }
    } else if account_connectable(ready_saved) && store_ready {
        allowed_actions.push(ManagedAuthConnectionAction::ConnectAccount);
    }
    if pending_restart {
        allowed_actions.push(ManagedAuthConnectionAction::Restart);
    }

    let display_account = if live_matches_bound {
        bound
    } else {
        connection
            .and(bound)
            .or(ready_saved.filter(|_| bound.is_none()))
    };

    let connection_id = connection
        .map(|row| row.connection_id.clone())
        .unwrap_or_else(|| stable_connection_id(ManagedAuthConsumer::Codex, "", "openai"));
    let revision = connection
        .and_then(|row| row.observed_revision.clone())
        .or_else(|| observation.auth_revision.clone())
        .unwrap_or_else(|| {
            stable_revision(&[
                "codex-observation",
                observation.request_mode_label(),
                observation.effective_store.as_str(),
            ])
        });

    ManagedAuthConnectionSummary {
        connection_id,
        revision,
        consumer: ManagedAuthConsumer::Codex,
        target_id: None,
        target_label: None,
        provider: Some(ManagedAuthProvider::Openai),
        account_id: display_account.map(|row| row.identity.identity_id.clone()),
        auth_status,
        credential_manager: if store_ready {
            ManagedAuthCredentialManager::Codex
        } else {
            ManagedAuthCredentialManager::Unavailable
        },
        request_mode: observation.request_mode,
        request_provider_label: observation.request_provider_label.clone(),
        official_session_preserved: Some(official_session_preserved),
        pending_restart,
        allowed_actions,
        checked_at,
        reason_codes,
    }
}

fn account_connectable(account: Option<&CredentialWithIdentity>) -> bool {
    account.is_some_and(|row| {
        row.credential.purpose == CredentialPurpose::CodexNative
            && row.credential.status == CredentialStatus::Ready
    })
}

impl CodexManagedAuthObservation {
    fn request_mode_label(&self) -> &'static str {
        match self.request_mode {
            crate::services::managed_auth::ManagedAuthRequestMode::OfficialSubscription => {
                "official"
            }
            crate::services::managed_auth::ManagedAuthRequestMode::ThirdPartyApi => "third_party",
            crate::services::managed_auth::ManagedAuthRequestMode::ProviderConnections => {
                "providers"
            }
            crate::services::managed_auth::ManagedAuthRequestMode::None => "none",
            crate::services::managed_auth::ManagedAuthRequestMode::Unknown => "unknown",
        }
    }
}

#[allow(dead_code)]
pub(crate) fn default_codex_home() -> std::path::PathBuf {
    crate::codex_config::get_codex_config_dir()
}

#[cfg(test)]
mod tests {
    use super::observation::CodexProviderRoute;
    use super::*;
    use crate::services::managed_auth::now_timestamp;
    use tempfile::tempdir;

    #[test]
    fn credential_presence_is_not_connected() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "model_provider = \"openai\"\n",
        )
        .unwrap();
        let observed = observe_codex_home(dir.path());
        let summary = connection_summary(&observed, None, None, &[], now_timestamp());
        assert_eq!(
            summary.auth_status,
            ManagedAuthConnectionState::Disconnected
        );
        assert!(!summary
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        assert!(summary
            .allowed_actions
            .contains(&ManagedAuthConnectionAction::Refresh));
        assert!(!summary
            .allowed_actions
            .contains(&ManagedAuthConnectionAction::SwitchToOfficial));
    }

    #[test]
    fn missing_model_provider_is_official() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), "model = \"gpt-5\"\n").unwrap();
        let observed = observe_codex_home(dir.path());
        assert_eq!(observed.provider_route, CodexProviderRoute::Official);
        assert_eq!(
            observed.request_mode,
            crate::services::managed_auth::ManagedAuthRequestMode::OfficialSubscription
        );
    }

    #[test]
    fn third_party_preserves_official_session_flag_shape() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "model_provider = \"deepseek\"\ncli_auth_credentials_store = \"file\"\n",
        )
        .unwrap();
        let observed = observe_codex_home(dir.path());
        let summary = connection_summary(&observed, None, None, &[], now_timestamp());
        assert_eq!(
            summary.request_mode,
            crate::services::managed_auth::ManagedAuthRequestMode::ThirdPartyApi
        );
        let text = serde_json::to_string(&summary)
            .unwrap()
            .to_ascii_lowercase();
        assert!(!text.contains("auth.json"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn keyring_store_marks_projection_unavailable() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "cli_auth_credentials_store = \"keyring\"\n",
        )
        .unwrap();
        let observed = observe_codex_home(dir.path());
        let summary = connection_summary(&observed, None, None, &[], now_timestamp());
        assert!(summary
            .reason_codes
            .contains(&ManagedAuthReasonCode::NativeProjectionUnavailable));
        assert_eq!(
            summary.credential_manager,
            ManagedAuthCredentialManager::Unavailable
        );
    }
}
