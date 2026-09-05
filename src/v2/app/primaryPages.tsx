import { lazy, type ComponentType, type LazyExoticComponent } from "react";

import {
  navigationItems,
  type NavigationItem,
} from "../shared/config/navigation";

type PrimaryPageModule = { default: ComponentType };

function cachedLoader(
  load: () => Promise<PrimaryPageModule>,
): () => Promise<PrimaryPageModule> {
  let promise: Promise<PrimaryPageModule> | null = null;
  return () => {
    promise ??= load().catch((error: unknown) => {
      promise = null;
      throw error;
    });
    return promise;
  };
}

const primaryPageLoaders = {
  agents: cachedLoader(async () => ({
    default: (await import("../pages/agents/Page")).AgentsPage,
  })),
  auth: cachedLoader(async () => ({
    default: (await import("../pages/auth/Page")).AuthPage,
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
  auth: lazy(primaryPageLoaders.auth),
  models: lazy(primaryPageLoaders.models),
  skills: lazy(primaryPageLoaders.skills),
  mcp: lazy(primaryPageLoaders.mcp),
  prompts: lazy(primaryPageLoaders.prompts),
  memory: lazy(primaryPageLoaders.memory),
};

export function prefetchPrimaryRoutes(): void {
  for (const load of Object.values(primaryPageLoaders)) {
    // Prefetch is optional. A failed warm-up must not poison a later visit
    // or create an unhandled rejection; actual navigation still shows errors.
    void load().catch(() => undefined);
  }
}

export function initialPrimaryPageId(hash: string): NavigationItem["id"] {
  const pathname = hash.replace(/^#/, "").split(/[?#]/, 1)[0];
  return navigationItems.find((item) => item.path === pathname)?.id ?? "agents";
}

export async function preloadInitialPrimaryRoute(hash: string): Promise<void> {
  await primaryPageLoaders[initialPrimaryPageId(hash)]();
}
