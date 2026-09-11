//! Vendor configuration observations expose only owner-produced metadata.

use super::types::{
    HealthAction as Action, HealthCheck, HealthCheckId as Id, HealthCheckState as State,
    HealthReasonCode as Reason,
};
use crate::services::{
    external_agents::AgentCatalogId,
    traework_models::{self, TraeWorkHealthMetadata},
    workbuddy::{self, WorkBuddyHealthMetadata},
};

const IDS: [Id; 4] = [Id::Configuration, Id::Model, Id::Secret, Id::Endpoint];

pub(super) async fn vendor_checks(agent: AgentCatalogId, at: &str) -> Vec<HealthCheck> {
    match agent {
        AgentCatalogId::WorkBuddy => match workbuddy::health_metadata().await {
            Ok(facts) => workbuddy_checks(facts, at),
            Err(_) => unavailable(Reason::ReadFailed, at),
        },
        AgentCatalogId::TraeWork => match traework_models::health_metadata().await {
            Ok(facts) => trae_checks(facts, at),
            Err(_) => unavailable(Reason::ReadFailed, at),
        },
        AgentCatalogId::QoderWork => IDS
            .into_iter()
            .map(|id| HealthCheck::new(id, State::NotSupported, Reason::NotSupported, None, at))
            .collect(),
        _ => Vec::new(),
    }
}

fn unavailable(reason: Reason, at: &str) -> Vec<HealthCheck> {
    IDS.into_iter()
        .map(|id| HealthCheck::new(id, State::Unknown, reason, Some(Action::Refresh), at))
        .collect()
}

fn workbuddy_checks(facts: WorkBuddyHealthMetadata, at: &str) -> Vec<HealthCheck> {
    let mut checks = vec![
        check(
            Id::Configuration,
            if facts.exists {
                State::Ok
            } else {
                State::NotConfigured
            },
            if facts.exists {
                Reason::ConfigurationPresent
            } else {
                Reason::ConfigurationMissing
            },
            at,
        ),
        check(
            Id::Model,
            if facts.model_count > 0 {
                State::Ok
            } else {
                State::NotConfigured
            },
            if facts.model_count > 0 {
                Reason::ModelConfigured
            } else {
                Reason::ModelMissing
            },
            at,
        ),
    ];
    checks.extend(connection_checks(
        facts.entry_count,
        facts.credentials_present,
        facts.endpoints_present,
        facts.endpoints_valid,
        at,
    ));
    checks
}

fn trae_checks(facts: TraeWorkHealthMetadata, at: &str) -> Vec<HealthCheck> {
    let mut checks = vec![
        check(
            Id::Configuration,
            if facts.cache_present {
                State::Ok
            } else {
                State::Unknown
            },
            if facts.cache_present {
                Reason::ConfigurationPresent
            } else {
                Reason::ConfigurationUnreadable
            },
            at,
        ),
        check(
            Id::Model,
            if facts.model_count > 0 {
                State::Ok
            } else {
                State::Unknown
            },
            if facts.model_count > 0 {
                Reason::ModelConfigured
            } else {
                Reason::ModelUnknown
            },
            at,
        ),
    ];
    // No custom rows does not mean TRAE has no model: its built-in/cloud models
    // and chosen model are vendor-owned and cannot be inferred from this cache.
    checks.extend(connection_checks(
        facts.entry_count,
        facts.credentials_present,
        facts.endpoints_present,
        facts.endpoints_valid,
        at,
    ));
    checks
}

fn connection_checks(
    entries: usize,
    credentials: usize,
    endpoints: usize,
    valid_endpoints: usize,
    at: &str,
) -> [HealthCheck; 2] {
    let secret = if entries > 0 && credentials == entries {
        check(Id::Secret, State::Ok, Reason::CredentialAvailable, at)
    } else {
        // WorkBuddy accepts no-key connections and TRAE owns cloud secrets.
        // An empty stored field cannot prove a required credential is missing.
        check(Id::Secret, State::Unknown, Reason::CredentialUnknown, at)
    };
    let endpoint = if valid_endpoints < endpoints {
        check(Id::Endpoint, State::Attention, Reason::EndpointInvalid, at)
    } else if entries > 0 && valid_endpoints == entries {
        check(Id::Endpoint, State::Ok, Reason::EndpointConfigured, at)
    } else {
        check(Id::Endpoint, State::Unknown, Reason::EndpointMissing, at)
    };
    [secret, endpoint]
}

fn check(id: Id, state: State, reason: Reason, at: &str) -> HealthCheck {
    HealthCheck::new(
        id,
        state,
        reason,
        if state == State::Ok {
            None
        } else {
            Some(Action::Configuration)
        },
        at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const AT: &str = "2026-09-10T00:00:00Z";

    #[test]
    fn vendor_health_projects_saved_workbuddy_models_with_partial_connections() {
        let checks = workbuddy_checks(
            WorkBuddyHealthMetadata {
                exists: true,
                model_count: 2,
                entry_count: 2,
                credentials_present: 1,
                endpoints_present: 2,
                endpoints_valid: 1,
            },
            AT,
        );
        assert_eq!(checks[0].state, State::Ok);
        assert_eq!(checks[1].state, State::Ok);
        assert_eq!(checks[2].state, State::Unknown);
        assert_eq!(checks[3].state, State::Attention);
        assert_eq!(checks[3].reason_code, Reason::EndpointInvalid);
        assert!(checks
            .iter()
            .all(|check| check.value.is_none() && check.evidence_at.is_none()));
        assert!(checks
            .iter()
            .filter(|check| check.state == State::Ok)
            .all(|check| check.action.is_none()));
        let complete = connection_checks(2, 2, 2, 2, AT);
        assert!(complete.iter().all(|check| check.state == State::Ok));
    }

    #[test]
    fn vendor_health_does_not_equate_missing_trae_cache_with_missing_cloud_configuration() {
        let checks = trae_checks(
            TraeWorkHealthMetadata {
                cache_present: false,
                model_count: 0,
                entry_count: 0,
                credentials_present: 0,
                endpoints_present: 0,
                endpoints_valid: 0,
            },
            AT,
        );
        assert!(checks.iter().all(|check| check.state == State::Unknown));
        let workbuddy = workbuddy_checks(
            WorkBuddyHealthMetadata {
                exists: false,
                model_count: 0,
                entry_count: 0,
                credentials_present: 0,
                endpoints_present: 0,
                endpoints_valid: 0,
            },
            AT,
        );
        assert_eq!(workbuddy[0].state, State::NotConfigured);
        assert_eq!(workbuddy[1].state, State::NotConfigured);
        assert_eq!(workbuddy[2].state, State::Unknown);
    }

    #[tokio::test]
    async fn vendor_health_reserves_not_supported_for_unimplemented_observation() {
        let qoder = vendor_checks(AgentCatalogId::QoderWork, AT).await;
        assert_eq!(qoder.len(), 4);
        assert!(qoder
            .iter()
            .all(|check| check.state == State::NotSupported && check.action.is_none()));
        assert!(unavailable(Reason::ReadFailed, AT)
            .iter()
            .all(|check| check.state == State::Unknown && check.reason_code == Reason::ReadFailed));
    }
}
