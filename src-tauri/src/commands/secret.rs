use crate::secret::{
    BeginSecretCaptureRequest, CheckSecretApplyReadinessRequest, DeleteSecretRequest,
    DiscardSecretCandidateRequest, DiscardSecretCandidateResult, GetSecretCleanupImpactRequest,
    GetSecretDeleteImpactRequest, ListSecretAuditRequest, ListSecretBackendOptionsRequest,
    ListSecretBackendOptionsResult, ListSecretCandidatesRequest, ListSecretCandidatesResult,
    ListSecretSummariesRequest, ListSecretSummariesResult, MigrateLegacyCodexSecretsRequest,
    ResumeStagedImportCutoverRequest, ResumeStagedImportCutoverResultDto,
    RetrySecretCleanupRequest, RotateSecretRequest, SecretApplyReadiness, SecretAuditPage,
    SecretCommandResult, SecretDeleteImpact, SecretDeleteResult,
    SecretMigrationReport, SecretMutationResult, SecretRecoveryImpact, SecretRecoveryResult,
    SecretValidationResult, SetSecretLockedRequest, StageSecretCandidateResult,
    ValidateSecretRequest,
};
use crate::store::AppState;
use tauri::State;

fn unavailable<T>() -> SecretCommandResult<T> {
    crate::secret::command_unavailable()
}

fn require_opened_store(
    state: &AppState,
) -> Result<&crate::secret::OpenedDeviceLocalSecretStore, crate::secret::SecretCommandError> {
    state
        .secret_store
        .as_ref()
        .ok_or_else(crate::secret::command_unavailable_error)
}

/// Sync helper so tests can exercise fail-closed / load without spinning Tauri.
///
/// Contract DTO success mapping is unpublished: `ListSecretSummariesResult`
/// needs `SecretOwnerCredentialSummary` / `LegacySourceCoverageView` (coverage
/// receipt + `todo!` constructor) and `SecretRefAggregate::checked_from_authority`
/// (`todo!`). After a successful `load()` this still returns `unavailable()`.
pub(crate) fn list_secret_summaries_from_state(
    state: &AppState,
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    let opened = require_opened_store(state)?;
    let _payload = opened
        .store()
        .load()
        .map_err(crate::secret::service_command_error)?;
    let _ = request;
    unavailable()
}

/// Same fail-closed + load, then stop: `ListSecretCandidatesResult` needs
/// `SecretCandidateActivationProjection::validate_repr` (`todo!`).
pub(crate) fn list_secret_candidates_from_state(
    state: &AppState,
    request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult> {
    let opened = require_opened_store(state)?;
    let _payload = opened
        .store()
        .load()
        .map_err(crate::secret::service_command_error)?;
    let _ = request;
    unavailable()
}

/// Fail-closed when the store is missing. Production discard would have to
/// pretend a Keychain delete (`InMemorySecretBackend` is test-only), and
/// `DiscardSecretCandidateResult::checked_from_candidate_journal` is `todo!`.
pub(crate) fn discard_secret_candidate_from_state(
    state: &AppState,
    request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult> {
    let opened = require_opened_store(state)?;
    let _payload = opened
        .store()
        .load()
        .map_err(crate::secret::service_command_error)?;
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn list_secret_summaries(
    state: State<'_, AppState>,
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    list_secret_summaries_from_state(&state, request)
}

#[tauri::command]
pub async fn list_secret_backend_options(
    request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn begin_secret_capture(
    request: BeginSecretCaptureRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn rotate_secret(
    request: RotateSecretRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn list_secret_candidates(
    state: State<'_, AppState>,
    request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult> {
    list_secret_candidates_from_state(&state, request)
}

#[tauri::command]
pub async fn discard_secret_candidate(
    state: State<'_, AppState>,
    request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult> {
    discard_secret_candidate_from_state(&state, request)
}

#[tauri::command]
pub async fn set_secret_locked(
    request: SetSecretLockedRequest,
) -> SecretCommandResult<SecretMutationResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn get_secret_delete_impact(
    request: GetSecretDeleteImpactRequest,
) -> SecretCommandResult<SecretDeleteImpact> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn delete_secret(
    request: DeleteSecretRequest,
) -> SecretCommandResult<SecretDeleteResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn get_secret_cleanup_impact(
    request: GetSecretCleanupImpactRequest,
) -> SecretCommandResult<SecretRecoveryImpact> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn retry_secret_cleanup(
    request: RetrySecretCleanupRequest,
) -> SecretCommandResult<SecretRecoveryResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn validate_secret(
    request: ValidateSecretRequest,
) -> SecretCommandResult<SecretValidationResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn check_secret_apply_readiness(
    request: CheckSecretApplyReadinessRequest,
) -> SecretCommandResult<SecretApplyReadiness> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn migrate_legacy_codex_secrets(
    request: MigrateLegacyCodexSecretsRequest,
) -> SecretCommandResult<SecretMigrationReport> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn list_secret_audit(
    request: ListSecretAuditRequest,
) -> SecretCommandResult<SecretAuditPage> {
    let _ = request;
    unavailable()
}

/// Main-integration resume handler (`SecretMainIntegrationCommandName`).
/// Not a `SecretCommandName`. `MainIntegrationCommandResult` is not in the
/// crate yet, so this stub uses the existing `SecretCommandResult` envelope.
#[tauri::command]
pub async fn resume_staged_import_cutover(
    request: ResumeStagedImportCutoverRequest,
) -> SecretCommandResult<ResumeStagedImportCutoverResultDto> {
    let _ = request;
    unavailable()
}

#[cfg(test)]
mod secret_command_dto_tests {
    use super::*;
    use crate::database::Database;
    use crate::secret::{
        list_secret_candidates_from_store, list_secret_summaries_from_store,
        seed_opened_store_pending_candidate, SecretBootstrap,
    };
    use crate::store::AppState;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn reject_unknown<T: serde::de::DeserializeOwned>(json: &str) {
        let err = match serde_json::from_str::<T>(json) {
            Ok(_) => panic!("unknown field must fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field") || msg.contains("did you mean"),
            "{msg}"
        );
    }

    fn summaries_request() -> ListSecretSummariesRequest {
        serde_json::from_str(r#"{"schemaVersion":1,"includeUnboundOwners":true,"limit":10}"#)
            .expect("summaries request")
    }

    fn candidates_request() -> ListSecretCandidatesRequest {
        serde_json::from_str(r#"{"schemaVersion":1,"includeTerminal":false}"#)
            .expect("candidates request")
    }

    fn discard_request() -> DiscardSecretCandidateRequest {
        use crate::secret::SecretCandidateId;
        let candidate_id = SecretCandidateId::generate();
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"candidateId":"{}","expectedCandidateRevision":1}}"#,
            candidate_id.as_str()
        ))
        .expect("discard request")
    }

    fn assert_err_not_empty_ok<T>(result: SecretCommandResult<T>) {
        assert!(result.is_err(), "missing/unmapped store must not succeed");
    }

    #[test]
    fn secret_list_summaries_request_deny_unknown_fields() {
        reject_unknown::<ListSecretSummariesRequest>(
            r#"{"schemaVersion":1,"includeUnboundOwners":false,"limit":10,"unexpected":true}"#,
        );
    }

    #[test]
    fn secret_begin_capture_request_deny_unknown_fields() {
        use crate::secret::{SecretBackendInstanceId, SecretCaptureIntentId};
        let id = SecretCaptureIntentId::generate();
        let backend = SecretBackendInstanceId::generate();
        let json = format!(
            r#"{{"schemaVersion":1,"captureIntentId":"{}","backendInstanceId":"{}","extra":1}}"#,
            id.as_str(),
            backend.as_str()
        );
        reject_unknown::<BeginSecretCaptureRequest>(&json);
    }

    #[test]
    fn secret_validate_request_deny_unknown_fields() {
        use crate::secret::SecretRef;
        let secret_ref = SecretRef::generate();
        let json = format!(
            r#"{{"schemaVersion":1,"secretRef":"{}","expectedRecordRevision":1,"nope":false}}"#,
            secret_ref.as_str()
        );
        reject_unknown::<ValidateSecretRequest>(&json);
    }

    #[test]
    fn secret_delete_request_deny_unknown_fields() {
        use crate::secret::{SecretOperationId, SecretRef};
        let secret_ref = SecretRef::generate();
        let op = SecretOperationId::generate();
        let json = format!(
            r#"{{"schemaVersion":1,"operationId":"{}","secretRef":"{}","expectedRecordRevision":1,"expectedBindingSet":{{"revision":1,"digest":"{}","count":1}},"unknown":true}}"#,
            op.as_str(),
            secret_ref.as_str(),
            "a".repeat(64),
        );
        reject_unknown::<DeleteSecretRequest>(&json);
    }

    #[test]
    fn secret_list_summaries_request_accepts_known_fields() {
        let parsed: ListSecretSummariesRequest = serde_json::from_str(
            r#"{"schemaVersion":1,"includeUnboundOwners":false,"limit":10}"#,
        )
        .expect("known fields must parse");
        let _ = parsed;
    }

    #[test]
    fn secret_app_state_new_without_store_fails_closed_for_local_commands() {
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new(db);
        assert!(state.secret_store.is_none());
        assert_err_not_empty_ok(list_secret_summaries_from_state(&state, summaries_request()));
        assert_err_not_empty_ok(list_secret_candidates_from_state(&state, candidates_request()));
        assert_err_not_empty_ok(discard_secret_candidate_from_state(&state, discard_request()));
    }

    #[test]
    fn secret_opened_store_seed_lists_local_projection_and_contract_dto_stays_unavailable() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store())
            .expect("seed on the same opened store");

        let summaries = list_secret_summaries_from_store(opened.store(), None, false)
            .expect("local summaries");
        assert!(
            summaries
                .refs
                .iter()
                .any(|row| row.secret_ref == seeded.secret_ref),
            "seeded secret_ref must appear in the local projection"
        );
        assert!(
            summaries.owners.iter().any(|owner| {
                owner.secret_ref.as_deref() == Some(seeded.secret_ref.as_str())
            }),
            "seeded owner/ref must appear in the local projection"
        );

        let candidates = list_secret_candidates_from_store(opened.store(), false)
            .expect("local candidates");
        assert!(
            candidates
                .iter()
                .any(|row| row.candidate_id == seeded.candidate_id),
            "seeded candidate must appear in the local projection"
        );

        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert!(state.secret_store.is_some());
        // load() succeeded above; contract DTO mapping is unpublished.
        assert_err_not_empty_ok(list_secret_summaries_from_state(&state, summaries_request()));
        assert_err_not_empty_ok(list_secret_candidates_from_state(&state, candidates_request()));
        assert_err_not_empty_ok(discard_secret_candidate_from_state(&state, discard_request()));
    }
}
