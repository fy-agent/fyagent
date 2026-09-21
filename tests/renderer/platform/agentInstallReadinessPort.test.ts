import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  AGENT_ACTION_CONTRACT_VERSION,
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
} from "@/shared/features/agent-install-readiness";
import { createAgentInstallReadinessPort } from "@/shared/platform/tauri/feature-ports/agentInstallReadiness";
import { installPreflightFixture } from "../../fixtures/agentInstallPreflight";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function wire(agentId = "qoderwork") {
  const codex = agentId === "codex";
  const grok = agentId === "grokbuild";
  const claudeCli = agentId === "claude-code";
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    configurationEligibility: { state: "unknown", evidence: "none" },
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
      : grok || claudeCli
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

  it("checks before starting and rejects a substituted preflight target", async () => {
    const request = {
      agentId: "qoderwork" as const,
      action: "install" as const,
      inventoryId: `i1:${"a".repeat(32)}`,
      targetId: `d1:${"b".repeat(32)}`,
      expectedTargetRevision: `r1:${"c".repeat(64)}`,
    };
    const checked = installPreflightFixture(request);
    invoke.mockResolvedValueOnce(checked);
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).resolves.toEqual(checked);
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "get_agent_install_preflight",
      { request },
    );
    invoke.mockResolvedValueOnce({
      ...checked,
      request: { ...request, targetId: `d1:${"e".repeat(32)}` },
    });
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).rejects.toThrow();
    invoke.mockResolvedValueOnce({
      ...checked,
      downloadUrl: "javascript:alert(1)",
    });
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).rejects.toThrow();
    invoke.mockResolvedValueOnce({
      ...checked,
      installerPath: "/tmp/untrusted",
    });
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).rejects.toThrow();
    invoke.mockReset();
    await expect(
      createAgentInstallReadinessPort().preflight({
        ...request,
        action: "launch",
      }),
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("rejects invalid or insufficient space budgets and admits exact capacity", async () => {
    const request = {
      agentId: "qoderwork" as const,
      action: "install" as const,
    };
    const checked = installPreflightFixture(request);
    const requiredBytes = 3 * 1024 ** 3;
    const known = {
      ...checked,
      spaceBudgetBasis: "source_size",
      artifactSizeBytes: 1024 ** 3,
      requiredBytes,
      availableBytes: requiredBytes,
    };
    invoke.mockResolvedValueOnce(known);
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).resolves.toEqual(known);
    for (const invalid of [
      { ...known, availableBytes: 1024 },
      { ...known, availableBytes: requiredBytes - 1 },
      { ...known, requiredBytes: 0 },
      { ...known, requiredBytes: 1 },
      { ...known, requiredBytes: null },
      { ...known, requiredBytes: requiredBytes + 3 },
      { ...known, requiredBytes: Number.MAX_SAFE_INTEGER + 1 },
      { ...known, requiredBytes: Number.NaN },
      { ...known, artifactSizeBytes: Number.MAX_SAFE_INTEGER },
      { ...known, artifactSizeBytes: null },
      { ...known, spaceBudgetBasis: "vendor_guarantee" },
      { ...checked, spaceBudgetBasis: "cli_unknown", requiredBytes: null },
      { ...checked, runtime: "node_npm" },
      { ...checked, requiredBytes: 3 },
      {
        ...known,
        artifactSizeBytes: 3 * 1024 ** 3,
        requiredBytes: 9 * 1024 ** 3,
        availableBytes: 10 * 1024 ** 3,
      },
    ]) {
      invoke.mockResolvedValueOnce(invalid);
      await expect(
        createAgentInstallReadinessPort().preflight(request),
      ).rejects.toThrow("Invalid Agent installation preflight");
    }
    const cliReserve = {
      ...checked,
      runtime: "node_npm" as const,
      artifactSizeBytes: 29410539,
      requiredBytes: 29410539 * 3,
      availableBytes: 29410539 * 3,
      spaceBudgetBasis: "package_reserve" as const,
      downloadUrl: null,
    };
    invoke.mockResolvedValueOnce(cliReserve);
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).resolves.toEqual(cliReserve);
    const cli = {
      ...checked,
      runtime: "node_npm",
      requiredBytes: null,
      artifactSizeBytes: null,
      spaceBudgetBasis: "cli_unknown",
      downloadUrl: null,
    };
    invoke.mockResolvedValueOnce(cli);
    await expect(
      createAgentInstallReadinessPort().preflight(request),
    ).resolves.toEqual(cli);
  });

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

    invoke.mockResolvedValue(inventoryWire("opencode"));
    await expect(
      createAgentInstallReadinessPort().getInventory("opencode"),
    ).resolves.toMatchObject({ agentId: "opencode" });
    expect(invoke).toHaveBeenCalledWith("get_agent_installation_inventory", {
      agentId: "opencode",
    });

    invoke.mockResolvedValue({
      contractVersion: AGENT_ACTION_CONTRACT_VERSION,
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
