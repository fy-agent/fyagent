//! Codex official-account projection coordinator.

use std::path::Path;

use crate::app_config::AppType;
use crate::database::CODEX_OFFICIAL_PROVIDER_ID;
use crate::services::managed_auth::{
    ManagedAuthMutationOutcome, ManagedAuthReasonCode, ManagedAuthSecretBundle,
};
use crate::services::provider::ProviderService;
use crate::store::AppState;

use super::auth_document::{CodexChatGptAuthDocument, CodexNativeAuthState};
use super::delta::{plan_codex_managed_auth_delta, CodexDeltaError, CodexManagedAuthDelta};
use super::observation::{document_from_bundle, observe_managed_auth};
use super::swap::{
    auth_path_in, capture_auth_preimage, restore_codex_auth_preimage, swap_codex_chatgpt_auth,
    CodexAuthSwapError, CodexAuthSwapReceipt,
};

#[derive(Debug, Clone)]
pub(crate) struct CodexProjectionOutcome {
    pub outcome: ManagedAuthMutationOutcome,
    pub reason: Option<ManagedAuthReasonCode>,
    pub pending_restart: bool,
    #[allow(dead_code)]
    pub auth_revision: Option<String>,
    pub wrote_auth: bool,
    pub switched_provider: bool,
}

impl CodexProjectionOutcome {
    fn noop() -> Self {
        Self {
            outcome: ManagedAuthMutationOutcome::Completed,
            reason: None,
            pending_restart: false,
            auth_revision: None,
            wrote_auth: false,
            switched_provider: false,
        }
    }
}

/// Project a selected official account into Codex using the minimum delta.
///
/// Caller must already hold any credential locks required for materialization.
/// This function acquires the Codex Provider mutation guard for all paths,
/// including AuthOnly, so auth swaps serialize with Provider switches.
pub(crate) fn project_codex_official_account(
    app_state: &AppState,
    codex_home: &Path,
    target_provider_subject: &str,
    target_document: &CodexChatGptAuthDocument,
    expected_auth_revision: Option<&str>,
) -> Result<CodexProjectionOutcome, ManagedAuthReasonCode> {
    if !target_document.identity_matches(target_provider_subject) {
        return Err(ManagedAuthReasonCode::IdentityMismatch);
    }

    let _guard = futures::executor::block_on(
        app_state
            .proxy_service
            .lock_switch_for_app(AppType::Codex.as_str()),
    );

    let live = observe_managed_auth(codex_home);
    let delta = plan_codex_managed_auth_delta(&live, target_provider_subject)
        .map_err(CodexDeltaError::reason_code)?;

    match delta {
        CodexManagedAuthDelta::Noop => Ok(CodexProjectionOutcome {
            auth_revision: live.auth_revision,
            ..CodexProjectionOutcome::noop()
        }),
        CodexManagedAuthDelta::AuthOnly => {
            prepare_before_auth_overwrite(app_state, &live.auth_state, false)?;
            let expected = expected_auth_revision.or(live.auth_revision.as_deref());
            let receipt = swap_auth(codex_home, expected, target_document)?;
            Ok(CodexProjectionOutcome {
                outcome: ManagedAuthMutationOutcome::Completed,
                reason: receipt
                    .pending_restart
                    .then_some(ManagedAuthReasonCode::PendingRestart),
                pending_restart: receipt.pending_restart,
                auth_revision: Some(receipt.revision),
                wrote_auth: receipt.changed,
                switched_provider: false,
            })
        }
        CodexManagedAuthDelta::ProviderOnly => {
            switch_official(app_state)?;
            let after = observe_managed_auth(codex_home);
            if !after.provider_route.is_official() {
                return Err(ManagedAuthReasonCode::PartialCompletion);
            }
            Ok(CodexProjectionOutcome {
                outcome: ManagedAuthMutationOutcome::Completed,
                reason: None,
                pending_restart: false,
                auth_revision: after.auth_revision,
                wrote_auth: false,
                switched_provider: true,
            })
        }
        CodexManagedAuthDelta::AuthThenProvider => {
            prepare_before_auth_overwrite(app_state, &live.auth_state, true)?;

            let auth_path = auth_path_in(codex_home);
            let preimage = capture_auth_preimage(&auth_path)
                .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?;
            let expected = expected_auth_revision.or(live.auth_revision.as_deref());
            let receipt = swap_auth(codex_home, expected, target_document)?;
            match switch_official(app_state) {
                Ok(()) => {
                    let after = observe_managed_auth(codex_home);
                    if !after.provider_route.is_official() {
                        return Ok(CodexProjectionOutcome {
                            outcome: ManagedAuthMutationOutcome::Partial,
                            reason: Some(ManagedAuthReasonCode::PartialCompletion),
                            pending_restart: receipt.pending_restart,
                            auth_revision: Some(receipt.revision),
                            wrote_auth: receipt.changed,
                            switched_provider: false,
                        });
                    }
                    Ok(CodexProjectionOutcome {
                        outcome: ManagedAuthMutationOutcome::Completed,
                        reason: receipt
                            .pending_restart
                            .then_some(ManagedAuthReasonCode::PendingRestart),
                        pending_restart: receipt.pending_restart,
                        auth_revision: Some(receipt.revision),
                        wrote_auth: receipt.changed,
                        switched_provider: true,
                    })
                }
                Err(reason) => {
                    // Restore auth preimage only when route is still third-party
                    // and auth revision is still ours.
                    let after = observe_managed_auth(codex_home);
                    if after.provider_route.is_official() {
                        return Ok(CodexProjectionOutcome {
                            outcome: ManagedAuthMutationOutcome::Partial,
                            reason: Some(ManagedAuthReasonCode::PartialCompletion),
                            pending_restart: receipt.pending_restart,
                            auth_revision: Some(receipt.revision),
                            wrote_auth: receipt.changed,
                            switched_provider: true,
                        });
                    }
                    if after.auth_revision.as_deref() == Some(receipt.revision.as_str()) {
                        let _ = restore_codex_auth_preimage(
                            &auth_path,
                            &receipt.revision,
                            preimage.as_deref(),
                        );
                    }
                    Err(reason)
                }
            }
        }
    }
}

/// Before covering live auth, optionally backfill the current Provider row.
/// Legacy API-key-only auth must prove the key is recoverable first.
fn prepare_before_auth_overwrite(
    app_state: &AppState,
    auth_state: &CodexNativeAuthState,
    always_backfill: bool,
) -> Result<(), ManagedAuthReasonCode> {
    let api_key_only = matches!(
        auth_state,
        CodexNativeAuthState::ThirdPartyApiKeyOnly { .. }
    );
    if !always_backfill && !api_key_only {
        return Ok(());
    }
    let backfilled = ProviderService::backfill_current_live_under_lock(app_state, AppType::Codex)
        .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?;
    if !api_key_only {
        return Ok(());
    }
    if !backfilled {
        return Err(ManagedAuthReasonCode::PartialCompletion);
    }
    let providers = app_state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?;
    let current_id =
        crate::settings::get_effective_current_provider(&app_state.db, &AppType::Codex)
            .map_err(|_| ManagedAuthReasonCode::PartialCompletion)?
            .ok_or(ManagedAuthReasonCode::PartialCompletion)?;
    let Some(current) = providers.get(&current_id) else {
        return Err(ManagedAuthReasonCode::PartialCompletion);
    };
    let has_key = current
        .settings_config
        .pointer("/auth/OPENAI_API_KEY")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());
    if !has_key {
        return Err(ManagedAuthReasonCode::PartialCompletion);
    }
    Ok(())
}

fn swap_auth(
    codex_home: &Path,
    expected: Option<&str>,
    target: &CodexChatGptAuthDocument,
) -> Result<CodexAuthSwapReceipt, ManagedAuthReasonCode> {
    swap_codex_chatgpt_auth(&auth_path_in(codex_home), expected, target).map_err(
        |error| match error {
            CodexAuthSwapError::Stale | CodexAuthSwapError::ExternalChange => {
                ManagedAuthReasonCode::ExternalChangeDetected
            }
            CodexAuthSwapError::IdentityMismatch => ManagedAuthReasonCode::IdentityMismatch,
            CodexAuthSwapError::Invalid | CodexAuthSwapError::Io => {
                ManagedAuthReasonCode::PartialCompletion
            }
        },
    )
}

fn switch_official(app_state: &AppState) -> Result<(), ManagedAuthReasonCode> {
    ProviderService::switch_with_lock_held_skipping_backfill(
        app_state,
        AppType::Codex,
        CODEX_OFFICIAL_PROVIDER_ID,
    )
    .map(|_| ())
    .map_err(|_| ManagedAuthReasonCode::PartialCompletion)
}

/// Materialize a projection document from a complete bundle, or signal reauth.
pub(crate) fn materialize_from_bundle(
    bundle: &ManagedAuthSecretBundle,
    expected_subject: &str,
) -> Result<CodexChatGptAuthDocument, ManagedAuthReasonCode> {
    document_from_bundle(bundle, expected_subject).ok_or(ManagedAuthReasonCode::RequiresReauth)
}
