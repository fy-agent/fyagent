import { describe, expect, it } from "vitest";

import {
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentInstallReadiness,
} from "@/shared/features/agent-install-readiness";
import {
  AGENT_CATALOG_IDS,
  AGENT_DISPLAY_ORDER,
  agentDirectoryPriority,
  type AgentCatalogId,
} from "@/shared/features/directory";
import {
  applyCommittedAgentDirectoryOrder,
  classifyAgentDirectoryOrderBucket,
  nextCommittedAgentDirectoryOrderIds,
  orderAgentDirectoryEntries,
  type AgentDirectoryOrderBucket,
} from "@/pages/agents/agentDirectoryOrder";
import type { AgentDirectoryScanView } from "@/pages/agents/agentDirectoryScanProjection";
import {
  scanReducer,
  type AgentDirectoryScanState,
} from "@/pages/agents/useAgentDirectoryScan";

type CatalogEntry = { id: AgentCatalogId };

const CATALOG: CatalogEntry[] = AGENT_CATALOG_IDS.map((id) => ({ id }));

function readiness(
  agentId: AgentCatalogId,
  installState: AgentInstallReadiness["installState"],
): AgentInstallReadiness {
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    configurationEligibility:
      installState === "installed" || installState === "installed_not_runnable"
        ? { state: "eligible", evidence: "installation_detected" }
        : {
            state:
              installState === "not_installed" ? "not_detected" : installState,
            evidence: "none",
          },
    agentId,
    reviewedAt: "2026-08-31",
    installState,
    inventoryState: installState === "installed" ? "single" : "not_observed",
    requiresTargetSelection: false,
    updateState: installState === "installed" ? "up_to_date" : "unknown",
    releaseId: null,
    localVersion: installState === "installed" ? "1.0.0" : null,
    remoteVersion: null,
    authOwnership: "agent_owned",
    authState: "unknown",
    sourceKind: "managed_desktop",
    allowedActions: [],
    reasonCodes: ["auth_state_unknown"],
  };
}

function scanView(
  overrides: Partial<AgentDirectoryScanView> = {},
): AgentDirectoryScanView {
  return {
    status: "complete",
    settledIds: [...AGENT_CATALOG_IDS],
    currentFailureIds: [],
    results: {},
    ...overrides,
  };
}

function completeScan(states: {
  [K in AgentCatalogId]?:
    | AgentInstallReadiness["installState"]
    | "error"
    | "stale_failure";
}): AgentDirectoryScanView {
  const results: Partial<Record<AgentCatalogId, AgentInstallReadiness>> = {};
  const currentFailureIds: AgentCatalogId[] = [];
  for (const agentId of AGENT_CATALOG_IDS) {
    const value = states[agentId] ?? "not_installed";
    if (value === "error") {
      currentFailureIds.push(agentId);
      continue;
    }
    if (value === "stale_failure") {
      currentFailureIds.push(agentId);
      results[agentId] = readiness(agentId, "installed");
      continue;
    }
    results[agentId] = readiness(agentId, value);
  }
  return scanView({ results, currentFailureIds });
}

function orderedIds(
  scan: AgentDirectoryScanView,
  entries: readonly CatalogEntry[] = CATALOG,
): AgentCatalogId[] {
  return orderAgentDirectoryEntries(entries, scan).map((entry) => entry.id);
}

function idleScanState(
  overrides: Partial<AgentDirectoryScanState> = {},
): AgentDirectoryScanState {
  return {
    status: "idle",
    requestId: 0,
    settledIds: [],
    currentSuccessIds: [],
    currentFailureIds: [],
    results: {},
    lastSuccessfulScanAt: null,
    committedOrderIds: null,
    ...overrides,
  };
}

function finishScan(
  states: Partial<
    Record<AgentCatalogId, AgentInstallReadiness["installState"]>
  >,
): AgentDirectoryScanState {
  let state = scanReducer(idleScanState(), { type: "start", requestId: 1 });
  for (const agentId of AGENT_CATALOG_IDS) {
    state = scanReducer(state, {
      type: "settled",
      requestId: 1,
      agentId,
      data: readiness(agentId, states[agentId] ?? "not_installed"),
    });
  }
  return scanReducer(state, {
    type: "finish",
    requestId: 1,
    finishedAt: 1,
  });
}

describe("agentDirectoryPriority", () => {
  it("marks only QoderWork, TRAE Work, and WorkBuddy as domestic", () => {
    expect(agentDirectoryPriority("qoderwork")).toBe("domestic");
    expect(agentDirectoryPriority("trae-work")).toBe("domestic");
    expect(agentDirectoryPriority("workbuddy")).toBe("domestic");
    expect(agentDirectoryPriority("grokbuild")).toBe("standard");
    expect(agentDirectoryPriority("codex")).toBe("standard");
    expect(agentDirectoryPriority("claude-code")).toBe("standard");
    expect(agentDirectoryPriority("opencode")).toBe("standard");
    expect(agentDirectoryPriority("omp")).toBe("standard");
  });
});

describe("classifyAgentDirectoryOrderBucket", () => {
  it("classifies configurable vs official-only entries into closed buckets", () => {
    const cases: Array<{
      agentId: AgentCatalogId;
      scan: AgentDirectoryScanView;
      bucket: AgentDirectoryOrderBucket;
    }> = [
      {
        agentId: "workbuddy",
        scan: completeScan({ workbuddy: "installed" }),
        bucket: "installed_domestic",
      },
      {
        agentId: "qoderwork",
        scan: completeScan({ qoderwork: "installed" }),
        bucket: "official_only_installed",
      },
      {
        agentId: "trae-work",
        scan: completeScan({ "trae-work": "not_installed" }),
        bucket: "official_only_other",
      },
      {
        agentId: "codex",
        scan: completeScan({ codex: "installed" }),
        bucket: "installed_other",
      },
      {
        agentId: "omp",
        scan: completeScan({ omp: "installed_not_runnable" }),
        bucket: "installed_other",
      },
      {
        agentId: "workbuddy",
        scan: completeScan({ workbuddy: "not_installed" }),
        bucket: "not_installed",
      },
      {
        agentId: "claude-code",
        scan: completeScan({ "claude-code": "unavailable" }),
        bucket: "unresolved",
      },
      {
        agentId: "codex",
        scan: completeScan({ codex: "stale_failure" }),
        bucket: "unresolved",
      },
    ];

    for (const item of cases) {
      expect(classifyAgentDirectoryOrderBucket(item.agentId, item.scan)).toBe(
        item.bucket,
      );
    }
  });
});

describe("orderAgentDirectoryEntries", () => {
  it("places configurable installed entries first and demotes official-only entries to the bottom", () => {
    const scan = completeScan({
      qoderwork: "installed",
      "trae-work": "installed",
      workbuddy: "installed",
      grokbuild: "not_installed",
      codex: "installed",
      "claude-code": "not_installed",
      opencode: "installed",
      omp: "installed",
    });
    expect(orderedIds(scan)).toEqual([
      "workbuddy",
      "codex",
      "opencode",
      "omp",
      "claude-code",
      "grokbuild",
      "qoderwork",
      "trae-work",
    ]);
  });

  it("keeps official-only installed above official-only missing, both below configurable missing", () => {
    const scan = completeScan({
      qoderwork: "not_installed",
      "trae-work": "installed",
      workbuddy: "not_installed",
      grokbuild: "installed",
      codex: "installed",
      "claude-code": "not_installed",
      opencode: "not_installed",
      omp: "not_installed",
    });
    expect(orderedIds(scan)).toEqual([
      "codex",
      "grokbuild",
      "workbuddy",
      "claude-code",
      "opencode",
      "omp",
      "trae-work",
      "qoderwork",
    ]);
  });

  it("keeps canonical display order ties inside each bucket and before scan completes", () => {
    const allMissing = completeScan({});
    expect(orderedIds(allMissing)).toEqual([...AGENT_DISPLAY_ORDER]);
    expect(
      applyCommittedAgentDirectoryOrder(CATALOG, null).map((entry) => entry.id),
    ).toEqual([...AGENT_DISPLAY_ORDER]);
  });

  it("preserves committed order across start and commits a new order on finish", () => {
    let state = finishScan({
      codex: "installed",
      grokbuild: "installed",
    });
    const firstCommit = state.committedOrderIds;
    expect(firstCommit?.[0]).toBe("codex");

    state = scanReducer(state, { type: "start", requestId: 4 });
    expect(state.status).toBe("scanning");
    expect(state.committedOrderIds).toBe(firstCommit);

    for (const agentId of AGENT_CATALOG_IDS) {
      state = scanReducer(state, {
        type: "settled",
        requestId: 4,
        agentId,
        data: readiness(
          agentId,
          agentId === "workbuddy" ? "installed" : "not_installed",
        ),
      });
    }
    state = scanReducer(state, {
      type: "finish",
      requestId: 4,
      finishedAt: 4,
    });
    expect(state.committedOrderIds?.[0]).toBe("workbuddy");
    expect(state.committedOrderIds?.slice(-2)).toEqual([
      "qoderwork",
      "trae-work",
    ]);
    expect(
      nextCommittedAgentDirectoryOrderIds(
        scanView({ status: "scanning" }),
        state.committedOrderIds,
      ),
    ).toEqual(state.committedOrderIds);
  });
});
