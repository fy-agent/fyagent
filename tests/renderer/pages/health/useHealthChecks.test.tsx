import { act, renderHook, waitFor } from "@testing-library/react";
import { type ReactNode, StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useHealthChecks } from "@/pages/health/useHealthChecks";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "@/shared/features/directory";
import { healthStatus } from "@/shared/features/health-presentation";
import { HEALTH_STALE_AFTER_MS } from "@/shared/features/health";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { healthSnapshotFixture } from "../../fixtures/health";

function deferred() {
  let resolve = () => {};
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

afterEach(() => vi.useRealTimers());

describe("health read dispatch and retained facts", () => {
  it("only reads the selected Agent on first visit, including StrictMode", async () => {
    const ports = createBrowserFeaturePorts();
    ports.health.get = vi.fn(async (id) => healthSnapshotFixture(id));
    const wrapper = ({ children }: { children: ReactNode }) => (
      <StrictMode>
        <FeatureProvider ports={ports}>{children}</FeatureProvider>
      </StrictMode>
    );
    const { result } = renderHook(() => useHealthChecks("codex", true), {
      wrapper,
    });
    await waitFor(() => expect(result.current.busy).toBe(false));
    await waitFor(() => expect(ports.health.get).toHaveBeenCalledTimes(1));
    expect(ports.health.get).toHaveBeenCalledWith("codex");
    expect(result.current.queries.filter((query) => query.data)).toHaveLength(
      1,
    );
  });

  it("serializes a batch, rejects duplicate dispatch and preserves success through a partial failure", async () => {
    const ports = createBrowserFeaturePorts();
    let batch = false;
    let concurrent = 0;
    let peak = 0;
    const gate = deferred();
    ports.health.get = vi.fn(async (id) => {
      concurrent += 1;
      peak = Math.max(peak, concurrent);
      try {
        if (batch && id === "qoderwork") await gate.promise;
        if (batch && id === "codex") throw new Error("private raw error");
        return healthSnapshotFixture(id);
      } finally {
        concurrent -= 1;
      }
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <FeatureProvider ports={ports}>{children}</FeatureProvider>
    );
    const { result } = renderHook(() => useHealthChecks("codex", true), {
      wrapper,
    });
    await waitFor(() => expect(result.current.queries[4].data).toBeDefined());
    const previous = result.current.queries[4].data;
    batch = true;
    act(() => {
      void result.current.refreshAll();
      void result.current.refreshSelected();
    });
    await waitFor(() => expect(ports.health.get).toHaveBeenCalledTimes(2));
    await act(async () => {
      gate.resolve();
    });
    await waitFor(() =>
      expect(result.current.progress?.state).toBe("complete"),
    );
    expect(peak).toBe(1);
    expect(vi.mocked(ports.health.get).mock.calls.map(([id]) => id)).toEqual([
      "codex",
      ...AGENT_CATALOG_IDS,
    ]);
    expect(result.current.progress?.failed).toBe(1);
    expect(
      result.current.queries.every((query) => query.data !== undefined),
    ).toBe(true);
    expect(result.current.queries[4].data).toBe(previous);
    expect(
      healthStatus(
        previous!,
        result.current.now,
        result.current.queries[4].isError,
      ),
    ).toBe("stale");
  });

  it("stops further dispatch and does not resume a cancelled batch", async () => {
    const ports = createBrowserFeaturePorts();
    const gate = deferred();
    ports.health.get = vi.fn(async (id) => {
      if (id === "qoderwork") await gate.promise;
      return healthSnapshotFixture(id);
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <FeatureProvider ports={ports}>{children}</FeatureProvider>
    );
    const { result } = renderHook(() => useHealthChecks("codex", true), {
      wrapper,
    });
    await waitFor(() => expect(result.current.queries[4].data).toBeDefined());
    act(() => {
      void result.current.refreshAll();
      result.current.stop();
    });
    await act(async () => {
      gate.resolve();
    });
    await waitFor(() => expect(result.current.progress?.state).toBe("stopped"));
    expect(vi.mocked(ports.health.get).mock.calls.map(([id]) => id)).toEqual([
      "codex",
      "qoderwork",
    ]);
  });

  it("revokes hidden work and waits for the outstanding read before refreshing the selected Agent on return", async () => {
    const ports = createBrowserFeaturePorts();
    const gate = deferred();
    ports.health.get = vi.fn(async (id) => {
      if (id === "qoderwork") await gate.promise;
      return healthSnapshotFixture(id);
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <FeatureProvider ports={ports}>{children}</FeatureProvider>
    );
    const { result, rerender } = renderHook(
      ({ visible, selected }: { visible: boolean; selected: AgentCatalogId }) =>
        useHealthChecks(selected, visible),
      { wrapper, initialProps: { visible: false, selected: "codex" } },
    );
    await act(async () => {
      await result.current.refreshAll();
    });
    expect(ports.health.get).not.toHaveBeenCalled();
    rerender({ visible: true, selected: "codex" });
    await waitFor(() => expect(result.current.queries[4].data).toBeDefined());
    act(() => {
      void result.current.refreshAll();
    });
    rerender({ visible: false, selected: "codex" });
    rerender({ visible: true, selected: "claude-code" });
    expect(ports.health.get).toHaveBeenCalledTimes(2);
    await act(async () => {
      gate.resolve();
    });
    await waitFor(() => expect(result.current.queries[5].data).toBeDefined());
    expect(vi.mocked(ports.health.get).mock.calls.map(([id]) => id)).toEqual([
      "codex",
      "qoderwork",
      "claude-code",
    ]);
  });

  it("ages visible results after five minutes without dispatching another native read", async () => {
    const ports = createBrowserFeaturePorts();
    ports.health.get = vi.fn(async (id) => healthSnapshotFixture(id));
    const wrapper = ({ children }: { children: ReactNode }) => (
      <FeatureProvider ports={ports}>{children}</FeatureProvider>
    );
    vi.useFakeTimers();
    const { result } = renderHook(() => useHealthChecks("codex", true), {
      wrapper,
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    const snapshot = result.current.queries[4].data;
    expect(snapshot).toBeDefined();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HEALTH_STALE_AFTER_MS);
    });
    expect(healthStatus(snapshot!, result.current.now)).toBe("stale");
    expect(ports.health.get).toHaveBeenCalledTimes(1);
  });

  it("marks an expired snapshot stale immediately on return while the fresh read is pending", async () => {
    const ports = createBrowserFeaturePorts();
    const gate = deferred();
    let reads = 0;
    ports.health.get = vi.fn(async (id) => {
      reads += 1;
      if (reads > 1) await gate.promise;
      return healthSnapshotFixture(id);
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <FeatureProvider ports={ports}>{children}</FeatureProvider>
    );
    vi.useFakeTimers();
    const { result, rerender } = renderHook(
      ({ visible }) => useHealthChecks("codex", visible),
      { wrapper, initialProps: { visible: true } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    const snapshot = result.current.queries[4].data;
    expect(snapshot).toBeDefined();
    rerender({ visible: false });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HEALTH_STALE_AFTER_MS + 1);
    });
    expect(ports.health.get).toHaveBeenCalledTimes(1);
    await act(async () => {
      rerender({ visible: true });
    });
    expect(healthStatus(snapshot!, result.current.now)).toBe("stale");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(ports.health.get).toHaveBeenCalledTimes(2);
    expect(result.current.busy).toBe(true);
    expect(result.current.queries[4].data).toBe(snapshot);
    await act(async () => {
      gate.resolve();
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(
      healthStatus(result.current.queries[4].data!, result.current.now),
    ).toBe("ready");
  });
});
