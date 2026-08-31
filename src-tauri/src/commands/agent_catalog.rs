use serde::{Deserialize, Serialize};

use crate::services::external_agents::{
    AgentCapabilityId, AgentCapabilityMode, AgentCapabilityReasonCode, AgentCatalogId,
    AgentEvidenceId, AgentVariantId, ExternalAgentLaunchDestination, ExternalAgentLaunchResult,
    ExternalAgentRuntimeService, ExternalAgentRuntimeStatus,
};

const AGENT_CATALOG_CONTRACT_VERSION: u16 = 5;
const AGENT_CATALOG_REVIEWED_AT: &str = "2026-08-31";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOfficialLinkId {
    Product,
    Cli,
    Desktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentOfficialLink {
    pub id: AgentOfficialLinkId,
    pub label: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredAgentCapability {
    pub id: AgentCapabilityId,
    pub mode: AgentCapabilityMode,
    pub reason_code: AgentCapabilityReasonCode,
    pub evidence_ids: &'static [AgentEvidenceId],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCatalogEntry {
    pub id: AgentCatalogId,
    pub variant_id: AgentVariantId,
    pub display_name: &'static str,
    pub description: &'static str,
    pub official_links: &'static [AgentOfficialLink],
    pub capabilities: &'static [DeclaredAgentCapability],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCatalogResult {
    pub contract_version: u16,
    pub reviewed_at: &'static str,
    pub agents: Vec<AgentCatalogEntry>,
}

const fn official_link(
    id: AgentOfficialLinkId,
    label: &'static str,
    url: &'static str,
) -> AgentOfficialLink {
    AgentOfficialLink { id, label, url }
}

const fn capability(
    id: AgentCapabilityId,
    mode: AgentCapabilityMode,
    reason_code: AgentCapabilityReasonCode,
    evidence_ids: &'static [AgentEvidenceId],
) -> DeclaredAgentCapability {
    DeclaredAgentCapability {
        id,
        mode,
        reason_code,
        evidence_ids,
    }
}

const QODERWORK_OFFICIAL_LINKS: [AgentOfficialLink; 1] = [official_link(
    AgentOfficialLinkId::Product,
    "打开 QoderWork 官方页面",
    "https://qoder.com.cn/qoderwork",
)];

const TRAE_WORK_OFFICIAL_LINKS: [AgentOfficialLink; 1] = [official_link(
    AgentOfficialLinkId::Product,
    "打开 TRAE Work CN 官方页面",
    "https://www.trae.cn/sem-work",
)];

const WORKBUDDY_OFFICIAL_LINKS: [AgentOfficialLink; 1] = [official_link(
    AgentOfficialLinkId::Product,
    "打开 WorkBuddy 官方页面",
    "https://www.workbuddy.cn/",
)];

const GROKBUILD_OFFICIAL_LINKS: [AgentOfficialLink; 1] = [official_link(
    AgentOfficialLinkId::Product,
    "打开 Grok Build 官方页面",
    "https://x.ai/grok",
)];

const CLAUDE_OFFICIAL_LINKS: [AgentOfficialLink; 1] = [official_link(
    AgentOfficialLinkId::Desktop,
    "Claude Desktop",
    "https://claude.com/download",
)];

const OPENCODE_OFFICIAL_LINKS: [AgentOfficialLink; 2] = [
    official_link(
        AgentOfficialLinkId::Product,
        "打开 OpenCode 官方页面",
        "https://opencode.ai",
    ),
    official_link(
        AgentOfficialLinkId::Desktop,
        "打开 OpenCode 官方下载页",
        "https://opencode.ai/download",
    ),
];

const QODER_PRODUCT_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::QoderworkProduct];
const QODER_RUNTIME_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::QoderworkInstall];
const QODER_SKILLS_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::QoderworkSkills,
    AgentEvidenceId::SkillServiceContract,
];
const QODER_HOOKS_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::QoderworkHooks,
    AgentEvidenceId::QoderworkHooksNativeContract,
];
const QODER_MCP_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::QoderworkConnectors,
    AgentEvidenceId::ExternalMcpValidationContract,
];
const TRAE_PRODUCT_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::TraeWorkProduct];
const TRAE_SKILLS_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::TraeWorkSkills,
    AgentEvidenceId::SkillServiceContract,
];
const TRAE_MODELS_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::TraeWorkModels,
    AgentEvidenceId::TraeWorkModelValidationContract,
];
const TRAE_MCP_EVIDENCE: &[AgentEvidenceId] = &[
    AgentEvidenceId::TraeWorkMcp,
    AgentEvidenceId::ExternalMcpValidationContract,
];
const WORKBUDDY_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::WorkbuddyNativeContract];
const CODEX_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::CodexDesktopInstallerContract];
const PROVIDER_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::ProviderQuickSetupContract];
const SKILL_SERVICE_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::SkillServiceContract];
const MCP_SERVICE_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::McpServiceContract];
const CLAUDE_LINK_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::ClaudeOfficialLinks];
const OPENCODE_PRODUCT_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::OpencodeProduct];
const OPENCODE_MODELS_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::OpencodeModels];
const GROKBUILD_PRODUCT_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::GrokbuildProduct];
const P0_SCOPE_EVIDENCE: &[AgentEvidenceId] = &[AgentEvidenceId::P0Scope];

const QODERWORK_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        QODER_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        QODER_RUNTIME_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        QODER_RUNTIME_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        QODER_SKILLS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        QODER_SKILLS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentHookManagement,
        QODER_HOOKS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentHookManagement,
        QODER_HOOKS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::VendorPrivateStorageUnsupported,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        QODER_MCP_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        QODER_MCP_EVIDENCE,
    ),
];

const TRAE_WORK_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        TRAE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        TRAE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        TRAE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        TRAE_SKILLS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        TRAE_SKILLS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Assisted,
        AgentCapabilityReasonCode::VendorUiRequired,
        TRAE_MODELS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Assisted,
        AgentCapabilityReasonCode::VendorUiRequired,
        TRAE_MODELS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        TRAE_MCP_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        TRAE_MCP_EVIDENCE,
    ),
];

const WORKBUDDY_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        WORKBUDDY_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        WORKBUDDY_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        WORKBUDDY_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentModelValidation,
        WORKBUDDY_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        WORKBUDDY_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        MCP_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        MCP_SERVICE_EVIDENCE,
    ),
];

const GROKBUILD_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        GROKBUILD_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        GROKBUILD_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        GROKBUILD_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentModelValidation,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        MCP_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        MCP_SERVICE_EVIDENCE,
    ),
];

const CODEX_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::NoCatalogProductLink,
        CODEX_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        CODEX_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        CODEX_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentModelValidation,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        MCP_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        MCP_SERVICE_EVIDENCE,
    ),
];

const CLAUDE_CODE_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        CLAUDE_LINK_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        CLAUDE_LINK_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        CLAUDE_LINK_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentModelValidation,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        PROVIDER_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        MCP_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        MCP_SERVICE_EVIDENCE,
    ),
];

const OPENCODE_CAPABILITIES: [DeclaredAgentCapability; 11] = [
    capability(
        AgentCapabilityId::ProductOpen,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::OfficialLinkReviewed,
        OPENCODE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppDetect,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        OPENCODE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::AppLaunch,
        AgentCapabilityMode::Unverified,
        AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
        OPENCODE_PRODUCT_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsRead,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentSkillSynchronization,
        SKILL_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksRead,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::HooksWrite,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityMode::Unsupported,
        AgentCapabilityReasonCode::CapabilityNotApplicable,
        P0_SCOPE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        OPENCODE_MODELS_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpValidate,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::FyagentMcpValidation,
        MCP_SERVICE_EVIDENCE,
    ),
    capability(
        AgentCapabilityId::McpWrite,
        AgentCapabilityMode::Direct,
        AgentCapabilityReasonCode::DedicatedNativeContract,
        MCP_SERVICE_EVIDENCE,
    ),
];

const AGENT_CATALOG: [AgentCatalogEntry; 7] = [
    AgentCatalogEntry {
        id: AgentCatalogId::QoderWork,
        variant_id: AgentVariantId::QoderWorkCn,
        display_name: "QoderWork CN",
        description: "支持 Skills 同步与 MCP 直接分配；不支持第三方模型配置。",
        official_links: &QODERWORK_OFFICIAL_LINKS,
        capabilities: &QODERWORK_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::TraeWork,
        variant_id: AgentVariantId::TraeWorkCn,
        display_name: "TRAE Work CN",
        description:
            "支持 Skills 同步与 MCP 直接分配；自定义模型需在 TRAE Work CN 中添加；不支持 Hooks。",
        official_links: &TRAE_WORK_OFFICIAL_LINKS,
        capabilities: &TRAE_WORK_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::WorkBuddy,
        variant_id: AgentVariantId::WorkBuddy,
        display_name: "WorkBuddy",
        description: "支持 Skills 同步、模型配置与 MCP 直接分配；不支持 Hooks。",
        official_links: &WORKBUDDY_OFFICIAL_LINKS,
        capabilities: &WORKBUDDY_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::GrokBuild,
        variant_id: AgentVariantId::GrokBuild,
        display_name: "Grok Build",
        description: "支持 Skills 同步、模型配置与 MCP 直接分配。本机识别和启动暂无法确认。",
        official_links: &GROKBUILD_OFFICIAL_LINKS,
        capabilities: &GROKBUILD_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::Codex,
        variant_id: AgentVariantId::Codex,
        display_name: "Codex",
        description: "支持桌面安装、Skills、模型配置与 MCP；不支持 Hooks。",
        official_links: &[],
        capabilities: &CODEX_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::ClaudeCode,
        variant_id: AgentVariantId::ClaudeCode,
        display_name: "Claude Code",
        description: "支持 Skills、模型配置与 MCP；不支持 Hooks。本机识别和启动暂无法确认。",
        official_links: &CLAUDE_OFFICIAL_LINKS,
        capabilities: &CLAUDE_CODE_CAPABILITIES,
    },
    AgentCatalogEntry {
        id: AgentCatalogId::OpenCode,
        variant_id: AgentVariantId::OpenCode,
        display_name: "OpenCode",
        description: "支持 Skills、模型配置与 MCP；不支持 Hooks。本机识别和启动暂无法确认。",
        official_links: &OPENCODE_OFFICIAL_LINKS,
        capabilities: &OPENCODE_CAPABILITIES,
    },
];

#[tauri::command]
pub fn get_agent_catalog() -> AgentCatalogResult {
    AgentCatalogResult {
        contract_version: AGENT_CATALOG_CONTRACT_VERSION,
        reviewed_at: AGENT_CATALOG_REVIEWED_AT,
        agents: AGENT_CATALOG.to_vec(),
    }
}

/// Return only privacy-safe runtime facts. Unknown observations remain `null`;
/// they are never converted to a negative installation or running claim.
#[tauri::command]
pub fn get_external_agent_status(agent_id: AgentCatalogId) -> ExternalAgentRuntimeStatus {
    ExternalAgentRuntimeService::get_status(agent_id)
}

/// Resolve a launch request from closed semantic IDs only. The P0 QoderWork
/// and TRAE Work adapters intentionally return `unverified` until a trusted
/// executable/bundle identity is separately reviewed and registered.
#[tauri::command]
pub fn launch_external_agent(
    agent_id: AgentCatalogId,
    destination: ExternalAgentLaunchDestination,
) -> ExternalAgentLaunchResult {
    ExternalAgentRuntimeService::launch(agent_id, destination)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const CAPABILITY_ORDER: [AgentCapabilityId; 11] = [
        AgentCapabilityId::ProductOpen,
        AgentCapabilityId::AppDetect,
        AgentCapabilityId::AppLaunch,
        AgentCapabilityId::SkillsRead,
        AgentCapabilityId::SkillsWrite,
        AgentCapabilityId::HooksRead,
        AgentCapabilityId::HooksWrite,
        AgentCapabilityId::ModelsValidate,
        AgentCapabilityId::ModelsWrite,
        AgentCapabilityId::McpValidate,
        AgentCapabilityId::McpWrite,
    ];

    fn sorted_object_keys(value: &serde_json::Value) -> Vec<&str> {
        let mut keys = value
            .as_object()
            .expect("catalog wire node must be an object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn agent_catalog_freezes_v5_order_variants_links_and_capability_matrix() {
        let catalog = get_agent_catalog();

        assert_eq!(catalog.contract_version, 5);
        assert_eq!(catalog.reviewed_at, "2026-08-31");
        assert_eq!(
            catalog
                .agents
                .iter()
                .map(|entry| (entry.id, entry.variant_id, entry.display_name))
                .collect::<Vec<_>>(),
            [
                (
                    AgentCatalogId::QoderWork,
                    AgentVariantId::QoderWorkCn,
                    "QoderWork CN",
                ),
                (
                    AgentCatalogId::TraeWork,
                    AgentVariantId::TraeWorkCn,
                    "TRAE Work CN",
                ),
                (
                    AgentCatalogId::WorkBuddy,
                    AgentVariantId::WorkBuddy,
                    "WorkBuddy",
                ),
                (
                    AgentCatalogId::GrokBuild,
                    AgentVariantId::GrokBuild,
                    "Grok Build",
                ),
                (AgentCatalogId::Codex, AgentVariantId::Codex, "Codex"),
                (
                    AgentCatalogId::ClaudeCode,
                    AgentVariantId::ClaudeCode,
                    "Claude Code",
                ),
                (
                    AgentCatalogId::OpenCode,
                    AgentVariantId::OpenCode,
                    "OpenCode",
                ),
            ]
        );

        for entry in &catalog.agents {
            assert_eq!(
                entry
                    .capabilities
                    .iter()
                    .map(|capability| capability.id)
                    .collect::<Vec<_>>(),
                CAPABILITY_ORDER
            );
        }

        let qoder = &catalog.agents[0];
        let trae = &catalog.agents[1];
        let grok = &catalog.agents[3];
        let codex = &catalog.agents[4];
        assert_eq!(
            qoder
                .capabilities
                .iter()
                .map(|capability| capability.mode)
                .collect::<Vec<_>>(),
            [
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
            ]
        );
        assert_eq!(
            trae.capabilities
                .iter()
                .map(|capability| capability.mode)
                .collect::<Vec<_>>(),
            [
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Assisted,
                AgentCapabilityMode::Assisted,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
            ]
        );
        assert_eq!(
            qoder
                .capabilities
                .iter()
                .map(|capability| (capability.reason_code, capability.evidence_ids))
                .collect::<Vec<_>>(),
            [
                (
                    AgentCapabilityReasonCode::OfficialLinkReviewed,
                    QODER_PRODUCT_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    QODER_RUNTIME_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    QODER_RUNTIME_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentSkillSynchronization,
                    QODER_SKILLS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentSkillSynchronization,
                    QODER_SKILLS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentHookManagement,
                    QODER_HOOKS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentHookManagement,
                    QODER_HOOKS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::CapabilityNotApplicable,
                    P0_SCOPE_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::VendorPrivateStorageUnsupported,
                    P0_SCOPE_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentMcpValidation,
                    QODER_MCP_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::DedicatedNativeContract,
                    QODER_MCP_EVIDENCE,
                ),
            ]
        );
        assert_eq!(
            trae.capabilities
                .iter()
                .map(|capability| (capability.reason_code, capability.evidence_ids))
                .collect::<Vec<_>>(),
            [
                (
                    AgentCapabilityReasonCode::OfficialLinkReviewed,
                    TRAE_PRODUCT_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    TRAE_PRODUCT_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::TrustedRuntimeIdentityUnavailable,
                    TRAE_PRODUCT_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentSkillSynchronization,
                    TRAE_SKILLS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentSkillSynchronization,
                    TRAE_SKILLS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::CapabilityNotApplicable,
                    P0_SCOPE_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::CapabilityNotApplicable,
                    P0_SCOPE_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::VendorUiRequired,
                    TRAE_MODELS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::VendorUiRequired,
                    TRAE_MODELS_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::FyagentMcpValidation,
                    TRAE_MCP_EVIDENCE,
                ),
                (
                    AgentCapabilityReasonCode::DedicatedNativeContract,
                    TRAE_MCP_EVIDENCE,
                ),
            ]
        );
        let workbuddy = &catalog.agents[2];
        assert_eq!(
            workbuddy
                .capabilities
                .iter()
                .map(|capability| capability.mode)
                .collect::<Vec<_>>(),
            [
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Unverified,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
                AgentCapabilityMode::Direct,
            ]
        );
        assert_eq!(workbuddy.capabilities[3].id, AgentCapabilityId::SkillsRead);
        assert_eq!(workbuddy.capabilities[4].id, AgentCapabilityId::SkillsWrite);
        assert_eq!(workbuddy.capabilities[9].id, AgentCapabilityId::McpValidate);
        assert_eq!(workbuddy.capabilities[10].id, AgentCapabilityId::McpWrite);
        assert!(workbuddy.description.contains("支持 Skills 同步"));
        assert!(workbuddy.description.contains("MCP 直接分配"));
        assert!(!qoder.description.contains("本机识别和启动暂无法确认"));
        assert!(!trae.description.contains("本机识别和启动暂无法确认"));
        assert!(!workbuddy.description.contains("本机识别和启动暂无法确认"));
        assert!(grok.description.contains("本机识别和启动暂无法确认"));
        assert_eq!(grok.display_name, "Grok Build");
        assert_eq!(grok.official_links[0].label, "打开 Grok Build 官方页面");
        assert_eq!(grok.official_links[0].url, "https://x.ai/grok");
        assert!(grok.description.contains("支持 Skills 同步"));
        assert!(grok.description.contains("MCP 直接分配"));
        assert_eq!(trae.display_name, "TRAE Work CN");
        assert_eq!(trae.official_links[0].label, "打开 TRAE Work CN 官方页面");
        assert_eq!(trae.official_links[0].url, "https://www.trae.cn/sem-work");
        assert!(qoder.description.contains("不支持第三方模型配置"));
        assert!(qoder.description.contains("MCP 直接分配"));
        assert!(
            !qoder.description.contains("Hooks") && !qoder.description.contains("hooks"),
            "QoderWork CN description must not mention Hooks"
        );
        assert!(trae.description.contains("MCP 直接分配"));
        assert!(trae
            .description
            .contains("自定义模型需在 TRAE Work CN 中添加"));
        for entry in &catalog.agents {
            assert!(
                !entry.description.contains("可通过 FyAgent"),
                "{} must not use 可通过 FyAgent",
                entry.display_name
            );
            assert!(
                !entry.description.contains("可在 FyAgent"),
                "{} must not use 可在 FyAgent",
                entry.display_name
            );
        }
        assert_eq!(
            catalog.agents[6].capabilities[8],
            capability(
                AgentCapabilityId::ModelsWrite,
                AgentCapabilityMode::Direct,
                AgentCapabilityReasonCode::DedicatedNativeContract,
                OPENCODE_MODELS_EVIDENCE,
            )
        );
        assert!(codex.official_links.is_empty());
        assert_eq!(
            codex.capabilities[0],
            capability(
                AgentCapabilityId::ProductOpen,
                AgentCapabilityMode::Unsupported,
                AgentCapabilityReasonCode::NoCatalogProductLink,
                CODEX_EVIDENCE,
            )
        );
        let claude = &catalog.agents[5];
        let opencode = &catalog.agents[6];
        assert_eq!(claude.display_name, "Claude Code");
        assert_eq!(
            claude
                .official_links
                .iter()
                .map(|link| (link.id, link.label, link.url))
                .collect::<Vec<_>>(),
            [(
                AgentOfficialLinkId::Desktop,
                "Claude Desktop",
                "https://claude.com/download",
            )]
        );
        assert_eq!(
            opencode
                .official_links
                .iter()
                .map(|link| (link.id, link.label, link.url))
                .collect::<Vec<_>>(),
            [
                (
                    AgentOfficialLinkId::Product,
                    "打开 OpenCode 官方页面",
                    "https://opencode.ai",
                ),
                (
                    AgentOfficialLinkId::Desktop,
                    "打开 OpenCode 官方下载页",
                    "https://opencode.ai/download",
                ),
            ]
        );
        for entry in &catalog.agents {
            assert!(
                entry
                    .official_links
                    .iter()
                    .all(|link| link.id != AgentOfficialLinkId::Cli),
                "{} must not advertise an Agent Catalog CLI install link",
                entry.display_name
            );
        }
    }

    #[test]
    fn agent_catalog_rejects_duplicate_ids_and_invalid_official_links_by_contract() {
        let catalog = get_agent_catalog();
        let mut agent_ids = HashSet::new();

        for entry in &catalog.agents {
            assert!(agent_ids.insert(entry.id));

            let mut capability_ids = HashSet::new();
            for capability in entry.capabilities {
                assert!(capability_ids.insert(capability.id));
                let mut evidence_ids = HashSet::new();
                for evidence_id in capability.evidence_ids {
                    assert!(evidence_ids.insert(*evidence_id));
                }
            }

            let mut link_ids = HashSet::new();
            for link in entry.official_links {
                assert!(link_ids.insert(link.id));
                assert!(!link.label.trim().is_empty());
                let url = url::Url::parse(link.url).expect("official URL must parse");
                assert_eq!(url.scheme(), "https");
                assert!(url.host_str().is_some());
                assert!(url.username().is_empty());
                assert!(url.password().is_none());
                assert!(url.query().is_none());
                assert!(url.fragment().is_none());
            }
        }
    }

    #[test]
    fn agent_catalog_wire_is_exact_v5_and_contains_no_legacy_or_sensitive_fields() {
        let value = serde_json::to_value(get_agent_catalog()).expect("catalog serializes");

        assert_eq!(value["contractVersion"], 5);
        assert_eq!(value["reviewedAt"], "2026-08-31");
        assert_eq!(
            sorted_object_keys(&value),
            ["agents", "contractVersion", "reviewedAt"]
        );
        for entry in value["agents"].as_array().expect("agents must be an array") {
            assert_eq!(
                sorted_object_keys(entry),
                [
                    "capabilities",
                    "description",
                    "displayName",
                    "id",
                    "officialLinks",
                    "variantId",
                ]
            );
            for link in entry["officialLinks"]
                .as_array()
                .expect("officialLinks must be an array")
            {
                assert_eq!(sorted_object_keys(link), ["id", "label", "url"]);
            }
            for capability in entry["capabilities"]
                .as_array()
                .expect("capabilities must be an array")
            {
                assert_eq!(
                    sorted_object_keys(capability),
                    ["evidenceIds", "id", "mode", "reasonCode"]
                );
            }
            for legacy in ["status", "actions", "evidenceLabel", "officialUrl"] {
                assert!(entry.get(legacy).is_none());
            }
        }

        let serialized = serde_json::to_string(&value)
            .expect("catalog wire value serializes")
            .to_ascii_lowercase();
        for prohibited in [
            "apikey",
            "api_key",
            "access_token",
            "password",
            "processid",
            "process_id",
            "executable",
            "c:\\\\",
            "/users/",
            "~/.",
            "docs/cli",
            "claude-code/getting-started",
            "claude code cli",
            "opencode cli",
        ] {
            assert!(!serialized.contains(prohibited));
        }
    }

    #[test]
    fn agent_catalog_closed_enums_reject_legacy_and_unknown_values() {
        for invalid in [r#""trae""#, r#""qoder""#, r#""legacy""#, r#""unknown""#] {
            assert!(serde_json::from_str::<AgentCatalogId>(invalid).is_err());
            assert!(serde_json::from_str::<AgentVariantId>(invalid).is_err());
            assert!(serde_json::from_str::<AgentCapabilityId>(invalid).is_err());
            assert!(serde_json::from_str::<AgentCapabilityMode>(invalid).is_err());
            assert!(serde_json::from_str::<AgentCapabilityReasonCode>(invalid).is_err());
            assert!(serde_json::from_str::<AgentEvidenceId>(invalid).is_err());
            assert!(serde_json::from_str::<AgentOfficialLinkId>(invalid).is_err());
            assert!(serde_json::from_str::<ExternalAgentLaunchDestination>(invalid).is_err());
        }
    }

    #[test]
    fn agent_catalog_runtime_commands_are_exported_without_expanding_their_input_boundary() {
        let commands_index = include_str!("mod.rs").replace("\r\n", "\n");
        assert_eq!(commands_index.matches("mod agent_catalog;").count(), 1);
        assert_eq!(
            commands_index.matches("pub use agent_catalog::*;").count(),
            1
        );

        let _: fn(AgentCatalogId) -> ExternalAgentRuntimeStatus = get_external_agent_status;
        let _: fn(AgentCatalogId, ExternalAgentLaunchDestination) -> ExternalAgentLaunchResult =
            launch_external_agent;
    }

    #[test]
    fn application_acl_covers_every_registered_command_without_remote_access() {
        use std::collections::BTreeSet;

        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../../capabilities/default.json"))
                .expect("default capability parses");
        let permissions = capability["permissions"]
            .as_array()
            .expect("default capability has permissions");
        for expected in [
            "allow-legacy-application-commands",
            "allow-external-agent-observe",
            "allow-external-agent-launch",
            "allow-qoderwork-hooks-write",
            "allow-external-agent-endpoint-probe",
            "allow-change-plan",
            "allow-agent-install-readiness",
        ] {
            assert_eq!(
                permissions
                    .iter()
                    .filter(|permission| permission.as_str() == Some(expected))
                    .count(),
                1,
                "permission {expected} must be attached exactly once"
            );
        }
        assert!(capability.get("remote").is_none());

        fn allowed_commands(manifest: &str) -> BTreeSet<String> {
            let value: toml::Value = toml::from_str(manifest).expect("permission TOML parses");
            value["permission"]
                .as_array()
                .expect("permission manifest contains permission entries")
                .iter()
                .flat_map(|permission| {
                    permission["commands"]["allow"]
                        .as_array()
                        .expect("permission contains commands.allow")
                })
                .map(|command| {
                    command
                        .as_str()
                        .expect("allowed command is a string")
                        .to_owned()
                })
                .collect()
        }

        let legacy_manifest = include_str!("../../permissions/legacy-application-commands.toml");
        let external_manifest = include_str!("../../permissions/external-agent-p0.toml");
        let change_plan_manifest = include_str!("../../permissions/change-plan-readiness.toml");
        let legacy_commands = allowed_commands(legacy_manifest);
        let external_commands = allowed_commands(external_manifest);
        let change_plan_commands = allowed_commands(change_plan_manifest);
        assert!(legacy_commands.is_disjoint(&external_commands));
        assert!(legacy_commands.is_disjoint(&change_plan_commands));
        assert!(external_commands.is_disjoint(&change_plan_commands));

        let mut allowed = legacy_commands;
        allowed.extend(external_commands);
        allowed.extend(change_plan_commands);

        let handler = include_str!("../lib.rs");
        let registered = handler
            .lines()
            .skip_while(|line| !line.contains(".invoke_handler(tauri::generate_handler!["))
            .filter_map(|line| {
                let line = line.trim();
                line.strip_prefix("commands::")
                    .and_then(|command| command.strip_suffix(','))
                    .or_else(|| (line == "update_tray_menu,").then_some("update_tray_menu"))
                    .map(str::to_owned)
            })
            .collect::<BTreeSet<_>>();

        assert_eq!(registered.len(), 354, "review intentional handler changes");
        assert_eq!(allowed, registered, "every registered application command must be granted exactly once while an app ACL manifest exists");
    }
}
