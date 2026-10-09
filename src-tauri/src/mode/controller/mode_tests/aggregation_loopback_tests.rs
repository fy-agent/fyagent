//! Real HTTP through the production proxy, with isolated local upstream fixtures.
use super::*;
use axum::body::Bytes;
use axum::http::{StatusCode, Uri};
use std::sync::atomic::{AtomicU16, Ordering};
use tokio::sync::Mutex;

struct Upstream {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    status: Arc<AtomicU16>,
    server: tokio::task::JoinHandle<()>,
}

impl Upstream {
    async fn new() -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let status = Arc::new(AtomicU16::new(200));
        let app = {
            let requests = requests.clone();
            let status = status.clone();
            axum::Router::new().fallback(move |uri: Uri, bytes: Bytes| {
                let requests = requests.clone();
                let status = status.clone();
                async move {
                    let body: Value = serde_json::from_slice(&bytes).unwrap();
                    requests.lock().await.push(body.clone());
                    let status = StatusCode::from_u16(status.load(Ordering::SeqCst)).unwrap();
                    let response = if status.is_server_error() {
                        json!({"error": {"type": "overloaded_error", "message": "fixture failure"}})
                    } else if uri.path().ends_with("messages") {
                        json!({"id": "msg_fixture", "type": "message", "role": "assistant",
                            "model": body["model"], "content": [{"type": "text", "text": "ok"}],
                            "stop_reason": "end_turn", "usage": {"input_tokens": 1, "output_tokens": 1}})
                    } else {
                        json!({"id": "resp_fixture", "object": "response", "status": "completed",
                            "model": body["model"], "output": [], "usage": {"input_tokens": 1, "output_tokens": 1}})
                    };
                    (status, axum::Json(response))
                }
            })
        };
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            url,
            requests,
            status,
            server,
        }
    }
}

impl Drop for Upstream {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn aggregation_http_round_trip(app: AppType) {
    let _home = Home::new();
    let default = Upstream::new().await;
    let alternate = Upstream::new().await;
    let (rows, default_model, alternate_model, alias, endpoint, history_key) = match app {
        AppType::Claude => {
            seed_settings(USER_SETTINGS);
            (
                [
                    claude("a", &default.url, json!({})),
                    claude(
                        "alt",
                        &alternate.url,
                        json!({"ANTHROPIC_MODEL": "alt-model"}),
                    ),
                ],
                "claude-sonnet-4-6",
                "alt-model",
                "ccs-claude-alt--alt-model",
                "/v1/messages",
                "messages",
            )
        }
        AppType::Codex => {
            seed_codex("approval_policy = \"on-request\"\n", None);
            let mut rows = [
                codex_native("a", &format!("{}/v1", default.url), "", None),
                codex_native("alt", &format!("{}/v1", alternate.url), "", None),
            ];
            for row in &mut rows {
                row.settings_config["config"] = json!(format!(
                    "{}requires_openai_auth = true\n",
                    row.settings_config["config"].as_str().unwrap()
                ));
            }
            (
                rows,
                "gpt-a",
                "gpt-alt",
                "ccs-alt/gpt-alt",
                "/v1/responses",
                "input",
            )
        }
        _ => unreachable!(),
    };
    let state = state_with(app.clone(), &rows, "a").await;
    let mut config = state
        .db
        .get_proxy_config_for_app(app.as_str())
        .await
        .unwrap();
    config.auto_failover_enabled = true;
    config.max_retries = 3;
    state.db.update_proxy_config_for_app(config).await.unwrap();
    for id in ["a", "alt"] {
        state.db.add_to_failover_queue(app.as_str(), id).unwrap();
    }
    enter(&state, &app, true).await.unwrap();
    set_stack_member(&state, &app, "alt", true).await.unwrap();
    let client_path = match app {
        AppType::Claude => settings_path(),
        _ => crate::codex_config::get_codex_config_path(),
    };
    let client_before = fs::read(&client_path).unwrap();
    let proxy = state.proxy_service.get_status().await.unwrap();
    let url = format!("http://127.0.0.1:{}{endpoint}", proxy.port);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let history = json!([
        {"role": "user", "content": "Keep the same conversation: fixture-session"},
        {"role": "assistant", "content": "Previous answer"},
        {"role": "user", "content": "Continue with the selected model"}
    ]);
    let send = |model: &str| {
        let mut body = json!({"model": model, "stream": false, "max_tokens": 32});
        body[history_key] = history.clone();
        client
            .post(&url)
            .header("session_id", "fixture-session")
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
    };

    // Switch models in the same conversation without changing the client's config or route.
    for model in [default_model, alias, default_model] {
        let response = send(model).await.unwrap();
        assert!(
            response.status().is_success(),
            "{app:?} {model}: {}",
            response.text().await.unwrap()
        );
    }
    let default_requests = default.requests.lock().await.clone();
    let alternate_requests = alternate.requests.lock().await.clone();
    assert_eq!(default_requests.len(), 2);
    assert_eq!(alternate_requests.len(), 1);
    assert_eq!(alternate_requests[0]["model"], alternate_model);
    assert_eq!(alternate_requests[0][history_key], history);
    assert!(default_requests
        .iter()
        .all(|body| body["model"] == default_model && body[history_key] == history));
    assert_eq!(fs::read(&client_path).unwrap(), client_before);
    assert_eq!(in_use(&state, &app).as_deref(), Some("a"));

    // Both an explicit member and an unprefixed default fail at their own upstream.
    alternate.status.store(503, Ordering::SeqCst);
    assert!(send(alias).await.unwrap().status().is_server_error());
    assert_eq!(alternate.requests.lock().await.len(), 2);
    assert_eq!(default.requests.lock().await.len(), 2);
    default.status.store(503, Ordering::SeqCst);
    assert!(send(default_model)
        .await
        .unwrap()
        .status()
        .is_server_error());
    assert_eq!(default.requests.lock().await.len(), 3);
    assert_eq!(alternate.requests.lock().await.len(), 2);
    assert_eq!(in_use(&state, &app).as_deref(), Some("a"));

    set_stack_member(&state, &app, "alt", false).await.unwrap();
    assert_eq!(send(alias).await.unwrap().status(), StatusCode::BAD_REQUEST);
    assert_eq!(default.requests.lock().await.len(), 3);
    assert_eq!(alternate.requests.lock().await.len(), 2);
    assert!(!stack_views(&state, &app)
        .unwrap()
        .members
        .iter()
        .any(|member| member.provider_id == "alt"));

    // Exit restores the direct client; the same saved membership survives the next startup.
    detach_all(&state).await;
    assert!(!state.proxy_service.get_status().await.unwrap().running);
    assert!(!mode(&app).attached);
    assert!(mode(&app).is_proxy());
    assert!(!state.proxy_service.live_has_proxy_placeholder(&app));
    crate::restore_proxy_state_on_startup(&state).await;
    assert!(mode(&app).attached);
    assert!(state.proxy_service.get_status().await.unwrap().running);
    assert_eq!(stack_views(&state, &app).unwrap().members.len(), 1);
    exit(&state, &app).await.unwrap();
    assert!(!mode(&app).is_proxy());
    assert!(!state.proxy_service.get_status().await.unwrap().running);
    assert!(!state.proxy_service.live_has_proxy_placeholder(&app));
}

#[tokio::test]
#[serial]
async fn claude_stack_http_routes_one_conversation_and_never_fails_over() {
    aggregation_http_round_trip(AppType::Claude).await;
}

#[tokio::test]
#[serial]
async fn codex_stack_http_routes_one_conversation_and_never_fails_over() {
    aggregation_http_round_trip(AppType::Codex).await;
}
