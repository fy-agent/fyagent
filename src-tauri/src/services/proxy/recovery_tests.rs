use super::*;
use serial_test::serial;
use std::fs;

fn fixture() -> (super::tests::TempHome, Arc<Database>, ProxyService) {
    let home = super::tests::TempHome::new();
    crate::settings::reload_settings().unwrap();
    let db = Arc::new(Database::memory().unwrap());
    let service = ProxyService::new(db.clone());
    (home, db, service)
}

async fn mark_enabled(db: &Database, app: &str) {
    let mut config = db.get_proxy_config_for_app(app).await.unwrap();
    config.enabled = true;
    db.update_proxy_config_for_app(config).await.unwrap();
}

#[tokio::test]
#[serial]
async fn restore_preview_is_closed_and_does_not_invent_missing_backup_authority() {
    let (_home, db, service) = fixture();
    for app in ["claude", "codex", "grokbuild"] {
        let disabled = service.get_restore_preview(app).await.unwrap();
        assert!(!disabled.enabled && !disabled.can_restore && disabled.targets.is_empty());
        mark_enabled(&db, app).await;
        let missing = service.get_restore_preview(app).await.unwrap();
        assert!(missing.enabled && !missing.can_restore && missing.targets.is_empty());
        assert!(db.get_live_backup_sync(app).unwrap().is_none());
    }
    for app in ["opencode", "gemini", "../claude", "Claude", ""] {
        assert!(service.get_restore_preview(app).await.is_err());
    }
}

#[tokio::test]
#[serial]
async fn legacy_restore_preview_preserves_files_backups_and_stale_selection() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    mark_enabled(&db, "claude").await;
    let saved = db
        .get_live_backup_sync("claude")
        .unwrap()
        .unwrap()
        .original_config;
    let live = fs::read(&path).unwrap();
    let receipt = fs::read(crate::config::rolling_backup_path(&path)).unwrap();
    let preview = service.get_restore_preview("claude").await.unwrap();
    assert!(preview.enabled && preview.can_restore);
    assert_eq!(preview.targets.len(), 1);
    assert_eq!(
        preview.targets[0].path,
        crate::config::display_user_path(&path)
    );
    assert!(preview.targets[0].exists);
    let dto = serde_json::to_string(&preview).unwrap();
    for excluded in ["fixture-key", "ANTHROPIC_API_KEY", "backupPath", "preimage"] {
        assert!(
            !dto.contains(excluded),
            "unexpected DTO content: {excluded}"
        );
    }
    assert_eq!(fs::read(&path).unwrap(), live);
    assert_eq!(
        fs::read(crate::config::rolling_backup_path(&path)).unwrap(),
        receipt
    );
    assert_eq!(
        db.get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
    assert!(!db.get_proxy_config_for_app("codex").await.unwrap().enabled);

    crate::settings::set_current_provider(&AppType::Claude, Some("stale-provider")).unwrap();
    let unknown = service.get_restore_preview("claude").await.unwrap();
    assert!(unknown.enabled && !unknown.can_restore);
    assert_closed_claude_target(&unknown, &path);
    let original = serde_json::from_str(&saved).unwrap();
    assert!(service
        .legacy_restore_preview_paths(&AppType::Claude, &original)
        .is_err());
    assert_eq!(
        crate::settings::get_current_provider(&AppType::Claude).as_deref(),
        Some("stale-provider")
    );
    assert_eq!(fs::read(&path).unwrap(), live);
}

#[tokio::test]
#[serial]
async fn restore_preview_refuses_external_edit_and_exit_rechecks_after_preview() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    mark_enabled(&db, "claude").await;
    assert!(
        service
            .get_restore_preview("claude")
            .await
            .unwrap()
            .can_restore
    );
    let mut external = fs::read(&path).unwrap();
    external.extend_from_slice(b"\n ");
    fs::write(&path, &external).unwrap();
    let changed = service.get_restore_preview("claude").await.unwrap();
    assert!(changed.enabled && !changed.can_restore);
    assert_closed_claude_target(&changed, &path);
    assert!(service.set_takeover_for_app("claude", false).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), external);
    assert!(db.get_proxy_config_for_app("claude").await.unwrap().enabled);
    assert!(db.get_live_backup_sync("claude").unwrap().is_some());
}

#[tokio::test]
#[serial]
async fn managed_restore_preview_discloses_config_and_catalog_without_native_auth() {
    let (_home, db, service) = fixture();
    let config = crate::codex_config::get_codex_config_path();
    let catalog = crate::codex_config::get_codex_model_catalog_path();
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, b"# original\n").unwrap();
    fs::write(&catalog, b"original catalog").unwrap();
    db.save_live_backup_sync(
        "codex",
        &json!({"config":"# original\n","auth":{}}).to_string(),
    )
    .unwrap();
    service
        .mark_managed_restore_proof(&AppType::Codex, None)
        .unwrap();
    crate::config::atomic_write(&config, b"# projected\n").unwrap();
    crate::config::atomic_write(&catalog, b"projected catalog").unwrap();
    service
        .mark_managed_restore_proof(
            &AppType::Codex,
            Some(ProxyService::managed_restore_files(&AppType::Codex).unwrap()),
        )
        .unwrap();
    mark_enabled(&db, "codex").await;
    let saved = db
        .get_live_backup_sync("codex")
        .unwrap()
        .unwrap()
        .original_config;
    let preview = service.get_restore_preview("codex").await.unwrap();
    assert!(preview.enabled && preview.can_restore);
    assert_eq!(
        preview
            .targets
            .iter()
            .map(|target| target.path.clone())
            .collect::<Vec<_>>(),
        vec![
            crate::config::display_user_path(&config),
            crate::config::display_user_path(&catalog)
        ]
    );
    assert!(!serde_json::to_string(&preview)
        .unwrap()
        .contains("auth.json"));
    assert_eq!(fs::read(&config).unwrap(), b"# projected\n");
    assert_eq!(fs::read(&catalog).unwrap(), b"projected catalog");
    assert_eq!(
        db.get_live_backup_sync("codex")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
    service.set_takeover_for_app("codex", false).await.unwrap();
    let finished = service.get_restore_preview("codex").await.unwrap();
    assert!(!finished.enabled && !finished.can_restore && finished.targets.is_empty());
    assert_eq!(fs::read(config).unwrap(), b"# original\n");
    assert_eq!(fs::read(catalog).unwrap(), b"original catalog");
}

#[tokio::test]
#[serial]
async fn legacy_managed_restore_preview_does_not_upgrade_database_proof() {
    let (_home, db, service) = fixture();
    let path = get_claude_settings_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = json!({"env":{"ANTHROPIC_API_KEY":"fixture-original-key","ANTHROPIC_BASE_URL":"https://example.test"},"permissions":{"allow":["Read"]}});
    let original_bytes = crate::config::json_file_contents(&original).unwrap();
    fs::write(&path, &original_bytes).unwrap();
    let saved = original.to_string();
    db.save_live_backup_sync("claude", &saved).unwrap();
    let mut provider = Provider::with_id(
        "managed-preview".into(),
        "Managed fixture".into(),
        json!({"env":{"ANTHROPIC_MODEL":"fixture-model"}}),
        None,
    );
    provider.meta = Some(crate::provider::ProviderMeta {
        provider_type: Some("xai_oauth".into()),
        ..Default::default()
    });
    assert!(provider.uses_subscription_proxy());
    db.save_provider("claude", &provider).unwrap();
    db.set_current_provider("claude", &provider.id).unwrap();
    let mut listener = db.get_global_proxy_config().await.unwrap();
    listener.listen_address = "127.0.0.1".into();
    listener.listen_port = 12345;
    db.update_global_proxy_config(listener).await.unwrap();
    let mut projected = original.clone();
    ProxyService::apply_claude_takeover_fields_for_provider(
        &mut projected,
        "http://127.0.0.1:12345",
        &provider,
    );
    service.write_claude_live(&projected).unwrap();
    mark_enabled(&db, "claude").await;
    let live = fs::read(&path).unwrap();
    let receipt = fs::read(crate::config::rolling_backup_path(&path)).unwrap();
    let selected = crate::settings::get_current_provider(&AppType::Claude);
    for _ in 0..2 {
        let preview = service.get_restore_preview("claude").await.unwrap();
        assert!(preview.enabled && preview.can_restore);
        assert_eq!(preview.targets.len(), 1);
        assert_eq!(
            preview.targets[0].path,
            crate::config::display_user_path(&path)
        );
        assert_eq!(
            db.get_live_backup_sync("claude")
                .unwrap()
                .unwrap()
                .original_config,
            saved
        );
        assert_eq!(fs::read(&path).unwrap(), live);
        assert_eq!(
            fs::read(crate::config::rolling_backup_path(&path)).unwrap(),
            receipt
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Claude),
            selected
        );
    }
    // Admission may compute an in-memory proof. It must not replace the
    // logical backup before owned restore proof is checked.
    assert!(
        service
            .verify_and_unwrap_managed_restore_for_exit(&AppType::Claude, original)
            .unwrap()
            .1
    );
    assert_eq!(
        db.get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
}

#[tokio::test]
#[serial]
async fn legacy_exit_guards_gemini_grok_and_codex_files_and_keeps_refreshed_native_login() {
    for app in [AppType::Gemini, AppType::GrokBuild, AppType::Codex] {
        let (_home, db, service) = fixture();
        let (path, before, original) = match app {
            AppType::Gemini => {
                let bytes = b"# user comment\nGEMINI_API_KEY=fixture-key\nGOOGLE_GEMINI_BASE_URL=https://example.test\n".to_vec();
                let original = crate::gemini_config::env_to_json(
                    &crate::gemini_config::parse_env_file(std::str::from_utf8(&bytes).unwrap()),
                );
                (crate::gemini_config::get_gemini_env_path(), bytes, original)
            }
            AppType::GrokBuild => {
                let config = "# user comment\n[models]\ndefault = \"grok\"\n[model.grok]\nmodel = \"grok\"\nbase_url = \"https://example.test/v1\"\nname = \"Grok\"\napi_key = \"fixture-key\"\napi_backend = \"responses\"\ncontext_window = 500000\n";
                assert!(crate::grok_config::extract_model_config(config).is_some());
                crate::grok_config::validate_config_toml(config).unwrap();
                (
                    crate::grok_config::get_grok_config_path(),
                    config.as_bytes().to_vec(),
                    json!({"config":config}),
                )
            }
            AppType::Codex => {
                let config = "# user comment\nmodel_provider = \"custom\"\nmodel = \"fixture-model\"\n[model_providers.custom]\nname = \"Custom\"\nbase_url = \"https://example.test/v1\"\nwire_api = \"responses\"\nexperimental_bearer_token = \"fixture-key\"\n";
                (
                    crate::codex_config::get_codex_config_path(),
                    config.as_bytes().to_vec(),
                    json!({"config":config,"auth":{}}),
                )
            }
            _ => unreachable!(),
        };
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &before).unwrap();
        db.save_live_backup_sync(app.as_str(), &original.to_string())
            .unwrap();
        let mut projected = original;
        match app {
            AppType::Gemini => {
                projected["env"]["GOOGLE_GEMINI_BASE_URL"] = json!("http://127.0.0.1:12345");
                projected["env"]["GEMINI_API_KEY"] = json!(PROXY_TOKEN_PLACEHOLDER);
                service.write_gemini_live(&projected).unwrap();
            }
            AppType::GrokBuild => {
                ProxyService::apply_grok_takeover_fields(
                    &mut projected,
                    "http://127.0.0.1:12345/grokbuild/v1",
                )
                .unwrap();
                service.write_grok_live(&projected).unwrap();
            }
            AppType::Codex => {
                ProxyService::apply_codex_takeover_auth_placeholder(&mut projected, None);
                projected["config"] =
                    json!(ProxyService::apply_codex_proxy_toml_config_for_provider(
                        projected["config"].as_str().unwrap(),
                        "http://127.0.0.1:12345/v1",
                        None
                    )
                    .unwrap());
                service
                    .write_codex_takeover_live_for_provider(&projected, None)
                    .unwrap();
                fs::write(
                    crate::codex_config::get_codex_auth_path(),
                    b"refreshed native login",
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let owned = fs::read(&path).unwrap();
        let mut external = owned.clone();
        external.extend_from_slice(b"\n# external edit\n");
        fs::write(&path, &external).unwrap();
        assert!(
            service
                .restore_live_config_for_app_inner(&app)
                .await
                .is_err(),
            "{app:?}"
        );
        assert_eq!(fs::read(&path).unwrap(), external);
        // The fixture explicitly resolves the conflict back to the owned bytes.
        fs::write(&path, &owned).unwrap();
        service
            .restore_live_config_for_app_inner(&app)
            .await
            .unwrap_or_else(|error| panic!("{app:?} restore failed: {error}"));
        assert_eq!(fs::read(path).unwrap(), before, "{app:?}");
        if app == AppType::Codex {
            assert_eq!(
                fs::read(crate::codex_config::get_codex_auth_path()).unwrap(),
                b"refreshed native login"
            );
        }
    }
}

fn claude_takeover(db: &Database, service: &ProxyService) -> (std::path::PathBuf, Vec<u8>) {
    let path = get_claude_settings_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = b"{\n  \"env\": {\"ANTHROPIC_API_KEY\": \"fixture-key\", \"ANTHROPIC_BASE_URL\": \"https://example.test\"},\n  \"permissions\": {\"allow\": [\"Read\"]}\n}\n";
    fs::write(&path, original).unwrap();
    let config: Value = serde_json::from_slice(original).unwrap();
    db.save_live_backup_sync("claude", &config.to_string())
        .unwrap();
    let mut projected = config;
    ProxyService::apply_claude_takeover_fields_with_policy(
        &mut projected,
        "http://127.0.0.1:12345",
        ClaudeTakeoverAuthPolicy::PreserveExistingOrAuthToken,
    );
    service.write_claude_live(&projected).unwrap();
    (path, original.to_vec())
}

#[tokio::test]
#[serial]
async fn legacy_exit_preserves_external_file_and_original_backup_in_both_branches() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    let saved = db
        .get_live_backup_sync("claude")
        .unwrap()
        .unwrap()
        .original_config;
    let mut external: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    external["permissions"]["allow"] = json!(["Read", "Edit"]);
    let bytes = serde_json::to_vec_pretty(&external).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .is_err());
    assert!(service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        db.get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
}

#[tokio::test]
#[serial]
async fn legacy_exit_restores_exact_bytes_and_retry_is_a_noop() {
    let (_home, db, service) = fixture();
    let (path, before) = claude_takeover(&db, &service);
    service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    let backup = fs::read(crate::config::rolling_backup_path(&path)).unwrap();
    service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read(crate::config::rolling_backup_path(&path)).unwrap(),
        backup
    );
}

#[tokio::test]
#[serial]
async fn legacy_exit_does_not_adopt_a_later_fyagent_writer_as_takeover_ownership() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    let mut changed: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    changed["permissions"]["deny"] = json!(["Bash"]);
    service.write_claude_live(&changed).unwrap(); // A valid, newer receipt alone is insufficient.
    let bytes = fs::read(&path).unwrap();
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[tokio::test]
#[serial]
async fn legacy_exit_rechecks_external_edit_after_admission() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    let target = path.clone();
    service.set_managed_activation_test_hook(Some(Arc::new(move |_, phase| {
        if phase == "legacy_restore_admitted" {
            fs::write(&target, b"external after admission").unwrap();
        }
        Ok(())
    })));
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .is_err());
    assert_eq!(fs::read(path).unwrap(), b"external after admission");
    assert!(db.get_live_backup_sync("claude").unwrap().is_some());
}

#[tokio::test]
#[serial]
async fn legacy_exit_rejects_missing_or_corrupt_receipts_without_overwriting() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    let marker = path.with_file_name("settings.json.fyagent.undo.json");
    fs::write(&marker, b"invalid receipt").unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .is_err());
    fs::remove_file(marker).unwrap();
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[tokio::test]
#[serial]
async fn legacy_exit_without_db_backup_uses_a_verified_original_receipt() {
    let (_home, db, service) = fixture();
    let (path, before) = claude_takeover(&db, &service);
    db.delete_live_backup("claude").await.unwrap();
    service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .unwrap();
    assert_eq!(fs::read(path).unwrap(), before);
}

#[tokio::test]
#[serial]
async fn legacy_exit_handles_hot_switch_backup_without_losing_source_or_unrelated_fields() {
    let (_home, db, service) = fixture();
    let (path, _) = claude_takeover(&db, &service);
    let original = json!({"env":{"ANTHROPIC_API_KEY":"new-fixture-key","ANTHROPIC_BASE_URL":"https://new.example.test"},"permissions":{"allow":["Read"]}});
    db.save_live_backup_sync("claude", &original.to_string())
        .unwrap();
    let mut next = original.clone();
    ProxyService::apply_claude_takeover_fields_with_policy(
        &mut next,
        "http://127.0.0.1:12345",
        ClaudeTakeoverAuthPolicy::PreserveExistingOrAuthToken,
    );
    service.write_claude_live(&next).unwrap();
    service
        .restore_live_config_for_app_inner(&AppType::Claude)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(path).unwrap()).unwrap(),
        original
    );
}

#[tokio::test]
#[serial]
async fn managed_exit_retries_partial_files_but_rebinding_stays_strict() {
    let (_home, db, service) = fixture();
    let config = crate::codex_config::get_codex_config_path();
    let catalog = crate::codex_config::get_codex_model_catalog_path();
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, b"# original\n").unwrap();
    fs::write(&catalog, b"original catalog").unwrap();
    db.save_live_backup_sync(
        "codex",
        &json!({"config":"# original\n","auth":{}}).to_string(),
    )
    .unwrap();
    service
        .mark_managed_restore_proof(&AppType::Codex, None)
        .unwrap();
    crate::config::atomic_write(&config, b"# projected\n").unwrap();
    crate::config::atomic_write(&catalog, b"projected catalog").unwrap();
    service
        .mark_managed_restore_proof(
            &AppType::Codex,
            Some(ProxyService::managed_restore_files(&AppType::Codex).unwrap()),
        )
        .unwrap();
    service.set_managed_activation_test_hook(Some(Arc::new(|_, phase| {
        if phase == "restored_first_file" {
            return Err("fixture interruption".into());
        }
        Ok(())
    })));
    assert!(service
        .restore_live_config_for_app_inner(&AppType::Codex)
        .await
        .is_err());
    assert_eq!(fs::read(&config).unwrap(), b"# original\n");
    assert_eq!(fs::read(&catalog).unwrap(), b"projected catalog");
    let saved = db
        .get_live_backup_sync("codex")
        .unwrap()
        .unwrap()
        .original_config;
    assert!(service
        .validate_existing_managed_backup(&AppType::Codex, &saved)
        .is_err());
    service.set_managed_activation_test_hook(None);
    service
        .restore_live_config_for_app_inner(&AppType::Codex)
        .await
        .unwrap();
    service
        .restore_live_config_for_app_inner(&AppType::Codex)
        .await
        .unwrap();
    assert_eq!(fs::read(config).unwrap(), b"# original\n");
    assert_eq!(fs::read(catalog).unwrap(), b"original catalog");
}

fn assert_closed_claude_target(preview: &ProxyRestorePreview, path: &std::path::Path) {
    assert_eq!(preview.targets.len(), 1);
    assert_eq!(
        preview.targets[0].path,
        crate::config::display_user_path(path)
    );
    assert!(preview.targets[0].exists);
    let dto = serde_json::to_string(preview).unwrap();
    for excluded in [
        "fixture-key",
        "fixture-original-key",
        "ANTHROPIC_API_KEY",
        "backupPath",
        "preimage",
        "fyagentManagedRestore",
    ] {
        assert!(
            !dto.contains(excluded),
            "unexpected DTO content: {excluded}"
        );
    }
}

fn fail_complete_app_restore_after_first_statement(db: &Database) {
    let conn = db.conn.lock().expect("db lock");
    conn.execute_batch(
        "CREATE TEMP TRIGGER fixture_fail_complete_app_restore
         BEFORE DELETE ON proxy_live_backup
         BEGIN
           SELECT RAISE(ABORT, 'fixture sql failure after first statement');
         END;",
    )
    .unwrap();
}

fn clear_complete_app_restore_failure(db: &Database) {
    let conn = db.conn.lock().expect("db lock");
    conn.execute_batch("DROP TRIGGER IF EXISTS fixture_fail_complete_app_restore;")
        .unwrap();
}

async fn legacy_managed_claude(
    db: &Database,
    service: &ProxyService,
) -> (std::path::PathBuf, String, Vec<u8>, Vec<u8>) {
    let path = get_claude_settings_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = json!({"env":{"ANTHROPIC_API_KEY":"fixture-original-key","ANTHROPIC_BASE_URL":"https://example.test"},"permissions":{"allow":["Read"]}});
    let original_bytes = crate::config::json_file_contents(&original).unwrap();
    fs::write(&path, &original_bytes).unwrap();
    let saved = original.to_string();
    db.save_live_backup_sync("claude", &saved).unwrap();
    let mut provider = Provider::with_id(
        "managed-preview".into(),
        "Managed fixture".into(),
        json!({"env":{"ANTHROPIC_MODEL":"fixture-model"}}),
        None,
    );
    provider.meta = Some(crate::provider::ProviderMeta {
        provider_type: Some("xai_oauth".into()),
        ..Default::default()
    });
    assert!(provider.uses_subscription_proxy());
    db.save_provider("claude", &provider).unwrap();
    db.set_current_provider("claude", &provider.id).unwrap();
    let mut listener = db.get_global_proxy_config().await.unwrap();
    listener.listen_address = "127.0.0.1".into();
    listener.listen_port = 12345;
    db.update_global_proxy_config(listener).await.unwrap();
    let mut projected = original;
    ProxyService::apply_claude_takeover_fields_for_provider(
        &mut projected,
        "http://127.0.0.1:12345",
        &provider,
    );
    service.write_claude_live(&projected).unwrap();
    mark_enabled(db, "claude").await;
    let live = fs::read(&path).unwrap();
    (path, saved, original_bytes, live)
}

#[tokio::test]
#[serial]
async fn legacy_unchanged_preview_discloses_closed_targets_then_disable_succeeds() {
    let (_home, db, service) = fixture();
    let (path, before) = claude_takeover(&db, &service);
    mark_enabled(&db, "claude").await;
    service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    let retries = db
        .get_proxy_config_for_app("claude")
        .await
        .unwrap()
        .max_retries;
    let preview = service.get_restore_preview("claude").await.unwrap();
    assert!(preview.enabled && preview.can_restore);
    assert_closed_claude_target(&preview, &path);
    assert_eq!(fs::read(&path).unwrap(), before);
    service.set_takeover_for_app("claude", false).await.unwrap();
    let finished = service.get_restore_preview("claude").await.unwrap();
    assert!(!finished.enabled && !finished.can_restore && finished.targets.is_empty());
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(db.get_live_backup_sync("claude").unwrap().is_none());
    assert_eq!(
        db.get_proxy_config_for_app("claude")
            .await
            .unwrap()
            .max_retries,
        retries
    );
}

#[tokio::test]
#[serial]
async fn complete_app_restore_sql_failure_keeps_retryable_preview() {
    let (_home, db, service) = fixture();
    let (path, before) = claude_takeover(&db, &service);
    mark_enabled(&db, "claude").await;
    service
        .restore_live_config_for_app_with_fallback_inner(&AppType::Claude)
        .await
        .unwrap();
    let retries = db
        .get_proxy_config_for_app("claude")
        .await
        .unwrap()
        .max_retries;
    let saved = db
        .get_live_backup_sync("claude")
        .unwrap()
        .unwrap()
        .original_config;
    fail_complete_app_restore_after_first_statement(&db);
    assert!(service.set_takeover_for_app("claude", false).await.is_err());
    assert!(db.get_proxy_config_for_app("claude").await.unwrap().enabled);
    assert_eq!(
        db.get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
    assert_eq!(
        db.get_proxy_config_for_app("claude")
            .await
            .unwrap()
            .max_retries,
        retries
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let retry = service.get_restore_preview("claude").await.unwrap();
    assert!(retry.enabled && retry.can_restore);
    assert_closed_claude_target(&retry, &path);
    clear_complete_app_restore_failure(&db);
    service.set_takeover_for_app("claude", false).await.unwrap();
    let finished = service.get_restore_preview("claude").await.unwrap();
    assert!(!finished.enabled && !finished.can_restore && finished.targets.is_empty());
    assert!(db.get_live_backup_sync("claude").unwrap().is_none());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[tokio::test]
#[serial]
async fn legacy_managed_preview_then_exit_restores_logical_backup() {
    let (_home, db, service) = fixture();
    let (path, saved, original_bytes, live) = legacy_managed_claude(&db, &service).await;
    let preview = service.get_restore_preview("claude").await.unwrap();
    assert!(preview.enabled && preview.can_restore);
    assert_closed_claude_target(&preview, &path);
    assert_eq!(
        db.get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        saved
    );
    assert_eq!(fs::read(&path).unwrap(), live);
    service.set_takeover_for_app("claude", false).await.unwrap();
    let finished = service.get_restore_preview("claude").await.unwrap();
    assert!(!finished.enabled && !finished.can_restore && finished.targets.is_empty());
    assert_eq!(fs::read(path).unwrap(), original_bytes);
    assert!(db.get_live_backup_sync("claude").unwrap().is_none());
}

#[tokio::test]
#[serial]
async fn legacy_managed_external_edit_preserves_logical_backup_until_owned_exit() {
    let (_home, db, service) = fixture();
    let (path, saved, original_bytes, live) = legacy_managed_claude(&db, &service).await;
    assert!(
        service
            .get_restore_preview("claude")
            .await
            .unwrap()
            .can_restore
    );
    let mut external = live.clone();
    external.extend_from_slice(b"\n ");
    fs::write(&path, &external).unwrap();
    let changed = service.get_restore_preview("claude").await.unwrap();
    assert!(changed.enabled && !changed.can_restore);
    assert_closed_claude_target(&changed, &path);
    assert!(service.set_takeover_for_app("claude", false).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), external);
    let after_conflict = db
        .get_live_backup_sync("claude")
        .unwrap()
        .unwrap()
        .original_config;
    assert_eq!(after_conflict, saved);
    assert!(serde_json::from_str::<Value>(&after_conflict)
        .unwrap()
        .get("fyagentManagedRestore")
        .is_none());
    fs::write(&path, &live).unwrap();
    let retry = service.get_restore_preview("claude").await.unwrap();
    assert!(retry.enabled && retry.can_restore);
    assert_closed_claude_target(&retry, &path);
    service.set_takeover_for_app("claude", false).await.unwrap();
    assert_eq!(fs::read(path).unwrap(), original_bytes);
    assert!(!db.get_proxy_config_for_app("claude").await.unwrap().enabled);
    assert!(db.get_live_backup_sync("claude").unwrap().is_none());
}

#[tokio::test]
#[serial]
async fn legacy_managed_first_file_interrupt_keeps_retryable_preview() {
    let (_home, db, service) = fixture();
    let (path, saved, original_bytes, _live) = legacy_managed_claude(&db, &service).await;
    assert!(serde_json::from_str::<Value>(&saved)
        .unwrap()
        .get("fyagentManagedRestore")
        .is_none());
    service.set_managed_activation_test_hook(Some(Arc::new(|_, phase| {
        if phase == "restored_first_file" {
            return Err("fixture interruption".into());
        }
        Ok(())
    })));
    assert!(service.set_takeover_for_app("claude", false).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), original_bytes);
    assert!(db.get_proxy_config_for_app("claude").await.unwrap().enabled);
    let interrupted = db
        .get_live_backup_sync("claude")
        .unwrap()
        .unwrap()
        .original_config;
    assert_ne!(interrupted, saved);
    assert!(serde_json::from_str::<Value>(&interrupted)
        .unwrap()
        .get("fyagentManagedRestore")
        .is_some());
    let retry = service.get_restore_preview("claude").await.unwrap();
    assert!(retry.enabled && retry.can_restore);
    assert_closed_claude_target(&retry, &path);
    service.set_managed_activation_test_hook(None);
    service.set_takeover_for_app("claude", false).await.unwrap();
    let finished = service.get_restore_preview("claude").await.unwrap();
    assert!(!finished.enabled && !finished.can_restore && finished.targets.is_empty());
    assert_eq!(fs::read(path).unwrap(), original_bytes);
    assert!(db.get_live_backup_sync("claude").unwrap().is_none());
}
