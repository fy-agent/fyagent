use super::*;
#[cfg(target_os = "macos")]
use evidence::SourceClass;
use evidence::{Checker, KitFixture, Outcome, RunRequest, Validity};
#[cfg(target_os = "macos")]
use projects::domain::BindKitRequest;
use projects::domain::ProjectMutation;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn fde_workspace_sample_binding_persistence_export_and_revision_invalidation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let db = Arc::new(Database::memory().unwrap());
    let workspace = compose(db.clone(), root.clone());
    let customer_a = workspace.projects.create_customer("客户甲").unwrap();
    let customer_b = workspace.projects.create_customer("客户乙").unwrap();
    let a = workspace
        .projects
        .create(&customer_a.customer_id, "经营周报")
        .unwrap();
    let b = workspace
        .projects
        .create(&customer_b.customer_id, "知识库")
        .unwrap();
    let context = workspace
        .projects
        .write_context(
            &ProjectMutation {
                project_id: a.project_id.clone(),
                expected_revision: a.project_revision,
            },
            "检查受控周报样例，保留问题和来源。",
        )
        .unwrap();
    let kit = workspace
        .kits
        .lock()
        .unwrap()
        .list()
        .unwrap()
        .into_iter()
        .find(|kit| kit.identity.kit_id.contains("weekly-report"))
        .unwrap()
        .identity;
    let bound = workspace
        .projects
        .bind_delivery_kit(&BindKitRequest {
            project_id: a.project_id.clone(),
            expected_revision: context.project_revision,
            kit_id: kit.kit_id.clone(),
            kit_version: kit.kit_version.clone(),
            manifest_digest: kit.manifest_digest.clone(),
            binding_intent_id: uuid::Uuid::new_v4().to_string(),
        })
        .unwrap();
    let baseline = RunRequest {
        project_id: a.project_id.clone(),
        expected_revision: bound.project_revision as u64,
        checker: Checker::KitValidator,
        fixture: Some(KitFixture::Baseline),
        run_id: uuid::Uuid::new_v4().to_string(),
    };
    let first = workspace.verification.run(baseline.clone()).await.unwrap();
    assert_eq!(first.evidence.len(), 1);
    let record = &first.evidence[0];
    assert_eq!(record.outcome, Outcome::Passed);
    assert_eq!(record.validity, Validity::Current);
    assert_eq!(record.source_class, SourceClass::LocalFixture);
    let serialized = serde_json::to_value(record).unwrap();
    assert_eq!(serialized["sample"]["metrics"]["growthBps"], 2000);
    assert_eq!(serialized["sample"]["metrics"]["targetBps"], 9000);
    assert!(!serialized["sample"]["sourceRowIds"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        workspace
            .verification
            .run(baseline.clone())
            .await
            .unwrap()
            .evidence
            .len(),
        1
    );
    let negative = RunRequest {
        fixture: Some(KitFixture::MissingField),
        run_id: uuid::Uuid::new_v4().to_string(),
        ..baseline
    };
    let second = workspace.verification.run(negative).await.unwrap();
    assert_eq!(second.evidence.len(), 2);
    assert!(second
        .evidence
        .iter()
        .any(|e| e.outcome == Outcome::Failed && e.validity == Validity::Current));
    assert!(second
        .evidence
        .iter()
        .any(|e| e.outcome == Outcome::Passed && e.validity == Validity::Current));
    assert!(workspace
        .verification
        .snapshot(&b.project_id)
        .unwrap()
        .evidence
        .is_empty());
    let reopened = compose(db, root.clone());
    assert_eq!(
        reopened
            .verification
            .snapshot(&a.project_id)
            .unwrap()
            .evidence
            .len(),
        2
    );
    let export_path = root.join("handoff.json");
    reopened
        .verification
        .export_to(&a.project_id, &export_path, false)
        .unwrap();
    let exported = std::fs::read_to_string(export_path).unwrap();
    assert!(exported.contains("local_fixture"));
    assert!(exported.contains("growthBps"));
    assert!(!exported.contains("credentialGeneration"));
    assert!(!exported.contains("resourceRevision"));
    assert!(!exported.contains(&root.to_string_lossy().to_string()));
    let updated = reopened
        .projects
        .update(
            &ProjectMutation {
                project_id: a.project_id.clone(),
                expected_revision: bound.project_revision,
            },
            "经营周报二版",
            false,
        )
        .unwrap();
    assert!(updated.project_revision > bound.project_revision);
    let after = reopened.verification.snapshot(&a.project_id).unwrap();
    assert!(after.evidence.iter().all(|e| e.validity == Validity::Stale));
    assert!(reopened
        .verification
        .snapshot(&b.project_id)
        .unwrap()
        .evidence
        .is_empty());
}

#[cfg(target_os = "windows")]
#[test]
fn fde_workspace_windows_context_write_reports_unavailable_without_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let workspace = compose(Arc::new(Database::memory().unwrap()), root.clone());
    let customer = workspace
        .projects
        .create_customer("Windows fixture")
        .unwrap();
    let project = workspace
        .projects
        .create(&customer.customer_id, "Unavailable context")
        .unwrap();
    let error = workspace
        .projects
        .write_context(
            &ProjectMutation {
                project_id: project.project_id.clone(),
                expected_revision: project.project_revision,
            },
            "This context must not be published on Windows.",
        )
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::InvalidInput(code) if code == "projects_platform_unavailable"
    ));
    assert_eq!(
        workspace.projects.get(&project.project_id).unwrap(),
        project
    );
    let context = workspace.projects.context(&project.project_id).unwrap();
    assert_eq!(context.state, ContextState::NotCreated);
    assert_eq!(context.project_revision, project.project_revision);
    assert!(context.content.is_empty());
    assert!(context.directory.is_none());
    assert!(context.codex_instructions.is_none());
    assert!(!root.join("projects").exists());
}

#[tokio::test]
async fn fde_workspace_unbound_package_cannot_create_a_pass() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = compose(
        Arc::new(Database::memory().unwrap()),
        temp.path().canonicalize().unwrap(),
    );
    let customer = workspace.projects.create_customer("新客户").unwrap();
    let project = workspace
        .projects
        .create(&customer.customer_id, "待配置")
        .unwrap();
    let result = workspace
        .verification
        .run(RunRequest {
            project_id: project.project_id,
            expected_revision: 0,
            checker: Checker::KitValidator,
            fixture: Some(KitFixture::Baseline),
            run_id: uuid::Uuid::new_v4().to_string(),
        })
        .await
        .unwrap();
    assert_eq!(result.evidence[0].outcome, Outcome::Unsupported);
}

#[tokio::test]
async fn release_integration_fde_saved_probe_rejects_subscription_sources_before_transport() {
    use crate::provider::{Provider, ProviderMeta};
    use evidence::ProjectDependencyReader;
    use serde_json::json;

    // A real API-key probe would connect to this listener. No subscription
    // source may issue that request or report a vendor/network failure.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    for source in ["codex_oauth", "xai_oauth"] {
        for app in ["claude", "codex", "grokbuild", "opencode"] {
            let temp = tempfile::tempdir().unwrap();
            let db = Arc::new(Database::memory().unwrap());
            let workspace = compose(db.clone(), temp.path().canonicalize().unwrap());
            let settings = match app {
                "claude" => {
                    json!({"env":{"ANTHROPIC_BASE_URL":endpoint,"ANTHROPIC_MODEL":"fixture","ANTHROPIC_API_KEY":"PROXY_MANAGED"}})
                }
                "codex" => {
                    json!({"auth":{"OPENAI_API_KEY":"PROXY_MANAGED"},"config":format!("model_provider='fixture'\nmodel='fixture'\n[model_providers.fixture]\nbase_url='{endpoint}'\nwire_api='responses'\n")})
                }
                "grokbuild" => {
                    json!({"config":format!("[models]\ndefault='fixture'\n[model.fixture]\nmodel='fixture'\nbase_url='{endpoint}'\napi_key='PROXY_MANAGED'\napi_backend='responses'\ncontext_window=128000\n")})
                }
                _ => {
                    json!({"npm":"@ai-sdk/openai","options":{"baseURL":endpoint,"apiKey":"PROXY_MANAGED"},"models":{"fixture":{}}})
                }
            };
            let mut provider =
                Provider::with_id("subscription".into(), "Subscription".into(), settings, None);
            provider.meta = Some(ProviderMeta {
                provider_type: Some(source.into()),
                ..Default::default()
            });
            db.save_provider(app, &provider).unwrap();
            let customer = workspace
                .projects
                .create_customer("Fixture customer")
                .unwrap();
            let project = workspace
                .projects
                .create(&customer.customer_id, "Subscription check")
                .unwrap();
            let bound = workspace
                .projects
                .bind_resource(
                    &ProjectMutation {
                        project_id: project.project_id.clone(),
                        expected_revision: project.project_revision,
                    },
                    ResourceKind::Provider,
                    app,
                    &provider.id,
                    Some("fixture".into()),
                )
                .unwrap();
            let reader = DependencyReader {
                db,
                projects: workspace.projects.clone(),
                kits: workspace.kits.clone(),
            };
            assert_eq!(
                reader.saved_model(&project.project_id).err(),
                Some("saved_model_unavailable"),
                "{source}/{app}"
            );
            let result = workspace
                .verification
                .run(RunRequest {
                    project_id: project.project_id,
                    expected_revision: bound.project_revision as u64,
                    checker: Checker::SavedModelProbe,
                    fixture: None,
                    run_id: uuid::Uuid::new_v4().to_string(),
                })
                .await
                .unwrap();
            assert_eq!(result.evidence.len(), 1);
            let record = &result.evidence[0];
            assert_eq!(record.outcome, Outcome::Unsupported, "{source}/{app}");
            assert_eq!(record.reason_code, "saved_model_unavailable");
            assert_eq!(record.validity, Validity::Current);
            let wire = serde_json::to_string(record).unwrap();
            assert!(!wire.contains("PROXY_MANAGED"));
            assert!(!wire.contains(&endpoint));
        }
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
