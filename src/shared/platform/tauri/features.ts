import type { FeaturePorts } from "../../features/ports";
import { createAgentAuthPort } from "./feature-ports/agentAuth";
import { createAgentFeaturePorts } from "./feature-ports/agents";
import { createAgentInstallReadinessPort } from "./feature-ports/agentInstallReadiness";
import { createChangePlansPort } from "./feature-ports/changePlans";
import { createCodexDesktopPort } from "./feature-ports/codexDesktop";
import { createContentFeaturePorts } from "./feature-ports/content";
import { createQoderTraeFeaturePorts } from "./feature-ports/qoderTrae";
import { createGrokToolingPort } from "./feature-ports/grokTooling";
import { createManagedAuthPort } from "./feature-ports/managedAuth";
import { createSimpleFeaturePorts } from "./feature-ports/simple";

export function createTauriFeaturePorts(): FeaturePorts {
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
  const projects = async () => {
    const { createProjectsPort } = await import("./feature-ports/projects");
    return createProjectsPort();
  };
  const deliveryKits = async () => {
    const { createDeliveryKitsPort } = await import(
      "./feature-ports/delivery-kits"
    );
    return createDeliveryKitsPort();
  };
  const verification = async () => {
    const { createVerificationPort } = await import(
      "./feature-ports/verification"
    );
    return createVerificationPort();
  };
  return {
    deliveryKits: {
      list: async (...args) => (await deliveryKits()).list(...args),
      previewBuiltin: async (...args) =>
        (await deliveryKits()).previewBuiltin(...args),
      pickImport: async (...args) => (await deliveryKits()).pickImport(...args),
      apply: async (...args) => (await deliveryKits()).apply(...args),
      cancel: async (...args) => (await deliveryKits()).cancel(...args),
      previewExport: async (...args) =>
        (await deliveryKits()).previewExport(...args),
      saveExport: async (...args) => (await deliveryKits()).saveExport(...args),
      runDemo: async (...args) => (await deliveryKits()).runDemo(...args),
    },
    projects: {
      listCustomers: async (...args) =>
        (await projects()).listCustomers(...args),
      createCustomer: async (...args) =>
        (await projects()).createCustomer(...args),
      updateCustomer: async (...args) =>
        (await projects()).updateCustomer(...args),
      list: async (...args) => (await projects()).list(...args),
      get: async (...args) => (await projects()).get(...args),
      create: async (...args) => (await projects()).create(...args),
      update: async (...args) => (await projects()).update(...args),
      resourceOptions: async (...args) =>
        (await projects()).resourceOptions(...args),
      credentialOptions: async (...args) =>
        (await projects()).credentialOptions(...args),
      bindResource: async (...args) => (await projects()).bindResource(...args),
      removeResource: async (...args) =>
        (await projects()).removeResource(...args),
      bindCredential: async (...args) =>
        (await projects()).bindCredential(...args),
      removeCredential: async (...args) =>
        (await projects()).removeCredential(...args),
      getContext: async (...args) => (await projects()).getContext(...args),
      writeContext: async (...args) => (await projects()).writeContext(...args),
      prepareCodex: async (...args) => (await projects()).prepareCodex(...args),
      bindDeliveryKit: async (...args) =>
        (await projects()).bindDeliveryKit(...args),
      dependencySnapshot: async (...args) =>
        (await projects()).dependencySnapshot(...args),
    },
    verification: {
      cancel: async (...args) => (await verification()).cancel(...args),
      get: async (...args) => (await verification()).get(...args),
      run: async (...args) => (await verification()).run(...args),
      record: async (...args) => (await verification()).record(...args),
      revoke: async (...args) => (await verification()).revoke(...args),
      saveHandoff: async (...args) =>
        (await verification()).saveHandoff(...args),
      preview: async (...args) => (await verification()).preview(...args),
      export: async (...args) => (await verification()).export(...args),
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
    codexDesktop: createCodexDesktopPort(),
    providers: {
      getSummary: async (...args) =>
        (await models()).providers.getSummary(...args),
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
