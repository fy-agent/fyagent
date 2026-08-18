import { invoke } from "@tauri-apps/api/core";
import type {
  InstallContract,
  PlanLayerState,
  PreflightLayerState,
  SourceLayerState,
} from "@/types/agentInstall";

export const agentInstallApi = {
  listCatalog(): Promise<SourceLayerState[]> {
    return invoke("agent_install_list_catalog");
  },

  getContract(agentId: string): Promise<InstallContract> {
    return invoke("agent_install_get_contract", { agentId });
  },

  refreshPreflight(agentId: string): Promise<PreflightLayerState> {
    return invoke("agent_install_refresh_preflight", { agentId });
  },

  createPlan(agentId: string): Promise<PlanLayerState> {
    return invoke("agent_install_create_plan", { agentId });
  },

  reconfirmPlan(snapshotId: string): Promise<PlanLayerState> {
    return invoke("agent_install_reconfirm_plan", { snapshotId });
  },

  startInstall(snapshotId: string): Promise<PlanLayerState> {
    return invoke("agent_install_start_install", {
      request: { snapshotId },
    });
  },

  probeHealth(agentId: string): Promise<unknown> {
    return invoke("agent_install_probe_health", { agentId });
  },

  openOfficialGuide(agentId: string): Promise<boolean> {
    return invoke("agent_install_open_official_guide", { agentId });
  },
};
