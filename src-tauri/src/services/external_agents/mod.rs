//! Evidence-bounded runtime boundary for catalog-owned external agents.
//!
//! The renderer may select only a closed agent ID and destination. This module
//! deliberately contains no path, executable, process-name, URL, or command
//! input. A future positive adapter must bring a separately reviewed trusted
//! application identity; product names and configuration directories are not
//! sufficient evidence for detection or launch.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AgentCatalogId {
    #[serde(rename = "qoderwork")]
    QoderWork,
    #[serde(rename = "trae-work")]
    TraeWork,
    #[serde(rename = "workbuddy")]
    WorkBuddy,
    #[serde(rename = "grokbuild")]
    GrokBuild,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "opencode")]
    OpenCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AgentVariantId {
    #[serde(rename = "qoderwork-cn")]
    QoderWorkCn,
    #[serde(rename = "trae-work-cn")]
    TraeWorkCn,
    #[serde(rename = "workbuddy")]
    WorkBuddy,
    #[serde(rename = "grokbuild")]
    GrokBuild,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "opencode")]
    OpenCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AgentCapabilityId {
    #[serde(rename = "product.open")]
    ProductOpen,
    #[serde(rename = "app.detect")]
    AppDetect,
    #[serde(rename = "app.launch")]
    AppLaunch,
    #[serde(rename = "skills.read")]
    SkillsRead,
    #[serde(rename = "skills.write")]
    SkillsWrite,
    #[serde(rename = "hooks.read")]
    HooksRead,
    #[serde(rename = "hooks.write")]
    HooksWrite,
    #[serde(rename = "models.validate")]
    ModelsValidate,
    #[serde(rename = "models.write")]
    ModelsWrite,
    #[serde(rename = "mcp.validate")]
    McpValidate,
    #[serde(rename = "mcp.write")]
    McpWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityMode {
    Direct,
    Assisted,
    Unsupported,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityReasonCode {
    OfficialLinkReviewed,
    TrustedRuntimeIdentityUnavailable,
    DedicatedAgentFlow,
    FyagentSkillSynchronization,
    FyagentHookManagement,
    FyagentModelValidation,
    FyagentMcpValidation,
    VendorUiRequired,
    VendorPrivateStorageUnsupported,
    DedicatedNativeContract,
    CapabilityNotApplicable,
    NoCatalogProductLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvidenceId {
    QoderworkProduct,
    QoderworkInstall,
    QoderworkSkills,
    QoderworkHooks,
    QoderworkHooksNativeContract,
    QoderworkConnectors,
    TraeWorkProduct,
    TraeWorkSkills,
    TraeWorkModels,
    TraeWorkModelValidationContract,
    TraeWorkMcp,
    ExternalMcpValidationContract,
    WorkbuddyNativeContract,
    CodexDesktopInstallerContract,
    ProviderQuickSetupContract,
    SkillServiceContract,
    McpServiceContract,
    ClaudeOfficialLinks,
    OpencodeProduct,
    OpencodeModels,
    GrokbuildProduct,
    P0Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAgentLaunchDestination {
    Home,
    Skills,
    Hooks,
    Models,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAgentRuntimeCapabilityState {
    Available,
    Assisted,
    Unavailable,
    Unverified,
    BlockedByVersion,
    ProbeFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAgentInstallSource {
    ManagedInstaller,
    OfficialInstaller,
    SystemPackage,
    UserInstallation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentRuntimeCapability {
    pub id: AgentCapabilityId,
    pub state: ExternalAgentRuntimeCapabilityState,
    pub reason_code: AgentCapabilityReasonCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentRuntimeStatus {
    pub agent_id: AgentCatalogId,
    pub detected: Option<bool>,
    pub running: Option<bool>,
    pub version: Option<String>,
    pub install_source: Option<ExternalAgentInstallSource>,
    pub capabilities: Vec<ExternalAgentRuntimeCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentLaunchResult {
    pub agent_id: AgentCatalogId,
    pub destination: ExternalAgentLaunchDestination,
    pub state: ExternalAgentRuntimeCapabilityState,
    pub reason_code: AgentCapabilityReasonCode,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ExternalAgentRuntimeService;

impl ExternalAgentRuntimeService {
    pub fn get_status(agent_id: AgentCatalogId) -> ExternalAgentRuntimeStatus {
        let (state, reason_code) = runtime_boundary(agent_id);
        ExternalAgentRuntimeStatus {
            agent_id,
            detected: None,
            running: None,
            version: None,
            install_source: None,
            capabilities: vec![
                ExternalAgentRuntimeCapability {
                    id: AgentCapabilityId::AppDetect,
                    state,
                    reason_code,
                },
                ExternalAgentRuntimeCapability {
                    id: AgentCapabilityId::AppLaunch,
                    state,
                    reason_code,
                },
            ],
        }
    }

    pub fn launch(
        agent_id: AgentCatalogId,
        destination: ExternalAgentLaunchDestination,
    ) -> ExternalAgentLaunchResult {
        let (state, reason_code) = runtime_boundary(agent_id);
        ExternalAgentLaunchResult {
            agent_id,
            destination,
            state,
            reason_code,
        }
    }
}

const fn runtime_boundary(
    agent_id: AgentCatalogId,
) -> (
    ExternalAgentRuntimeCapabilityState,
    AgentCapabilityReasonCode,
) {
    match agent_id {
        AgentCatalogId::QoderWork
        | AgentCatalogId::TraeWork
        | AgentCatalogId::WorkBuddy
        | AgentCatalogId::GrokBuild
        | AgentCatalogId::ClaudeCode
        | AgentCatalogId::OpenCode => (
            ExternalAgentRuntimeCapabilityState::Unverified,
            AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        ),
        AgentCatalogId::Codex => (
            ExternalAgentRuntimeCapabilityState::Unavailable,
            AgentCapabilityReasonCode::DedicatedAgentFlow,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_catalog_runtime_observation_stays_unknown_without_identity_evidence() {
        for agent_id in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
            AgentCatalogId::GrokBuild,
            AgentCatalogId::ClaudeCode,
            AgentCatalogId::OpenCode,
        ] {
            let status = ExternalAgentRuntimeService::get_status(agent_id);

            assert_eq!(status.detected, None);
            assert_eq!(status.running, None);
            assert_eq!(status.version, None);
            assert_eq!(status.install_source, None);
            assert_eq!(
                status.capabilities,
                vec![
                    ExternalAgentRuntimeCapability {
                        id: AgentCapabilityId::AppDetect,
                        state: ExternalAgentRuntimeCapabilityState::Unverified,
                        reason_code: AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    },
                    ExternalAgentRuntimeCapability {
                        id: AgentCapabilityId::AppLaunch,
                        state: ExternalAgentRuntimeCapabilityState::Unverified,
                        reason_code: AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    },
                ]
            );
        }
    }

    #[test]
    fn agent_catalog_launch_accepts_only_closed_ids_and_destinations_and_never_claims_success() {
        for agent_id in [AgentCatalogId::QoderWork, AgentCatalogId::TraeWork] {
            for destination in [
                ExternalAgentLaunchDestination::Home,
                ExternalAgentLaunchDestination::Skills,
                ExternalAgentLaunchDestination::Hooks,
                ExternalAgentLaunchDestination::Models,
                ExternalAgentLaunchDestination::Mcp,
            ] {
                let result = ExternalAgentRuntimeService::launch(agent_id, destination);
                assert_eq!(result.agent_id, agent_id);
                assert_eq!(result.destination, destination);
                assert_eq!(
                    result.state,
                    ExternalAgentRuntimeCapabilityState::Unverified
                );
                assert_eq!(
                    result.reason_code,
                    AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable
                );
            }
        }

        for invalid in [
            r#""C:\\Users\\Alice\\QoderWork.exe""#,
            r#""/Applications/TRAE.app""#,
            r#""powershell""#,
            r#""https://example.test""#,
            r#""legacy""#,
        ] {
            assert!(serde_json::from_str::<AgentCatalogId>(invalid).is_err());
            assert!(serde_json::from_str::<ExternalAgentLaunchDestination>(invalid).is_err());
        }
    }

    #[test]
    fn agent_catalog_dedicated_codex_flow_does_not_duplicate_runtime_authority() {
        let status = ExternalAgentRuntimeService::get_status(AgentCatalogId::Codex);
        assert_eq!(status.detected, None);
        assert_eq!(status.running, None);
        assert!(status.capabilities.iter().all(|capability| {
            capability.state == ExternalAgentRuntimeCapabilityState::Unavailable
                && capability.reason_code == AgentCapabilityReasonCode::DedicatedAgentFlow
        }));
    }

    #[test]
    fn agent_catalog_runtime_wire_is_sanitized_and_contains_no_launch_primitive() {
        let status = serde_json::to_value(ExternalAgentRuntimeService::get_status(
            AgentCatalogId::QoderWork,
        ))
        .expect("runtime status serializes");
        let launch = serde_json::to_value(ExternalAgentRuntimeService::launch(
            AgentCatalogId::TraeWork,
            ExternalAgentLaunchDestination::Models,
        ))
        .expect("launch result serializes");

        let mut status_keys = status
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        status_keys.sort_unstable();
        assert_eq!(
            status_keys,
            [
                "agentId",
                "capabilities",
                "detected",
                "installSource",
                "running",
                "version",
            ]
        );
        let mut launch_keys = launch
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        launch_keys.sort_unstable();
        assert_eq!(
            launch_keys,
            ["agentId", "destination", "reasonCode", "state"]
        );

        let serialized = format!("{status}{launch}").to_ascii_lowercase();
        for prohibited in [
            "path",
            "executable",
            "command",
            "arguments",
            "processid",
            "process_id",
            "url",
            "apikey",
            "api_key",
            "access_token",
            "password",
        ] {
            assert!(!serialized.contains(prohibited));
        }
    }
}
