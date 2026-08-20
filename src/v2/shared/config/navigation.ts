export type NavigationItem = {
  id: "agents" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path: "/agents" | "/models" | "/skills" | "/mcp" | "/prompts" | "/memory";
  label: string;
};

export const navigationItems = [
  { id: "agents", path: "/agents", label: "Agent 目录" },
  { id: "models", path: "/models", label: "模型" },
  { id: "skills", path: "/skills", label: "Skills" },
  { id: "mcp", path: "/mcp", label: "MCP" },
  { id: "prompts", path: "/prompts", label: "提示词" },
  { id: "memory", path: "/memory", label: "记忆" },
] as const satisfies readonly NavigationItem[];
