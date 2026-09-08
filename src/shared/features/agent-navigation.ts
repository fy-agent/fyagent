import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export const AGENT_SECTION_IDS = [
  "models",
  "skills",
  "mcp",
  "prompts",
] as const;

export type AgentSection = (typeof AGENT_SECTION_IDS)[number];

const AGENT_RETURN_ID_PARAM = "agentReturn";
const AGENT_RETURN_SECTION_PARAM = "agentSection";

export type AgentReturnDescriptor = Readonly<{
  agentId: AgentCatalogId;
  section: AgentSection;
}>;

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

export function agentReturnPath(descriptor: AgentReturnDescriptor): string {
  return `/agents?target=${encodeURIComponent(descriptor.agentId)}&section=${encodeURIComponent(descriptor.section)}`;
}

export function agentReturnDescriptorFromSearch(
  search: string,
): AgentReturnDescriptor | null {
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
  return { agentId, section };
}

export function agentReturnPathFromSearch(search: string): string | null {
  const descriptor = agentReturnDescriptorFromSearch(search);
  return descriptor ? agentReturnPath(descriptor) : null;
}

export function agentReturnDescriptorFromManagementSearch(
  search: string,
): AgentReturnDescriptor | null {
  const params = new URLSearchParams(search);
  const agentIds = params.getAll(AGENT_RETURN_ID_PARAM);
  const sections = params.getAll(AGENT_RETURN_SECTION_PARAM);
  if (agentIds.length !== 1 || sections.length !== 1) {
    return null;
  }
  const agentId = agentIds[0];
  const section = sections[0];
  if (!isAgentCatalogId(agentId) || !isAgentSection(section)) {
    return null;
  }
  return { agentId, section };
}

/**
 * Carries only the closed, non-secret Agent/section tuple into another
 * internal management route. Existing route-owned query fields are retained;
 * no caller-provided return path is accepted.
 */
export function appendAgentReturnToPath(
  path: string,
  descriptor: AgentReturnDescriptor,
): string {
  const queryIndex = path.indexOf("?");
  const pathname = queryIndex < 0 ? path : path.slice(0, queryIndex);
  const search = queryIndex < 0 ? "" : path.slice(queryIndex + 1);
  const params = new URLSearchParams(search);
  params.set(AGENT_RETURN_ID_PARAM, descriptor.agentId);
  params.set(AGENT_RETURN_SECTION_PARAM, descriptor.section);
  return `${pathname}?${params.toString()}`;
}
