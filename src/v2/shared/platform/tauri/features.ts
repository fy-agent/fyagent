import type { FeaturePorts } from "../../features/ports";
import { createAgentFeaturePorts } from "./feature-ports/agents";
import { createAgentInstallReadinessPort } from "./feature-ports/agentInstallReadiness";
import { createChangePlansPort } from "./feature-ports/changePlans";
import { createCodexDesktopPort } from "./feature-ports/codexDesktop";
import { createContentFeaturePorts } from "./feature-ports/content";
import { createModelFeaturePorts } from "./feature-ports/models";
import { createQoderTraeFeaturePorts } from "./feature-ports/qoderTrae";
import { createShurufaPort } from "./feature-ports/shurufa";
import { createSimpleFeaturePorts } from "./feature-ports/simple";

export function createTauriFeaturePorts(): FeaturePorts {
  return {
    agentInstallReadiness: createAgentInstallReadinessPort(),
    changePlans: createChangePlansPort(),
    ...createAgentFeaturePorts(),
    ...createQoderTraeFeaturePorts(),
    codexDesktop: createCodexDesktopPort(),
    ...createModelFeaturePorts(),
    ...createSimpleFeaturePorts(),
    ...createContentFeaturePorts(),
    shurufa: createShurufaPort(),
  };
}
