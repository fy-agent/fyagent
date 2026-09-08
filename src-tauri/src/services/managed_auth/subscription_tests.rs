//! Isolated subscription integration: synthetic vault -> real Provider/Change
//! Plan -> real loopback Proxy -> fake vendor HTTP. No external login or file.

use super::migration::LegacyCredentialInput;
use super::*;
use crate::app_config::AppType;
use crate::database::Database;
use crate::services::change_plan::{ChangePlanService, WriterReceipt};
use crate::services::provider::{BindXaiManagedError, BindXaiManagedRequest, ProviderService};
use crate::services::secret::{MemorySecretBackend, SecretService};
use crate::store::AppState;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;

struct TestHome(Option<std::ffi::OsString>);
impl TestHome {
    fn set(path: &std::path::Path) -> Self {
        let previous = std::env::var_os("FYAGENT_TEST_HOME");
        std::env::set_var("FYAGENT_TEST_HOME", path);
        Self(previous)
    }
}
impl Drop for TestHome {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
    }
}

fn seed(auth: &ManagedAuthService<MemorySecretBackend>, account: &str) -> CredentialRecord {
    let credential = auth
        .provision_legacy_credential(LegacyCredentialInput {
            migration_id: None,
            provider: ManagedAuthProvider::Xai,
            purpose: CredentialPurpose::ProxyUpstream,
            consumer: Some(ManagedAuthConsumer::FyagentProxy),
            legacy_account_id: account.into(),
            provider_subject: account.into(),
            provider_tenant: String::new(),
            login: format!("{account}@example.test"),
            display_name: None,
            avatar_url: None,
            access_token: None,
            refresh_token: Some(zeroize::Zeroizing::new(format!(
                "synthetic-refresh-{account}"
            ))),
            id_token: None,
            desired_status: CredentialStatus::Ready,
            refresh_owner: RefreshOwner::Fyagent,
            authenticated_at: 1_700_000_000,
            make_default: true,
        })
        .unwrap();
    let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
        credential_id: credential.credential_id.clone(),
        provider: ManagedAuthProvider::Xai,
        generation: credential.generation + 1,
        access_token: Some(format!("synthetic-access-{account}")),
        refresh_token: Some(format!("synthetic-refresh-{account}")),
        id_token: None,
        token_type: Some("Bearer".into()),
        granted_scopes: vec![],
        issued_at: Some(chrono::Utc::now().timestamp()),
        expires_at: Some(chrono::Utc::now().timestamp() + 3600),
    })
    .unwrap();
    assert!(auth
        .replace_bundle_cas(
            &credential.credential_id,
            credential.generation,
            RefreshOwner::Fyagent,
            bundle
        )
        .unwrap());
    auth.repository
        .get_credential(&credential.credential_id)
        .unwrap()
        .unwrap()
}

fn request(app: &str, identity: &str) -> BindXaiManagedRequest {
    BindXaiManagedRequest {
        app: app.into(),
        account_id: identity.into(),
        model_id: "grok-build".into(),
    }
}

fn fixture() -> (
    tempfile::TempDir,
    TestHome,
    Arc<AppState>,
    Arc<ManagedAuthService<MemorySecretBackend>>,
) {
    let home = tempfile::tempdir().unwrap();
    let guard = TestHome::set(home.path());
    let db = Arc::new(Database::memory().unwrap());
    db.create_change_plan_tables_for_tests().unwrap();
    let auth = Arc::new(ManagedAuthService::new(
        db.clone(),
        SecretService::new(MemorySecretBackend::new()),
        home.path().join("vault-meta"),
    ));
    let state = Arc::new(AppState::new(db));
    (home, guard, state, auth)
}

#[test]
#[serial]
fn subscription_vault_binding_rejects_missing_purpose_and_conflicting_provider() {
    let (_home, _guard, state, auth) = fixture();
    assert!(matches!(
        ProviderService::bind_xai_managed(&state, &auth, request("codex", "missing")),
        Err(BindXaiManagedError::AccountUnavailable)
    ));
    let credential = seed(&auth, "selected");
    let bound =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &credential.identity_id))
            .unwrap();
    assert!(!bound.activated);
    let again =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &credential.identity_id))
            .unwrap();
    assert!(again.already_bound);
    let mut altered = state
        .db
        .get_provider_by_id(&bound.provider_id, "codex")
        .unwrap()
        .unwrap();
    altered.settings_config["config"] = json!("model = \"changed-model\"");
    state.db.save_provider("codex", &altered).unwrap();
    assert!(matches!(
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &credential.identity_id)),
        Err(BindXaiManagedError::ProviderConflict)
    ));
    assert_eq!(
        state
            .db
            .get_provider_by_id(&bound.provider_id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        altered.settings_config
    );
    auth.repository
        .transfer_refresh_owner(
            &credential.credential_id,
            credential.generation,
            RefreshOwner::Fyagent,
            RefreshOwner::GrokNative,
            chrono::Utc::now().timestamp(),
        )
        .unwrap();
    assert!(matches!(
        ProviderService::bind_xai_managed(
            &state,
            &auth,
            request("claude", &credential.identity_id)
        ),
        Err(BindXaiManagedError::AccountUnavailable)
    ));
}

#[test]
#[serial]
fn subscription_account_names_are_distinct_and_saved_names_are_idempotent() {
    let (_home, _guard, state, auth) = fixture();
    let first = seed(&auth, "selected");
    let second = seed(&auth, "other");
    state.db.conn.lock().unwrap().execute(
        "UPDATE managed_auth_identities SET display_name = 'Shared label' WHERE provider = 'xai'", [],
    ).unwrap();
    let bound =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &first.identity_id))
            .unwrap();
    let other =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &second.identity_id))
            .unwrap();
    assert!(bound.provider_name.contains("Shared label"));
    assert!(other.provider_name.contains("Shared label"));
    assert_ne!(bound.provider_name, other.provider_name);
    assert_ne!(bound.provider_id, other.provider_id);
    let mut renamed = state
        .db
        .get_provider_by_id(&bound.provider_id, "codex")
        .unwrap()
        .unwrap();
    renamed.name = "My preferred Grok source".into();
    state.db.save_provider("codex", &renamed).unwrap();
    state.db.conn.lock().unwrap().execute(
        "UPDATE managed_auth_identities SET display_name = 'New public label' WHERE identity_id = ?1", [&first.identity_id],
    ).unwrap();
    let repeated =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &first.identity_id))
            .unwrap();
    assert!(repeated.already_bound);
    assert_eq!(repeated.provider_name, renamed.name);
    assert_eq!(repeated.provider_id, bound.provider_id);
    renamed
        .meta
        .as_mut()
        .unwrap()
        .auth_binding
        .as_mut()
        .unwrap()
        .account_id = Some(second.legacy_account_id);
    state.db.save_provider("codex", &renamed).unwrap();
    assert!(matches!(
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &first.identity_id)),
        Err(BindXaiManagedError::ProviderConflict)
    ));
}

#[test]
#[serial]
fn subscription_concurrent_targets_do_not_stop_the_committed_listener() {
    check_concurrent_targets(true);
    check_concurrent_targets(false);
}

fn check_concurrent_targets(use_change_plan: bool) {
    use std::sync::mpsc;
    use std::time::Duration;

    let (_home, _guard, state, auth) = fixture();
    let credential = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut global = runtime
        .block_on(state.db.get_global_proxy_config())
        .unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = port;
    global.proxy_enabled = false;
    runtime
        .block_on(state.db.update_global_proxy_config(global))
        .unwrap();
    let claude_path = crate::config::get_claude_settings_path();
    crate::config::write_json_file(&claude_path, &json!({"env":{"CUSTOM":"preserved"}})).unwrap();
    let claude_before = std::fs::read(&claude_path).unwrap();
    let codex_path = crate::codex_config::get_codex_config_path();
    std::fs::create_dir_all(codex_path.parent().unwrap()).unwrap();
    std::fs::write(&codex_path, "model = \"original\"\n").unwrap();
    let codex =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &credential.identity_id))
            .unwrap();
    let plan = ChangePlanService::plan_codex_switch(&state, &codex.provider_id).unwrap();
    if !use_change_plan {
        state
            .db
            .set_current_provider("codex", &codex.provider_id)
            .unwrap();
        crate::settings::set_current_provider(&AppType::Codex, Some(&codex.provider_id)).unwrap();
    }

    // Freeze A after it starts the shared socket and fail that preparation.
    // B enters through Change Plan or the manual takeover while A is pending.
    let (events_tx, events_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    state
        .proxy_service
        .set_managed_activation_test_hook(Some(Arc::new(move |app, phase| {
            events_tx.send((app.to_owned(), phase.to_owned())).unwrap();
            if app == "claude" && phase == "started" {
                release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
                return Err("synthetic preparation failure".into());
            }
            Ok(())
        })));
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let _entered = runtime.enter();
            ProviderService::bind_xai_managed(
                &state,
                &auth,
                request("claude", &credential.identity_id),
            )
        });
        assert_eq!(
            events_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            ("claude".into(), "waiting".into())
        );
        assert_eq!(
            events_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            ("claude".into(), "started".into())
        );
        let second = scope.spawn(|| {
            let _entered = runtime.enter();
            if !use_change_plan {
                runtime
                    .block_on(state.proxy_service.set_takeover_for_app("codex", true))
                    .unwrap();
                return None;
            }
            Some(
                ChangePlanService::apply_codex_switch_at_with_writer(
                    &state,
                    &plan.plan_id,
                    &plan.plan_digest,
                    chrono::Utc::now().timestamp(),
                    |id| {
                        ProviderService::with_live_config_result(AppType::Codex, || {
                            ProviderService::switch_with_lock_held(&state, AppType::Codex, id)
                        })
                        .map(|result| WriterReceipt {
                            live_config_changed: result.live_config_changed,
                        })
                    },
                )
                .unwrap(),
            )
        });
        assert_eq!(
            events_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            ("codex".into(), "waiting".into())
        );
        assert!(
            matches!(
                events_rx.recv_timeout(Duration::from_millis(150)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "B must not adopt the listener before A completes compensation"
        );
        release_tx.send(()).unwrap();
        assert!(matches!(
            first.join().unwrap(),
            Err(BindXaiManagedError::ApplyFailedRolledBack)
        ));
        if let Some(outcome) = second.join().unwrap() {
            assert!(matches!(
                serde_json::to_value(outcome).unwrap()["job"]["resultCode"].as_str(),
                Some("applied" | "applied_restart_recommended" | "applied_with_warning")
            ));
        }
    });
    state.proxy_service.set_managed_activation_test_hook(None);
    assert_eq!(std::fs::read(&claude_path).unwrap(), claude_before);
    assert!(state.db.get_all_providers("claude").unwrap().is_empty());
    assert!(
        !runtime
            .block_on(state.db.get_proxy_config_for_app("claude"))
            .unwrap()
            .enabled
    );
    assert!(
        runtime
            .block_on(state.db.get_proxy_config_for_app("codex"))
            .unwrap()
            .enabled
    );
    assert_eq!(
        state.db.get_current_provider("codex").unwrap().as_deref(),
        Some(codex.provider_id.as_str())
    );
    assert!(runtime.block_on(state.proxy_service.is_running()));
    assert!(std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_ok());
    assert!(std::fs::read_to_string(codex_path)
        .unwrap()
        .contains(&format!("127.0.0.1:{port}")));
    runtime
        .block_on(state.proxy_service.stop_with_restore())
        .unwrap();
}

#[test]
#[serial]
fn subscription_port_conflict_restores_current_files_flags_and_provider_rows() {
    let (_home, _guard, state, auth) = fixture();
    let credential = seed(&auth, "selected");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut global = runtime
        .block_on(state.db.get_global_proxy_config())
        .unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = occupied.local_addr().unwrap().port();
    global.proxy_enabled = false;
    runtime
        .block_on(state.db.update_global_proxy_config(global.clone()))
        .unwrap();
    let mut app = runtime
        .block_on(state.db.get_proxy_config_for_app("claude"))
        .unwrap();
    app.enabled = false;
    app.auto_failover_enabled = true;
    runtime
        .block_on(state.db.update_proxy_config_for_app(app.clone()))
        .unwrap();
    let original = crate::provider::Provider::with_id(
        "original-source".into(),
        "Original".into(),
        json!({"env":{"ANTHROPIC_BASE_URL":"https://original.example", "ANTHROPIC_API_KEY":"synthetic-api-key"}}),
        None,
    );
    state.db.save_provider("claude", &original).unwrap();
    state
        .db
        .set_current_provider("claude", &original.id)
        .unwrap();
    crate::settings::set_current_provider(&AppType::Claude, Some(&original.id)).unwrap();
    let settings = crate::config::get_claude_settings_path();
    crate::config::write_json_file(
        &settings,
        &json!({"permissions":{"deny":["Bash(rm:*)"]},"env":{"CUSTOM":"preserve"}}),
    )
    .unwrap();
    let before = std::fs::read(&settings).unwrap();
    assert!(matches!(
        ProviderService::bind_xai_managed(
            &state,
            &auth,
            request("claude", &credential.identity_id)
        ),
        Err(BindXaiManagedError::ApplyFailedRolledBack)
    ));
    assert_eq!(std::fs::read(&settings).unwrap(), before);
    assert_eq!(state.db.get_all_providers("claude").unwrap().len(), 1);
    assert_eq!(
        state.db.get_current_provider("claude").unwrap().as_deref(),
        Some("original-source")
    );
    assert_eq!(
        crate::settings::get_current_provider(&AppType::Claude).as_deref(),
        Some("original-source")
    );
    assert!(runtime
        .block_on(state.db.get_live_backup("claude"))
        .unwrap()
        .is_none());
    assert!(!runtime.block_on(state.proxy_service.is_running()));
    assert_eq!(
        serde_json::to_value(
            runtime
                .block_on(state.db.get_proxy_config_for_app("claude"))
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(app).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            runtime
                .block_on(state.db.get_global_proxy_config())
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(global).unwrap()
    );
}

#[test]
#[serial]
fn subscription_runtime_rollback_tampering_reports_unknown() {
    for trigger in [
        "CREATE TRIGGER tamper_runtime AFTER UPDATE ON proxy_config WHEN NEW.app_type = 'claude' BEGIN UPDATE proxy_config SET max_retries = NEW.max_retries + 1 WHERE app_type = 'claude'; END;",
        "CREATE TRIGGER tamper_runtime AFTER UPDATE OF proxy_enabled ON proxy_config WHEN NEW.app_type = 'claude' AND NEW.proxy_enabled = 0 BEGIN UPDATE proxy_config SET enable_logging = 0 WHERE app_type = 'claude'; END;",
    ] {
        let (_home, _guard, state, auth) = fixture();
        let credential = seed(&auth, "selected");
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut global = runtime.block_on(state.db.get_global_proxy_config()).unwrap();
        global.listen_address = "127.0.0.1".into();
        global.listen_port = occupied.local_addr().unwrap().port();
        global.proxy_enabled = false;
        global.enable_logging = true;
        runtime.block_on(state.db.update_global_proxy_config(global)).unwrap();
        state.db.conn.lock().unwrap().execute_batch(trigger).unwrap();
        assert!(matches!(ProviderService::bind_xai_managed(&state, &auth, request("claude", &credential.identity_id)), Err(BindXaiManagedError::RollbackPartialStateUnknown)));
        assert!(!runtime.block_on(state.proxy_service.is_running()));
        assert!(state.db.get_all_providers("claude").unwrap().is_empty());
    }
}

#[test]
#[serial]
fn subscription_vault_to_both_cli_protocols_and_restore() {
    let (home, _guard, state, auth) = fixture();
    let selected = seed(&auth, "selected");
    let _other_default = seed(&auth, "other-default");
    assert!(!home.path().join("vault-meta/xai_oauth_auth.json").exists());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut global = runtime
        .block_on(state.db.get_global_proxy_config())
        .unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = port;
    runtime
        .block_on(state.db.update_global_proxy_config(global))
        .unwrap();
    let claude_path = crate::config::get_claude_settings_path();
    crate::config::write_json_file(
        &claude_path,
        &json!({"permissions":{"deny":["Bash(rm:*)"]},"env":{"CUSTOM":"kept"}}),
    )
    .unwrap();
    let claude_before: Value = crate::config::read_json_file(&claude_path).unwrap();
    let config_path = crate::codex_config::get_codex_config_path();
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    let config_before = "model = \"original\"\n[features]\nweb_search_request = false\n[mcp_servers.fixture]\ncommand = \"fixture-command\"\n";
    std::fs::write(&config_path, config_before).unwrap();
    let auth_path = crate::codex_config::get_codex_auth_path();
    std::fs::write(
        &auth_path,
        b"{\"auth_mode\":\"chatgpt\",\"fixture\":\"preserved-native-login\"}",
    )
    .unwrap();
    let auth_before = std::fs::read(&auth_path).unwrap();
    let claude =
        ProviderService::bind_xai_managed(&state, &auth, request("claude", &selected.identity_id))
            .unwrap();
    assert!(claude.activated);
    let codex =
        ProviderService::bind_xai_managed(&state, &auth, request("codex", &selected.identity_id))
            .unwrap();
    assert!(!codex.activated);
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        config_before
    );
    let saved = state
        .db
        .get_provider_by_id(&codex.provider_id, "codex")
        .unwrap()
        .unwrap();
    assert!(ProviderService::xai_managed_account_is_ready(
        &state, &saved
    ));
    assert!(
        ProviderService::xai_managed_codex_shape_is_valid(&saved),
        "shape: {}",
        saved.settings_config
    );
    let environment = crate::services::provider::inspect_codex_switch_environment(&state).unwrap();
    crate::services::provider::build_codex_switch_target_live_projection(
        &state,
        &saved,
        &environment,
    )
    .unwrap();
    let plan = ChangePlanService::plan_codex_switch(&state, &codex.provider_id).unwrap();
    let outcome = ChangePlanService::apply_codex_switch_at_with_writer(
        &state,
        &plan.plan_id,
        &plan.plan_digest,
        chrono::Utc::now().timestamp(),
        |id| {
            ProviderService::with_live_config_result(AppType::Codex, || {
                ProviderService::switch_with_lock_held(&state, AppType::Codex, id)
            })
            .map(|result| WriterReceipt {
                live_config_changed: result.live_config_changed,
            })
        },
    )
    .unwrap();
    assert!(
        matches!(
            serde_json::to_value(&outcome).unwrap()["job"]["resultCode"].as_str(),
            Some("applied" | "applied_restart_recommended" | "applied_with_warning")
        ),
        "outcome: {:?}",
        serde_json::to_value(&outcome).unwrap()
    );
    assert_eq!(std::fs::read(&auth_path).unwrap(), auth_before);
    let codex_live = std::fs::read_to_string(&config_path).unwrap();
    assert!(codex_live.contains(&format!("127.0.0.1:{port}")));
    assert!(codex_live.contains("fixture-command"));
    let claude_live: Value = crate::config::read_json_file(&claude_path).unwrap();
    assert_eq!(claude_live["permissions"], claude_before["permissions"]);
    assert_eq!(claude_live["env"]["CUSTOM"], "kept");
    let captured = Arc::new(tokio::sync::Mutex::new(
        Vec::<(axum::http::HeaderMap, Value)>::new(),
    ));
    let capture = captured.clone();
    let (upstream, upstream_task) = runtime.block_on(async move {
        let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = server.local_addr().unwrap();
        let router = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| {
            let capture = capture.clone();
            async move {
                use axum::response::IntoResponse;
                let streaming = body["stream"] == true;
                capture.lock().await.push((headers, body));
                if streaming {
                    // Same split tool-call shape used by streaming_codex_chat's
                    // converts_tool_call_chat_sse_to_responses_sse regression.
                    let chunks = [
                        json!({"id":"chatcmpl_fixture","model":"grok-build","choices":[{"index":0,"delta":{"role":"assistant","content":"fixture-ok"}}]}),
                        json!({"id":"chatcmpl_fixture","model":"grok-build","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_fixture","type":"function","function":{"name":"get_weather","arguments":""}}]}}]}),
                        json!({"id":"chatcmpl_fixture","model":"grok-build","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":\"Tokyo\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}}),
                    ];
                    let events = chunks.into_iter().map(|chunk| format!("data: {chunk}\n\n")).collect::<String>() + "data: [DONE]\n\n";
                    ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], events).into_response()
                } else {
                    axum::Json(json!({"id":"chatcmpl_fixture","object":"chat.completion","created":123,"model":"grok-build","choices":[{"index":0,"message":{"role":"assistant","content":"fixture-ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":2,"total_tokens":4}})).into_response()
                }
            }
        }));
        let task = tokio::spawn(async move { axum::serve(server, router).await.unwrap(); });
        (format!("http://{address}"), task)
    });
    for id in [&claude.provider_id, &codex.provider_id] {
        crate::proxy::set_xai_integration_fixture(
            id,
            Some(crate::proxy::XaiIntegrationFixture {
                auth: auth.clone(),
                upstream: upstream.clone(),
            }),
        );
    }
    runtime.block_on(async {
        let client = reqwest::Client::new();
        let messages = client.post(format!("http://127.0.0.1:{port}/v1/messages"))
            .header("x-api-key", "PROXY_MANAGED").header("anthropic-version", "2023-06-01")
            .json(&json!({"model":"claude-sonnet-4-6","max_tokens":100,"messages":[{"role":"user","content":"hello"}],"stream":false}))
            .send().await.unwrap();
        let status = messages.status(); let body = messages.text().await.unwrap();
        assert!(status.is_success(), "Claude request: {status} {body}");
        assert!(body.contains("fixture-ok"));
        let responses = client.post(format!("http://127.0.0.1:{port}/v1/responses"))
            .bearer_auth("PROXY_MANAGED")
            .json(&json!({"model":"grok-build","input":[{"role":"user","content":"hello"}],"stream":false}))
            .send().await.unwrap();
        let status = responses.status(); let body = responses.text().await.unwrap();
        assert!(status.is_success(), "Codex request: {status} {body}");
        assert!(body.contains("fixture-ok"));
        let messages_stream = client.post(format!("http://127.0.0.1:{port}/v1/messages"))
            .header("x-api-key", "PROXY_MANAGED").header("anthropic-version", "2023-06-01")
            .header("x-xai-token-auth", "client-spoof").header("x-grok-model-override", "wrong-route")
            .json(&json!({"model":"claude-sonnet-4-6","max_tokens":100,"messages":[{"role":"user","content":"hello"}],"stream":true,"tools":[{"name":"get_weather","input_schema":{"type":"object","properties":{"city":{"type":"string"}}}}]}))
            .send().await.unwrap();
        assert!(messages_stream.status().is_success());
        let body = messages_stream.text().await.unwrap();
        for expected in ["event: message_start", "event: content_block_start", "get_weather", "input_json_delta", "Tokyo", "tool_use", "event: message_stop"] {
            assert!(body.contains(expected), "Missing Claude stream event {expected}: {body}");
        }
        let responses_stream = client.post(format!("http://127.0.0.1:{port}/v1/responses"))
            .bearer_auth("PROXY_MANAGED")
            .header("x-xai-token-auth", "client-spoof").header("x-grok-model-override", "wrong-route")
            .json(&json!({"model":"untrusted-client-model","input":[{"role":"user","content":"hello"}],"stream":true,"tools":[{"type":"function","name":"get_weather","parameters":{"type":"object","properties":{"city":{"type":"string"}}}}]}))
            .send().await.unwrap();
        assert!(responses_stream.status().is_success());
        let body = responses_stream.text().await.unwrap();
        for expected in ["event: response.created", "event: response.function_call_arguments.delta", "event: response.function_call_arguments.done", "get_weather", "call_fixture", "Tokyo", "event: response.completed"] {
            assert!(body.contains(expected), "Missing Codex stream event {expected}: {body}");
        }
        let requests = captured.lock().await;
        assert_eq!(requests.len(), 4);
        for (headers, body) in requests.iter() {
            assert_eq!(headers["authorization"], "Bearer synthetic-access-selected");
            assert_eq!(headers["x-xai-token-auth"], "xai-grok-cli");
            assert_eq!(headers["x-grok-model-override"], "grok-build");
            assert_eq!(body["model"], "grok-build");
            assert!(body.get("messages").is_some());
            assert!(body.get("input").is_none());
        }
    });
    let public = serde_json::to_string(&plan).unwrap()
        + &serde_json::to_string(&outcome).unwrap()
        + &serde_json::to_string(&claude).unwrap()
        + &serde_json::to_string(&codex).unwrap()
        + &codex_live
        + &serde_json::to_string(&claude_live).unwrap();
    assert!(!public.contains("synthetic-access"));
    assert!(!public.contains("synthetic-refresh"));
    let mut next_request = request("codex", &selected.identity_id);
    next_request.model_id = "grok-next".into();
    let next = ProviderService::bind_xai_managed(&state, &auth, next_request).unwrap();
    let next_plan = ChangePlanService::plan_codex_switch(&state, &next.provider_id).unwrap();
    auth.repository
        .set_status(
            &selected.credential_id,
            CredentialStatus::RequiresReauth,
            chrono::Utc::now().timestamp(),
        )
        .unwrap();
    let rejected = ChangePlanService::apply_codex_switch_at_with_writer(
        &state,
        &next_plan.plan_id,
        &next_plan.plan_digest,
        chrono::Utc::now().timestamp(),
        |_| -> Result<WriterReceipt, ()> { panic!("revoked account must not reach the writer") },
    )
    .unwrap();
    assert_eq!(
        rejected.error_code,
        Some(crate::services::change_plan::ChangePlanErrorCode::SecretDependencyUnavailable)
    );
    assert_eq!(
        state.db.get_current_provider("codex").unwrap().as_deref(),
        Some(codex.provider_id.as_str())
    );
    runtime.block_on(async {
        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .timeout(std::time::Duration::from_secs(5))
            .bearer_auth("PROXY_MANAGED")
            .json(&json!({"model":"grok-build","input":"revoked","stream":false}))
            .send()
            .await
            .unwrap();
        assert!(!response.status().is_success());
        assert_eq!(
            captured.lock().await.len(),
            4,
            "revocation must not fall back or call upstream"
        );
    });
    // Native Codex login can rotate independently during proxy use. Restore
    // must retain the new file instead of replaying the old login backup.
    let rotated_auth = b"{\"auth_mode\":\"chatgpt\",\"fixture\":\"new-native-login\"}";
    std::fs::write(&auth_path, rotated_auth).unwrap();
    runtime
        .block_on(state.proxy_service.stop_with_restore())
        .unwrap();
    assert_eq!(
        crate::config::read_json_file::<Value>(&claude_path).unwrap(),
        claude_before
    );
    assert_eq!(std::fs::read(&auth_path).unwrap(), rotated_auth);
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        config_before
    );
    for id in [&claude.provider_id, &codex.provider_id] {
        crate::proxy::set_xai_integration_fixture(id, None);
    }
    upstream_task.abort();
}
