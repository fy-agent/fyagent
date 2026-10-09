import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  parseManagedAuthLoginSession,
  type ManagedAuthPort,
  type StartManagedAuthLoginRequest,
} from "@/shared/features/managed-auth";
import { useManagedAuthLoginSession } from "@/pages/auth/useManagedAuthLoginSession";
import { deviceLoginSessionFixture } from "../../fixtures/managedAuth";

const deviceLoginRequest: StartManagedAuthLoginRequest = {
  provider: "openai",
  purpose: "connect_consumer",
  consumer: "codex",
  method: "device_code",
  accountId: null,
};

function loginPort(overrides: Partial<ManagedAuthPort> = {}): ManagedAuthPort {
  return {
    getOverview: vi.fn(),
    getAccountQuota: vi.fn(),
    startLogin: vi.fn(async () => deviceLoginSessionFixture()),
    getLoginSession: vi.fn(async () => deviceLoginSessionFixture()),
    cancelLogin: vi.fn(),
    reopenLogin: vi.fn(),
    switchLoginMethod: vi.fn(),
    setDefaultAccount: vi.fn(),
    previewAccountRemoval: vi.fn(),
    removeAccount: vi.fn(),
    applyConnectionAction: vi.fn(),
    previewConnectionAction: vi.fn(),
    ...overrides,
  };
}

describe("useManagedAuthLoginSession", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it.each([
    {
      label: "secret unavailable",
      cause: { contractVersion: 1, reasonCode: "secret_unavailable" },
      expected: "系统凭据库暂时不可用。",
    },
    {
      label: "operation conflict",
      cause: { contractVersion: 1, reasonCode: "operation_conflict" },
      expected: "已有账号操作正在进行，请先完成或取消。",
    },
    {
      label: "raw native error",
      cause: new Error("token=private-token C:/private/auth.json"),
      expected: "请稍后重试。",
    },
    {
      label: "command error with forbidden fields",
      cause: {
        contractVersion: 1,
        reasonCode: "secret_unavailable",
        token: "private-token",
      },
      expected: "请稍后重试。",
    },
  ])(
    "retains the request after $label without exposing raw errors",
    async ({ cause, expected }) => {
      const preparing = deviceLoginSessionFixture({
        stage: "preparing",
        userCode: null,
        verificationUri: null,
        expiresAt: null,
      });
      const startLogin = vi
        .fn<ManagedAuthPort["startLogin"]>()
        .mockRejectedValueOnce(cause)
        .mockImplementationOnce(async () =>
          parseManagedAuthLoginSession(preparing),
        );
      const { result } = renderHook(() =>
        useManagedAuthLoginSession({
          port: loginPort({ startLogin }),
          active: true,
        }),
      );

      await act(async () => {
        expect(await result.current.start(deviceLoginRequest)).toBeNull();
      });
      expect(result.current.snapshot).toBeNull();
      expect(result.current.error).toBe(expected);
      expect(result.current.submitting).toBe(false);
      expect(result.current.busy).toBe(false);

      await act(async () => {
        await result.current.retry();
      });
      expect(startLogin).toHaveBeenLastCalledWith(deviceLoginRequest);
      expect(startLogin).toHaveBeenCalledTimes(2);
      expect(result.current.snapshot?.stage).toBe("preparing");
      expect(result.current.error).toBeNull();
    },
  );

  it("polls a parsed Preparing session and ignores a late poll after backend cancellation", async () => {
    vi.useFakeTimers();
    let completeLatePoll: (value: unknown) => void = () => {
      throw new Error("The delayed poll was not initialized");
    };
    const latePoll = new Promise<unknown>((resolve) => {
      completeLatePoll = resolve;
    });
    const awaitingUser = deviceLoginSessionFixture();
    const getLoginSession = vi
      .fn<ManagedAuthPort["getLoginSession"]>()
      .mockImplementationOnce(async () =>
        parseManagedAuthLoginSession(awaitingUser),
      )
      .mockImplementationOnce(async () =>
        parseManagedAuthLoginSession(await latePoll),
      );
    const cancelled = deviceLoginSessionFixture({
      stage: "cancelled",
      terminal: true,
      canCancel: false,
      reasonCode: "cancelled",
      userCode: null,
      verificationUri: null,
      expiresAt: null,
    });
    const cancelLogin = vi.fn(async () =>
      parseManagedAuthLoginSession(cancelled),
    );
    const onTerminal = vi.fn();
    const port = loginPort({
      startLogin: vi.fn(async () =>
        parseManagedAuthLoginSession(
          deviceLoginSessionFixture({
            stage: "preparing",
            userCode: null,
            verificationUri: null,
            expiresAt: null,
          }),
        ),
      ),
      getLoginSession,
      cancelLogin,
    });
    const { result } = renderHook(() =>
      useManagedAuthLoginSession({ port, active: true, onTerminal }),
    );

    await act(async () => {
      await result.current.start(deviceLoginRequest);
    });
    expect(result.current.snapshot?.stage).toBe("preparing");
    expect(result.current.error).toBeNull();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(result.current.snapshot?.stage).toBe("awaiting_user");
    expect(result.current.snapshot?.userCode).toBe("ABCD-EFGH");
    expect(getLoginSession).toHaveBeenCalledWith(awaitingUser.sessionId);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(getLoginSession).toHaveBeenCalledTimes(2);
    await act(async () => {
      await result.current.cancel();
    });
    expect(cancelLogin).toHaveBeenCalledWith(awaitingUser.sessionId);
    expect(result.current.snapshot?.stage).toBe("cancelled");
    expect(onTerminal).toHaveBeenCalledTimes(1);

    await act(async () => {
      completeLatePoll(awaitingUser);
      await vi.advanceTimersByTimeAsync(3_000);
    });
    expect(result.current.snapshot?.stage).toBe("cancelled");
    expect(result.current.error).toBeNull();
    expect(getLoginSession).toHaveBeenCalledTimes(2);
    expect(onTerminal).toHaveBeenCalledTimes(1);
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
