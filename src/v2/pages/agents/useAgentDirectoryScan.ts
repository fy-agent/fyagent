import { useReducer } from "react";

import type { AgentInstallReadiness } from "../../shared/features/agent-install-readiness";
import { useAgentInstallReadiness } from "../../shared/features/queries";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "../../shared/features/types";

type ScanStatus = "idle" | "scanning" | "complete";

export type AgentDirectoryScanState = {
  status: ScanStatus;
  requestId: number;
  settledIds: AgentCatalogId[];
  currentSuccessIds: AgentCatalogId[];
  currentFailureIds: AgentCatalogId[];
  results: Partial<Record<AgentCatalogId, AgentInstallReadiness>>;
  lastSuccessfulScanAt: number | null;
};

type AgentDirectoryScanAction =
  | { type: "start"; requestId: number }
  | {
      type: "settled";
      requestId: number;
      agentId: AgentCatalogId;
      data?: AgentInstallReadiness;
    }
  | { type: "finish"; requestId: number; finishedAt: number };

const initialScanState: AgentDirectoryScanState = {
  status: "idle",
  requestId: 0,
  settledIds: [],
  currentSuccessIds: [],
  currentFailureIds: [],
  results: {},
  lastSuccessfulScanAt: null,
};

function appendUnique<T>(items: readonly T[], item: T): T[] {
  return items.includes(item) ? [...items] : [...items, item];
}

function scanReducer(
  state: AgentDirectoryScanState,
  action: AgentDirectoryScanAction,
): AgentDirectoryScanState {
  if (action.type === "start") {
    return {
      ...state,
      status: "scanning",
      requestId: action.requestId,
      settledIds: [],
      currentSuccessIds: [],
      currentFailureIds: [],
    };
  }
  if (action.requestId !== state.requestId) return state;
  if (action.type === "settled") {
    const settledIds = appendUnique(state.settledIds, action.agentId);
    if (action.data) {
      return {
        ...state,
        settledIds,
        currentSuccessIds: appendUnique(
          state.currentSuccessIds,
          action.agentId,
        ),
        currentFailureIds: state.currentFailureIds.filter(
          (id) => id !== action.agentId,
        ),
        results: { ...state.results, [action.agentId]: action.data },
      };
    }
    return {
      ...state,
      settledIds,
      currentFailureIds: appendUnique(state.currentFailureIds, action.agentId),
    };
  }
  return {
    ...state,
    status: "complete",
    lastSuccessfulScanAt:
      state.currentSuccessIds.length > 0
        ? action.finishedAt
        : state.lastSuccessfulScanAt,
  };
}

function useReadinessQueries() {
  return {
    qoderwork: useAgentInstallReadiness("qoderwork", false),
    "trae-work": useAgentInstallReadiness("trae-work", false),
    workbuddy: useAgentInstallReadiness("workbuddy", false),
    grokbuild: useAgentInstallReadiness("grokbuild", false),
    codex: useAgentInstallReadiness("codex", false),
    "claude-code": useAgentInstallReadiness("claude-code", false),
    opencode: useAgentInstallReadiness("opencode", false),
  };
}

export function useAgentDirectoryScan() {
  const [state, dispatch] = useReducer(scanReducer, initialScanState);
  const queries = useReadinessQueries();

  const start = () => {
    if (state.status === "scanning") return;
    const requestId = state.requestId + 1;
    dispatch({ type: "start", requestId });
    void Promise.all(
      AGENT_CATALOG_IDS.map(async (agentId) => {
        try {
          const result = await queries[agentId].refetch();
          dispatch({
            type: "settled",
            requestId,
            agentId,
            data: result.error ? undefined : result.data,
          });
        } catch {
          dispatch({ type: "settled", requestId, agentId });
        }
      }),
    ).then(() => {
      dispatch({ type: "finish", requestId, finishedAt: Date.now() });
    });
  };

  return { state, start };
}

export type AgentDirectoryScanController = ReturnType<
  typeof useAgentDirectoryScan
>;
