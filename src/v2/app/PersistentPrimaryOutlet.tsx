import { useState, type ComponentType } from "react";
import { Outlet, useLocation } from "react-router-dom";

import { AgentsPage } from "../pages/agents/Page";
import { McpPage } from "../pages/mcp/Page";
import { MemoryPage } from "../pages/memory/Page";
import { ModelsPage } from "../pages/models/Page";
import { PromptsPage } from "../pages/prompts/Page";
import { SkillsPage } from "../pages/skills/Page";
import {
  navigationItems,
  type NavigationItem,
} from "../shared/config/navigation";
import { PersistentSurface } from "../shared/ui/PersistentSurface";

const primaryPages: Record<NavigationItem["id"], ComponentType> = {
  agents: AgentsPage,
  models: ModelsPage,
  skills: SkillsPage,
  mcp: McpPage,
  prompts: PromptsPage,
  memory: MemoryPage,
};

export function PersistentPrimaryOutlet() {
  const { pathname } = useLocation();
  const current = navigationItems.find((item) => item.path === pathname);
  const [visited, setVisited] = useState<ReadonlySet<NavigationItem["id"]>>(
    () => (current ? new Set([current.id]) : new Set()),
  );

  if (current && !visited.has(current.id)) {
    const next = new Set(visited);
    next.add(current.id);
    setVisited(next);
  }

  return (
    <>
      {navigationItems.map((item) => {
        if (!visited.has(item.id)) return null;
        const Page = primaryPages[item.id];
        return (
          <PersistentSurface key={item.id} active={item.path === pathname}>
            <Page />
          </PersistentSurface>
        );
      })}
      {current ? null : <Outlet />}
    </>
  );
}
