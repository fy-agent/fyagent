import { beforeEach, describe, expect, it, vi } from "vitest";

import { createAgentInstallReadinessPort } from "@/v2/shared/platform/tauri/feature-ports/agentInstallReadiness";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function wire(agentId = "qoderwork") {
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
    updateState: "unknown",
    releaseId: null,
    localVersion: null,
    remoteVersion: null,
    authOwnership: codex
      ? "fyagent_managed"
      : agentId === "opencode"
        ? "provider_owned"
        : "agent_owned",
    authState: "unknown",
    sourceKind: codex
      ? "codex_desktop"
      : cli
        ? "cli_tooling"
        : "managed_desktop",
    allowedActions: [],
    reasonCodes: ["auth_state_unknown"],
  };
}

function inventoryWire(agentId = "qoderwork") {
  return {
    contractVersion: 1,
    inventoryId: `i1:${"a".repeat(32)}`,
    agentId,
    state: "not_observed",
    candidates: [],
    freshDestinations: [
      {
        destinationId: `d1:${"b".repeat(32)}`,
        destinationRevision: `r1:${"c".repeat(64)}`,
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

describe("Tauri Agent install readiness port", () => {
  beforeEach(() => invoke.mockReset());

  it("invokes readiness and action commands with closed payloads", async () => {
    invoke.mockResolvedValue(wire("codex"));
    await expect(
      createAgentInstallReadinessPort().get("codex"),
    ).resolves.toEqual({
      ...wire("codex"),
      reasonCodes: ["auth_state_unknown"],
    });
    expect(invoke).toHaveBeenCalledWith("get_agent_install_readiness", {
      agentId: "codex",
    });

    invoke.mockResolvedValue(inventoryWire());
    await expect(
      createAgentInstallReadinessPort().getInventory("qoderwork"),
    ).resolves.toEqual(inventoryWire());
    expect(invoke).toHaveBeenCalledWith("get_agent_installation_inventory", {
      agentId: "qoderwork",
    });

    invoke.mockResolvedValue({
      ...inventoryWire("opencode"),
      surface: "desktop",
    });
    await expect(
      createAgentInstallReadinessPort().getInventory("opencode", "desktop"),
    ).resolves.toMatchObject({ agentId: "opencode", surface: "desktop" });
    expect(invoke).toHaveBeenCalledWith("get_agent_installation_inventory", {
      agentId: "opencode",
      surface: "desktop",
    });

    invoke.mockResolvedValue({
      contractVersion: 3,
      agentId: "qoderwork",
      action: "install",
      jobId: "job-1",
      stage: "checking",
      reasonCode: null,
    });
    await expect(
      createAgentInstallReadinessPort().startAction({
        agentId: "qoderwork",
        action: "install",
        expectedReleaseId:
          "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        inventoryId: `i1:${"a".repeat(32)}`,
        targetId: `d1:${"b".repeat(32)}`,
        expectedTargetRevision: `r1:${"c".repeat(64)}`,
      }),
    ).resolves.toMatchObject({ jobId: "job-1" });
    expect(invoke).toHaveBeenCalledWith("start_agent_action", {
      request: {
        agentId: "qoderwork",
        action: "install",
        expectedReleaseId:
          "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        inventoryId: `i1:${"a".repeat(32)}`,
        targetId: `d1:${"b".repeat(32)}`,
        expectedTargetRevision: `r1:${"c".repeat(64)}`,
      },
    });
  });

  it("rejects unknown IDs before IPC and rejects an excess response field", async () => {
    await expect(
      createAgentInstallReadinessPort().get("codex-cli" as "codex"),
    ).rejects.toThrow("Agent install readiness request is invalid");
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValue({ ...wire(), signer: "sentinel" });
    await expect(
      createAgentInstallReadinessPort().get("qoderwork"),
    ).rejects.toThrow("Agent install readiness is unavailable");
  });
});
