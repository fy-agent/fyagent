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
                let config = "# user comment\n[models]\ndefault = \"grok\"\n[model.grok]\nmodel = \"grok\"\nbase_url = \"https://example.test/v1\"\nname = \"Grok\"\napi_key = \"fixture-key\"\napi_backend = \"responses\"\n";
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
            .unwrap();
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
