import { beforeEach, describe, expect, it, vi } from "vitest";

import { createAgentAuthPort } from "@/v2/shared/platform/tauri/feature-ports/agentAuth";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const SESSION_ID = "123e4567-e89b-12d3-a456-426614174000";

function observation() {
  return {
    kind: "account",
    contractVersion: 1,
    agentId: "claude-code",
    ownership: "agent_owned",
    authority: "verified",
    state: "logged_out",
    allowedIntents: ["login", "logout"],
    checkedAt: "2026-08-30T00:00:00Z",
    reasonCodes: [],
  };
}

function session() {
  return {
    contractVersion: 1,
    sessionId: SESSION_ID,
    agentId: "claude-code",
    intent: "login",
    stage: "preparing",
    canStopWaiting: false,
    outcome: null,
    observation: observation(),
    reasonCode: null,
  };
}

describe("Tauri Agent auth port", () => {
  beforeEach(() => invoke.mockReset());

  it("uses only the four closed commands and bounded payload fields", async () => {
    invoke.mockResolvedValueOnce(observation());
    await expect(
      createAgentAuthPort().getObservation("claude-code"),
    ).resolves.toMatchObject({ kind: "account", state: "logged_out" });
    expect(invoke).toHaveBeenLastCalledWith("get_agent_auth_observation", {
      agentId: "claude-code",
    });

    invoke.mockResolvedValueOnce(session());
    await createAgentAuthPort().startSession({
      agentId: "claude-code",
      intent: "login",
      inventoryId: `i1:${"a".repeat(32)}`,
      targetId: `c1:${"b".repeat(32)}`,
      expectedTargetRevision: `r1:${"c".repeat(64)}`,
    });
    expect(invoke).toHaveBeenLastCalledWith("start_agent_auth_session", {
      request: {
        agentId: "claude-code",
        intent: "login",
        inventoryId: `i1:${"a".repeat(32)}`,
        targetId: `c1:${"b".repeat(32)}`,
        expectedTargetRevision: `r1:${"c".repeat(64)}`,
      },
    });

    invoke.mockResolvedValueOnce(session());
    await createAgentAuthPort().getSession(SESSION_ID);
    expect(invoke).toHaveBeenLastCalledWith("get_agent_auth_session", {
      sessionId: SESSION_ID,
    });

    invoke.mockResolvedValueOnce({
      ...session(),
      stage: "cancelled",
      outcome: "cancelled",
      reasonCode: "monitoring_stopped",
    });
    await createAgentAuthPort().stopWaiting(SESSION_ID);
    expect(invoke).toHaveBeenLastCalledWith("stop_waiting_for_agent_auth", {
      sessionId: SESSION_ID,
    });
  });

  it("rejects unknown IDs before IPC and rejects excess response fields", async () => {
    await expect(
      createAgentAuthPort().getObservation("claude" as "claude-code"),
    ).rejects.toThrow("Agent auth request is invalid");
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValue({ ...observation(), token: "sentinel" });
    await expect(
      createAgentAuthPort().getObservation("claude-code"),
    ).rejects.toThrow("Agent auth is unavailable");
  });
});
