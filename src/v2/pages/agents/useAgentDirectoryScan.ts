import { useCallback, useEffect, useReducer, useRef } from "react";

import type { AgentInstallReadiness } from "../../shared/features/agent-install-readiness";
import { useAgentInstallReadiness } from "../../shared/features/queries";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "../../shared/features/types";

import { committedAgentDirectoryOrderIds } from "./agentDirectoryOrder";

type ScanStatus = "idle" | "scanning" | "complete";

export type AgentDirectoryScanState = {
  status: ScanStatus;
  requestId: number;
  settledIds: AgentCatalogId[];
  currentSuccessIds: AgentCatalogId[];
  currentFailureIds: AgentCatalogId[];
  results: Partial<Record<AgentCatalogId, AgentInstallReadiness>>;
  lastSuccessfulScanAt: number | null;
  committedOrderIds?: AgentCatalogId[] | null;
};

export type AgentDirectoryScanAction =
  | { type: "start"; requestId: number }
  | {
      type: "settled";
      requestId: number;
      agentId: AgentCatalogId;
      data?: AgentInstallReadiness;
    }
  | { type: "finish"; requestId: number; finishedAt: number }
  | {
      type: "applyReadiness";
      agentId: AgentCatalogId;
      data: AgentInstallReadiness;
    };

export type UseAgentDirectoryScanOptions = {
  /** AgentsPage must pass true. Default stays false for hook-level tests. */
  autoStart?: boolean;
};

const initialScanState: AgentDirectoryScanState = {
  status: "idle",
  requestId: 0,
  settledIds: [],
  currentSuccessIds: [],
  currentFailureIds: [],
  results: {},
  lastSuccessfulScanAt: null,
  committedOrderIds: null,
};

function appendUnique<T>(items: readonly T[], item: T): T[] {
  return items.includes(item) ? [...items] : [...items, item];
}

export function scanReducer(
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
  if (action.type === "applyReadiness") {
    const results = { ...state.results, [action.agentId]: action.data };
    if (state.status !== "complete") {
      return { ...state, results };
    }
    const currentFailureIds = state.currentFailureIds.filter(
      (id) => id !== action.agentId,
    );
    const currentSuccessIds = appendUnique(
      state.currentSuccessIds,
      action.agentId,
    );
    const next = {
      ...state,
      results,
      currentFailureIds,
      currentSuccessIds,
    };
    return {
      ...next,
      committedOrderIds: committedAgentDirectoryOrderIds(next),
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
  const completed = {
    ...state,
    status: "complete" as const,
    lastSuccessfulScanAt:
      state.currentSuccessIds.length > 0
        ? action.finishedAt
        : state.lastSuccessfulScanAt,
  };
  return {
    ...completed,
    committedOrderIds: committedAgentDirectoryOrderIds(completed),
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

export function useAgentDirectoryScan(options?: UseAgentDirectoryScanOptions) {
  const autoStart = options?.autoStart ?? false;
  const [state, dispatch] = useReducer(scanReducer, initialScanState);
  const queries = useReadinessQueries();
  const queriesRef = useRef(queries);
  const stateRef = useRef(state);

  useEffect(() => {
    queriesRef.current = queries;
    stateRef.current = state;
  });

  const start = useCallback(() => {
    if (stateRef.current.status === "scanning") return;
    const requestId = stateRef.current.requestId + 1;
    stateRef.current = {
      ...stateRef.current,
      status: "scanning",
      requestId,
      settledIds: [],
      currentSuccessIds: [],
      currentFailureIds: [],
    };
    dispatch({ type: "start", requestId });
    void Promise.all(
      AGENT_CATALOG_IDS.map(async (agentId) => {
        try {
          const result = await queriesRef.current[agentId].refetch();
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
  }, []);

  const applyReadiness = useCallback(
    (agentId: AgentCatalogId, data: AgentInstallReadiness) => {
      dispatch({ type: "applyReadiness", agentId, data });
    },
    [],
  );

  useEffect(() => {
    if (!autoStart) return;
    start();
  }, [autoStart, start]);

  return { state, start, applyReadiness };
}

export type AgentDirectoryScanController = ReturnType<
  typeof useAgentDirectoryScan
>;

export {
  isAgentExistenceProven,
  observeAgentDirectoryRow,
} from "./agentDirectoryScanProjection";
export type {
  AgentDirectoryRowKind,
  AgentDirectoryRowObservation,
  AgentDirectoryScanView,
} from "./agentDirectoryScanProjection";
