export type NavigationItem = {
  id: "agents" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path: "/agents" | "/models" | "/skills" | "/mcp" | "/prompts" | "/memory";
  label: string;
};

export type NavigationGroup = {
  id: "agent-configuration" | "configuration-management" | "memory";
  label: string;
  collapsible: boolean;
  items: readonly NavigationItem[];
};

export const navigationGroups = [
  {
    id: "agent-configuration",
    label: "AI软件配置",
    collapsible: false,
    items: [{ id: "agents", path: "/agents", label: "Agent 目录" }],
  },
  {
    id: "configuration-management",
    label: "配置管理",
    collapsible: true,
    items: [
      { id: "models", path: "/models", label: "模型" },
      { id: "skills", path: "/skills", label: "Skills" },
      { id: "mcp", path: "/mcp", label: "MCP" },
      { id: "prompts", path: "/prompts", label: "提示词" },
    ],
  },
  {
    id: "memory",
    label: "记忆模块",
    collapsible: false,
    items: [{ id: "memory", path: "/memory", label: "记忆" }],
  },
] as const satisfies readonly NavigationGroup[];

export const navigationItems: readonly NavigationItem[] =
  navigationGroups.flatMap<NavigationItem>((group) => group.items);
