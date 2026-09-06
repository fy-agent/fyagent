//! Codex account projection. This owner never writes model configuration.

use std::path::Path;

use crate::app_config::AppType;
use crate::services::managed_auth::{
    ManagedAuthMutationOutcome, ManagedAuthReasonCode, ManagedAuthSecretBundle,
};
use crate::store::AppState;

use super::auth_document::CodexChatGptAuthDocument;
use super::delta::{plan_codex_managed_auth_delta, CodexDeltaError, CodexManagedAuthDelta};
use super::observation::{document_from_bundle, observe_managed_auth};
use super::swap::{auth_path_in, swap_codex_chatgpt_auth, CodexAuthSwapError};

#[derive(Debug, Clone)]
pub(crate) struct CodexProjectionOutcome {
    pub outcome: ManagedAuthMutationOutcome,
    pub reason: Option<ManagedAuthReasonCode>,
    pub pending_restart: bool,
    pub wrote_auth: bool,
}

/// Serialize with Provider writes, but never invoke a Provider writer from an
/// account operation. The displayed revision covers both auth and the store/
/// routing configuration used to decide whether file projection is supported.
pub(crate) fn project_codex_official_account(
    app_state: &AppState,
    codex_home: &Path,
    target_provider_subject: &str,
    target_document: &CodexChatGptAuthDocument,
    expected_connection_revision: &str,
) -> Result<CodexProjectionOutcome, ManagedAuthReasonCode> {
    let _guard = futures::executor::block_on(
        app_state
            .proxy_service
            .lock_switch_for_app(AppType::Codex.as_str()),
    );
    project_under_guard(
        codex_home,
        target_provider_subject,
        target_document,
        expected_connection_revision,
    )
}

fn project_under_guard(
    codex_home: &Path,
    target_provider_subject: &str,
    target_document: &CodexChatGptAuthDocument,
    expected_connection_revision: &str,
) -> Result<CodexProjectionOutcome, ManagedAuthReasonCode> {
    if !target_document.identity_matches(target_provider_subject) {
        return Err(ManagedAuthReasonCode::IdentityMismatch);
    }
    let live = observe_managed_auth(codex_home);
    if live.connection_revision() != expected_connection_revision {
        return Err(ManagedAuthReasonCode::ExternalChangeDetected);
    }
    let delta = plan_codex_managed_auth_delta(&live, target_provider_subject)
        .map_err(CodexDeltaError::reason_code)?;
    if delta == CodexManagedAuthDelta::Noop {
        return Ok(CodexProjectionOutcome {
            outcome: ManagedAuthMutationOutcome::Completed,
            reason: None,
            pending_restart: false,
            wrote_auth: false,
        });
    }
    // The shared writer retains the exact old auth file even when it contains
    // only a legacy API key. No hidden Provider backfill or config write is
    // needed to make that credential recoverable.
    let receipt = swap_codex_chatgpt_auth(
        &auth_path_in(codex_home),
        live.auth_revision.as_deref(),
        target_document,
    )
    .map_err(|error| match error {
        CodexAuthSwapError::Stale | CodexAuthSwapError::ExternalChange => {
            ManagedAuthReasonCode::ExternalChangeDetected
        }
        CodexAuthSwapError::IdentityMismatch => ManagedAuthReasonCode::IdentityMismatch,
        CodexAuthSwapError::Invalid | CodexAuthSwapError::Io => {
            ManagedAuthReasonCode::PartialCompletion
        }
    })?;
    Ok(CodexProjectionOutcome {
        outcome: ManagedAuthMutationOutcome::Completed,
        reason: receipt
            .pending_restart
            .then_some(ManagedAuthReasonCode::PendingRestart),
        pending_restart: receipt.pending_restart,
        wrote_auth: receipt.changed,
    })
}

pub(crate) fn materialize_from_bundle(
    bundle: &ManagedAuthSecretBundle,
    expected_subject: &str,
) -> Result<CodexChatGptAuthDocument, ManagedAuthReasonCode> {
    document_from_bundle(bundle, expected_subject).ok_or(ManagedAuthReasonCode::RequiresReauth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_json::json;

    fn account(id: &str) -> CodexChatGptAuthDocument {
        let payload = URL_SAFE_NO_PAD
            .encode(json!({"chatgpt_account_id":id,"email":"test@example.com"}).to_string());
        let token = format!("e30.{payload}.sig");
        CodexChatGptAuthDocument::from_tokens(
            &token,
            &token,
            "fixture-refresh",
            Some(id),
            Some(1_700_000_000),
        )
        .unwrap()
    }

    #[test]
    fn account_switch_preserves_commented_custom_config_without_provider_rows() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.toml");
        let original = b"# user configuration\r\nmodel_provider = 'custom'\r\nmodel = 'user-model'\r\n[model_providers.custom]\r\nname = 'User provider'\r\nbase_url = 'https://example.test/v1'\r\nwire_api = 'responses'\r\n[mcp_servers.example]\r\ncommand = 'keep-me'\r\n[features]\r\nmulti_agent = true\r\n";
        std::fs::write(&config, original).unwrap();
        let auth = auth_path_in(dir.path());
        std::fs::write(&auth, br#"{ "OPENAI_API_KEY": "old-fixture-key" }"#).unwrap();
        let before_auth = std::fs::read(&auth).unwrap();
        let revision = observe_managed_auth(dir.path()).connection_revision();
        let result =
            project_under_guard(dir.path(), "acct-a", &account("acct-a"), &revision).unwrap();
        assert!(result.wrote_auth && result.pending_restart);
        assert_eq!(std::fs::read(&config).unwrap(), original);
        assert!(!crate::config::rolling_backup_path(&config).exists());
        assert_eq!(
            std::fs::read(crate::config::rolling_backup_path(&auth)).unwrap(),
            before_auth
        );
        assert!(!observe_managed_auth(dir.path())
            .provider_route
            .is_official());
    }

    #[test]
    fn stale_auth_or_config_snapshot_rejects_without_writes() {
        for change_auth in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let revision = observe_managed_auth(dir.path()).connection_revision();
            let path = dir.path().join(if change_auth {
                "auth.json"
            } else {
                "config.toml"
            });
            let bytes: &[u8] = if change_auth {
                br#"{"OPENAI_API_KEY":"external"}"#
            } else {
                b"# external configuration\nmodel = 'external'\n"
            };
            std::fs::write(&path, bytes).unwrap();
            let result = project_under_guard(dir.path(), "acct-a", &account("acct-a"), &revision);
            assert_eq!(
                result.unwrap_err(),
                ManagedAuthReasonCode::ExternalChangeDetected
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert!(!crate::config::rolling_backup_path(&path).exists());
            if !change_auth {
                assert!(!auth_path_in(dir.path()).exists());
            }
        }
    }

    #[test]
    fn existing_matching_account_never_changes_route_or_rotates_auth_backup() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.toml");
        let config_bytes = b"# keep\nmodel_provider = 'custom'\n";
        std::fs::write(&config, config_bytes).unwrap();
        let auth = auth_path_in(dir.path());
        let document = account("acct-a");
        std::fs::write(&auth, document.serialize_bytes().unwrap()).unwrap();
        let before = std::fs::read(&auth).unwrap();
        let revision = observe_managed_auth(dir.path()).connection_revision();
        let result = project_under_guard(dir.path(), "acct-a", &document, &revision).unwrap();
        assert!(!result.wrote_auth);
        assert_eq!(std::fs::read(&auth).unwrap(), before);
        assert_eq!(std::fs::read(&config).unwrap(), config_bytes);
        assert!(!crate::config::rolling_backup_path(&auth).exists());
    }
}
