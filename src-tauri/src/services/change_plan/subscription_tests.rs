//! Knife 2 Change Plan admission and replay tests; no upstream traffic.
use super::*;
use crate::services::workbuddy::subscription::plan_tests::Fixture;
use serial_test::serial;

fn subscription_plan_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .state
        .db
        .create_change_plan_tables_for_tests()
        .unwrap();
    fixture
}

fn request(account_id: &str) -> SaveWorkBuddySubscriptionRequest {
    SaveWorkBuddySubscriptionRequest {
        account_id: account_id.into(),
        model_id: "probe-model".into(),
        expected_revision: None,
        expected_binding_version: None,
    }
}

fn apply(state: &AppState, plan: &ChangePlan, now: i64) -> ApplyChangePlanOutcome {
    ChangePlanService::apply_workbuddy_subscription_inner(
        state,
        CodexApplyIdentity {
            plan_id: &plan.plan_id,
            plan_digest: &plan.plan_digest,
            now,
        },
        |_| {},
        None,
    )
    .unwrap()
}

#[test]
#[serial]
fn workbuddy_subscription_plan_preview_is_readonly_and_secret_free() {
    let fixture = subscription_plan_fixture();
    let state = &fixture.state;
    let path = fixture.home.path().join(".workbuddy/models.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = br#"{"models":[{"id":"untouched","url":"https://example.test/v1","apiKey":"sentinel-file-secret"}],"unknown":7}"#;
    std::fs::write(&path, original).unwrap();
    let revision = subscription::read_subscription_source(state)
        .unwrap()
        .revision;
    let mut input = request(&fixture.account_id);
    input.expected_revision = revision;
    let plan = ChangePlanService::plan_workbuddy_subscription(state, input.clone()).unwrap();
    let repeat = ChangePlanService::plan_workbuddy_subscription(state, input).unwrap();
    assert_ne!(plan.plan_id, repeat.plan_id);
    assert_eq!(plan.plan_digest, repeat.plan_digest);
    assert_eq!(plan.operation, ChangeOperation::WorkbuddySubscriptionSave);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(!path.with_file_name("models.json.backup").exists());
    assert!(state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
    assert!(!tauri::async_runtime::block_on(
        state.proxy_service.is_running()
    ));
    let public = serde_json::to_string(&plan).unwrap();
    let persisted: String = state.db.conn.lock().unwrap().query_row(
        "SELECT plan_id || operation || target_provider_id || target_provider_name || plan_digest || baseline_digest || target_definition_digest || live_baseline_digest || target_projection_digest || contract_digest FROM change_plans WHERE plan_id = ?1",
        rusqlite::params![plan.plan_id],
        |row| row.get(0),
    ).unwrap();
    for secret in [
        "synthetic-access",
        "synthetic-refresh",
        "sentinel-file-secret",
        "apiKey",
        "refresh_token",
    ] {
        assert!(!public.contains(secret), "public plan contains {secret}");
        assert!(
            !persisted.contains(secret),
            "persisted plan contains {secret}"
        );
    }
}

#[test]
#[serial]
fn workbuddy_subscription_plan_expired_is_rejected_without_write() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let outcome = apply(&fixture.state, &plan, plan.expires_at + 1);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Expired));
    assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_missing_process_draft_is_stale() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    subscription_drafts().lock().unwrap().remove(&plan.plan_id);
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
    assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_external_edit_is_stale_and_preserved() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let path = fixture.home.path().join(".workbuddy/models.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let external =
        br#"{"models":[{"id":"external","url":"https://external.test/v1"}],"external":true}"#;
    std::fs::write(&path, external).unwrap();
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
    assert_eq!(std::fs::read(&path).unwrap(), external);
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_confirm_replays_without_rewriting_and_reads_source() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Admitted);
    let job = outcome.job.unwrap();
    assert_eq!(job.status, ChangeJobStatus::Succeeded);
    let observed = subscription::read_subscription_source(&fixture.state).unwrap();
    assert!(observed.revision.is_some());
    let source = observed.source.unwrap();
    assert_eq!(source.account_id, fixture.account_id);
    assert_eq!(source.model_id, "probe-model");
    assert_eq!(source.provider, "openai");
    assert_eq!(source.binding_version, 1);
    assert!(source.local_base_url.ends_with("/workbuddy/v1"));
    let path = fixture.home.path().join(".workbuddy/models.json");
    let bytes = std::fs::read(&path).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let binding = fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap();
    let replay = apply(&fixture.state, &plan, plan.created_at + 1);
    assert_eq!(replay.kind, ChangeApplyOutcomeKind::IdempotentReplay);
    assert_eq!(replay.job.unwrap().job_id, job.job_id);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(
        fixture
            .state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap(),
        binding
    );
}

#[test]
#[serial]
fn workbuddy_subscription_plan_binding_version_change_is_stale() {
    let fixture = subscription_plan_fixture();
    let first = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    assert_eq!(
        apply(&fixture.state, &first, first.created_at)
            .job
            .unwrap()
            .status,
        ChangeJobStatus::Succeeded
    );
    let observed = subscription::read_subscription_source(&fixture.state).unwrap();
    let mut input = request(&fixture.account_id);
    input.expected_revision = observed.revision;
    input.expected_binding_version = Some(observed.source.unwrap().binding_version);
    let plan = ChangePlanService::plan_workbuddy_subscription(&fixture.state, input).unwrap();
    let mut binding: serde_json::Value = serde_json::from_str(
        &fixture
            .state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    binding["version"] = serde_json::json!(2);
    let changed = serde_json::to_string(&binding).unwrap();
    fixture
        .state
        .db
        .set_workbuddy_subscription_binding(&changed)
        .unwrap();
    let path = fixture.home.path().join(".workbuddy/models.json");
    let before = std::fs::read(&path).unwrap();
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(
        fixture
            .state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap(),
        Some(changed)
    );
}

#[test]
#[serial]
fn workbuddy_subscription_plan_account_loses_admission_is_stale() {
    use crate::services::managed_auth::{
        stable_credential_id, CredentialPurpose, CredentialStatus, ManagedAuthConsumer,
        ManagedAuthProvider,
    };
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let credential_id = stable_credential_id(
        ManagedAuthProvider::Openai,
        CredentialPurpose::ProxyUpstream,
        Some(ManagedAuthConsumer::FyagentProxy),
        "subscription-plan",
    );
    subscription::workbuddy_subscription_auth_for_test()
        .unwrap()
        .repository
        .set_status(
            &credential_id,
            CredentialStatus::RequiresReauth,
            plan.created_at,
        )
        .unwrap();
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::Stale));
    assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_wrong_confirmation_digest_does_not_write() {
    let fixture = subscription_plan_fixture();
    let mut plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    plan.plan_digest = "not-the-confirmed-digest".into();
    let outcome = apply(&fixture.state, &plan, plan.created_at);
    assert_eq!(outcome.kind, ChangeApplyOutcomeKind::Rejected);
    assert_eq!(outcome.error_code, Some(ChangePlanErrorCode::InvalidDigest));
    assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
    assert!(fixture
        .state
        .db
        .get_change_job_by_plan_id(&plan.plan_id)
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_interruption_before_write_recovers_without_writing() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let result = ChangePlanService::apply_workbuddy_subscription_inner(
        &fixture.state,
        CodexApplyIdentity {
            plan_id: &plan.plan_id,
            plan_digest: &plan.plan_digest,
            now: plan.created_at,
        },
        |_| {},
        Some(ChangeFaultPoint::BeforeManagedWrite),
    );
    assert_eq!(result, Err(ChangePlanErrorCode::Internal));
    let persisted = fixture
        .state
        .db
        .get_change_job_by_plan_id(&plan.plan_id)
        .unwrap()
        .unwrap();
    let recovered = ChangePlanService::get_job(&fixture.state, &persisted.job_id).unwrap();
    assert_eq!(recovered.status, ChangeJobStatus::Failed);
    assert_eq!(
        recovered.result_code,
        ChangeResultCode::InterruptedBeforeWrite
    );
    assert_eq!(recovered.recovery_state, RecoveryState::Succeeded);
    assert!(!fixture.home.path().join(".workbuddy/models.json").exists());
    assert!(fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

#[test]
#[serial]
fn workbuddy_subscription_plan_interruption_after_write_remains_unconfirmed() {
    let fixture = subscription_plan_fixture();
    let plan = ChangePlanService::plan_workbuddy_subscription(
        &fixture.state,
        request(&fixture.account_id),
    )
    .unwrap();
    let result = ChangePlanService::apply_workbuddy_subscription_inner(
        &fixture.state,
        CodexApplyIdentity {
            plan_id: &plan.plan_id,
            plan_digest: &plan.plan_digest,
            now: plan.created_at,
        },
        |_| {},
        Some(ChangeFaultPoint::AfterManagedWriteBeforeRecord),
    );
    assert_eq!(result, Err(ChangePlanErrorCode::Internal));
    assert!(!subscription_drafts()
        .lock()
        .unwrap()
        .contains_key(&plan.plan_id));
    let path = fixture.home.path().join(".workbuddy/models.json");
    let written = std::fs::read(&path).unwrap();
    assert!(subscription::read_subscription_source(&fixture.state)
        .unwrap()
        .source
        .is_some());
    let binding = fixture
        .state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap();
    let persisted = fixture
        .state
        .db
        .get_change_job_by_plan_id(&plan.plan_id)
        .unwrap()
        .unwrap();
    let recovered = ChangePlanService::get_job(&fixture.state, &persisted.job_id).unwrap();
    assert_eq!(recovered.status, ChangeJobStatus::Failed);
    assert_eq!(recovered.result_code, ChangeResultCode::PostWriteMismatch);
    assert_eq!(recovered.recovery_state, RecoveryState::RecoveryRequired);
    let replay = apply(&fixture.state, &plan, plan.created_at + 1);
    assert_eq!(replay.kind, ChangeApplyOutcomeKind::IdempotentReplay);
    assert_eq!(replay.job.unwrap().status, ChangeJobStatus::Failed);
    assert_eq!(std::fs::read(path).unwrap(), written);
    assert_eq!(
        fixture
            .state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap(),
        binding
    );
}

#[test]
#[serial]
fn workbuddy_subscription_same_model_wrong_route_or_account_never_confirms() {
    let fixture = subscription_plan_fixture();
    let state = &fixture.state;
    let plan = ChangePlanService::plan_workbuddy_subscription(state, request(&fixture.account_id))
        .unwrap();
    let mut job = apply(state, &plan, plan.created_at).job.unwrap();
    assert_eq!(job.status, ChangeJobStatus::Succeeded);
    let stored = state
        .db
        .get_stored_change_plan(&plan.plan_id)
        .unwrap()
        .unwrap();
    let path = fixture.home.path().join(".workbuddy/models.json");
    let original = std::fs::read(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(document["models"][0]["id"], "probe-model");
    document["models"][0]["url"] = json!("http://127.0.0.1:15721/v1");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    classify_subscription_job(state, &stored, &mut job, true, plan.created_at + 1);
    assert_eq!(job.status, ChangeJobStatus::Failed);
    assert_eq!(job.recovery_state, RecoveryState::RecoveryRequired);
    std::fs::write(&path, original).unwrap();
    let raw = state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .unwrap();
    let mut binding: serde_json::Value = serde_json::from_str(&raw).unwrap();
    binding["accountId"] = json!("ma1:00000000000000000000000000000000");
    state
        .db
        .set_workbuddy_subscription_binding(&binding.to_string())
        .unwrap();
    classify_subscription_job(state, &stored, &mut job, true, plan.created_at + 2);
    assert_eq!(job.status, ChangeJobStatus::Failed);
    assert_eq!(job.recovery_state, RecoveryState::RecoveryRequired);
}
