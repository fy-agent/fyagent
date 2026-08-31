import { describe, expect, it } from "vitest";

import {
  AGENT_ACTION_CONTRACT_VERSION,
  assertAgentInstallReadinessId,
  installationTargetsForAction,
  isLegalAgentSurface,
  parseAgentActionJobSnapshot,
  parseAgentInstallationInventory,
  parseAgentInstallReadiness,
  surfacesForAgent,
  type AgentInstallReadiness,
  type AgentInstallState,
  type AgentSurfaceReadiness,
} from "@/v2/shared/features/agent-install-readiness";
import { AGENT_CATALOG_IDS } from "@/v2/shared/features/directory";

function surfaceRow(
  surface: "cli" | "desktop",
  installState: AgentInstallState,
): AgentSurfaceReadiness {
  const cli = surface === "cli";
  const installed = installState === "installed";
  return {
    surface,
    installState,
    inventoryState: installed ? "single" : "not_observed",
    requiresTargetSelection: false,
    updateState: installed ? "up_to_date" : "latest_unknown",
    releaseId: cli
      ? null
      : "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    localVersion: installed ? "1.0.0" : null,
    remoteVersion: installed || !cli ? "1.0.0" : null,
    sourceKind: cli ? "cli_tooling" : "managed_desktop",
    allowedActions: installed
      ? surface === "desktop"
        ? ["launch"]
        : []
      : ["install"],
    reasonCodes: [],
  };
}

function readiness(
  agentId: (typeof AGENT_CATALOG_IDS)[number] = "qoderwork",
  combo: { cli: AgentInstallState; desktop: AgentInstallState } = {
    cli: "not_installed",
    desktop: "not_installed",
  },
): AgentInstallReadiness {
  const codex = agentId === "codex";
  const cli =
    agentId === "claude-code" ||
    agentId === "grokbuild" ||
    agentId === "opencode";
  const base: AgentInstallReadiness = {
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
  if (agentId === "opencode") {
    const cliSurface = surfaceRow("cli", combo.cli);
    const desktopSurface = surfaceRow("desktop", combo.desktop);
    return {
      ...base,
      installState: cliSurface.installState,
      inventoryState: cliSurface.inventoryState,
      updateState: cliSurface.updateState,
      localVersion: cliSurface.localVersion,
      remoteVersion: cliSurface.remoteVersion,
      allowedActions: cliSurface.allowedActions,
      surfaces: [cliSurface, desktopSurface],
    };
  }
  return base;
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

  it("parses action job transfer telemetry and rejects v2 or locator fields", () => {
    const snapshot = {
      contractVersion: AGENT_ACTION_CONTRACT_VERSION,
      jobId: "job-1",
      agentId: "qoderwork",
      action: "install",
      stage: "downloading",
      cancellable: true,
      reasonCode: null,
      transfer: {
        phase: "download",
        completedBytes: 1048576,
        totalBytes: 2097152,
        attempt: 1,
        maxAttempts: 1,
        sequence: 2,
        observedAt: "2026-08-31T00:00:00Z",
      },
    };
    expect(parseAgentActionJobSnapshot(snapshot).transfer).toEqual(
      snapshot.transfer,
    );
    expect(
      parseAgentActionJobSnapshot({ ...snapshot, transfer: null }).transfer,
    ).toBeNull();
    expect(
      parseAgentActionJobSnapshot({
        ...snapshot,
        transfer: { ...snapshot.transfer, totalBytes: null },
      }).transfer?.totalBytes,
    ).toBeNull();

    expect(() =>
      parseAgentActionJobSnapshot({ ...snapshot, contractVersion: 2 }),
    ).toThrow("Agent action job is unavailable");
    expect(() =>
      parseAgentActionJobSnapshot({
        ...snapshot,
        transfer: { ...snapshot.transfer, percent: 50 },
      }),
    ).toThrow("Agent action job is unavailable");
    expect(() =>
      parseAgentActionJobSnapshot({
        ...snapshot,
        transfer: { ...snapshot.transfer, path: "/tmp/installer.dmg" },
      }),
    ).toThrow("Agent action job is unavailable");
    expect(() =>
      parseAgentActionJobSnapshot({
        ...snapshot,
        transfer: { ...snapshot.transfer, totalBytes: 8 },
      }),
    ).toThrow("Agent action job is unavailable");
  });

  it("parses OpenCode CLI-only, desktop-only, both, and neither as independent surfaces", () => {
    expect(surfacesForAgent("opencode")).toEqual(["cli", "desktop"]);
    expect(isLegalAgentSurface("qoderwork", "cli")).toBe(false);
    expect(isLegalAgentSurface("claude-code", "desktop")).toBe(false);

    const combos = [
      { cli: "not_installed", desktop: "not_installed" },
      { cli: "installed", desktop: "not_installed" },
      { cli: "not_installed", desktop: "installed" },
      { cli: "installed", desktop: "installed" },
    ] as const;
    for (const combo of combos) {
      const parsed = parseAgentInstallReadiness(
        readiness("opencode", combo),
        "opencode",
      );
      expect(parsed.surfaces).toHaveLength(2);
      expect(parsed.surfaces?.[0]?.surface).toBe("cli");
      expect(parsed.surfaces?.[0]?.installState).toBe(combo.cli);
      expect(parsed.surfaces?.[0]?.allowedActions).not.toContain("launch");
      expect(parsed.surfaces?.[1]?.surface).toBe("desktop");
      expect(parsed.surfaces?.[1]?.installState).toBe(combo.desktop);
      if (combo.desktop === "installed") {
        expect(parsed.surfaces?.[1]?.allowedActions).toContain("launch");
      }
    }
  });

  it("rejects unknown surfaces, illegal product pairs, and CLI launch", () => {
    expect(() =>
      parseAgentInstallReadiness(
        {
          ...readiness("qoderwork"),
          surfaces: [surfaceRow("cli", "installed")],
        },
        "qoderwork",
      ),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallReadiness(
        { ...readiness("opencode"), surfaces: undefined },
        "opencode",
      ),
    ).toThrow("Agent install readiness is unavailable");
    const withLaunchOnCli = readiness("opencode");
    withLaunchOnCli.surfaces = [
      { ...surfaceRow("cli", "installed"), allowedActions: ["launch"] },
      surfaceRow("desktop", "not_installed"),
    ];
    expect(() =>
      parseAgentInstallReadiness(withLaunchOnCli, "opencode"),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallationInventory(
        { ...inventory(), agentId: "opencode" },
        "opencode",
      ),
    ).toThrow("Agent installation inventory is unavailable");
    expect(() =>
      parseAgentActionJobSnapshot({
        contractVersion: AGENT_ACTION_CONTRACT_VERSION,
        jobId: "job-1",
        agentId: "qoderwork",
        action: "install",
        stage: "checking",
        cancellable: true,
        reasonCode: null,
        transfer: null,
        surface: "cli",
      }),
    ).toThrow("Agent action job is unavailable");
  });
});
