//! Proxy evidence is tied to this Agent's persisted intent and actual route.
use super::configuration::{valid_endpoint, Credential};
use super::{
    configuration::ConfigurationObservation,
    types::{
        HealthAction as Action, HealthCheck, HealthCheckId as Id, HealthCheckState as State,
        HealthReasonCode as Reason,
    },
};
use crate::{database::Database, proxy::types::ProxyStatus, services::ProxyService, AppType};

pub(super) async fn proxy_checks(
    db: &Database,
    service: &ProxyService,
    app: &AppType,
    observed: &ConfigurationObservation,
    at: &str,
) -> Vec<HealthCheck> {
    if *app == AppType::OpenCode {
        return vec![HealthCheck::new(
            Id::Proxy,
            State::NotSupported,
            Reason::NotSupported,
            None,
            at,
        )];
    }
    let enabled = db.health_proxy_enabled(app.as_str()).ok().flatten();
    let takeover = observed
        .live
        .as_ref()
        .map(|live| ProxyService::live_has_proxy_placeholder_for_app(app, live));
    let runtime = service.get_status().await.ok();
    project(app, enabled, takeover, runtime.as_ref(), observed, at)
}

pub(super) fn project(
    app: &AppType,
    enabled: Option<bool>,
    takeover: Option<bool>,
    runtime: Option<&ProxyStatus>,
    observed: &ConfigurationObservation,
    at: &str,
) -> Vec<HealthCheck> {
    let running = runtime.map(|status| status.running);
    let route_matches = runtime.is_some_and(|status| {
        observed
            .facts
            .as_ref()
            .and_then(|f| f.endpoint.as_deref())
            .is_some_and(|endpoint| {
                url::Url::parse(endpoint).is_ok_and(|url| {
                    let actual = url.host_str().unwrap_or("");
                    let same_host = actual == status.address
                        || (matches!(status.address.as_str(), "0.0.0.0" | "127.0.0.1")
                            && matches!(actual, "127.0.0.1" | "localhost"))
                        || (matches!(status.address.as_str(), "::" | "::1" | "[::]" | "[::1]")
                            && matches!(actual, "::1" | "[::1]"));
                    let expected_path = match app {
                        AppType::Codex => "/v1",
                        AppType::GrokBuild => "/grokbuild/v1",
                        _ => "",
                    };
                    url.scheme() == "http"
                        && same_host
                        && url.port_or_known_default() == Some(status.port)
                        && url.path().trim_end_matches('/') == expected_path
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.query().is_none()
                        && url.fragment().is_none()
                })
            })
    });
    let (state, reason) = match (enabled, takeover, running) {
        (Some(true), Some(true), Some(true)) if route_matches => (State::Ok, Reason::ProxyRunning),
        (_, Some(true), Some(false)) => (State::Blocked, Reason::ProxyStopped),
        (Some(false), Some(false), _) => (State::Ok, Reason::ProxyNotUsed),
        (None, _, _) => (State::Unknown, Reason::ProxyUnknown),
        (Some(true), Some(false), _) | (_, Some(true), Some(true)) => {
            (State::Attention, Reason::ConfigurationDrifted)
        }
        _ => (State::Unknown, Reason::ProxyUnknown),
    };
    let mut checks = vec![HealthCheck::new(
        Id::Proxy,
        state,
        reason,
        Some(Action::Configuration),
        at,
    )];
    if takeover == Some(true) {
        // Managed proxy projection intentionally differs from the saved upstream.
        // Compare only its model and listener; never report that intentional rewrite as drift.
        let comparable =
            enabled.is_some() && runtime.is_some_and(|s| s.port > 0 && !s.address.is_empty());
        let aligned = enabled == Some(true)
            && route_matches
            && observed
                .expected
                .as_ref()
                .zip(observed.facts.as_ref())
                .is_some_and(|(expected, live)| expected.model == live.model);
        checks.push(HealthCheck::new(
            Id::Drift,
            if aligned {
                State::Ok
            } else if comparable && observed.expected.is_some() {
                State::Attention
            } else {
                State::Unknown
            },
            if aligned {
                Reason::ConfigurationInSync
            } else if comparable && observed.expected.is_some() {
                Reason::ConfigurationDrifted
            } else {
                Reason::ConfigurationDriftUnknown
            },
            Some(Action::Configuration),
            at,
        ));
        if let Some(upstream) = &observed.expected {
            let (state, reason) = match upstream.credential {
                Credential::Present => (State::Ok, Reason::CredentialAvailable),
                Credential::Missing => (State::Blocked, Reason::CredentialMissing),
                Credential::NotRequired => (State::Ok, Reason::CredentialNotRequired),
                Credential::Native | Credential::Unknown => {
                    (State::Unknown, Reason::CredentialUnknown)
                }
            };
            // Native credentials have their own account observer; don't erase
            // that result merely because the upstream doesn't store an API key.
            if upstream.credential != Credential::Native {
                checks.push(HealthCheck::new(
                    Id::Secret,
                    state,
                    reason,
                    Some(Action::Configuration),
                    at,
                ));
            }
            if state == State::Ok {
                checks.push(HealthCheck::new(
                    Id::Auth,
                    State::Ok,
                    Reason::AuthManaged,
                    Some(Action::ModelTest),
                    at,
                ));
            }
            if upstream
                .endpoint
                .as_deref()
                .is_some_and(|url| !valid_endpoint(url))
            {
                checks.push(HealthCheck::new(
                    Id::Endpoint,
                    State::Blocked,
                    Reason::EndpointInvalid,
                    Some(Action::Configuration),
                    at,
                ));
            }
        }
    }
    checks
}
