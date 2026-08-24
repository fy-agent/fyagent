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
  const officialGuide = [
    "qoderwork",
    "trae-work",
    "workbuddy",
    "grokbuild",
  ].includes(agentId);
  return {
    contractVersion: 1,
    agentId,
    reviewedAt: "2026-08-24",
    automation: {
      state: "unavailable",
      reasonCode: codex
        ? "managed_by_codex_desktop"
        : officialGuide
          ? "official_guide_only"
          : "executor_not_implemented",
    },
    source: {
      state: "unknown",
      reasonCode: "source_review_not_refreshed",
      installMode: codex
        ? "managed_package"
        : officialGuide
          ? "official_guide"
          : "unsupported",
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
    ]) {
      expect(() =>
        assertAgentInstallReadinessId(
          invalid as (typeof AGENT_CATALOG_IDS)[number],
        ),
      ).toThrow("Agent install readiness request is invalid");
    }
  });

  it("preserves unavailable automation, unknown layers, and a null-only plan", () => {
    const generic = parseAgentInstallReadiness(
      readiness("workbuddy"),
      "workbuddy",
    );
    expect(generic.automation).toEqual({
      state: "unavailable",
      reasonCode: "official_guide_only",
    });
    expect([
      generic.source.state,
      generic.integrity.state,
      generic.preflight.state,
      generic.plan.state,
    ]).toEqual(["unknown", "unknown", "unknown", "unknown"]);
    expect(generic.plan).toEqual({
      state: "unknown",
      reasonCode: "plan_not_created",
      snapshotId: null,
      snapshotStale: null,
    });

    const codex = parseAgentInstallReadiness(readiness("codex"), "codex");
    expect(codex.automation.reasonCode).toBe("managed_by_codex_desktop");
    expect(codex.source.installMode).toBe("managed_package");
  });

  it("rejects excess, mismatched, positive, snapshot, and sensitive fields", () => {
    const cases: unknown[] = [
      { ...readiness(), contractVersion: 2 },
      { ...readiness(), agentId: "trae-work" },
      { ...readiness(), reviewedAt: "not-a-date" },
      { ...readiness(), packageUrl: "https://example.test/tool.pkg" },
      {
        ...readiness(),
        source: { ...readiness().source, licenseScope: "public_open_source" },
      },
      {
        ...readiness(),
        integrity: { ...readiness().integrity, packageHash: "sentinel" },
      },
      {
        ...readiness(),
        preflight: {
          ...readiness().preflight,
          checks: [{ code: "fake_doctor", state: "ok" }],
        },
      },
      {
        ...readiness(),
        plan: { ...readiness().plan, snapshotId: "snapshot-sentinel" },
      },
      {
        ...readiness(),
        automation: {
          state: "available",
          reasonCode: "executor_not_implemented",
        },
      },
    ];

    for (const candidate of cases) {
      expect(() => parseAgentInstallReadiness(candidate, "qoderwork")).toThrow(
        "Agent install readiness is unavailable",
      );
    }
  });

  it("keeps the accepted wire free of sensitive installation material", () => {
    const serialized = JSON.stringify(
      parseAgentInstallReadiness(readiness(), "qoderwork"),
    ).toLowerCase();
    for (const prohibited of [
      "url",
      "path",
      "hash",
      "script",
      "secret",
      "package",
      "signer",
      "fingerprint",
      "token",
    ]) {
      expect(serialized).not.toContain(prohibited);
    }
  });
});
