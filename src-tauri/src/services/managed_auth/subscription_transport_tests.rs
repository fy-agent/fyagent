use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

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

#[test]
#[serial]
fn subscription_opencode_responses_stream_json_tool_roundtrip_and_no_api_key_fallback() {
    for source in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        let (_home, _guard, state, auth) = fixture();
        let selected = seed_provider(&auth, "selected", source);
        seed_provider(&auth, "default-other-account", source);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        super::opencode_tests::ephemeral(&runtime, &state);
        let bound = ProviderService::bind_opencode_managed_proxy(
            &state,
            &auth,
            super::opencode_tests::bind_request(&selected.identity_id, "selected-model"),
        )
        .unwrap();
        let port = runtime
            .block_on(state.proxy_service.get_status())
            .unwrap()
            .port;
        let captured = Arc::new(tokio::sync::Mutex::new(
            Vec::<(axum::http::HeaderMap, Value)>::new(),
        ));
        let capture = captured.clone();
        let (upstream, task) = runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let path = if source == ManagedAuthProvider::Openai {
                "/backend-api/codex/responses"
            } else {
                "/v1/responses"
            };
            let app = axum::Router::new().route(
                path,
                axum::routing::post(
                    move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| {
                        let capture = capture.clone();
                        async move {
                            use axum::response::IntoResponse;
                            capture.lock().await.push((headers, body.clone()));
                            // The fake vendor enforces ChatGPT wire requirements;
                            // permissive JSON stubs would hide this real regression.
                            if source == ManagedAuthProvider::Openai
                                && !(body["stream"] == true
                                    && body["store"] == false
                                    && body["instructions"].is_string()
                                    && body["tools"].is_array()
                                    && body["parallel_tool_calls"].is_boolean()
                                    && body.get("temperature").is_none()
                                    && body.get("top_p").is_none()
                                    && body.get("max_output_tokens").is_none())
                            {
                                return (
                                    axum::http::StatusCode::BAD_REQUEST,
                                    "invalid subscription wire request",
                                )
                                    .into_response();
                            }
                            let (response, events) = synthetic_responses("selected-model");
                            if body["stream"] == true {
                                (
                                    [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                                    events,
                                )
                                    .into_response()
                            } else {
                                axum::Json(response).into_response()
                            }
                        }
                    },
                ),
            );
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://{address}"), task)
        });
        crate::proxy::set_xai_integration_fixture(
            &bound.provider_id,
            Some(crate::proxy::XaiIntegrationFixture {
                auth: auth.clone(),
                upstream,
            }),
        );
        runtime.block_on(async {
            let client = reqwest::Client::new();
            for streaming in [false, true] {
                for tool_reply in [false, true] {
                    let input = if tool_reply { json!([
                        {"type":"function_call","call_id":"call_fixture","name":"get_weather","arguments":"{\"city\":\"Tokyo\"}"},
                        {"type":"function_call_output","call_id":"call_fixture","output":""}
                    ]) } else { json!([{"role":"user","content":[{"type":"input_text","text":"hello"}]}]) };
                    let response = client.post(format!("http://127.0.0.1:{port}/opencode/v1/responses"))
                        .timeout(std::time::Duration::from_secs(10)).bearer_auth("PROXY_MANAGED")
                        .header("chatgpt-account-id", "spoofed-other").header("x-xai-token-auth", "spoofed-other")
                        .json(&json!({"model":"untrusted-client-model", "input":input, "stream":streaming,
                            "temperature":0.7,"top_p":0.9,"max_output_tokens":1024,
                            "tools":[{"type":"function","name":"get_weather","parameters":{"type":"object","properties":{"city":{"type":"string"}}}}]}))
                        .send().await.unwrap();
                    let status = response.status();
                    let content_type = response.headers()["content-type"].to_str().unwrap().to_string();
                    let body = response.text().await.unwrap();
                    assert!(status.is_success(), "{source:?}: {status} {body}");
                    assert!(!body.contains("synthetic-access"));
                    assert!(!body.contains("synthetic-refresh"));
                    if streaming {
                        assert!(content_type.contains("text/event-stream"));
                        for event in ["response.created", "response.function_call_arguments.delta", "response.function_call_arguments.done", "response.completed", "call_fixture", "get_weather", "Tokyo"] {
                            assert!(body.contains(event), "missing {event}: {body}");
                        }
                    } else {
                        assert!(content_type.contains("application/json"));
                        let response: Value = serde_json::from_str(&body).unwrap();
                        assert_eq!(response["status"], "completed");
                        assert_eq!(response["model"], "selected-model");
                        assert_eq!(response["usage"]["output_tokens"], 3);
                        assert!(response["output"].as_array().unwrap().iter().any(|item| item["type"] == "function_call" && item["call_id"] == "call_fixture"));
                    }
                }
            }
            let requests = captured.lock().await;
            assert_eq!(requests.len(), 4);
            for (index, (headers, body)) in requests.iter().enumerate() {
                assert_eq!(headers["authorization"], "Bearer synthetic-access-selected");
                assert_eq!(body["model"], "selected-model");
                if index % 2 == 1 {
                    assert!(body["input"].as_array().unwrap().iter().any(|item| item["type"] == "function_call_output" && item["output"] == ""));
                }
                if source == ManagedAuthProvider::Openai {
                    assert_eq!(headers["chatgpt-account-id"], "selected");
                    assert_eq!(body["stream"], true);
                    assert!(body["include"].as_array().unwrap().iter().any(|value| value == "reasoning.encrypted_content"));
                } else {
                    assert_eq!(headers["x-xai-token-auth"], "xai-grok-cli");
                    assert_eq!(headers["x-grok-model-override"], "selected-model");
                }
            }
        });
        // A later queue/flag change cannot widen the subscription-only route.
        let mut paid = crate::provider::Provider::with_id(
            "paid-opencode".into(),
            "Paid".into(),
            json!({"auth":{"OPENAI_API_KEY":"paid-secret"}, "base_url":"https://unused.example/v1"}),
            None,
        );
        paid.in_failover_queue = true;
        state.db.save_provider("opencode", &paid).unwrap();
        let mut config = runtime
            .block_on(state.db.get_proxy_config_for_app("opencode"))
            .unwrap();
        config.auto_failover_enabled = true;
        runtime
            .block_on(state.db.update_proxy_config_for_app(config))
            .unwrap();
        auth.repository
            .set_status(
                &selected.credential_id,
                CredentialStatus::RequiresReauth,
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        runtime.block_on(async {
            let response = reqwest::Client::new()
                .post(format!("http://127.0.0.1:{port}/opencode/v1/responses"))
                .timeout(std::time::Duration::from_secs(5))
                .json(&json!({"model":"selected-model","input":"revoked","stream":false}))
                .send()
                .await
                .unwrap();
            assert!(!response.status().is_success());
            assert_eq!(
                captured.lock().await.len(),
                4,
                "revoked account must not fall back or reach the vendor"
            );
            assert_eq!(
                state
                    .proxy_service
                    .get_status()
                    .await
                    .unwrap()
                    .failover_count,
                0
            );
            state.proxy_service.stop_with_restore().await.unwrap();
        });
        crate::proxy::set_xai_integration_fixture(&bound.provider_id, None);
        task.abort();
    }
}
