import { keepPreviousData, useQueries, useQuery } from "@tanstack/react-query";

import { useFeatures } from "./provider";
import {
  PROMPT_APP_IDS,
  SKILL_DISCOVERY_PAGE_SIZE,
  type MemoryDocumentId,
  type PromptAppId,
  type SkillDiscoveryStatus,
} from "./types";
import type { ProviderAppId } from "./types";

export const featureKeys = {
  agentCatalog: ["v2", "agents", "catalog"] as const,
  providerSummary: (app: ProviderAppId) =>
    ["v2", "providers", app, "summary"] as const,
  workbuddyStatus: ["v2", "workbuddy", "status"] as const,
  workbuddyModelIds: ["v2", "workbuddy", "model-ids"] as const,
  traeWorkModelIds: ["v2", "trae-work", "model-ids"] as const,
  openCodeModelSnapshot: ["v2", "opencode", "model-snapshot"] as const,
  skills: ["v2", "skills", "installed"] as const,
  skillBackups: ["v2", "skills", "backups"] as const,
  skillRepos: ["v2", "skills", "repos"] as const,
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
    enabled,
  });
}

export function useProviderSummary(app: ProviderAppId, enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.providerSummary(app),
    queryFn: () => ports.providers.getSummary(app),
    enabled,
  });
}

export function useWorkBuddyStatus(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.workbuddyStatus,
    queryFn: ports.workbuddy.getStatus,
    enabled,
  });
}

export function useWorkBuddyModelIds(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.workbuddyModelIds,
    queryFn: ports.workbuddy.getModelIds,
    enabled,
  });
}

export function useTraeWorkModelIds(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.traeWorkModelIds,
    queryFn: ports.traeWork.getModelIds,
    enabled,
  });
}

export function useOpenCodeModelSnapshot(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.openCodeModelSnapshot,
    queryFn: ports.opencodeModels.getSnapshot,
    enabled,
  });
}

export function useInstalledSkills() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skills,
    queryFn: ports.skills.getInstalled,
  });
}
export function useSkillBackups(enabled = true) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillBackups,
    queryFn: ports.skills.getBackups,
    enabled,
  });
}
export function useSkillRepos() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillRepos,
    queryFn: ports.skills.getRepos,
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
    enabled,
    placeholderData: keepPreviousData,
  });
}
export function useUnmanagedSkills(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillUnmanaged,
    queryFn: ports.skills.scanUnmanaged,
    enabled,
  });
}
export function useSkillUpdates(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.skillUpdates,
    queryFn: ports.skills.checkUpdates,
    enabled,
  });
}
export function useSkillHubSearch(
  query: string,
  page: number,
  enabled: boolean,
) {
  const { ports } = useFeatures();
  const limit = SKILL_DISCOVERY_PAGE_SIZE;
  return useQuery({
    queryKey: ["v2", "skills", "skillhub", query, page],
    queryFn: async () => {
      const load = (nextPage: number) =>
        ports.skills.searchSkillHub(
          query,
          limit,
          Math.max(0, nextPage - 1) * limit,
        );
      const result = await load(page);
      const totalPages = Math.max(1, Math.ceil(result.totalCount / limit));
      if (page <= 1 || page <= totalPages) return result;
      return load(totalPages);
    },
    enabled,
    placeholderData: keepPreviousData,
  });
}
export function useMcpServers() {
  const { ports } = useFeatures();
  return useQuery({ queryKey: featureKeys.mcp, queryFn: ports.mcp.getAll });
}
export function usePrompts(app: PromptAppId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.prompts(app),
    queryFn: () => ports.prompts.getAll(app),
  });
}
export function usePromptLibraries() {
  const { ports } = useFeatures();
  return useQueries({
    queries: PROMPT_APP_IDS.map((app) => ({
      queryKey: featureKeys.prompts(app),
      queryFn: () => ports.prompts.getAll(app),
    })),
  });
}
export function usePromptLiveFile(app: PromptAppId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.promptLiveFile(app),
    queryFn: () => ports.prompts.getCurrentFileContent(app),
  });
}
export function useMemoryDocument(id: MemoryDocumentId) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.memoryDocument(id),
    queryFn: () => ports.memory.readDocument(id),
  });
}
export function useHermesMemoryLimits() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.hermesMemoryLimits,
    queryFn: ports.memory.getHermesLimits,
  });
}
export function useDailyMemoryFiles() {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemoryFiles,
    queryFn: ports.memory.listDailyFiles,
  });
}
export function useDailyMemoryFile(filename: string | null) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemoryFile(filename ?? ""),
    queryFn: () => ports.memory.readDailyFile(filename ?? ""),
    enabled: filename !== null,
  });
}
export function useDailyMemorySearch(query: string) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.dailyMemorySearch(query),
    queryFn: () => ports.memory.searchDailyFiles(query),
    enabled: query.length > 0,
  });
}
export function useFeatureSettings(enabled = false) {
  const { ports } = useFeatures();
  return useQuery({
    queryKey: featureKeys.settings,
    queryFn: ports.settings.get,
    enabled,
  });
}
