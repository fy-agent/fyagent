//! Agent-owned and provider-owned auth actions. Credentials are never read
//! from vendor files.

use super::cli::{CLAUDE_TOOL_ID, GROK_TOOL_ID, OPENCODE_TOOL_ID};
use super::types::{AgentActionId, AgentAuthState, AgentReasonCode};
use crate::services::external_agents::AgentCatalogId;
use crate::services::tooling::{launch_terminal_running, run_detected_tool_command_with_timeout};

const AUTH_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

pub fn observe_auth_state(
    agent_id: AgentCatalogId,
    cli_detected: bool,
    cli_unavailable: bool,
) -> AgentAuthState {
    if cli_unavailable {
        return AgentAuthState::Unavailable;
    }
    match agent_id {
        AgentCatalogId::ClaudeCode if cli_detected => {
            claude_auth_status().unwrap_or(AgentAuthState::Unknown)
        }
        AgentCatalogId::ClaudeCode
        | AgentCatalogId::GrokBuild
        | AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy => AgentAuthState::Unknown,
        AgentCatalogId::OpenCode if cli_detected => AgentAuthState::ProviderConnectionRequired,
        AgentCatalogId::OpenCode => AgentAuthState::Unknown,
        AgentCatalogId::Codex => AgentAuthState::Unknown,
    }
}

pub fn start_auth_action(
    agent_id: AgentCatalogId,
    action: AgentActionId,
) -> Result<(), AgentReasonCode> {
    match (agent_id, action) {
        (AgentCatalogId::ClaudeCode, AgentActionId::AuthLogin) => {
            launch_closed_cli(CLAUDE_TOOL_ID, "claude auth login", "claude_auth_login")
        }
        (AgentCatalogId::ClaudeCode, AgentActionId::AuthLogout) => {
            run_closed_cli(CLAUDE_TOOL_ID, &["auth", "logout"])
        }
        (AgentCatalogId::GrokBuild, AgentActionId::AuthLogin) => {
            launch_closed_cli(GROK_TOOL_ID, "grok login", "grok_login")
        }
        (AgentCatalogId::GrokBuild, AgentActionId::AuthLogout) => {
            launch_closed_cli(GROK_TOOL_ID, "grok logout", "grok_logout")
        }
        (AgentCatalogId::OpenCode, AgentActionId::AuthConnectProvider) => {
            launch_closed_cli(OPENCODE_TOOL_ID, "opencode /connect", "opencode_connect")
        }
        (
            AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy,
            AgentActionId::AuthLogin,
        ) => Err(AgentReasonCode::AuthStateUnknown),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

fn launch_closed_cli(
    tool: &str,
    command: &'static str,
    label: &'static str,
) -> Result<(), AgentReasonCode> {
    tooling_id_for_tool(tool)?;
    launch_terminal_running(command, label).map_err(|_| AgentReasonCode::InteractiveUserUnavailable)
}

fn run_closed_cli(tool: &str, args: &[&str]) -> Result<(), AgentReasonCode> {
    tooling_id_for_tool(tool)?;
    let cwd = crate::config::get_home_dir();
    run_detected_tool_command_with_timeout(tool, args, Some(AUTH_COMMAND_TIMEOUT), &[], &cwd)
        .map(|_| ())
        .map_err(|error| {
            if error.contains("elevated Windows") || error.contains("unavailable") {
                AgentReasonCode::InteractiveUserUnavailable
            } else {
                AgentReasonCode::AuthStateUnknown
            }
        })
}

fn tooling_id_for_tool(tool: &str) -> Result<(), AgentReasonCode> {
    match tool {
        "claude" | "grok" | "opencode" => Ok(()),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

fn claude_auth_status() -> Option<AgentAuthState> {
    let cwd = crate::config::get_home_dir();
    let output = run_detected_tool_command_with_timeout(
        CLAUDE_TOOL_ID,
        &["auth", "status"],
        Some(AUTH_COMMAND_TIMEOUT),
        &[],
        &cwd,
    )
    .ok()?;
    if !output.status.success() {
        return Some(AgentAuthState::LoggedOut);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        if json_contains_secret(&value) {
            return Some(AgentAuthState::Unknown);
        }
        let logged_in = value
            .get("loggedIn")
            .or_else(|| value.get("logged_in"))
            .or_else(|| value.get("authenticated"))
            .and_then(|item| item.as_bool());
        return Some(match logged_in {
            Some(true) => AgentAuthState::LoggedIn,
            Some(false) => AgentAuthState::LoggedOut,
            None => AgentAuthState::Unknown,
        });
    }
    Some(AgentAuthState::Unknown)
}

fn json_contains_secret(value: &serde_json::Value) -> bool {
    let serialized = value.to_string().to_ascii_lowercase();
    [
        "access_token",
        "refresh_token",
        "api_key",
        "authorization",
        "sk-",
        "\"token\"",
    ]
    .iter()
    .any(|needle| serialized.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_is_provider_owned_not_a_global_login_bool() {
        assert_eq!(
            observe_auth_state(AgentCatalogId::OpenCode, true, false),
            AgentAuthState::ProviderConnectionRequired
        );
        assert_eq!(
            observe_auth_state(AgentCatalogId::OpenCode, false, false),
            AgentAuthState::Unknown
        );
        assert_eq!(
            observe_auth_state(AgentCatalogId::GrokBuild, true, false),
            AgentAuthState::Unknown
        );
    }

    #[test]
    fn secret_bearing_status_json_is_ignored() {
        let value = serde_json::json!({"loggedIn": true, "access_token": "secret"});
        assert!(json_contains_secret(&value));
        let clean = serde_json::json!({"loggedIn": true});
        assert!(!json_contains_secret(&clean));
    }

    #[test]
    fn opencode_rejects_global_login_and_logout_actions() {
        assert_eq!(
            start_auth_action(AgentCatalogId::OpenCode, AgentActionId::AuthLogin),
            Err(AgentReasonCode::ExecutorNotImplemented)
        );
        assert_eq!(
            start_auth_action(AgentCatalogId::OpenCode, AgentActionId::AuthLogout),
            Err(AgentReasonCode::ExecutorNotImplemented)
        );
    }
}
