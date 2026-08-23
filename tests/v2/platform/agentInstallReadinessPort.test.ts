import { beforeEach, describe, expect, it, vi } from "vitest";

import { createAgentInstallReadinessPort } from "@/v2/shared/platform/tauri/feature-ports/agentInstallReadiness";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function wire(agentId = "qoderwork") {
  const codex = agentId === "codex";
  return {
    contractVersion: 1,
    agentId,
    reviewedAt: "2026-08-24",
    automation: {
      state: "unavailable",
      reasonCode: codex ? "managed_by_codex_desktop" : "official_guide_only",
    },
    source: {
      state: "unknown",
      reasonCode: "source_review_not_refreshed",
      installMode: codex ? "managed_package" : "official_guide",
      licenseScope: "unconfirmed",
      distributionState: "unconfirmed",
      checkedAt: null,
    },
    integrity: {
      state: "unknown",
      summaryCode: "integrity_not_checked",
      checkedAt: null,
    },
    preflight: {
      state: "unknown",
      reasonCode: "preflight_not_run",
      checks: [],
      checkedAt: null,
    },
    plan: {
      state: "unknown",
      reasonCode: "plan_not_created",
      snapshotId: null,
      snapshotStale: null,
    },
  };
}

describe("Tauri Agent install readiness port", () => {
  beforeEach(() => invoke.mockReset());

  it("invokes the sole read-only command with one canonical ID", async () => {
    invoke.mockResolvedValue(wire("codex"));
    await expect(
      createAgentInstallReadinessPort().get("codex"),
    ).resolves.toEqual(wire("codex"));
    expect(invoke).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("get_agent_install_readiness", {
      agentId: "codex",
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
