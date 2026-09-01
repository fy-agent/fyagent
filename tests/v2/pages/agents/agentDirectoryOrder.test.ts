import { describe, expect, it } from "vitest";

import {
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentInstallReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import {
  AGENT_CATALOG_IDS,
  agentDirectoryPriority,
  type AgentCatalogId,
} from "@/v2/shared/features/directory";
import {
  applyCommittedAgentDirectoryOrder,
  classifyAgentDirectoryOrderBucket,
  nextCommittedAgentDirectoryOrderIds,
  orderAgentDirectoryEntries,
  type AgentDirectoryOrderBucket,
} from "@/v2/pages/agents/agentDirectoryOrder";
import type { AgentDirectoryScanView } from "@/v2/pages/agents/agentDirectoryScanProjection";
import {
  scanReducer,
  type AgentDirectoryScanState,
} from "@/v2/pages/agents/useAgentDirectoryScan";

type CatalogEntry = { id: AgentCatalogId };

const CATALOG: CatalogEntry[] = AGENT_CATALOG_IDS.map((id) => ({ id }));

function readiness(
  agentId: AgentCatalogId,
  installState: AgentInstallReadiness["installState"],
): AgentInstallReadiness {
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
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
  });
});

describe("classifyAgentDirectoryOrderBucket", () => {
  it("classifies every install state into the closed bucket set", () => {
    const cases: Array<{
      agentId: AgentCatalogId;
      scan: AgentDirectoryScanView;
      bucket: AgentDirectoryOrderBucket;
    }> = [
      {
        agentId: "qoderwork",
        scan: completeScan({ qoderwork: "installed" }),
        bucket: "installed_domestic",
      },
      {
        agentId: "qoderwork",
        scan: completeScan({ qoderwork: "installed_not_runnable" }),
        bucket: "installed_domestic",
      },
      {
        agentId: "codex",
        scan: completeScan({ codex: "installed" }),
        bucket: "installed_other",
      },
      {
        agentId: "opencode",
        scan: completeScan({ opencode: "installed_not_runnable" }),
        bucket: "installed_other",
      },
      {
        agentId: "workbuddy",
        scan: completeScan({ workbuddy: "not_installed" }),
        bucket: "not_installed",
      },
      {
        agentId: "trae-work",
        scan: completeScan({ "trae-work": "unknown" }),
        bucket: "unresolved",
      },
      {
        agentId: "claude-code",
        scan: completeScan({ "claude-code": "unavailable" }),
        bucket: "unresolved",
      },
      {
        agentId: "grokbuild",
        scan: completeScan({ grokbuild: "error" }),
        bucket: "unresolved",
      },
      {
        agentId: "codex",
        scan: scanView({ status: "scanning", settledIds: [], results: {} }),
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

  it("keeps current failure unresolved even when stale readiness is installed", () => {
    const scan = scanView({
      currentFailureIds: ["codex"],
      results: { codex: readiness("codex", "installed") },
    });
    expect(classifyAgentDirectoryOrderBucket("codex", scan)).toBe("unresolved");
    expect(scan.results.codex?.installState).toBe("installed");
  });
});

describe("orderAgentDirectoryEntries", () => {
  it("orders example A: installed domestic, then installed other, then not installed", () => {
    const scan = completeScan({
      qoderwork: "not_installed",
      "trae-work": "installed",
      workbuddy: "not_installed",
      grokbuild: "installed",
      codex: "installed",
      "claude-code": "not_installed",
      opencode: "installed",
    });
    expect(orderedIds(scan)).toEqual([
      "trae-work",
      "grokbuild",
      "codex",
      "opencode",
      "qoderwork",
      "workbuddy",
      "claude-code",
    ]);
  });

  it("does not let uninstalled domestic products jump over installed non-domestic products", () => {
    const scan = completeScan({
      qoderwork: "not_installed",
      "trae-work": "not_installed",
      workbuddy: "not_installed",
      grokbuild: "installed",
      codex: "installed",
      "claude-code": "not_installed",
      opencode: "not_installed",
    });
    expect(orderedIds(scan)).toEqual([
      "grokbuild",
      "codex",
      "qoderwork",
      "trae-work",
      "workbuddy",
      "claude-code",
      "opencode",
    ]);
  });

  it("orders example B with installed_not_runnable, unknown, error, and unavailable", () => {
    const scan = completeScan({
      qoderwork: "installed_not_runnable",
      "trae-work": "unknown",
      workbuddy: "installed",
      grokbuild: "error",
      codex: "not_installed",
      "claude-code": "unavailable",
      opencode: "installed",
    });
    expect(orderedIds(scan)).toEqual([
      "qoderwork",
      "workbuddy",
      "opencode",
      "trae-work",
      "grokbuild",
      "claude-code",
      "codex",
    ]);
  });

  it("keeps canonical ties inside each bucket", () => {
    const allInstalled = completeScan({
      qoderwork: "installed",
      "trae-work": "installed_not_runnable",
      workbuddy: "installed",
      grokbuild: "installed",
      codex: "installed_not_runnable",
      "claude-code": "installed",
      opencode: "installed",
    });
    expect(orderedIds(allInstalled)).toEqual([...AGENT_CATALOG_IDS]);

    const allMissing = completeScan({
      qoderwork: "not_installed",
      "trae-work": "not_installed",
      workbuddy: "not_installed",
      grokbuild: "not_installed",
      codex: "not_installed",
      "claude-code": "not_installed",
      opencode: "not_installed",
    });
    expect(orderedIds(allMissing)).toEqual([...AGENT_CATALOG_IDS]);

    const allUnresolved = completeScan({
      qoderwork: "unknown",
      "trae-work": "unavailable",
      workbuddy: "error",
      grokbuild: "unknown",
      codex: "unavailable",
      "claude-code": "unavailable",
      opencode: "unknown",
    });
    expect(orderedIds(allUnresolved)).toEqual([...AGENT_CATALOG_IDS]);

    const noCurrentResults = scanView({
      results: {},
      currentFailureIds: [],
    });
    expect(orderedIds(noCurrentResults)).toEqual([...AGENT_CATALOG_IDS]);
  });

  it("sorts a stale installed current failure below confirmed installed rows", () => {
    const scan = completeScan({
      qoderwork: "installed",
      "trae-work": "not_installed",
      workbuddy: "not_installed",
      grokbuild: "not_installed",
      codex: "stale_failure",
      "claude-code": "not_installed",
      opencode: "not_installed",
    });
    expect(orderedIds(scan)).toEqual([
      "qoderwork",
      "codex",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "claude-code",
      "opencode",
    ]);
  });

  it("does not mutate the input catalog array or entry objects", () => {
    const entries = CATALOG.map((entry) => ({ id: entry.id }));
    const snapshot = entries.map((entry) => entry.id);
    Object.freeze(entries);
    const scan = completeScan({
      "trae-work": "installed",
      grokbuild: "installed",
      qoderwork: "not_installed",
    });
    const ordered = orderAgentDirectoryEntries(entries, scan);
    expect(ordered).not.toBe(entries);
    expect(entries.map((entry) => entry.id)).toEqual(snapshot);
    expect(ordered[0]).toBe(entries[1]);
    expect(ordered.map((entry) => entry.id)).toEqual([
      "trae-work",
      "grokbuild",
      "qoderwork",
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });
});

describe("committed directory order lifecycle", () => {
  it("keeps canonical order while the first scan is incomplete", () => {
    const scanning = scanView({
      status: "scanning",
      settledIds: ["qoderwork"],
      results: { qoderwork: readiness("qoderwork", "installed") },
    });
    expect(nextCommittedAgentDirectoryOrderIds(scanning, null)).toBeNull();
    expect(
      applyCommittedAgentDirectoryOrder(CATALOG, null).map((entry) => entry.id),
    ).toEqual([...AGENT_CATALOG_IDS]);
  });

  it("does not reorder as individual rows settle", () => {
    const previous = nextCommittedAgentDirectoryOrderIds(
      scanView({ status: "scanning", settledIds: [], results: {} }),
      null,
    );
    const midScan = scanView({
      status: "scanning",
      settledIds: ["qoderwork", "codex"],
      results: {
        qoderwork: readiness("qoderwork", "not_installed"),
        codex: readiness("codex", "installed"),
      },
    });
    expect(nextCommittedAgentDirectoryOrderIds(midScan, previous)).toBeNull();
    expect(
      applyCommittedAgentDirectoryOrder(CATALOG, previous).map(
        (entry) => entry.id,
      ),
    ).toEqual([...AGENT_CATALOG_IDS]);
  });

  it("commits bucket order once the first scan completes", () => {
    const scan = completeScan({
      qoderwork: "not_installed",
      "trae-work": "installed",
      grokbuild: "installed",
    });
    expect(nextCommittedAgentDirectoryOrderIds(scan, null)).toEqual([
      "trae-work",
      "grokbuild",
      "qoderwork",
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });

  it("freezes the previous committed order during a rescan", () => {
    const committed = orderedIds(
      completeScan({
        "trae-work": "installed",
        grokbuild: "installed",
      }),
    );
    const rescan = scanView({
      status: "scanning",
      settledIds: ["qoderwork"],
      results: {
        qoderwork: readiness("qoderwork", "installed"),
        "trae-work": readiness("trae-work", "installed"),
        grokbuild: readiness("grokbuild", "installed"),
      },
    });
    expect(nextCommittedAgentDirectoryOrderIds(rescan, committed)).toEqual(
      committed,
    );
    expect(
      applyCommittedAgentDirectoryOrder(CATALOG, committed).map(
        (entry) => entry.id,
      ),
    ).toEqual(committed);
  });

  it("replaces the committed order once when a rescan completes", () => {
    const previous = orderedIds(
      completeScan({
        "trae-work": "installed",
        grokbuild: "installed",
      }),
    );
    const rescanComplete = completeScan({
      qoderwork: "installed",
      grokbuild: "not_installed",
    });
    expect(
      nextCommittedAgentDirectoryOrderIds(rescanComplete, previous),
    ).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });

  it("reorders after an authoritative applyReadiness while not scanning", () => {
    let state = finishScan({
      qoderwork: "not_installed",
      grokbuild: "installed",
    });
    expect(state.committedOrderIds).toEqual([
      "grokbuild",
      "qoderwork",
      "trae-work",
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
    ]);

    state = scanReducer(state, {
      type: "applyReadiness",
      agentId: "qoderwork",
      data: readiness("qoderwork", "installed"),
    });
    expect(state.status).toBe("complete");
    expect(state.committedOrderIds).toEqual([
      "qoderwork",
      "grokbuild",
      "trae-work",
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });

  it("does not commit a new order from applyReadiness during a scan", () => {
    let state = finishScan({ grokbuild: "installed" });
    const frozen = state.committedOrderIds;
    state = scanReducer(state, { type: "start", requestId: 3 });
    state = scanReducer(state, {
      type: "applyReadiness",
      agentId: "qoderwork",
      data: readiness("qoderwork", "installed"),
    });
    expect(state.status).toBe("scanning");
    expect(state.committedOrderIds).toEqual(frozen);
    expect(state.committedOrderIds).toEqual([
      "grokbuild",
      "qoderwork",
      "trae-work",
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });
});

describe("scanReducer committed order", () => {
  it("preserves committed order across start and commits a new order on finish", () => {
    let state = finishScan({
      "trae-work": "installed",
      grokbuild: "installed",
    });
    const firstCommit = state.committedOrderIds;
    expect(firstCommit?.[0]).toBe("trae-work");

    state = scanReducer(state, { type: "start", requestId: 4 });
    expect(state.status).toBe("scanning");
    expect(state.committedOrderIds).toBe(firstCommit);

    state = scanReducer(state, {
      type: "settled",
      requestId: 4,
      agentId: "qoderwork",
      data: readiness("qoderwork", "installed"),
    });
    expect(state.committedOrderIds).toBe(firstCommit);

    for (const agentId of AGENT_CATALOG_IDS) {
      if (agentId === "qoderwork") continue;
      state = scanReducer(state, {
        type: "settled",
        requestId: 4,
        agentId,
        data: readiness(agentId, "not_installed"),
      });
    }
    state = scanReducer(state, {
      type: "finish",
      requestId: 4,
      finishedAt: 4,
    });
    expect(state.committedOrderIds).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude-code",
      "opencode",
    ]);
  });
});
