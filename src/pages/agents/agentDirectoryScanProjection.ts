import type {
  AgentInstallReadiness,
  AgentInstallState,
} from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";

export type AgentDirectoryScanView = {
  status: "idle" | "scanning" | "complete";
  settledIds: readonly AgentCatalogId[];
  currentFailureIds: readonly AgentCatalogId[];
  results: Partial<Record<AgentCatalogId, AgentInstallReadiness>>;
};

export type AgentDirectoryRowKind =
  | "pending"
  | "installed"
  | "not_installed"
  | "unknown"
  | "unavailable"
  | "error";

export type AgentDirectoryRowObservation = {
  agentId: AgentCatalogId;
  kind: AgentDirectoryRowKind;
  readiness: AgentInstallReadiness | undefined;
  scanning: boolean;
  refreshing: boolean;
  configurable: boolean;
  readFailed: boolean;
};

export function isAgentExistenceProven(
  installState: AgentInstallState | undefined,
): installState is "installed" | "installed_not_runnable" {
  return (
    installState === "installed" || installState === "installed_not_runnable"
  );
}

function kindFromInstallState(
  installState: AgentInstallState,
): Exclude<AgentDirectoryRowKind, "pending" | "error"> {
  if (isAgentExistenceProven(installState)) return "installed";
  return installState;
}

export function observeAgentDirectoryRow(
  agentId: AgentCatalogId,
  scan: AgentDirectoryScanView,
): AgentDirectoryRowObservation {
  const settled = scan.settledIds.includes(agentId);
  const scanning = scan.status === "scanning" && !settled;
  const readFailed = scan.currentFailureIds.includes(agentId);
  const readiness = scan.results[agentId];
  const configurable = isAgentExistenceProven(readiness?.installState);
  const refreshing = scanning && readiness !== undefined;
  const kind = readiness
    ? kindFromInstallState(readiness.installState)
    : settled || readFailed || scan.status === "complete"
      ? "error"
      : "pending";

  return {
    agentId,
    kind,
    readiness,
    scanning,
    refreshing,
    configurable,
    readFailed,
  };
}
