import type {
  AgentActionId,
  AgentInstallReadiness,
} from "./agent-install-readiness";
import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export type AgentDirectoryUpdateUi = "none" | "generic" | "codex_desktop";

/**
 * Directory chrome for FyAgent-managed one-click update.
 * Native admission remains `lifecycle_policy.rs`; this table must stay aligned.
 */
export const AGENT_DIRECTORY_UPDATE_UI = {
  qoderwork: "none",
  "trae-work": "none",
  workbuddy: "none",
  grokbuild: "generic",
  codex: "codex_desktop",
  "claude-code": "generic",
  opencode: "generic",
} as const satisfies Record<AgentCatalogId, AgentDirectoryUpdateUi>;

export const DIRECTORY_UPDATE_DISABLED_AGENT_IDS = AGENT_CATALOG_IDS.filter(
  (id) => AGENT_DIRECTORY_UPDATE_UI[id] === "none",
);

export function directoryUpdateUi(
  agentId: AgentCatalogId,
): AgentDirectoryUpdateUi {
  return AGENT_DIRECTORY_UPDATE_UI[agentId];
}

export function visibleAllowedActions(
  agentId: AgentCatalogId,
  allowed: readonly AgentActionId[],
): AgentActionId[] {
  if (AGENT_DIRECTORY_UPDATE_UI[agentId] !== "none") {
    return [...allowed];
  }
  return allowed.filter((action) => action !== "update");
}

export function canOfferDirectoryUpdate(
  agentId: AgentCatalogId,
  readiness: AgentInstallReadiness | null,
): boolean {
  if (!readiness) return false;
  if (AGENT_DIRECTORY_UPDATE_UI[agentId] !== "generic") return false;
  if (
    readiness.installState !== "installed" &&
    readiness.installState !== "installed_not_runnable"
  ) {
    return false;
  }
  return (
    readiness.updateState === "update_available" &&
    readiness.allowedActions.includes("update")
  );
}
