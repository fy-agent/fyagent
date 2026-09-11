//! Local authentication metadata is distinct from successful remote requests.
use super::{
    configuration::{self, Credential},
    types::{
        HealthAction as Action, HealthCheck, HealthCheckId as Id, HealthCheckState as State,
        HealthReasonCode as Reason,
    },
};
use crate::services::{
    external_agents::AgentCatalogId,
    managed_auth::{
        ManagedAuthConnectionState, ManagedAuthConsumer, ManagedAuthOverview,
        ManagedAuthReasonCode, ManagedAuthRequestMode,
    },
};
pub(super) fn auth_checks(
    agent: AgentCatalogId,
    overview: &ManagedAuthOverview,
    facts: Option<&configuration::RoutingFacts>,
    at: &str,
) -> Vec<HealthCheck> {
    let mut auth = HealthCheck::new(
        Id::Auth,
        State::Unknown,
        Reason::AuthUnknown,
        Some(Action::Authentication),
        at,
    );
    let mut restart = HealthCheck::new(
        Id::Restart,
        State::Unknown,
        Reason::RestartUnknown,
        Some(Action::Authentication),
        at,
    );
    // API key presence is local credential evidence, never a remote-login claim.
    if facts.is_some_and(|f| matches!(f.credential, Credential::Present | Credential::NotRequired))
    {
        auth = HealthCheck::new(
            Id::Auth,
            State::Ok,
            Reason::AuthManaged,
            Some(Action::ModelTest),
            at,
        );
    }
    let consumer = match agent {
        AgentCatalogId::Codex => Some(ManagedAuthConsumer::Codex),
        AgentCatalogId::GrokBuild => Some(ManagedAuthConsumer::Grokbuild),
        AgentCatalogId::OpenCode => Some(ManagedAuthConsumer::Opencode),
        _ => None,
    };
    let connections: Vec<_> = overview
        .connections
        .iter()
        .filter(|c| Some(c.consumer) == consumer)
        .filter(|c| {
            if agent != AgentCatalogId::OpenCode {
                return true;
            }
            matches!(
                (facts.and_then(|f| f.selected.as_deref()), c.provider),
                (
                    Some("openai"),
                    Some(crate::services::managed_auth::ManagedAuthProvider::Openai)
                ) | (
                    Some("xai"),
                    Some(crate::services::managed_auth::ManagedAuthProvider::Xai)
                ) | (
                    Some("github-copilot"),
                    Some(crate::services::managed_auth::ManagedAuthProvider::GithubCopilot)
                )
            )
        })
        .collect();
    if connections.iter().any(|c| c.pending_restart) {
        restart = HealthCheck::new(
            Id::Restart,
            State::Attention,
            Reason::RestartRequired,
            Some(Action::Authentication),
            at,
        );
    } else if !connections.is_empty()
        && !overview
            .reason_codes
            .contains(&ManagedAuthReasonCode::ObserverUnavailable)
        && connections.iter().all(|c| {
            !matches!(
                c.auth_status,
                ManagedAuthConnectionState::Unavailable | ManagedAuthConnectionState::Checking
            ) && !c
                .reason_codes
                .contains(&ManagedAuthReasonCode::ObserverUnavailable)
        })
    {
        restart = HealthCheck::new(Id::Restart, State::Ok, Reason::RestartNotRequired, None, at);
    }
    let mut out = vec![auth, restart];
    if facts.is_some_and(|f| f.credential == Credential::Native) {
        // Multiple OpenCode account connections are not interchangeable with its selected model.
        let connection = connections.first();
        if let Some(connection) = connection {
            let uncertain = overview
                .reason_codes
                .contains(&ManagedAuthReasonCode::ObserverUnavailable)
                || connection.reason_codes.iter().any(|r| {
                    matches!(
                        r,
                        ManagedAuthReasonCode::ObserverUnavailable
                            | ManagedAuthReasonCode::NativeProjectionUnavailable
                            | ManagedAuthReasonCode::ExternalChangeDetected
                    )
                });
            let (state, reason, secret_state, secret_reason) = if uncertain {
                (
                    State::Unknown,
                    Reason::AuthUnknown,
                    State::Unknown,
                    Reason::CredentialUnknown,
                )
            } else {
                match connection.auth_status {
                    ManagedAuthConnectionState::Connected => (
                        State::Ok,
                        if connection.request_mode == ManagedAuthRequestMode::ProviderConnections {
                            Reason::AuthManaged
                        } else {
                            Reason::AuthLoggedIn
                        },
                        State::Ok,
                        Reason::CredentialAvailable,
                    ),
                    ManagedAuthConnectionState::PendingRestart => (
                        State::Attention,
                        Reason::RestartRequired,
                        State::Ok,
                        Reason::CredentialAvailable,
                    ),
                    ManagedAuthConnectionState::Disconnected => (
                        State::NotConfigured,
                        Reason::AuthLoggedOut,
                        State::NotConfigured,
                        Reason::CredentialMissing,
                    ),
                    ManagedAuthConnectionState::RequiresReauth => (
                        State::Blocked,
                        Reason::AuthLoggedOut,
                        State::Blocked,
                        Reason::CredentialMissing,
                    ),
                    ManagedAuthConnectionState::Checking
                    | ManagedAuthConnectionState::Unavailable => (
                        State::Unknown,
                        Reason::AuthUnknown,
                        State::Unknown,
                        Reason::CredentialUnknown,
                    ),
                }
            };
            out[0] = HealthCheck::new(Id::Auth, state, reason, Some(Action::Authentication), at);
            out.push(HealthCheck::new(
                Id::Secret,
                secret_state,
                secret_reason,
                Some(Action::Authentication),
                at,
            ));
            if connection
                .reason_codes
                .contains(&ManagedAuthReasonCode::ExternalChangeDetected)
            {
                out.push(HealthCheck::new(
                    Id::Drift,
                    State::Attention,
                    Reason::ConfigurationDrifted,
                    Some(Action::Configuration),
                    at,
                ));
            }
        }
    }
    out
}
