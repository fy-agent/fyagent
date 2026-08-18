//! Post-install health probe (#32).

use serde::{Deserialize, Serialize};

use super::types::AgentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    InstalledHealthyPendingAuth,
    InstalledUnhealthy,
    ProbeUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthProbeResult {
    pub agent_id: String,
    pub status: ProbeStatus,
    pub checked_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeObservation {
    pub binary_found: bool,
    pub doctor_ok: Option<bool>,
    pub authenticated: bool,
}

pub fn probe_argv(agent_id: AgentId) -> Option<&'static [&'static str]> {
    match agent_id {
        AgentId::CodexCli => Some(&["codex", "--version"]),
        AgentId::ClaudeCode => Some(&["claude", "-v"]),
        AgentId::QoderworkCn | AgentId::DingtalkWukong | AgentId::Workbuddy | AgentId::TraeWork => {
            None
        }
    }
}

pub fn probe_from_observation(
    agent_id: AgentId,
    observation: ProbeObservation,
    now: &str,
) -> HealthProbeResult {
    let status = match agent_id {
        AgentId::QoderworkCn | AgentId::DingtalkWukong | AgentId::Workbuddy | AgentId::TraeWork => {
            ProbeStatus::ProbeUnavailable
        }
        AgentId::CodexCli | AgentId::ClaudeCode => {
            if !observation.binary_found || observation.doctor_ok == Some(false) {
                ProbeStatus::InstalledUnhealthy
            } else if observation.authenticated {
                ProbeStatus::InstalledHealthyPendingAuth
            } else {
                ProbeStatus::InstalledHealthyPendingAuth
            }
        }
    };
    HealthProbeResult {
        agent_id: agent_id.as_str().to_owned(),
        status,
        checked_at: now.to_owned(),
    }
}

pub fn probe_health(agent_id: AgentId, now: &str) -> HealthProbeResult {
    probe_from_observation(
        agent_id,
        ProbeObservation {
            binary_found: false,
            doctor_ok: None,
            authenticated: false,
        },
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthenticated_is_pending_auth_not_failure() {
        let result = probe_from_observation(
            AgentId::CodexCli,
            ProbeObservation {
                binary_found: true,
                doctor_ok: Some(true),
                authenticated: false,
            },
            "t0",
        );
        assert_eq!(result.status, ProbeStatus::InstalledHealthyPendingAuth);
    }

    #[test]
    fn doctor_failure_is_installed_unhealthy() {
        let result = probe_from_observation(
            AgentId::ClaudeCode,
            ProbeObservation {
                binary_found: true,
                doctor_ok: Some(false),
                authenticated: true,
            },
            "t0",
        );
        assert_eq!(result.status, ProbeStatus::InstalledUnhealthy);
    }

    #[test]
    fn qoderwork_does_not_use_qodercli() {
        assert_eq!(probe_argv(AgentId::QoderworkCn), None);
        let result = probe_health(AgentId::QoderworkCn, "t0");
        assert_eq!(result.status, ProbeStatus::ProbeUnavailable);
    }

    #[test]
    fn no_model_request_argv_in_probe() {
        for id in AgentId::ALL {
            if let Some(argv) = probe_argv(id) {
                assert!(!argv.iter().any(|part| *part == "exec" || *part == "-p"));
            }
        }
    }
}
