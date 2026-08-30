import { invoke } from "@tauri-apps/api/core";

import {
  assertAgentAuthId,
  parseAgentAuthObservation,
  parseAgentAuthSessionSnapshot,
  type AgentAuthPort,
  type StartAgentAuthSessionRequest,
} from "../../../features/agent-auth";

export function createAgentAuthPort(): AgentAuthPort {
  return {
    getObservation: async (agentId) => {
      const safeAgentId = assertAgentAuthId(agentId);
      return parseAgentAuthObservation(
        await invoke<unknown>("get_agent_auth_observation", {
          agentId: safeAgentId,
        }),
        safeAgentId,
      );
    },
    getActiveSession: async (agentId) => {
      const safeAgentId = assertAgentAuthId(agentId);
      const value = await invoke<unknown>("get_active_agent_auth_session", {
        agentId: safeAgentId,
      });
      if (value === null) return null;
      const snapshot = parseAgentAuthSessionSnapshot(value);
      if (snapshot.agentId !== safeAgentId) {
        throw new Error("Agent auth session is unavailable");
      }
      return snapshot;
    },
    startSession: async (request: StartAgentAuthSessionRequest) => {
      const safeAgentId = assertAgentAuthId(request.agentId);
      return parseAgentAuthSessionSnapshot(
        await invoke<unknown>("start_agent_auth_session", {
          request: {
            agentId: safeAgentId,
            intent: request.intent,
            ...(request.providerId ? { providerId: request.providerId } : {}),
            ...(request.inventoryId
              ? { inventoryId: request.inventoryId }
              : {}),
            ...(request.targetId ? { targetId: request.targetId } : {}),
            ...(request.expectedTargetRevision
              ? { expectedTargetRevision: request.expectedTargetRevision }
              : {}),
          },
        }),
      );
    },
    getSession: async (sessionId) =>
      parseAgentAuthSessionSnapshot(
        await invoke<unknown>("get_agent_auth_session", { sessionId }),
      ),
    stopWaiting: async (sessionId) =>
      parseAgentAuthSessionSnapshot(
        await invoke<unknown>("stop_waiting_for_agent_auth", { sessionId }),
      ),
  };
}
