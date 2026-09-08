export type NavigationItem = {
  id: "agents" | "auth" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path:
    | "/agents"
    | "/auth"
    | "/models"
    | "/skills"
    | "/mcp"
    | "/prompts"
    | "/memory";
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
    items: [
      { id: "agents", path: "/agents", label: "AI软件配置" },
      { id: "auth", path: "/auth", label: "账号与认证" },
    ],
  },
  {
    id: "configuration-management",
    label: "配置管理",
    collapsible: true,
    items: [
      { id: "models", path: "/models", label: "模型管理" },
      { id: "skills", path: "/skills", label: "Skills 管理" },
      { id: "mcp", path: "/mcp", label: "MCP 管理" },
      { id: "prompts", path: "/prompts", label: "提示词管理" },
    ],
  },
  {
    id: "memory",
    label: "记忆模块",
    collapsible: false,
    items: [{ id: "memory", path: "/memory", label: "记忆模块" }],
  },
] as const satisfies readonly NavigationGroup[];

export const navigationItems: readonly NavigationItem[] =
  navigationGroups.flatMap<NavigationItem>((group) => group.items);
