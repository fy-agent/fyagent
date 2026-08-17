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

fn unavailable<T>() -> SecretCommandResult<T> {
    crate::secret::command_unavailable()
}

#[tauri::command]
pub async fn list_secret_summaries(
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    let _ = request;
    unavailable()
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
    request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult> {
    let _ = request;
    unavailable()
}

#[tauri::command]
pub async fn discard_secret_candidate(
    request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult> {
    let _ = request;
    unavailable()
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
}
