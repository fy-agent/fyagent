import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentInstallReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import { useAgentInstallReadiness } from "@/v2/shared/features/queries";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "@/v2/shared/features/types";

vi.mock("@/v2/shared/features/queries", () => ({
  useAgentInstallReadiness: vi.fn(),
}));

import {
  observeAgentDirectoryRow,
  scanReducer,
  useAgentDirectoryScan,
  type AgentDirectoryScanState,
} from "@/v2/pages/agents/useAgentDirectoryScan";

type RefetchResult = {
  data?: AgentInstallReadiness;
  error?: unknown;
};

function readiness(
  agentId: AgentCatalogId,
  installState: AgentInstallReadiness["installState"],
): AgentInstallReadiness {
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    agentId,
    reviewedAt: "2026-08-29",
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
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
    ...overrides,
  };
}

function mockRefetchMap(
  refetchById: Record<AgentCatalogId, () => Promise<RefetchResult>>,
) {
  vi.mocked(useAgentInstallReadiness).mockImplementation((agentId) => {
    return {
      refetch: refetchById[agentId],
    } as ReturnType<typeof useAgentInstallReadiness>;
  });
}

function resolvingRefetch(
  installState: AgentInstallReadiness["installState"] = "not_installed",
) {
  const refetchById = {} as Record<
    AgentCatalogId,
    ReturnType<typeof vi.fn<() => Promise<RefetchResult>>>
  >;
  for (const agentId of AGENT_CATALOG_IDS) {
    refetchById[agentId] = vi.fn(async () => ({
      data: readiness(agentId, installState),
      error: null,
    }));
  }
  mockRefetchMap(refetchById);
  return refetchById;
}

describe("observeAgentDirectoryRow", () => {
  it("treats idle and in-flight rows without results as pending, not missing", () => {
    const idle = observeAgentDirectoryRow("codex", idleScanState());
    expect(idle).toMatchObject({
      kind: "pending",
      scanning: false,
      refreshing: false,
      configurable: false,
      readFailed: false,
    });

    const scanning = observeAgentDirectoryRow(
      "codex",
      idleScanState({ status: "scanning", requestId: 1 }),
    );
    expect(scanning).toMatchObject({
      kind: "pending",
      scanning: true,
      configurable: false,
    });
  });

  it("marks installed and installed_not_runnable as configurable existence", () => {
    const installed = observeAgentDirectoryRow(
      "codex",
      idleScanState({
        status: "complete",
        requestId: 1,
        settledIds: ["codex"],
        currentSuccessIds: ["codex"],
        results: { codex: readiness("codex", "installed") },
        lastSuccessfulScanAt: 1,
      }),
    );
    expect(installed).toMatchObject({
      kind: "installed",
      configurable: true,
      scanning: false,
    });

    const notRunnable = observeAgentDirectoryRow(
      "opencode",
      idleScanState({
        status: "complete",
        requestId: 1,
        settledIds: ["opencode"],
        currentSuccessIds: ["opencode"],
        results: { opencode: readiness("opencode", "installed_not_runnable") },
        lastSuccessfulScanAt: 1,
      }),
    );
    expect(notRunnable).toMatchObject({
      kind: "installed",
      configurable: true,
    });
  });

  it("marks a settled technical failure as error while other rows are still scanning", () => {
    const observation = observeAgentDirectoryRow(
      "codex",
      idleScanState({
        status: "scanning",
        requestId: 1,
        settledIds: ["codex"],
        currentFailureIds: ["codex"],
      }),
    );
    expect(observation).toMatchObject({
      kind: "error",
      scanning: false,
      configurable: false,
      readFailed: true,
    });
  });

  it("keeps not_installed, unknown, and unavailable distinct from read failure", () => {
    const base = {
      status: "complete" as const,
      requestId: 1,
      lastSuccessfulScanAt: 1,
    };

    expect(
      observeAgentDirectoryRow(
        "qoderwork",
        idleScanState({
          ...base,
          settledIds: ["qoderwork"],
          currentSuccessIds: ["qoderwork"],
          results: { qoderwork: readiness("qoderwork", "not_installed") },
        }),
      ),
    ).toMatchObject({ kind: "not_installed", configurable: false });

    expect(
      observeAgentDirectoryRow(
        "trae-work",
        idleScanState({
          ...base,
          settledIds: ["trae-work"],
          currentSuccessIds: ["trae-work"],
          results: { "trae-work": readiness("trae-work", "unknown") },
        }),
      ),
    ).toMatchObject({ kind: "unknown", configurable: false });

    expect(
      observeAgentDirectoryRow(
        "workbuddy",
        idleScanState({
          ...base,
          settledIds: ["workbuddy"],
          currentSuccessIds: ["workbuddy"],
          results: { workbuddy: readiness("workbuddy", "unavailable") },
        }),
      ),
    ).toMatchObject({ kind: "unavailable", configurable: false });

    expect(
      observeAgentDirectoryRow(
        "grokbuild",
        idleScanState({
          ...base,
          settledIds: ["grokbuild"],
          currentFailureIds: ["grokbuild"],
        }),
      ),
    ).toMatchObject({
      kind: "error",
      configurable: false,
      readFailed: true,
      readiness: undefined,
    });
  });

  it("retains configurability while a previous installed result is refreshing", () => {
    const observation = observeAgentDirectoryRow(
      "codex",
      idleScanState({
        status: "scanning",
        requestId: 2,
        results: { codex: readiness("codex", "installed") },
        lastSuccessfulScanAt: 1,
      }),
    );
    expect(observation).toMatchObject({
      kind: "installed",
      scanning: true,
      refreshing: true,
      configurable: true,
      readFailed: false,
    });
  });

  it("does not convert a later technical failure into not-installed", () => {
    const observation = observeAgentDirectoryRow(
      "codex",
      idleScanState({
        status: "complete",
        requestId: 2,
        settledIds: ["codex"],
        currentFailureIds: ["codex"],
        results: { codex: readiness("codex", "installed") },
        lastSuccessfulScanAt: 1,
      }),
    );
    expect(observation.kind).toBe("installed");
    expect(observation.kind).not.toBe("not_installed");
    expect(observation.configurable).toBe(true);
    expect(observation.readFailed).toBe(true);
    expect(observation.readiness?.installState).toBe("installed");
  });
});

describe("scanReducer", () => {
  it("ignores settled completions from a previous requestId", () => {
    let state = scanReducer(idleScanState(), { type: "start", requestId: 1 });
    state = scanReducer(state, {
      type: "settled",
      requestId: 1,
      agentId: "codex",
      data: readiness("codex", "installed"),
    });
    state = scanReducer(state, {
      type: "finish",
      requestId: 1,
      finishedAt: 10,
    });
    state = scanReducer(state, { type: "start", requestId: 2 });

    const ignored = scanReducer(state, {
      type: "settled",
      requestId: 1,
      agentId: "codex",
      data: readiness("codex", "not_installed"),
    });

    expect(ignored).toBe(state);
    expect(ignored.results.codex?.installState).toBe("installed");
    expect(ignored.settledIds).toEqual([]);
  });

  it("patches readiness without resetting scan status or request identity", () => {
    const started = scanReducer(idleScanState(), {
      type: "start",
      requestId: 3,
    });
    const patched = scanReducer(started, {
      type: "applyReadiness",
      agentId: "qoderwork",
      data: readiness("qoderwork", "installed"),
    });

    expect(patched.status).toBe("scanning");
    expect(patched.requestId).toBe(3);
    expect(patched.settledIds).toEqual([]);
    expect(patched.results.qoderwork?.installState).toBe("installed");
    expect(observeAgentDirectoryRow("qoderwork", patched).configurable).toBe(
      true,
    );
  });
});

describe("useAgentDirectoryScan", () => {
  beforeEach(() => {
    resolvingRefetch("not_installed");
  });

  it("does not refetch when autoStart is false", () => {
    const refetchById = resolvingRefetch("not_installed");
    const { result } = renderHook(() => useAgentDirectoryScan());

    expect(result.current.state.status).toBe("idle");
    for (const agentId of AGENT_CATALOG_IDS) {
      expect(refetchById[agentId]).not.toHaveBeenCalled();
      expect(observeAgentDirectoryRow(agentId, result.current.state).kind).toBe(
        "pending",
      );
    }
  });

  it("auto-starts once and ignores start() while scanning", async () => {
    const pending = deferred<AgentInstallReadiness>();
    const refetchById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof vi.fn<() => Promise<RefetchResult>>>
    >;
    for (const agentId of AGENT_CATALOG_IDS) {
      refetchById[agentId] = vi.fn(() =>
        pending.promise.then((data) => ({
          data: { ...data, agentId },
          error: null,
        })),
      );
    }
    mockRefetchMap(refetchById);

    const { result } = renderHook(() =>
      useAgentDirectoryScan({ autoStart: true }),
    );

    await waitFor(() => expect(result.current.state.status).toBe("scanning"));
    expect(refetchById.codex).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.start();
    });
    expect(refetchById.codex).toHaveBeenCalledTimes(1);

    await act(async () => {
      pending.resolve(readiness("codex", "installed"));
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
  });

  it("updates a settled row before the rest of the scan finishes", async () => {
    const pendingById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof deferred<AgentInstallReadiness>>
    >;
    const refetchById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof vi.fn<() => Promise<RefetchResult>>>
    >;
    for (const agentId of AGENT_CATALOG_IDS) {
      pendingById[agentId] = deferred<AgentInstallReadiness>();
      refetchById[agentId] = vi.fn(() =>
        pendingById[agentId].promise.then((data) => ({
          data,
          error: null,
        })),
      );
    }
    mockRefetchMap(refetchById);

    const { result } = renderHook(() => useAgentDirectoryScan());
    act(() => {
      result.current.start();
    });

    await waitFor(() => expect(result.current.state.status).toBe("scanning"));

    await act(async () => {
      pendingById.qoderwork.resolve(readiness("qoderwork", "installed"));
    });

    await waitFor(() => {
      const row = observeAgentDirectoryRow("qoderwork", result.current.state);
      expect(row.kind).toBe("installed");
      expect(row.configurable).toBe(true);
      expect(row.scanning).toBe(false);
    });

    expect(result.current.state.status).toBe("scanning");
    expect(observeAgentDirectoryRow("codex", result.current.state).kind).toBe(
      "pending",
    );
    expect(
      observeAgentDirectoryRow("codex", result.current.state).scanning,
    ).toBe(true);

    await act(async () => {
      for (const agentId of AGENT_CATALOG_IDS.slice(1)) {
        pendingById[agentId].resolve(readiness(agentId, "not_installed"));
      }
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
  });

  it("marks a failed row as error before the remaining queries settle", async () => {
    const pendingById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof deferred<AgentInstallReadiness>>
    >;
    const refetchById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof vi.fn<() => Promise<RefetchResult>>>
    >;
    for (const agentId of AGENT_CATALOG_IDS) {
      pendingById[agentId] = deferred<AgentInstallReadiness>();
      refetchById[agentId] = vi.fn(() =>
        pendingById[agentId].promise.then((data) => ({
          data,
          error: null,
        })),
      );
    }
    mockRefetchMap(refetchById);

    const { result } = renderHook(() => useAgentDirectoryScan());
    act(() => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("scanning"));

    await act(async () => {
      pendingById.codex.reject(new Error("readiness offline"));
    });

    await waitFor(() => {
      const row = observeAgentDirectoryRow("codex", result.current.state);
      expect(row.kind).toBe("error");
      expect(row.kind).not.toBe("not_installed");
      expect(row.readFailed).toBe(true);
      expect(row.scanning).toBe(false);
    });
    expect(
      observeAgentDirectoryRow("qoderwork", result.current.state).kind,
    ).toBe("pending");

    await act(async () => {
      for (const agentId of AGENT_CATALOG_IDS) {
        if (agentId === "codex") continue;
        pendingById[agentId].resolve(readiness(agentId, "not_installed"));
      }
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
  });

  it("distinguishes a partial technical failure from not-installed", async () => {
    const refetchById = {} as Record<
      AgentCatalogId,
      ReturnType<typeof vi.fn<() => Promise<RefetchResult>>>
    >;
    for (const agentId of AGENT_CATALOG_IDS) {
      refetchById[agentId] = vi.fn(async () => {
        if (agentId === "codex") {
          throw new Error("readiness offline");
        }
        return {
          data: readiness(
            agentId,
            agentId === "qoderwork" ? "not_installed" : "installed",
          ),
          error: null,
        };
      });
    }
    mockRefetchMap(refetchById);

    const { result } = renderHook(() => useAgentDirectoryScan());
    await act(async () => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));

    const failed = observeAgentDirectoryRow("codex", result.current.state);
    expect(failed.kind).toBe("error");
    expect(failed.kind).not.toBe("not_installed");
    expect(failed.readFailed).toBe(true);
    expect(failed.configurable).toBe(false);
    expect(result.current.state.results.codex).toBeUndefined();

    const missing = observeAgentDirectoryRow("qoderwork", result.current.state);
    expect(missing.kind).toBe("not_installed");
    expect(missing.readFailed).toBe(false);
    expect(result.current.state.currentFailureIds).toEqual(["codex"]);
  });

  it("retains previous successful results when a later scan fails entirely", async () => {
    const refetchById = resolvingRefetch("installed");
    const { result } = renderHook(() => useAgentDirectoryScan());

    await act(async () => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
    expect(
      observeAgentDirectoryRow("codex", result.current.state).configurable,
    ).toBe(true);

    for (const agentId of AGENT_CATALOG_IDS) {
      refetchById[agentId].mockRejectedValueOnce(new Error("offline"));
    }

    await act(async () => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));

    expect(result.current.state.currentSuccessIds).toEqual([]);
    expect(result.current.state.currentFailureIds).toEqual([
      ...AGENT_CATALOG_IDS,
    ]);
    for (const agentId of AGENT_CATALOG_IDS) {
      const row = observeAgentDirectoryRow(agentId, result.current.state);
      expect(row.kind).toBe("installed");
      expect(row.kind).not.toBe("not_installed");
      expect(row.configurable).toBe(true);
      expect(row.readFailed).toBe(true);
      expect(row.readiness?.installState).toBe("installed");
    }
  });

  it("keeps a retained installed result visible during rescan", async () => {
    const refetchById = resolvingRefetch("installed");
    const { result } = renderHook(() => useAgentDirectoryScan());

    await act(async () => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));

    const pending = deferred<AgentInstallReadiness>();
    for (const agentId of AGENT_CATALOG_IDS) {
      refetchById[agentId].mockImplementation(() =>
        pending.promise.then((data) => ({
          data: { ...data, agentId },
          error: null,
        })),
      );
    }

    act(() => {
      result.current.start();
    });

    await waitFor(() => expect(result.current.state.status).toBe("scanning"));
    const row = observeAgentDirectoryRow("codex", result.current.state);
    expect(row).toMatchObject({
      kind: "installed",
      scanning: true,
      refreshing: true,
      configurable: true,
    });

    await act(async () => {
      pending.resolve(readiness("codex", "installed"));
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
  });

  it("lets applyReadiness write an authoritative reread into retained results", async () => {
    const refetchById = resolvingRefetch("not_installed");
    const { result } = renderHook(() => useAgentDirectoryScan());

    await act(async () => {
      result.current.start();
    });
    await waitFor(() => expect(result.current.state.status).toBe("complete"));
    expect(
      observeAgentDirectoryRow("qoderwork", result.current.state).configurable,
    ).toBe(false);

    act(() => {
      result.current.applyReadiness(
        "qoderwork",
        readiness("qoderwork", "installed"),
      );
    });

    const row = observeAgentDirectoryRow("qoderwork", result.current.state);
    expect(row.kind).toBe("installed");
    expect(row.configurable).toBe(true);
    expect(result.current.state.status).toBe("complete");
    expect(refetchById.qoderwork).toHaveBeenCalledTimes(1);
  });
});
