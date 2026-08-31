import { describe, expect, it } from "vitest";

import {
  AGENT_ACTION_CONTRACT_VERSION,
  AGENT_INSTALL_READINESS_CONTRACT_VERSION,
  AGENT_REASON_CODES,
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
): AgentInstallReadiness {
  const codex = agentId === "codex";
  const grok = agentId === "grokbuild";
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
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
      : grok
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
    expect(() =>
      parseAgentInstallReadiness(
        { ...readiness(), contractVersion: 3 },
        "qoderwork",
      ),
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
      parseAgentActionJobSnapshot({ ...snapshot, contractVersion: 3 }),
    ).toThrow("Agent action job is unavailable");
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

  it("maps desktop-only Agent lifecycle surfaces except Grok CLI", () => {
    expect(AGENT_CATALOG_IDS.map(surfacesForAgent)).toEqual([
      ["desktop"],
      ["desktop"],
      ["desktop"],
      ["cli"],
      ["desktop"],
      ["desktop"],
      ["desktop"],
    ]);
    expect(isLegalAgentSurface("qoderwork", "cli")).toBe(false);
    expect(isLegalAgentSurface("claude-code", "cli")).toBe(false);
    expect(isLegalAgentSurface("opencode", "cli")).toBe(false);
    expect(isLegalAgentSurface("claude-code", "desktop")).toBe(true);
    expect(isLegalAgentSurface("opencode", "desktop")).toBe(true);
    expect(isLegalAgentSurface("grokbuild", "desktop")).toBe(false);
    expect(isLegalAgentSurface("grokbuild", "cli")).toBe(true);
  });

  it("parses compact Claude and OpenCode desktop readiness without dual-surface aggregation", () => {
    const claude = parseAgentInstallReadiness(
      readiness("claude-code"),
      "claude-code",
    );
    expect(claude.sourceKind).toBe("managed_desktop");
    expect(claude.surfaces).toBeUndefined();

    const openCode = parseAgentInstallReadiness(
      {
        ...readiness("opencode"),
        installState: "installed",
        inventoryState: "single",
        updateState: "up_to_date",
        localVersion: "1.18.19",
        remoteVersion: "1.18.19",
        allowedActions: ["launch"],
        surfaces: [surfaceRow("desktop", "installed")],
      },
      "opencode",
    );
    expect(openCode.sourceKind).toBe("managed_desktop");
    expect(openCode.surfaces).toHaveLength(1);
    expect(openCode.surfaces?.[0]?.surface).toBe("desktop");
    expect(openCode.surfaces?.[0]?.allowedActions).toContain("launch");

    expect(
      parseAgentInstallReadiness(
        { ...readiness("opencode"), surfaces: undefined },
        "opencode",
      ).surfaces,
    ).toBeUndefined();
  });

  it("rejects unknown surfaces, removed CLI pairs, and CLI launch", () => {
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
        {
          ...readiness("claude-code"),
          surfaces: [surfaceRow("cli", "installed")],
        },
        "claude-code",
      ),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallReadiness(
        {
          ...readiness("opencode"),
          surfaces: [
            surfaceRow("cli", "not_installed"),
            surfaceRow("desktop", "installed"),
          ],
        },
        "opencode",
      ),
    ).toThrow("Agent install readiness is unavailable");
    expect(() =>
      parseAgentInstallReadiness(
        {
          ...readiness("opencode"),
          sourceKind: "cli_tooling",
        },
        "opencode",
      ),
    ).toThrow("Agent install readiness is unavailable");
    const grokWithLaunch = {
      ...readiness("grokbuild"),
      surfaces: [
        { ...surfaceRow("cli", "installed"), allowedActions: ["launch"] },
      ],
    };
    expect(() =>
      parseAgentInstallReadiness(grokWithLaunch, "grokbuild"),
    ).toThrow("Agent install readiness is unavailable");
    expect(
      parseAgentInstallationInventory(
        {
          ...inventory(),
          agentId: "opencode",
          state: "not_observed",
          candidates: [],
          freshDestinations: [],
        },
        "opencode",
      ).agentId,
    ).toBe("opencode");
    expect(() =>
      parseAgentInstallationInventory(
        {
          ...inventory(),
          agentId: "opencode",
          state: "not_observed",
          candidates: [],
          freshDestinations: [],
          surface: "desktop",
        },
        "opencode",
      ),
    ).toThrow("Agent installation inventory is unavailable");
    expect(() =>
      parseAgentInstallationInventory(
        {
          ...inventory(),
          agentId: "opencode",
          state: "not_observed",
          candidates: [],
          freshDestinations: [],
          surface: "cli",
        },
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
    expect(() =>
      parseAgentActionJobSnapshot({
        contractVersion: AGENT_ACTION_CONTRACT_VERSION,
        jobId: "job-1",
        agentId: "claude-code",
        action: "install",
        stage: "checking",
        cancellable: true,
        reasonCode: null,
        transfer: null,
        surface: "cli",
      }),
    ).toThrow("Agent action job is unavailable");
    expect(() =>
      parseAgentActionJobSnapshot({
        contractVersion: AGENT_ACTION_CONTRACT_VERSION,
        jobId: "job-1",
        agentId: "opencode",
        action: "install",
        stage: "checking",
        cancellable: true,
        reasonCode: null,
        transfer: null,
        surface: "cli",
      }),
    ).toThrow("Agent action job is unavailable");
  });

  it("accepts action_not_supported as a closed reason code", () => {
    expect(AGENT_REASON_CODES).toContain("action_not_supported");
    expect(
      parseAgentInstallReadiness(
        {
          ...readiness("qoderwork"),
          reasonCodes: ["action_not_supported"],
        },
        "qoderwork",
      ).reasonCodes,
    ).toEqual(["action_not_supported"]);
  });
});
