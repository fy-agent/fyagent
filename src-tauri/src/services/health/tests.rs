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
