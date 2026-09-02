import { keepPreviousData, useQueries, useQuery } from "@tanstack/react-query";

import { usePersistentVisibility } from "../ui/PersistentSurface";
import { useFeatures } from "./provider";
import {
  PROMPT_APP_IDS,
  SKILL_DISCOVERY_PAGE_SIZE,
  SKILLHUB_CATEGORY_ALL,
  type AgentCatalogId,
  type MemoryDocumentId,
  type PromptAppId,
  type SkillDiscoveryStatus,
  type SkillHubCategoryFilter,
} from "./types";
import type { ProviderAppId } from "./types";

function useVisibleEnabled(enabled = true): boolean {
  const visible = usePersistentVisibility();
  return enabled && visible;
}

export const featureKeys = {
  agentCatalog: ["v2", "agents", "catalog"] as const,
  agentAuthObservation: (agentId: AgentCatalogId) =>
    ["v2", "agents", agentId, "auth-observation"] as const,
  agentInstallReadiness: (agentId: AgentCatalogId) =>
    ["v2", "agents", agentId, "install-readiness"] as const,
  agentInstallationInventory: (agentId: AgentCatalogId) =>
    ["v2", "agents", agentId, "installation-inventory"] as const,
  recoverableChangeJobs: ["v2", "change-plans", "recoverable"] as const,
  providerSummary: (app: ProviderAppId) =>
    ["v2", "providers", app, "summary"] as const,
  workbuddyStatus: ["v2", "workbuddy", "status"] as const,
  workbuddyModelIds: ["v2", "workbuddy", "model-ids"] as const,
  traeWorkModelIds: ["v2", "trae-work", "model-ids"] as const,
  openCodeModelSnapshot: ["v2", "opencode", "model-snapshot"] as const,
  skills: ["v2", "skills", "installed"] as const,
  skillBackups: ["v2", "skills", "backups"] as const,
  skillDiscovery: ["v2", "skills", "discovery"] as const,
  skillUnmanaged: ["v2", "skills", "unmanaged"] as const,
  skillUpdates: ["v2", "skills", "updates"] as const,
  mcp: ["v2", "mcp"] as const,
  prompts: (app: PromptAppId) => ["v2", "prompts", app, "list"] as const,
  promptLiveFile: (app: PromptAppId) =>
    ["v2", "prompts", app, "live-file"] as const,
  memoryDocument: (id: MemoryDocumentId) =>
    ["v2", "memory", "document", id] as const,
  hermesMemoryLimits: ["v2", "memory", "hermes-limits"] as const,
  dailyMemoryFiles: ["v2", "memory", "daily", "list"] as const,
  dailyMemoryFile: (filename: string) =>
    ["v2", "memory", "daily", "file", filename] as const,
  dailyMemorySearch: (query: string) =>
    ["v2", "memory", "daily", "search", query] as const,
  settings: ["v2", "settings"] as const,
};

export function useAgentCatalog(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.agentCatalog,
    queryFn: ports.catalog.get,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useAgentInstallReadiness(
  agentId: AgentCatalogId,
  enabled = true,
) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.agentInstallReadiness(agentId),
    queryFn: () => ports.agentInstallReadiness.get(agentId),
    enabled: useVisibleEnabled(enabled),
  });
}

export function useAgentAuthObservation(
  agentId: AgentCatalogId,
  enabled = true,
) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.agentAuthObservation(agentId),
    queryFn: () => ports.agentAuth.getObservation(agentId),
    enabled: useVisibleEnabled(enabled),
  });
}

export function useAgentInstallationInventory(
  agentId: AgentCatalogId,
  enabled = true,
) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.agentInstallationInventory(agentId),
    queryFn: () => ports.agentInstallReadiness.getInventory(agentId),
    enabled: useVisibleEnabled(enabled),
  });
}

export function useRecoverableChangeJobs(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.recoverableChangeJobs,
    queryFn: ports.changePlans.listRecoverableChangeJobs,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useProviderSummary(app: ProviderAppId, enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.providerSummary(app),
    queryFn: () => ports.providers.getSummary(app),
    enabled: useVisibleEnabled(enabled),
  });
}

export function useWorkBuddyStatus(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.workbuddyStatus,
    queryFn: ports.workbuddy.getStatus,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useWorkBuddyModelIds(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.workbuddyModelIds,
    queryFn: ports.workbuddy.getModelIds,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useTraeWorkModelIds(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.traeWorkModelIds,
    queryFn: ports.traeWork.getModelIds,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useOpenCodeModelSnapshot(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.openCodeModelSnapshot,
    queryFn: ports.opencodeModels.getSnapshot,
    enabled: useVisibleEnabled(enabled),
  });
}

export function useInstalledSkills() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skills,
    queryFn: ports.skills.getInstalled,
    enabled: useVisibleEnabled(),
  });
}
export function useSkillBackups(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillBackups,
    queryFn: ports.skills.getBackups,
    enabled: useVisibleEnabled(enabled),
  });
}
export function useSkillDiscoveryPage(
  query: string,
  repo: string | undefined,
  status: SkillDiscoveryStatus,
  page: number,
  enabled = true,
) {
  const { ports } = useFeatures();
  const limit = SKILL_DISCOVERY_PAGE_SIZE;
  const offset = Math.max(0, page - 1) * limit;
  return useQuery({
    queryKey: [
      ...featureKeys.skillDiscovery,
      query,
      repo ?? "all",
      status,
      page,
    ],
    queryFn: () =>
      ports.skills.discoverPage({
        query,
        repo,
        status,
        limit,
        offset,
      }),
    enabled: useVisibleEnabled(enabled),
    placeholderData: keepPreviousData,
  });
}
export function useUnmanagedSkills(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillUnmanaged,
    queryFn: ports.skills.scanUnmanaged,
    enabled: useVisibleEnabled(enabled),
  });
}
export function useSkillUpdates(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillUpdates,
    queryFn: ports.skills.checkUpdates,
    enabled: useVisibleEnabled(enabled),
  });
}
export function useSkillHubSearch(
  query: string,
  page: number,
  category: SkillHubCategoryFilter,
  enabled: boolean,
) {
  const { ports } = useFeatures();
  const limit = SKILL_DISCOVERY_PAGE_SIZE;
  const categoryKey = category === SKILLHUB_CATEGORY_ALL ? "" : category;
  return useQuery({
    queryKey: [
      ...featureKeys.skillDiscovery,
      "skillhub",
      query,
      category,
      page,
    ],
    queryFn: async () => {
      const load = (nextPage: number) =>
        ports.skills.searchSkillHub(
          query,
          limit,
          Math.max(0, nextPage - 1) * limit,
          categoryKey,
        );
      const result = await load(page);
      const totalPages = Math.max(1, Math.ceil(result.totalCount / limit));
      if (page <= 1 || page <= totalPages) return result;
      return load(totalPages);
    },
    enabled: useVisibleEnabled(enabled),
    placeholderData: keepPreviousData,
  });
}
export function useMcpServers() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.mcp,
    queryFn: ports.mcp.getAll,
    enabled: useVisibleEnabled(),
  });
}
export function usePrompts(app: PromptAppId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.prompts(app),
    queryFn: () => ports.prompts.getAll(app),
    enabled: useVisibleEnabled(),
  });
}
export function usePromptLibraries() {
  const { ports } = useFeatures();
  const enabled = useVisibleEnabled();
  return useQueries({
    queries: PROMPT_APP_IDS.map((app) => ({
      queryKey: featureKeys.prompts(app),
      queryFn: () => ports.prompts.getAll(app),
      enabled,
    })),
  });
}
export function usePromptLiveFile(app: PromptAppId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.promptLiveFile(app),
    queryFn: () => ports.prompts.getCurrentFileContent(app),
    enabled: useVisibleEnabled(),
  });
}
export function useMemoryDocument(id: MemoryDocumentId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.memoryDocument(id),
    queryFn: () => ports.memory.readDocument(id),
    enabled: useVisibleEnabled(),
  });
}
export function useHermesMemoryLimits() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.hermesMemoryLimits,
    queryFn: ports.memory.getHermesLimits,
    enabled: useVisibleEnabled(),
  });
}
export function useDailyMemoryFiles() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemoryFiles,
    queryFn: ports.memory.listDailyFiles,
    enabled: useVisibleEnabled(),
  });
}
export function useDailyMemoryFile(filename: string | null) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemoryFile(filename ?? ""),
    queryFn: () => ports.memory.readDailyFile(filename ?? ""),
    enabled: useVisibleEnabled(filename !== null),
  });
}
export function useDailyMemorySearch(query: string) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemorySearch(query),
    queryFn: () => ports.memory.searchDailyFiles(query),
    enabled: useVisibleEnabled(query.length > 0),
  });
}
export function useFeatureSettings(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.settings,
    queryFn: ports.settings.get,
    enabled: useVisibleEnabled(enabled),
  });
}
