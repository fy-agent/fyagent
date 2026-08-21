import {
  MCP_TARGET_IDS,
  SKILL_TARGET_IDS,
  type AgentCatalogId,
  type AgentVariantId,
  type McpTargetId,
  type PromptAppId,
  type SkillTargetId,
} from "./directory";

export {
  AGENT_CATALOG_IDS,
  AGENT_VARIANT_IDS,
  MCP_TARGET_IDS,
  MCP_TARGETS,
  PRODUCT_DIRECTORY,
  PROMPT_APP_IDS,
  PROMPT_ONLY_DIRECTORY,
  SKILL_TARGET_IDS,
  SKILL_TARGETS,
  SUPPORTED_APP_IDS,
  SUPPORTED_APPS,
} from "./directory";
export type {
  AgentCatalogId,
  AgentVariantId,
  McpTargetId,
  PromptAppId,
  SkillTargetId,
};

/** @deprecated Use McpTargetId or SkillTargetId at the owning feature boundary. */
export type SupportedAppId = import("./directory").McpTargetId;

export type McpAssignments = Record<McpTargetId, boolean> &
  Record<string, boolean | undefined>;

export type SkillAssignments = Record<SkillTargetId, boolean> &
  Record<string, boolean | undefined>;

/** @deprecated Use SkillAssignments or McpAssignments. */
export type AppAssignments = SkillAssignments;

export interface InstalledSkill {
  id: string;
  name: string;
  description?: string;
  directory: string;
  path?: string;
  repoOwner?: string;
  repoName?: string;
  repoBranch?: string;
  readmeUrl?: string;
  apps: SkillAssignments;
  installedAt: number;
  contentHash?: string;
  updatedAt: number;
}

export interface DiscoverableSkill {
  key: string;
  name: string;
  description: string;
  directory: string;
  readmeUrl?: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
}

export const SKILL_DISCOVERY_PAGE_SIZE = 21;
export const SKILL_DISCOVERY_MAX_PAGE_SIZE = 50;
export const SKILLHUB_MARKET_OWNER = "skillhub.cn";
export const SKILLHUB_CATEGORY_ALL = "all" as const;
/** 官方 12 个一级分类，口径见 SkillHub `find-skill-skillhub` 的 categories.md。 */
export const SKILLHUB_OFFICIAL_CATEGORIES = [
  { key: "office-efficiency", name: "办公效率" },
  { key: "content-creation", name: "内容创作" },
  { key: "dev-programming", name: "开发编程" },
  { key: "data-analysis", name: "数据分析" },
  { key: "design-media", name: "设计多媒体" },
  { key: "ai-agent", name: "AI Agent" },
  { key: "knowledge-management", name: "知识管理" },
  { key: "business-ops", name: "商业运营" },
  { key: "education", name: "教育学习" },
  { key: "professional", name: "行业专业" },
  { key: "it-ops-security", name: "IT 运维与安全" },
  { key: "life-service", name: "生活服务" },
] as const;

export type SkillHubCategoryKey =
  (typeof SKILLHUB_OFFICIAL_CATEGORIES)[number]["key"];
export type SkillHubCategoryFilter =
  | typeof SKILLHUB_CATEGORY_ALL
  | SkillHubCategoryKey;

export interface SkillHubCategory {
  key: string;
  name: string;
}

export const SKILLHUB_CATEGORY_TABS: ReadonlyArray<{
  id: SkillHubCategoryFilter;
  label: string;
}> = [
  { id: SKILLHUB_CATEGORY_ALL, label: "全部" },
  ...SKILLHUB_OFFICIAL_CATEGORIES.map((item) => ({
    id: item.key,
    label: item.name,
  })),
];

export type SkillDiscoveryStatus = "all" | "installed" | "uninstalled";

export interface DiscoverableSkillsPage {
  skills: DiscoverableSkill[];
  totalCount: number;
}

export interface DiscoverSkillsPageRequest {
  query: string;
  repo?: string;
  status: SkillDiscoveryStatus;
  limit: number;
  offset: number;
}

export interface SkillHubSkill {
  key: string;
  slug: string;
  name: string;
  description: string;
  directory: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  version?: string;
  ownerName?: string;
  installs?: number;
  downloads?: number;
  homepageUrl: string;
  readmeUrl?: string;
  category?: string;
}

export interface SkillHubSearchResult {
  skills: SkillHubSkill[];
  totalCount: number;
  query: string;
  categories?: SkillHubCategory[];
}

export interface SkillUpdateInfo {
  id: string;
  name: string;
  currentHash?: string;
  remoteHash: string;
}

export interface SkillRepo {
  owner: string;
  name: string;
  branch: string;
  enabled: boolean;
}

export interface UnmanagedSkill {
  directory: string;
  name: string;
  description?: string;
  foundIn: string[];
  path: string;
}

export interface ImportSkillSelection {
  directory: string;
  apps: SkillAssignments;
}

export interface SkillBackupEntry {
  backupId: string;
  backupPath: string;
  createdAt: number;
  skill: InstalledSkill;
}

export interface SkillMigrationResult {
  migratedCount: number;
  skippedCount: number;
  errors: string[];
}

export interface McpServerSpec extends Record<string, unknown> {
  type?: "stdio" | "http" | "sse";
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  headers?: Record<string, string>;
}

export interface McpServer extends Record<string, unknown> {
  id: string;
  name: string;
  server: McpServerSpec;
  apps: McpAssignments;
  description?: string;
  tags?: string[];
  homepage?: string;
  docs?: string;
  source?: string;
}

export type McpServersMap = Record<string, McpServer>;

export type FeatureSettings = Record<string, unknown> & {
  skillSyncMethod?: "auto" | "symlink" | "copy";
  skillStorageLocation?: "fyagent" | "unified";
};

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

export type ProviderAppId = "claude" | "codex" | "grokbuild";

export interface ProviderQuickSetupRequest {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  /** Codex 原生能力意图，仅 codex 目标生效。 */
  codexFeatures?: {
    imageExtension?: boolean;
    websockets?: boolean;
  };
}

/** Non-secret projection returned by Provider reads in V2. */
export interface ProviderSummary {
  id: string;
  name: string;
  modelId?: string;
}

export type ProviderSummaryMap = Record<string, ProviderSummary>;

export interface ProviderSummaryQueryData {
  providers: ProviderSummaryMap;
  currentId: string;
}

export type CodexProviderMutationWarning =
  | "CODEX_WEBSOCKET_NON_GPT_MODEL"
  | "CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED";

export interface ProviderMutationResult<T> {
  value: T;
  liveConfigChanged: boolean;
  app: ProviderAppId;
  warningCodes?: CodexProviderMutationWarning[];
}

export interface ProviderSwitchResult {
  warnings: string[];
}

export type ProviderQuickSetupFailureCode =
  | "APPLY_FAILED_ROLLED_BACK"
  | "ROLLBACK_PARTIAL_STATE_UNKNOWN";

export interface ProviderQuickSetupCommandError {
  code: ProviderQuickSetupFailureCode;
}

export interface WorkBuddyStatus {
  path: string;
  exists: boolean;
  modelCount: number;
  revision: string | null;
  backupExists: boolean;
  format: "legacyArray" | "objectRoot" | "missing";
}

export interface WorkBuddyModelIdsResult {
  ids: string[];
  revision: string | null;
}

export interface WorkBuddyFetchModelsRequest {
  baseUrl: string;
  apiKey: string;
  allowNoApiKey: boolean;
}

export interface WorkBuddyFetchModelsResult {
  models: string[];
  truncated: boolean;
}

export interface WorkBuddySaveModelsRequest
  extends WorkBuddyFetchModelsRequest {
  selectedModelIds: string[];
  manualModelIds: string[];
  removedModelIds?: string[];
  clearExistingApiKeys: boolean;
  expectedRevision: string | null;
  overwriteToken?: string;
}

export interface WorkBuddySaveModelsSavedResult {
  state: "saved";
  revision: string;
  modelCount: number;
  createdEntries: number;
  updatedEntries: number;
}

export interface WorkBuddyOverwriteConfirmationRequiredResult {
  state: "overwrite_confirmation_required";
  token: string;
  existingIds: string[];
}

export interface WorkBuddyConcurrentModificationResult {
  state: "concurrent_modification";
}

export type WorkBuddySaveModelsResult =
  | WorkBuddySaveModelsSavedResult
  | WorkBuddyOverwriteConfirmationRequiredResult
  | WorkBuddyConcurrentModificationResult;

export interface FetchedModelRef {
  id: string;
  ownedBy?: string | null;
}

export interface FetchedModelList {
  models: FetchedModelRef[];
  truncated: boolean;
}

export type ReachabilityStatus = "operational" | "degraded" | "failed";

export interface ReachabilityResult {
  success: boolean;
  status: ReachabilityStatus;
  message: string;
  responseTimeMs: number | null;
  httpStatus: number | null;
}

export type ModelProbeAppId =
  | "claude"
  | "codex"
  | "grokbuild"
  | "workbuddy"
  | "opencode";

export interface ModelProbeRequest {
  app: ModelProbeAppId;
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

export interface ModelProbeResult {
  success: boolean;
  status: ReachabilityStatus;
  message: string;
  responseTimeMs: number | null;
  httpStatus: number | null;
  modelUsed: string;
  errorCategory: string | null;
}

export interface TraeWorkModelIdsResult {
  modelIds: string[];
  revision: string | null;
  truncated: boolean;
}

export interface OpenCodeProviderSnapshot {
  id: string;
  name: string;
  modelIds: string[];
}

export interface OpenCodeModelSnapshot {
  providers: OpenCodeProviderSnapshot[];
  revision: string | null;
}

export interface OpenCodeFetchModelsRequest {
  baseUrl: string;
  apiKey: string;
  allowNoApiKey: boolean;
}

export interface OpenCodeSaveModelsRequest {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  selectedModelIds: string[];
  removedModelIds?: string[];
  expectedRevision: string | null;
  overwriteToken?: string;
}

export type OpenCodeSaveModelsResult =
  | WorkBuddySaveModelsSavedResult
  | WorkBuddyOverwriteConfirmationRequiredResult
  | WorkBuddyConcurrentModificationResult;

export function createSkillAssignments(
  enabled: readonly SkillTargetId[] = [],
): SkillAssignments {
  const enabledSet = new Set(enabled);
  return {
    ...(Object.fromEntries(
      SKILL_TARGET_IDS.map((id) => [id, enabledSet.has(id)]),
    ) as SkillAssignments),
    "claude-desktop": false,
    openclaw: false,
  };
}

export function createMcpAssignments(
  enabled: readonly McpTargetId[] = [],
): McpAssignments {
  const enabledSet = new Set(enabled);
  return Object.fromEntries(
    MCP_TARGET_IDS.map((id) => [id, enabledSet.has(id)]),
  ) as McpAssignments;
}

/** @deprecated Use createSkillAssignments or createMcpAssignments. */
export const createAssignments = createSkillAssignments;

export interface ManagedPrompt {
  id: string;
  name: string;
  content: string;
  description?: string;
  enabled: boolean;
  createdAt?: number;
  updatedAt?: number;
}

export const MEMORY_DOCUMENT_IDS = [
  "openclaw-memory",
  "openclaw-user",
  "hermes-memory",
  "hermes-user",
] as const;

export type MemoryDocumentId = (typeof MEMORY_DOCUMENT_IDS)[number];

export const HERMES_MEMORY_KINDS = ["memory", "user"] as const;

export type HermesMemoryKind = (typeof HERMES_MEMORY_KINDS)[number];

export interface HermesMemoryLimits {
  memory: number;
  user: number;
  memoryEnabled: boolean;
  userEnabled: boolean;
}

export interface DailyMemoryFileInfo {
  filename: string;
  date: string;
  sizeBytes: number;
  modifiedAt: number;
  preview: string;
}

export interface DailyMemorySearchResult {
  filename: string;
  date: string;
  sizeBytes: number;
  modifiedAt: number;
  snippet: string;
  matchCount: number;
}

export type OpenClawDirectory = "workspace" | "memory";
