import type { FeaturePorts } from "../../features/ports";
import { createAgentAuthPort } from "./feature-ports/agentAuth";
import { createAgentFeaturePorts } from "./feature-ports/agents";
import { createAgentInstallReadinessPort } from "./feature-ports/agentInstallReadiness";
import { createChangePlansPort } from "./feature-ports/changePlans";
import { createContentFeaturePorts } from "./feature-ports/content";
import { createQoderTraeFeaturePorts } from "./feature-ports/qoderTrae";
import { createGrokToolingPort } from "./feature-ports/grokTooling";
import { createManagedAuthPort } from "./feature-ports/managedAuth";
import { createSimpleFeaturePorts } from "./feature-ports/simple";

export function createTauriFeaturePorts(): FeaturePorts {
  const codexDesktop = async () => {
    const { createCodexDesktopPort } = await import(
      "./feature-ports/codexDesktop"
    );
    return createCodexDesktopPort();
  };
  const configPack = async () => {
    const { createConfigPackPort } = await import("./feature-ports/configPack");
    return createConfigPackPort();
  };
  const configRecovery = async () => {
    const { createConfigRecoveryPort } = await import(
      "./feature-ports/configRecovery"
    );
    return createConfigRecoveryPort();
  };
  const models = async () => {
    const { createModelFeaturePorts } = await import("./feature-ports/models");
    return createModelFeaturePorts();
  };
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
    health: {
      get: async (agentId) => {
        const { createHealthPort } = await import("./feature-ports/health");
        return createHealthPort().get(agentId);
      },
    },
    configRecovery: {
      list: async (...args) => (await configRecovery()).list(...args),
      restore: async (...args) => (await configRecovery()).restore(...args),
    },
    agentAuth: createAgentAuthPort(),
    managedAuth: createManagedAuthPort(),
    agentInstallReadiness: createAgentInstallReadinessPort(),
    changePlans: createChangePlansPort(),
    ...createAgentFeaturePorts(),
    ...createQoderTraeFeaturePorts(),
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
      bindXaiManaged: async (...args) =>
        (await models()).providers.bindXaiManaged(...args),
      bindManagedProxy: async (...args) =>
        (await models()).providers.bindManagedProxy(...args),
      fetchXaiManagedModels: async (...args) =>
        (await models()).providers.fetchXaiManagedModels(...args),
    },
    workbuddy: {
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
    },
    ...createSimpleFeaturePorts(),
    ...createContentFeaturePorts(),
    tooling: createGrokToolingPort(),
  };
}
