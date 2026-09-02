import { describe, expect, it } from "vitest";

import {
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  type AgentInstallReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import {
  AGENT_DIRECTORY_UPDATE_UI,
  DIRECTORY_UPDATE_DISABLED_AGENT_IDS,
  canOfferDirectoryUpdate,
  visibleAllowedActions,
} from "@/v2/shared/features/agent-lifecycle-capabilities";
import { AGENT_CATALOG_IDS } from "@/v2/shared/features/types";

function readiness(
  overrides: Partial<AgentInstallReadiness> = {},
): AgentInstallReadiness {
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    agentId: "opencode",
    reviewedAt: "2026-08-29",
    installState: "installed",
    inventoryState: "single",
    requiresTargetSelection: false,
    updateState: "update_available",
    releaseId: null,
    localVersion: "1.0.0",
    remoteVersion: "1.1.0",
    authOwnership: "agent_owned",
    authState: "unknown",
    sourceKind: "managed_desktop",
    allowedActions: ["update", "launch"],
    reasonCodes: [],
    ...overrides,
  };
}

describe("agent lifecycle capabilities", () => {
  it("covers every catalog id and disables update chrome for the three domestic products", () => {
    expect(Object.keys(AGENT_DIRECTORY_UPDATE_UI).sort()).toEqual(
      [...AGENT_CATALOG_IDS].sort(),
    );
    expect(DIRECTORY_UPDATE_DISABLED_AGENT_IDS).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
    ]);
    expect(AGENT_DIRECTORY_UPDATE_UI.codex).toBe("codex_desktop");
    expect(AGENT_DIRECTORY_UPDATE_UI.opencode).toBe("generic");
  });

  it("never offers directory update for disabled products even when allowedActions contains update", () => {
    for (const agentId of DIRECTORY_UPDATE_DISABLED_AGENT_IDS) {
      expect(
        canOfferDirectoryUpdate(
          agentId,
          readiness({ agentId, allowedActions: ["update"] }),
        ),
      ).toBe(false);
      expect(
        visibleAllowedActions(agentId, ["install", "update", "launch"]),
      ).toEqual(["install", "launch"]);
    }
  });

  it("offers generic update only with backend-proven eligibility", () => {
    expect(canOfferDirectoryUpdate("opencode", readiness())).toBe(true);
    expect(
      canOfferDirectoryUpdate(
        "opencode",
        readiness({ allowedActions: ["launch"] }),
      ),
    ).toBe(false);
    expect(visibleAllowedActions("opencode", ["update", "launch"])).toEqual([
      "update",
      "launch",
    ]);
  });
});
