//! Shared DTOs for the four-layer agent install contract.

use serde::{Deserialize, Serialize};

use super::error::AgentInstallError;
use super::integrity::IntegrityLayerState;
use super::plan::PlanLayerState;
use super::preflight::PreflightLayerState;
use super::source::SourceLayerState;

pub const CONTRACT_SCHEMA: &str = "fyagent-agent-install-contract-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    QoderworkCn,
    DingtalkWukong,
    Workbuddy,
    TraeWork,
    CodexCli,
    ClaudeCode,
}

impl AgentId {
    pub const ALL: [Self; 6] = [
        Self::QoderworkCn,
        Self::DingtalkWukong,
        Self::Workbuddy,
        Self::TraeWork,
        Self::CodexCli,
        Self::ClaudeCode,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QoderworkCn => "qoderwork-cn",
            Self::DingtalkWukong => "dingtalk-wukong",
            Self::Workbuddy => "workbuddy",
            Self::TraeWork => "trae-work",
            Self::CodexCli => "codex-cli",
            Self::ClaudeCode => "claude-code",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AgentInstallError> {
        match value {
            "qoderwork-cn" => Ok(Self::QoderworkCn),
            "dingtalk-wukong" => Ok(Self::DingtalkWukong),
            "workbuddy" => Ok(Self::Workbuddy),
            "trae-work" => Ok(Self::TraeWork),
            "codex-cli" => Ok(Self::CodexCli),
            "claude-code" => Ok(Self::ClaudeCode),
            _ => Err(AgentInstallError::unknown_agent()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerState {
    Ok,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    OfficialGuide,
    PackageManager,
    NativeVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSourceKind {
    Documented,
    Dynamic,
    PackageManager,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseScope {
    PublicOpenSource,
    SourceAvailable,
    RestrictedNonCommercial,
    EnterpriseOnly,
    NeedsVendorConfirmation,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallContract {
    pub schema: String,
    pub agent_id: AgentId,
    pub catalog: SourceLayerState,
    pub package: IntegrityLayerState,
    pub environment: PreflightLayerState,
    pub plan: PlanLayerState,
    pub updated_at: String,
    pub install_allowed: bool,
    pub guide_allowed: bool,
}

impl InstallContract {
    pub fn new(
        agent_id: AgentId,
        catalog: SourceLayerState,
        package: IntegrityLayerState,
        environment: PreflightLayerState,
        plan: PlanLayerState,
        updated_at: String,
        install_allowed: bool,
        guide_allowed: bool,
    ) -> Self {
        Self {
            schema: CONTRACT_SCHEMA.to_owned(),
            agent_id,
            catalog,
            package,
            environment,
            plan,
            updated_at,
            install_allowed,
            guide_allowed,
        }
    }
}
