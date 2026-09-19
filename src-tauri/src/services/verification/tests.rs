use super::*;
use chrono::Utc;
use std::sync::RwLock;

const PROJECT: &str = "00000000-0000-4000-8000-000000000001";
const OTHER: &str = "00000000-0000-4000-8000-000000000002";

struct Reader(RwLock<ProjectDependencySnapshot>);
impl ProjectDependencyReader for Reader {
    fn read(&self, project: &str) -> VerificationResult<ProjectDependencySnapshot> {
        let s = self.0.read().unwrap().clone();
        if s.project_id != project {
            return Err("project_unavailable");
        }
        Ok(s)
    }
    fn saved_model(&self, _: &str) -> VerificationResult<SavedModel> {
        Err("saved_model_unavailable")
    }
    fn validate_kit(
        &self,
        _: &str,
        selected: KitFixture,
    ) -> VerificationResult<KitValidationReceipt> {
        Ok(KitValidationReceipt {
            kit: self.0.read().unwrap().kit.clone().unwrap(),
            passed: selected == KitFixture::Baseline,
            validator_version: 1,
            fixture: selected,
            sample: SampleReceipt {
                input_digest: "b".repeat(64),
                code: if selected == KitFixture::Baseline {
                    SampleCode::Ok
                } else {
                    SampleCode::InvalidInput
                },
                matches_expectation: true,
                metrics: (selected == KitFixture::Baseline).then_some(SampleMetrics {
                    current_minor: 18_000_000,
                    previous_minor: 15_000_000,
                    growth_bps: 2_000,
                    target_bps: 9_000,
                }),
                source_row_ids: if selected == KitFixture::Baseline {
                    vec!["row-1".into()]
                } else {
                    vec![]
                },
                validator: "weekly-report/v1".into(),
            },
        })
    }
}
fn fixture() -> (Arc<VerificationService>, Arc<Reader>) {
    let reader = Arc::new(Reader(RwLock::new(ProjectDependencySnapshot {
        project_id: PROJECT.into(),
        project_revision: 1,
        kit: Some(KitIdentity {
            kit_id: "weekly-report".into(),
            kit_version: "1.0.0".into(),
            manifest_digest: "a".repeat(64),
        }),
        active: true,
        resource_revision: "configuration-A".into(),
        credential_generation: Some("private-canary-generation".into()),
        configuration_saved: true,
        projection: ProjectionState::Unavailable,
    })));
    (
        Arc::new(VerificationService::new(
            Arc::new(Database::memory().unwrap()),
            reader.clone(),
        )),
        reader,
    )
}
fn manual(stage: Stage) -> ManualRequest {
    ManualRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        stage,
        outcome: Outcome::Passed,
        person: "张三".into(),
        role: "客户负责人".into(),
        scope: "经营周报试点".into(),
        observed_at: domain::stamp(Utc::now()),
        external_basis: Some(ExternalBasis {
            reference: "UAT-2026-09".into(),
            issuer: "客户业务部".into(),
        }),
        basis_evidence_ids: vec![],
    }
}

#[tokio::test]
async fn verification_config_is_not_auth_tool_sample_or_customer() {
    let (s, _) = fixture();
    let result = s
        .run(RunRequest {
            project_id: PROJECT.into(),
            expected_revision: 1,
            checker: Checker::SavedConfigurationReadback,
            run_id: uuid::Uuid::new_v4().to_string(),
            fixture: None,
        })
        .await
        .unwrap();
    assert_eq!(result.evidence.len(), 1);
    let e = &result.evidence[0];
    assert_eq!(e.stage, Stage::ConfigurationSaved);
    assert_eq!(e.outcome, Outcome::Passed);
    assert_eq!(e.reason_code, "saved_projection_unavailable");
}

#[test]
fn verification_external_acceptance_without_machine_pass_and_revocation() {
    let (s, _) = fixture();
    let result = s.record_manual(manual(Stage::CustomerAccepted)).unwrap();
    assert_eq!(result.evidence[0].source_class, SourceClass::ManualRecord);
    assert_eq!(result.evidence[0].validity, Validity::Current);
    let result = s
        .revoke(RevokeRequest {
            project_id: PROJECT.into(),
            evidence_id: result.evidence[0].id.clone(),
        })
        .unwrap();
    assert_eq!(result.evidence[0].validity, Validity::Revoked);
}

#[tokio::test]
async fn verification_fixture_only_rejected_as_customer_basis() {
    let (s, _) = fixture();
    let result = s
        .run(RunRequest {
            project_id: PROJECT.into(),
            expected_revision: 1,
            checker: Checker::KitValidator,
            run_id: uuid::Uuid::new_v4().to_string(),
            fixture: Some(KitFixture::Baseline),
        })
        .await
        .unwrap();
    assert_eq!(result.evidence[0].source_class, SourceClass::LocalFixture);
    let mut r = manual(Stage::CustomerAccepted);
    r.external_basis = None;
    r.basis_evidence_ids = vec![result.evidence[0].id.clone()];
    assert_eq!(
        s.record_manual(r).unwrap_err(),
        "real_acceptance_basis_required"
    );
}

#[test]
fn verification_a_b_a_revision_kit_and_cross_project_invalidation() {
    let (s, reader) = fixture();
    s.record_manual(manual(Stage::CustomerAccepted)).unwrap();
    {
        let mut d = reader.0.write().unwrap();
        d.project_revision = 2;
        d.resource_revision = "B".into();
    }
    assert_eq!(
        s.snapshot(PROJECT).unwrap().evidence[0].validity,
        Validity::Stale
    );
    {
        let mut d = reader.0.write().unwrap();
        d.project_revision = 3;
        d.resource_revision = "configuration-A".into();
    }
    assert_eq!(
        s.snapshot(PROJECT).unwrap().evidence[0].validity,
        Validity::Stale
    );
    reader.0.write().unwrap().project_revision = 1;
    reader
        .0
        .write()
        .unwrap()
        .kit
        .as_mut()
        .unwrap()
        .manifest_digest = "b".repeat(64);
    assert_eq!(
        s.snapshot(PROJECT).unwrap().evidence[0].validity,
        Validity::Stale
    );
    assert!(s.snapshot(OTHER).unwrap().evidence.is_empty());
    let id = s.snapshot(PROJECT).unwrap().evidence[0].id.clone();
    assert_eq!(
        s.revoke(RevokeRequest {
            project_id: OTHER.into(),
            evidence_id: id
        })
        .unwrap_err(),
        "evidence_not_found"
    );
}

#[test]
fn verification_acceptance_inherits_revoked_basis_and_ttl() {
    let (s, _) = fixture();
    let r = s.record_manual(manual(Stage::ToolCallable)).unwrap();
    let id = r.evidence[0].id.clone();
    let mut accept = manual(Stage::CustomerAccepted);
    accept.external_basis = None;
    accept.basis_evidence_ids = vec![id.clone()];
    s.record_manual(accept).unwrap();
    s.revoke(RevokeRequest {
        project_id: PROJECT.into(),
        evidence_id: id,
    })
    .unwrap();
    assert!(s
        .snapshot(PROJECT)
        .unwrap()
        .evidence
        .iter()
        .any(|e| e.stage == Stage::CustomerAccepted && e.validity == Validity::Stale));
    let mut expired = manual(Stage::AuthenticationAvailable);
    expired.observed_at = domain::stamp(Utc::now() - Duration::minutes(16));
    let r = s.record_manual(expired).unwrap();
    assert!(r
        .evidence
        .iter()
        .any(|e| e.stage == Stage::AuthenticationAvailable && e.validity == Validity::Stale));
}

#[test]
fn verification_manual_validation_and_safe_handoff_export() {
    let (s, _) = fixture();
    let mut missing = manual(Stage::CustomerAccepted);
    missing.person = String::new();
    assert!(s.record_manual(missing).is_err());
    let mut empty = manual(Stage::CustomerAccepted);
    empty.external_basis = None;
    assert_eq!(s.record_manual(empty).unwrap_err(), "basis_required");
    let mut secret = manual(Stage::CustomerAccepted);
    secret.scope = "sk-secret-canary".into();
    assert!(s.record_manual(secret).is_err());
    s.record_manual(manual(Stage::CustomerAccepted)).unwrap();
    s.save_handoff(SaveHandoffRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        handoff_revision: 0,
        handoff: HandoffNotes {
            items: vec![HandoffItem {
                title: "客户复核".into(),
                owner: None,
                completed: false,
            }],
            rollback: Some(Rollback::ManualOnly),
        },
    })
    .unwrap();
    let p = s.preview(PROJECT).unwrap();
    assert!(p.markdown.contains("未指定"));
    assert!(p.markdown.contains("客户复核"));
    assert!(p.markdown.contains("张三"));
    assert!(!p.json.contains("private-canary-generation"));
    assert!(!p.json.contains("configuration-A"));
    assert!(!p.json.contains("credentialGeneration"));
    assert_eq!(
        serde_json::from_str::<VerificationSnapshot>(&p.json)
            .unwrap()
            .project_id,
        PROJECT
    );
}

#[test]
fn verification_manual_chinese_punctuation_preserves_boundaries_and_readback() {
    let (s, _) = fixture();
    let scope = "本机演练：经营周报样例，不是真实客户验收";
    let title = "阶段一；口径确认。张三·负责人（只读）！“周报”‘演练’？";
    let mut request = manual(Stage::SamplePassed);
    request.scope = scope.into();
    s.record_manual(request).unwrap();
    s.save_handoff(SaveHandoffRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        handoff_revision: 0,
        handoff: HandoffNotes {
            items: vec![HandoffItem {
                title: title.into(),
                owner: Some("交付·张三".into()),
                completed: false,
            }],
            rollback: Some(Rollback::ManualOnly),
        },
    })
    .unwrap();
    let snapshot = s.snapshot(PROJECT).unwrap();
    assert_eq!(snapshot.evidence[0].manual.as_ref().unwrap().scope, scope);
    assert_eq!(snapshot.handoff.items[0].title, title);
    let exported = s.preview(PROJECT).unwrap();
    assert!(exported.markdown.contains(scope));
    assert!(exported.markdown.contains(title));
    assert!(exported.json.contains(scope));

    for value in [
        "https://example.com/doc",
        "/tmp/report",
        "C:\\reports\\a",
        "../report",
        "api_key：private",
        "Bearer private",
        "secretRef：private",
        "token：private",
        "javascript:alert(1)",
        "资料\n秘密",
        " 前置空格",
    ] {
        let mut request = manual(Stage::SamplePassed);
        request.scope = value.into();
        assert!(s.record_manual(request).is_err(), "unsafe scope accepted");
        assert!(
            domain::validate_handoff(&HandoffNotes {
                items: vec![HandoffItem {
                    title: value.into(),
                    owner: None,
                    completed: false
                }],
                rollback: None,
            })
            .is_err(),
            "unsafe handoff accepted"
        );
    }
    assert_eq!(s.snapshot(PROJECT).unwrap().evidence.len(), 1);
}

#[test]
fn verification_handoff_cas_and_history_survive_service_restart() {
    let (s, reader) = fixture();
    s.record_manual(manual(Stage::CustomerAccepted)).unwrap();
    let req = SaveHandoffRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        handoff_revision: 0,
        handoff: HandoffNotes::default(),
    };
    s.save_handoff(req.clone()).unwrap();
    assert_eq!(s.save_handoff(req).unwrap_err(), "handoff_conflict");
    let restarted = VerificationService::new(s.db.clone(), reader);
    assert_eq!(restarted.snapshot(PROJECT).unwrap().evidence.len(), 1);
}

#[test]
fn verification_missing_adapter_never_produces_success() {
    let s = VerificationService::new(
        Arc::new(Database::memory().unwrap()),
        Arc::new(UnavailableProjectReader),
    );
    assert!(!s.snapshot(PROJECT).unwrap().available);
    assert_eq!(
        s.record_manual(manual(Stage::CustomerAccepted))
            .unwrap_err(),
        "project_reader_unavailable"
    );
}

#[test]
fn verification_migration_fresh_old_and_failed_ddl_rolls_back() {
    use rusqlite::Connection;
    let conn = Connection::open_in_memory().unwrap();
    Database::set_user_version(&conn, 21).unwrap();
    Database::apply_schema_migrations_on_conn(&conn).unwrap();
    assert_eq!(
        Database::get_user_version(&conn).unwrap(),
        crate::database::SCHEMA_VERSION
    );
    Database::apply_schema_migrations_on_conn(&conn).unwrap();
    let bad = Connection::open_in_memory().unwrap();
    bad.execute_batch("CREATE TABLE verification_evidence(id TEXT); PRAGMA user_version=21;")
        .unwrap();
    assert!(Database::apply_schema_migrations_on_conn(&bad).is_err());
    assert_eq!(Database::get_user_version(&bad).unwrap(), 21);
    assert_eq!(
        bad.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='verification_handoff'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let fresh = Database::memory().unwrap();
    assert!(fresh.verification_records(PROJECT).unwrap().0.is_empty());
}

#[test]
fn verification_sync_export_omits_private_evidence() {
    let (s, _) = fixture();
    s.record_manual(manual(Stage::CustomerAccepted)).unwrap();
    let sql = s.db.export_sql_string_for_sync().unwrap();
    assert!(!sql.contains("private-canary-generation"));
    assert!(!sql.contains("UAT-2026-09"));
    assert!(s.db.export_sql_string().unwrap().contains("UAT-2026-09"));
}

#[test]
fn verification_export_writes_redacted_json_and_markdown_in_temp_directory() {
    let (service, _) = fixture();
    service
        .record_manual(manual(Stage::CustomerAccepted))
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let json = directory.path().join("handoff.json");
    let markdown = directory.path().join("handoff.md");
    service.export_to(PROJECT, &json, false).unwrap();
    service.export_to(PROJECT, &markdown, true).unwrap();
    let content = std::fs::read_to_string(json).unwrap();
    assert_eq!(
        serde_json::from_str::<VerificationSnapshot>(&content)
            .unwrap()
            .project_id,
        PROJECT
    );
    assert!(!content.contains("private-canary-generation"));
    assert!(!content.contains("configuration-A"));
    assert!(!content.contains(directory.path().to_str().unwrap()));
    let content = std::fs::read_to_string(markdown).unwrap();
    assert!(content.contains("人工登记：张三"));
    assert!(content.contains("尚无回退办法"));
}

#[test]
fn verification_persist_failure_does_not_claim_recorded_success() {
    let (service, _) = fixture();
    service.db.conn.lock().unwrap().execute_batch(
        "CREATE TRIGGER reject_verification BEFORE INSERT ON verification_evidence BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
    ).unwrap();
    assert_eq!(
        service
            .record_manual(manual(Stage::CustomerAccepted))
            .unwrap_err(),
        "evidence_persist_failed"
    );
    assert!(service.snapshot(PROJECT).unwrap().evidence.is_empty());
}

#[tokio::test]
async fn verification_manual_copy_cannot_launder_fixture_into_customer_acceptance() {
    let (s, _) = fixture();
    let records = s
        .run(RunRequest {
            project_id: PROJECT.into(),
            expected_revision: 1,
            checker: Checker::KitValidator,
            run_id: uuid::Uuid::new_v4().to_string(),
            fixture: Some(KitFixture::Baseline),
        })
        .await
        .unwrap();
    let mut copied = manual(Stage::SamplePassed);
    copied.external_basis = None;
    copied.basis_evidence_ids = vec![records.evidence[0].id.clone()];
    let records = s.record_manual(copied).unwrap();
    let mut accepted = manual(Stage::CustomerAccepted);
    accepted.external_basis = None;
    accepted.basis_evidence_ids = vec![records.evidence.last().unwrap().id.clone()];
    assert_eq!(
        s.record_manual(accepted).unwrap_err(),
        "real_acceptance_basis_required"
    );
}

#[test]
fn verification_secret_rotation_and_clock_rollback_invalidate() {
    let (s, reader) = fixture();
    s.record_manual(manual(Stage::ToolCallable)).unwrap();
    let (records, revoked) = s.db.verification_records(PROJECT).unwrap();
    let snapshot = reader.0.read().unwrap().clone();
    assert_eq!(
        domain::views(
            &records,
            Some(&snapshot),
            &revoked,
            Utc::now() - Duration::minutes(1)
        )[0]
        .validity,
        Validity::Unverifiable
    );
    reader.0.write().unwrap().credential_generation = Some("rotated-secret-generation".into());
    assert_eq!(
        s.snapshot(PROJECT).unwrap().evidence[0].validity,
        Validity::Stale
    );
}

struct BlockingReader {
    inner: Reader,
    entered: Arc<tokio::sync::Notify>,
}
impl ProjectDependencyReader for BlockingReader {
    fn read(&self, id: &str) -> VerificationResult<ProjectDependencySnapshot> {
        self.inner.read(id)
    }
    fn saved_model(&self, _: &str) -> VerificationResult<SavedModel> {
        self.entered.notify_one();
        std::thread::sleep(std::time::Duration::from_millis(100));
        Err("saved_model_unavailable")
    }
    fn validate_kit(
        &self,
        _: &str,
        selected: KitFixture,
    ) -> VerificationResult<KitValidationReceipt> {
        self.inner.0.write().unwrap().project_revision += 1;
        self.inner.validate_kit(PROJECT, selected)
    }
}

#[tokio::test]
async fn verification_cancel_and_concurrent_revision_change_are_durable() {
    let (original, reader) = fixture();
    let entered = Arc::new(tokio::sync::Notify::new());
    let reader = Arc::new(BlockingReader {
        inner: Reader(RwLock::new(reader.0.read().unwrap().clone())),
        entered: entered.clone(),
    });
    let service = Arc::new(VerificationService::new(original.db.clone(), reader));
    let run_service = service.clone();
    let run = tokio::spawn(async move {
        run_service
            .run(RunRequest {
                project_id: PROJECT.into(),
                expected_revision: 1,
                checker: Checker::SavedModelProbe,
                run_id: uuid::Uuid::new_v4().to_string(),
                fixture: None,
            })
            .await
    });
    entered.notified().await;
    assert!(service.cancel(PROJECT).unwrap());
    let snapshot = run.await.unwrap().unwrap();
    assert_eq!(snapshot.evidence[0].outcome, Outcome::Cancelled);
    let snapshot = service
        .run(RunRequest {
            project_id: PROJECT.into(),
            expected_revision: 1,
            checker: Checker::KitValidator,
            run_id: uuid::Uuid::new_v4().to_string(),
            fixture: Some(KitFixture::Baseline),
        })
        .await
        .unwrap();
    assert!(snapshot
        .evidence
        .iter()
        .any(|e| e.stage == Stage::SamplePassed && e.validity == Validity::Stale));
}

#[test]
fn verification_shared_basis_graph_is_bounded_and_cycles_fail_closed() {
    let (service, reader) = fixture();
    service.record_manual(manual(Stage::SamplePassed)).unwrap();
    let (mut records, revoked) = service.db.verification_records(PROJECT).unwrap();
    for _ in 0..40 {
        let mut next = records[0].clone();
        next.id = uuid::Uuid::new_v4().to_string();
        next.basis_evidence_ids = records.iter().rev().take(2).map(|e| e.id.clone()).collect();
        records.push(next);
    }
    let snapshot = reader.0.read().unwrap().clone();
    assert!(
        domain::views(&records, Some(&snapshot), &revoked, Utc::now())
            .iter()
            .all(|e| e.validity == Validity::Current)
    );
    records[0].basis_evidence_ids = vec![records.last().unwrap().id.clone()];
    assert!(
        domain::views(&records, Some(&snapshot), &revoked, Utc::now())
            .iter()
            .all(|e| e.validity != Validity::Current)
    );
}

#[tokio::test]
async fn verification_closed_sample_selection_and_durable_run_id_replay() {
    let (service, reader) = fixture();
    let request = RunRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        checker: Checker::KitValidator,
        run_id: uuid::Uuid::new_v4().to_string(),
        fixture: Some(KitFixture::MissingField),
    };
    let first = service.run(request.clone()).await.unwrap();
    let record = &first.evidence[0];
    assert_eq!(record.id, request.run_id);
    assert_eq!(record.outcome, Outcome::Failed);
    assert_eq!(
        record.sample.as_ref().unwrap().code,
        SampleCode::InvalidInput
    );
    assert!(record.sample.as_ref().unwrap().matches_expectation);
    let restarted = Arc::new(VerificationService::new(service.db.clone(), reader.clone()));
    assert_eq!(
        restarted.run(request.clone()).await.unwrap().evidence.len(),
        1
    );
    let mut conflict = request.clone();
    conflict.fixture = Some(KitFixture::Baseline);
    assert_eq!(
        restarted.run(conflict).await.unwrap_err(),
        "run_id_conflict"
    );
    let mut conflict = request.clone();
    conflict.project_id = OTHER.into();
    assert_eq!(
        restarted.run(conflict).await.unwrap_err(),
        "run_id_conflict"
    );
    let mut invalid = request;
    invalid.checker = Checker::SavedModelProbe;
    assert_eq!(restarted.run(invalid).await.unwrap_err(), "invalid_request");
    reader.0.write().unwrap().project_revision = 2;
    assert_eq!(
        restarted.snapshot(PROJECT).unwrap().evidence[0].validity,
        Validity::Stale
    );
}

#[test]
fn verification_failed_customer_review_remains_a_failed_fact_not_acceptance() {
    let (service, _) = fixture();
    let mut request = manual(Stage::CustomerAccepted);
    request.outcome = Outcome::Failed;
    let snapshot = service.record_manual(request).unwrap();
    assert_eq!(snapshot.evidence[0].outcome, Outcome::Failed);
    assert_eq!(snapshot.evidence[0].validity, Validity::Current);
    assert!(!snapshot
        .evidence
        .iter()
        .any(|e| e.stage == Stage::CustomerAccepted && e.outcome == Outcome::Passed));
}

#[test]
fn verification_missing_credential_generation_keeps_external_record_unverifiable() {
    let (service, reader) = fixture();
    reader.0.write().unwrap().credential_generation = None;
    let snapshot = service
        .record_manual(manual(Stage::CustomerAccepted))
        .unwrap();
    assert_eq!(snapshot.evidence[0].source_class, SourceClass::ManualRecord);
    assert_eq!(snapshot.evidence[0].outcome, Outcome::Passed);
    assert_eq!(snapshot.evidence[0].validity, Validity::Unverifiable);
}

#[test]
fn verification_later_nonpass_supersedes_pass_and_customer_basis() {
    for outcome in [
        Outcome::Failed,
        Outcome::Unknown,
        Outcome::Unsupported,
        Outcome::Cancelled,
    ] {
        let (s, _) = fixture();
        s.record_manual(manual(Stage::ToolCallable)).unwrap();
        let (records, _) = s.db.verification_records(PROJECT).unwrap();
        let mut passed = records[0].clone();
        passed.id = uuid::Uuid::new_v4().to_string();
        passed.source_class = SourceClass::NativeRemote;
        passed.manual = None;
        passed.checker_id = "saved_model_probe".into();
        passed.reason_code = "model_identity_confirmed".into();
        s.db.verification_append(&passed).unwrap();
        let mut later = passed.clone();
        later.id = uuid::Uuid::new_v4().to_string();
        later.outcome = outcome;
        later.recorded_at = domain::stamp(Utc::now());
        later.reason_code = "model_request_failed".into();
        s.db.verification_append(&later).unwrap();
        let snap = s.snapshot(PROJECT).unwrap();
        assert_eq!(
            snap.evidence
                .iter()
                .find(|e| e.id == passed.id)
                .unwrap()
                .validity,
            Validity::Stale
        );
        let mut request = manual(Stage::CustomerAccepted);
        request.external_basis = None;
        request.basis_evidence_ids = vec![passed.id];
        assert_eq!(s.record_manual(request).unwrap_err(), "basis_not_current");
        assert!(s
            .preview(PROJECT)
            .unwrap()
            .markdown
            .contains("通过 / 待复核"));
    }
}

#[test]
fn verification_document_reference_is_bounded_https_not_secret_or_path() {
    let (s, _) = fixture();
    let mut request = manual(Stage::CustomerAccepted);
    request.external_basis.as_mut().unwrap().reference =
        "https://example.feishu.cn/docx/Abc123".into();
    s.record_manual(request.clone()).unwrap();
    assert!(s
        .preview(PROJECT)
        .unwrap()
        .markdown
        .contains("https://example.feishu.cn/docx/Abc123"));
    for bad in [
        "javascript:alert(1)",
        "/Users/private/report",
        "file:///tmp/report",
        "https://user:password@example.com/doc",
        "https://example.com/doc?token=private",
        "https://example.com/doc#private",
        "https://example.com/%73k-private",
    ] {
        request.external_basis.as_mut().unwrap().reference = bad.into();
        assert!(s.record_manual(request.clone()).is_err(), "{bad}");
    }
    request = manual(Stage::CustomerAccepted);
    request.person = "https://example.com/person".into();
    assert!(s.record_manual(request).is_err());
}

#[tokio::test]
async fn verification_sample_metrics_survive_reload_and_negative_fixture_is_independent() {
    let (s, reader) = fixture();
    let baseline = s
        .run(RunRequest {
            project_id: PROJECT.into(),
            expected_revision: 1,
            checker: Checker::KitValidator,
            run_id: uuid::Uuid::new_v4().to_string(),
            fixture: Some(KitFixture::Baseline),
        })
        .await
        .unwrap();
    let baseline_id = baseline.evidence[0].id.clone();
    s.run(RunRequest {
        project_id: PROJECT.into(),
        expected_revision: 1,
        checker: Checker::KitValidator,
        run_id: uuid::Uuid::new_v4().to_string(),
        fixture: Some(KitFixture::MissingField),
    })
    .await
    .unwrap();
    let restarted = VerificationService::new(s.db.clone(), reader);
    let preview = restarted.preview(PROJECT).unwrap();
    let baseline = preview
        .snapshot
        .evidence
        .iter()
        .find(|e| e.id == baseline_id)
        .unwrap();
    assert_eq!(baseline.validity, Validity::Current);
    assert_eq!(
        baseline
            .sample
            .as_ref()
            .unwrap()
            .metrics
            .as_ref()
            .unwrap()
            .growth_bps,
        2000
    );
    assert!(preview.json.contains("18000000"));
    assert!(preview.markdown.contains("增长：20.00%"));
    assert!(preview.markdown.contains("目标完成：90.00%"));
    assert!(preview.markdown.contains("来源行：row-1"));
    assert!(preview.markdown.contains("缺少必填字段或输入格式有误"));
}
