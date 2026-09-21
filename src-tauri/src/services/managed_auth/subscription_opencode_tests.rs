use super::*;
use crate::services::provider::BindOpenCodeManagedProxyRequest;

fn revision() -> Option<String> {
    crate::services::opencode_models::get_opencode_model_snapshot()
        .unwrap()
        .revision
}

pub(super) fn bind_request(identity: &str, model: &str) -> BindOpenCodeManagedProxyRequest {
    BindOpenCodeManagedProxyRequest {
        account_id: identity.into(),
        model_id: model.into(),
        expected_revision: revision(),
    }
}

pub(super) fn ephemeral(runtime: &tokio::runtime::Runtime, state: &AppState) {
    let mut config = runtime
        .block_on(state.db.get_global_proxy_config())
        .unwrap();
    config.listen_address = "127.0.0.1".into();
    config.listen_port = 0;
    runtime
        .block_on(state.db.update_global_proxy_config(config))
        .unwrap();
}

fn original_config() -> Value {
    json!({"model":"user/original", "theme":"retained", "unknown":{"enabled":true},
        "mcp":{"fixture":{"type":"local","command":["fixture"]}},
        "provider":{"user":{"npm":"@ai-sdk/openai-compatible", "options":{"apiKey":"native-api-key"},
            "models":{"original":{"name":"Original"}}}}})
}

fn write_native() -> (std::path::PathBuf, Vec<u8>, std::path::PathBuf, Vec<u8>) {
    let path = crate::opencode_config::get_opencode_config_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = serde_json::to_vec_pretty(&original_config()).unwrap();
    std::fs::write(&path, &original).unwrap();
    let auth = crate::opencode_config::get_opencode_auth_json_path();
    std::fs::create_dir_all(auth.parent().unwrap()).unwrap();
    let native_auth =
        b"{\"openai\":{\"type\":\"oauth\",\"refresh\":\"native-refresh-never-copy\"}}".to_vec();
    std::fs::write(&auth, &native_auth).unwrap();
    (path, original, auth, native_auth)
}

#[test]
fn subscription_opencode_request_requires_explicit_revision_and_closed_keys() {
    let valid = json!({"accountId":format!("ma1:{}", "a".repeat(32)),"modelId":"model-1","expectedRevision":null});
    assert!(serde_json::from_value::<BindOpenCodeManagedProxyRequest>(valid.clone()).is_ok());
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("expectedRevision");
    assert!(serde_json::from_value::<BindOpenCodeManagedProxyRequest>(missing).is_err());
    let mut unknown = valid;
    unknown["app"] = json!("codex");
    assert!(serde_json::from_value::<BindOpenCodeManagedProxyRequest>(unknown).is_err());
}

#[test]
#[serial]
fn subscription_opencode_both_sources_revision_isolation_secret_free_and_restore() {
    for source in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        let (_home, _guard, state, auth) = fixture();
        let selected = seed_provider(&auth, "selected", source);
        let other = seed_provider(&auth, "other", source);
        let (path, original, auth_path, auth_bytes) = write_native();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        ephemeral(&runtime, &state);
        let before_revision = revision();
        let bound = ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            bind_request(&selected.identity_id, "selected-model"),
        )
        .unwrap();
        assert!(bound.activated && !bound.already_bound);
        assert_eq!(bound.app, "opencode");
        let port = runtime
            .block_on(state.proxy_service.get_status())
            .unwrap()
            .port;
        assert_ne!(port, 0);
        let snapshot = crate::services::opencode_models::get_opencode_model_snapshot().unwrap();
        assert_eq!(
            snapshot.selected_model,
            Some(format!("{}/selected-model", bound.provider_id))
        );
        assert!(snapshot
            .providers
            .iter()
            .any(|provider| provider.id == bound.provider_id
                && provider.model_ids == ["selected-model"]));
        let live: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            live["model"],
            format!("{}/selected-model", bound.provider_id)
        );
        assert_eq!(live["mcp"], original_config()["mcp"]);
        assert_eq!(live["unknown"], original_config()["unknown"]);
        assert_eq!(
            live["provider"]["user"],
            original_config()["provider"]["user"]
        );
        assert_eq!(
            live["provider"][&bound.provider_id]["npm"],
            "@ai-sdk/openai"
        );
        assert_eq!(
            live["provider"][&bound.provider_id]["options"]["baseURL"],
            format!("http://127.0.0.1:{port}/opencode/v1")
        );
        assert_eq!(
            std::fs::read(crate::opencode_config::get_opencode_dir().join("opencode.json.backup"))
                .unwrap(),
            original
        );
        let row = state
            .db
            .get_provider_by_id(&bound.provider_id, "opencode")
            .unwrap()
            .unwrap();
        for value in [
            serde_json::to_string(&row).unwrap(),
            serde_json::to_string(&snapshot).unwrap(),
            serde_json::to_string(&bound).unwrap(),
            live.to_string(),
        ] {
            assert!(!value.contains("synthetic-access"));
            assert!(!value.contains("synthetic-refresh"));
            assert!(!value.contains("native-refresh-never-copy"));
        }
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);
        for app in ["claude", "codex", "grokbuild"] {
            assert!(state.db.get_all_providers(app).unwrap().is_empty());
        }
        // Startup imports native providers before resuming proxy state. The
        // loopback projection must not replace the saved upstream definition.
        crate::services::provider::import_opencode_providers_from_live(&state).unwrap();
        let after_import = state
            .db
            .get_provider_by_id(&bound.provider_id, "opencode")
            .unwrap()
            .unwrap();
        assert_eq!(after_import.settings_config, row.settings_config);
        assert_eq!(
            serde_json::to_value(&after_import.meta).unwrap(),
            serde_json::to_value(&row.meta).unwrap()
        );
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);
        let mut stale = bind_request(&selected.identity_id, "selected-model");
        stale.expected_revision = before_revision;
        assert!(matches!(
            ProviderService::bind_opencode_managed_proxy(&state, &auth, stale),
            Err(BindXaiManagedError::ProviderConflict)
        ));
        let again = ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            bind_request(&selected.identity_id, "selected-model"),
        )
        .unwrap();
        assert!(again.already_bound);
        let next = ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            bind_request(&other.identity_id, "other-model"),
        )
        .unwrap();
        assert_ne!(next.provider_id, bound.provider_id);
        let live: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(
            live["provider"].get(&bound.provider_id).is_none(),
            "stale account-labelled native slots must not reach a new current account"
        );
        assert_eq!(
            runtime
                .block_on(state.proxy_service.get_status())
                .unwrap()
                .port,
            port
        );
        let kind = if source == ManagedAuthProvider::Openai {
            "codex_oauth"
        } else {
            "xai_oauth"
        };
        crate::settings::set_current_provider(&AppType::Claude, Some("stale-unrelated-target"))
            .unwrap();
        assert_eq!(
            state
                .proxy_service
                .observe_managed_account_route(kind, "other", true),
            Some(true)
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Claude).as_deref(),
            Some("stale-unrelated-target")
        );
        crate::settings::set_current_provider(&AppType::Claude, None).unwrap();
        assert_eq!(
            state
                .proxy_service
                .observe_managed_account_route(kind, "selected", false),
            Some(false)
        );
        runtime
            .block_on(state.proxy_service.stop_with_restore())
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read(auth_path).unwrap(), auth_bytes);
        assert!(
            !runtime
                .block_on(state.proxy_service.get_takeover_status())
                .unwrap()
                .opencode
        );
    }
}

fn activate_target(
    state: &AppState,
    auth: &ManagedAuthService<MemorySecretBackend>,
    app: &str,
    account: &str,
    model: &str,
) -> Result<crate::services::provider::BindManagedProxyResult, BindXaiManagedError> {
    if app == "opencode" {
        return ProviderService::bind_opencode_managed_proxy(
            state,
            auth,
            bind_request(account, model),
        );
    }
    let mut req = request(app, account);
    req.model_id = model.into();
    let bound = ProviderService::bind_managed_proxy(state, auth, req)?;
    if app == "codex" {
        let provider = state
            .db
            .get_provider_by_id(&bound.provider_id, app)
            .unwrap()
            .unwrap();
        let _lock = ProviderService::lock_provider_mutation(state, &AppType::Codex);
        ProviderService::apply_quick_setup_with_lock_held(state, AppType::Codex, provider)
            .map_err(BindXaiManagedError::from)?;
    }
    Ok(bound)
}

fn native_target(app: &str) -> std::path::PathBuf {
    let (path, original) = match app {
        "opencode" => return write_native().0,
        "claude" => (
            crate::config::get_claude_settings_path(),
            "{ \"env\" : { \"ANTHROPIC_API_KEY\" : \"native-key\" }, \"permissions\" : {} }\n",
        ),
        "codex" => (
            crate::codex_config::get_codex_config_path(),
            "# native configuration\nmodel = \"original\"\n",
        ),
        _ => (
            crate::grok_config::get_grok_config_path(),
            "# native official configuration\n",
        ),
    };
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, original).unwrap();
    path
}

#[test]
#[serial]
fn subscription_managed_proof_window_and_rebind_refuse_external_edits() {
    for app in ["opencode", "claude", "codex", "grokbuild"] {
        for during_proof in [true, false] {
            let (_home, _guard, state, auth) = fixture();
            let account = seed(&auth, "selected");
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let _entered = runtime.enter();
            ephemeral(&runtime, &state);
            let path = native_target(app);
            let external = if matches!(app, "codex" | "grokbuild") {
                "# external edit\n"
            } else {
                "{\"external\":true}"
            };
            if during_proof {
                let target = path.clone();
                state
                    .proxy_service
                    .set_managed_activation_test_hook(Some(Arc::new(move |target_app, phase| {
                        if target_app == app && phase == "before_proof" {
                            std::fs::write(&target, external).unwrap();
                        }
                        Ok(())
                    })));
            } else {
                activate_target(&state, &auth, app, &account.identity_id, "first").unwrap();
                std::fs::write(&path, external).unwrap();
            }
            let backup_before = state.db.get_live_backup_sync(app).unwrap();
            let result = activate_target(&state, &auth, app, &account.identity_id, "second");
            assert!(result.is_err(), "{app} during_proof={during_proof}");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
            if !during_proof {
                assert_eq!(
                    state
                        .db
                        .get_live_backup_sync(app)
                        .unwrap()
                        .unwrap()
                        .original_config,
                    backup_before.unwrap().original_config
                );
            } else {
                assert!(matches!(
                    result,
                    Err(BindXaiManagedError::RollbackPartialStateUnknown)
                ));
            }
            state.proxy_service.set_managed_activation_test_hook(None);
            assert!(runtime
                .block_on(state.proxy_service.recover_from_crash())
                .is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
            if runtime.block_on(state.proxy_service.is_running()) {
                runtime.block_on(state.proxy_service.stop()).unwrap();
            }
        }
    }
}

#[test]
#[serial]
fn subscription_managed_restart_keeps_proof_exact_preimage_and_secret_free_provider() {
    for app in ["opencode", "claude", "codex", "grokbuild"] {
        let (_home, _guard, state, auth) = fixture();
        let account = seed(&auth, "selected");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        ephemeral(&runtime, &state);
        let path = native_target(app);
        let original = std::fs::read(&path).unwrap();
        let bound = activate_target(&state, &auth, app, &account.identity_id, "model").unwrap();
        let saved = serde_json::to_value(
            state
                .db
                .get_provider_by_id(&bound.provider_id, app)
                .unwrap(),
        )
        .unwrap();
        runtime
            .block_on(state.proxy_service.stop_with_restore_keep_state())
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original, "{app}");
        assert!(ProviderService::resume_managed_proxy(&state, app.parse().unwrap()).unwrap());
        assert_eq!(
            serde_json::to_value(
                state
                    .db
                    .get_provider_by_id(&bound.provider_id, app)
                    .unwrap()
            )
            .unwrap(),
            saved
        );
        assert!(!saved.to_string().contains("native-key"));
        assert!(state.db.get_live_backup_sync(app).unwrap().is_some());
        std::fs::write(&path, "external edit").unwrap();
        assert!(runtime
            .block_on(state.proxy_service.stop_with_restore())
            .is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external edit");
    }
}

#[test]
#[serial]
fn subscription_managed_restore_preserves_catalog_edited_after_first_file() {
    let (_home, _guard, state, auth) = fixture();
    let account = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    native_target("codex");
    activate_target(&state, &auth, "codex", &account.identity_id, "model").unwrap();
    let catalog = crate::codex_config::get_codex_model_catalog_path();
    let target = catalog.clone();
    state
        .proxy_service
        .set_managed_activation_test_hook(Some(Arc::new(move |app, phase| {
            if app == "codex" && phase == "restored_first_file" {
                std::fs::write(&target, "external catalog edit").unwrap();
            }
            Ok(())
        })));
    assert!(runtime
        .block_on(state.proxy_service.stop_with_restore())
        .is_err());
    assert_eq!(
        std::fs::read_to_string(catalog).unwrap(),
        "external catalog edit"
    );
    assert!(state.db.get_live_backup_sync("codex").unwrap().is_some());
    state.proxy_service.set_managed_activation_test_hook(None);
}

#[test]
#[serial]
fn subscription_does_not_adopt_legacy_proxy_placeholders_as_native_preimage() {
    let (_home, _guard, state, auth) = fixture();
    let account = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    let path = native_target("claude");
    let original: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state
        .db
        .save_live_backup_sync("claude", &original.to_string())
        .unwrap();
    let proxy = b"{\"env\":{\"ANTHROPIC_BASE_URL\":\"http://127.0.0.1:15721\",\"ANTHROPIC_AUTH_TOKEN\":\"PROXY_MANAGED\"}}";
    std::fs::write(&path, proxy).unwrap();
    assert!(activate_target(&state, &auth, "claude", &account.identity_id, "model").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), proxy);
    assert_eq!(
        state
            .db
            .get_live_backup_sync("claude")
            .unwrap()
            .unwrap()
            .original_config,
        original.to_string()
    );
}

#[test]
#[serial]
fn subscription_upgrade_recovers_unchanged_legacy_binding() {
    for app in ["claude", "codex", "grokbuild"] {
        let (_home, _guard, state, auth) = fixture();
        let account = seed(&auth, "selected");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        ephemeral(&runtime, &state);
        native_target(app);
        activate_target(&state, &auth, app, &account.identity_id, "model").unwrap();
        let wrapped: Value = serde_json::from_str(
            &state
                .db
                .get_live_backup_sync(app)
                .unwrap()
                .unwrap()
                .original_config,
        )
        .unwrap();
        let legacy = wrapped["fyagentManagedRestore"]["config"].to_string();
        state.db.save_live_backup_sync(app, &legacy).unwrap();
        runtime.block_on(state.proxy_service.stop()).unwrap();
        runtime
            .block_on(state.proxy_service.recover_from_crash())
            .unwrap_or_else(|error| panic!("unchanged v0.4.5 {app} binding must recover: {error}"));
        assert!(ProviderService::resume_managed_proxy(&state, app.parse().unwrap()).unwrap());
        runtime
            .block_on(state.proxy_service.stop_with_restore())
            .unwrap();
    }
}

fn legacy_backup(state: &AppState, app: &str) -> String {
    let wrapped: Value = serde_json::from_str(
        &state
            .db
            .get_live_backup_sync(app)
            .unwrap()
            .unwrap()
            .original_config,
    )
    .unwrap();
    let legacy = wrapped["fyagentManagedRestore"]["config"].to_string();
    state.db.save_live_backup_sync(app, &legacy).unwrap();
    legacy
}

fn assert_legacy_upgrade_shapes(app: &str) {
    for scenario in ["single", "absent", "null", "rebind", "source-change"] {
        if scenario == "null" && app != "claude" {
            continue;
        }
        let (_home, _guard, state, auth) = fixture();
        let account = seed_provider(&auth, "selected", ManagedAuthProvider::Xai);
        let other = seed_provider(&auth, "other", ManagedAuthProvider::Openai);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        ephemeral(&runtime, &state);
        let path = native_target(app);
        if scenario == "null" {
            std::fs::write(&path, " null \n").unwrap();
        }
        let original = std::fs::read(&path).unwrap();
        if scenario == "absent" {
            std::fs::remove_file(&path).unwrap();
        }
        let native_auth = crate::codex_config::get_codex_auth_path();
        std::fs::create_dir_all(native_auth.parent().unwrap()).unwrap();
        let auth_bytes = br#"{"auth_mode":"chatgpt","tokens":{"access_token":"native-access","refresh_token":"native-refresh","id_token":"native-id"}}"#;
        std::fs::write(&native_auth, auth_bytes).unwrap();
        activate_target(&state, &auth, app, &account.identity_id, "first").unwrap();
        if matches!(scenario, "rebind" | "source-change") {
            activate_target(&state, &auth, app, &account.identity_id, "second").unwrap();
        }
        if scenario == "source-change" {
            activate_target(&state, &auth, app, &other.identity_id, "third").unwrap();
        }
        legacy_backup(&state, app);
        runtime.block_on(state.proxy_service.stop()).unwrap();
        runtime
            .block_on(state.proxy_service.recover_from_crash())
            .unwrap_or_else(|error| panic!("{app}/{scenario}: {error}"));
        if scenario == "absent" {
            assert!(
                !path.exists(),
                "{app}: originally absent config must stay absent"
            );
        } else if app == "claude" && matches!(scenario, "rebind" | "source-change") {
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap(),
                serde_json::from_slice::<Value>(&original).unwrap(),
                "{app}/{scenario}"
            );
        } else {
            assert_eq!(std::fs::read(&path).unwrap(), original, "{app}/{scenario}");
        }
        assert_eq!(std::fs::read(&native_auth).unwrap(), auth_bytes);
        assert!(
            ProviderService::resume_managed_proxy(&state, app.parse().unwrap()).unwrap(),
            "{app}/{scenario}"
        );
        runtime
            .block_on(state.proxy_service.stop_with_restore())
            .unwrap();
        assert_eq!(std::fs::read(&native_auth).unwrap(), auth_bytes);
    }
}

#[test]
#[serial]
fn subscription_upgrade_claude_preserves_single_absent_null_rebind_and_source_change() {
    assert_legacy_upgrade_shapes("claude");
}

#[test]
#[serial]
fn subscription_upgrade_codex_preserves_single_absent_rebind_and_source_change() {
    assert_legacy_upgrade_shapes("codex");
}

#[test]
#[serial]
fn subscription_upgrade_grok_preserves_single_absent_rebind_and_source_change() {
    assert_legacy_upgrade_shapes("grokbuild");
}

#[test]
#[serial]
fn subscription_upgrade_rebind_retains_legacy_database_restore_authority() {
    let (_home, _guard, state, auth) = fixture();
    let account = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    let path = native_target("claude");
    activate_target(&state, &auth, "claude", &account.identity_id, "first").unwrap();
    activate_target(&state, &auth, "claude", &account.identity_id, "second").unwrap();
    let mut original: Value = serde_json::from_str(&legacy_backup(&state, "claude")).unwrap();
    // Both subscription projections mask these fields, and the rolling
    // receipt no longer has the first native preimage. v0.4.5 did not retain
    // an independent digest, so its DB backup remains the restore authority.
    original["env"]["ANTHROPIC_API_KEY"] = json!("legacy-database-original-key");
    original["env"]["ANTHROPIC_BASE_URL"] = json!("https://legacy-origin.example");
    state
        .db
        .save_live_backup_sync("claude", &original.to_string())
        .unwrap();
    runtime.block_on(state.proxy_service.stop()).unwrap();
    runtime
        .block_on(state.proxy_service.recover_from_crash())
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap(),
        original
    );
}

#[test]
#[serial]
fn subscription_upgrade_does_not_adopt_later_fyagent_edits_or_tampered_receipts() {
    for app in ["claude", "codex", "grokbuild"] {
        for scenario in [
            "mcp",
            "endpoint-port",
            "endpoint-path",
            "rolling-backup",
            "receipt",
            "database-backup",
        ] {
            let (_home, _guard, state, auth) = fixture();
            let account = seed(&auth, "selected");
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let _entered = runtime.enter();
            ephemeral(&runtime, &state);
            let path = native_target(app);
            activate_target(&state, &auth, app, &account.identity_id, "model").unwrap();
            let mut legacy = legacy_backup(&state, app);
            match scenario {
                "mcp" => {
                    if app == "claude" {
                        let mut value: Value =
                            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                        value["permissions"]["allow"] = json!(["Read(added-later)"]);
                        crate::config::write_json_file(&path, &value).unwrap();
                    } else {
                        let current = std::fs::read_to_string(&path).unwrap();
                        crate::config::write_text_file(
                            &path,
                            &format!("{current}\n[mcp_servers.added_later]\ncommand = 'keep-me'\n"),
                        )
                        .unwrap();
                    }
                    assert!(
                        crate::config::file_recovery(&path)
                            .unwrap()
                            .unwrap()
                            .can_restore
                    );
                }
                "endpoint-port" | "endpoint-path" => {
                    let port = runtime
                        .block_on(state.proxy_service.get_status())
                        .unwrap()
                        .port;
                    let origin = format!("http://127.0.0.1:{port}");
                    let altered = if scenario == "endpoint-port" {
                        format!("http://127.0.0.1:{}", if port == 1 { 2 } else { 1 })
                    } else {
                        format!("{origin}/unproven")
                    };
                    let current = std::fs::read_to_string(&path).unwrap();
                    assert!(current.contains(&origin));
                    crate::config::write_text_file(&path, &current.replace(&origin, &altered))
                        .unwrap();
                    assert!(
                        crate::config::file_recovery(&path)
                            .unwrap()
                            .unwrap()
                            .can_restore
                    );
                }
                "rolling-backup" => {
                    std::fs::write(crate::config::rolling_backup_path(&path), "tampered-backup")
                        .unwrap()
                }
                "receipt" => {
                    let marker = path.with_file_name(format!(
                        "{}.fyagent.undo.json",
                        path.file_name().unwrap().to_str().unwrap()
                    ));
                    std::fs::write(marker, "{}").unwrap();
                }
                _ => {
                    let mut value: Value = serde_json::from_str(&legacy).unwrap();
                    if app == "claude" {
                        value["permissions"]["unproven"] = json!(true);
                    } else {
                        value["config"] = json!("unproven = true\n");
                    }
                    legacy = value.to_string();
                    state.db.save_live_backup_sync(app, &legacy).unwrap();
                }
            }
            let before = std::fs::read(&path).unwrap();
            assert!(
                runtime
                    .block_on(state.proxy_service.recover_from_crash())
                    .is_err(),
                "{app}/{scenario}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
            assert_eq!(
                state
                    .db
                    .get_live_backup_sync(app)
                    .unwrap()
                    .unwrap()
                    .original_config,
                legacy
            );
            runtime.block_on(state.proxy_service.stop()).unwrap();
        }
    }
}

#[test]
#[serial]
fn subscription_upgrade_refuses_unproven_legacy_backup_but_keeps_recovery_evidence() {
    for app in ["claude", "codex", "grokbuild"] {
        let (_home, _guard, state, auth) = fixture();
        let account = seed(&auth, "selected");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        ephemeral(&runtime, &state);
        let path = native_target(app);
        activate_target(&state, &auth, app, &account.identity_id, "model").unwrap();
        let wrapped: Value = serde_json::from_str(
            &state
                .db
                .get_live_backup_sync(app)
                .unwrap()
                .unwrap()
                .original_config,
        )
        .unwrap();
        let legacy = wrapped["fyagentManagedRestore"]["config"].to_string();
        state.db.save_live_backup_sync(app, &legacy).unwrap();
        std::fs::write(&path, "external edit after legacy binding").unwrap();
        assert!(
            runtime
                .block_on(state.proxy_service.recover_from_crash())
                .is_err(),
            "{app}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "external edit after legacy binding"
        );
        assert_eq!(
            state
                .db
                .get_live_backup_sync(app)
                .unwrap()
                .unwrap()
                .original_config,
            legacy
        );
        assert!(ProviderService::resume_managed_proxy(&state, app.parse().unwrap()).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "external edit after legacy binding"
        );
        assert_eq!(
            state
                .db
                .get_live_backup_sync(app)
                .unwrap()
                .unwrap()
                .original_config,
            legacy
        );
        if runtime.block_on(state.proxy_service.is_running()) {
            runtime.block_on(state.proxy_service.stop()).unwrap();
        }
    }
}

#[test]
#[serial]
fn subscription_opencode_api_writer_and_backup_compensation_are_isolated() {
    let (_home, _guard, state, auth) = fixture();
    let account = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    let (path, original, _, _) = write_native();
    let bound = activate_target(&state, &auth, "opencode", &account.identity_id, "model").unwrap();
    let live = std::fs::read(&path).unwrap();
    let make_request = |name: &str| crate::services::opencode_models::SaveOpenCodeModelsRequest {
        provider_id: None,
        provider_name: name.into(),
        base_url: "https://api.example.com/v1".into(),
        api_key: "ordinary-api-key".into(),
        selected_model_ids: vec!["another".into()],
        removed_model_ids: vec![],
        expected_revision: revision(),
        overwrite_token: None,
    };
    assert!(
        crate::services::opencode_models::save_opencode_models(make_request("Ordinary")).is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), live);
    runtime
        .block_on(state.proxy_service.set_takeover_for_app("opencode", false))
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(
        crate::services::opencode_models::save_opencode_models(make_request(&bound.provider_id))
            .is_err()
    );
    let sidecar = crate::opencode_config::get_opencode_dir().join("opencode.json.backup");
    let target = sidecar.clone();
    state
        .proxy_service
        .set_managed_activation_test_hook(Some(Arc::new(move |app, phase| {
            if app == "opencode" && phase == "projected" {
                std::fs::write(&target, "external backup edit").unwrap();
                return Err("injected DB failure".into());
            }
            Ok(())
        })));
    assert!(matches!(
        activate_target(&state, &auth, "opencode", &account.identity_id, "second"),
        Err(BindXaiManagedError::RollbackPartialStateUnknown)
    ));
    assert_eq!(
        std::fs::read_to_string(sidecar).unwrap(),
        "external backup edit"
    );
    assert!(state.db.get_live_backup_sync("opencode").unwrap().is_some());
    state.proxy_service.set_managed_activation_test_hook(None);
    if runtime.block_on(state.proxy_service.is_running()) {
        runtime.block_on(state.proxy_service.stop()).unwrap();
    }
}

#[test]
#[serial]
fn subscription_opencode_invalid_admission_backup_and_commit_failures_do_not_mutate() {
    let (_home, _guard, state, auth) = fixture();
    let selected = seed(&auth, "selected");
    let (path, original, _, _) = write_native();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    for (account, model, expected_revision) in [
        ("missing".to_string(), "model".to_string(), revision()),
        (
            selected.identity_id.to_uppercase(),
            "model".into(),
            revision(),
        ),
        (
            selected.identity_id.clone(),
            "invalid/model".into(),
            revision(),
        ),
        (
            selected.identity_id.clone(),
            "model".into(),
            Some("bad-revision".into()),
        ),
    ] {
        assert!(matches!(
            ProviderService::bind_opencode_managed_proxy(
                &state,
                &auth,
                BindOpenCodeManagedProxyRequest {
                    account_id: account,
                    model_id: model,
                    expected_revision
                }
            ),
            Err(BindXaiManagedError::InvalidRequest)
        ));
    }
    assert!(matches!(
        ProviderService::bind_managed_proxy(
            &state,
            &auth,
            request("opencode", &selected.identity_id)
        ),
        Err(BindXaiManagedError::InvalidRequest)
    ));
    let backup = crate::opencode_config::get_opencode_dir().join("opencode.json.backup");
    std::fs::create_dir(&backup).unwrap();
    assert!(matches!(
        ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            bind_request(&selected.identity_id, "model")
        ),
        Err(BindXaiManagedError::ApplyFailedRolledBack)
    ));
    std::fs::remove_dir(&backup).unwrap();
    state.db.conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_opencode BEFORE INSERT ON providers WHEN NEW.app_type='opencode' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(matches!(
        ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            bind_request(&selected.identity_id, "model")
        ),
        Err(BindXaiManagedError::ApplyFailedRolledBack)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(state.db.get_all_providers("opencode").unwrap().is_empty());
    assert!(runtime
        .block_on(state.db.get_live_backup("opencode"))
        .unwrap()
        .is_none());
    assert!(!runtime.block_on(state.proxy_service.is_running()));
}

#[test]
#[serial]
fn subscription_opencode_concurrent_revision_admission_only_commits_one_selection() {
    let (_home, _guard, state, auth) = fixture();
    let selected = seed(&auth, "selected");
    write_native();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    let captured_revision = revision();
    let start = Arc::new(std::sync::Barrier::new(3));
    let mut jobs = Vec::new();
    for model in ["first", "second"] {
        let (state, auth, start, account, revision, handle) = (
            state.clone(),
            auth.clone(),
            start.clone(),
            selected.identity_id.clone(),
            captured_revision.clone(),
            runtime.handle().clone(),
        );
        jobs.push(std::thread::spawn(move || {
            let _entered = handle.enter();
            start.wait();
            ProviderService::bind_opencode_managed_proxy(
                &state,
                &auth,
                BindOpenCodeManagedProxyRequest {
                    account_id: account,
                    model_id: model.into(),
                    expected_revision: revision,
                },
            )
        }));
    }
    start.wait();
    let results: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(BindXaiManagedError::ProviderConflict)))
            .count(),
        1
    );
    assert_eq!(state.db.get_all_providers("opencode").unwrap().len(), 1);
    runtime
        .block_on(state.proxy_service.stop_with_restore())
        .unwrap();
}

#[test]
#[serial]
fn subscription_opencode_stop_restart_and_crash_restore_preserve_native_bytes() {
    let (_home, _guard, state, auth) = fixture();
    let selected = seed(&auth, "selected");
    let (path, original, auth_path, auth_bytes) = write_native();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    ephemeral(&runtime, &state);
    ProviderService::bind_opencode_managed_proxy(
        &state,
        &auth,
        bind_request(&selected.identity_id, "model"),
    )
    .unwrap();
    let projected = std::fs::read(&path).unwrap();
    runtime.block_on(state.proxy_service.stop()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), projected);
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(false)
    );
    runtime.block_on(state.proxy_service.start()).unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(true)
    );
    runtime
        .block_on(state.proxy_service.stop_with_restore_keep_state())
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(
        runtime
            .block_on(state.db.get_proxy_config_for_app("opencode"))
            .unwrap()
            .enabled
    );
    assert!(ProviderService::resume_managed_proxy(&state, AppType::OpenCode).unwrap());
    runtime.block_on(state.proxy_service.stop()).unwrap();
    let restarted = crate::services::ProxyService::new(state.db.clone());
    runtime.block_on(restarted.recover_from_crash()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read(auth_path).unwrap(), auth_bytes);
    runtime.block_on(restarted.recover_from_crash()).unwrap();
}

#[test]
#[serial]
fn subscription_managed_shutdown_and_transaction_rollback_refuse_external_edits() {
    for app in ["opencode", "claude", "codex", "grokbuild"] {
        for fail_during_commit in [false, true] {
            let (_home, _guard, state, auth) = fixture();
            let selected = seed(&auth, "selected");
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let _entered = runtime.enter();
            ephemeral(&runtime, &state);
            let path = match app {
                "opencode" => write_native().0,
                "claude" => crate::config::get_claude_settings_path(),
                "codex" => {
                    let path = crate::codex_config::get_codex_config_path();
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(&path, "model = \"original\"\n").unwrap();
                    path
                }
                _ => crate::grok_config::get_grok_config_path(),
            };
            let external = if matches!(app, "grokbuild" | "codex") {
                "# external MCP edit\n[models]\ndefault=\"external\"\n"
            } else {
                "{\"external\":\"new permissions and MCP\"}"
            };
            if fail_during_commit {
                let target = path.clone();
                state
                    .proxy_service
                    .set_managed_activation_test_hook(Some(Arc::new(move |target_app, phase| {
                        if target_app == app && phase == "projected" {
                            std::fs::write(&target, external).unwrap();
                            return Err("injected failure after external edit".into());
                        }
                        Ok(())
                    })));
            }
            let result = if app == "opencode" {
                ProviderService::bind_opencode_managed_proxy(
                    &state,
                    &auth,
                    bind_request(&selected.identity_id, "model"),
                )
            } else if app == "codex" {
                let bound = ProviderService::bind_managed_proxy(
                    &state,
                    &auth,
                    request(app, &selected.identity_id),
                )
                .unwrap();
                let provider = state
                    .db
                    .get_provider_by_id(&bound.provider_id, app)
                    .unwrap()
                    .unwrap();
                let _lock = ProviderService::lock_provider_mutation(&state, &AppType::Codex);
                ProviderService::apply_quick_setup_with_lock_held(&state, AppType::Codex, provider)
                    .map(|_| bound)
                    .map_err(BindXaiManagedError::from)
            } else {
                ProviderService::bind_managed_proxy(
                    &state,
                    &auth,
                    request(app, &selected.identity_id),
                )
            };
            if fail_during_commit {
                assert!(
                    matches!(
                        result,
                        Err(BindXaiManagedError::RollbackPartialStateUnknown)
                    ),
                    "{app}: {result:?}"
                );
                state.proxy_service.set_managed_activation_test_hook(None);
            } else {
                result.unwrap_or_else(|error| panic!("{app}: {error:?}"));
                std::fs::write(&path, external).unwrap();
                assert!(runtime
                    .block_on(state.proxy_service.stop_with_restore())
                    .is_err());
                assert!(runtime
                    .block_on(state.proxy_service.recover_from_crash())
                    .is_err());
            }
            assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
            assert!(runtime
                .block_on(state.db.get_live_backup(app))
                .unwrap()
                .is_some());
            if runtime.block_on(state.proxy_service.is_running()) {
                runtime.block_on(state.proxy_service.stop()).unwrap();
            }
        }
    }
}
