//! Environment preflight (#27).

use serde::{Deserialize, Serialize};

use super::types::{AgentId, LayerState};

pub const PF_OS_UNSUPPORTED: &str = "PF_OS_UNSUPPORTED";
pub const PF_ARCH_UNSUPPORTED: &str = "PF_ARCH_UNSUPPORTED";
pub const PF_REQUIREMENT_UNVERIFIED: &str = "PF_REQUIREMENT_UNVERIFIED";
pub const PF_AUTH_REQUIRED: &str = "PF_AUTH_REQUIRED";
pub const OS_TTL_SECS: i64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreflightItemState {
    Pass,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItem {
    pub code: String,
    pub state: PreflightItemState,
    pub message: String,
    pub hint: String,
    pub checked_at: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreflightLayerState {
    pub preflight_state: LayerState,
    pub checks: Vec<PreflightItem>,
    pub checked_at: String,
}

impl PreflightLayerState {
    pub fn unknown(checked_at: impl Into<String>) -> Self {
        Self {
            preflight_state: LayerState::Unknown,
            checks: Vec::new(),
            checked_at: checked_at.into(),
        }
    }

    pub fn from_checks(checks: Vec<PreflightItem>, checked_at: impl Into<String>) -> Self {
        let preflight_state = rollup(&checks);
        Self {
            preflight_state,
            checks,
            checked_at: checked_at.into(),
        }
    }
}

pub fn rollup(checks: &[PreflightItem]) -> LayerState {
    let blocking: Vec<&PreflightItem> = checks
        .iter()
        .filter(|item| item.code != PF_REQUIREMENT_UNVERIFIED)
        .collect();
    if blocking
        .iter()
        .any(|item| matches!(item.state, PreflightItemState::Fail))
    {
        return LayerState::Fail;
    }
    if blocking.is_empty()
        || blocking
            .iter()
            .any(|item| matches!(item.state, PreflightItemState::Unknown))
    {
        return LayerState::Unknown;
    }
    if blocking
        .iter()
        .any(|item| matches!(item.state, PreflightItemState::Warn))
    {
        return LayerState::Warn;
    }
    LayerState::Ok
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineFacts {
    pub os: String,
    pub arch: String,
    pub macos_major: Option<u32>,
    pub windows_build: Option<u32>,
}

impl MachineFacts {
    pub fn host() -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            macos_major: None,
            windows_build: None,
        }
    }
}

pub fn observe_preflight(
    agent_id: AgentId,
    facts: &MachineFacts,
    now: &str,
) -> PreflightLayerState {
    let mut checks = vec![
        os_check(agent_id, facts, now),
        arch_check(agent_id, facts, now),
    ];
    checks.push(unverified(
        PF_REQUIREMENT_UNVERIFIED,
        "RAM and disk minima are not published as a hard floor.",
        now,
    ));
    checks.push(PreflightItem {
        code: PF_AUTH_REQUIRED.to_owned(),
        state: PreflightItemState::Warn,
        message: "Account login is required after install.".to_owned(),
        hint: "Complete official login after the package is healthy.".to_owned(),
        checked_at: now.to_owned(),
        source: "preflight".to_owned(),
    });
    PreflightLayerState::from_checks(checks, now)
}

fn os_check(agent_id: AgentId, facts: &MachineFacts, now: &str) -> PreflightItem {
    let supported = match facts.os.as_str() {
        "linux" => false,
        "windows" | "macos" => matches!(
            agent_id,
            AgentId::QoderworkCn
                | AgentId::DingtalkWukong
                | AgentId::Workbuddy
                | AgentId::TraeWork
                | AgentId::CodexCli
                | AgentId::ClaudeCode
        ),
        _ => false,
    };
    if supported {
        pass(
            PF_OS_UNSUPPORTED,
            "Operating system is in the published matrix.",
            now,
        )
    } else {
        fail(
            PF_OS_UNSUPPORTED,
            "Operating system is not an official FyAgent target.",
            now,
        )
    }
}

fn arch_check(agent_id: AgentId, facts: &MachineFacts, now: &str) -> PreflightItem {
    if agent_id == AgentId::QoderworkCn && facts.os == "windows" && facts.arch != "x86_64" {
        return fail(
            PF_ARCH_UNSUPPORTED,
            "QoderWork CN Windows packages are x86_64 only.",
            now,
        );
    }
    if facts.os == "windows" && agent_id == AgentId::TraeWork && facts.arch != "x86_64" {
        return fail(
            PF_ARCH_UNSUPPORTED,
            "TRAE Work Windows packages are x64.",
            now,
        );
    }
    pass(
        PF_ARCH_UNSUPPORTED,
        "Architecture is in the published matrix.",
        now,
    )
}

fn unverified(code: &str, message: &str, now: &str) -> PreflightItem {
    PreflightItem {
        code: code.to_owned(),
        state: PreflightItemState::Unknown,
        message: message.to_owned(),
        hint: "Treat unknown as not pass.".to_owned(),
        checked_at: now.to_owned(),
        source: "preflight".to_owned(),
    }
}

fn pass(code: &str, message: &str, now: &str) -> PreflightItem {
    PreflightItem {
        code: code.to_owned(),
        state: PreflightItemState::Pass,
        message: message.to_owned(),
        hint: String::new(),
        checked_at: now.to_owned(),
        source: "preflight".to_owned(),
    }
}

fn fail(code: &str, message: &str, now: &str) -> PreflightItem {
    PreflightItem {
        code: code.to_owned(),
        state: PreflightItemState::Fail,
        message: message.to_owned(),
        hint: "Fix the environment before installing.".to_owned(),
        checked_at: now.to_owned(),
        source: "preflight".to_owned(),
    }
}

pub fn stale_green_is_unknown(age_secs: i64, ttl_secs: i64) -> bool {
    age_secs > ttl_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows_arm() -> MachineFacts {
        MachineFacts {
            os: "windows".to_owned(),
            arch: "aarch64".to_owned(),
            macos_major: None,
            windows_build: Some(19045),
        }
    }

    fn windows_x64() -> MachineFacts {
        MachineFacts {
            os: "windows".to_owned(),
            arch: "x86_64".to_owned(),
            macos_major: None,
            windows_build: Some(19045),
        }
    }

    #[test]
    fn unknown_requirement_is_not_pass() {
        let layer = observe_preflight(AgentId::CodexCli, &windows_x64(), "t0");
        let unverified = layer
            .checks
            .iter()
            .find(|item| item.code == PF_REQUIREMENT_UNVERIFIED)
            .expect("unverified row");
        assert_eq!(unverified.state, PreflightItemState::Unknown);
        assert_ne!(layer.preflight_state, LayerState::Ok);
    }

    #[test]
    fn stale_green_preflight_is_unknown() {
        assert!(stale_green_is_unknown(OS_TTL_SECS + 1, OS_TTL_SECS));
        assert!(!stale_green_is_unknown(10, OS_TTL_SECS));
    }

    #[test]
    fn os_unsupported_fails() {
        let linux = MachineFacts {
            os: "linux".to_owned(),
            arch: "x86_64".to_owned(),
            macos_major: None,
            windows_build: None,
        };
        let layer = observe_preflight(AgentId::CodexCli, &linux, "t0");
        assert_eq!(layer.preflight_state, LayerState::Fail);
        assert!(layer
            .checks
            .iter()
            .any(|item| item.code == PF_OS_UNSUPPORTED && item.state == PreflightItemState::Fail));
    }

    #[test]
    fn arch_mismatch_fails() {
        let layer = observe_preflight(AgentId::QoderworkCn, &windows_arm(), "t0");
        assert!(layer.checks.iter().any(|item| {
            item.code == PF_ARCH_UNSUPPORTED && item.state == PreflightItemState::Fail
        }));
    }

    #[test]
    fn auth_required_is_warn_not_install_failure() {
        let layer = observe_preflight(AgentId::ClaudeCode, &windows_x64(), "t0");
        let auth = layer
            .checks
            .iter()
            .find(|item| item.code == PF_AUTH_REQUIRED)
            .expect("auth row");
        assert_eq!(auth.state, PreflightItemState::Warn);
    }

    #[test]
    fn refresh_preflight_updates_checked_at() {
        let first = observe_preflight(AgentId::CodexCli, &windows_x64(), "t0");
        let second = observe_preflight(AgentId::CodexCli, &windows_x64(), "t1");
        assert_ne!(first.checked_at, second.checked_at);
    }
}
