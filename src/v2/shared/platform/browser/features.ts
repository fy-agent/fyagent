import type { FeaturePorts } from "../../features/ports";

export const NATIVE_ONLY_ERROR = "此操作仅在 FyAgent 桌面应用中可用";

const rejectNativeOnly = async (): Promise<never> => {
  throw new Error(NATIVE_ONLY_ERROR);
};

export function createBrowserFeaturePorts(): FeaturePorts {
  return {
    // The native command is the only Agent capability authority. Browser
    // preview renders the controlled unavailable state instead of carrying a
    // second capability matrix that could drift into a support claim.
    catalog: {
      get: rejectNativeOnly,
    },
    agentInstallReadiness: {
      get: rejectNativeOnly,
    },
    changePlans: {
      createCodexProviderSwitchPlan: rejectNativeOnly,
      createCodexProviderUpsertPlan: rejectNativeOnly,
      createWorkBuddySavePlan: rejectNativeOnly,
      applyChangePlan: rejectNativeOnly,
      cancelChangeJob: rejectNativeOnly,
      getChangeJob: rejectNativeOnly,
      listRecoverableChangeJobs: rejectNativeOnly,
    },
    externalAgents: {
      getStatus: async (agentId) => ({
        agentId,
        detected: null,
        running: null,
        version: null,
        installSource: null,
        capabilities: [
          {
            id: "app.detect",
            state: "unverified",
            reasonCode: "trusted_runtime_identity_unavailable",
          },
          {
            id: "app.launch",
            state: "unverified",
            reasonCode: "trusted_runtime_identity_unavailable",
          },
        ],
      }),
      launch: async (agentId, destination) => ({
        agentId,
        destination,
        state: "unverified",
        reasonCode: "trusted_runtime_identity_unavailable",
      }),
    },
    qoderwork: {
      getHooks: rejectNativeOnly,
      saveHooks: rejectNativeOnly,
    },
    externalMcp: {
      validate: rejectNativeOnly,
    },
    traeWork: {
      validateModelConfig: rejectNativeOnly,
      testModelEndpoint: rejectNativeOnly,
      cancelModelEndpoint: rejectNativeOnly,
      getModelIds: rejectNativeOnly,
    },
    codexDesktop: {
      getLocalStatus: rejectNativeOnly,
      checkLatest: rejectNativeOnly,
      getJob: rejectNativeOnly,
      startInstall: rejectNativeOnly,
      cancelInstall: rejectNativeOnly,
      launch: rejectNativeOnly,
      openLogDirectory: rejectNativeOnly,
      subscribeJobUpdates: rejectNativeOnly,
    },
    providers: {
      getSummary: rejectNativeOnly,
      applyQuickSetupWithResult: rejectNativeOnly,
      fetchModels: rejectNativeOnly,
      checkReachability: rejectNativeOnly,
      checkModel: rejectNativeOnly,
    },
    workbuddy: {
      getStatus: rejectNativeOnly,
      getModelIds: rejectNativeOnly,
      fetchModels: rejectNativeOnly,
      saveModels: rejectNativeOnly,
      checkReachability: rejectNativeOnly,
      checkModel: rejectNativeOnly,
    },
    opencodeModels: {
      getSnapshot: rejectNativeOnly,
      fetchProviderModels: rejectNativeOnly,
      saveModels: rejectNativeOnly,
      checkReachability: rejectNativeOnly,
      checkModel: rejectNativeOnly,
    },
    skills: {
      getInstalled: async () => [],
      getBackups: async () => [],
      deleteBackup: rejectNativeOnly,
      install: rejectNativeOnly,
      uninstall: rejectNativeOnly,
      restoreBackup: rejectNativeOnly,
      toggleApp: rejectNativeOnly,
      scanUnmanaged: async () => [],
      importFromApps: rejectNativeOnly,
      discoverPage: async () => ({ skills: [], totalCount: 0 }),
      checkUpdates: async () => [],
      update: rejectNativeOnly,
      migrateStorage: rejectNativeOnly,
      searchSkillHub: async (query) => ({
        skills: [],
        totalCount: 0,
        query,
        categories: [],
      }),
      installSkillHub: rejectNativeOnly,
      getRepos: async () => [],
      addRepo: rejectNativeOnly,
      removeRepo: rejectNativeOnly,
      pickZip: rejectNativeOnly,
      installFromZip: rejectNativeOnly,
    },
    mcp: {
      getAll: async () => ({}),
      upsert: rejectNativeOnly,
      delete: rejectNativeOnly,
      toggleApp: rejectNativeOnly,
      importFromApps: rejectNativeOnly,
    },
    prompts: {
      getAll: rejectNativeOnly,
      getCurrentFileContent: rejectNativeOnly,
      upsert: rejectNativeOnly,
      delete: rejectNativeOnly,
      enable: rejectNativeOnly,
      importFromFile: rejectNativeOnly,
    },
    memory: {
      readDocument: rejectNativeOnly,
      writeDocument: rejectNativeOnly,
      getHermesLimits: rejectNativeOnly,
      setHermesEnabled: rejectNativeOnly,
      listDailyFiles: rejectNativeOnly,
      readDailyFile: rejectNativeOnly,
      writeDailyFile: rejectNativeOnly,
      deleteDailyFile: rejectNativeOnly,
      searchDailyFiles: rejectNativeOnly,
      openOpenClawDirectory: rejectNativeOnly,
    },
    settings: {
      get: async () => ({}),
      save: rejectNativeOnly,
      openExternal: rejectNativeOnly,
    },
  };
}
