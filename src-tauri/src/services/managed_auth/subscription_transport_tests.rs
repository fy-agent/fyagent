use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
#[serial]
fn health_admission_collect_preserves_seeded_database_native_files_and_vault() {
    use crate::services::external_agents::AgentCatalogId;
    use crate::services::secret::MemoryFailureMode;

    let home = tempfile::tempdir().unwrap();
    let _home_guard = TestHome::set(home.path());
    // Restore process-local settings while the temporary home is still active.
    // No write in setup, observation or teardown can target the user's home.
    struct SettingsRestore(crate::settings::AppSettings);
    impl Drop for SettingsRestore {
        fn drop(&mut self) {
            let _ = crate::settings::update_settings(self.0.clone());
        }
    }
    let _settings_guard = SettingsRestore(crate::settings::get_settings());
    crate::settings::update_settings(crate::settings::AppSettings::default()).unwrap();
    let db = Arc::new(Database::memory().unwrap());
    let backend = MemorySecretBackend::new();
    let auth = Arc::new(ManagedAuthService::new(
        db.clone(),
        SecretService::new(backend.clone()),
        home.path().join("vault-meta"),
    ));
    seed_provider(&auth, "health-fixture", ManagedAuthProvider::Openai);
    let state = AppState::new(db.clone());
    // OpenCode's desktop inventory is confined to this temporary home. Unlike
    // CLI inventory, this path never needs a login shell or Agent execution.
    let config_path = home.path().join(".config/opencode/opencode.json");
    let codex_path = auth.codex_home().join("auth.json");
    let opencode_auth_path = auth.opencode_auth_path();
    let fixtures: Vec<(_, &[u8])> = vec![
        (
            config_path,
            br#"{"model":"fixture/model","provider":{"fixture":{"npm":"@ai-sdk/openai-compatible","options":{"apiKey":"fixture-config-secret","baseURL":"https://fixture.invalid/v1"},"models":{"model":{}}}}}"#,
        ),
        (
            codex_path,
            br#"{"auth_mode":"chatgpt","tokens":{"access_token":"fixture-native-access","refresh_token":"fixture-native-refresh","id_token":"fixture-native-id"}}"#,
        ),
        (
            opencode_auth_path,
            br#"{"openai":{"type":"api","key":"fixture-opencode-secret"}}"#,
        ),
    ];
    for (path, bytes) in &fixtures {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let settings_path = home.path().join(".fyagent/settings.json");
    let settings_before = std::fs::read(&settings_path).unwrap();
    let sql_snapshot = || {
        db.export_sql_string()
            .unwrap()
            .lines()
            // Export metadata changes with wall time; business timestamps do not.
            .filter(|line| !line.starts_with("-- 生成时间:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let before = sql_snapshot();
    backend.set_mode(MemoryFailureMode::Denied);
    let vault_operations = backend.operation_count();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for _ in 0..3 {
        let snapshot = runtime
            .block_on(crate::services::health::read(
                AgentCatalogId::OpenCode,
                state.clone(),
                auth.clone(),
            ))
            .expect("the real admission and collector must complete");
        let wire = serde_json::to_value(snapshot).unwrap();
        assert_eq!(wire["agentId"], "opencode");
        assert_eq!(wire["checks"].as_array().unwrap().len(), 12);
        assert!(wire["checks"].as_array().unwrap().iter().any(|check| {
            check["id"] == "configuration" && check["reasonCode"] == "configuration_present"
        }));
        for secret in [
            "synthetic-refresh-health-fixture",
            "synthetic-access-health-fixture",
            "fixture-config-secret",
            "fixture-native-access",
            "fixture-native-refresh",
            "fixture-opencode-secret",
            "secretRef",
        ] {
            assert!(!wire.to_string().contains(secret));
        }
        assert_eq!(sql_snapshot(), before);
        assert_eq!(backend.operation_count(), vault_operations);
        assert_eq!(std::fs::read(&settings_path).unwrap(), settings_before);
        for (path, bytes) in &fixtures {
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }
        assert!(auth.repository.list_connections().unwrap().is_empty());
    }
}

#[test]
#[serial]
fn subscription_401_replays_once_with_same_lineage_and_never_uses_default_account() {
    for provider in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        for reject_retry in [false, true] {
            let (_home, _guard, state, auth) = fixture();
            let selected = seed_provider(&auth, "selected", provider);
            seed_provider(&auth, "other-default", provider);
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let _entered = runtime.enter();
            let mut global = runtime
                .block_on(state.db.get_global_proxy_config())
                .unwrap();
            global.listen_address = "127.0.0.1".into();
            global.listen_port = 0;
            runtime
                .block_on(state.db.update_global_proxy_config(global))
                .unwrap();
            let binding = ProviderService::bind_managed_proxy(
                &state,
                &auth,
                request("claude", &selected.identity_id),
            )
            .unwrap();
            let port = runtime
                .block_on(state.proxy_service.get_status())
                .unwrap()
                .port;
            let calls = Arc::new(AtomicUsize::new(0));
            let capture_calls = calls.clone();
            let rotate_auth = auth.clone();
            let (upstream, server_task) = runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let path = if provider == ManagedAuthProvider::Openai {
                    "/backend-api/codex/responses"
                } else {
                    "/v1/responses"
                };
                let router = axum::Router::new().route(
                    path,
                    axum::routing::post(
                        move |headers: axum::http::HeaderMap,
                              axum::Json(body): axum::Json<Value>| {
                            let calls = capture_calls.clone();
                            let auth = rotate_auth.clone();
                            let credential = selected.clone();
                            async move {
                                use axum::response::IntoResponse;
                                let attempt = calls.fetch_add(1, Ordering::SeqCst);
                                assert!(attempt < 2, "a subscription must never loop on 401");
                                let expected = if attempt == 0 {
                                    "Bearer synthetic-access-selected"
                                } else {
                                    "Bearer synthetic-retry-access"
                                };
                                assert_eq!(headers["authorization"], expected);
                                if provider == ManagedAuthProvider::Openai {
                                    assert_eq!(headers["chatgpt-account-id"], "selected");
                                }
                                if attempt == 0 {
                                    // Model another request completing the shared refresh
                                    // between this request's send and its 401 response.
                                    // The retry owner must adopt only this same lineage.
                                    tokio::task::spawn_blocking(move || {
                                        let bundle = ManagedAuthSecretBundle::new(
                                            ManagedAuthSecretBundleParts {
                                                credential_id: credential.credential_id.clone(),
                                                provider,
                                                generation: credential.generation + 1,
                                                access_token: Some("synthetic-retry-access".into()),
                                                refresh_token: Some(
                                                    "synthetic-retry-refresh".into(),
                                                ),
                                                id_token: None,
                                                token_type: Some("Bearer".into()),
                                                granted_scopes: Vec::new(),
                                                issued_at: None,
                                                expires_at: Some(
                                                    chrono::Utc::now().timestamp() + 3600,
                                                ),
                                            },
                                        )
                                        .unwrap();
                                        assert!(auth
                                            .replace_bundle_cas(
                                                &credential.credential_id,
                                                credential.generation,
                                                RefreshOwner::Fyagent,
                                                bundle
                                            )
                                            .unwrap());
                                    })
                                    .await
                                    .unwrap();
                                }
                                if attempt == 0 || reject_retry {
                                    (axum::http::StatusCode::UNAUTHORIZED, "synthetic rejection")
                                        .into_response()
                                } else {
                                    let (response, events) = synthetic_responses("grok-build");
                                    if body["stream"] == true {
                                        (
                                            [(
                                                axum::http::header::CONTENT_TYPE,
                                                "text/event-stream",
                                            )],
                                            events,
                                        )
                                            .into_response()
                                    } else {
                                        axum::Json(response).into_response()
                                    }
                                }
                            }
                        },
                    ),
                );
                let task = tokio::spawn(async move {
                    axum::serve(listener, router).await.unwrap();
                });
                (format!("http://{address}"), task)
            });
            crate::proxy::set_xai_integration_fixture(
                &binding.provider_id,
                Some(crate::proxy::XaiIntegrationFixture {
                    auth: auth.clone(),
                    upstream,
                }),
            );
            runtime.block_on(async {
                let response = reqwest::Client::new().post(format!("http://127.0.0.1:{port}/v1/messages"))
                    .timeout(std::time::Duration::from_secs(10))
                    .header("x-api-key", "PROXY_MANAGED")
                    .json(&json!({"model":"claude-sonnet-4-6","max_tokens":100,"messages":[{"role":"user","content":"hello"}],"stream":false}))
                    .send().await.unwrap();
                let status = response.status();
                let body = response.text().await.unwrap();
                assert_eq!(status.is_success(), !reject_retry, "{provider:?} retry={reject_retry}: {status}; {body}");
                assert!(!body.contains("synthetic-retry-access"));
                assert!(!body.contains("synthetic-access-selected"));
                assert_eq!(calls.load(Ordering::SeqCst), 2);
                assert_eq!(state.proxy_service.get_status().await.unwrap().failover_count, 0);
                state.proxy_service.stop_with_restore().await.unwrap();
            });
            crate::proxy::set_xai_integration_fixture(&binding.provider_id, None);
            server_task.abort();
        }
    }
}

#[test]
#[serial]
fn subscription_observation_distinguishes_saved_stopped_adopted_and_unknown() {
    let (_home, _guard, state, auth) = fixture();
    // The settings cache is process-local, unlike this fixture's fresh DB.
    // Start with no selections from preceding serial tests. Observation must
    // report stale selections as unknown, never silently repair that leakage.
    for app in [AppType::Claude, AppType::Codex, AppType::GrokBuild] {
        crate::settings::set_current_provider(&app, None).unwrap();
    }
    let selected = seed(&auth, "selected");
    auth.upsert_proxy_connections().unwrap();
    let overview = auth.overview();
    assert!(overview
        .connections
        .iter()
        .filter(|row| row.consumer == ManagedAuthConsumer::FyagentProxy)
        .all(|row| row.auth_status != ManagedAuthConnectionState::Connected));
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(false)
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let mut global = runtime
        .block_on(state.db.get_global_proxy_config())
        .unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = 0;
    runtime
        .block_on(state.db.update_global_proxy_config(global))
        .unwrap();
    let binding = ProviderService::bind_managed_proxy(
        &state,
        &auth,
        request("claude", &selected.identity_id),
    )
    .unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(true)
    );
    // The request router's effective selection takes precedence over a stale
    // database current marker; observation must consult the same owner.
    let other = crate::provider::Provider::with_id(
        "other-route".into(),
        "Other".into(),
        json!({"env": {}}),
        None,
    );
    state.db.save_provider("claude", &other).unwrap();
    crate::settings::set_current_provider(&AppType::Claude, Some(&other.id)).unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(false)
    );
    // Observation must not repair a stale user selection or fall back to the
    // database's prior managed route. Unknown preserves the evidence boundary.
    crate::settings::set_current_provider(&AppType::Claude, Some("missing-fixture-provider"))
        .unwrap();
    let snapshot = || {
        state
            .db
            .export_sql_string()
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("-- 生成时间:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let before = snapshot();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        None
    );
    assert_eq!(
        crate::settings::get_current_provider(&AppType::Claude).as_deref(),
        Some("missing-fixture-provider")
    );
    assert_eq!(snapshot(), before);
    crate::settings::set_current_provider(&AppType::Claude, Some(&binding.provider_id)).unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(true)
    );
    let activation = runtime.block_on(
        state
            .proxy_service
            .lock_managed_activation(&AppType::Claude),
    );
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        None
    );
    drop(activation);
    let path = crate::config::get_claude_settings_path();
    let owned = std::fs::read(&path).unwrap();
    std::fs::write(&path, "not JSON").unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        None
    );
    std::fs::write(
        &path,
        b"{\"env\":{\"ANTHROPIC_BASE_URL\":\"https://other.example\"}}",
    )
    .unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(false)
    );
    std::fs::write(&path, owned).unwrap();
    runtime
        .block_on(state.proxy_service.stop_with_restore())
        .unwrap();
    assert_eq!(
        state
            .proxy_service
            .observe_managed_account_route("xai_oauth", "selected", true),
        Some(false)
    );
}
