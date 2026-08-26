export const AGENT_SECTION_IDS = [
  "models",
  "skills",
  "mcp",
  "prompts",
] as const;

export type AgentSection = (typeof AGENT_SECTION_IDS)[number];
