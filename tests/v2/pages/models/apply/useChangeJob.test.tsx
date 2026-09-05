import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useChangeJob } from "@/v2/pages/models/apply/useChangeJob";
import type {
  ChangeJobSnapshot,
  ChangePlansPort,
} from "@/v2/shared/features/change-plans";
import { featureKeys } from "@/v2/shared/features/queries";
import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { changeJobWire } from "../../../fixtures/changePlans";

const running: ChangeJobSnapshot = { ...changeJobWire, status: "running" };
const terminal: ChangeJobSnapshot = {
  ...running,
  status: "succeeded",
  resultCode: "applied",
};

function deferred() {
  let resolve!: (snapshot: ChangeJobSnapshot) => void;
  const promise = new Promise<ChangeJobSnapshot>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

describe("Change Job Query lifecycle", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // The macOS 12 minimum cannot assume newer AbortSignal convenience methods.
    vi.spyOn(AbortSignal.prototype, "throwIfAborted").mockImplementation(() => {
      throw new Error("throwIfAborted is unavailable in the minimum WebView");
    });
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  function harness(read: ChangePlansPort["getChangeJob"]) {
    const client = new QueryClient();
    const port = {
      ...createBrowserFeaturePorts().changePlans,
      getChangeJob: read,
    };
    let visible = true;
    const view = renderHook(
      ({ active }: { active: boolean }) => useChangeJob(port, active),
      {
        initialProps: { active: true },
        wrapper: ({ children }: { children: ReactNode }) => (
          <QueryClientProvider client={client}>
            <PersistentSurface active={visible}>{children}</PersistentSurface>
          </QueryClientProvider>
        ),
      },
    );
    act(() => view.result.current.setJob(running));
    return {
      ...view,
      client,
      setVisible: (next: boolean) => {
        visible = next;
        view.rerender({ active: true });
      },
    };
  }

  it("does not overlap a slow read and stops after a terminal snapshot", async () => {
    const pending = deferred();
    const read = vi.fn(() => pending.promise);
    const { result } = harness(read);
    expect(read).not.toHaveBeenCalled();
    await advance(1_000);
    expect(read).toHaveBeenCalledTimes(1);
    await advance(5_000);
    expect(read).toHaveBeenCalledTimes(1);
    await act(async () => {
      pending.resolve(terminal);
    });
    await advance(5_000);
    expect(result.current.job).toEqual(terminal);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("pauses hidden persistent pages, ignores their in-flight result, and resumes", async () => {
    const pending = deferred();
    const read = vi
      .fn()
      .mockImplementationOnce(() => pending.promise)
      .mockResolvedValue(terminal);
    const { result, setVisible } = harness(read);
    await advance(1_000);
    setVisible(false);
    await act(async () => {
      pending.resolve(terminal);
    });
    await advance(5_000);
    expect(read).toHaveBeenCalledTimes(1);
    expect(result.current.job).toEqual(running);
    setVisible(true);
    await advance(1_001);
    expect(read).toHaveBeenCalledTimes(2);
    expect(result.current.job).toEqual(terminal);
  });

  it("stops on a sanitized error without discarding the last authoritative snapshot", async () => {
    const secret = "never-cache-raw-native-diagnostics";
    const read = vi.fn().mockRejectedValue(new Error(secret));
    const { result, client } = harness(read);
    await advance(1_001);
    expect(result.current.error).toEqual({ code: "internal" });
    expect(result.current.job).toEqual(running);
    await advance(5_000);
    expect(read).toHaveBeenCalledTimes(1);
    expect(
      JSON.stringify(
        client
          .getQueryCache()
          .getAll()
          .map((query) => query.state),
      ),
    ).not.toContain(secret);
    expect(client.getMutationCache().getAll()).toHaveLength(0);
  });

  it("clearing or unmounting rejects a late read and releases the job cache", async () => {
    const pending = deferred();
    const { result, client, unmount } = harness(vi.fn(() => pending.promise));
    await advance(1_000);
    act(() => result.current.setJob(null));
    await act(async () => {
      pending.resolve(terminal);
    });
    await advance(10);
    expect(result.current.job).toBeNull();
    unmount();
    await advance(10);
    expect(
      client.getQueryData(featureKeys.changeJob(running.jobId)),
    ).toBeUndefined();
  });

  it("shares one read between observers and hiding one does not cancel the other", async () => {
    const pending = deferred();
    const read = vi.fn(() => pending.promise);
    const client = new QueryClient();
    const port = {
      ...createBrowserFeaturePorts().changePlans,
      getChangeJob: read,
    };
    const { result, rerender } = renderHook(
      ({ active }: { active: boolean }) => ({
        first: useChangeJob(port, active),
        second: useChangeJob(port, true),
      }),
      {
        initialProps: { active: true },
        wrapper: ({ children }: { children: ReactNode }) => (
          <QueryClientProvider client={client}>{children}</QueryClientProvider>
        ),
      },
    );
    act(() => {
      result.current.first.setJob(running);
      result.current.second.setJob(running);
    });
    await advance(1_000);
    expect(read).toHaveBeenCalledTimes(1);
    rerender({ active: false });
    act(() => result.current.first.setJob(null));
    await act(async () => {
      pending.resolve(terminal);
    });
    await advance(10);
    expect(result.current.second.job).toEqual(terminal);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("does not replace a newer native revision with a late observer's older seed", async () => {
    const { result } = harness(vi.fn().mockResolvedValue(running));
    const latest = { ...terminal, revision: running.revision + 1 };
    act(() => result.current.setJob(latest));
    await advance(1);
    act(() => result.current.setJob(running));
    await advance(1);
    expect(result.current.job).toEqual(latest);
  });
});
