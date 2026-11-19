export type AgentDirectoryPriority = "domestic" | "standard";
export type AgentModelCapability = "configurable" | "official-only";

export const PRODUCT_DIRECTORY = [
  {
    agentId: "qoderwork",
    assignmentId: "qoderwork",
    modelTarget: "qoderwork",
    promptAppId: null,
    displayName: "QoderWork CN",
    directoryPriority: "domestic",
    modelCapability: "official-only",
  },
  {
    agentId: "trae-work",
    assignmentId: "trae-work",
    modelTarget: "trae",
    promptAppId: null,
    displayName: "TRAE Work CN",
    directoryPriority: "domestic",
    modelCapability: "official-only",
  },
  {
    agentId: "workbuddy",
    assignmentId: "workbuddy",
    modelTarget: "workbuddy",
    promptAppId: null,
    displayName: "WorkBuddy",
    directoryPriority: "domestic",
    modelCapability: "configurable",
  },
  {
    agentId: "grokbuild",
    assignmentId: "grokbuild",
    modelTarget: "grokbuild",
    promptAppId: "grokbuild",
    displayName: "Grok Build",
    directoryPriority: "standard",
    modelCapability: "configurable",
  },
  {
    agentId: "codex",
    assignmentId: "codex",
    modelTarget: "codex",
    promptAppId: "codex",
    displayName: "Codex",
    directoryPriority: "standard",
    modelCapability: "configurable",
  },
  {
    agentId: "claude-code",
    assignmentId: "claude",
    modelTarget: "claude",
    promptAppId: "claude",
    displayName: "Claude Code",
    directoryPriority: "standard",
    modelCapability: "configurable",
  },
  {
    agentId: "opencode",
    assignmentId: "opencode",
    modelTarget: "opencode",
    promptAppId: "opencode",
    displayName: "OpenCode",
    directoryPriority: "standard",
    modelCapability: "configurable",
  },
  {
    agentId: "omp",
    assignmentId: null,
    modelTarget: "omp",
    promptAppId: null,
    displayName: "Oh My Pi (omp)",
    directoryPriority: "standard",
    modelCapability: "configurable",
  },
] as const;

export const PROMPT_ONLY_DIRECTORY = [
  { promptAppId: "gemini", displayName: "Gemini" },
  { promptAppId: "openclaw", displayName: "OpenClaw" },
  { promptAppId: "hermes", displayName: "Hermes" },
] as const;

export const AGENT_CATALOG_IDS = [
  "qoderwork",
  "trae-work",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude-code",
  "opencode",
  "omp",
] as const;

/**
 * UI display order: configurable self-owned model clients first, official-only
 * vendor-locked clients (`qoderwork`, `trae-work`) muted at the bottom.
 */
export const AGENT_DISPLAY_ORDER = [
  "workbuddy",
  "codex",
  "claude-code",
  "opencode",
  "omp",
  "grokbuild",
  "qoderwork",
  "trae-work",
] as const satisfies readonly AgentCatalogId[];

export const MCP_TARGET_IDS = [
  "qoderwork",
  "trae-work",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude",
  "opencode",
] as const;

export const MODEL_DIRECTORY_IDS = [
  "workbuddy",
  "codex",
  "claude",
  "opencode",
  "omp",
  "grokbuild",
  "qoderwork",
  "trae",
] as const;

export const PROMPT_APP_IDS = [
  "grokbuild",
  "codex",
  "claude",
  "opencode",
  "gemini",
  "openclaw",
  "hermes",
] as const;

export type ProductDirectoryEntry = (typeof PRODUCT_DIRECTORY)[number];
export type AgentCatalogId = (typeof AGENT_CATALOG_IDS)[number];

export function agentDirectoryPriority(
  agentId: AgentCatalogId,
): AgentDirectoryPriority {
  const entry = PRODUCT_DIRECTORY.find((item) => item.agentId === agentId);
  return entry?.directoryPriority ?? "standard";
}

export function isOfficialOnlyAgent(agentId: AgentCatalogId): boolean {
  const entry = PRODUCT_DIRECTORY.find((item) => item.agentId === agentId);
  return entry?.modelCapability === "official-only";
}

export type McpTargetId = (typeof MCP_TARGET_IDS)[number];
export type SkillTargetId = McpTargetId;
export type ModelDirectoryId = (typeof MODEL_DIRECTORY_IDS)[number];
export type PromptAppId = (typeof PROMPT_APP_IDS)[number];

export function isOfficialOnlyModelTarget(target: ModelDirectoryId): boolean {
  const entry = PRODUCT_DIRECTORY.find((item) => item.modelTarget === target);
  return entry?.modelCapability === "official-only";
}

export const SKILL_TARGET_IDS = MCP_TARGET_IDS;
export const SUPPORTED_APP_IDS = MCP_TARGET_IDS;

export const MCP_TARGETS: ReadonlyArray<{
  id: McpTargetId;
  label: string;
}> = PRODUCT_DIRECTORY.flatMap((entry) =>
  entry.assignmentId === null
    ? []
    : [{ id: entry.assignmentId, label: entry.displayName }],
);

export const SKILL_TARGETS = MCP_TARGETS;
export const SUPPORTED_APPS = MCP_TARGETS;

export const AGENT_VARIANT_IDS = [
  "qoderwork-cn",
  "trae-work-cn",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude-code",
  "opencode",
  "omp",
] as const;

export type AgentVariantId = (typeof AGENT_VARIANT_IDS)[number];
