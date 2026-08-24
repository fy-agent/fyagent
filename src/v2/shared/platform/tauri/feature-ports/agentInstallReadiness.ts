import { invoke } from "@tauri-apps/api/core";

import {
  assertAgentInstallReadinessId,
  parseAgentInstallReadiness,
  type AgentInstallReadinessPort,
} from "../../../features/agent-install-readiness";

export function createAgentInstallReadinessPort(): AgentInstallReadinessPort {
  return {
    get: async (agentId) => {
      const safeAgentId = assertAgentInstallReadinessId(agentId);
      return parseAgentInstallReadiness(
        await invoke<unknown>("get_agent_install_readiness", {
          agentId: safeAgentId,
        }),
        safeAgentId,
      );
    },
  };
}
