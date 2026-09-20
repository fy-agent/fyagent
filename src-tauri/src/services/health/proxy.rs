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
    let enabled = db.health_proxy_enabled(app.as_str()).ok().flatten();
    let takeover = observed.live.as_ref().map(|live| {
        if *app == AppType::OpenCode {
            observed.provider_id.as_deref().is_some_and(|id| {
                crate::services::opencode_models::is_managed_proxy_provider_id(id)
                    && live
                        .get("provider")
                        .and_then(|providers| providers.get(id))
                        .and_then(|provider| provider.pointer("/options/apiKey"))
                        .and_then(serde_json::Value::as_str)
                        == Some("PROXY_MANAGED")
            })
        } else {
            ProxyService::live_has_proxy_placeholder_for_app(app, live)
        }
    });
    let runtime = service.get_status().await.ok();
    let mut checks = project(app, enabled, takeover, runtime.as_ref(), observed, at);
    if *app == AppType::OpenCode && takeover == Some(true) {
        // The dedicated route serves the app's selected managed binding. A
        // retained slot pointing to this listener cannot claim another account.
        let current = crate::settings::get_current_provider(app)
            .map(|id| Ok(Some(id)))
            .unwrap_or_else(|| db.get_current_provider(app.as_str()));
        let binding_matches = current.and_then(|id| {
            let Some(id) = id else {
                return Ok(false);
            };
            Ok(observed.provider_id.as_deref() == Some(id.as_str())
                && db
                    .get_provider_by_id(&id, app.as_str())?
                    .is_some_and(|provider| provider.uses_subscription_proxy()))
        });
        if !matches!(binding_matches, Ok(true)) {
            let (state, reason) = if binding_matches.is_ok() {
                (State::Attention, Reason::ConfigurationSourceDrifted)
            } else {
                (State::Unknown, Reason::ReadFailed)
            };
            for check in checks
                .iter_mut()
                .filter(|check| matches!(check.id, Id::Proxy | Id::Drift))
            {
                *check = HealthCheck::new(check.id, state, reason, Some(Action::Configuration), at);
            }
        }
    }
    checks
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
                        AppType::OpenCode => "/opencode/v1",
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
                .is_some_and(|(expected, live)| {
                    expected.model == live.model
                        && (*app != AppType::OpenCode || expected.selected == live.selected)
                });
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
