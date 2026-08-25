import { beforeEach, describe, expect, it, vi } from "vitest";

import { createAgentInstallReadinessPort } from "@/v2/shared/platform/tauri/feature-ports/agentInstallReadiness";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function wire(agentId = "qoderwork") {
  const codex = agentId === "codex";
  const cli =
    agentId === "claude-code" || agentId === "grokbuild" || agentId === "opencode";
  return {
    contractVersion: 2,
    agentId,
    reviewedAt: "2026-08-25",
    installState: "unknown",
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
    sourceKind: codex ? "codex_desktop" : cli ? "cli_tooling" : "managed_desktop",
    allowedActions: [],
    reasonCodes: ["auth_state_unknown"],
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

    invoke.mockResolvedValue({
      contractVersion: 1,
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
      }),
    ).resolves.toMatchObject({ jobId: "job-1" });
    expect(invoke).toHaveBeenCalledWith("start_agent_action", {
      request: {
        agentId: "qoderwork",
        action: "install",
        expectedReleaseId:
          "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
