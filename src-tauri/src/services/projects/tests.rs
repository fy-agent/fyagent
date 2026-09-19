use super::*;

fn fixture() -> (tempfile::TempDir, ProjectsService) {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().canonicalize().unwrap().join("projects");
    let db = Arc::new(Database::memory().unwrap());
    (
        t,
        ProjectsService::new(db, root, Arc::new(UnavailableKitReader)),
    )
}
fn request(p: &Project) -> ProjectMutation {
    ProjectMutation {
        project_id: p.project_id.clone(),
        expected_revision: p.project_revision,
    }
}

#[test]
fn projects_crud_cas_archive_preserves_global_state() {
    let (_t, s) = fixture();
    s.db.set_setting("current_profile_id_codex", "existing")
        .unwrap();
    let c = s.create_customer("客户 A").unwrap();
    let p = s.create(&c.customer_id, "周报").unwrap();
    assert!(s
        .update_customer(&c.customer_id, c.revision, &c.name, true)
        .is_err());
    let q = s.update(&request(&p), "改名", false).unwrap();
    assert_eq!(q.project_revision, 1);
    assert!(s.update(&request(&p), "过期", false).is_err());
    let a = s.update(&request(&q), "改名", true).unwrap();
    assert!(a.archived);
    assert!(s.write_context(&request(&a), "禁止").is_err());
    assert_eq!(
        s.db.get_setting("current_profile_id_codex")
            .unwrap()
            .as_deref(),
        Some("existing")
    );
    assert!(
        s.update_customer(&c.customer_id, c.revision, &c.name, true)
            .unwrap()
            .archived
    );
}

#[cfg(target_os = "macos")]
#[test]
fn projects_context_ab_isolation_and_stale_write() {
    let (_t, s) = fixture();
    let c = s.create_customer("A").unwrap();
    let d = s.create_customer("B").unwrap();
    let a = s.create(&c.customer_id, "同名").unwrap();
    let b = s.create(&d.customer_id, "同名").unwrap();
    let ca = s.write_context(&request(&a), "客户 A 私有资料").unwrap();
    let cb = s.write_context(&request(&b), "客户 B 私有资料").unwrap();
    assert_ne!(ca.directory, cb.directory);
    assert!(s.write_context(&request(&a), "过期覆盖").is_err());
    assert_eq!(s.context(&a.project_id).unwrap().content, "客户 A 私有资料");
    assert_eq!(s.context(&b.project_id).unwrap().content, "客户 B 私有资料");
    assert!(
        !s.dependency_snapshot(&a.project_id)
            .unwrap()
            .runtime_available
    );
    assert!(s.context("../escape").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn projects_context_rejects_symlink_parent_and_leaf() {
    use std::os::unix::fs::symlink;
    let (t, s) = fixture();
    let c = s.create_customer("A").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    std::fs::create_dir_all(&s.root).unwrap();
    symlink(t.path(), s.root.join(&p.project_id)).unwrap();
    assert!(s.write_context(&request(&p), "escape").is_err());
    std::fs::remove_file(s.root.join(&p.project_id)).unwrap();
    let ctx = s.write_context(&request(&p), "safe").unwrap();
    let path = std::path::Path::new(ctx.directory.as_ref().unwrap()).join("context.md");
    std::fs::remove_file(&path).unwrap();
    symlink("/etc/hosts", path).unwrap();
    assert!(s.context(&p.project_id).is_err());
}

struct KitFixture;
impl ProjectKitReader for KitFixture {
    fn confirm(&self, k: &KitBinding) -> Result<(), AppError> {
        if k.kit_id == "weekly" && k.kit_version == "1.0.0" && k.manifest_digest == "a".repeat(64) {
            Ok(())
        } else {
            Err(project_error("kit_missing"))
        }
    }
}
#[test]
fn projects_kit_native_confirmation_atomic_cas_and_intent_replay() {
    let (_t, mut s) = fixture();
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    let mut r = BindKitRequest {
        project_id: p.project_id.clone(),
        expected_revision: 0,
        kit_id: "weekly".into(),
        kit_version: "1.0.0".into(),
        manifest_digest: "a".repeat(64),
        binding_intent_id: uuid::Uuid::new_v4().to_string(),
    };
    assert!(s.bind_delivery_kit(&r).is_err());
    assert_eq!(s.get(&p.project_id).unwrap(), p);
    s.kits = Arc::new(KitFixture);
    let bound = s.bind_delivery_kit(&r).unwrap();
    assert_eq!(s.bind_delivery_kit(&r).unwrap(), bound);
    r.kit_version = "2.0.0".into();
    assert!(s.bind_delivery_kit(&r).is_err());
    r.kit_version = "1.0.0".into();
    r.binding_intent_id = uuid::Uuid::new_v4().to_string();
    assert!(s.bind_delivery_kit(&r).is_err());
    assert_eq!(s.get(&p.project_id).unwrap().project_revision, 1);
}

#[test]
fn projects_bindings_reject_renderer_claims_and_snapshot_is_read_only() {
    let (_t, s) = fixture();
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    assert!(s
        .bind_resource(
            &request(&p),
            ResourceKind::Provider,
            "codex",
            "invented",
            None
        )
        .is_err());
    assert!(s
        .bind_credential(&request(&p), "fake", "proxy_upstream", "fyagent_proxy")
        .is_err());
    let before = s.db.export_sql_string().unwrap();
    s.dependency_snapshot(&p.project_id).unwrap();
    assert_eq!(s.db.export_sql_string().unwrap(), before);
}

#[test]
fn projects_credentials_follow_owner_generation_and_revocation_without_secrets() {
    use crate::services::secret::{SecretRef, SecretVersion};
    let (_t, s) = fixture();
    let secret_ref = SecretRef::generate();
    let secret_version = SecretVersion::generate();
    {
        let conn = s.db.conn.lock().unwrap();
        conn.execute_batch("INSERT INTO managed_auth_identities(identity_id,provider,provider_subject,login,created_at,updated_at) VALUES('fixture-identity','openai','fixture','Fixture account',1,1);").unwrap();
        conn.execute("INSERT INTO managed_auth_credentials(credential_id,identity_id,provider,purpose,consumer,legacy_account_id,secret_ref,secret_version,refresh_owner,generation,status,authenticated_at,created_at,updated_at) VALUES('fixture-credential','fixture-identity','openai','proxy_upstream','fyagent_proxy','fixture',?1,?2,'fyagent',1,'ready',1,1,1)", rusqlite::params![secret_ref.as_str(),secret_version.as_str()]).unwrap();
    }
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    assert!(s
        .bind_credential(&request(&p), "fixture-credential", "codex_native", "codex")
        .is_err());
    let p = s
        .bind_credential(
            &request(&p),
            "fixture-credential",
            "proxy_upstream",
            "fyagent_proxy",
        )
        .unwrap();
    let snapshot = s.dependency_snapshot(&p.project_id).unwrap();
    assert_eq!(
        snapshot.credentials[0].state,
        ObservationState::Unverifiable
    );
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains(secret_ref.as_str()));
    assert!(!json.contains(secret_version.as_str()));
    assert!(!json.contains("secretRef"));
    s.db.conn
        .lock()
        .unwrap()
        .execute_batch("UPDATE managed_auth_credentials SET generation=2;")
        .unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().credentials[0].state,
        ObservationState::Drifted
    );
    s.db.conn
        .lock()
        .unwrap()
        .execute_batch("UPDATE managed_auth_credentials SET status='revoked';")
        .unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().credentials[0].state,
        ObservationState::Revoked
    );
    assert!(s
        .bind_credential(
            &request(&p),
            "fixture-credential",
            "proxy_upstream",
            "fyagent_proxy"
        )
        .is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn projects_concurrent_context_publication_has_one_winner() {
    let (_t, s) = fixture();
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            s.write_context(&request(&p), "A")
        });
        let b = scope.spawn(|| {
            barrier.wait();
            s.write_context(&request(&p), "B")
        });
        assert_ne!(a.join().unwrap().is_ok(), b.join().unwrap().is_ok());
    });
    assert_eq!(s.get(&p.project_id).unwrap().project_revision, 1);
    assert!(["A", "B"].contains(&s.context(&p.project_id).unwrap().content.as_str()));
}

#[test]
fn projects_migration_fresh_predecessor_rollback_and_future() {
    use rusqlite::Connection;
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE profiles(id TEXT PRIMARY KEY,payload TEXT); INSERT INTO profiles VALUES ('legacy','{original}'); PRAGMA user_version=21;").unwrap();
    Database::apply_schema_migrations_on_conn(&c).unwrap();
    assert_eq!(
        Database::get_user_version(&c).unwrap(),
        crate::database::SCHEMA_VERSION
    );
    assert_eq!(
        c.query_row("SELECT payload FROM profiles", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "{original}"
    );
    Database::apply_schema_migrations_on_conn(&c).unwrap();
    let broken = Connection::open_in_memory().unwrap();
    broken
        .execute_batch("PRAGMA user_version=21; CREATE TABLE fde_projects(wrong TEXT);")
        .unwrap();
    assert!(Database::apply_schema_migrations_on_conn(&broken).is_err());
    assert_eq!(Database::get_user_version(&broken).unwrap(), 21);
    assert_eq!(
        broken
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='fde_customers'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    c.execute_batch("PRAGMA user_version=999;").unwrap();
    assert!(Database::apply_schema_migrations_on_conn(&c).is_err());
    assert_eq!(Database::get_user_version(&c).unwrap(), 999);
}

#[test]
fn projects_resource_missing_and_unverifiable_content_versions() {
    let (_t, s) = fixture();
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    let prompt = crate::prompt::Prompt {
        id: "source".into(),
        name: "说明".into(),
        content: "may contain sensitive input".into(),
        description: None,
        enabled: false,
        created_at: None,
        updated_at: None,
    };
    s.db.save_prompt("codex", &prompt).unwrap();
    let p = s
        .bind_resource(&request(&p), ResourceKind::Prompt, "codex", "source", None)
        .unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
        ObservationState::Unverifiable
    );
    s.db.delete_prompt("codex", "source").unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
        ObservationState::Missing
    );
    let replacement = crate::prompt::Prompt {
        id: "different-id".into(),
        ..prompt
    };
    s.db.save_prompt("codex", &replacement).unwrap();
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().resources[0].state,
        ObservationState::Missing
    );
}

#[test]
fn projects_kit_late_failure_rolls_back_project_revision() {
    let (_t, mut s) = fixture();
    s.kits = Arc::new(KitFixture);
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    s.db.conn.lock().unwrap().execute_batch("CREATE TRIGGER fail_intent BEFORE INSERT ON fde_project_kit_intents BEGIN SELECT RAISE(ABORT,'fixture late failure'); END;").unwrap();
    let r = BindKitRequest {
        project_id: p.project_id.clone(),
        expected_revision: 0,
        kit_id: "weekly".into(),
        kit_version: "1.0.0".into(),
        manifest_digest: "a".repeat(64),
        binding_intent_id: uuid::Uuid::new_v4().to_string(),
    };
    assert!(s.bind_delivery_kit(&r).is_err());
    assert_eq!(s.get(&p.project_id).unwrap(), p);
}

#[cfg(target_os = "macos")]
#[test]
fn projects_context_external_change_is_not_current_evidence() {
    let (_t, s) = fixture();
    let c = s.create_customer("C").unwrap();
    let p = s.create(&c.customer_id, "P").unwrap();
    let ctx = s.write_context(&request(&p), "original").unwrap();
    std::fs::write(
        std::path::Path::new(ctx.directory.as_ref().unwrap()).join("context.md"),
        "changed externally",
    )
    .unwrap();
    assert!(s.context(&p.project_id).is_err());
    assert_eq!(
        s.dependency_snapshot(&p.project_id).unwrap().context_state,
        ContextState::Unavailable
    );
}

#[test]
fn projects_sync_export_excludes_local_rows() {
    let (_t, s) = fixture();
    let c = s.create_customer("Private customer").unwrap();
    s.create(&c.customer_id, "private project").unwrap();
    let sql = s.db.export_sql_string_for_sync().unwrap();
    assert!(!sql.contains("Private customer"));
    assert!(!sql.contains("private project"));
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&sql).unwrap();
    for table in [
        "fde_customers",
        "fde_projects",
        "fde_project_kit_intents",
        "fde_project_context_versions",
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn projects_codex_preparation_is_explicit_scoped_and_readback_checked() {
    let (_t, s) = fixture();
    let c = s.create_customer("C").unwrap();
    let a = s.create(&c.customer_id, "A").unwrap();
    let b = s.create(&c.customer_id, "B").unwrap();
    assert!(s.prepare_codex(&request(&a)).is_err());
    s.write_context(&request(&a), "approved context A").unwrap();
    let a = s.get(&a.project_id).unwrap();
    let ctx = s.prepare_codex(&request(&a)).unwrap();
    assert!(ctx.codex_instructions.as_ref().unwrap().contains("env -i"));
    let dir = std::path::Path::new(ctx.directory.as_ref().unwrap());
    assert_eq!(
        std::fs::read_to_string(dir.join("workspace/AGENTS.md")).unwrap(),
        "approved context A"
    );
    assert!(!dir.join("home/.codex/auth.json").exists());
    assert!(s.get(&b.project_id).unwrap().context_generation.is_none());
    assert!(
        !s.dependency_snapshot(&a.project_id)
            .unwrap()
            .runtime_available
    );
    std::fs::write(dir.join("home/.codex/project.config.toml"), "modified").unwrap();
    assert!(s.context(&a.project_id).is_err());
}
