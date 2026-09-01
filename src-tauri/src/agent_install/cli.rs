//! Catalog → Tooling adapter. Do not copy lifecycle command construction.

use crate::services::external_agents::AgentCatalogId;
use crate::services::tooling::{self, ToolVersion};

pub const CLAUDE_TOOL_ID: &str = "claude";
pub const GROK_TOOL_ID: &str = "grok";
pub const OPENCODE_TOOL_ID: &str = "opencode";

pub fn tooling_id_for(agent_id: AgentCatalogId) -> Option<&'static str> {
    match agent_id {
        AgentCatalogId::ClaudeCode => Some(CLAUDE_TOOL_ID),
        AgentCatalogId::GrokBuild => Some(GROK_TOOL_ID),
        AgentCatalogId::OpenCode => Some(OPENCODE_TOOL_ID),
        AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy
        | AgentCatalogId::Codex => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliObservation {
    pub detected: bool,
    pub runnable: bool,
    pub local_version: Option<String>,
    pub latest_version: Option<String>,
    pub unavailable: bool,
}

impl CliObservation {
    pub fn from_tool_version(version: &ToolVersion) -> Self {
        let unavailable = version.error().is_some()
            && version.local_version().is_none()
            && !version.installed_but_broken();
        Self {
            detected: version.is_detected(),
            runnable: version.local_version().is_some() && !version.installed_but_broken(),
            local_version: version.local_version().map(str::to_string),
            latest_version: version.latest_version().map(str::to_string),
            unavailable,
        }
    }
}

pub async fn observe_cli(agent_id: AgentCatalogId) -> Option<CliObservation> {
    let tool = tooling_id_for(agent_id)?;
    let versions = tooling::get_tool_versions(Some(vec![tool.to_string()]))
        .await
        .ok()?;
    versions
        .iter()
        .find(|version| version.name() == tool)
        .map(CliObservation::from_tool_version)
}

pub async fn run_cli_lifecycle(
    agent_id: AgentCatalogId,
    action: super::types::AgentActionId,
) -> Result<(), super::types::AgentReasonCode> {
    let tool =
        tooling_id_for(agent_id).ok_or(super::types::AgentReasonCode::ExecutorNotImplemented)?;
    let lifecycle = match action {
        super::types::AgentActionId::Install => "install",
        super::types::AgentActionId::Update => "update",
        _ => return Err(super::types::AgentReasonCode::ExecutorNotImplemented),
    };
    tooling::run_tool_lifecycle_action(vec![tool.to_string()], lifecycle.to_string())
        .await
        .map_err(|error| {
            if error.contains("elevated Windows")
                || error.contains("unavailable for the current Windows user")
            {
                super::types::AgentReasonCode::InteractiveUserUnavailable
            } else if error.contains("Codex CLI lifecycle")
                || error.contains("only available for Grok Build")
            {
                super::types::AgentReasonCode::ExecutorNotImplemented
            } else {
                super::types::AgentReasonCode::SourceNotVerified
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_maps_only_the_three_cli_agents() {
        assert_eq!(tooling_id_for(AgentCatalogId::ClaudeCode), Some("claude"));
        assert_eq!(tooling_id_for(AgentCatalogId::GrokBuild), Some("grok"));
        assert_eq!(tooling_id_for(AgentCatalogId::OpenCode), Some("opencode"));
        assert_eq!(tooling_id_for(AgentCatalogId::Codex), None);
        assert_eq!(tooling_id_for(AgentCatalogId::QoderWork), None);
        assert_eq!(tooling_id_for(AgentCatalogId::TraeWork), None);
        assert_eq!(tooling_id_for(AgentCatalogId::WorkBuddy), None);
    }

    #[test]
    fn mapping_never_routes_gemini_hermes_or_openclaw() {
        for id in [
            AgentCatalogId::ClaudeCode,
            AgentCatalogId::GrokBuild,
            AgentCatalogId::OpenCode,
        ] {
            let tool = tooling_id_for(id).unwrap();
            assert_ne!(tool, "gemini");
            assert_ne!(tool, "hermes");
            assert_ne!(tool, "openclaw");
            assert_ne!(tool, "codex");
        }
    }
}
