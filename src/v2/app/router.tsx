import { lazy, Suspense, type ReactNode } from "react";
import { Navigate, createHashRouter, type RouteObject } from "react-router-dom";

import {
  navigationItems,
  type NavigationItem,
} from "../shared/config/navigation";
import { FeatureProvider } from "../shared/features/provider";
import { AppShell } from "../widgets/app-shell/AppShell";
import { PersistentPrimaryOutlet } from "./PersistentPrimaryOutlet";
import { RootError } from "./RootError";

const AgentsPage = lazy(async () => ({
  default: (await import("../pages/agents/Page")).AgentsPage,
}));
const ModelsPage = lazy(async () => ({
  default: (await import("../pages/models/Page")).ModelsPage,
}));
const SkillsPage = lazy(async () => ({
  default: (await import("../pages/skills/Page")).SkillsPage,
}));
const McpPage = lazy(async () => ({
  default: (await import("../pages/mcp/Page")).McpPage,
}));
const PromptsPage = lazy(async () => ({
  default: (await import("../pages/prompts/Page")).PromptsPage,
}));
const MemoryPage = lazy(async () => ({
  default: (await import("../pages/memory/Page")).MemoryPage,
}));

function lazyPage(page: ReactNode): ReactNode {
  return (
    <Suspense
      fallback={
        <div className="fy-feature-route-loading" role="status">
          正在加载页面
        </div>
      }
    >
      {page}
    </Suspense>
  );
}

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

const primaryPages: Record<NavigationItem["id"], ReactNode> = {
  agents: <AgentsPage />,
  models: <ModelsPage />,
  skills: <SkillsPage />,
  mcp: <McpPage />,
  prompts: <PromptsPage />,
  memory: <MemoryPage />,
};

const primaryRoutes: RouteObject[] = navigationItems.map((item) => ({
  path: item.path.slice(1),
  element: lazyPage(primaryPages[item.id]),
}));

export const appRoutes: RouteObject[] = [
  {
    path: "/",
    element: (
      <FeatureProvider>
        <AppShell />
      </FeatureProvider>
    ),
    children: [
      {
        errorElement: <RootError />,
        children: [
          { index: true, element: <Navigate to="/agents" replace /> },
          {
            element: <PersistentPrimaryOutlet />,
            children: [
              ...primaryRoutes,
              ...developmentRoutes,
              { path: "*", element: <Navigate to="/agents" replace /> },
            ],
          },
        ],
      },
    ],
  },
];

export function createAppRouter(): ReturnType<typeof createHashRouter> {
  return createHashRouter(appRoutes);
}
