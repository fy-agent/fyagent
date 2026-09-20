use super::*;
use crate::provider::sanitize_provider_for_export;
use crate::services::secret::{
    MemoryFailureMode, MemorySecretBackend, SecretBackend, SecretService,
};

const CANARY: &str = "fixture-provider-secret-canary-2026";

fn fixture() -> Provider {
    Provider::with_id(
        "fixture-codex".into(),
        "Fixture".into(),
        json!({
            "auth": {"OPENAI_API_KEY": CANARY},
            "config": format!("# keep this comment\nmodel_provider = 'custom'\nmodel = 'fixture-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\nexperimental_bearer_token = '{CANARY}'\n")
        }),
        None,
    )
}

fn database() -> (Database, MemorySecretBackend) {
    let mut db = Database::memory().unwrap();
    let backend = MemorySecretBackend::new();
    db.provider_secrets = SecretService::new(Box::new(backend.clone()));
    (db, backend)
}

#[test]
fn save_uses_reference_and_native_resolution_without_dto_or_debug_leak() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert!(binding(&stored).unwrap().starts_with("pc_"));
    assert!(!serde_json::to_string(&stored).unwrap().contains(CANARY));
    assert!(!format!("{:?}", fixture()).contains(CANARY));
    let native = ProviderCredentials::resolve(&db, "codex", &stored).unwrap();
    assert_eq!(native.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
    let public = sanitize_provider_for_export(&native);
    let public = serde_json::to_string(&public).unwrap();
    assert!(!public.contains(CANARY));
    assert!(!public.contains("credentialRef"));
    assert!(stored.settings_config["config"]
        .as_str()
        .unwrap()
        .contains("# keep this comment"));
}

#[test]
fn blank_and_masked_edits_retain_the_same_credential() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let original = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    for mask in ["", "********", "••••••", "[REDACTED]"] {
        let mut edit = sanitize_provider_for_export(&original);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!(mask);
        edit.name = "Edited".into();
        let merged = ProviderCredentials::merge_edit(&db, "codex", &edit).unwrap();
        assert_eq!(merged.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
        db.save_provider("codex", &merged).unwrap();
        let saved = db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert_eq!(binding(&saved), binding(&original));
        assert_eq!(saved.name, "Edited");
    }
}

#[test]
fn rotation_retains_old_key_for_rollback_then_revokes_after_commit() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let original = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let old = ProviderCredentials::record(&db, &original).unwrap();
    let mut replacement = fixture();
    replacement.settings_config["config"] = json!("model = 'fixture-model'");
    replacement.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-new-secret");
    db.save_provider("codex", &replacement).unwrap();
    let updated = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert_ne!(binding(&original), binding(&updated));
    assert!(backend.contains(old.handle.secret_ref()));
    db.save_provider_record("codex", &original).unwrap();
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &original)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
    db.save_provider_record("codex", &updated).unwrap();
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(!backend.contains(old.handle.secret_ref()));
    assert!(ProviderCredentials::resolve(&db, "codex", &original)
        .unwrap_err()
        .to_string()
        .contains("revoked"));
}

#[test]
fn locked_missing_and_foreign_refs_never_fall_back_to_inline_material() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let mut stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &stored).unwrap();
    stored.settings_config["auth"]["OPENAI_API_KEY"] = json!("do-not-fall-back");
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("locked"));
    backend.set_mode(MemoryFailureMode::Healthy);
    backend.delete(record.handle.secret_ref()).unwrap();
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("missing"));
    stored.id = "another-provider".into();
    assert!(ProviderCredentials::resolve(&db, "codex", &stored).is_err());
    assert!(db.save_provider("codex", &stored).is_err());
}

#[test]
fn migration_failure_preserves_legacy_bytes_then_retries_idempotently() {
    let (db, backend) = database();
    let legacy = fixture();
    db.save_provider_record("codex", &legacy).unwrap();
    backend.set_mode(MemoryFailureMode::Locked);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    let untouched = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert_eq!(untouched.settings_config, legacy.settings_config);
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &untouched)
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
    backend.set_mode(MemoryFailureMode::Healthy);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 1);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    let migrated = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert!(!migrated.settings_config.to_string().contains(CANARY));
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &migrated)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn failed_database_commit_does_not_destroy_previous_key_or_config() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let previous = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    db.conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_provider_edit BEFORE UPDATE ON providers WHEN NEW.name = 'Reject' BEGIN SELECT RAISE(ABORT, 'fixture-private-error'); END;").unwrap();
    let mut replacement = fixture();
    replacement.name = "Reject".into();
    replacement.settings_config["config"] = json!("model = 'fixture-model'");
    replacement.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-replacement-secret");
    let error = db
        .save_provider("codex", &replacement)
        .unwrap_err()
        .to_string();
    assert!(!error.contains("fixture-private-error"));
    let restored = db
        .get_provider_by_id(&previous.id, "codex")
        .unwrap()
        .unwrap();
    assert_eq!(restored.settings_config, previous.settings_config);
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &restored)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn deletion_revokes_locally_even_when_backend_cleanup_must_retry() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &stored).unwrap();
    db.delete_provider("codex", &stored.id).unwrap();
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(ProviderCredentials::cleanup(&db).is_err());
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("revoked"));
    backend.set_mode(MemoryFailureMode::Healthy);
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(!backend.contains(record.handle.secret_ref()));
}

#[test]
fn ordinary_and_sync_exports_scrub_legacy_and_current_provider_secrets() {
    let (db, _) = database();
    db.save_provider_record("codex", &fixture()).unwrap();
    for exported in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!exported.contains(CANARY));
        assert!(db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap()
            .settings_config
            .to_string()
            .contains(CANARY));
    }
    ProviderCredentials::migrate_legacy(&db).unwrap();
    let record = db.provider_credential_records().unwrap().remove(0);
    for exported in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!exported.contains(CANARY));
        assert!(!exported.contains(record.handle.secret_ref().as_str()));
        assert!(!exported.contains(&record.id));
    }
}

#[test]
#[serial_test::serial]
fn provider_credentials_native_create_edit_switch_and_delete_round_trip() {
    use crate::app_config::AppType;
    use crate::services::ProviderService;
    super::super::tests::with_test_home(|state, _home| {
        crate::settings::set_current_provider(&AppType::Codex, None).unwrap();
        let auth_path = crate::codex_config::get_codex_auth_path();
        std::fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        let auth_bytes = b"fixture consumer-owned auth bytes, untouched even if malformed";
        std::fs::write(&auth_path, auth_bytes).unwrap();
        let config_path = crate::codex_config::get_codex_config_path();
        std::fs::write(
            &config_path,
            "# user comment\nmodel = 'previous'\n[features]\nuser_choice = true\n",
        )
        .unwrap();

        let mut first = fixture();
        first.id = super::super::QUICK_SETUP_CODEX_PROVIDER_ID.into();
        let result = ProviderService::apply_quick_setup(state, AppType::Codex, first).unwrap();
        assert!(result.live_config_changed);
        let saved = state
            .db
            .get_provider_by_id(super::super::QUICK_SETUP_CODEX_PROVIDER_ID, "codex")
            .unwrap()
            .unwrap();
        assert!(binding(&saved).is_some());
        assert!(!saved.settings_config.to_string().contains(CANARY));
        let live = std::fs::read_to_string(&config_path).unwrap();
        assert!(live.contains(CANARY));
        assert!(live.contains("user_choice"));
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);

        let mut edit = sanitize_provider_for_export(&saved);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!("");
        edit.settings_config["config"] = json!("model_provider = 'custom'\nmodel = 'changed-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\nrequires_openai_auth = true\n");
        ProviderService::update(state, AppType::Codex, None, edit).unwrap();
        let updated = state
            .db
            .get_provider_by_id(&saved.id, "codex")
            .unwrap()
            .unwrap();
        assert_eq!(binding(&saved), binding(&updated));
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains("changed-model"));
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains(CANARY));

        let mut second = fixture();
        second.id = "fixture-second".into();
        second.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-second-secret");
        second.settings_config["config"] = json!("model_provider = 'second'\nmodel = 'second-model'\n[model_providers.second]\nname = 'Second'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n");
        ProviderService::add_draft(state, AppType::Codex, second.clone()).unwrap();
        ProviderService::switch(state, AppType::Codex, &second.id).unwrap();
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains("fixture-second-secret"));
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);
        ProviderService::delete(state, AppType::Codex, &second.id).unwrap();
        assert!(state
            .db
            .get_provider_by_id(&second.id, "codex")
            .unwrap()
            .is_none());
        assert!(state
            .db
            .provider_credential_records()
            .unwrap()
            .iter()
            .filter(|record| record.provider_id == second.id)
            .all(|record| record.status == "deleted"));
    });
}

#[test]
fn explicit_replacement_repairs_missing_material_without_inline_fallback() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let old = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &old).unwrap();
    backend.delete(record.handle.secret_ref()).unwrap();
    assert!(ProviderCredentials::resolve(&db, "codex", &old).is_err());
    db.save_provider("codex", &fixture()).unwrap();
    let repaired = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert_ne!(binding(&old), binding(&repaired));
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &repaired)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn interrupted_admitted_create_reuses_verified_material_and_preserves_legacy_until_retry() {
    let (db, backend) = database();
    let legacy = fixture();
    db.save_provider_record("codex", &legacy).unwrap();
    // Simulate interruption after native create but before readback/DB cutover.
    let record = ProviderCredentialRecord {
        id: "pc_interrupted".into(),
        provider_id: legacy.id.clone(),
        handle: db.provider_secrets.reserve(),
        status: "pending".into(),
    };
    db.admit_provider_credential(&record).unwrap();
    backend
        .create_new(
            record.handle.secret_ref(),
            &SecretMaterial::from_native_input(CANARY.as_bytes().to_vec(), PURPOSE).unwrap(),
        )
        .unwrap();
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(backend.contains(record.handle.secret_ref()));
    assert_eq!(
        db.get_provider_by_id(&legacy.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
    db.save_provider("codex", &legacy).unwrap();
    let saved = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert_eq!(binding(&saved), Some("pc_interrupted"));
    assert_eq!(db.provider_credential_records().unwrap().len(), 1);
}

#[test]
fn conflicting_inline_credentials_fail_without_erasing_legacy() {
    let (db, _) = database();
    let mut legacy = fixture();
    legacy.settings_config["env"] = json!({"OPENAI_API_KEY": "other-fixture-account"});
    db.save_provider_record("codex", &legacy).unwrap();
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    assert_eq!(
        db.get_provider_by_id(&legacy.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
}

#[test]
fn all_provider_export_shapes_and_snapshot_copies_are_secret_free() {
    let (db, _) = database();
    let shapes = [
        (
            "claude",
            json!({"env":{"ANTHROPIC_AUTH_TOKEN":CANARY,"ANTHROPIC_BASE_URL":"https://example.invalid"},"headers":{"Authorization":CANARY}}),
        ),
        ("gemini", json!({"env":{"GEMINI_API_KEY":CANARY}})),
        (
            "opencode",
            json!({"options":{"apiKey":CANARY,"headers":{"Authorization":CANARY},"baseURL":"https://example.invalid"}}),
        ),
        (
            "openclaw",
            json!({"apiKey":CANARY,"baseUrl":"https://example.invalid","models":[{"id":"fixture-model"}]}),
        ),
        ("hermes", json!({"api_key":CANARY,"model":"fixture-model"})),
        (
            "grokbuild",
            json!({"config":format!("# {CANARY}\napi_key = '{CANARY}'\n[api]\nbase_url = 'https://example.invalid'\n")}),
        ),
        (
            "claude-desktop",
            json!({"unknownExtension":CANARY,"auth":{"accessToken":CANARY}}),
        ),
    ];
    for (app, shape) in shapes {
        let mut provider =
            Provider::with_id(format!("fixture-{app}"), "Fixture".into(), shape, None);
        provider.notes = Some(format!("embedded {CANARY}"));
        db.save_provider(app, &provider).unwrap();
        db.set_current_provider(app, &provider.id).unwrap();
    }
    db.set_setting(
        "universal_providers",
        &json!({"fixture":{"apiKey":CANARY}}).to_string(),
    )
    .unwrap();
    db.set_setting("common_config_codex", &format!("token='{CANARY}'"))
        .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO profiles(id,name,payload) VALUES('profile','Profile',?1)",
            [CANARY],
        )
        .unwrap();
        conn.execute_batch("CREATE TABLE credential_spy(value TEXT); CREATE TRIGGER credential_spy_copy BEFORE UPDATE ON providers BEGIN INSERT INTO credential_spy(value) VALUES(OLD.settings_config); END;").unwrap();
    }
    for export in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!export.contains(CANARY));
        assert!(!export.contains("unknownExtension"));
    }
    // Public projections do not mutate the underlying private backup source.
    assert!(db
        .get_provider_by_id("fixture-claude", "claude")
        .unwrap()
        .unwrap()
        .settings_config
        .to_string()
        .contains(CANARY));
}

#[test]
#[serial_test::serial]
fn sync_import_preserves_local_credential_route_without_grafting_to_remote_endpoint() {
    super::super::tests::with_test_home(|state, _home| {
        state.db.save_provider("codex", &fixture()).unwrap();
        let before = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        let remote = Database::memory().unwrap();
        let mut remote_provider = fixture();
        remote_provider.settings_config["config"] =
            json!("model='remote-model'\nbase_url='https://other.invalid'\n");
        remote.save_provider("codex", &remote_provider).unwrap();
        state
            .db
            .import_sql_string_for_sync(&remote.export_sql_string_for_sync().unwrap())
            .unwrap();
        let after = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert_eq!(after.settings_config, before.settings_config);
        assert_eq!(
            ProviderCredentials::resolve(&state.db, "codex", &after)
                .unwrap()
                .settings_config["auth"]["OPENAI_API_KEY"],
            CANARY
        );
    });
}

#[test]
#[serial_test::serial]
fn imported_portable_provider_is_a_credentialless_inactive_draft() {
    super::super::tests::with_test_home(|state, _home| {
        let remote = Database::memory().unwrap();
        remote.save_provider("codex", &fixture()).unwrap();
        remote
            .set_current_provider("codex", "fixture-codex")
            .unwrap();
        let config_path = crate::codex_config::get_codex_config_path();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, "model='user-choice'\n").unwrap();
        let before = std::fs::read(&config_path).unwrap();
        state
            .db
            .import_sql_string(&remote.export_sql_string().unwrap())
            .unwrap();
        let draft = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert!(binding(&draft).is_none());
        assert!(key(&draft).is_none());
        assert!(state.db.get_current_provider("codex").unwrap().is_none());
        assert_eq!(std::fs::read(&config_path).unwrap(), before);
    });
}

#[test]
#[serial_test::serial]
fn missing_target_credential_does_not_change_current_provider_or_live_file() {
    use crate::app_config::AppType;
    use crate::services::ProviderService;
    super::super::tests::with_test_home(|state, _home| {
        let mut first = fixture();
        first.id = super::super::QUICK_SETUP_CODEX_PROVIDER_ID.into();
        ProviderService::apply_quick_setup(state, AppType::Codex, first).unwrap();
        let mut second = fixture();
        second.id = "other-target".into();
        ProviderService::add_draft(state, AppType::Codex, second).unwrap();
        let second = state
            .db
            .get_provider_by_id("other-target", "codex")
            .unwrap()
            .unwrap();
        let record = ProviderCredentials::record(&state.db, &second).unwrap();
        state.db.provider_secrets.delete(&record.handle).unwrap();
        let live_before = std::fs::read(crate::codex_config::get_codex_config_path()).unwrap();
        assert!(ProviderService::switch(state, AppType::Codex, "other-target").is_err());
        assert_eq!(
            state.db.get_current_provider("codex").unwrap().as_deref(),
            Some(super::super::QUICK_SETUP_CODEX_PROVIDER_ID)
        );
        assert_eq!(
            std::fs::read(crate::codex_config::get_codex_config_path()).unwrap(),
            live_before
        );
    });
}

#[test]
fn api_key_persistence_cannot_be_bypassed_by_an_official_category_label() {
    let (db, _) = database();
    let mut provider = fixture();
    provider.category = Some("official".into());
    db.save_provider("codex", &provider).unwrap();
    let stored = db
        .get_provider_by_id(&provider.id, "codex")
        .unwrap()
        .unwrap();
    assert!(binding(&stored).is_some());
    assert!(!stored.settings_config.to_string().contains(CANARY));
}

#[test]
#[serial_test::serial]
fn proxy_router_receives_native_material_while_database_remains_reference_only() {
    super::super::tests::with_test_home(|state, _home| {
        state.db.save_provider("codex", &fixture()).unwrap();
        state
            .db
            .set_current_provider("codex", "fixture-codex")
            .unwrap();
        crate::settings::set_current_provider(
            &crate::app_config::AppType::Codex,
            Some("fixture-codex"),
        )
        .unwrap();
        let router = crate::proxy::provider_router::ProviderRouter::new(state.db.clone());
        let providers = futures::executor::block_on(router.select_providers("codex")).unwrap();
        assert_eq!(
            providers[0].settings_config["auth"]["OPENAI_API_KEY"],
            CANARY
        );
        let stored = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert!(!stored.settings_config.to_string().contains(CANARY));
        let record = ProviderCredentials::record(&state.db, &stored).unwrap();
        state.db.provider_secrets.delete(&record.handle).unwrap();
        assert!(futures::executor::block_on(router.select_providers("codex")).is_err());
    });
}

#[test]
fn blank_legacy_edit_does_not_erase_the_only_copy_when_migration_is_locked() {
    let (db, backend) = database();
    db.save_provider_record("codex", &fixture()).unwrap();
    let mut edit = fixture();
    edit.settings_config["auth"]["OPENAI_API_KEY"] = json!("");
    edit.settings_config["config"] = json!("model='changed-model'");
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(db.save_provider("codex", &edit).is_err());
    assert_eq!(
        db.get_provider_by_id(&edit.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        fixture().settings_config
    );
    backend.set_mode(MemoryFailureMode::Healthy);
    db.save_provider("codex", &edit).unwrap();
    let saved = db.get_provider_by_id(&edit.id, "codex").unwrap().unwrap();
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &saved)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
    let mut new_mask = edit;
    new_mask.id = "new-mask".into();
    new_mask.settings_config["auth"]["OPENAI_API_KEY"] = json!("********");
    assert!(db.save_provider("codex", &new_mask).is_err());
}
