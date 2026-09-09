//! On-demand local health. This collector owns no mutation or target-command path.
mod admission;
mod auth;
mod configuration;
mod installation;
mod proxy;
mod requests;
#[cfg(test)]
mod tests;
mod types;
mod vendor;

use crate::{
    services::{external_agents::AgentCatalogId, managed_auth::ManagedAuthOverview},
    store::AppState,
};
pub(crate) use admission::read;
use chrono::{SecondsFormat, Utc};
pub use types::AgentHealthSnapshot;
use types::{
    HealthAction as Action, HealthCheck, HealthCheckId as Id, HealthCheckState as State,
    HealthReasonCode as Reason, CHECK_IDS,
};

async fn collect(
    agent: AgentCatalogId,
    state: &AppState,
    overview: ManagedAuthOverview,
) -> AgentHealthSnapshot {
    let at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut checks: Vec<_> = CHECK_IDS
        .into_iter()
        .map(|id| HealthCheck::new(id, State::NotSupported, Reason::NotSupported, None, &at))
        .collect();
    replace(
        &mut checks,
        installation::installation(agent, state, &at).await,
    );
    if let Some(app) = configuration::app_type(agent) {
        let db = state.db.clone();
        let kind = app.clone();
        let started_at = at.clone();
        let observed = tauri::async_runtime::spawn_blocking(move || {
            configuration::observe(&db, &kind, &started_at)
        })
        .await;
        match observed {
            Ok(observation) => {
                let proxy =
                    proxy::proxy_checks(&state.db, &state.proxy_service, &app, &observation, &at)
                        .await;
                let auth_facts = if observation.live.as_ref().is_some_and(|live| {
                    crate::services::ProxyService::live_has_proxy_placeholder_for_app(&app, live)
                }) {
                    observation.expected.as_ref()
                } else {
                    observation.facts.as_ref()
                };
                let auth = auth::auth_checks(agent, &overview, auth_facts, &at);
                let request = requests::request_check(
                    &state.db,
                    app.as_str(),
                    observation.provider_id.as_deref(),
                    &at,
                );
                replace(&mut checks, observation.checks);
                replace(&mut checks, vec![request]);
                replace(&mut checks, auth);
                replace(&mut checks, proxy);
            }
            _ => {
                for id in [
                    Id::Configuration,
                    Id::Secret,
                    Id::Endpoint,
                    Id::Auth,
                    Id::Model,
                    Id::Drift,
                    Id::Proxy,
                    Id::Restart,
                    Id::LastRequest,
                ] {
                    replace(
                        &mut checks,
                        vec![HealthCheck::new(
                            id,
                            State::Unknown,
                            Reason::ReadFailed,
                            Some(Action::Refresh),
                            &at,
                        )],
                    );
                }
            }
        }
    } else {
        replace(&mut checks, vendor::vendor_checks(agent, &at).await);
    }
    // Every check shares this completed local-read time; original request time is separate.
    let finished = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    for check in &mut checks {
        check.checked_at.clone_from(&finished);
        if check
            .evidence_at
            .as_ref()
            .is_some_and(|time| time > &finished)
        {
            check.evidence_at = None;
        }
    }
    AgentHealthSnapshot {
        contract_version: 1,
        agent_id: agent,
        checked_at: finished,
        checks,
    }
}

fn replace(checks: &mut [HealthCheck], updates: Vec<HealthCheck>) {
    for update in updates {
        if let Some(check) = checks.iter_mut().find(|check| check.id == update.id) {
            *check = update;
        }
    }
}
