import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useAuthoritativeAssignmentMutation } from "@/v2/shared/features/authoritative-assignment";

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
});
