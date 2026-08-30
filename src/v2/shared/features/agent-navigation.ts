import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export const AGENT_SECTION_IDS = [
  "models",
  "skills",
  "mcp",
  "prompts",
] as const;

export type AgentSection = (typeof AGENT_SECTION_IDS)[number];

const AGENT_RETURN_STATE_KEY = "fyagentAgentReturn";

export type AgentReturnLocationState = Readonly<{
  [AGENT_RETURN_STATE_KEY]: Readonly<{
    agentId: AgentCatalogId;
    section: AgentSection;
  }>;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isAgentCatalogId(value: unknown): value is AgentCatalogId {
  return (
    typeof value === "string" &&
    AGENT_CATALOG_IDS.includes(value as AgentCatalogId)
  );
}

function isAgentSection(value: unknown): value is AgentSection {
  return (
    typeof value === "string" &&
    AGENT_SECTION_IDS.includes(value as AgentSection)
  );
}

export function agentReturnPathFromSearch(search: string): string | null {
  const params = new URLSearchParams(search);
  const keys = [...params.keys()];
  if (
    keys.length !== 2 ||
    new Set(keys).size !== 2 ||
    !keys.includes("target") ||
    !keys.includes("section")
  ) {
    return null;
  }
  const agentId = params.get("target");
  const section = params.get("section");
  if (!isAgentCatalogId(agentId) || !isAgentSection(section)) {
    return null;
  }
  return `/agents?target=${encodeURIComponent(agentId)}&section=${encodeURIComponent(section)}`;
}

export function createAgentReturnLocationState(
  agentId: AgentCatalogId,
  section: AgentSection,
): AgentReturnLocationState {
  return {
    [AGENT_RETURN_STATE_KEY]: { agentId, section },
  };
}

/**
 * Derives the return URL from a closed, non-secret tuple. Router state never
 * supplies an arbitrary path or command string.
 */
export function agentReturnPathFromLocationState(
  value: unknown,
): string | null {
  if (!isRecord(value)) return null;
  const descriptor = value[AGENT_RETURN_STATE_KEY];
  if (!isRecord(descriptor)) return null;
  if (
    !isAgentCatalogId(descriptor.agentId) ||
    !isAgentSection(descriptor.section)
  ) {
    return null;
  }
  return `/agents?target=${encodeURIComponent(descriptor.agentId)}&section=${encodeURIComponent(descriptor.section)}`;
}
