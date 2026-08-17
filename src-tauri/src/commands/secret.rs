use crate::secret::{
    BeginSecretCaptureRequest, CheckSecretApplyReadinessRequest, DeleteSecretRequest,
    DiscardSecretCandidateRequest, DiscardSecretCandidateResult, GetSecretCleanupImpactRequest,
    GetSecretDeleteImpactRequest, ListSecretAuditRequest, ListSecretBackendOptionsRequest,
    ListSecretBackendOptionsResult, ListSecretCandidatesRequest, ListSecretCandidatesResult,
    ListSecretSummariesRequest, ListSecretSummariesResult, MigrateLegacyCodexSecretsRequest,
    RetrySecretCleanupRequest, RotateSecretRequest, SecretApplyReadiness, SecretAuditPage,
    SecretCommandResult, SecretDeleteImpact, SecretDeleteResult, SecretInternalError,
    SecretMigrationReport, SecretMutationResult, SecretRecoveryImpact, SecretRecoveryResult,
    SecretService, SecretValidationResult, SetSecretLockedRequest, StageSecretCandidateResult,
    ValidateSecretRequest,
};

fn unavailable<T>() -> SecretCommandResult<T> {
    Err(crate::secret::service_command_error(
        SecretInternalError::input_invalid(),
    ))
}

pub async fn list_secret_summaries(
    _service: &SecretService,
    _request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult> {
    unavailable()
}

pub async fn list_secret_backend_options(
    _service: &SecretService,
    _request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult> {
    unavailable()
}

pub async fn begin_secret_capture(
    _service: &SecretService,
    _request: BeginSecretCaptureRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    unavailable()
}

pub async fn rotate_secret(
    _service: &SecretService,
    _request: RotateSecretRequest,
) -> SecretCommandResult<StageSecretCandidateResult> {
    unavailable()
}

pub async fn list_secret_candidates(
    _service: &SecretService,
    _request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult> {
    unavailable()
}

pub async fn discard_secret_candidate(
    _service: &SecretService,
    _request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult> {
    unavailable()
}

pub async fn set_secret_locked(
    _service: &SecretService,
    _request: SetSecretLockedRequest,
) -> SecretCommandResult<SecretMutationResult> {
    unavailable()
}

pub async fn get_secret_delete_impact(
    _service: &SecretService,
    _request: GetSecretDeleteImpactRequest,
) -> SecretCommandResult<SecretDeleteImpact> {
    unavailable()
}

pub async fn delete_secret(
    _service: &SecretService,
    _request: DeleteSecretRequest,
) -> SecretCommandResult<SecretDeleteResult> {
    unavailable()
}

pub async fn get_secret_cleanup_impact(
    _service: &SecretService,
    _request: GetSecretCleanupImpactRequest,
) -> SecretCommandResult<SecretRecoveryImpact> {
    unavailable()
}

pub async fn retry_secret_cleanup(
    _service: &SecretService,
    _request: RetrySecretCleanupRequest,
) -> SecretCommandResult<SecretRecoveryResult> {
    unavailable()
}

pub async fn validate_secret(
    _service: &SecretService,
    _request: ValidateSecretRequest,
) -> SecretCommandResult<SecretValidationResult> {
    unavailable()
}

pub async fn check_secret_apply_readiness(
    _service: &SecretService,
    _request: CheckSecretApplyReadinessRequest,
) -> SecretCommandResult<SecretApplyReadiness> {
    unavailable()
}

pub async fn migrate_legacy_codex_secrets(
    _service: &SecretService,
    _request: MigrateLegacyCodexSecretsRequest,
) -> SecretCommandResult<SecretMigrationReport> {
    unavailable()
}

pub async fn list_secret_audit(
    _service: &SecretService,
    _request: ListSecretAuditRequest,
) -> SecretCommandResult<SecretAuditPage> {
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
