import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ManagedAuthPort } from "@/v2/shared/features/managed-auth";
import { useManagedAuthLoginSession } from "@/v2/pages/auth/useManagedAuthLoginSession";
import { deviceLoginSessionFixture } from "../../fixtures/managedAuth";

function loginPort(
  overrides: Partial<ManagedAuthPort> = {},
): ManagedAuthPort {
  return {
    getOverview: vi.fn(),
    startLogin: vi.fn(async () => deviceLoginSessionFixture()),
    getLoginSession: vi.fn(async () => deviceLoginSessionFixture()),
    cancelLogin: vi.fn(),
    reopenLogin: vi.fn(),
    switchLoginMethod: vi.fn(),
    setDefaultAccount: vi.fn(),
    previewAccountRemoval: vi.fn(),
    removeAccount: vi.fn(),
    applyConnectionAction: vi.fn(),
    ...overrides,
  };
}

describe("useManagedAuthLoginSession", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("pauses polling while the persistent route is hidden and resumes afterward", async () => {
    vi.useFakeTimers();
    const getLoginSession = vi.fn(async () => deviceLoginSessionFixture());
    const port = loginPort({ getLoginSession });
    const { result, rerender } = renderHook(
      ({ active }: { active: boolean }) =>
        useManagedAuthLoginSession({ port, active }),
      { initialProps: { active: true } },
    );

    await act(async () => {
      await result.current.start({
        provider: "openai",
        purpose: "connect_consumer",
        consumer: "codex",
        method: "device_code",
        accountId: null,
      });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(getLoginSession).toHaveBeenCalledTimes(1);

    rerender({ active: false });
    getLoginSession.mockClear();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3_000);
    });
    expect(getLoginSession).not.toHaveBeenCalled();

    rerender({ active: true });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(getLoginSession).toHaveBeenCalledTimes(1);
  });
});
