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
  /** Pause UI dispatch while the Agents surface is hidden. */
  active?: boolean;
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

type PendingScanWork = {
  requestId: number;
  settled: Array<{
    agentId: AgentCatalogId;
    data?: AgentInstallReadiness;
  }>;
  finishedAt: number | null;
};

export function useAgentDirectoryScan(options?: UseAgentDirectoryScanOptions) {
  const autoStart = options?.autoStart ?? false;
  const active = options?.active ?? true;
  const [state, dispatch] = useReducer(scanReducer, initialScanState);
  const queries = useReadinessQueries();
  const queriesRef = useRef(queries);
  const stateRef = useRef(state);
  const activeRef = useRef(active);
  const pendingRef = useRef<PendingScanWork>({
    requestId: 0,
    settled: [],
    finishedAt: null,
  });

  useEffect(() => {
    queriesRef.current = queries;
    stateRef.current = state;
    activeRef.current = active;
  });

  const start = useCallback(() => {
    if (stateRef.current.status === "scanning") return;
    const requestId = stateRef.current.requestId + 1;
    pendingRef.current = { requestId, settled: [], finishedAt: null };
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
          const data = result.error ? undefined : result.data;
          if (!activeRef.current) {
            if (pendingRef.current.requestId === requestId) {
              pendingRef.current.settled.push({ agentId, data });
            }
            return;
          }
          dispatch({
            type: "settled",
            requestId,
            agentId,
            data,
          });
        } catch {
          if (!activeRef.current) {
            if (pendingRef.current.requestId === requestId) {
              pendingRef.current.settled.push({ agentId });
            }
            return;
          }
          dispatch({ type: "settled", requestId, agentId });
        }
      }),
    ).then(() => {
      if (!activeRef.current) {
        if (pendingRef.current.requestId === requestId) {
          pendingRef.current.finishedAt = Date.now();
        }
        return;
      }
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
    if (!active) return;
    const pending = pendingRef.current;
    const requestId = pending.requestId;
    if (requestId === 0) return;
    for (const item of pending.settled) {
      dispatch({
        type: "settled",
        requestId,
        agentId: item.agentId,
        data: item.data,
      });
    }
    pending.settled = [];
    if (pending.finishedAt !== null) {
      dispatch({
        type: "finish",
        requestId,
        finishedAt: pending.finishedAt,
      });
      pending.finishedAt = null;
    }
  }, [active]);

  useEffect(() => {
    if (!autoStart || !active) return;
    if (stateRef.current.status !== "idle") return;
    start();
  }, [autoStart, active, start]);

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
