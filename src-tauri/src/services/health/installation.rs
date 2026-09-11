//! Filesystem inventory projected without executing an Agent.
use super::types::{
    safe_value, HealthAction as Action, HealthCheck, HealthCheckId as Id,
    HealthCheckState as State, HealthReasonCode as Reason,
};
use crate::{
    agent_install::{self, InstallationInventoryState},
    services::external_agents::AgentCatalogId,
    store::AppState,
};
pub(super) async fn installation(
    agent: AgentCatalogId,
    state: &AppState,
    at: &str,
) -> Vec<HealthCheck> {
    let mut installation = HealthCheck::new(
        Id::Installation,
        State::Unknown,
        Reason::InstallationUnknown,
        Some(Action::Installation),
        at,
    );
    let mut conflicts = HealthCheck::new(
        Id::Conflicts,
        State::Unknown,
        Reason::InstallationUnknown,
        Some(Action::Installation),
        at,
    );
    let helper = HealthCheck::new(
        Id::Helper,
        State::NotSupported,
        Reason::NotSupported,
        None,
        at,
    );
    if let Some(inventory) = agent_install::local_health_inventory_for(agent, state).await {
        let conflict = inventory
            .reason_codes
            .contains(&agent_install::AgentReasonCode::CandidateConflict);
        match inventory.state {
            InstallationInventoryState::Single => {
                let candidate = inventory
                    .candidates
                    .iter()
                    .find(|candidate| candidate.launch_eligible || candidate.update_eligible);
                installation = HealthCheck::new(
                    Id::Installation,
                    if candidate.is_some_and(|c| c.launch_eligible) {
                        State::Ok
                    } else {
                        State::Attention
                    },
                    if candidate.is_some_and(|c| c.launch_eligible) {
                        Reason::InstallationFound
                    } else {
                        Reason::InstallationNotRunnable
                    },
                    Some(Action::Installation),
                    at,
                );
                let version = candidate
                    .and_then(|c| c.local_version.as_deref())
                    .filter(|v| {
                        v.len() <= 32 && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                    });
                installation.value = Some(
                    version.map_or_else(|| "桌面应用".to_owned(), |v| format!("{v} / 桌面应用")),
                );
                conflicts = HealthCheck::new(
                    Id::Conflicts,
                    State::Ok,
                    Reason::SingleInstallation,
                    None,
                    at,
                );
            }
            InstallationInventoryState::NotObserved => {
                installation = HealthCheck::new(
                    Id::Installation,
                    State::NotConfigured,
                    Reason::InstallationNotFound,
                    Some(Action::Installation),
                    at,
                )
            }
            InstallationInventoryState::Multiple => {
                conflicts = HealthCheck::new(
                    Id::Conflicts,
                    State::Attention,
                    Reason::MultipleInstallations,
                    Some(Action::Installation),
                    at,
                )
            }
            InstallationInventoryState::Unsupported => {
                installation = HealthCheck::new(
                    Id::Installation,
                    State::NotSupported,
                    Reason::NotSupported,
                    None,
                    at,
                )
            }
            InstallationInventoryState::Unknown if conflict => {
                conflicts = HealthCheck::new(
                    Id::Conflicts,
                    State::Attention,
                    Reason::InstallationConflict,
                    Some(Action::Installation),
                    at,
                )
            }
            InstallationInventoryState::Unknown => {}
        }
    } else {
        let tool = match agent {
            AgentCatalogId::ClaudeCode => "claude",
            AgentCatalogId::GrokBuild => "grok",
            _ => return vec![installation, conflicts, helper],
        };
        let result = tauri::async_runtime::spawn_blocking(move || {
            crate::services::tooling::observe_local_tool_health(tool)
        })
        .await;
        match result {
            Ok(Ok(local)) if local.observed_count > 0 => {
                installation = HealthCheck::new(
                    Id::Installation,
                    State::Ok,
                    Reason::InstallationFound,
                    Some(Action::Installation),
                    at,
                );
                let label = match local.source.as_deref() {
                    Some("mise") => "mise 命令行",
                    Some("npm") => "npm 命令行",
                    Some("homebrew") => "Homebrew 命令行",
                    _ => "本机命令行",
                };
                installation.value = local
                    .version
                    .as_deref()
                    .and_then(|v| safe_value(&format!("{v} / {label}")))
                    .or_else(|| Some(label.into()));
                conflicts = HealthCheck::new(
                    Id::Conflicts,
                    if local.observed_count > 1 {
                        State::Attention
                    } else {
                        State::Ok
                    },
                    if local.observed_count > 1 {
                        Reason::MultipleInstallations
                    } else {
                        Reason::SingleInstallation
                    },
                    if local.observed_count > 1 {
                        Some(Action::Installation)
                    } else {
                        None
                    },
                    at,
                );
            }
            Ok(Ok(_)) => {
                installation = HealthCheck::new(
                    Id::Installation,
                    State::NotConfigured,
                    Reason::InstallationNotFound,
                    Some(Action::Installation),
                    at,
                )
            }
            _ => {}
        }
        return vec![
            installation,
            conflicts,
            HealthCheck::new(Id::Helper, State::Ok, Reason::HelperNotRequired, None, at),
        ];
    }
    vec![installation, conflicts, helper]
}
