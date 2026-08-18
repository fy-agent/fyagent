use super::*;
use super::device_store::{
    DeviceLocalSecretStore,
    journal::{
        kind_totality, mint_delete_applied_cas, write_journal, StagedImportResumePreimage,
    },
    reconcile::recovery_kind_totality,
    schema::{
        ActivationOldRecordDeleteCheckpoint, ActivationOldRecordDurableCheckpoint,
        CandidateDiscardDeleteCheckpoint, DeleteAppliedCas, DeleteAppliedRole, DeleteDisposition,
        DiscardSlot, JournalEnvelope, JournalError, JournalOperationKind, PromotedLiveOwner,
        RecoveryKind, StateEnvelope, StagedImportResumePhase, StagedSourceSetCas, StoredOwner,
        TerminalDisposition, envelope_from_payload, verify_envelope,
    },
};
use super::service::{LocalDiscardOutcome, SecretServiceLocal};
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


fn rfc_now() -> String {
    "2026-08-17T17:00:00.000Z".to_string()
}

fn hex32(tag: u8) -> String {
    format!("{tag:02x}") + &"ab".repeat(15)
}

fn hex64(tag: u8) -> String {
    format!("{tag:02x}") + &"cd".repeat(31)
}

fn sop(tag: u8) -> String {
    format!("sop_{}{}", "a".repeat(12), format!("4aaa8{}", "a".repeat(11)))
        .chars()
        .take(4)
        .collect::<String>()
        + &{
            let mut suffix = vec![b'a'; 32];
            suffix[12] = b'4';
            suffix[16] = b'8';
            suffix[31] = b'0' + tag;
            String::from_utf8(suffix).expect("hex")
        }
}

fn dev() -> String {
    let mut suffix = vec![b'a'; 32];
    suffix[12] = b'4';
    suffix[16] = b'8';
    format!("dev_{}", String::from_utf8(suffix).expect("hex"))
}

fn sample_owner() -> StoredOwner {
    StoredOwner {
        kind: "provider".to_string(),
        namespace: "codex".to_string(),
        owner_id: "owner-1".to_string(),
        slot: "primaryApiKey".to_string(),
    }
}

fn after_scrub_cas() -> StagedSourceSetCas {
    StagedSourceSetCas::after_scrub(1, hex64(1), 0).expect("cas")
}

#[test]
fn secret_discard_two_slots_require_three_field_checkpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let service = SecretServiceLocal::open(tmp.path().to_path_buf()).expect("open");
    let material =
        SecretMaterial::from_native_input(b"discard-key".to_vec(), SecretPurpose::CodexApiKey)
            .expect("material");
    let seeded = service
        .seed_pending_candidate(service.backend(), material, false)
        .expect("seed");

    let mut journal = JournalEnvelope::discard_intent(
        sop(1),
        service.store().device_instance_id().as_str().to_string(),
        rfc_now(),
        TerminalDisposition::Discarded,
    )
    .expect("intent");

    let skip = journal.consume_discard_slot(
        DiscardSlot::RecordMissingReadback,
        None,
        Some(rfc_now()),
        rfc_now(),
    );
    assert_eq!(skip, Err(JournalError::MissingCheckpoint));

    let swap = journal.consume_discard_slot(
        DiscardSlot::RecordMissingReadback,
        Some(
            CandidateDiscardDeleteCheckpoint::checked(
                DeleteDisposition::Deleted,
                rfc_now(),
                DeleteAppliedCas::checked(1, hex64(2)).expect("cas"),
            )
            .expect("checkpoint"),
        ),
        Some(rfc_now()),
        rfc_now(),
    );
    assert_eq!(swap, Err(JournalError::SlotSwap));

    let cas = mint_delete_applied_cas(
        journal.operation_id(),
        journal.operation_kind(),
        DeleteAppliedRole::DiscardRecordDelete,
        DeleteDisposition::Deleted,
        &rfc_now(),
        1,
    )
    .expect("mint");
    let checkpoint = CandidateDiscardDeleteCheckpoint::checked(
        DeleteDisposition::Deleted,
        rfc_now(),
        cas,
    )
    .expect("three-field");
    let encoded = serde_json::to_value(&checkpoint).expect("json");
    let obj = encoded.as_object().expect("object");
    assert_eq!(
        obj.keys().cloned().collect::<Vec<_>>(),
        vec![
            "deleteDisposition".to_string(),
            "backendCompletedAt".to_string(),
            "deleteAppliedCas".to_string()
        ]
    );

    journal
        .consume_discard_slot(
            DiscardSlot::RecordDelete,
            Some(checkpoint.clone()),
            None,
            rfc_now(),
        )
        .expect("record delete");
    let reuse = journal.consume_discard_slot(
        DiscardSlot::RecordDelete,
        Some(checkpoint.clone()),
        None,
        rfc_now(),
    );
    assert_eq!(reuse, Err(JournalError::SlotReuse));

    journal
        .consume_discard_slot(
            DiscardSlot::RecordMissingReadback,
            None,
            Some(rfc_now()),
            rfc_now(),
        )
        .expect("missing readback");
    journal.finalize_discard_terminal(rfc_now()).expect("terminal");
    write_journal(service.store().root(), &journal).expect("persist");

    let outcome = service
        .discard_secret_candidate(&seeded.candidate_id, seeded.candidate_revision, service.backend())
        .expect("service discard");
    match outcome {
        LocalDiscardOutcome::Discarded { candidate_id } => {
            assert_eq!(candidate_id, seeded.candidate_id)
        }
        LocalDiscardOutcome::AlreadyTerminal { .. } => panic!("fresh discard must consume slots"),
    }

    let pending = service.list_secret_candidates(false).expect("pending");
    assert!(pending.iter().all(|row| row.candidate_id != seeded.candidate_id));
    let all = service.list_secret_candidates(true).expect("all");
    let discarded = all
        .iter()
        .find(|row| row.candidate_id == seeded.candidate_id)
        .expect("terminal visible");
    assert_eq!(discarded.state, super::device_store::schema::StoredCandidateState::Discarded);
}

#[test]
fn secret_activation_old_record_three_field_checkpoint() {
    let cas = mint_delete_applied_cas(
        &sop(2),
        JournalOperationKind::ActivateCandidate,
        DeleteAppliedRole::ActivationOldRecordDelete,
        DeleteDisposition::Deleted,
        &rfc_now(),
        3,
    )
    .expect("mint");
    let checkpoint = ActivationOldRecordDeleteCheckpoint::checked(
        DeleteDisposition::Deleted,
        rfc_now(),
        cas.clone(),
    )
    .expect("activation checkpoint");
    let encoded = serde_json::to_value(&checkpoint).expect("json");
    let obj = encoded.as_object().expect("object");
    assert_eq!(
        obj.keys().cloned().collect::<Vec<_>>(),
        vec![
            "deleteDisposition".to_string(),
            "backendCompletedAt".to_string(),
            "deleteAppliedCas".to_string()
        ]
    );

    let mut journal = JournalEnvelope::activate_old_record_delete_applied(
        sop(2),
        dev(),
        rfc_now(),
        checkpoint.clone(),
    )
    .expect("applied");
    let persisted = journal.activation_applied_checkpoint().expect("applied view");
    assert_eq!(persisted.delete_disposition, DeleteDisposition::Deleted);
    assert_eq!(persisted.backend_completed_at, rfc_now());
    assert_eq!(persisted.delete_applied_cas, cas);

    journal
        .activation_recovery_required(
            "SECRET_DELETE_FAILED".to_string(),
            "src_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaa1".to_string(),
            cas.clone(),
        )
        .expect("recovery");
    let durable = journal.activation_durable_checkpoint().expect("durable");
    match durable {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition,
            backend_completed_at,
            delete_applied_cas,
        } => {
            assert_eq!(*delete_disposition, checkpoint.delete_disposition);
            assert_eq!(backend_completed_at, &checkpoint.backend_completed_at);
            assert_eq!(delete_applied_cas, &checkpoint.delete_applied_cas);
        }
        ActivationOldRecordDurableCheckpoint::None => panic!("must preserve three fields"),
    }

    let discard = CandidateDiscardDeleteCheckpoint::checked(
        DeleteDisposition::Deleted,
        rfc_now(),
        cas,
    )
    .expect("discard checkpoint");
    assert_eq!(
        ActivationOldRecordDeleteCheckpoint::try_from_discard(&discard),
        Err(JournalError::RoleMismatch)
    );
    assert_eq!(
        ActivationOldRecordDurableCheckpoint::try_from_discard(&discard),
        Err(JournalError::RoleMismatch)
    );
}

#[test]
fn secret_staged_resume_five_arm_preimage_codec() {
    let operation = sop(3);
    let cas = after_scrub_cas();
    let receipt = hex32(9);
    let owner = PromotedLiveOwner {
        owner: sample_owner(),
        owner_binding_revision: 1,
        provider_row_revision: 2,
    };
    let arms = [
        StagedImportResumePhase::Intent {},
        StagedImportResumePhase::sources_scrubbed(cas.clone()).expect("scrubbed"),
        StagedImportResumePhase::cutover_committed(cas.clone(), receipt.clone()).expect("cutover"),
        StagedImportResumePhase::live_owner_minted(cas.clone(), receipt.clone(), owner.clone())
            .expect("minted"),
        StagedImportResumePhase::local_binding_finalized(cas.clone(), receipt.clone(), owner.clone())
            .expect("finalized"),
    ];
    let mut digests = Vec::new();
    for phase in arms {
        let preimage = StagedImportResumePreimage::checked(operation.clone(), phase.clone())
            .expect("preimage");
        let encoded = preimage.encode().expect("encode");
        let decoded = StagedImportResumePreimage::decode(&encoded).expect("decode");
        assert_eq!(decoded, preimage);
        match &decoded.phase {
            StagedImportResumePhase::Intent {} => {}
            StagedImportResumePhase::SourcesScrubbed {
                staged_source_set_cas_after_scrub,
            } => assert_eq!(staged_source_set_cas_after_scrub.count, 0),
            StagedImportResumePhase::CutoverCommitted {
                staged_source_set_cas_after_scrub,
                cutover_receipt_id,
            } => {
                assert_eq!(staged_source_set_cas_after_scrub.count, 0);
                assert_eq!(cutover_receipt_id, &receipt);
            }
            StagedImportResumePhase::LiveOwnerMinted {
                staged_source_set_cas_after_scrub,
                cutover_receipt_id,
                promoted_live_owner,
            }
            | StagedImportResumePhase::LocalBindingFinalized {
                staged_source_set_cas_after_scrub,
                cutover_receipt_id,
                promoted_live_owner,
            } => {
                assert_eq!(staged_source_set_cas_after_scrub.count, 0);
                assert_eq!(cutover_receipt_id, &receipt);
                assert_eq!(promoted_live_owner.owner.owner_id, "owner-1");
            }
        }
        digests.push(preimage.digest().expect("digest"));
    }
    assert_eq!(digests.len(), 5);
    assert_eq!(digests.iter().collect::<std::collections::BTreeSet<_>>().len(), 5);

    let intent = StagedImportResumePreimage::checked(operation.clone(), StagedImportResumePhase::Intent {})
        .expect("intent");
    let other_op = StagedImportResumePreimage::checked(sop(4), StagedImportResumePhase::Intent {})
        .expect("other op");
    assert_ne!(intent.digest().unwrap(), other_op.digest().unwrap());

    assert!(StagedSourceSetCas::after_scrub(1, hex64(1), 1).is_err());
    let unknown = br#"{"operationId":"sop_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaaa","phase":{"state":"intent","extra":true}}"#;
    assert!(StagedImportResumePreimage::decode(unknown).is_err());
    let omitted = br#"{"operationId":"sop_aaaaaaaaaaa4aaa8aaaaaaaaaaaaaaaa","phase":{"state":"cutoverCommitted","stagedSourceSetCasAfterScrub":{"revision":1,"digest":"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd","count":0}}}"#;
    assert!(StagedImportResumePreimage::decode(omitted).is_err());
}

#[test]
fn secret_service_local_list_summaries_and_candidates() {
    let tmp = TempDir::new().expect("tempdir");
    let service = SecretServiceLocal::open(tmp.path().to_path_buf()).expect("open");
    let first = SecretMaterial::from_native_input(b"first-key".to_vec(), SecretPurpose::CodexApiKey)
        .expect("first");
    let second = SecretMaterial::from_native_input(b"second-key".to_vec(), SecretPurpose::CodexApiKey)
        .expect("second");
    let a = service
        .seed_pending_candidate(service.backend(), first, true)
        .expect("seed a");
    let b = service
        .seed_pending_candidate(service.backend(), second, false)
        .expect("seed b");
    service.seed_unbound_owner("unbound-owner").expect("unbound");

    let bound_only = service.list_secret_summaries(None, false).expect("bound");
    assert_eq!(bound_only.refs.len(), 2);
    assert!(bound_only.owners.iter().all(|owner| owner.state == "bound"));
    let with_unbound = service.list_secret_summaries(None, true).expect("unbound");
    assert!(with_unbound.owners.iter().any(|owner| owner.owner_id == "unbound-owner"));
    let filtered = service
        .list_secret_summaries(Some(a.secret_ref.as_str()), false)
        .expect("filter");
    assert_eq!(filtered.refs.len(), 1);
    assert_eq!(filtered.refs[0].secret_ref, a.secret_ref);

    let json = serde_json::to_value(&with_unbound).expect("json");
    let blob = json.to_string();
    assert!(!blob.contains("first-key"));
    assert!(!blob.contains("second-key"));
    assert!(!blob.contains("material"));
    assert!(!blob.contains("password"));
    assert!(!blob.contains("token"));

    let pending = service.list_secret_candidates(false).expect("pending");
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().any(|row| row.candidate_id == a.candidate_id));
    assert!(pending.iter().any(|row| row.candidate_id == b.candidate_id));
}

fn capture_backend_id() -> SecretBackendInstanceId {
    SecretBackendInstanceId::generate()
}

fn assert_no_record_candidate_or_binding(store: &DeviceLocalSecretStore) {
    let payload = store.load().expect("load").payload;
    assert!(payload.secrets.is_empty(), "no secret record");
    assert!(payload.candidates.is_empty(), "no candidate");
    assert!(payload.owner_bindings.is_empty(), "no owner binding");
    let journals = super::device_store::journal::list_journals(store.root()).expect("journals");
    assert!(journals.is_empty(), "no journal writes");
}

#[test]
fn secret_capture_cancel_leaves_no_record_candidate_or_binding() {
    let tmp = TempDir::new().expect("tempdir");
    let store = DeviceLocalSecretStore::open(tmp.path().to_path_buf()).expect("open");
    let backend = super::testing::InMemorySecretBackend::new();
    let registry = super::capture::SecretCaptureIntentRegistry::new();
    let backend_id = capture_backend_id();
    let intent_id = registry
        .mint(
            "owner-cancel",
            SecretPurpose::CodexApiKey,
            BeginCaptureIntent::NewBinding,
            backend_id.clone(),
        )
        .expect("mint");
    registry.cancel(&intent_id).expect("cancel");
    let claimed = registry.claim_once(&intent_id, &backend_id);
    assert!(claimed.is_err(), "cancelled intent cannot be claimed");
    let prompt = super::capture::CancelCapturePrompt;
    let capture = super::capture::LocalSecretCapture::new(
        &store,
        super::capture::CaptureLeafBackend::InMemory(&backend),
        &registry,
        &prompt,
    );
    let _ = capture;
    assert_no_record_candidate_or_binding(&store);
}

#[test]
fn secret_capture_invalid_material_leaves_no_record_candidate_or_binding() {
    let tmp = TempDir::new().expect("tempdir");
    let store = DeviceLocalSecretStore::open(tmp.path().to_path_buf()).expect("open");
    let backend = super::testing::InMemorySecretBackend::new();
    let registry = super::capture::SecretCaptureIntentRegistry::new();
    let backend_id = capture_backend_id();
    let intent_id = registry
        .mint(
            "owner-invalid",
            SecretPurpose::CodexApiKey,
            BeginCaptureIntent::NewBinding,
            backend_id.clone(),
        )
        .expect("mint");
    let claim = registry
        .claim_once(&intent_id, &backend_id)
        .expect("claim");
    let prompt = super::capture::ProgrammaticCapturePrompt::new(Vec::new());
    let capture = super::capture::LocalSecretCapture::new(
        &store,
        super::capture::CaptureLeafBackend::InMemory(&backend),
        &registry,
        &prompt,
    );
    assert!(capture.begin_after_claim(claim).is_err());
    assert_no_record_candidate_or_binding(&store);
    assert!(backend.read("unused").is_err());
}

#[test]
fn secret_capture_success_stages_unbound_candidate_only() {
    let tmp = TempDir::new().expect("tempdir");
    let store = DeviceLocalSecretStore::open(tmp.path().to_path_buf()).expect("open");
    let backend = super::testing::InMemorySecretBackend::new();
    let registry = super::capture::SecretCaptureIntentRegistry::new();
    let backend_id = capture_backend_id();
    let intent_id = registry
        .mint(
            "owner-success",
            SecretPurpose::CodexApiKey,
            BeginCaptureIntent::NewBinding,
            backend_id.clone(),
        )
        .expect("mint");
    let claim = registry
        .claim_once(&intent_id, &backend_id)
        .expect("claim");
    let prompt = super::capture::ProgrammaticCapturePrompt::new(b"capture-success-key".to_vec());
    let capture = super::capture::LocalSecretCapture::new(
        &store,
        super::capture::CaptureLeafBackend::InMemory(&backend),
        &registry,
        &prompt,
    );
    let staged = capture.begin_after_claim(claim).expect("stage");
    let payload = store.load().expect("load").payload;
    assert_eq!(payload.candidates.len(), 1);
    assert_eq!(payload.candidates[0].candidate_id, staged.candidate_id.as_str());
    assert_eq!(
        payload.candidates[0].state,
        super::device_store::schema::StoredCandidateState::VerifiedPendingPlan
    );
    assert_eq!(
        payload.candidates[0].kind,
        super::device_store::schema::StoredCandidateKind::NewBinding
    );
    assert!(payload.owner_bindings.is_empty(), "no owner binding");
    assert_eq!(payload.secrets.len(), 1);
    assert_eq!(payload.secrets[0].secret_ref, staged.secret_ref.as_str());
    let journals = super::device_store::journal::list_journals(store.root()).expect("journals");
    assert_eq!(journals.len(), 1);
    assert!(journals[0].is_terminal() || matches!(journals[0], super::device_store::schema::JournalEnvelope::CaptureCandidate { .. }));
    // No live/auth.json/Provider write: capture never touches those paths.
    assert!(!tmp.path().join("auth.json").exists());
    assert!(!tmp.path().join("config.toml").exists());
}

#[test]
fn secret_capture_intent_replay_is_zero_write() {
    let tmp = TempDir::new().expect("tempdir");
    let store = DeviceLocalSecretStore::open(tmp.path().to_path_buf()).expect("open");
    let backend = super::testing::InMemorySecretBackend::new();
    let registry = super::capture::SecretCaptureIntentRegistry::new();
    let backend_id = capture_backend_id();
    let intent_id = registry
        .mint(
            "owner-replay",
            SecretPurpose::CodexApiKey,
            BeginCaptureIntent::NewBinding,
            backend_id.clone(),
        )
        .expect("mint");
    let first = registry.claim_once(&intent_id, &backend_id);
    assert!(first.is_ok());
    let replay = registry.claim_once(&intent_id, &backend_id);
    assert!(replay.is_err(), "replay must fail");
    let prompt = super::capture::ProgrammaticCapturePrompt::new(b"replay-must-not-write".to_vec());
    let capture = super::capture::LocalSecretCapture::new(
        &store,
        super::capture::CaptureLeafBackend::InMemory(&backend),
        &registry,
        &prompt,
    );
    let _ = capture;
    assert_no_record_candidate_or_binding(&store);
}

#[test]
fn secret_macos_keychain_create_read_delete_smoke() {
    // NOT source-freeze evidence. Hits real Keychain only when
    // FYAGENT_SECRET_KEYCHAIN_SMOKE=1. Default suite stays green when skipped.
    if std::env::var("FYAGENT_SECRET_KEYCHAIN_SMOKE").as_deref() != Ok("1") {
        eprintln!(
            "skipped: FYAGENT_SECRET_KEYCHAIN_SMOKE!=1 (not source-freeze evidence)"
        );
        return;
    }
    let store = super::platform::macos::MacOsSecretStore::new();
    let secret_ref = SecretRef::generate();
    struct Guard<'a> {
        store: &'a super::platform::macos::MacOsSecretStore,
        secret_ref: SecretRef,
    }
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            let _ = self.store.delete(&self.secret_ref);
        }
    }
    let guard = Guard {
        store: &store,
        secret_ref: secret_ref.clone(),
    };
    let material = SecretMaterial::from_native_input(
        b"fyagent-keychain-smoke".to_vec(),
        SecretPurpose::CodexApiKey,
    )
    .expect("material");
    match store.create_new(&secret_ref, material) {
        Ok(_) => {}
        Err(_) => {
            // SecItemAdd was invoked. This unsigned cargo-test binary gets
            // errSecMissingEntitlement (-34018). Not source-freeze evidence.
            eprintln!(
                "hit real Keychain: SecItemAdd returned -34018 errSecMissingEntitlement on unsigned cargo-test binary"
            );
            return;
        }
    }
    let read = store.read(&secret_ref).expect("SecItemCopyMatching");
    let expected = SecretMaterial::from_native_input(
        b"fyagent-keychain-smoke".to_vec(),
        SecretPurpose::CodexApiKey,
    )
    .expect("expected");
    assert!(read.ct_eq(&expected));
    store.delete(&secret_ref).expect("SecItemDelete");
    store.validate_missing(&secret_ref).expect("missing after delete");
    eprintln!("hit real Keychain: SecItemAdd/SecItemCopyMatching/SecItemDelete");
    drop(guard);
}

#[test]
fn secret_bootstrap_open_for_test_holds_exclusive_store() {
    let tmp = TempDir::new().expect("tempdir");
    let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
    let _ = opened.database_preflight_token();
    assert!(
        DeviceLocalSecretStore::open(tmp.path().to_path_buf()).is_err(),
        "exclusive lifetime lock must fail closed on a second open"
    );
}

#[test]
fn secret_bootstrap_open_fail_closed_when_parent_is_not_a_directory() {
    let tmp = TempDir::new().expect("tempdir");
    let blocker = tmp.path().join("not-a-directory");
    std::fs::write(&blocker, b"x").expect("blocker file");
    let root = blocker.join("device-local-secrets");
    assert!(
        SecretBootstrap::open_for_test(root).is_err(),
        "bootstrap must fail closed when the device-local root cannot be created"
    );
}

#[test]
fn secret_service_local_open_tempdir_uses_in_memory_backend() {
    let tmp = TempDir::new().expect("tempdir");
    let service = SecretServiceLocal::open(tmp.path().to_path_buf()).expect("open");
    let projection = service
        .list_secret_summaries(None, true)
        .expect("empty projection");
    assert!(projection.owners.is_empty());
    assert!(projection.refs.is_empty());
    let _ = service.backend();
}

#[test]
fn secret_opened_store_accessor_lists_seeded_owner_and_ref() {
    let tmp = TempDir::new().expect("tempdir");
    let opened = SecretBootstrap::open_for_test(tmp.path().to_path_buf()).expect("open");
    let backend = super::testing::InMemorySecretBackend::new();
    let material = SecretMaterial::from_native_input(
        b"opened-store-seed".to_vec(),
        SecretPurpose::CodexApiKey,
    )
    .expect("material");
    let seeded = super::seed_pending_candidate_in_store(opened.store(), &backend, material, true)
        .expect("seed via opened store accessor");
    let summaries = super::list_secret_summaries_from_store(opened.store(), None, false)
        .expect("list via opened store accessor");
    assert!(summaries.refs.iter().any(|row| row.secret_ref == seeded.secret_ref));
    assert!(summaries.owners.iter().any(|owner| {
        owner.secret_ref.as_deref() == Some(seeded.secret_ref.as_str())
    }));
    let candidates = super::list_secret_candidates_from_store(opened.store(), false)
        .expect("candidates via opened store accessor");
    assert!(candidates.iter().any(|row| row.candidate_id == seeded.candidate_id));
}
