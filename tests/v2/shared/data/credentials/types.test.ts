import { describe, expect, it } from "vitest";

import {
  AVAILABILITY_LABELS_ZH,
  BINDING_STATE_LABELS_ZH,
  COMPARISON_MEANING_ZH,
  SECRET_LOCK_SOURCES,
  SECRET_REVOCATION_SOURCES,
  SECRET_STABLE_AVAILABILITIES,
  SECRET_USER_ACTION_LABELS_ZH,
  SECRET_USER_ACTIONS,
  secretRefDisplayOf,
  type CodexProviderDeleteImpactDto,
  type SecretDeleteImpact,
  type SecretRef,
} from "@/v2/shared/data/credentials";

describe("credentials public no-value types", () => {
  it("closes the seven stable availabilities with icon-ready Chinese labels", () => {
    expect(SECRET_STABLE_AVAILABILITIES).toEqual([
      "ready",
      "missing",
      "locked",
      "denied",
      "stale",
      "revoked",
      "unavailable",
    ]);
    expect(Object.keys(AVAILABILITY_LABELS_ZH)).toEqual([
      ...SECRET_STABLE_AVAILABILITIES,
    ]);
    expect(AVAILABILITY_LABELS_ZH.revoked).toBe("已撤销");
    expect(AVAILABILITY_LABELS_ZH.missing).toBe("缺失");
    expect(AVAILABILITY_LABELS_ZH.revoked).not.toBe(
      AVAILABILITY_LABELS_ZH.missing,
    );
  });

  it("splits lock and revocation sources", () => {
    expect(SECRET_LOCK_SOURCES).toEqual(["fyAgentPolicy", "backend"]);
    expect(SECRET_REVOCATION_SOURCES).toEqual([
      "userDelete",
      "centralBackend",
      "deviceAdministration",
      "supersededByRotation",
    ]);
    expect(BINDING_STATE_LABELS_ZH).toEqual({
      bound: "已绑定",
      legacy: "明文待处理",
      unbound: "未绑定",
    });
  });

  it("keeps SecretUserAction a closed labeled enum", () => {
    expect(SECRET_USER_ACTIONS).toHaveLength(24);
    expect(SECRET_USER_ACTIONS).not.toContain("retry");
    expect(Object.keys(SECRET_USER_ACTION_LABELS_ZH)).toEqual([
      ...SECRET_USER_ACTIONS,
    ]);
    expect(SECRET_USER_ACTION_LABELS_ZH.unlockFyAgent).toBe("解锁 FyAgent");
    expect(SECRET_USER_ACTION_LABELS_ZH.unlockBackend).toBe("到系统解锁");
    expect(SECRET_USER_ACTION_LABELS_ZH.unlockFyAgent).not.toBe(
      SECRET_USER_ACTION_LABELS_ZH.unlockBackend,
    );
  });

  it("derives secretRefDisplay as sec_… plus last four and never as identity", () => {
    const secretRef =
      "sec_1111111111114111811111111111ab12" as SecretRef;
    expect(secretRefDisplayOf(secretRef)).toBe("sec_…ab12");
    expect(secretRefDisplayOf(secretRef)).not.toBe(secretRef);
  });

  it("requires secret delete noFallback and full owner objects", () => {
    const impact: SecretDeleteImpact = {
      impact: {
        schemaVersion: 1,
        secretRefDisplay: "sec_…ab12" as never,
        bindingSetCas: {
          revision: 1 as never,
          digest: "11".repeat(32) as never,
          count: 2,
        },
        affectedOwners: [
          {
            owner: {
              kind: "provider",
              namespace: "codex" as never,
              ownerId: "alpha-ready" as never,
              slot: "primaryApiKey",
            },
            purpose: "codexApiKey",
            bindingRevision: 1 as never,
            createdAt: "2026-08-18T00:00:00.000Z" as never,
            updatedAt: "2026-08-18T00:00:00.000Z" as never,
          },
        ],
        effect: "allBindingsAffected",
        noFallback: true,
      },
      readiness: { status: "ready" },
    };
    expect(impact.impact.noFallback).toBe(true);
    expect(impact.impact.affectedOwners.length).toBeGreaterThanOrEqual(1);
  });

  it("keeps provider delete ready and blocked as distinct DTO arms", () => {
    const ready: CodexProviderDeleteImpactDto = {
      schemaVersion: 1,
      status: "ready",
      impact: {
        bindingState: "bound",
        providerDeleteImpactId: "pdi_eeeeeeeeeeee4eee8eeeeeeeeeee6006" as never,
        owner: {
          kind: "provider",
          namespace: "codex" as never,
          ownerId: "alpha-ready" as never,
          slot: "primaryApiKey",
        },
        existingBinding: {
          state: "bound",
          secretRefDisplay: "sec_…ab12" as never,
          remainingOwners: [],
          becomesOrphan: true,
        },
        legacySourceCoverage: {
          state: "clear",
          currentScrubbable: { state: "none", sourceCount: 0, categories: [] },
          adjacentBlocked: {
            state: "none",
            observationCount: 0,
            observations: [],
          },
        },
        deleteAllowed: true,
        effect: "none",
        secretRetained: true,
        separateSecretDeleteAction: "get_secret_delete_impact",
      },
    };
    const blocked: CodexProviderDeleteImpactDto = {
      schemaVersion: 1,
      status: "blockedLegacyResolutionRequired",
      blocked: {
        bindingState: "legacy",
        owner: {
          kind: "provider",
          namespace: "codex" as never,
          ownerId: "beta-legacy" as never,
          slot: "primaryApiKey",
        },
        existingBinding: {
          state: "bound",
          secretRefDisplay: "sec_…ab12" as never,
          remainingOwners: [],
          becomesOrphan: false,
        },
        legacySourceCoverage: {
          state: "blockingSourcesPresent",
          currentScrubbable: {
            state: "currentSourcesPresent",
            sourceCount: 1,
            categories: ["providerAuthJson"],
          },
          adjacentBlocked: {
            state: "none",
            observationCount: 0,
            observations: [],
          },
        },
        deleteAllowed: false,
        effect: "none",
        action: "resolveLegacyConflict",
      },
    };
    expect(ready.status).toBe("ready");
    expect(ready.impact.secretRetained).toBe(true);
    expect(blocked.status).toBe("blockedLegacyResolutionRequired");
    expect("providerDeleteImpactId" in blocked.blocked).toBe(false);
    expect(COMPARISON_MEANING_ZH.candidateEquality).toBe(
      "核验同一凭据后迁移",
    );
    expect(COMPARISON_MEANING_ZH.explicitReplacement).toBe(
      "替换这些旧来源，不要求旧值等于新值",
    );
  });
});
