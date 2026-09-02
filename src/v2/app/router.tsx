import { Navigate, createHashRouter, type RouteObject } from "react-router-dom";

import { navigationItems } from "../shared/config/navigation";
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

function PrimaryRouteMatch() {
  return null;
}

const primaryRoutes: RouteObject[] = navigationItems.map((item) => ({
  path: item.path.slice(1),
  Component: PrimaryRouteMatch,
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
