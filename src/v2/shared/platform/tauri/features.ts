import type { FeaturePorts } from "../../features/ports";
import { createAgentFeaturePorts } from "./feature-ports/agents";
import { createCodexDesktopPort } from "./feature-ports/codexDesktop";
import { createChangePlanPort } from "./feature-ports/changePlan";
import { createContentFeaturePorts } from "./feature-ports/content";
import { createModelFeaturePorts } from "./feature-ports/models";
import { createQoderTraeFeaturePorts } from "./feature-ports/qoderTrae";
import { createSimpleFeaturePorts } from "./feature-ports/simple";

export function createTauriFeaturePorts(): FeaturePorts {
  return {
    ...createAgentFeaturePorts(),
    ...createQoderTraeFeaturePorts(),
    codexDesktop: createCodexDesktopPort(),
    changePlan: createChangePlanPort(),
    ...createModelFeaturePorts(),
    ...createSimpleFeaturePorts(),
    ...createContentFeaturePorts(),
  };
}
