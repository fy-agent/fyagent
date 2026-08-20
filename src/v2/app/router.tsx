import { Navigate, createHashRouter, type RouteObject } from "react-router-dom";

import { FeatureProvider } from "../shared/features/provider";
import { AppShell } from "../widgets/app-shell/AppShell";
import { PersistentPrimaryOutlet } from "./PersistentPrimaryOutlet";
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
    element: (
      <FeatureProvider>
        <AppShell />
      </FeatureProvider>
    ),
    children: [
      {
        errorElement: <RootError />,
        children: [
          { index: true, element: <Navigate to="/models" replace /> },
          {
            element: <PersistentPrimaryOutlet />,
            children: [
              { path: "agents", element: null },
              { path: "models", element: null },
              { path: "skills", element: null },
              { path: "mcp", element: null },
              { path: "prompts", element: null },
              { path: "memory", element: null },
              ...developmentRoutes,
              { path: "*", element: <Navigate to="/models" replace /> },
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
