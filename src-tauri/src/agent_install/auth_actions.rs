//! Closed Agent-auth adapters.
//!
//! This owner executes only reviewed CLI actions and parses only bounded,
//! official status surfaces. Raw output, backing credential paths, account
//! identifiers and secrets never cross the Agent-auth DTO boundary.

use std::{collections::HashSet, path::Path, time::Duration};

use chrono::{SecondsFormat, Utc};

use super::{
    cli::{CLAUDE_TOOL_ID, GROK_TOOL_ID},
    types::{
        AgentAuthAccountState, AgentAuthAuthority, AgentAuthIntent, AgentAuthManagedDestination,
        AgentAuthObservationDto, AgentAuthOwnership, AgentAuthProviderConnectionState,
        AgentAuthProviderSummaryDto, AgentAuthReasonCode, AgentAuthState,
        AGENT_AUTH_CONTRACT_VERSION,
    },
};
use crate::services::{
    external_agents::AgentCatalogId,
    tooling::{launch_terminal_running, run_detected_tool_command_with_timeout_and_output_limit},
};

const AUTH_COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const AUTH_PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const AUTH_OUTPUT_LIMIT: usize = 64 * 1024;
const AUTH_PROBE_OUTPUT_LIMIT: usize = 8 * 1024;
const MAX_CLAUDE_STATUS_FIELDS: usize = 16;
const MAX_STATUS_STRING_CHARS: usize = 512;

const CLAUDE_STATUS_FIELDS: &[&str] = &[
    "loggedIn",
    "authMethod",
    "apiProvider",
    "email",
    "orgId",
    "orgName",
    "subscriptionType",
];

const SENSITIVE_JSON_KEYS: &[&str] = &[
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "authorization",
    "password",
    "secret",
    "token",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthLaunchDisposition {
    AwaitingVerification,
    HandoffComplete,
}

pub async fn observe_agent_auth(agent_id: AgentCatalogId) -> AgentAuthObservationDto {
    match agent_id {
        AgentCatalogId::Codex => fyagent_managed_observation(),
        AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy => {
            handoff_only_observation(agent_id, vec![AgentAuthIntent::Login])
        }
        AgentCatalogId::GrokBuild => {
            let available = tokio::task::spawn_blocking(|| ensure_tool_available(GROK_TOOL_ID))
                .await
                .ok()
                .and_then(Result::ok)
                .is_some();
            if available {
                handoff_only_observation(
                    agent_id,
                    vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
                )
            } else {
                unavailable_observation(
                    agent_id,
                    AgentAuthOwnership::AgentOwned,
                    AgentAuthReasonCode::AuthObserverUnavailable,
                )
            }
        }
        AgentCatalogId::ClaudeCode => tokio::task::spawn_blocking(observe_claude_account)
            .await
            .unwrap_or_else(|_| {
                unavailable_observation(
                    AgentCatalogId::ClaudeCode,
                    AgentAuthOwnership::AgentOwned,
                    AgentAuthReasonCode::AuthObserverUnavailable,
                )
            }),
        AgentCatalogId::OpenCode => tokio::task::spawn_blocking(observe_opencode_providers)
            .await
            .unwrap_or_else(|_| {
                unavailable_observation(
                    AgentCatalogId::OpenCode,
                    AgentAuthOwnership::ProviderOwned,
                    AgentAuthReasonCode::AuthObserverUnavailable,
                )
            }),
    }
}

/// Compatibility projection retained for the install-readiness DTO. New UI
/// and actions consume [`observe_agent_auth`] and the Auth-session port.
pub fn observe_auth_state(
    agent_id: AgentCatalogId,
    cli_detected: bool,
    cli_unavailable: bool,
) -> AgentAuthState {
    if cli_unavailable && agent_id != AgentCatalogId::OpenCode {
        return AgentAuthState::Unavailable;
    }
    match agent_id {
        AgentCatalogId::ClaudeCode if cli_detected => {
            legacy_state_from_observation(&observe_claude_account())
        }
        AgentCatalogId::OpenCode => AgentAuthState::ProviderConnectionRequired,
        AgentCatalogId::ClaudeCode
        | AgentCatalogId::GrokBuild
        | AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy
        | AgentCatalogId::Codex => AgentAuthState::Unknown,
    }
}

pub(super) fn launch_auth_action(
    agent_id: AgentCatalogId,
    intent: AgentAuthIntent,
) -> Result<AuthLaunchDisposition, AgentAuthReasonCode> {
    match (agent_id, intent) {
        (AgentCatalogId::ClaudeCode, AgentAuthIntent::Login) => {
            launch_closed_cli(CLAUDE_TOOL_ID, "claude auth login", "claude_auth_login")?;
            Ok(AuthLaunchDisposition::AwaitingVerification)
        }
        (AgentCatalogId::ClaudeCode, AgentAuthIntent::Logout) => {
            run_closed_cli(CLAUDE_TOOL_ID, &["auth", "logout"])?;
            Ok(AuthLaunchDisposition::AwaitingVerification)
        }
        (AgentCatalogId::OpenCode, AgentAuthIntent::ConnectProvider) => {
            Err(AgentAuthReasonCode::TargetSelectionRequired)
        }
        (AgentCatalogId::OpenCode, AgentAuthIntent::Logout) => {
            Err(AgentAuthReasonCode::ProviderSelectionRequired)
        }
        (AgentCatalogId::GrokBuild, AgentAuthIntent::Login) => {
            launch_closed_cli(GROK_TOOL_ID, "grok login", "grok_login")?;
            Ok(AuthLaunchDisposition::HandoffComplete)
        }
        (AgentCatalogId::GrokBuild, AgentAuthIntent::Logout) => {
            run_closed_cli(GROK_TOOL_ID, &["logout"])?;
            Ok(AuthLaunchDisposition::HandoffComplete)
        }
        _ => Err(AgentAuthReasonCode::ExecutorNotImplemented),
    }
}

pub(super) fn account_state(
    observation: &AgentAuthObservationDto,
) -> Option<AgentAuthAccountState> {
    match observation {
        AgentAuthObservationDto::Account { state, .. } => Some(*state),
        _ => None,
    }
}

pub(super) fn provider_ids(observation: &AgentAuthObservationDto) -> Option<HashSet<String>> {
    match observation {
        AgentAuthObservationDto::ProviderConnections {
            authority: AgentAuthAuthority::Verified,
            providers,
            ..
        } => Some(
            providers
                .iter()
                .map(|provider| provider.provider_id.clone())
                .collect(),
        ),
        _ => None,
    }
}

fn observe_claude_account() -> AgentAuthObservationDto {
    let output = match run_bounded(CLAUDE_TOOL_ID, &["auth", "status"]) {
        Ok(output) => output,
        Err(reason) => {
            return unavailable_observation(
                AgentCatalogId::ClaudeCode,
                AgentAuthOwnership::AgentOwned,
                reason,
            )
        }
    };
    match parse_claude_status_output(output.status.code(), &output.stdout) {
        Some(state) => account_observation(
            AgentCatalogId::ClaudeCode,
            AgentAuthAuthority::Verified,
            state,
            Vec::new(),
        ),
        None => account_observation(
            AgentCatalogId::ClaudeCode,
            AgentAuthAuthority::Unverified,
            AgentAuthAccountState::Unknown,
            vec![AgentAuthReasonCode::AuthOutputInvalid],
        ),
    }
}

fn observe_opencode_providers() -> AgentAuthObservationDto {
    observe_opencode_auth_json(&crate::opencode_config::get_opencode_auth_json_path())
}

fn observe_opencode_auth_json(path: &Path) -> AgentAuthObservationDto {
    let observed = crate::services::managed_auth::consumers::opencode::observe_auth_store(path);
    if !observed.readable {
        return provider_observation(
            AgentAuthAuthority::Unverified,
            AgentAuthProviderConnectionState::Unknown,
            Vec::new(),
            vec![AgentAuthReasonCode::AuthOutputInvalid],
        );
    }
    let providers =
        crate::services::managed_auth::consumers::opencode::agent_auth_providers(&observed)
            .into_iter()
            .map(|(provider_id, label)| AgentAuthProviderSummaryDto { provider_id, label })
            .collect::<Vec<_>>();
    provider_observation(
        AgentAuthAuthority::Verified,
        if providers.is_empty() {
            AgentAuthProviderConnectionState::Empty
        } else {
            AgentAuthProviderConnectionState::Configured
        },
        providers,
        Vec::new(),
    )
}

fn parse_claude_status_output(
    exit_code: Option<i32>,
    stdout: &[u8],
) -> Option<AgentAuthAccountState> {
    if stdout.len() > AUTH_OUTPUT_LIMIT {
        return None;
    }
    let text = std::str::from_utf8(stdout).ok()?.trim();
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let object = value.as_object()?;
    if object.is_empty() || object.len() > MAX_CLAUDE_STATUS_FIELDS {
        return None;
    }
    for (key, value) in object {
        if !CLAUDE_STATUS_FIELDS.contains(&key.as_str())
            || SENSITIVE_JSON_KEYS
                .iter()
                .any(|sensitive| key.eq_ignore_ascii_case(sensitive))
        {
            return None;
        }
        match value {
            serde_json::Value::Null | serde_json::Value::Bool(_) => {}
            serde_json::Value::String(value)
                if value.chars().count() <= MAX_STATUS_STRING_CHARS
                    && !value.chars().any(char::is_control) => {}
            _ => return None,
        }
    }
    let logged_in = object.get("loggedIn")?.as_bool()?;
    match (exit_code, logged_in) {
        (Some(0), true) => Some(AgentAuthAccountState::LoggedIn),
        (Some(1), false) => Some(AgentAuthAccountState::LoggedOut),
        _ => None,
    }
}

fn run_bounded(tool: &str, args: &[&str]) -> Result<std::process::Output, AgentAuthReasonCode> {
    let cwd = crate::config::get_home_dir();
    run_detected_tool_command_with_timeout_and_output_limit(
        tool,
        args,
        Some(AUTH_COMMAND_TIMEOUT),
        AUTH_OUTPUT_LIMIT,
        &[],
        &cwd,
    )
    .map_err(map_tooling_error)
}

fn ensure_tool_available(tool: &str) -> Result<(), AgentAuthReasonCode> {
    let cwd = crate::config::get_home_dir();
    run_detected_tool_command_with_timeout_and_output_limit(
        tool,
        &["--version"],
        Some(AUTH_PROBE_TIMEOUT),
        AUTH_PROBE_OUTPUT_LIMIT,
        &[],
        &cwd,
    )
    .map(|_| ())
    .map_err(map_tooling_error)
}

fn launch_closed_cli(
    tool: &str,
    command: &'static str,
    label: &'static str,
) -> Result<(), AgentAuthReasonCode> {
    ensure_tool_available(tool)?;
    launch_terminal_running(command, label)
        .map_err(|_| AgentAuthReasonCode::InteractiveUserUnavailable)
}

fn run_closed_cli(tool: &str, args: &[&str]) -> Result<(), AgentAuthReasonCode> {
    let output = run_bounded(tool, args)?;
    output
        .status
        .success()
        .then_some(())
        .ok_or(AgentAuthReasonCode::CommandFailed)
}

fn map_tooling_error(error: String) -> AgentAuthReasonCode {
    let lower = error.to_ascii_lowercase();
    if lower.contains("elevated windows") || lower.contains("interactive") {
        AgentAuthReasonCode::InteractiveUserUnavailable
    } else if lower.contains("not installed") || lower.contains("unavailable") {
        AgentAuthReasonCode::AuthObserverUnavailable
    } else if lower.contains("output exceeded") {
        AgentAuthReasonCode::AuthOutputInvalid
    } else {
        AgentAuthReasonCode::CommandFailed
    }
}

fn checked_at() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn account_observation(
    agent_id: AgentCatalogId,
    authority: AgentAuthAuthority,
    state: AgentAuthAccountState,
    reason_codes: Vec<AgentAuthReasonCode>,
) -> AgentAuthObservationDto {
    let allowed_intents = match state {
        AgentAuthAccountState::LoggedIn => vec![AgentAuthIntent::Logout],
        AgentAuthAccountState::LoggedOut => vec![AgentAuthIntent::Login],
        AgentAuthAccountState::Unknown => vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
    };
    AgentAuthObservationDto::Account {
        contract_version: AGENT_AUTH_CONTRACT_VERSION,
        agent_id,
        ownership: AgentAuthOwnership::AgentOwned,
        authority,
        state,
        allowed_intents,
        checked_at: checked_at(),
        reason_codes,
    }
}

fn provider_observation(
    authority: AgentAuthAuthority,
    state: AgentAuthProviderConnectionState,
    providers: Vec<AgentAuthProviderSummaryDto>,
    reason_codes: Vec<AgentAuthReasonCode>,
) -> AgentAuthObservationDto {
    let mut allowed_intents = vec![AgentAuthIntent::ConnectProvider];
    if !providers.is_empty() {
        allowed_intents.push(AgentAuthIntent::Logout);
    }
    AgentAuthObservationDto::ProviderConnections {
        contract_version: AGENT_AUTH_CONTRACT_VERSION,
        agent_id: AgentCatalogId::OpenCode,
        ownership: AgentAuthOwnership::ProviderOwned,
        authority,
        state,
        providers,
        allowed_intents,
        checked_at: checked_at(),
        reason_codes,
    }
}

fn handoff_only_observation(
    agent_id: AgentCatalogId,
    allowed_intents: Vec<AgentAuthIntent>,
) -> AgentAuthObservationDto {
    AgentAuthObservationDto::HandoffOnly {
        contract_version: AGENT_AUTH_CONTRACT_VERSION,
        agent_id,
        ownership: AgentAuthOwnership::AgentOwned,
        authority: AgentAuthAuthority::Unverified,
        allowed_intents,
        checked_at: checked_at(),
        reason_codes: vec![AgentAuthReasonCode::HandoffOnly],
    }
}

fn fyagent_managed_observation() -> AgentAuthObservationDto {
    AgentAuthObservationDto::FyagentManaged {
        contract_version: AGENT_AUTH_CONTRACT_VERSION,
        agent_id: AgentCatalogId::Codex,
        ownership: AgentAuthOwnership::FyagentManaged,
        authority: AgentAuthAuthority::Verified,
        destination: AgentAuthManagedDestination::AuthCenter,
        allowed_intents: Vec::new(),
        checked_at: checked_at(),
        reason_codes: vec![AgentAuthReasonCode::ManagedByAuthCenter],
    }
}

fn unavailable_observation(
    agent_id: AgentCatalogId,
    ownership: AgentAuthOwnership,
    reason_code: AgentAuthReasonCode,
) -> AgentAuthObservationDto {
    AgentAuthObservationDto::Unavailable {
        contract_version: AGENT_AUTH_CONTRACT_VERSION,
        agent_id,
        ownership,
        authority: AgentAuthAuthority::Unavailable,
        allowed_intents: Vec::new(),
        checked_at: checked_at(),
        reason_codes: vec![reason_code],
    }
}

fn legacy_state_from_observation(observation: &AgentAuthObservationDto) -> AgentAuthState {
    match observation {
        AgentAuthObservationDto::Account {
            state: AgentAuthAccountState::LoggedIn,
            ..
        } => AgentAuthState::LoggedIn,
        AgentAuthObservationDto::Account {
            state: AgentAuthAccountState::LoggedOut,
            ..
        } => AgentAuthState::LoggedOut,
        AgentAuthObservationDto::Unavailable { .. } => AgentAuthState::Unavailable,
        _ => AgentAuthState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_status_requires_matching_exit_and_allowlisted_json() {
        assert_eq!(
            parse_claude_status_output(
                Some(0),
                br#"{"loggedIn":true,"authMethod":"oauth_token","apiProvider":"firstParty"}"#,
            ),
            Some(AgentAuthAccountState::LoggedIn)
        );
        assert_eq!(
            parse_claude_status_output(
                Some(1),
                br#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#,
            ),
            Some(AgentAuthAccountState::LoggedOut)
        );
        assert_eq!(
            parse_claude_status_output(Some(0), br#"{"loggedIn":false}"#),
            None
        );
        assert_eq!(
            parse_claude_status_output(Some(0), br#"{"loggedIn":true,"access_token":"secret"}"#,),
            None
        );
        assert_eq!(
            parse_claude_status_output(Some(0), br#"{"loggedIn":true,"unexpected":"value"}"#,),
            None
        );
    }

    #[test]
    fn opencode_auth_json_observation_does_not_require_cli_and_hides_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "openai": {
                    "type": "oauth",
                    "refresh": "rt-secret",
                    "access": "at-secret",
                    "expires": 9
                },
                "anthropic": {
                    "type": "api",
                    "key": "sk-secret"
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let observation = observe_opencode_auth_json(&path);
        let encoded = serde_json::to_string(&observation).unwrap();
        match observation {
            AgentAuthObservationDto::ProviderConnections {
                authority,
                state,
                providers,
                reason_codes,
                ..
            } => {
                assert_eq!(authority, AgentAuthAuthority::Verified);
                assert_eq!(state, AgentAuthProviderConnectionState::Configured);
                assert!(reason_codes.is_empty());
                let labels: Vec<_> = providers
                    .iter()
                    .map(|provider| provider.label.as_str())
                    .collect();
                assert!(labels.contains(&"OpenAI"));
                assert!(labels.contains(&"anthropic"));
                assert!(providers.iter().all(|provider| {
                    super::super::types::validate_opaque_auth_provider_id(&provider.provider_id)
                }));
            }
            other => panic!("expected provider connections, got {other:?}"),
        }
        assert!(!encoded.contains("rt-secret"));
        assert!(!encoded.contains("sk-secret"));
        assert!(!encoded.contains("auth.json"));
        assert!(!encoded.to_ascii_lowercase().contains("access_token"));
    }

    #[test]
    fn opencode_missing_auth_json_is_empty_not_observer_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let observation = observe_opencode_auth_json(&dir.path().join("auth.json"));
        match observation {
            AgentAuthObservationDto::ProviderConnections {
                authority,
                state,
                providers,
                ..
            } => {
                assert_eq!(authority, AgentAuthAuthority::Verified);
                assert_eq!(state, AgentAuthProviderConnectionState::Empty);
                assert!(providers.is_empty());
            }
            other => panic!("expected empty providers, got {other:?}"),
        }
        assert_eq!(
            observe_auth_state(AgentCatalogId::OpenCode, false, false),
            AgentAuthState::ProviderConnectionRequired
        );
        assert_eq!(
            observe_auth_state(AgentCatalogId::OpenCode, false, true),
            AgentAuthState::ProviderConnectionRequired
        );
    }

    #[test]
    fn opencode_remains_provider_owned_not_a_global_login_bool() {
        let observation = provider_observation(
            AgentAuthAuthority::Verified,
            AgentAuthProviderConnectionState::Configured,
            vec![AgentAuthProviderSummaryDto {
                provider_id: crate::services::managed_auth::consumers::opencode::capability_id(
                    "openai",
                ),
                label: "OpenAI".into(),
            }],
            Vec::new(),
        );
        assert_eq!(account_state(&observation), None);
        assert_eq!(provider_ids(&observation).unwrap().len(), 1);
        assert_eq!(
            legacy_state_from_observation(&observation),
            AgentAuthState::Unknown
        );
    }
}
