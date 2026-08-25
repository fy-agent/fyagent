import { describe, expect, it } from "vitest";

import {
  assertAgentInstallReadinessId,
  parseAgentInstallReadiness,
  type AgentInstallReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import { AGENT_CATALOG_IDS } from "@/v2/shared/features/directory";

function readiness(
  agentId: (typeof AGENT_CATALOG_IDS)[number] = "qoderwork",
): AgentInstallReadiness {
  const codex = agentId === "codex";
  const cli = agentId === "claude-code" || agentId === "grokbuild" || agentId === "opencode";
  return {
    contractVersion: 2,
    agentId,
    reviewedAt: "2026-08-25",
    installState: "unknown",
    updateState: codex ? "unknown" : "latest_unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    authOwnership: codex
      ? "fyagent_managed"
      : agentId === "opencode"
        ? "provider_owned"
        : "agent_owned",
    authState: agentId === "opencode" ? "provider_connection_required" : "unknown",
    sourceKind: codex ? "codex_desktop" : cli ? "cli_tooling" : "managed_desktop",
    allowedActions: [],
    reasonCodes: codex
      ? ["managed_by_codex_desktop", "auth_state_unknown"]
      : agentId === "opencode"
        ? ["provider_connection_required"]
        : ["auth_state_unknown"],
  };
}

describe("Agent install readiness wire contract", () => {
  it("accepts the exact canonical seven IDs without defining another order", () => {
    expect(
      AGENT_CATALOG_IDS.map((agentId) =>
        parseAgentInstallReadiness(readiness(agentId), agentId),
      ).map((value) => value.agentId),
    ).toEqual(AGENT_CATALOG_IDS);

    for (const invalid of [
      "qoderwork-cn",
      "dingtalk-wukong",
      "codex-cli",
      "claude",
      "unknown",
      "pi",
    ]) {
      expect(() =>
        assertAgentInstallReadinessId(
          invalid as (typeof AGENT_CATALOG_IDS)[number],
        ),
      ).toThrow("Agent install readiness request is invalid");
    }
  });

  it("rejects leftover v1 fields and sensitive locators", () => {
    expect(() =>
      parseAgentInstallReadiness(
        { ...readiness(), automation: { state: "unavailable" } },
        "qoderwork",
      ),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallReadiness(
        { ...readiness(), downloadUrl: "https://example.invalid" },
        "qoderwork",
      ),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallReadiness(readiness("codex"), "qoderwork"),
    ).toThrow("Agent install readiness is unavailable");
    expect(
      parseAgentInstallReadiness(
        {
          ...readiness("qoderwork"),
          reasonCodes: ["source_not_verified", "official_page_only"],
        },
        "qoderwork",
      ).reasonCodes,
    ).toEqual(["source_not_verified", "official_page_only"]);
  });
});
