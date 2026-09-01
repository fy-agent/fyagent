export type AgentDirectoryPriority = "domestic" | "standard";

export const PRODUCT_DIRECTORY = [
  {
    agentId: "qoderwork",
    assignmentId: "qoderwork",
    modelTarget: "qoderwork",
    promptAppId: null,
    displayName: "QoderWork CN",
    directoryPriority: "domestic",
  },
  {
    agentId: "trae-work",
    assignmentId: "trae-work",
    modelTarget: "trae",
    promptAppId: null,
    displayName: "TRAE Work CN",
    directoryPriority: "domestic",
  },
  {
    agentId: "workbuddy",
    assignmentId: "workbuddy",
    modelTarget: "workbuddy",
    promptAppId: null,
    displayName: "WorkBuddy",
    directoryPriority: "domestic",
  },
  {
    agentId: "grokbuild",
    assignmentId: "grokbuild",
    modelTarget: "grokbuild",
    promptAppId: "grokbuild",
    displayName: "Grok Build",
    directoryPriority: "standard",
  },
  {
    agentId: "codex",
    assignmentId: "codex",
    modelTarget: "codex",
    promptAppId: "codex",
    displayName: "Codex",
    directoryPriority: "standard",
  },
  {
    agentId: "claude-code",
    assignmentId: "claude",
    modelTarget: "claude",
    promptAppId: "claude",
    displayName: "Claude Code",
    directoryPriority: "standard",
  },
  {
    agentId: "opencode",
    assignmentId: "opencode",
    modelTarget: "opencode",
    promptAppId: "opencode",
    displayName: "OpenCode",
    directoryPriority: "standard",
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
] as const;

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
  "qoderwork",
  "trae",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude",
  "opencode",
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

export type McpTargetId = (typeof MCP_TARGET_IDS)[number];
export type SkillTargetId = McpTargetId;
export type ModelDirectoryId = (typeof MODEL_DIRECTORY_IDS)[number];
export type PromptAppId = (typeof PROMPT_APP_IDS)[number];

export const SKILL_TARGET_IDS = MCP_TARGET_IDS;
export const SUPPORTED_APP_IDS = MCP_TARGET_IDS;

export const MCP_TARGETS: ReadonlyArray<{
  id: McpTargetId;
  label: string;
}> = PRODUCT_DIRECTORY.map((entry) => ({
  id: entry.assignmentId,
  label: entry.displayName,
}));

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
] as const;

export type AgentVariantId = (typeof AGENT_VARIANT_IDS)[number];
