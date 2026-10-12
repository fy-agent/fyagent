import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useAuthoritativeAssignmentMutation } from "@/shared/features/authoritative-assignment";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

describe("useAuthoritativeAssignmentMutation", () => {
  it("serializes writes and confirms only authoritative readback", async () => {
    const mutation = deferred<boolean>();
    const mutate = vi.fn(() => mutation.promise);
    const snapshot: Record<string, boolean> = {
      alpha: true,
      beta: false,
    };
    const reread = vi.fn(async () => ({
      data: snapshot,
      error: null,
    }));
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snapshot, itemId: string) => snapshot?.[itemId],
      }),
    );

    let first!: ReturnType<typeof result.current.run>;
    await act(async () => {
      first = result.current.run("alpha", true);
      await Promise.resolve();
    });

    expect(result.current.busy).toBe(true);
    expect(result.current.pendingId).toBe("alpha");

    let second!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      second = await result.current.run("beta", true);
    });
    expect(second).toEqual({ status: "busy" });
    expect(mutate).toHaveBeenCalledTimes(1);

    let firstOutcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      mutation.resolve(true);
      firstOutcome = await first;
    });

    expect(firstOutcome).toEqual({ status: "confirmed" });
    expect(reread).toHaveBeenCalledTimes(1);
    expect(result.current.busy).toBe(false);
    expect(result.current.pendingId).toBeNull();
  });

  it("fails closed and rereads again after a mismatch", async () => {
    const mutate = vi.fn(async () => true);
    const snapshot: Record<string, boolean> = { alpha: false };
    const reread = vi.fn(async () => ({
      data: snapshot,
      error: null,
    }));
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snapshot, itemId: string) => snapshot?.[itemId],
      }),
    );

    let outcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      outcome = await result.current.run("alpha", true);
    });

    expect(outcome).toEqual({ status: "rejected" });
    expect(reread).toHaveBeenCalledTimes(2);
    expect(result.current.busy).toBe(false);
  });

  it("captures mutation rejection stage and reason", async () => {
    const mutate = vi.fn(async () => {
      throw new Error("backend mutation rejected");
    });
    const snapshot: Record<string, boolean> = { alpha: false };
    const reread = vi.fn(async () => ({
      data: snapshot,
      error: null,
    }));
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snap: Record<string, boolean> | undefined, itemId: string) => snap?.[itemId],
      }),
    );

    let outcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      outcome = await result.current.run("alpha", true);
    });

    expect(outcome.status).toBe("rejected");
    if (outcome.status !== "rejected") throw new Error("Expected rejected");
    expect(outcome.stage).toBe("mutate");
    expect(outcome.reason).toContain("backend mutation rejected");
    expect(outcome.observedValue).toBe(false);
    expect(reread).toHaveBeenCalledTimes(1);
  });

  it("captures readback failure stage and reason", async () => {
    const mutate = vi.fn(async () => true);
    const reread = vi
      .fn<() => Promise<{ data: Record<string, boolean> | undefined; error: unknown }>>()
      .mockResolvedValueOnce({
        data: undefined,
        error: new Error("readback network timeout"),
      })
      .mockResolvedValueOnce({
        data: { alpha: true },
        error: null,
      });
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snap: Record<string, boolean> | undefined, itemId: string) => snap?.[itemId],
      }),
    );

    let outcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      outcome = await result.current.run("alpha", true);
    });

    expect(outcome.status).toBe("rejected");
    if (outcome.status !== "rejected") throw new Error("Expected rejected");
    expect(outcome.stage).toBe("readback");
    expect(outcome.reason).toContain("readback network timeout");
    expect(outcome.observedValue).toBe(true);
    expect(reread).toHaveBeenCalledTimes(2);
  });

  it("captures mismatch stage and returns observed value from recovery reread", async () => {
    const mutate = vi.fn(async () => true);
    const reread = vi
      .fn<() => Promise<{ data: Record<string, boolean> | undefined; error: unknown }>>()
      .mockResolvedValueOnce({
        data: { alpha: false },
        error: null,
      })
      .mockResolvedValueOnce({
        data: { alpha: false },
        error: null,
      });
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snap: Record<string, boolean> | undefined, itemId: string) => snap?.[itemId],
      }),
    );

    let outcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      outcome = await result.current.run("alpha", true);
    });

    expect(outcome.status).toBe("rejected");
    if (outcome.status !== "rejected") throw new Error("Expected rejected");
    expect(outcome.stage).toBe("mismatch");
    expect(outcome.reason).toBe("回读状态与预期不一致");
    expect(outcome.observedValue).toBe(false);
    expect(reread).toHaveBeenCalledTimes(2);
  });

  it("captures plain string rejection reason from native mutation", async () => {
    const mutate = vi.fn(async () => {
      throw "权限不足";
    });
    const snapshot: Record<string, boolean> = { alpha: false };
    const reread = vi.fn(async () => ({
      data: snapshot,
      error: null,
    }));
    const { result } = renderHook(() =>
      useAuthoritativeAssignmentMutation({
        mutate,
        reread,
        readValue: (snap: Record<string, boolean> | undefined, itemId: string) => snap?.[itemId],
      }),
    );

    let outcome!: Awaited<ReturnType<typeof result.current.run>>;
    await act(async () => {
      outcome = await result.current.run("alpha", true);
    });

    expect(outcome.status).toBe("rejected");
    if (outcome.status !== "rejected") throw new Error("Expected rejected");
    expect(outcome.stage).toBe("mutate");
    expect(outcome.reason).toBe("权限不足");
    expect(outcome.error).toBe("权限不足");
    expect(outcome.observedValue).toBe(false);
    expect(reread).toHaveBeenCalledTimes(1);
  });
});
