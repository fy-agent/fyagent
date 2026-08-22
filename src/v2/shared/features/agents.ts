import type { AgentCatalogId, AgentVariantId } from "./directory";

export const AGENT_OFFICIAL_LINK_IDS = ["product", "cli", "desktop"] as const;
export type AgentOfficialLinkId = (typeof AGENT_OFFICIAL_LINK_IDS)[number];

export interface AgentOfficialLink {
  id: AgentOfficialLinkId;
  label: string;
  url: string;
}

export const AGENT_CAPABILITY_IDS = [
  "product.open",
  "app.detect",
  "app.launch",
  "skills.read",
  "skills.write",
  "hooks.read",
  "hooks.write",
  "models.validate",
  "models.write",
  "mcp.validate",
  "mcp.write",
] as const;
export type AgentCapabilityId = (typeof AGENT_CAPABILITY_IDS)[number];

export const AGENT_CAPABILITY_MODES = [
  "direct",
  "assisted",
  "unsupported",
  "unverified",
] as const;
export type AgentCapabilityMode = (typeof AGENT_CAPABILITY_MODES)[number];

export const AGENT_CAPABILITY_REASON_CODES = [
  "official_link_reviewed",
  "trusted_runtime_identity_unavailable",
  "dedicated_agent_flow",
  "fyagent_skill_synchronization",
  "fyagent_hook_management",
  "fyagent_model_validation",
  "fyagent_mcp_validation",
  "vendor_ui_required",
  "vendor_private_storage_unsupported",
  "dedicated_native_contract",
  "capability_not_applicable",
  "no_catalog_product_link",
] as const;
export type AgentCapabilityReasonCode =
  (typeof AGENT_CAPABILITY_REASON_CODES)[number];

export const AGENT_EVIDENCE_IDS = [
  "qoderwork_product",
  "qoderwork_install",
  "qoderwork_skills",
  "qoderwork_hooks",
  "qoderwork_hooks_native_contract",
  "qoderwork_connectors",
  "trae_work_product",
  "trae_work_skills",
  "trae_work_models",
  "trae_work_model_validation_contract",
  "trae_work_mcp",
  "external_mcp_validation_contract",
  "workbuddy_native_contract",
  "codex_desktop_installer_contract",
  "provider_quick_setup_contract",
  "skill_service_contract",
  "mcp_service_contract",
  "claude_official_links",
  "opencode_product",
  "opencode_models",
  "grokbuild_product",
  "p0_scope",
] as const;
export type AgentEvidenceId = (typeof AGENT_EVIDENCE_IDS)[number];

export interface DeclaredAgentCapability {
  id: AgentCapabilityId;
  mode: AgentCapabilityMode;
  reasonCode: AgentCapabilityReasonCode;
  evidenceIds: AgentEvidenceId[];
}

export interface AgentCatalogEntry {
  id: AgentCatalogId;
  variantId: AgentVariantId;
  displayName: string;
  description: string;
  officialLinks: AgentOfficialLink[];
  capabilities: DeclaredAgentCapability[];
}

export const AGENT_CATALOG_CONTRACT_VERSION = 4;
export interface AgentCatalogResult {
  contractVersion: typeof AGENT_CATALOG_CONTRACT_VERSION;
  reviewedAt: string;
  agents: AgentCatalogEntry[];
}

export const EXTERNAL_AGENT_LAUNCH_DESTINATIONS = [
  "home",
  "skills",
  "hooks",
  "models",
  "mcp",
] as const;
export type ExternalAgentLaunchDestination =
  (typeof EXTERNAL_AGENT_LAUNCH_DESTINATIONS)[number];

export const EXTERNAL_AGENT_RUNTIME_STATES = [
  "available",
  "assisted",
  "unavailable",
  "unverified",
  "blocked_by_version",
  "probe_failed",
] as const;
export type ExternalAgentRuntimeCapabilityState =
  (typeof EXTERNAL_AGENT_RUNTIME_STATES)[number];

export const EXTERNAL_AGENT_INSTALL_SOURCES = [
  "managed_installer",
  "official_installer",
  "system_package",
  "user_installation",
] as const;
export type ExternalAgentInstallSource =
  (typeof EXTERNAL_AGENT_INSTALL_SOURCES)[number];

export interface ExternalAgentRuntimeCapability {
  id: AgentCapabilityId;
  state: ExternalAgentRuntimeCapabilityState;
  reasonCode: AgentCapabilityReasonCode;
}

export interface ExternalAgentRuntimeStatus {
  agentId: AgentCatalogId;
  detected: boolean | null;
  running: boolean | null;
  version: string | null;
  installSource: ExternalAgentInstallSource | null;
  capabilities: ExternalAgentRuntimeCapability[];
}

export interface ExternalAgentLaunchResult {
  agentId: AgentCatalogId;
  destination: ExternalAgentLaunchDestination;
  state: ExternalAgentRuntimeCapabilityState;
  reasonCode: AgentCapabilityReasonCode;
}

export const QODERWORK_HOOK_EVENTS = [
  "SessionStart",
  "SessionEnd",
  "UserPromptSubmit",
  "PreToolUse",
  "PostToolUse",
  "PostToolUseFailure",
  "Stop",
  "SubagentStart",
  "SubagentStop",
  "PreCompact",
  "Notification",
  "PermissionRequest",
] as const;
export type QoderWorkHookEvent = (typeof QODERWORK_HOOK_EVENTS)[number];

export interface QoderWorkCommandHook {
  type: "command";
  command: string;
  timeout?: number;
}

export interface QoderWorkHookGroup {
  event: QoderWorkHookEvent;
  matcher?: string;
  hooks: QoderWorkCommandHook[];
}

export interface QoderWorkHooksSnapshot {
  revision: string | null;
  exists: boolean;
  groups: QoderWorkHookGroup[];
  restartRequired: true;
  supportedStructure: boolean;
}

export interface SaveQoderWorkHooksRequest {
  expectedRevision?: string | null;
  groups: QoderWorkHookGroup[];
  overwriteToken?: string;
}

export type SaveQoderWorkHooksResult =
  | { state: "saved"; snapshot: QoderWorkHooksSnapshot }
  | { state: "overwrite_confirmation_required"; token: string }
  | { state: "concurrent_modification" };

export type ExternalMcpAgentId = "qoderwork" | "trae-work";
export const EXTERNAL_MCP_TRANSPORTS = ["stdio", "http"] as const;
export type ExternalMcpTransport = (typeof EXTERNAL_MCP_TRANSPORTS)[number];

export const EXTERNAL_MCP_FINDING_REASON_CODES = [
  "TRAE_MCP_SERVER_VALID",
  "TRAE_MCP_UNKNOWN_FIELD",
  "TRAE_MCP_INVALID_COMMAND",
  "TRAE_MCP_COMMAND_NOT_FOUND",
  "TRAE_MCP_INVALID_ARGS",
  "TRAE_MCP_INVALID_ENV",
  "TRAE_MCP_INVALID_URL",
  "TRAE_MCP_UNSAFE_ADDRESS",
  "TRAE_MCP_INVALID_HEADERS",
  "TRAE_MCP_CONTROL_CHARACTER",
  "TRAE_MCP_LIMIT_EXCEEDED",
] as const;
export type ExternalMcpFindingReasonCode =
  (typeof EXTERNAL_MCP_FINDING_REASON_CODES)[number];

export interface ExternalMcpFinding {
  serverId: string;
  transport: ExternalMcpTransport;
  reasonCodes: ExternalMcpFindingReasonCode[];
  executableAvailable: boolean | null;
  hasSecrets: boolean;
}

export interface ExternalMcpValidationResult {
  agentId: ExternalMcpAgentId;
  valid: boolean;
  findings: ExternalMcpFinding[];
  redactedTemplate: Record<string, unknown>;
}

export const TRAE_MODEL_API_FORMATS = [
  "openai_chat_completions",
  "anthropic_messages",
] as const;
export type TraeModelApiFormat = (typeof TRAE_MODEL_API_FORMATS)[number];
export const TRAE_MODEL_URL_MODES = ["base_url", "complete_url"] as const;
export type TraeModelUrlMode = (typeof TRAE_MODEL_URL_MODES)[number];

export interface TraeWorkModelRequest {
  apiFormat: TraeModelApiFormat;
  urlMode: TraeModelUrlMode;
  url: string;
  modelId: string;
  apiKey: string;
  allowNoApiKey: boolean;
  allowLoopback: boolean;
  allowPrivateNetwork: boolean;
}

export const TRAE_MODEL_RESULT_STATES = [
  "valid",
  "reachable",
  "auth_rejected",
  "model_rejected",
  "network_rejected",
  "timeout",
  "cancelled",
] as const;
export type TraeModelResultState = (typeof TRAE_MODEL_RESULT_STATES)[number];

export const TRAE_MODEL_RESULT_REASON_CODES = [
  "TRAE_MODEL_CONFIG_VALID",
  "TRAE_ENDPOINT_REACHABLE",
  "TRAE_ENDPOINT_AUTH_REJECTED",
  "TRAE_ENDPOINT_MODEL_REJECTED",
  "TRAE_ENDPOINT_HTTP_REJECTED",
  "TRAE_ENDPOINT_NETWORK_REJECTED",
  "TRAE_ENDPOINT_TIMEOUT",
  "TRAE_ENDPOINT_CANCELLED",
  "TRAE_DNS_RESOLUTION_FAILED",
  "TRAE_DNS_ADDRESS_REJECTED",
  "TRAE_DNS_ADDRESS_CLASS_MIXED",
  "TRAE_ENDPOINT_RESPONSE_TOO_LARGE",
  "PROXY_DNS_PIN_UNSUPPORTED",
] as const;
export type TraeModelResultReasonCode =
  (typeof TRAE_MODEL_RESULT_REASON_CODES)[number];

export const TRAE_MODEL_DURATION_BUCKETS = [
  "lt_1s",
  "1s_to_3s",
  "3s_to_10s",
  "gte_10s",
] as const;
export type TraeModelDurationBucket =
  (typeof TRAE_MODEL_DURATION_BUCKETS)[number];
export const TRAE_MODEL_STATUS_CLASSES = ["2xx", "3xx", "4xx", "5xx"] as const;
export type TraeModelStatusClass =
  | (typeof TRAE_MODEL_STATUS_CLASSES)[number]
  | null;

export interface TraeModelValidationResult {
  requestId: string;
  state: "valid";
  reasonCode: "TRAE_MODEL_CONFIG_VALID";
  durationBucket: "lt_1s";
  statusClass: null;
}

export interface TraeModelProbeResult {
  requestId: string;
  state: Exclude<TraeModelResultState, "valid">;
  reasonCode: Exclude<TraeModelResultReasonCode, "TRAE_MODEL_CONFIG_VALID">;
  durationBucket: TraeModelDurationBucket;
  statusClass: TraeModelStatusClass;
}

export interface CancelTraeModelProbeResult {
  requestId: string;
  cancelled: boolean;
}
