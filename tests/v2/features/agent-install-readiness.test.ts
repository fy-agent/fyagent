import { describe, expect, it } from "vitest";

import {
  assertAgentInstallReadinessId,
  installationTargetsForAction,
  parseAgentInstallationInventory,
  parseAgentInstallReadiness,
  type AgentInstallReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import { AGENT_CATALOG_IDS } from "@/v2/shared/features/directory";

function readiness(
  agentId: (typeof AGENT_CATALOG_IDS)[number] = "qoderwork",
): AgentInstallReadiness {
  const codex = agentId === "codex";
  const cli =
    agentId === "claude-code" ||
    agentId === "grokbuild" ||
    agentId === "opencode";
  return {
    contractVersion: 3,
    agentId,
    reviewedAt: "2026-08-29",
    installState: "unknown",
    inventoryState: "unknown",
    requiresTargetSelection: false,
    updateState: codex ? "unknown" : "latest_unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    authOwnership: codex
      ? "fyagent_managed"
      : agentId === "opencode"
        ? "provider_owned"
        : "agent_owned",
    authState:
      agentId === "opencode" ? "provider_connection_required" : "unknown",
    sourceKind: codex
      ? "codex_desktop"
      : cli
        ? "cli_tooling"
        : "managed_desktop",
    allowedActions: [],
    reasonCodes: codex
      ? ["managed_by_codex_desktop", "auth_state_unknown"]
      : agentId === "opencode"
        ? ["provider_connection_required"]
        : ["auth_state_unknown"],
  };
}

function inventory() {
  return {
    contractVersion: 1,
    inventoryId: `i1:${"a".repeat(32)}`,
    agentId: "qoderwork",
    state: "multiple",
    candidates: [
      {
        candidateId: `c1:${"b".repeat(32)}`,
        candidateRevision: `r1:${"c".repeat(64)}`,
        agentId: "qoderwork",
        scope: "current_user",
        owner: "vendor_installer",
        packageKind: "app_bundle",
        localVersion: "1.0.0",
        launchEligible: true,
        installEligible: false,
        updateEligible: true,
        reasonCodes: [],
        evidenceCodes: ["bundle_identity"],
        locationLabel: "~/Applications/QoderWork CN.app",
      },
      {
        candidateId: `c1:${"d".repeat(32)}`,
        candidateRevision: `r1:${"e".repeat(64)}`,
        agentId: "qoderwork",
        scope: "all_users",
        owner: "vendor_installer",
        packageKind: "app_bundle",
        localVersion: "1.1.0",
        launchEligible: true,
        installEligible: false,
        updateEligible: true,
        reasonCodes: [],
        evidenceCodes: ["bundle_identity"],
        locationLabel: "/Applications/QoderWork CN.app",
      },
    ],
    freshDestinations: [
      {
        destinationId: `d1:${"f".repeat(32)}`,
        destinationRevision: `r1:${"1".repeat(64)}`,
        scope: "current_user",
        owner: "vendor_installer",
        packageKind: "app_bundle",
        requiresElevation: false,
        writable: true,
        eligible: true,
        reasonCodes: [],
        locationLabel: "~/Applications",
      },
    ],
    reasonCodes: [],
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

  it("accepts opaque multi-install inventory and projects action-scoped targets", () => {
    const parsed = parseAgentInstallationInventory(inventory(), "qoderwork");

    expect(parsed.state).toBe("multiple");
    expect(installationTargetsForAction(parsed, "update")).toHaveLength(2);
    expect(installationTargetsForAction(parsed, "install")).toMatchObject([
      {
        kind: "fresh_destination",
        targetId: `d1:${"f".repeat(32)}`,
        eligibleActions: ["install"],
      },
    ]);
  });

  it("rejects raw locators, duplicate capabilities, and inconsistent inventory state", () => {
    expect(() =>
      parseAgentInstallationInventory(
        {
          ...inventory(),
          candidates: [
            { ...inventory().candidates[0], executablePath: "/tmp/app" },
          ],
          state: "single",
        },
        "qoderwork",
      ),
    ).toThrow("Agent installation inventory is unavailable");

    expect(() =>
      parseAgentInstallationInventory(
        {
          ...inventory(),
          candidates: [
            inventory().candidates[0],
            {
              ...inventory().candidates[1],
              candidateId: inventory().candidates[0].candidateId,
            },
          ],
        },
        "qoderwork",
      ),
    ).toThrow("Agent installation inventory is unavailable");

    expect(() =>
      parseAgentInstallationInventory(
        { ...inventory(), state: "single" },
        "qoderwork",
      ),
    ).toThrow("Agent installation inventory is unavailable");
  });
});
