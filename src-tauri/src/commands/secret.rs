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
/// After `load()`, owners/refs are mapped through
/// `LegacySourceCoverageView::checked_from_coverage_receipt` and
/// `SecretRefAggregate::checked_from_authority` into `ListSecretSummariesResult`.
/// Constructor failure fail-closes (`Err`), never `Ok(empty)`.
pub(crate) fn list_secret_summaries_from_state(
    state: &AppState,
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    let opened = require_opened_store(state)?;
    crate::secret::list_secret_summaries_result_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

/// Same fail-closed + load. A non-empty candidate list mints
/// `SecretCandidateActivationProjection` from D2 store fields and returns
/// the contract DTO when constructors pass. Constructor mismatch fail-closes.
/// An empty store may return Ok([]).
pub(crate) fn list_secret_candidates_from_state(
    state: &AppState,
    request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult> {
    let opened = require_opened_store(state)?;
    crate::secret::list_secret_candidates_result_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

/// Fail-closed when the store is missing. Success requires a recognized
/// InMemory test-double hold; production AppState has none and returns
/// typed unavailable (no Keychain delete).
pub(crate) fn discard_secret_candidate_from_state(
    state: &AppState,
    request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult> {
    let opened = require_opened_store(state)?;
    #[cfg(test)]
    let backend = state.secret_in_memory_backend.as_ref();
    #[cfg(not(test))]
    let backend = None;
    if backend.is_none() {
        return unavailable();
    }
    crate::secret::discard_secret_candidate_result_from_store(opened.store(), &request, backend)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

#[tauri::command]
pub async fn list_secret_summaries(
    state: State<'_, AppState>,
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    list_secret_summaries_from_state(&state, request)
}

/// Sync helper so tests can exercise fail-closed / InMemory mint without
/// spinning Tauri. Production AppState has no InMemory/registry hold and
/// returns typed unavailable (no Keychain write). A successful InMemory mint
/// still fail-closes: `ListSecretBackendOptionsResult::checked_from_registry`
/// stays unimplemented, so this never returns Ok(contract DTO).
pub(crate) fn list_secret_backend_options_from_state(
    state: &AppState,
    request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult> {
    let opened = require_opened_store(state)?;
    #[cfg(test)]
    {
        let _ = opened;
        return list_secret_backend_options_in_memory(state, request);
    }
    #[cfg(not(test))]
    {
        let _ = (opened, request);
        unavailable()
    }
}

#[cfg(test)]
fn list_secret_backend_options_in_memory(
    state: &AppState,
    request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult> {
    let Some(_backend) = state.secret_in_memory_backend.as_ref() else {
        return unavailable();
    };
    let Some(registry) = state.secret_capture_registry.as_ref() else {
        return unavailable();
    };
    let backend_id = crate::secret::SecretBackendInstanceId::generate();
    match registry.mint(
        request.owner.owner_id.as_str(),
        request.purpose,
        request.intent,
        backend_id,
    ) {
        // Minted into the test registry, but the contract DTO constructor is still todo.
        Ok(_) => unavailable(),
        Err(error) => Err(crate::secret::service_command_error(error)),
    }
}

#[tauri::command]
pub async fn list_secret_backend_options(
    state: State<'_, AppState>,
    request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult> {
    list_secret_backend_options_from_state(&state, request)
}

/// Sync helper so tests can exercise fail-closed / InMemory stage without
/// spinning Tauri. Production AppState has no InMemory hold and returns
/// typed unavailable (no Keychain write). A successful InMemory stage returns
/// the contract DTO when `StageSecretCandidateResult::checked_from_candidate_snapshot`
/// passes; constructor mismatch stays Err.
pub(crate) fn begin_secret_capture_from_state(
    state: &AppState,
    request: BeginSecretCaptureRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    let opened = require_opened_store(state)?;
    #[cfg(test)]
    {
        return begin_secret_capture_in_memory(
            state,
            opened,
            request,
            &crate::secret::capture::ProgrammaticCapturePrompt::new(
                b"capture-success-key".to_vec(),
            ),
        );
    }
    #[cfg(not(test))]
    {
        let _ = (opened, request);
        unavailable()
    }
}

#[cfg(test)]
fn begin_secret_capture_in_memory(
    state: &AppState,
    opened: &crate::secret::OpenedDeviceLocalSecretStore,
    request: BeginSecretCaptureRequest,
    prompt: &dyn crate::secret::capture::CapturePrompt,
) -> SecretCommandResult<StageSecretCandidateResult> {
    let Some(backend) = state.secret_in_memory_backend.as_ref() else {
        return unavailable();
    };
    let Some(registry) = state.secret_capture_registry.as_ref() else {
        return unavailable();
    };
    let claim = match registry.claim_once(&request.capture_intent_id, &request.backend_instance_id)
    {
        Ok(claim) => claim,
        Err(error) => return Err(crate::secret::service_command_error(error)),
    };
    let capture = crate::secret::capture::LocalSecretCapture::new(
        opened.store(),
        crate::secret::capture::CaptureLeafBackend::InMemory(backend),
        registry,
        prompt,
    );
    let owner_id = claim.owner().to_string();
    match capture.begin_after_claim(claim) {
        Ok(staged) => {
            let owner = crate::secret::SecretOwner {
                kind: crate::secret::SecretOwnerKind::Provider,
                namespace: crate::secret::SecretOwnerNamespace::parse("codex".to_string())
                    .map_err(|_| crate::secret::command_unavailable_error())?,
                owner_id: crate::secret::OwnerId::parse(owner_id)
                    .map_err(|_| crate::secret::command_unavailable_error())?,
                slot: crate::secret::SecretSlot::PrimaryApiKey,
            };
            crate::secret::stage_secret_candidate_result_from_store(
                opened.store(),
                &staged.candidate_id,
                owner,
            )
            .map(crate::secret::command_success)
            .map_err(crate::secret::service_command_error)
        }
        Err(error) => Err(crate::secret::service_command_error(error)),
    }
}

#[tauri::command]
pub async fn begin_secret_capture(
    state: State<'_, AppState>,
    request: BeginSecretCaptureRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    begin_secret_capture_from_state(&state, request)
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

/// Sync helper so tests can exercise fail-closed / contract DTO without
/// spinning Tauri. Writes only D2 policy_state; never touches Keychain.
pub(crate) fn set_secret_locked_from_state(
    state: &AppState,
    request: SetSecretLockedRequest,
) -> SecretCommandResult<SecretMutationResult> {
    let opened = require_opened_store(state)?;
    crate::secret::set_secret_locked_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

#[tauri::command]
pub async fn set_secret_locked(
    state: State<'_, AppState>,
    request: SetSecretLockedRequest,
) -> SecretCommandResult<SecretMutationResult> {
    set_secret_locked_from_state(&state, request)
}

/// Read-only impact. Never writes or deletes Keychain.
pub(crate) fn get_secret_delete_impact_from_state(
    state: &AppState,
    request: GetSecretDeleteImpactRequest,
) -> SecretCommandResult<SecretDeleteImpact> {
    let opened = require_opened_store(state)?;
    crate::secret::get_secret_delete_impact_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

#[tauri::command]
pub async fn get_secret_delete_impact(
    state: State<'_, AppState>,
    request: GetSecretDeleteImpactRequest,
) -> SecretCommandResult<SecretDeleteImpact> {
    get_secret_delete_impact_from_state(&state, request)
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

/// Read-only validate. Never writes or deletes Keychain.
pub(crate) fn validate_secret_from_state(
    state: &AppState,
    request: ValidateSecretRequest,
) -> SecretCommandResult<SecretValidationResult> {
    let opened = require_opened_store(state)?;
    crate::secret::validate_secret_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

#[tauri::command]
pub async fn validate_secret(
    state: State<'_, AppState>,
    request: ValidateSecretRequest,
) -> SecretCommandResult<SecretValidationResult> {
    validate_secret_from_state(&state, request)
}

/// Sync helper so tests can exercise fail-closed / contract DTO without
/// spinning Tauri. Production still never writes Keychain or calls
/// resolve_for_apply; this only mints SecretApplyReadiness when D2 store
/// fields pass the existing constructors.
pub(crate) fn check_secret_apply_readiness_from_state(
    state: &AppState,
    request: CheckSecretApplyReadinessRequest,
) -> SecretCommandResult<SecretApplyReadiness> {
    let opened = require_opened_store(state)?;
    crate::secret::check_secret_apply_readiness_from_store(opened.store(), &request)
        .map(crate::secret::command_success)
        .map_err(crate::secret::service_command_error)
}

#[tauri::command]
pub async fn check_secret_apply_readiness(
    state: State<'_, AppState>,
    request: CheckSecretApplyReadinessRequest,
) -> SecretCommandResult<SecretApplyReadiness> {
    check_secret_apply_readiness_from_state(&state, request)
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
        seed_opened_store_pending_candidate, InMemorySecretBackend, SecretBootstrap,
        SecretCandidateId,
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
        let candidate_id = SecretCandidateId::generate();
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"candidateId":"{}","expectedCandidateRevision":1}}"#,
            candidate_id.as_str()
        ))
        .expect("discard request")
    }

    fn discard_request_for(
        candidate_id: &str,
        revision: u64,
    ) -> DiscardSecretCandidateRequest {
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"candidateId":"{}","expectedCandidateRevision":{revision}}}"#,
            candidate_id
        ))
        .expect("discard request")
    }

    fn begin_request() -> BeginSecretCaptureRequest {
        let intent_id = crate::secret::SecretCaptureIntentId::generate();
        let backend_id = crate::secret::SecretBackendInstanceId::generate();
        begin_request_for(intent_id.as_str(), backend_id.as_str())
    }

    fn begin_request_for(intent_id: &str, backend_id: &str) -> BeginSecretCaptureRequest {
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"captureIntentId":"{intent_id}","backendInstanceId":"{backend_id}"}}"#
        ))
        .expect("begin request")
    }

    fn options_request() -> ListSecretBackendOptionsRequest {
        serde_json::from_str(
            r#"{"schemaVersion":1,"owner":{"kind":"provider","namespace":"codex","ownerId":"owner-options-cmd","slot":"primaryApiKey"},"purpose":"codexApiKey","intent":"newBinding"}"#,
        )
        .expect("options request")
    }

    fn local_candidates(state: &AppState) -> Vec<crate::secret::LocalCandidateProjection> {
        list_secret_candidates_from_store(
            state.secret_store.as_ref().expect("store").store(),
            false,
        )
        .expect("local candidates")
    }

    fn mint_begin_intent(
        state: &mut AppState,
    ) -> (crate::secret::SecretCaptureIntentId, crate::secret::SecretBackendInstanceId) {
        let registry = crate::secret::capture::SecretCaptureIntentRegistry::new();
        let backend_id = crate::secret::SecretBackendInstanceId::generate();
        let intent_id = registry
            .mint(
                "owner-begin-cmd",
                crate::secret::SecretPurpose::CodexApiKey,
                crate::secret::BeginCaptureIntent::NewBinding,
                backend_id.clone(),
            )
            .expect("mint");
        state.attach_secret_capture_registry(registry);
        (intent_id, backend_id)
    }

    fn assert_err_not_empty_ok<T>(result: SecretCommandResult<T>) {
        assert!(result.is_err(), "missing/unmapped store must not succeed");
    }

    fn opened_state() -> (TempDir, AppState) {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let db = Arc::new(Database::memory().expect("memory db"));
        (tmp, AppState::new_with_secret_store(db, opened))
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
        assert_err_not_empty_ok(begin_secret_capture_from_state(&state, begin_request()));
        assert_err_not_empty_ok(list_secret_backend_options_from_state(
            &state,
            options_request(),
        ));
        assert_err_not_empty_ok(check_secret_apply_readiness_from_state(
            &state,
            apply_target_request("owner-missing"),
        ));
    }

    #[test]
    fn secret_opened_store_seed_lists_summaries_contract_dto() {
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

        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        let result = list_secret_summaries_from_state(&state, summaries_request())
            .unwrap_or_else(|err| panic!("opened+seeded store maps to contract DTO: {}", serde_json::to_string(&err).unwrap_or_default()));
        let json = serde_json::to_value(&result.data).expect("json");
        let refs = json["refs"].as_array().expect("refs");
        assert!(
            refs.iter()
                .any(|row| row["secretRef"] == seeded.secret_ref),
            "seeded secret_ref must appear in ListSecretSummariesResult"
        );
        let owners = json["owners"].as_array().expect("owners");
        assert!(
            owners.iter().any(|owner| {
                owner["bindingState"]["secretRef"] == seeded.secret_ref
            }),
            "seeded owner/ref must appear in ListSecretSummariesResult"
        );
    }

    #[test]
    fn secret_opened_store_seed_list_candidates_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
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
        let result = list_secret_candidates_from_state(&state, candidates_request())
            .unwrap_or_else(|err| {
                panic!(
                    "seeded candidate must mint activation projection: {}",
                    serde_json::to_string(&err).unwrap_or_default()
                )
            });
        let json = serde_json::to_value(&result.data).expect("json");
        let rows = json["candidates"].as_array().expect("candidates");
        assert!(
            rows.iter().any(|row| {
                row["candidate"]["candidateId"] == seeded.candidate_id
                    && row["activationProjection"].is_object()
            }),
            "list must return contract DTO with activationProjection"
        );
    }

    #[test]
    fn secret_empty_opened_store_lists_empty_contract_dtos() {
        let (_tmp, state) = opened_state();
        let summaries = list_secret_summaries_from_state(&state, summaries_request())
            .unwrap_or_else(|_| panic!("empty opened store is a real empty matrix"));
        let json = serde_json::to_value(&summaries.data).expect("json");
        assert_eq!(json["owners"].as_array().expect("owners").len(), 0);
        assert_eq!(json["refs"].as_array().expect("refs").len(), 0);
        let candidates = list_secret_candidates_from_state(&state, candidates_request())
            .unwrap_or_else(|_| panic!("empty candidate list needs no activation projection"));
        let json = serde_json::to_value(&candidates.data).expect("json");
        assert_eq!(json["candidates"].as_array().expect("candidates").len(), 0);
    }

    #[test]
    fn secret_discard_with_in_memory_backend_returns_success_dto_and_removes_candidate() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let mut state = AppState::new_with_secret_store(db, opened);
        state.attach_in_memory_secret_backend(InMemorySecretBackend::new());
        let result = discard_secret_candidate_from_state(
            &state,
            discard_request_for(&seeded.candidate_id, seeded.candidate_revision),
        )
        .unwrap_or_else(|err| panic!("in-memory two-slot discard: {}", serde_json::to_string(&err).unwrap_or_default()));
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["status"], "discarded");
        assert_eq!(json["candidateId"], seeded.candidate_id);
        assert_eq!(json["terminalState"], "discarded");
        let remaining = list_secret_candidates_from_store(
            state.secret_store.as_ref().expect("store").store(),
            false,
        )
        .expect("local candidates");
        assert!(
            remaining
                .iter()
                .all(|row| row.candidate_id != seeded.candidate_id),
            "discarded candidate must be gone from the non-terminal list"
        );
    }

    #[test]
    fn secret_discard_without_recognized_backend_is_unavailable() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert!(state.secret_in_memory_backend.is_none());
        assert_err_not_empty_ok(discard_secret_candidate_from_state(
            &state,
            discard_request_for(&seeded.candidate_id, seeded.candidate_revision),
        ));
        let remaining = list_secret_candidates_from_store(
            state.secret_store.as_ref().expect("store").store(),
            false,
        )
        .expect("local candidates");
        assert!(
            remaining
                .iter()
                .any(|row| row.candidate_id == seeded.candidate_id),
            "unavailable discard must not pretend a Keychain delete"
        );
    }

    #[test]
    fn secret_list_summaries_constructor_mismatch_fails_closed() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let now = crate::secret::device_store::utc_now();
        let mut payload = opened.store().load().expect("load").payload;
        payload.secrets.push(crate::secret::device_store::schema::StoredSecretRecord {
            secret_ref: "not-a-secret-ref".to_string(),
            purpose: "codexApiKey".to_string(),
            backend_instance_id: "sbi_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaaa".to_string(),
            backend_locator: None,
            record_revision: 1,
            binding_set_cas: crate::secret::device_store::schema::StoredBindingSetCas {
                revision: 1,
                digest: "0".repeat(64),
                count: 0,
            },
            backend_generation: 1,
            device_binding_generation: 1,
            capability_revision: 1,
            policy_state: crate::secret::device_store::schema::StoredPolicyState::Active,
            retirement_state: crate::secret::device_store::schema::StoredRetirementState::Live,
            created_at: now.clone(),
            updated_at: now,
        });
        payload.store_revision = payload.store_revision.saturating_add(1);
        opened.store().store(payload).expect("store bad row");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert_err_not_empty_ok(list_secret_summaries_from_state(&state, summaries_request()));
    }

    #[test]
    fn secret_discard_journal_mismatch_fails_closed() {
        assert!(
            crate::secret::secret_discard_result_from_mismatched_journal_is_err(),
            "constructor/journal mismatch must be Err, not Ok(empty)"
        );
    }

    #[test]
    fn secret_begin_capture_without_in_memory_is_unavailable() {
        let (_tmp, state) = opened_state();
        assert!(state.secret_in_memory_backend.is_none());
        assert_err_not_empty_ok(begin_secret_capture_from_state(&state, begin_request()));
        assert!(
            local_candidates(&state).is_empty(),
            "unavailable begin must not stage a candidate or touch Keychain"
        );
    }

    #[test]
    fn secret_begin_capture_with_in_memory_stages_unbound_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let db = Arc::new(Database::memory().expect("memory db"));
        let mut state = AppState::new_with_secret_store(db, opened);
        state.attach_in_memory_secret_backend(InMemorySecretBackend::new());
        let (intent_id, backend_id) = mint_begin_intent(&mut state);
        let result = begin_secret_capture_from_state(
            &state,
            begin_request_for(intent_id.as_str(), backend_id.as_str()),
        )
        .unwrap_or_else(|err| {
            panic!(
                "staged InMemory capture must Ok contract DTO: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["status"], "staged");
        assert!(json["candidate"].is_object());
        assert!(json["activationProjection"].is_object());
        assert!(json["impact"].is_null());
        let remaining = local_candidates(&state);
        assert_eq!(remaining.len(), 1, "unbound candidate must be staged");
        let payload = state
            .secret_store
            .as_ref()
            .expect("store")
            .store()
            .load()
            .expect("load")
            .payload;
        assert_eq!(payload.candidates.len(), 1);
        assert_eq!(
            payload.candidates[0].state,
            crate::secret::device_store::schema::StoredCandidateState::VerifiedPendingPlan
        );
        assert!(
            payload.owner_bindings.is_empty(),
            "begin capture must not bind an owner"
        );
    }

    #[test]
    fn secret_begin_capture_cancel_prompt_is_zero_write_and_err() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let db = Arc::new(Database::memory().expect("memory db"));
        let mut state = AppState::new_with_secret_store(db, opened);
        state.attach_in_memory_secret_backend(InMemorySecretBackend::new());
        let (intent_id, backend_id) = mint_begin_intent(&mut state);
        let result = begin_secret_capture_in_memory(
            &state,
            state.secret_store.as_ref().expect("store"),
            begin_request_for(intent_id.as_str(), backend_id.as_str()),
            &crate::secret::capture::CancelCapturePrompt,
        );
        assert!(result.is_err(), "cancel must stay fail-closed");
        assert!(
            local_candidates(&state).is_empty(),
            "cancel must not stage a candidate"
        );
        let payload = state
            .secret_store
            .as_ref()
            .expect("store")
            .store()
            .load()
            .expect("load")
            .payload;
        assert!(payload.secrets.is_empty(), "cancel is zero-write");
        assert!(payload.candidates.is_empty(), "cancel is zero-write");
        assert!(payload.owner_bindings.is_empty(), "cancel is zero-write");
    }

    #[test]
    fn secret_list_backend_options_without_in_memory_or_registry_is_unavailable() {
        let (_tmp, mut state) = opened_state();
        state.attach_secret_capture_registry(
            crate::secret::capture::SecretCaptureIntentRegistry::new(),
        );
        assert!(state.secret_in_memory_backend.is_none());
        assert_err_not_empty_ok(list_secret_backend_options_from_state(
            &state,
            options_request(),
        ));
        let registry = state
            .secret_capture_registry
            .as_ref()
            .expect("registry");
        assert!(
            registry.last_minted_id().is_none(),
            "missing InMemory must not mint"
        );
        assert!(
            registry.ready_ids().is_empty(),
            "missing InMemory must not leave a Ready intent"
        );

        let (_tmp, mut state) = opened_state();
        state.attach_in_memory_secret_backend(InMemorySecretBackend::new());
        assert!(state.secret_capture_registry.is_none());
        assert_err_not_empty_ok(list_secret_backend_options_from_state(
            &state,
            options_request(),
        ));

        let (_tmp, state) = opened_state();
        assert!(state.secret_in_memory_backend.is_none());
        assert!(state.secret_capture_registry.is_none());
        assert_err_not_empty_ok(list_secret_backend_options_from_state(
            &state,
            options_request(),
        ));
    }

    #[test]
    fn secret_list_backend_options_mints_intent_but_command_stays_err_then_begin_can_claim() {
        let (_tmp, mut state) = opened_state();
        state.attach_in_memory_secret_backend(InMemorySecretBackend::new());
        state.attach_secret_capture_registry(
            crate::secret::capture::SecretCaptureIntentRegistry::new(),
        );
        let result = list_secret_backend_options_from_state(&state, options_request());
        assert!(
            result.is_err(),
            "minted InMemory options must not Ok a contract DTO"
        );
        let registry = state
            .secret_capture_registry
            .as_ref()
            .expect("registry");
        let intent_id = registry.last_minted_id().expect("minted sci_");
        assert!(
            intent_id.as_str().starts_with("sci_"),
            "options mint must produce a capture intent id"
        );
        let backend_id = registry
            .last_minted_backend_id()
            .expect("minted sbi_");
        assert!(
            backend_id.as_str().starts_with("sbi_"),
            "options mint must record the backend id used for later claim"
        );
        assert!(
            registry
                .ready_ids()
                .iter()
                .any(|id| id == &intent_id),
            "minted intent must stay Ready for begin claim"
        );
        let begin_result = begin_secret_capture_from_state(
            &state,
            begin_request_for(intent_id.as_str(), backend_id.as_str()),
        )
        .unwrap_or_else(|err| {
            panic!(
                "begin after options mint must Ok contract DTO: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&begin_result.data).expect("json");
        assert_eq!(json["status"], "staged");
        assert!(json["activationProjection"].is_object());
        assert_eq!(
            local_candidates(&state).len(),
            1,
            "begin must claim the options-minted sci_ and stage an unbound candidate"
        );
    }

    fn apply_target_request(owner_id: &str) -> CheckSecretApplyReadinessRequest {
        serde_json::from_str(&format!(
            r#"{{"role":"target","schemaVersion":1,"owner":{{"kind":"provider","namespace":"codex","ownerId":"{owner_id}","slot":"primaryApiKey"}},"consumer":"changePlanApply","targetSink":"externalConfigFile","liveSinkId":"codexAuthJsonOpenAiApiKey"}}"#
        ))
        .expect("apply request")
    }

    #[test]
    fn secret_apply_readiness_seed_zero_count_fails_closed() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let owner_id = seeded.owner_id.expect("bound owner");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert_err_not_empty_ok(check_secret_apply_readiness_from_state(
            &state,
            apply_target_request(&owner_id),
        ));
    }

    #[test]
    fn secret_apply_readiness_nonzero_binding_set_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let owner_id = seeded.owner_id.expect("bound owner");
        let mut payload = opened.store().load().expect("load").payload;
        for secret in &mut payload.secrets {
            if secret.secret_ref == seeded.secret_ref {
                secret.binding_set_cas.count = 1;
            }
        }
        payload.store_revision = payload.store_revision.saturating_add(1);
        opened.store().store(payload).expect("store");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        let result = check_secret_apply_readiness_from_state(
            &state,
            apply_target_request(&owner_id),
        )
        .unwrap_or_else(|err| {
            panic!(
                "nonzero binding-set must Ok apply readiness: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["status"], "ready");
        assert!(json["context"]["projection"].is_object());
    }
    fn lock_request(secret_ref: &str, locked: bool, revision: u64) -> SetSecretLockedRequest {
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"secretRef":"{secret_ref}","locked":{locked},"expectedRecordRevision":{revision},"expectedBindingSet":{{"revision":1,"digest":"{digest}","count":0}}}}"#,
            digest = "0".repeat(64),
        ))
        .expect("lock request")
    }

    #[test]
    fn secret_lock_matching_cas_returns_locked_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        let result = set_secret_locked_from_state(
            &state,
            lock_request(&seeded.secret_ref, true, seeded.record_revision),
        )
        .unwrap_or_else(|err| {
            panic!(
                "matching CAS must Ok lock DTO: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["aggregate"]["availability"], "locked");
        assert_eq!(json["aggregate"]["lock"]["source"], "fyAgentPolicy");
        assert!(json["auditEventId"].is_string());
    }

    #[test]
    fn secret_lock_revision_mismatch_fails_closed() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert_err_not_empty_ok(set_secret_locked_from_state(
            &state,
            lock_request(&seeded.secret_ref, true, seeded.record_revision.saturating_add(1)),
        ));
    }

    #[test]
    fn secret_lock_without_store_fails_closed() {
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new(db);
        assert_err_not_empty_ok(set_secret_locked_from_state(
            &state,
            lock_request(crate::secret::SecretRef::generate().as_str(), true, 1),
        ));
    }

    fn delete_impact_request(secret_ref: &str) -> GetSecretDeleteImpactRequest {
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"secretRef":"{secret_ref}"}}"#
        ))
        .expect("delete impact request")
    }

    #[test]
    fn secret_delete_impact_seed_returns_ready_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        let result = get_secret_delete_impact_from_state(
            &state,
            delete_impact_request(&seeded.secret_ref),
        )
        .unwrap_or_else(|err| {
            panic!(
                "seeded ref must Ok delete impact: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["readiness"]["status"], "ready");
        assert_eq!(json["impact"]["secretRef"], seeded.secret_ref);
        assert_eq!(json["impact"]["noFallback"], true);
    }

    #[test]
    fn secret_delete_impact_unknown_ref_fails_closed() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let _ = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert_err_not_empty_ok(get_secret_delete_impact_from_state(
            &state,
            delete_impact_request(crate::secret::SecretRef::generate().as_str()),
        ));
    }

    fn validate_request(secret_ref: &str, revision: u64) -> ValidateSecretRequest {
        serde_json::from_str(&format!(
            r#"{{"schemaVersion":1,"secretRef":"{secret_ref}","expectedRecordRevision":{revision}}}"#
        ))
        .expect("validate request")
    }

    #[test]
    fn secret_validate_matching_revision_returns_valid_contract_dto() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        let result = validate_secret_from_state(
            &state,
            validate_request(&seeded.secret_ref, seeded.record_revision),
        )
        .unwrap_or_else(|err| {
            panic!(
                "matching revision must Ok validate DTO: {}",
                serde_json::to_string(&err).unwrap_or_default()
            )
        });
        let json = serde_json::to_value(&result.data).expect("json");
        assert_eq!(json["outcome"], "valid");
        assert_eq!(json["aggregate"]["secretRef"], seeded.secret_ref);
    }

    #[test]
    fn secret_validate_revision_mismatch_fails_closed() {
        let tmp = TempDir::new().expect("tempdir");
        let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
        let seeded = seed_opened_store_pending_candidate(opened.store()).expect("seed");
        let db = Arc::new(Database::memory().expect("memory db"));
        let state = AppState::new_with_secret_store(db, opened);
        assert_err_not_empty_ok(validate_secret_from_state(
            &state,
            validate_request(&seeded.secret_ref, seeded.record_revision.saturating_add(1)),
        ));
    }

}
