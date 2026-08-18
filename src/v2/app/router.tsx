import { Navigate, createHashRouter, type RouteObject } from "react-router-dom";

import { AgentsPage } from "../pages/agents/Page";
import { McpPage } from "../pages/mcp/Page";
import { MemoryPage } from "../pages/memory/Page";
import { ModelsPage } from "../pages/models/Page";
import { PromptsPage } from "../pages/prompts/Page";
import { SkillsPage } from "../pages/skills/Page";
import { AppShell } from "../widgets/app-shell/AppShell";
import { RootError } from "./RootError";

const developmentRoutes: RouteObject[] = import.meta.env.DEV
  ? [
      {
        path: "__dev/ui-lab",
        lazy: async () => {
          const { UiLabPage } = await import("../dev/UiLabPage");

          return { Component: UiLabPage };
        },
      },
    ]
  : [];

export const appRoutes: RouteObject[] = [
  {
    path: "/",
    element: <AppShell />,
    children: [
      {
        errorElement: <RootError />,
        children: [
          { index: true, element: <Navigate to="/models" replace /> },
          { path: "agents", element: <AgentsPage /> },
          { path: "models", element: <ModelsPage /> },
          { path: "skills", element: <SkillsPage /> },
          { path: "mcp", element: <McpPage /> },
          { path: "prompts", element: <PromptsPage /> },
          { path: "memory", element: <MemoryPage /> },
          ...developmentRoutes,
          { path: "*", element: <Navigate to="/models" replace /> },
        ],
      },
    ],
  },
];

export function createAppRouter(): ReturnType<typeof createHashRouter> {
  return createHashRouter(appRoutes);
}
