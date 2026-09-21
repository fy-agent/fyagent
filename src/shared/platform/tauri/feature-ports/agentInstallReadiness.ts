import { invoke } from "@tauri-apps/api/core";

import {
  assertAgentInstallReadinessId,
  parseAgentActionJobSnapshot,
  parseAgentActionResult,
  parseAgentInstallationInventory,
  parseAgentInstallReadiness,
  parseAgentInstallPreflight,
  type AgentInstallReadinessPort,
  type StartAgentActionRequest,
} from "../../../features/agent-install-readiness";

function actionPayload(
  request: StartAgentActionRequest,
): StartAgentActionRequest {
  return {
    agentId: assertAgentInstallReadinessId(request.agentId),
    action: request.action,
    ...(request.expectedReleaseId
      ? { expectedReleaseId: request.expectedReleaseId }
      : {}),
    ...(request.inventoryId ? { inventoryId: request.inventoryId } : {}),
    ...(request.targetId ? { targetId: request.targetId } : {}),
    ...(request.expectedTargetRevision
      ? { expectedTargetRevision: request.expectedTargetRevision }
      : {}),
    ...(request.surface ? { surface: request.surface } : {}),
  };
}

export function createAgentInstallReadinessPort(): AgentInstallReadinessPort {
  return {
    preflight: async (request) => {
      if (request.action !== "install" && request.action !== "update")
        throw new Error("Agent install readiness request is invalid");
      const payload = actionPayload(request);
      return parseAgentInstallPreflight(
        await invoke<unknown>("get_agent_install_preflight", {
          request: payload,
        }),
        payload,
      );
    },
    get: async (agentId) => {
      const safeAgentId = assertAgentInstallReadinessId(agentId);
      return parseAgentInstallReadiness(
        await invoke<unknown>("get_agent_install_readiness", {
          agentId: safeAgentId,
        }),
        safeAgentId,
      );
    },
    getInventory: async (agentId, surface) => {
      const safeAgentId = assertAgentInstallReadinessId(agentId);
      return parseAgentInstallationInventory(
        await invoke<unknown>("get_agent_installation_inventory", {
          agentId: safeAgentId,
          ...(surface ? { surface } : {}),
        }),
        safeAgentId,
      );
    },
    startAction: async (request: StartAgentActionRequest) => {
      const safeAgentId = assertAgentInstallReadinessId(request.agentId);
      return parseAgentActionResult(
        await invoke<unknown>("start_agent_action", {
          request: actionPayload(request),
        }),
        safeAgentId,
        request.action,
      );
    },
    cancelAction: async (jobId) =>
      parseAgentActionJobSnapshot(
        await invoke<unknown>("cancel_agent_action", { jobId }),
      ),
    getActionJob: async (jobId) =>
      parseAgentActionJobSnapshot(
        await invoke<unknown>("get_agent_action_job", { jobId }),
      ),
  };
}
