import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  AgentAuthPort,
  AgentAuthSessionSnapshot,
} from "@/shared/features/agent-auth";
import { useAgentAuthSession } from "@/shared/features/useAgentAuthSession";

function snapshot(terminal = false): AgentAuthSessionSnapshot {
  return {
    contractVersion: 1,
    sessionId: "123e4567-e89b-42d3-a456-426614174000",
    agentId: "grokbuild",
    intent: "login",
    stage: terminal ? "handoff_complete" : "awaiting_user",
    canStopWaiting: !terminal,
    outcome: terminal ? "handoff_only" : null,
    reasonCode: terminal ? "handoff_only" : null,
    observation: {
      kind: "handoff_only",
      contractVersion: 1,
      agentId: "grokbuild",
      ownership: "agent_owned",
      authority: "unverified",
      allowedIntents: ["login", "logout"],
      checkedAt: "2026-09-20T00:00:00Z",
      reasonCodes: ["handoff_only"],
    },
  };
}
function port(): AgentAuthPort {
  return {
    getActiveSession: vi.fn(async () => null),
    getObservation: vi.fn(async () => snapshot().observation),
    startSession: vi.fn(async () => snapshot()),
    getSession: vi.fn(async () => snapshot()),
    stopWaiting: vi.fn(async () => snapshot(true)),
  };
}

describe("shared Agent authentication sessions", () => {
  afterEach(() => vi.useRealTimers());

  it("blocks duplicate starts until failed active-session recovery is explicitly retried", async () => {
    const api = port();
    vi.mocked(api.getActiveSession).mockRejectedValueOnce(
      new Error("unavailable"),
    );
    const { result } = renderHook(() =>
      useAgentAuthSession({ agentId: "grokbuild", port: api }),
    );
    await act(async () => {});
    expect(result.current.busy).toBe(true);
    await act(async () => {
      await result.current.start({ agentId: "grokbuild", intent: "login" });
    });
    expect(api.startSession).not.toHaveBeenCalled();
    await act(async () => {
      result.current.retryRecovery();
    });
    expect(result.current.busy).toBe(false);
    await act(async () => {
      await Promise.all([
        result.current.start({ agentId: "grokbuild", intent: "login" }),
        result.current.start({ agentId: "grokbuild", intent: "login" }),
      ]);
    });
    expect(api.startSession).toHaveBeenCalledTimes(1);
  });

  it("ignores a start reply after hiding, then recovers the native session once", async () => {
    const api = port();
    let finish!: (value: AgentAuthSessionSnapshot) => void;
    vi.mocked(api.startSession).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const onTerminal = vi.fn();
    const { result, rerender } = renderHook(
      ({ enabled }) =>
        useAgentAuthSession({
          agentId: "grokbuild",
          port: api,
          enabled,
          onTerminal,
        }),
      { initialProps: { enabled: true } },
    );
    await act(async () => {});
    act(() => {
      void result.current.start({ agentId: "grokbuild", intent: "login" });
    });
    rerender({ enabled: false });
    await act(async () => {
      finish(snapshot(true));
    });
    expect(result.current.snapshot).toBeNull();
    expect(onTerminal).not.toHaveBeenCalled();
    vi.mocked(api.getActiveSession).mockResolvedValue(snapshot(true));
    rerender({ enabled: true });
    await act(async () => {});
    expect(result.current.snapshot?.stage).toBe("handoff_complete");
    expect(onTerminal).toHaveBeenCalledTimes(1);
    await act(async () => {
      result.current.retryRecovery();
    });
    expect(onTerminal).toHaveBeenCalledTimes(1);
  });

  it("rejects crossed polling responses and stops polling while hidden", async () => {
    vi.useFakeTimers();
    const api = port();
    vi.mocked(api.getActiveSession).mockResolvedValue(snapshot());
    vi.mocked(api.getSession).mockResolvedValue({
      ...snapshot(true),
      sessionId: "223e4567-e89b-42d3-a456-426614174000",
    });
    const onTerminal = vi.fn();
    const { result, rerender } = renderHook(
      ({ enabled }) =>
        useAgentAuthSession({
          agentId: "grokbuild",
          port: api,
          enabled,
          onTerminal,
        }),
      { initialProps: { enabled: true } },
    );
    await act(async () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(750);
    });
    expect(result.current.error).not.toBeNull();
    expect(result.current.snapshot?.stage).toBe("awaiting_user");
    expect(onTerminal).not.toHaveBeenCalled();
    rerender({ enabled: false });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(api.getSession).toHaveBeenCalledTimes(1);
  });

  it("recovers a known terminal result completed while hidden even when no active session remains", async () => {
    const api = port();
    vi.mocked(api.getActiveSession)
      .mockResolvedValueOnce(snapshot())
      .mockResolvedValue(null);
    vi.mocked(api.getSession).mockResolvedValue(snapshot(true));
    const onTerminal = vi.fn();
    const { result, rerender } = renderHook(
      ({ enabled }) =>
        useAgentAuthSession({
          agentId: "grokbuild",
          port: api,
          enabled,
          onTerminal,
        }),
      { initialProps: { enabled: true } },
    );
    await act(async () => {});
    rerender({ enabled: false });
    rerender({ enabled: true });
    await act(async () => {});
    expect(api.getSession).toHaveBeenCalledWith(snapshot().sessionId);
    expect(result.current.snapshot?.stage).toBe("handoff_complete");
    expect(onTerminal).toHaveBeenCalledTimes(1);
  });
});
