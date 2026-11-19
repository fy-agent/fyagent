import type { FeaturePorts } from "../../features/ports";
import { createAgentAuthPort } from "./feature-ports/agentAuth";
import { createAgentFeaturePorts } from "./feature-ports/agents";
import { createAgentInstallReadinessPort } from "./feature-ports/agentInstallReadiness";
import { createGrokToolingPort } from "./feature-ports/grokTooling";
import { createSimpleFeaturePorts } from "./feature-ports/simple";

function memoizePort<T>(load: () => Promise<T>): () => Promise<T> {
  let cached: Promise<T> | null = null;
  return () => {
    cached ??= load().catch((error) => {
      cached = null;
      throw error;
    });
    return cached;
  };
}

export function createTauriFeaturePorts(): FeaturePorts {
  const codexDesktop = memoizePort(async () => {
    const { createCodexDesktopPort } = await import(
      "./feature-ports/codexDesktop"
    );
    return createCodexDesktopPort();
  });
  const configPack = memoizePort(async () => {
    const { createConfigPackPort } = await import("./feature-ports/configPack");
    return createConfigPackPort();
  });
  const databaseRecovery = memoizePort(async () => {
    const { createDatabaseRecoveryPort } = await import(
      "./feature-ports/databaseRecovery"
    );
    return createDatabaseRecoveryPort();
  });
  const models = memoizePort(async () => {
    const { createModelFeaturePorts } = await import("./feature-ports/models");
    return createModelFeaturePorts();
  });
  const sessionMigration = memoizePort(async () => {
    const { createSessionMigrationPort } = await import(
      "./feature-ports/sessionMigration"
    );
    return createSessionMigrationPort();
  });
  const workspaceTransfer = memoizePort(async () => {
    const { createWorkspaceTransferPort } = await import(
      "./feature-ports/workspaceTransfer"
    );
    return createWorkspaceTransferPort();
  });
  const managedAuth = memoizePort(async () => {
    const { createManagedAuthPort } = await import(
      "./feature-ports/managedAuth"
    );
    return createManagedAuthPort();
  });
  const changePlans = memoizePort(async () => {
    const { createChangePlansPort } = await import(
      "./feature-ports/changePlans"
    );
    return createChangePlansPort();
  });
  const qoderTrae = memoizePort(async () => {
    const { createQoderTraeFeaturePorts } = await import(
      "./feature-ports/qoderTrae"
    );
    return createQoderTraeFeaturePorts();
  });
  const content = memoizePort(async () => {
    const { createContentFeaturePorts } = await import(
      "./feature-ports/content"
    );
    return createContentFeaturePorts();
  });

  return {
    configPack: {
      list: async (...args) => (await configPack()).list(...args),
      pickFile: async (...args) => (await configPack()).pickFile(...args),
      previewImport: async (...args) =>
        (await configPack()).previewImport(...args),
      apply: async (...args) => (await configPack()).apply(...args),
      previewExport: async (...args) =>
        (await configPack()).previewExport(...args),
      saveExport: async (...args) => (await configPack()).saveExport(...args),
      cancel: async (...args) => (await configPack()).cancel(...args),
    },
    databaseRecovery: {
      list: async () => (await databaseRecovery()).list(),
      restore: async (filename) => (await databaseRecovery()).restore(filename),
      checkReadability: async () =>
        (await databaseRecovery()).checkReadability(),
    },
    agentAuth: createAgentAuthPort(),
    managedAuth: {
      getOverview: async () => (await managedAuth()).getOverview(),
      getAccountQuota: async (...args) =>
        (await managedAuth()).getAccountQuota(...args),
      startLogin: async (...args) => (await managedAuth()).startLogin(...args),
      getLoginSession: async (...args) =>
        (await managedAuth()).getLoginSession(...args),
      cancelLogin: async (...args) =>
        (await managedAuth()).cancelLogin(...args),
      reopenLogin: async (...args) =>
        (await managedAuth()).reopenLogin(...args),
      switchLoginMethod: async (...args) =>
        (await managedAuth()).switchLoginMethod(...args),
      setDefaultAccount: async (...args) =>
        (await managedAuth()).setDefaultAccount(...args),
      previewAccountRemoval: async (...args) =>
        (await managedAuth()).previewAccountRemoval(...args),
      removeAccount: async (...args) =>
        (await managedAuth()).removeAccount(...args),
      previewConnectionAction: async (...args) =>
        (await managedAuth()).previewConnectionAction(...args),
      applyConnectionAction: async (...args) =>
        (await managedAuth()).applyConnectionAction(...args),
    },
    agentInstallReadiness: createAgentInstallReadinessPort(),
    changePlans: {
      createCodexProviderSwitchPlan: async (...args) =>
        (await changePlans()).createCodexProviderSwitchPlan(...args),
      createCodexProviderUpsertPlan: async (...args) =>
        (await changePlans()).createCodexProviderUpsertPlan(...args),
      createWorkBuddySubscriptionPlan: async (...args) =>
        (await changePlans()).createWorkBuddySubscriptionPlan(...args),
      createWorkBuddySavePlan: async (...args) =>
        (await changePlans()).createWorkBuddySavePlan(...args),
      applyChangePlan: async (...args) =>
        (await changePlans()).applyChangePlan(...args),
      cancelChangeJob: async (...args) =>
        (await changePlans()).cancelChangeJob(...args),
      getChangeJob: async (...args) =>
        (await changePlans()).getChangeJob(...args),
      listRecoverableChangeJobs: async () =>
        (await changePlans()).listRecoverableChangeJobs(),
    },
    ...createAgentFeaturePorts(),
    qoderwork: {
      getHooks: async () => (await qoderTrae()).qoderwork.getHooks(),
      saveHooks: async (...args) =>
        (await qoderTrae()).qoderwork.saveHooks(...args),
    },
    externalMcp: {
      validate: async (...args) =>
        (await qoderTrae()).externalMcp.validate(...args),
    },
    traeWork: {
      validateModelConfig: async (...args) =>
        (await qoderTrae()).traeWork.validateModelConfig(...args),
      testModelEndpoint: async (...args) =>
        (await qoderTrae()).traeWork.testModelEndpoint(...args),
      cancelModelEndpoint: async (...args) =>
        (await qoderTrae()).traeWork.cancelModelEndpoint(...args),
      getModelIds: async () => (await qoderTrae()).traeWork.getModelIds(),
    },
    codexDesktop: {
      getLocalStatus: async (...args) =>
        (await codexDesktop()).getLocalStatus(...args),
      checkLatest: async (...args) =>
        (await codexDesktop()).checkLatest(...args),
      getJob: async (...args) => (await codexDesktop()).getJob(...args),
      prepareInstall: async (...args) =>
        (await codexDesktop()).prepareInstall(...args),
      startInstall: async (...args) =>
        (await codexDesktop()).startInstall(...args),
      cancelInstall: async (...args) =>
        (await codexDesktop()).cancelInstall(...args),
      launch: async (...args) => (await codexDesktop()).launch(...args),
      openLogDirectory: async (...args) =>
        (await codexDesktop()).openLogDirectory(...args),
      subscribeJobUpdates: async (...args) =>
        (await codexDesktop()).subscribeJobUpdates(...args),
    },
    providers: {
      getAll: async (...args) => (await models()).providers.getAll(...args),
      add: async (...args) => (await models()).providers.add(...args),
      update: async (...args) => (await models()).providers.update(...args),
      switch: async (...args) => (await models()).providers.switch(...args),
      getMode: async (...args) => (await models()).providers.getMode(...args),
      setMode: async (...args) => (await models()).providers.setMode(...args),
      setRoute: async (...args) => (await models()).providers.setRoute(...args),
      getStack: async (...args) => (await models()).providers.getStack(...args),
      setStackMember: async (...args) =>
        (await models()).providers.setStackMember(...args),
      adoptCodexCatalog: async (...args) =>
        (await models()).providers.adoptCodexCatalog(...args),
      getCodexReloadStatus: async () =>
        (await models()).providers.getCodexReloadStatus(),
      restartCodexDaemon: async (...args) =>
        (await models()).providers.restartCodexDaemon(...args),
      previewClaudeQuickSetup: async (...args) =>
        (await models()).providers.previewClaudeQuickSetup(...args),
      applyClaudeQuickSetupPreview: async (...args) =>
        (await models()).providers.applyClaudeQuickSetupPreview(...args),
      previewBatchSetup: async (...args) =>
        (await models()).providers.previewBatchSetup(...args),
      applyBatchSetup: async (...args) =>
        (await models()).providers.applyBatchSetup(...args),
      probeSavedModel: async (...args) =>
        (await models()).providers.probeSavedModel(...args),
      getModelHealthOverview: async () =>
        (await models()).providers.getModelHealthOverview(),
      previewCloneModelConfig: async (...args) =>
        (await models()).providers.previewCloneModelConfig(...args),
      applyCloneModelConfig: async (...args) =>
        (await models()).providers.applyCloneModelConfig(...args),
      previewApiKeyRotation: async (...args) =>
        (await models()).providers.previewApiKeyRotation(...args),
      applyApiKeyRotation: async (...args) =>
        (await models()).providers.applyApiKeyRotation(...args),
      listCustomGatewayTemplates: async () =>
        (await models()).providers.listCustomGatewayTemplates(),
      saveCustomGatewayTemplate: async (...args) =>
        (await models()).providers.saveCustomGatewayTemplate(...args),
      deleteCustomGatewayTemplate: async (...args) =>
        (await models()).providers.deleteCustomGatewayTemplate(...args),
      getSummary: async (...args) =>
        (await models()).providers.getSummary(...args),
      getProxyRestorePreview: async (...args) =>
        (await models()).providers.getProxyRestorePreview(...args),
      restoreManagedProxy: async (...args) =>
        (await models()).providers.restoreManagedProxy(...args),
      applyQuickSetupWithResult: async (...args) =>
        (await models()).providers.applyQuickSetupWithResult(...args),
      fetchModels: async (...args) =>
        (await models()).providers.fetchModels(...args),
      checkReachability: async (...args) =>
        (await models()).providers.checkReachability(...args),
      checkModel: async (...args) =>
        (await models()).providers.checkModel(...args),
      getModelProbeStatus: async (...args) =>
        (await models()).providers.getModelProbeStatus(...args),
      cancelModelProbe: async (...args) =>
        (await models()).providers.cancelModelProbe(...args),
      bindXaiManaged: async (...args) =>
        (await models()).providers.bindXaiManaged(...args),
      bindManagedProxy: async (...args) =>
        (await models()).providers.bindManagedProxy(...args),
      fetchXaiManagedModels: async (...args) =>
        (await models()).providers.fetchXaiManagedModels(...args),
    },
    workbuddy: {
      getSubscriptionSource: async () =>
        (await models()).workbuddy.getSubscriptionSource(),
      getStatus: async (...args) =>
        (await models()).workbuddy.getStatus(...args),
      getModelIds: async (...args) =>
        (await models()).workbuddy.getModelIds(...args),
      fetchModels: async (...args) =>
        (await models()).workbuddy.fetchModels(...args),
      saveModels: async (...args) =>
        (await models()).workbuddy.saveModels(...args),
      checkReachability: async (...args) =>
        (await models()).workbuddy.checkReachability(...args),
      checkModel: async (...args) =>
        (await models()).workbuddy.checkModel(...args),
      getModelProbeStatus: async (...args) =>
        (await models()).workbuddy.getModelProbeStatus(...args),
      cancelModelProbe: async (...args) =>
        (await models()).workbuddy.cancelModelProbe(...args),
    },
    opencodeModels: {
      restoreManagedProxy: async (...args) =>
        (await models()).opencodeModels.restoreManagedProxy(...args),
      bindManagedProxy: async (...args) =>
        (await models()).opencodeModels.bindManagedProxy(...args),
      getSnapshot: async (...args) =>
        (await models()).opencodeModels.getSnapshot(...args),
      fetchProviderModels: async (...args) =>
        (await models()).opencodeModels.fetchProviderModels(...args),
      saveModels: async (...args) =>
        (await models()).opencodeModels.saveModels(...args),
      checkReachability: async (...args) =>
        (await models()).opencodeModels.checkReachability(...args),
      checkModel: async (...args) =>
        (await models()).opencodeModels.checkModel(...args),
      getModelProbeStatus: async (...args) =>
        (await models()).opencodeModels.getModelProbeStatus(...args),
      cancelModelProbe: async (...args) =>
        (await models()).opencodeModels.cancelModelProbe(...args),
    },
    ompModels: {
      readConfig: async () => (await models()).ompModels.readConfig(),
      discoverCli: async () => (await models()).ompModels.discoverCli(),
      previewSave: async (...args) =>
        (await models()).ompModels.previewSave(...args),
      applySave: async (...args) =>
        (await models()).ompModels.applySave(...args),
    },
    ...createSimpleFeaturePorts(),
    prompts: {
      getAll: async (...args) => (await content()).prompts.getAll(...args),
      getCurrentFileContent: async (...args) =>
        (await content()).prompts.getCurrentFileContent(...args),
      upsert: async (...args) => (await content()).prompts.upsert(...args),
      delete: async (...args) => (await content()).prompts.delete(...args),
      enable: async (...args) => (await content()).prompts.enable(...args),
      importFromFile: async (...args) =>
        (await content()).prompts.importFromFile(...args),
    },
    memory: {
      readDocument: async (...args) =>
        (await content()).memory.readDocument(...args),
      writeDocument: async (...args) =>
        (await content()).memory.writeDocument(...args),
      getHermesLimits: async () => (await content()).memory.getHermesLimits(),
      setHermesEnabled: async (...args) =>
        (await content()).memory.setHermesEnabled(...args),
      listDailyFiles: async () => (await content()).memory.listDailyFiles(),
      readDailyFile: async (...args) =>
        (await content()).memory.readDailyFile(...args),
      writeDailyFile: async (...args) =>
        (await content()).memory.writeDailyFile(...args),
      deleteDailyFile: async (...args) =>
        (await content()).memory.deleteDailyFile(...args),
      searchDailyFiles: async (...args) =>
        (await content()).memory.searchDailyFiles(...args),
      openOpenClawDirectory: async (...args) =>
        (await content()).memory.openOpenClawDirectory(...args),
    },
    sessions: {
      listSessions: async (...args) =>
        (await sessionMigration()).listSessions(...args),
      getSessionMessages: async (...args) =>
        (await sessionMigration()).getSessionMessages(...args),
      previewSessionMigration: async (...args) =>
        (await sessionMigration()).previewSessionMigration(...args),
      exportSessionPackage: async (...args) =>
        (await sessionMigration()).exportSessionPackage(...args),
      readSessionPackage: async (...args) =>
        (await sessionMigration()).readSessionPackage(...args),
      probeLocalProvider: async (...args) =>
        (await sessionMigration()).probeLocalProvider(...args),
      getReleaseCapabilityMatrix: async (...args) =>
        (await sessionMigration()).getReleaseCapabilityMatrix(...args),
      restoreSessionPackage: async (...args) =>
        (await sessionMigration()).restoreSessionPackage(...args),
      verifyNativeReadback: async (...args) =>
        (await sessionMigration()).verifyNativeReadback(...args),
      listRestoreAttempts: async (...args) =>
        (await sessionMigration()).listRestoreAttempts(...args),
      reconcileRestoreAttempts: async (...args) =>
        (await sessionMigration()).reconcileRestoreAttempts(...args),
      recordUserAttestation: async (...args) =>
        (await sessionMigration()).recordUserAttestation(...args),
      openRestoredSession: async (...args) =>
        (await sessionMigration()).openRestoredSession(...args),
      pickDirectory: async (...args) =>
        (await sessionMigration()).pickDirectory(...args),
      pickPackageFile: async (...args) =>
        (await sessionMigration()).pickPackageFile(...args),
      pickExportPath: async (...args) =>
        (await sessionMigration()).pickExportPath(...args),
    },
    workspaceTransfer: {
      pickExportPath: async (...args) =>
        (await workspaceTransfer()).pickExportPath(...args),
      pickImportFile: async () => (await workspaceTransfer()).pickImportFile(),
      previewExport: async (...args) =>
        (await workspaceTransfer()).previewExport(...args),
      exportPack: async (...args) =>
        (await workspaceTransfer()).exportPack(...args),
      inspectPack: async (...args) =>
        (await workspaceTransfer()).inspectPack(...args),
      applyImport: async (...args) =>
        (await workspaceTransfer()).applyImport(...args),
      listSshHosts: async () => (await workspaceTransfer()).listSshHosts(),
      testSshTarget: async (...args) =>
        (await workspaceTransfer()).testSshTarget(...args),
      previewSshPush: async (...args) =>
        (await workspaceTransfer()).previewSshPush(...args),
      applySshPush: async (...args) =>
        (await workspaceTransfer()).applySshPush(...args),
    },
    tooling: createGrokToolingPort(),
  };
}
