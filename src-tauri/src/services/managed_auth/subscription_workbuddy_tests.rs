//! WorkBuddy subscription routing. Knife 1 only: an explicit account binding,
//! a loopback proxy, and a fake upstream. No models.json writer and no UI.

use super::migration::LegacyCredentialInput;
use super::*;
use crate::app_config::AppType;
use crate::database::Database;
use crate::services::provider::{BindXaiManagedError, BindXaiManagedRequest, ProviderService};
use crate::services::secret::{MemorySecretBackend, SecretService};
use crate::services::workbuddy::error::WorkBuddySubscriptionErrorCode;
use crate::services::workbuddy::subscription;
use crate::services::workbuddy::types::WorkBuddySubscriptionSnapshot;
use crate::store::AppState;
use serde_json::{json, Value};
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct TestHome(Option<std::ffi::OsString>, Option<std::ffi::OsString>);

impl TestHome {
    fn set(path: &std::path::Path) -> Self {
        #[cfg(target_os = "windows")]
        crate::initialize_windows_user_context().expect("Windows test user context");
        let previous = std::env::var_os("FYAGENT_TEST_HOME");
        let data_home = std::env::var_os("XDG_DATA_HOME");
        let guard = Self(previous, data_home);
        std::env::set_var("FYAGENT_TEST_HOME", path);
        assert_eq!(crate::config::get_home_dir(), path);
        std::env::set_var("XDG_DATA_HOME", path.join(".local/share"));
        crate::settings::reload_settings().unwrap();
        guard
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match self.1.take() {
            Some(value) => std::env::set_var("XDG_DATA_HOME", value),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match self.0.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
        crate::settings::reload_settings().unwrap();
    }
}

struct RuntimeGuard(Option<tokio::runtime::Runtime>);

impl RuntimeGuard {
    fn new() -> Self {
        Self(Some(tokio::runtime::Runtime::new().unwrap()))
    }

    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.0.as_ref().unwrap().block_on(future)
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_timeout(Duration::from_secs(2));
        }
    }
}

struct TaskGuard(Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>);

impl TaskGuard {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn track(&self, task: tokio::task::JoinHandle<()>) {
        self.0.lock().expect("task list").push(task);
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Ok(mut tasks) = self.0.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}

struct AuthGuard;

impl AuthGuard {
    fn arm(auth: Arc<ManagedAuthService<MemorySecretBackend>>) -> Self {
        subscription::set_workbuddy_subscription_auth_for_test(Some(auth));
        Self
    }
}

impl Drop for AuthGuard {
    fn drop(&mut self) {
        subscription::set_workbuddy_subscription_auth_for_test(None);
        crate::proxy::workbuddy::set_workbuddy_upstream_timeout_for_test(0);
    }
}

struct FixtureGuard(Mutex<Vec<String>>);

impl FixtureGuard {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    fn arm(
        &self,
        provider_id: &str,
        auth: Arc<ManagedAuthService<MemorySecretBackend>>,
        upstream: String,
    ) {
        crate::proxy::set_xai_integration_fixture(
            provider_id,
            Some(crate::proxy::XaiIntegrationFixture { auth, upstream }),
        );
        self.0
            .lock()
            .expect("fixtures")
            .push(provider_id.to_string());
    }
}

impl Drop for FixtureGuard {
    fn drop(&mut self) {
        if let Ok(ids) = self.0.lock() {
            for id in ids.iter() {
                crate::proxy::set_xai_integration_fixture(id, None);
            }
        }
    }
}

fn seed_provider(
    auth: &ManagedAuthService<MemorySecretBackend>,
    account: &str,
    provider: ManagedAuthProvider,
) -> CredentialRecord {
    let credential = auth
        .provision_legacy_credential(LegacyCredentialInput {
            migration_id: None,
            provider,
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
        provider,
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

/// `set_status(Ready)` is rejected on purpose. A fresh grant is the generation-aware way back.
fn admit_ready_again(
    auth: &ManagedAuthService<MemorySecretBackend>,
    credential_id: &str,
    account: &str,
) {
    let current = auth
        .repository
        .get_credential(credential_id)
        .unwrap()
        .expect("credential remains after fail-closed restore");
    let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
        credential_id: current.credential_id.clone(),
        provider: current.provider,
        generation: current.generation + 1,
        access_token: Some(format!("synthetic-access-{account}")),
        refresh_token: Some(format!("synthetic-refresh-{account}")),
        id_token: None,
        token_type: Some("Bearer".into()),
        granted_scopes: vec![],
        issued_at: Some(chrono::Utc::now().timestamp()),
        expires_at: Some(chrono::Utc::now().timestamp() + 3600),
    })
    .unwrap();
    assert!(
        auth.replace_bundle_cas(
            &current.credential_id,
            current.generation,
            RefreshOwner::Fyagent,
            bundle,
        )
        .unwrap(),
        "generation-aware ready transition must apply"
    );
}

fn open_fixture() -> (
    tempfile::TempDir,
    TestHome,
    Arc<AppState>,
    Arc<ManagedAuthService<MemorySecretBackend>>,
) {
    let home = tempfile::tempdir().unwrap();
    let guard = TestHome::set(home.path());
    let db = Arc::new(Database::memory().unwrap());
    let auth = Arc::new(ManagedAuthService::new(
        db.clone(),
        SecretService::new(MemorySecretBackend::new()),
        home.path().join("vault-meta"),
    ));
    let state = Arc::new(AppState::new(db));
    (home, guard, state, auth)
}

fn clear_agent_selection() {
    for app in [
        AppType::Claude,
        AppType::Codex,
        AppType::GrokBuild,
        AppType::OpenCode,
    ] {
        crate::settings::set_current_provider(&app, None).unwrap();
    }
}

async fn listen_loopback(state: &AppState, port: u16) {
    let mut global = state.db.get_global_proxy_config().await.unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = port;
    global.proxy_enabled = false;
    state.db.update_global_proxy_config(global).await.unwrap();
}

fn chat_message(content: &str) -> Value {
    json!({
        "model": "probe-model",
        "stream": true,
        "messages": [{ "role": "user", "content": content }]
    })
}

fn error_code(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["error"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

fn assert_closed(status: reqwest::StatusCode, body: &str, expected: u16, code: &str) {
    assert_eq!(status.as_u16(), expected, "{body}");
    assert_eq!(error_code(body), code);
    assert!(!body.contains("synthetic-access"));
    assert!(!body.contains("synthetic-refresh"));
    assert!(!body.contains("synthetic rejection"));
    assert!(!body.contains("SENTINEL_UPSTREAM_SECRET"));
}

fn binding_json(state: &AppState) -> Value {
    let raw = state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .expect("binding");
    serde_json::from_str(&raw).unwrap()
}

fn assert_no_secret_material(raw: &str, home: &Path) {
    assert!(raw.contains("~/.workbuddy/models.json"));
    assert!(!raw.contains("synthetic-access"));
    assert!(!raw.contains("synthetic-refresh"));
    assert!(!raw.contains("access_token"));
    assert!(!raw.contains("refresh_token"));
    assert!(!raw.contains("accessToken"));
    assert!(!raw.contains("refreshToken"));
    assert!(!raw.contains(&home.display().to_string()));
}

#[derive(Clone)]
struct Hit {
    authorization: String,
    api_key: String,
    account: String,
    model_override: String,
    token_auth: String,
    body: String,
}

fn header_text(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn text_sse(model: &str) -> String {
    format!(
        "{}{}",
        text_delta_sse("fixture-ok"),
        text_completed_sse(model, "fixture-ok")
    )
}

fn text_delta_sse(text: &str) -> String {
    use crate::proxy::providers::codex_responses_sse as sse;
    std::str::from_utf8(&sse::output_text_delta(0, "msg_fixture", text))
        .unwrap()
        .to_string()
}

fn text_completed_sse(model: &str, text: &str) -> String {
    use crate::proxy::providers::codex_responses_sse as sse;
    let message = sse::message_item("msg_fixture", text);
    let response = json!({
        "id": "resp_fixture",
        "object": "response",
        "status": "completed",
        "model": model,
        "output": [message]
    });
    std::str::from_utf8(&sse::response_completed(&response))
        .unwrap()
        .to_string()
}

fn failed_after_delta_sse() -> String {
    use crate::proxy::providers::codex_responses_sse as sse;
    let response = json!({
        "id": "resp_fixture",
        "object": "response",
        "status": "failed",
        "error": {
            "type": "server_error",
            "message": "SENTINEL_UPSTREAM_SECRET"
        }
    });
    format!(
        "{}{}",
        text_delta_sse("partial-visible"),
        std::str::from_utf8(&sse::response_failed(&response)).unwrap()
    )
}

fn tool_sse(model: &str) -> String {
    use crate::proxy::providers::codex_responses_sse as sse;
    let tool = json!({
        "id": "fc_fixture",
        "type": "function_call",
        "status": "completed",
        "call_id": "call_fixture",
        "name": "get_weather",
        "arguments": "{\"city\":\"Tokyo\"}"
    });
    let response = json!({
        "id": "resp_fixture",
        "object": "response",
        "status": "completed",
        "model": model,
        "output": [tool]
    });
    let pending = json!({
        "id": "fc_fixture",
        "type": "function_call",
        "call_id": "call_fixture",
        "name": "get_weather",
        "arguments": ""
    });
    format!(
        "{}{}{}{}",
        std::str::from_utf8(&sse::output_item_added(0, &pending)).unwrap(),
        std::str::from_utf8(&sse::function_call_arguments_delta(
            0,
            "fc_fixture",
            "{\"city\":"
        ))
        .unwrap(),
        std::str::from_utf8(&sse::function_call_arguments_done(
            0,
            "fc_fixture",
            "{\"city\":\"Tokyo\"}"
        ))
        .unwrap(),
        std::str::from_utf8(&sse::response_completed(&response)).unwrap()
    )
}

#[derive(Clone, Copy)]
enum UpstreamMode {
    Text,
    ToolAware,
    Fail500,
    Hang,
    MidStreamError,
    EarlyClose,
    StallAfterChunk,
}

async fn spawn_upstream(
    provider: ManagedAuthProvider,
    mode: UpstreamMode,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    Arc<AtomicUsize>,
    Arc<Mutex<Vec<Hit>>>,
) {
    let calls = Arc::new(AtomicUsize::new(0));
    let hits = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let path = if provider == ManagedAuthProvider::Openai {
        "/backend-api/codex/responses"
    } else {
        "/v1/responses"
    };
    let capture_calls = calls.clone();
    let capture_hits = hits.clone();
    let router = axum::Router::new().route(
        path,
        axum::routing::post(
            move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                let calls = capture_calls.clone();
                let hits = capture_hits.clone();
                async move {
                    use axum::response::IntoResponse;
                    calls.fetch_add(1, Ordering::SeqCst);
                    let text = String::from_utf8_lossy(&body).to_string();
                    hits.lock().expect("hits").push(Hit {
                        authorization: header_text(&headers, "authorization"),
                        api_key: header_text(&headers, "x-api-key"),
                        account: header_text(&headers, "chatgpt-account-id"),
                        model_override: header_text(&headers, "x-grok-model-override"),
                        token_auth: header_text(&headers, "x-xai-token-auth"),
                        body: text.clone(),
                    });
                    match mode {
                        UpstreamMode::Hang => std::future::pending().await,
                        UpstreamMode::Fail500 => (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            "SENTINEL_UPSTREAM_SECRET",
                        )
                            .into_response(),
                        UpstreamMode::MidStreamError => (
                            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            failed_after_delta_sse(),
                        )
                            .into_response(),
                        UpstreamMode::EarlyClose => (
                            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            text_delta_sse("partial-visible"),
                        )
                            .into_response(),
                        UpstreamMode::StallAfterChunk => {
                            let body = async_stream::stream! {
                                yield Ok::<bytes::Bytes, std::io::Error>(bytes::Bytes::from(
                                    text_delta_sse("partial-visible"),
                                ));
                                std::future::pending::<()>().await;
                            };
                            (
                                [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                                axum::body::Body::from_stream(body),
                            )
                                .into_response()
                        }
                        UpstreamMode::ToolAware if text.contains("function_call_output") => (
                            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            text_sse("probe-model"),
                        )
                            .into_response(),
                        UpstreamMode::ToolAware if text.contains("get_weather") => (
                            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            tool_sse("probe-model"),
                        )
                            .into_response(),
                        _ => (
                            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                            text_sse("probe-model"),
                        )
                            .into_response(),
                    }
                }
            },
        ),
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}"), task, calls, hits)
}

fn assert_bound_hit(hit: &Hit, provider: ManagedAuthProvider, proxy_key: &str) {
    assert_eq!(hit.authorization, "Bearer synthetic-access-selected");
    assert_ne!(hit.api_key, proxy_key);
    assert!(!hit.body.contains(proxy_key));
    assert!(!hit.body.contains("synthetic-access-other-default"));
    assert!(!hit.authorization.contains("other-default"));
    let parsed: Value = serde_json::from_str(&hit.body).unwrap();
    assert_eq!(parsed["model"], "probe-model");
    assert_eq!(parsed["stream"], true);
    if provider == ManagedAuthProvider::Openai {
        assert_eq!(hit.account, "selected");
        assert_eq!(parsed["store"], false);
    } else {
        assert_eq!(hit.model_override, "probe-model");
        assert_eq!(hit.token_auth, "xai-grok-cli");
    }
}

async fn post_chat(
    client: &reqwest::Client,
    port: u16,
    path: &str,
    key: &str,
    style: &str,
    body: Value,
    purpose: Option<&str>,
) -> (reqwest::StatusCode, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let mut request = client.post(url).timeout(Duration::from_secs(8)).json(&body);
    request = match style {
        "bearer" => request.header("authorization", format!("Bearer {key}")),
        "api-key" => request.header("x-api-key", key),
        "both" => request
            .header("authorization", format!("Bearer {key}"))
            .header("x-api-key", key),
        "bearer-ok-api-bad" => request
            .header("authorization", format!("Bearer {key}"))
            .header("x-api-key", "wrong-key"),
        "api-ok-bearer-bad" => request
            .header("authorization", "Bearer wrong-key")
            .header("x-api-key", key),
        "wrong" => request
            .header("authorization", "Bearer wrong-key")
            .header("x-api-key", "wrong-key"),
        "none" => request,
        _ => panic!("unknown auth style"),
    };
    if let Some(purpose) = purpose {
        request = request
            .header("x-agent-purpose", purpose)
            .header("x-conversation-id", "conv-fixture")
            .header("x-user-id", "user-fixture")
            .header("x-domain", "example.test")
            .header("x-ide-version", "5.8.2");
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    (status, text)
}

async fn publish_probe(
    state: &AppState,
    auth: &ManagedAuthService<MemorySecretBackend>,
    account_id: &str,
) -> WorkBuddySubscriptionSnapshot {
    subscription::publish_workbuddy_subscription(state, auth, account_id, "probe-model")
        .await
        .unwrap()
}

fn agent_files() -> Vec<PathBuf> {
    let mut paths = vec![
        crate::config::get_claude_settings_path(),
        crate::codex_config::get_codex_auth_path(),
        crate::codex_config::get_codex_config_path(),
        crate::grok_config::get_grok_config_path(),
    ];
    if let Ok(path) = crate::opencode_config::try_get_opencode_config_path() {
        paths.push(path);
    }
    paths
}

fn write_sentinels() -> Vec<(PathBuf, Vec<u8>)> {
    agent_files()
        .into_iter()
        .map(|path| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            let bytes = format!("sentinel:{}\n", path.display()).into_bytes();
            std::fs::write(&path, &bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

fn assert_sentinels(sentinels: &[(PathBuf, Vec<u8>)]) {
    for (path, bytes) in sentinels {
        assert_eq!(std::fs::read(path).unwrap(), *bytes, "{}", path.display());
    }
}

#[test]
#[serial]
fn subscription_workbuddy_openai_and_xai_stream_two_requests_and_ignore_other_agents() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    for provider in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        let (home, _guard, state, auth) = open_fixture();
        let _auth = AuthGuard::arm(auth.clone());
        clear_agent_selection();
        let selected = seed_provider(&auth, "selected", provider);
        seed_provider(&auth, "other-default", provider);
        crate::settings::set_current_provider(&AppType::Codex, Some("codex-sentinel")).unwrap();
        crate::settings::set_current_provider(&AppType::Claude, Some("claude-sentinel")).unwrap();
        let admitted = auth
            .admit_explicit_workbuddy_account(&selected.identity_id)
            .unwrap();
        assert!(!format!("{admitted:?}").contains("synthetic-"));
        runtime.block_on(async {
            listen_loopback(&state, 0).await;
            let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
            assert_eq!(snapshot.route_id, "workbuddy");
            assert_eq!(snapshot.provider, provider.as_str());
            assert_eq!(snapshot.model_id, "probe-model");
            assert_eq!(snapshot.local_base_path, "/workbuddy/v1");
            assert_eq!(snapshot.config_locator, "~/.workbuddy/models.json");
            assert!(snapshot.connected);
            assert!(!format!("{snapshot:?}").contains("synthetic-"));
            let raw = state
                .db
                .get_workbuddy_subscription_binding()
                .unwrap()
                .unwrap();
            assert_no_secret_material(&raw, home.path());
            for app in ["claude", "codex", "grokbuild", "opencode", "gemini"] {
                assert!(!state
                    .db
                    .get_provider_ids(app)
                    .unwrap()
                    .contains(&snapshot.provider_id));
            }
            let (upstream, task, calls, hits) = spawn_upstream(provider, UpstreamMode::Text).await;
            tasks.track(task);
            fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
            let key = state.db.ensure_proxy_access_key().unwrap();
            let port = state.proxy_service.get_status().await.unwrap().port;
            let client = reqwest::Client::new();
            for purpose in ["conversation_topic", "conversation"] {
                let (status, body) = post_chat(
                    &client,
                    port,
                    "/workbuddy/v1/chat/completions",
                    &key,
                    "both",
                    chat_message("hello"),
                    Some(purpose),
                )
                .await;
                assert!(status.is_success(), "{purpose} {status} {body}");
                assert!(body.contains("fixture-ok"), "{body}");
                assert!(body.contains("data: [DONE]"), "{body}");
                assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
                assert!(!body.contains("synthetic-access"), "{body}");
            }
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            let recorded = hits.lock().unwrap().clone();
            assert_eq!(recorded.len(), 2);
            for hit in &recorded {
                assert_bound_hit(hit, provider, &key);
            }
            let auth_kind = if provider == ManagedAuthProvider::Openai {
                "codex_oauth"
            } else {
                "xai_oauth"
            };
            // A missing Codex selection is unreadable, so a negative answer
            // stays unknown. The positive WorkBuddy route is still proven.
            assert_eq!(
                state
                    .proxy_service
                    .observe_managed_account_route(auth_kind, "selected", false),
                Some(true)
            );
            clear_agent_selection();
            assert_eq!(
                state
                    .proxy_service
                    .observe_managed_account_route(auth_kind, "other-default", true),
                Some(false)
            );
            assert_eq!(
                state
                    .proxy_service
                    .observe_managed_account_route(auth_kind, "selected", false),
                Some(true)
            );
            crate::settings::set_current_provider(
                &AppType::Codex,
                Some("missing-fixture-provider"),
            )
            .unwrap();
            assert_eq!(
                state
                    .proxy_service
                    .observe_managed_account_route(auth_kind, "selected", false),
                Some(true)
            );
            assert_eq!(
                crate::settings::get_current_provider(&AppType::Codex).as_deref(),
                Some("missing-fixture-provider")
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
            state.proxy_service.stop().await.unwrap();
        });
    }
}

#[test]
#[serial]
fn subscription_workbuddy_accepts_bearer_or_api_key_and_rejects_bad_credentials() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::Text).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let client = reqwest::Client::new();
        for style in [
            "bearer",
            "api-key",
            "both",
            "bearer-ok-api-bad",
            "api-ok-bearer-bad",
        ] {
            let (status, body) = post_chat(
                &client,
                port,
                "/workbuddy/v1/chat/completions",
                &key,
                style,
                chat_message("hello"),
                Some("conversation"),
            )
            .await;
            assert!(status.is_success(), "{style} {status} {body}");
            assert!(body.contains("fixture-ok"), "{style} {body}");
        }
        let accepted = calls.load(Ordering::SeqCst);
        assert_eq!(accepted, 5);
        for style in ["wrong", "none"] {
            let (status, body) = post_chat(
                &client,
                port,
                "/workbuddy/v1/chat/completions",
                &key,
                style,
                chat_message("hello"),
                Some("conversation"),
            )
            .await;
            assert_closed(status, &body, 401, "proxy_credential_rejected");
        }
        assert_eq!(calls.load(Ordering::SeqCst), accepted);
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_stream_end_and_tool_call_roundtrip() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Xai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, hits) =
            spawn_upstream(ManagedAuthProvider::Xai, UpstreamMode::ToolAware).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let client = reqwest::Client::new();
        let tools = json!([{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "weather",
                "parameters": { "type": "object" }
            }
        }]);
        let first = json!({
            "model": "probe-model",
            "stream": true,
            "messages": [{ "role": "user", "content": "weather in Tokyo" }],
            "tools": tools.clone()
        });
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            first,
            Some("conversation"),
        )
        .await;
        assert!(status.is_success(), "{body}");
        assert!(body.contains("get_weather"), "{body}");
        assert!(body.contains("call_fixture"), "{body}");
        assert!(body.contains("Tokyo"), "{body}");
        assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
        assert!(body.contains("data: [DONE]"), "{body}");
        let follow = json!({
            "model": "probe-model",
            "stream": true,
            "messages": [
                { "role": "user", "content": "weather in Tokyo" },
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_fixture",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Tokyo\"}"
                        }
                    }]
                },
                { "role": "tool", "tool_call_id": "call_fixture", "content": "sunny" }
            ],
            "tools": tools
        });
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "api-key",
            follow,
            Some("conversation"),
        )
        .await;
        assert!(status.is_success(), "{body}");
        assert!(body.contains("fixture-ok"), "{body}");
        assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
        assert!(body.contains("data: [DONE]"), "{body}");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let recorded = hits.lock().unwrap().clone();
        assert!(recorded[0].body.contains("get_weather"));
        assert!(!recorded[0].body.contains("function_call_output"));
        assert!(recorded[1].body.contains("function_call_output"));
        assert!(recorded[1].body.contains("sunny"));
        assert!(recorded[1].body.contains("get_weather"));
        assert_eq!(
            state
                .proxy_service
                .get_status()
                .await
                .unwrap()
                .failover_count,
            0
        );
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_401_refreshes_same_account_only() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    for provider in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        for reject_retry in [false, true] {
            let (_home, _guard, state, auth) = open_fixture();
            let _auth = AuthGuard::arm(auth.clone());
            let selected = seed_provider(&auth, "selected", provider);
            seed_provider(&auth, "other-default", provider);
            runtime.block_on(async {
                listen_loopback(&state, 0).await;
                let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
                let calls = Arc::new(AtomicUsize::new(0));
                let capture_calls = calls.clone();
                let rotate_auth = auth.clone();
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
                              axum::Json(_body): axum::Json<Value>| {
                            let calls = capture_calls.clone();
                            let auth = rotate_auth.clone();
                            let credential = selected.clone();
                            async move {
                                use axum::response::IntoResponse;
                                let attempt = calls.fetch_add(1, Ordering::SeqCst);
                                assert!(attempt < 2, "refresh must not loop");
                                let expected = if attempt == 0 {
                                    "Bearer synthetic-access-selected"
                                } else {
                                    "Bearer synthetic-retry-access"
                                };
                                assert_eq!(header_text(&headers, "authorization"), expected);
                                assert_ne!(
                                    header_text(&headers, "authorization"),
                                    "Bearer synthetic-access-other-default"
                                );
                                if provider == ManagedAuthProvider::Openai {
                                    assert_eq!(
                                        header_text(&headers, "chatgpt-account-id"),
                                        "selected"
                                    );
                                }
                                if attempt == 0 {
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
                                    (
                                        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                                        text_sse("probe-model"),
                                    )
                                        .into_response()
                                }
                            }
                        },
                    ),
                );
                let task = tokio::spawn(async move {
                    axum::serve(listener, router).await.unwrap();
                });
                tasks.track(task);
                fixtures.arm(
                    &snapshot.provider_id,
                    auth.clone(),
                    format!("http://{address}"),
                );
                let key = state.db.ensure_proxy_access_key().unwrap();
                let port = state.proxy_service.get_status().await.unwrap().port;
                let (status, body) = post_chat(
                    &reqwest::Client::new(),
                    port,
                    "/workbuddy/v1/chat/completions",
                    &key,
                    "both",
                    chat_message("hello"),
                    Some("conversation"),
                )
                .await;
                assert_eq!(calls.load(Ordering::SeqCst), 2, "{provider:?} {body}");
                assert_eq!(
                    state
                        .proxy_service
                        .get_status()
                        .await
                        .unwrap()
                        .failover_count,
                    0
                );
                if reject_retry {
                    assert_closed(status, &body, 401, "account_unavailable");
                } else {
                    assert!(status.is_success(), "{body}");
                    assert!(body.contains("fixture-ok"), "{body}");
                    assert!(!body.contains("synthetic-retry-access"), "{body}");
                    assert!(!body.contains("synthetic-access-selected"), "{body}");
                }
                let stored = state
                    .db
                    .get_workbuddy_subscription_binding()
                    .unwrap()
                    .unwrap();
                assert!(!stored.contains("synthetic-retry"));
                assert!(!stored.contains("synthetic-access"));
                state.proxy_service.stop().await.unwrap();
            });
        }
    }
}

#[test]
#[serial]
fn subscription_workbuddy_timeout_and_upstream_error_do_not_fall_back() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    seed_provider(&auth, "other-default", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        crate::proxy::workbuddy::set_workbuddy_upstream_timeout_for_test(1);
        let (upstream, task, calls, hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::Hang).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let client = reqwest::Client::new();
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert_closed(status, &body, 504, "timeout");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(hits.lock().unwrap().len(), 1);
        assert_eq!(
            hits.lock().unwrap()[0].authorization,
            "Bearer synthetic-access-selected"
        );
        state.proxy_service.stop().await.unwrap();

        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::Fail500).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        crate::proxy::workbuddy::set_workbuddy_upstream_timeout_for_test(0);
        let port = state.proxy_service.get_status().await.unwrap().port;
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "api-key",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert_closed(status, &body, 502, "upstream_error");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            state
                .proxy_service
                .get_status()
                .await
                .unwrap()
                .failover_count,
            0
        );
        let stored = binding_json(&state);
        assert_eq!(stored["desired"], true);
        assert_eq!(stored["status"], "active");
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_invalid_account_fails_closed() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Xai);
    seed_provider(&auth, "other-default", ManagedAuthProvider::Xai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        publish_probe(&state, &auth, &selected.identity_id).await;
        state.proxy_service.stop().await.unwrap();
        auth.repository
            .set_status(
                &selected.credential_id,
                CredentialStatus::RequiresReauth,
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let restored = AppState::new(state.db.clone());
        let error = subscription::restore_workbuddy_subscription(&restored, auth.as_ref())
            .await
            .unwrap_err();
        assert_eq!(
            error.code(),
            WorkBuddySubscriptionErrorCode::AccountUnavailable
        );
        let stored = binding_json(&restored);
        assert_eq!(stored["status"], "account_unavailable");
        assert_eq!(stored["connected"], false);
        assert_eq!(stored["desired"], true);
        assert!(!restored.proxy_service.is_running().await);

        let ready_auth = auth.clone();
        let ready_credential_id = selected.credential_id.clone();
        tokio::task::spawn_blocking(move || {
            admit_ready_again(&ready_auth, &ready_credential_id, "selected");
        })
        .await
        .expect("generation-aware ready transition task must complete");
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Xai, UpstreamMode::Text).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        auth.repository
            .set_status(
                &selected.credential_id,
                CredentialStatus::RequiresReauth,
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let (status, body) = post_chat(
            &reqwest::Client::new(),
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "both",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert_closed(status, &body, 401, "account_unavailable");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let stored = binding_json(&state);
        assert_eq!(stored["status"], "account_unavailable");
        assert_eq!(stored["connected"], false);
        assert_eq!(stored["desired"], true);
        assert_eq!(stored["legacyAccountId"], "selected");
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_binding_restores_after_restart_without_oauth_plaintext() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    seed_provider(&auth, "other-default", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::Text).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let client = reqwest::Client::new();
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert!(status.is_success(), "{body}");
        let raw = state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap()
            .unwrap();
        assert_no_secret_material(&raw, home.path());
        let provider_id = snapshot.provider_id.clone();
        state.proxy_service.stop().await.unwrap();
        let stopped = binding_json(&state);
        assert_eq!(stopped["connected"], false);
        assert_eq!(stopped["desired"], true);
        assert_eq!(stopped["status"], "active");

        let restarted = AppState::new(state.db.clone());
        let restored = subscription::restore_workbuddy_subscription(&restarted, auth.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert!(restored.connected);
        assert_eq!(restored.provider_id, provider_id);
        assert_eq!(restored.account_id, selected.identity_id);
        let port = restarted.proxy_service.get_status().await.unwrap().port;
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "api-key",
            chat_message("again"),
            Some("conversation_topic"),
        )
        .await;
        assert!(status.is_success(), "{body}");
        assert!(body.contains("fixture-ok"), "{body}");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let recorded = hits.lock().unwrap().clone();
        assert_eq!(
            recorded[1].authorization,
            "Bearer synthetic-access-selected"
        );
        assert_eq!(recorded[1].account, "selected");
        let raw = restarted
            .db
            .get_workbuddy_subscription_binding()
            .unwrap()
            .unwrap();
        assert_no_secret_material(&raw, home.path());
        restarted.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_rejects_missing_binding_model_mismatch_and_unknown_route() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        let mut global = state.db.get_global_proxy_config().await.unwrap();
        global.listen_address = "0.0.0.0".into();
        global.listen_port = 9;
        global.proxy_enabled = false;
        state
            .db
            .update_global_proxy_config(global)
            .await
            .unwrap();
        let error = subscription::publish_workbuddy_subscription(
            &state,
            &auth,
            &selected.identity_id,
            "probe-model",
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), WorkBuddySubscriptionErrorCode::InvalidRequest);
        assert!(state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap()
            .is_none());
        assert!(!state.proxy_service.is_running().await);

        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::Text).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let client = reqwest::Client::new();
        let mut wrong_model = chat_message("hello");
        wrong_model["model"] = json!("other-model");
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "both",
            wrong_model,
            None,
        )
        .await;
        assert_closed(status, &body, 400, "model_mismatch");
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            json!({"stream": true, "messages": [{"role": "user", "content": "hello"}]}),
            None,
        )
        .await;
        assert_closed(status, &body, 400, "model_mismatch");
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            json!("not-an-object"),
            None,
        )
        .await;
        assert_closed(status, &body, 400, "invalid_request");
        for path in [
            "/workbuddy/v1/models",
            "/workbuddy/v1/responses",
            "/workbuddy/v1/other",
        ] {
            let (status, body) = post_chat(
                &client,
                port,
                path,
                &key,
                "api-key",
                chat_message("hello"),
                None,
            )
            .await;
            assert_closed(status, &body, 404, "unknown_route");
            assert!(!body.contains("gpt-5"), "{path} {body}");
            assert!(!body.contains("fixture-ok"), "{path} {body}");
            assert!(!body.contains("\"data\""), "{path} {body}");
        }
        let response = client
            .get(format!(
                "http://127.0.0.1:{port}/workbuddy/v1/chat/completions"
            ))
            .timeout(Duration::from_secs(8))
            .header("authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_closed(status, &body, 405, "unknown_route");
        let (status, body) = post_chat(
            &client,
            port,
            "/v1/chat/completions",
            &key,
            "both",
            chat_message("hello"),
            None,
        )
        .await;
        assert!(!body.contains("fixture-ok"), "{status} {body}");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let stored = binding_json(&state);
        assert_eq!(stored["status"], "active");
        assert_eq!(stored["connected"], true);

        state.db.delete_workbuddy_subscription_binding().unwrap();
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "both",
            chat_message("hello"),
            None,
        )
        .await;
        assert_closed(status, &body, 404, "subscription_unavailable");
        state
            .db
            .set_workbuddy_subscription_binding(
                r#"{"schemaVersion":1,"routeId":"workbuddy","access_token":"synthetic-access-selected"}"#,
            )
            .unwrap();
        let (status, body) = post_chat(
            &client,
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            None,
        )
        .await;
        assert_closed(status, &body, 404, "subscription_unavailable");
        assert!(!body.contains("synthetic-access-selected"), "{body}");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_deactivate_does_not_rewrite_other_agents() {
    let runtime = RuntimeGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    clear_agent_selection();
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    for (app, id) in [
        (AppType::Claude, "claude-sentinel"),
        (AppType::Codex, "codex-sentinel"),
        (AppType::GrokBuild, "grok-sentinel"),
        (AppType::OpenCode, "opencode-sentinel"),
    ] {
        crate::settings::set_current_provider(&app, Some(id)).unwrap();
    }
    let sentinels = write_sentinels();
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        publish_probe(&state, &auth, &selected.identity_id).await;
        let mut claude = state.db.get_proxy_config_for_app("claude").await.unwrap();
        claude.enabled = true;
        state.db.update_proxy_config_for_app(claude).await.unwrap();
        let snapshot = subscription::deactivate_workbuddy_subscription(&state)
            .await
            .unwrap()
            .unwrap();
        assert!(!snapshot.connected);
        assert_eq!(snapshot.status, "inactive");
        assert!(state.proxy_service.is_running().await);
        assert_sentinels(&sentinels);
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Codex).as_deref(),
            Some("codex-sentinel")
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Claude).as_deref(),
            Some("claude-sentinel")
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::GrokBuild).as_deref(),
            Some("grok-sentinel")
        );
        assert_eq!(
            crate::settings::get_current_provider(&AppType::OpenCode).as_deref(),
            Some("opencode-sentinel")
        );
        let mut claude = state.db.get_proxy_config_for_app("claude").await.unwrap();
        claude.enabled = false;
        state.db.update_proxy_config_for_app(claude).await.unwrap();
        publish_probe(&state, &auth, &selected.identity_id).await;
        let snapshot = subscription::deactivate_workbuddy_subscription(&state)
            .await
            .unwrap()
            .unwrap();
        assert!(!snapshot.connected);
        assert_eq!(snapshot.status, "inactive");
        assert!(!state.proxy_service.is_running().await);
        assert_sentinels(&sentinels);
        assert_eq!(
            crate::settings::get_current_provider(&AppType::Codex).as_deref(),
            Some("codex-sentinel")
        );
    });
}

#[test]
#[serial]
fn subscription_workbuddy_port_conflict_does_not_record_connected() {
    let runtime = RuntimeGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Xai);
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        listen_loopback(&state, port).await;
        let error = subscription::publish_workbuddy_subscription(
            &state,
            &auth,
            &selected.identity_id,
            "probe-model",
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "port_conflict");
        assert_eq!(error.code(), WorkBuddySubscriptionErrorCode::PortConflict);
        assert!(state
            .db
            .get_workbuddy_subscription_binding()
            .unwrap()
            .is_none());
        assert!(!state.proxy_service.is_running().await);
        let global = state.db.get_global_proxy_config().await.unwrap();
        assert!(!global.proxy_enabled);
        assert!(listener.local_addr().is_ok());
        drop(listener);
    });
}

#[test]
#[serial]
fn subscription_workbuddy_generic_bind_rejects_workbuddy_target() {
    let (_home, _guard, state, auth) = open_fixture();
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    for app in ["workbuddy", "qoderwork", "trae-work", "opencode"] {
        let error = ProviderService::bind_managed_proxy(
            &state,
            &auth,
            BindXaiManagedRequest {
                app: app.into(),
                account_id: selected.identity_id.clone(),
                model_id: "probe-model".into(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, BindXaiManagedError::InvalidRequest),
            "{app} {error:?}"
        );
    }
    assert!(state
        .db
        .get_workbuddy_subscription_binding()
        .unwrap()
        .is_none());
}

fn assert_stream_failure(body: &str, code: &str) {
    assert!(body.contains(&format!("\"code\":\"{code}\"")), "{body}");
    assert!(!body.contains("data: [DONE]"), "{body}");
    assert!(!body.contains("\"finish_reason\":\"stop\""), "{body}");
    assert!(!body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
    assert!(!body.contains("SENTINEL_UPSTREAM_SECRET"), "{body}");
    assert!(!body.contains("synthetic-access"), "{body}");
    assert!(!body.contains("synthetic-refresh"), "{body}");
}

#[test]
#[serial]
fn subscription_workbuddy_mid_stream_error_is_not_a_normal_end() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::MidStreamError).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let (status, body) = post_chat(
            &reqwest::Client::new(),
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert!(status.is_success(), "{status} {body}");
        assert!(body.contains("partial-visible"), "{body}");
        assert_stream_failure(&body, "upstream_error");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_early_disconnect_is_not_a_normal_end() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::EarlyClose).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let (status, body) = post_chat(
            &reqwest::Client::new(),
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert!(status.is_success(), "{status} {body}");
        assert!(body.contains("partial-visible"), "{body}");
        assert_stream_failure(&body, "upstream_error");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_stall_after_first_chunk_times_out_without_done() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        crate::proxy::workbuddy::set_workbuddy_upstream_timeout_for_test(1);
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, calls, _hits) =
            spawn_upstream(ManagedAuthProvider::Openai, UpstreamMode::StallAfterChunk).await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let started = std::time::Instant::now();
        let (status, body) = post_chat(
            &reqwest::Client::new(),
            port,
            "/workbuddy/v1/chat/completions",
            &key,
            "bearer",
            chat_message("hello"),
            Some("conversation"),
        )
        .await;
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "stalled upstream should end the request"
        );
        assert!(status.is_success(), "{status} {body}");
        assert!(body.contains("partial-visible"), "{body}");
        assert_stream_failure(&body, "timeout");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        crate::proxy::workbuddy::set_workbuddy_upstream_timeout_for_test(0);
        state.proxy_service.stop().await.unwrap();
    });
}

async fn spawn_gated_text_upstream() -> (
    String,
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Sender<()>,
) {
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let release_rx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let release_rx = release_rx.clone();
    let router = axum::Router::new().route(
        "/backend-api/codex/responses",
        axum::routing::post(move |_body: axum::body::Bytes| {
            let release_rx = release_rx.clone();
            async move {
                let body = async_stream::stream! {
                    yield Ok::<bytes::Bytes, std::io::Error>(bytes::Bytes::from(text_delta_sse(
                        "fixture-ok",
                    )));
                    if let Some(rx) = release_rx.lock().await.take() {
                        let _ = rx.await;
                    }
                    yield Ok(bytes::Bytes::from(text_completed_sse("probe-model", "fixture-ok")));
                };
                (
                    [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                    axum::body::Body::from_stream(body),
                )
            }
        }),
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}"), task, release_tx)
}

#[test]
#[serial]
fn subscription_workbuddy_forwards_chunks_before_upstream_completes() {
    let runtime = RuntimeGuard::new();
    let tasks = TaskGuard::new();
    let fixtures = FixtureGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        let snapshot = publish_probe(&state, &auth, &selected.identity_id).await;
        let (upstream, task, release_tx) = spawn_gated_text_upstream().await;
        tasks.track(task);
        fixtures.arm(&snapshot.provider_id, auth.clone(), upstream);
        let key = state.db.ensure_proxy_access_key().unwrap();
        let port = state.proxy_service.get_status().await.unwrap().port;
        let mut response = reqwest::Client::new()
            .post(format!(
                "http://127.0.0.1:{port}/workbuddy/v1/chat/completions"
            ))
            .timeout(Duration::from_secs(8))
            .header("authorization", format!("Bearer {key}"))
            .json(&chat_message("hello"))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        let mut collected = String::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            assert!(
                !remaining.is_zero(),
                "first text should arrive before upstream completion: {collected}"
            );
            let chunk = tokio::time::timeout(remaining, response.chunk())
                .await
                .expect("timed out waiting for a forwarded chunk")
                .expect("read chunk");
            let Some(chunk) = chunk else {
                panic!("stream ended before text and before completion: {collected}");
            };
            collected.push_str(&String::from_utf8_lossy(&chunk));
            assert!(
                !collected.contains("data: [DONE]"),
                "normal end arrived before the upstream finished: {collected}"
            );
            if collected.contains("fixture-ok") {
                break;
            }
        }
        release_tx.send(()).expect("release upstream");
        let rest = response.text().await.unwrap();
        let all = format!("{collected}{rest}");
        assert!(all.contains("data: [DONE]"), "{all}");
        assert!(all.contains("\"finish_reason\":\"stop\""), "{all}");
        assert!(!all.contains("synthetic-access"), "{all}");
        state.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_startup_restores_active_binding() {
    let runtime = RuntimeGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        publish_probe(&state, &auth, &selected.identity_id).await;
        state.proxy_service.stop().await.unwrap();
        let stopped = binding_json(&state);
        assert_eq!(stopped["connected"], false);
        assert_eq!(stopped["desired"], true);
        assert_eq!(stopped["status"], "active");
        let restarted = AppState::new(state.db.clone());
        crate::restore_proxy_state_on_startup(&restarted).await;
        let stored = binding_json(&restarted);
        assert_eq!(stored["connected"], true);
        assert_eq!(stored["status"], "active");
        assert_eq!(stored["desired"], true);
        assert!(restarted.proxy_service.is_running().await);
        restarted.proxy_service.stop().await.unwrap();
    });
}

#[test]
#[serial]
fn subscription_workbuddy_startup_invalid_account_fails_closed() {
    let runtime = RuntimeGuard::new();
    let (_home, _guard, state, auth) = open_fixture();
    let _auth = AuthGuard::arm(auth.clone());
    let selected = seed_provider(&auth, "selected", ManagedAuthProvider::Openai);
    runtime.block_on(async {
        listen_loopback(&state, 0).await;
        publish_probe(&state, &auth, &selected.identity_id).await;
        state.proxy_service.stop().await.unwrap();
        auth.repository
            .set_status(
                &selected.credential_id,
                CredentialStatus::RequiresReauth,
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let restarted = AppState::new(state.db.clone());
        crate::restore_proxy_state_on_startup(&restarted).await;
        let stored = binding_json(&restarted);
        assert_eq!(stored["status"], "account_unavailable");
        assert_eq!(stored["connected"], false);
        assert_eq!(stored["desired"], true);
        assert!(!restarted.proxy_service.is_running().await);
    });
}
