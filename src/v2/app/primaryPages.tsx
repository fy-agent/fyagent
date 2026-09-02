import { lazy, type ComponentType, type LazyExoticComponent } from "react";

import type { NavigationItem } from "../shared/config/navigation";

type PrimaryPageModule = { default: ComponentType };

function cachedLoader(
  load: () => Promise<PrimaryPageModule>,
): () => Promise<PrimaryPageModule> {
  let promise: Promise<PrimaryPageModule> | null = null;
  return () => {
    promise ??= load();
    return promise;
  };
}

const primaryPageLoaders = {
  agents: cachedLoader(async () => ({
    default: (await import("../pages/agents/Page")).AgentsPage,
  })),
  models: cachedLoader(async () => ({
    default: (await import("../pages/models/Page")).ModelsPage,
  })),
  skills: cachedLoader(async () => ({
    default: (await import("../pages/skills/Page")).SkillsPage,
  })),
  mcp: cachedLoader(async () => ({
    default: (await import("../pages/mcp/Page")).McpPage,
  })),
  prompts: cachedLoader(async () => ({
    default: (await import("../pages/prompts/Page")).PromptsPage,
  })),
  memory: cachedLoader(async () => ({
    default: (await import("../pages/memory/Page")).MemoryPage,
  })),
} satisfies Record<
  NavigationItem["id"],
  () => Promise<{ default: ComponentType }>
>;

export const primaryPages: Record<
  NavigationItem["id"],
  LazyExoticComponent<ComponentType>
> = {
  agents: lazy(primaryPageLoaders.agents),
  models: lazy(primaryPageLoaders.models),
  skills: lazy(primaryPageLoaders.skills),
  mcp: lazy(primaryPageLoaders.mcp),
  prompts: lazy(primaryPageLoaders.prompts),
  memory: lazy(primaryPageLoaders.memory),
};

export function prefetchPrimaryRoutes(): void {
  for (const load of Object.values(primaryPageLoaders)) {
    void load();
  }
}
