use super::*;
use super::{auth::auth_checks, configuration::Credential, types::safe_value};
use serde_json::json;

#[test]
fn health_wire_is_complete_closed_and_has_no_source_documents() {
    let at = "2026-09-09T12:00:00.000Z";
    let snapshot = AgentHealthSnapshot {
        contract_version: 1,
        agent_id: AgentCatalogId::Codex,
        checked_at: at.into(),
        checks: CHECK_IDS
            .into_iter()
            .map(|id| HealthCheck::new(id, State::NotSupported, Reason::NotSupported, None, at))
            .collect(),
    };
    let value = serde_json::to_value(snapshot).unwrap();
    assert_eq!(value["checks"].as_array().unwrap().len(), 12);
    let ids: std::collections::HashSet<_> = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 12);
    assert!(ids.contains("last_request"));
    assert_eq!(value["agentId"], "codex");
    assert_eq!(value["contractVersion"], 1);
    assert!(!value.to_string().contains("settings_config"));
}

#[test]
fn health_only_compares_the_selected_codex_endpoint() {
    let facts = configuration::routing(&crate::AppType::Codex, &json!({
        "auth": {}, "config": "model_provider = 'selected'\nmodel = 'sample-model'\n[model_providers.selected]\nbase_url = 'https://selected.example/v1'\n[model_providers.other]\nbase_url = 'https://unrelated.example/v1'\n"
    })).unwrap();
    assert_eq!(
        facts.endpoint.as_deref(),
        Some("https://selected.example/v1")
    );
    assert_eq!(facts.model.as_deref(), Some("sample-model"));
    assert!(configuration::routing(&crate::AppType::Codex, &json!({"config":"[broken"})).is_err());
}

#[test]
fn health_does_not_present_external_env_references_as_loaded_credentials() {
    let facts = configuration::routing(&crate::AppType::OpenCode, &json!({
        "model":"custom/example", "provider":{"custom":{"options":{"apiKey":"{env:PRIVATE_TOKEN}", "baseURL":"https://example.test"}}}
    })).unwrap();
    assert!(facts.credential == Credential::Unknown);
    for endpoint in [
        "file:///tmp/sample",
        "https://user:password@example.test",
        "not-a-url",
    ] {
        assert!(!configuration::valid_endpoint(endpoint));
    }
    assert!(configuration::valid_endpoint("http://127.0.0.1:15721/v1"));
}

#[test]
fn health_metadata_rejects_paths_credentials_and_hostile_model_labels() {
    for value in [
        "/Users/person/file",
        "../relative",
        "C:\\Users\\person",
        "https://host/path",
        "sk-private",
        "xai-private",
        "eyJcredential",
        "bearer data",
        "secret",
        "foo\nbar",
    ] {
        assert_eq!(safe_value(value), None);
    }
    assert_eq!(
        safe_value("1.2.3 / 桌面应用").as_deref(),
        Some("1.2.3 / 桌面应用")
    );
    let facts = configuration::routing(
        &crate::AppType::Claude,
        &json!({
            "model":"prefix-PRIVATE-CANARY-suffix", "env":{"ANTHROPIC_API_KEY":"PRIVATE-CANARY"}
        }),
    )
    .unwrap();
    assert!(facts.model.is_none());
}

#[test]
fn health_api_key_presence_does_not_claim_official_login() {
    let facts = configuration::routing(
        &crate::AppType::Claude,
        &json!({
            "env":{"ANTHROPIC_API_KEY":"fixture-key", "ANTHROPIC_MODEL":"example"}
        }),
    )
    .unwrap();
    let checks = auth_checks(
        AgentCatalogId::ClaudeCode,
        &ManagedAuthOverview::unavailable(),
        Some(&facts),
        "2026-09-09T12:00:00.000Z",
    );
    assert_eq!(checks[0].reason_code, Reason::AuthManaged);
    assert_ne!(checks[0].reason_code, Reason::AuthLoggedIn);
}

const AT: &str = "2026-09-09T12:00:00.000Z";
fn find(checks: &[HealthCheck], id: Id) -> &HealthCheck {
    checks.iter().find(|c| c.id == id).unwrap()
}
fn connection(
    consumer: crate::services::managed_auth::ManagedAuthConsumer,
    provider: crate::services::managed_auth::ManagedAuthProvider,
) -> crate::services::managed_auth::ManagedAuthConnectionSummary {
    use crate::services::managed_auth::*;
    ManagedAuthConnectionSummary {
        connection_id: "fixture".into(),
        revision: "fixture".into(),
        consumer,
        target_id: None,
        target_label: None,
        provider: Some(provider),
        account_id: None,
        auth_status: ManagedAuthConnectionState::Connected,
        unmanaged_native_session: false,
        credential_manager: ManagedAuthCredentialManager::Fyagent,
        request_mode: if consumer == ManagedAuthConsumer::Opencode {
            ManagedAuthRequestMode::ProviderConnections
        } else {
            ManagedAuthRequestMode::OfficialSubscription
        },
        request_provider_label: None,
        official_session_preserved: None,
        pending_restart: false,
        allowed_actions: vec![],
        checked_at: AT.into(),
        reason_codes: vec![],
    }
}

#[test]
fn health_opencode_active_provider_owns_auth_and_restart() {
    use crate::services::managed_auth::*;
    let mut overview = ManagedAuthOverview::unavailable();
    overview.reason_codes.clear();
    let mut other = connection(ManagedAuthConsumer::Opencode, ManagedAuthProvider::Xai);
    other.pending_restart = true;
    other.auth_status = ManagedAuthConnectionState::PendingRestart;
    let active = connection(ManagedAuthConsumer::Opencode, ManagedAuthProvider::Openai);
    overview.connections = vec![other, active];
    let facts = configuration::routing(
        &crate::AppType::OpenCode,
        &json!({"model":"openai/example"}),
    )
    .unwrap();
    let checks = auth_checks(AgentCatalogId::OpenCode, &overview, Some(&facts), AT);
    assert_eq!(find(&checks, Id::Auth).reason_code, Reason::AuthManaged);
    assert_eq!(find(&checks, Id::Secret).state, State::Ok);
    assert_eq!(
        find(&checks, Id::Restart).reason_code,
        Reason::RestartNotRequired
    );
    let unknown = configuration::routing(
        &crate::AppType::OpenCode,
        &json!({"model":"unknown/example"}),
    )
    .unwrap();
    let checks = auth_checks(AgentCatalogId::OpenCode, &overview, Some(&unknown), AT);
    assert_eq!(find(&checks, Id::Auth).state, State::Unknown);
    assert_eq!(find(&checks, Id::Restart).state, State::Unknown);
    assert!(!unknown.official);
}

#[test]
fn health_external_auth_and_unavailable_observation_never_claim_logged_out() {
    use crate::services::managed_auth::*;
    let mut overview = ManagedAuthOverview::unavailable();
    overview.reason_codes.clear();
    let mut native = connection(ManagedAuthConsumer::Codex, ManagedAuthProvider::Openai);
    native.auth_status = ManagedAuthConnectionState::Disconnected;
    native.reason_codes = vec![ManagedAuthReasonCode::ExternalChangeDetected];
    overview.connections = vec![native];
    let facts = configuration::routing(
        &crate::AppType::Codex,
        &json!({"config":"model = 'example'", "auth":{}}),
    )
    .unwrap();
    let checks = auth_checks(AgentCatalogId::Codex, &overview, Some(&facts), AT);
    assert_eq!(find(&checks, Id::Auth).state, State::Unknown);
    assert_eq!(find(&checks, Id::Secret).state, State::Unknown);
    assert_eq!(find(&checks, Id::Drift).state, State::Attention);
    overview.connections[0].reason_codes.clear();
    overview.connections[0].auth_status = ManagedAuthConnectionState::Connected;
    overview.reason_codes = vec![ManagedAuthReasonCode::ObserverUnavailable];
    let checks = auth_checks(AgentCatalogId::Codex, &overview, Some(&facts), AT);
    assert_eq!(find(&checks, Id::Auth).state, State::Unknown);
}

#[test]
fn health_unmanaged_codex_native_session_remains_unknown() {
    use crate::services::managed_auth::{consumers::codex, ManagedAuthConnectionState};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let home = tempfile::tempdir().unwrap();
    let config = "model = 'fixture-model'\n";
    let payload = URL_SAFE_NO_PAD.encode(
        json!({"chatgpt_account_id":"fixture-native-account", "email":"fixture@example.test"})
            .to_string(),
    );
    let token = format!("e30.{payload}.fixture-signature");
    let auth = json!({
        "OPENAI_API_KEY": null,
        "auth_mode": "chatgpt",
        "tokens": {
            "id_token": token,
            "access_token": token,
            "refresh_token": "fixture-refresh-canary",
            "account_id": "fixture-native-account"
        }
    });
    let auth_bytes = serde_json::to_vec(&auth).unwrap();
    std::fs::write(home.path().join("config.toml"), config).unwrap();
    std::fs::write(home.path().join("auth.json"), &auth_bytes).unwrap();
    let observed = codex::observe_codex_home(home.path());
    assert!(matches!(
        observed.auth_state,
        codex::CodexNativeAuthState::ChatGptKnown { .. }
    ));
    // Empty account/connection inputs model a device with no managed-auth rows.
    let summary = codex::connection_summary(&observed, None, None, &[], AT.into());
    assert_eq!(
        summary.auth_status,
        ManagedAuthConnectionState::Disconnected
    );
    assert!(summary.account_id.is_none());
    let wire = serde_json::to_string(&summary).unwrap();
    for private in [
        &token,
        "fixture-refresh-canary",
        "fixture-native-account",
        "fixture@example.test",
        "unmanagedNativeSession",
        "unmanaged_native_session",
    ] {
        assert!(!wire.contains(private));
    }
    let mut overview = ManagedAuthOverview::unavailable();
    overview.reason_codes.clear();
    overview.connections = vec![summary];
    let facts = configuration::routing(
        &crate::AppType::Codex,
        &json!({"config": config, "auth": auth}),
    )
    .unwrap();
    let checks = auth_checks(AgentCatalogId::Codex, &overview, Some(&facts), AT);
    assert_eq!(find(&checks, Id::Auth).state, State::Unknown);
    assert_eq!(find(&checks, Id::Auth).reason_code, Reason::AuthUnknown);
    assert_eq!(find(&checks, Id::Secret).state, State::Unknown);
    assert_eq!(
        find(&checks, Id::Secret).reason_code,
        Reason::CredentialUnknown
    );
    assert!(!checks.iter().any(|check| check.id == Id::Drift));
    assert_eq!(
        std::fs::read(home.path().join("auth.json")).unwrap(),
        auth_bytes
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("config.toml")).unwrap(),
        config
    );
}

#[test]
fn health_codex_missing_auth_and_managed_disconnect_remain_logged_out() {
    use crate::services::managed_auth::{consumers::codex, *};

    let home = tempfile::tempdir().unwrap();
    let missing = codex::observe_codex_home(home.path());
    let known = codex::CodexManagedAuthObservation {
        auth_state: codex::CodexNativeAuthState::ChatGptKnown {
            account_id: "fixture-native-account".into(),
            revision: "fixture".into(),
        },
        ..missing.clone()
    };
    let disconnected = ConnectionRecord {
        connection_id: "fixture".into(),
        consumer: ManagedAuthConsumer::Codex,
        target_id: String::new(),
        provider_slot: "openai".into(),
        credential_id: None,
        desired_revision: "fixture".into(),
        observed_revision: None,
        status: ConnectionStatus::Disconnected,
        request_mode: ManagedAuthRequestMode::OfficialSubscription,
        request_provider_label: Some("openai".into()),
        official_session_preserved: None,
        pending_restart: false,
        created_at: 0,
        updated_at: 0,
    };
    let facts =
        configuration::routing(&crate::AppType::Codex, &json!({"config":"", "auth":{}})).unwrap();
    for (observed, record) in [(&missing, None), (&known, Some(&disconnected))] {
        let summary = codex::connection_summary(observed, None, record, &[], AT.into());
        assert!(!summary.unmanaged_native_session);
        let mut overview = ManagedAuthOverview::unavailable();
        overview.reason_codes.clear();
        overview.connections = vec![summary];
        let checks = auth_checks(AgentCatalogId::Codex, &overview, Some(&facts), AT);
        assert_eq!(find(&checks, Id::Auth).state, State::NotConfigured);
        assert_eq!(find(&checks, Id::Auth).reason_code, Reason::AuthLoggedOut);
        assert_eq!(find(&checks, Id::Secret).state, State::NotConfigured);
        assert_eq!(
            find(&checks, Id::Secret).reason_code,
            Reason::CredentialMissing
        );
    }
}

#[test]
fn health_codex_custom_credentials_use_the_active_route_and_project_stored_key() {
    let app = crate::AppType::Codex;
    let config = "model_provider = 'custom'\nmodel = 'example'\n[model_providers.custom]\nbase_url = 'https://example.test/v1'\n";
    let retained = json!({"auth":{"OPENAI_API_KEY":"retained-official-fixture"},"config":config});
    let live = configuration::routing(&app, &retained).unwrap();
    assert!(live.credential == Credential::Unknown);
    let native = configuration::routing(&app, &json!({"auth":retained["auth"], "config":format!("{config}requires_openai_auth = true\n")})).unwrap();
    assert!(native.credential == Credential::Native);
    assert!(native.official);
    let db = crate::database::Database::memory().unwrap();
    let provider =
        crate::provider::Provider::with_id("custom".into(), "fixture".into(), retained, None);
    let projected =
        crate::services::provider::build_health_settings_projection(&db, &app, &provider).unwrap();
    let expected = configuration::routing(&app, &projected).unwrap();
    assert!(expected.credential == Credential::Present);
    assert_eq!(expected.endpoint, live.endpoint);
}

#[test]
fn health_opencode_fragment_comparison_ignores_inactive_connections_and_preserves_selected_model() {
    let app = crate::AppType::OpenCode;
    let fragment = json!({"options":{"apiKey":"fixture-key","baseURL":"https://example.test/v1"},"models":{"example":{}}});
    let live = configuration::routing(&app, &json!({"model":"custom/example","provider":{"custom":fragment,"inactive":{"options":{"baseURL":"https://other.test"}}}})).unwrap();
    let expected =
        configuration::expected_routing(&app, "custom", fragment.clone(), Some(&live)).unwrap();
    assert!(live == expected);
    let changed = configuration::expected_routing(
        &app,
        "custom",
        json!({"options":{"apiKey":"fixture-key","baseURL":"https://changed.test"}}),
        Some(&live),
    )
    .unwrap();
    assert!(live != changed);
    assert!(
        configuration::expected_routing(&app, "custom", serde_json::Value::Null, Some(&live))
            .is_err()
    );
    assert!(configuration::routing(&crate::AppType::Claude, &json!({"env":"invalid"})).is_err());
}

#[test]
fn health_proxy_checks_the_selected_listener_path_model_and_upstream_auth() {
    use crate::proxy::types::ProxyStatus;
    use configuration::{ConfigurationObservation, RoutingFacts};
    let mut observed = ConfigurationObservation {
        checks: vec![],
        provider_id: Some("fixture".into()),
        live: None,
        facts: Some(RoutingFacts {
            endpoint: Some("http://127.0.0.1:15721/v1".into()),
            model: Some("example".into()),
            credential: Credential::Unknown,
            official: false,
            selected: Some("custom".into()),
        }),
        expected: Some(RoutingFacts {
            endpoint: Some("https://example.test/v1".into()),
            model: Some("example".into()),
            credential: Credential::Present,
            official: false,
            selected: Some("custom".into()),
        }),
    };
    let mut runtime = ProxyStatus {
        running: true,
        address: "0.0.0.0".into(),
        port: 15721,
        ..Default::default()
    };
    let check = |observed: &ConfigurationObservation, runtime: &ProxyStatus, enabled| {
        proxy::project(
            &crate::AppType::Codex,
            enabled,
            Some(true),
            Some(runtime),
            observed,
            AT,
        )
    };
    let healthy = check(&observed, &runtime, Some(true));
    assert_eq!(find(&healthy, Id::Proxy).reason_code, Reason::ProxyRunning);
    assert_eq!(find(&healthy, Id::Drift).state, State::Ok);
    assert_eq!(find(&healthy, Id::Auth).reason_code, Reason::AuthManaged);
    observed.facts.as_mut().unwrap().endpoint = Some("http://127.0.0.1:15721/wrong".into());
    assert_eq!(
        find(&check(&observed, &runtime, Some(true)), Id::Proxy).state,
        State::Attention
    );
    observed.facts.as_mut().unwrap().endpoint = Some("http://[::1]:15721/v1".into());
    runtime.address = "::".into();
    assert_eq!(
        find(&check(&observed, &runtime, Some(true)), Id::Proxy).state,
        State::Ok
    );
    observed.facts.as_mut().unwrap().model = Some("changed".into());
    assert_eq!(
        find(&check(&observed, &runtime, Some(true)), Id::Drift).state,
        State::Attention
    );
    assert_eq!(
        find(&check(&observed, &runtime, None), Id::Proxy).state,
        State::Unknown
    );
    let stopped = check(&observed, &ProxyStatus::default(), Some(true));
    assert_eq!(find(&stopped, Id::Proxy).state, State::Blocked);
    assert_eq!(find(&stopped, Id::Drift).state, State::Unknown);
    observed.expected.as_mut().unwrap().credential = Credential::Native;
    assert!(!check(&observed, &runtime, Some(true))
        .iter()
        .any(|c| c.id == Id::Secret));
}

#[test]
fn health_drift_explains_only_routing_metadata_without_values() {
    let base = configuration::routing(&crate::AppType::Claude, &json!({
        "model":"fixture-model", "env":{"ANTHROPIC_API_KEY":"fixture-private", "ANTHROPIC_BASE_URL":"https://fixture.test"}
    })).unwrap();
    let mut changed = base.clone();
    changed.model = Some("another-model".into());
    assert_eq!(
        configuration::drift_reason(&base, &changed),
        Reason::ConfigurationModelDrifted
    );
    changed.endpoint = Some("https://another.test".into());
    assert_eq!(
        configuration::drift_reason(&base, &changed),
        Reason::ConfigurationDrifted
    );
    let preferences = configuration::routing(&crate::AppType::Claude, &json!({
        "model":"fixture-model", "env":{"ANTHROPIC_API_KEY":"fixture-private", "ANTHROPIC_BASE_URL":"https://fixture.test"},
        "theme":"light", "mcpServers":{"unrelated":{"command":"fixture"}}
    })).unwrap();
    assert_eq!(
        configuration::drift_reason(&base, &preferences),
        Reason::ConfigurationInSync
    );
    let reason = serde_json::to_string(&configuration::drift_reason(&base, &changed)).unwrap();
    assert!(!reason.contains("fixture"));
    assert!(!reason.contains("https"));
}

#[test]
fn health_grok_unrecognized_profile_is_not_corrupt_toml_or_native_auth() {
    let unknown = json!({"config":"[models]\ndefault = 'builtin-profile'\n[model.custom]\nmodel = 'custom-model'\n"});
    assert!(configuration::unrecognized_grok_profile(
        &crate::AppType::GrokBuild,
        Some(&unknown)
    ));
    assert!(configuration::routing(&crate::AppType::GrokBuild, &unknown).is_err());
    for value in [
        json!({"config":"[broken"}),
        json!({"config":""}),
        json!({"config":"[mcp_servers.fixture]\ncommand = 'fixture'\n"}),
    ] {
        assert!(!configuration::unrecognized_grok_profile(
            &crate::AppType::GrokBuild,
            Some(&value)
        ));
    }
    assert!(!configuration::unrecognized_grok_profile(
        &crate::AppType::Codex,
        Some(&unknown)
    ));
}

#[test]
#[serial_test::serial]
fn release_integration_health_opencode_observes_owned_route_and_reports_real_drift() {
    use crate::provider::{Provider, ProviderMeta};
    use std::sync::Arc;
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
                None => std::env::remove_var("FYAGENT_TEST_HOME"),
            }
            crate::settings::reload_settings().unwrap();
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let _home = RestoreHome(std::env::var_os("FYAGENT_TEST_HOME"));
    std::env::set_var("FYAGENT_TEST_HOME", directory.path());
    crate::settings::reload_settings().unwrap();
    let db = Arc::new(crate::database::Database::memory().unwrap());
    let state = AppState::new(db.clone());
    let mut provider = Provider::with_id(
        "fyagent-openai-opencode-fixture".into(),
        "Subscription".into(),
        json!({"npm":"@ai-sdk/openai", "options":{"baseURL":"https://upstream.example/v1","apiKey":"PROXY_MANAGED"},"models":{"fixture":{}}}),
        None,
    );
    provider.meta = Some(ProviderMeta {
        provider_type: Some("codex_oauth".into()),
        ..Default::default()
    });
    db.save_provider("opencode", &provider).unwrap();
    db.set_current_provider("opencode", &provider.id).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let mut global = runtime.block_on(db.get_global_proxy_config()).unwrap();
    global.listen_address = "127.0.0.1".into();
    global.listen_port = 0;
    runtime
        .block_on(db.update_global_proxy_config(global))
        .unwrap();
    let listener = runtime.block_on(state.proxy_service.start()).unwrap();
    let mut intent = runtime
        .block_on(db.get_proxy_config_for_app("opencode"))
        .unwrap();
    intent.enabled = true;
    runtime
        .block_on(db.update_proxy_config_for_app(intent))
        .unwrap();
    let endpoint = format!("http://127.0.0.1:{}/opencode/v1", listener.port);
    let mut live = json!({"model":format!("{}/fixture",provider.id),"provider":{&provider.id:provider.settings_config.clone()}});
    live["provider"][&provider.id]["options"]["baseURL"] = json!(endpoint);
    let path = crate::opencode_config::get_opencode_config_path();
    assert!(path.starts_with(directory.path()));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let observe = |value: &serde_json::Value| {
        let bytes = serde_json::to_vec_pretty(value).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let before = db.export_sql_string().unwrap();
        let observation = configuration::observe(&db, &crate::AppType::OpenCode, AT);
        let checks = runtime.block_on(proxy::proxy_checks(
            &db,
            &state.proxy_service,
            &crate::AppType::OpenCode,
            &observation,
            AT,
        ));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let rows = |sql: &str| {
            sql.lines()
                .filter(|line| !line.starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(rows(&db.export_sql_string().unwrap()), rows(&before));
        checks
    };
    let healthy = observe(&live);
    assert_eq!(find(&healthy, Id::Proxy).reason_code, Reason::ProxyRunning);
    assert_eq!(
        find(&healthy, Id::Drift).reason_code,
        Reason::ConfigurationInSync
    );
    for wrong_endpoint in [
        format!("http://127.0.0.1:{}/grokbuild/v1", listener.port),
        "http://127.0.0.1:1/opencode/v1".into(),
    ] {
        let mut changed = live.clone();
        changed["provider"][&provider.id]["options"]["baseURL"] = json!(wrong_endpoint);
        assert_eq!(find(&observe(&changed), Id::Proxy).state, State::Attention);
        assert_eq!(find(&observe(&changed), Id::Drift).state, State::Attention);
    }
    let mut changed = live.clone();
    changed["model"] = json!(format!("{}/different-model", provider.id));
    assert_eq!(find(&observe(&changed), Id::Drift).state, State::Attention);
    let mut other = provider.clone();
    other.id = "fyagent-openai-opencode-other".into();
    db.save_provider("opencode", &other).unwrap();
    changed["provider"][&other.id] = live["provider"][&provider.id].clone();
    changed["model"] = json!(format!("{}/fixture", other.id));
    assert_eq!(
        find(&observe(&changed), Id::Drift).reason_code,
        Reason::ConfigurationSourceDrifted
    );
    runtime.block_on(state.proxy_service.stop()).unwrap();
    assert_eq!(
        find(&observe(&live), Id::Proxy).reason_code,
        Reason::ProxyStopped
    );
}
