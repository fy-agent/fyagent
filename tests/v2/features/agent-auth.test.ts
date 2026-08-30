import { describe, expect, it } from "vitest";

import {
  assertAgentAuthId,
  parseAgentAuthObservation,
  parseAgentAuthSessionSnapshot,
} from "@/v2/shared/features/agent-auth";

const PROVIDER_ID = `p1:${"a".repeat(32)}`;
const SESSION_ID = "123e4567-e89b-12d3-a456-426614174000";

function account(state: "logged_in" | "logged_out" | "unknown" = "logged_out") {
  return {
    kind: "account",
    contractVersion: 1,
    agentId: "claude-code",
    ownership: "agent_owned",
    authority: "verified",
    state,
    allowedIntents: ["login", "logout"],
    checkedAt: "2026-08-30T00:00:00Z",
    reasonCodes: [],
  };
}

function providers(ids = [PROVIDER_ID]) {
  return {
    kind: "provider_connections",
    contractVersion: 1,
    agentId: "opencode",
    ownership: "provider_owned",
    authority: "verified",
    state: ids.length === 0 ? "empty" : "configured",
    providers: ids.map((providerId) => ({ providerId, label: "OpenAI" })),
    allowedIntents: ["connect_provider", "logout"],
    checkedAt: "2026-08-30T00:00:00Z",
    reasonCodes: [],
  };
}

function session(
  stage = "verified",
  outcome: string | null = "verified_logged_in",
) {
  return {
    contractVersion: 1,
    sessionId: SESSION_ID,
    agentId: "claude-code",
    intent: "login",
    stage,
    canStopWaiting: stage === "awaiting_user" || stage === "verifying",
    outcome,
    observation: account("logged_in"),
    reasonCode: null,
  };
}

describe("Agent auth wire contract", () => {
  it("parses the five authority kinds without collapsing them to one bool", () => {
    expect(parseAgentAuthObservation(account(), "claude-code")).toMatchObject({
      kind: "account",
      state: "logged_out",
      authority: "verified",
    });
    expect(parseAgentAuthObservation(providers(), "opencode")).toMatchObject({
      kind: "provider_connections",
      providers: [{ providerId: PROVIDER_ID, label: "OpenAI" }],
    });
    expect(
      parseAgentAuthObservation(
        {
          kind: "handoff_only",
          contractVersion: 1,
          agentId: "grokbuild",
          ownership: "agent_owned",
          authority: "unverified",
          allowedIntents: ["login", "logout"],
          checkedAt: "2026-08-30T00:00:00Z",
          reasonCodes: ["handoff_only"],
        },
        "grokbuild",
      ),
    ).toMatchObject({ kind: "handoff_only", authority: "unverified" });
    expect(
      parseAgentAuthObservation(
        {
          kind: "fyagent_managed",
          contractVersion: 1,
          agentId: "codex",
          ownership: "fyagent_managed",
          authority: "verified",
          destination: "auth_center",
          allowedIntents: [],
          checkedAt: "2026-08-30T00:00:00Z",
          reasonCodes: ["managed_by_auth_center"],
        },
        "codex",
      ),
    ).toMatchObject({ kind: "fyagent_managed", destination: "auth_center" });
    expect(
      parseAgentAuthObservation(
        {
          kind: "unavailable",
          contractVersion: 1,
          agentId: "claude-code",
          ownership: "agent_owned",
          authority: "unavailable",
          allowedIntents: [],
          checkedAt: "2026-08-30T00:00:00Z",
          reasonCodes: ["auth_observer_unavailable"],
        },
        "claude-code",
      ),
    ).toMatchObject({ kind: "unavailable", authority: "unavailable" });
  });

  it("rejects excess fields, raw locators, unsafe labels, and mismatched IDs", () => {
    for (const excess of [
      { accessToken: "secret" },
      { executablePath: "/tmp/claude" },
      { loginUrl: "https://example.invalid" },
    ]) {
      expect(() =>
        parseAgentAuthObservation({ ...account(), ...excess }, "claude-code"),
      ).toThrow("Agent auth is unavailable");
    }
    expect(() =>
      parseAgentAuthObservation(
        {
          ...providers(),
          providers: [{ providerId: PROVIDER_ID, label: "C:\\Users\\Alice" }],
        },
        "opencode",
      ),
    ).toThrow("Agent auth is unavailable");
    expect(() => parseAgentAuthObservation(account(), "grokbuild")).toThrow(
      "Agent auth is unavailable",
    );
    expect(() => assertAgentAuthId("codex-cli" as "codex")).toThrow(
      "Agent auth request is invalid",
    );
  });

  it("requires exact terminal stage/outcome pairings", () => {
    expect(parseAgentAuthSessionSnapshot(session())).toMatchObject({
      stage: "verified",
      outcome: "verified_logged_in",
      canStopWaiting: false,
    });
    expect(
      parseAgentAuthSessionSnapshot(session("awaiting_user", null)),
    ).toMatchObject({ stage: "awaiting_user", canStopWaiting: true });
    for (const invalid of [
      session("verified", "handoff_only"),
      session("handoff_complete", "verified_logged_in"),
      session("failed", "timed_out"),
      { ...session("awaiting_user", null), canStopWaiting: false },
      { ...session(), command: "claude auth login" },
    ]) {
      expect(() => parseAgentAuthSessionSnapshot(invalid)).toThrow(
        "Agent auth session is unavailable",
      );
    }
  });
});
