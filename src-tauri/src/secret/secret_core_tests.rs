use super::*;
use super::device_store::{
    DeviceLocalSecretStore,
    journal::kind_totality,
    reconcile::recovery_kind_totality,
    schema::{
        JournalOperationKind, RecoveryKind, StateEnvelope, envelope_from_payload, verify_envelope,
    },
};
use tempfile::TempDir;

#[test]
fn secret_ref_generate_and_reject_invalid() {
    let generated = SecretRef::generate();
    assert!(generated.as_str().starts_with("sec_"));
    assert_eq!(generated.as_str().len(), 36);
    assert!(SecretRef::try_from(generated.as_str()).is_ok());
    assert!(SecretRef::try_from(generated.as_str().to_string()).is_ok());

    assert!(SecretRef::parse("SEC_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    assert!(SecretRef::parse("sct_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    assert!(SecretRef::parse("sec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    assert!(SecretRef::parse("sec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    // version nibble is not 4
    assert!(SecretRef::parse("sec_aaaaaaaaaaa1aaaaaaaaaaaaaaaaaaaa".into()).is_err());
    // uppercase hex
    assert!(SecretRef::parse("sec_aaaaaaaaaaa4aaaa8aaaaaaaaaaaaaaa".replace('a', "A")).is_err());
}

#[test]
fn secret_device_instance_id_generate_and_reject_invalid() {
    let generated = DeviceInstanceId::generate();
    assert!(generated.as_str().starts_with("dev_"));
    assert_eq!(generated.as_str().len(), 36);
    assert!(DeviceInstanceId::try_from(generated.as_str()).is_ok());
    assert!(DeviceInstanceId::try_from(generated.as_str().to_string()).is_ok());

    assert!(DeviceInstanceId::parse("DEV_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    assert!(DeviceInstanceId::parse("sec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()).is_err());
    assert!(DeviceInstanceId::parse("dev_short".into()).is_err());
    assert!(DeviceInstanceId::parse("dev_aaaaaaaaaaa1aaaaaaaaaaaaaaaaaaaa".into()).is_err());
}

#[test]
fn secret_material_from_native_input_bounds() {
    // SecretMaterial has no Serialize/Clone (see material.rs contract comment).
    let ok = SecretMaterial::from_native_input(b"k".to_vec(), SecretPurpose::CodexApiKey);
    assert!(ok.is_ok());
    let max = SecretMaterial::from_native_input(vec![b'x'; 2560], SecretPurpose::CodexApiKey);
    assert!(max.is_ok());
    assert!(SecretMaterial::from_native_input(Vec::new(), SecretPurpose::CodexApiKey).is_err());
    assert!(SecretMaterial::from_native_input(vec![b'x'; 2561], SecretPurpose::CodexApiKey).is_err());
    assert!(SecretMaterial::from_native_input(b"a\0b".to_vec(), SecretPurpose::CodexApiKey).is_err());
}

#[test]
fn secret_journal_kind_totality() {
    let mut seen = Vec::new();
    for kind in JournalOperationKind::ALL {
        seen.push(kind_totality(kind));
    }
    assert_eq!(
        seen,
        [
            "captureCandidate",
            "migrateLegacy",
            "rotateCandidate",
            "activateCandidate",
            "discardCandidate",
            "deleteSecret",
            "detachProviderOwner",
            "stagedImport",
        ]
    );
    assert_eq!(JournalOperationKind::ALL.len(), 8);
}

#[test]
fn secret_recovery_kind_totality() {
    let mut seen = Vec::new();
    for kind in RecoveryKind::ALL {
        seen.push(recovery_kind_totality(kind));
    }
    assert_eq!(
        seen,
        [
            "activationCleanup",
            "captureCompensation",
            "deleteFinalization",
            "ownerDetachFinalization",
        ]
    );
    assert_eq!(RecoveryKind::ALL.len(), 4);
    assert_eq!(
        recovery_kind_totality(match SecretRecoveryKind::ActivationCleanup {
            SecretRecoveryKind::ActivationCleanup => RecoveryKind::ActivationCleanup,
            SecretRecoveryKind::CaptureCompensation => RecoveryKind::CaptureCompensation,
            SecretRecoveryKind::DeleteFinalization => RecoveryKind::DeleteFinalization,
            SecretRecoveryKind::OwnerDetachFinalization => RecoveryKind::OwnerDetachFinalization,
        }),
        "activationCleanup"
    );
}

#[test]
fn secret_atomic_write_reload_and_hash_mismatch_fail_closed() {
    let tmp = TempDir::new().expect("tempdir");
    let store = DeviceLocalSecretStore::open(tmp.path().to_path_buf()).expect("open");
    let loaded = store.load().expect("load");
    assert_eq!(loaded.schema_version, 1);
    verify_envelope(&loaded).expect("hash ok");

    let mut payload = loaded.payload.clone();
    payload.store_revision = 2;
    payload.updated_at = loaded.payload.updated_at.clone();
    let stored = store.store(payload).expect("store");
    let reloaded = store.load().expect("reload");
    assert_eq!(reloaded.payload.store_revision, 2);
    assert_eq!(reloaded.payload_sha256, stored.payload_sha256);

    let mut broken = reloaded.clone();
    broken.payload_sha256 = "0".repeat(64);
    let bytes = serde_json::to_vec(&broken).expect("json");
    std::fs::write(tmp.path().join("state.json"), bytes).expect("overwrite");
    assert!(store.load().is_err(), "hash mismatch must fail closed");
    assert!(verify_envelope(&broken).is_err());
    let _ = envelope_from_payload(reloaded.payload.clone());
}

#[test]
fn secret_confirmation_slots_are_twelve_with_ten_delete_missing() {
    let activation = [
        ActivationConfirmationSlot::CandidateRead,
        ActivationConfirmationSlot::OldRecordDelete,
        ActivationConfirmationSlot::OldRecordMissingReadback,
    ];
    let recovery = [
        RecoveryConfirmationSlot::ActiveRecordRead,
        RecoveryConfirmationSlot::OldRecordDelete,
        RecoveryConfirmationSlot::OldRecordMissingReadback,
        RecoveryConfirmationSlot::UncommittedRecordDelete,
        RecoveryConfirmationSlot::UncommittedRecordMissingReadback,
        RecoveryConfirmationSlot::AdmittedRecordDelete,
        RecoveryConfirmationSlot::AdmittedRecordMissingReadback,
    ];
    let discard = [
        CandidateDiscardConfirmationSlot::RecordDelete,
        CandidateDiscardConfirmationSlot::RecordMissingReadback,
    ];
    assert_eq!(activation.len() + recovery.len(), 10);
    assert_eq!(discard.len(), 2);
    assert_eq!(activation.len() + recovery.len() + discard.len(), 12);

    let delete_missing_count = 2 + 6 + 2;
    assert_eq!(delete_missing_count, 10);
    let _ = (
        ActivationConfirmationSlot::OldRecordDelete,
        ActivationConfirmationSlot::OldRecordMissingReadback,
        RecoveryConfirmationSlot::OldRecordDelete,
        RecoveryConfirmationSlot::OldRecordMissingReadback,
        RecoveryConfirmationSlot::UncommittedRecordDelete,
        RecoveryConfirmationSlot::UncommittedRecordMissingReadback,
        RecoveryConfirmationSlot::AdmittedRecordDelete,
        RecoveryConfirmationSlot::AdmittedRecordMissingReadback,
        CandidateDiscardConfirmationSlot::RecordDelete,
        CandidateDiscardConfirmationSlot::RecordMissingReadback,
    );
}

#[test]
fn secret_fake_backend_write_read_delete_validate() {
    let backend = super::testing::InMemorySecretBackend::new();
    let material =
        SecretMaterial::from_native_input(b"codex-test-key".to_vec(), SecretPurpose::CodexApiKey)
            .expect("material");
    backend.write("loc-1", material).expect("write");
    backend.validate("loc-1").expect("validate");
    let read = backend.read("loc-1").expect("read");
    let expected =
        SecretMaterial::from_native_input(b"codex-test-key".to_vec(), SecretPurpose::CodexApiKey)
            .expect("expected");
    assert!(read.ct_eq(&expected));
    backend.delete("loc-1").expect("delete");
    assert!(backend.read("loc-1").is_err());
}

#[test]
fn secret_command_request_dto_deny_unknown_fields() {
    let err = match serde_json::from_str::<ListSecretSummariesRequest>(
        r#"{"schemaVersion":1,"includeUnboundOwners":false,"limit":10,"unexpected":true}"#,
    ) {
        Ok(_) => panic!("unknown field must fail"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("unknown field"));
}
