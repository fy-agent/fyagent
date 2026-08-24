import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export const AGENT_INSTALL_READINESS_CONTRACT_VERSION = 1 as const;

export const READINESS_LAYER_STATES = [
  "ok",
  "warn",
  "fail",
  "unknown",
] as const;
export type ReadinessLayerState = (typeof READINESS_LAYER_STATES)[number];

export const AGENT_INSTALL_MODES = [
  "official_guide",
  "managed_package",
  "native_verified",
  "unsupported",
] as const;
export type AgentInstallMode = (typeof AGENT_INSTALL_MODES)[number];

export interface AgentInstallReadiness {
  contractVersion: typeof AGENT_INSTALL_READINESS_CONTRACT_VERSION;
  agentId: AgentCatalogId;
  reviewedAt: string;
  automation: {
    state: "unavailable";
    reasonCode:
      | "official_guide_only"
      | "executor_not_implemented"
      | "managed_by_codex_desktop";
  };
  source: {
    state: ReadinessLayerState;
    reasonCode: "source_review_not_refreshed";
    installMode: AgentInstallMode;
    licenseScope: "unconfirmed";
    distributionState: "unconfirmed";
    checkedAt: null;
  };
  integrity: {
    state: ReadinessLayerState;
    summaryCode: "integrity_not_checked";
    checkedAt: null;
  };
  preflight: {
    state: ReadinessLayerState;
    reasonCode: "preflight_not_run";
    checks: Array<{
      code: "os_compatibility" | "architecture_compatibility" | "requirements";
      state: ReadinessLayerState;
    }>;
    checkedAt: null;
  };
  plan: {
    state: ReadinessLayerState;
    reasonCode: "plan_not_created";
    snapshotId: null;
    snapshotStale: null;
  };
}

export interface AgentInstallReadinessPort {
  get(agentId: AgentCatalogId): Promise<AgentInstallReadiness>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
  );
}

function isOneOf<T extends string>(
  value: unknown,
  candidates: readonly T[],
): value is T {
  return typeof value === "string" && candidates.includes(value as T);
}

function parseLayerState(value: unknown): ReadinessLayerState {
  if (!isOneOf(value, READINESS_LAYER_STATES)) {
    throw new Error("Agent install readiness is unavailable");
  }
  return value;
}

function parsePreflightCheck(
  value: unknown,
): AgentInstallReadiness["preflight"]["checks"][number] {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["code", "state"]) ||
    !isOneOf(value.code, [
      "os_compatibility",
      "architecture_compatibility",
      "requirements",
    ] as const)
  ) {
    throw new Error("Agent install readiness is unavailable");
  }
  return { code: value.code, state: parseLayerState(value.state) };
}

export function parseAgentInstallReadiness(
  value: unknown,
  expectedAgentId: AgentCatalogId,
): AgentInstallReadiness {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "agentId",
      "reviewedAt",
      "automation",
      "source",
      "integrity",
      "preflight",
      "plan",
    ]) ||
    value.contractVersion !== AGENT_INSTALL_READINESS_CONTRACT_VERSION ||
    value.agentId !== expectedAgentId ||
    typeof value.reviewedAt !== "string" ||
    !/^\d{4}-\d{2}-\d{2}$/u.test(value.reviewedAt) ||
    !isRecord(value.automation) ||
    !hasExactKeys(value.automation, ["state", "reasonCode"]) ||
    value.automation.state !== "unavailable" ||
    !isOneOf(value.automation.reasonCode, [
      "executor_not_implemented",
      "official_guide_only",
      "managed_by_codex_desktop",
    ] as const) ||
    !isRecord(value.source) ||
    !hasExactKeys(value.source, [
      "state",
      "reasonCode",
      "installMode",
      "licenseScope",
      "distributionState",
      "checkedAt",
    ]) ||
    value.source.reasonCode !== "source_review_not_refreshed" ||
    !isOneOf(value.source.installMode, AGENT_INSTALL_MODES) ||
    value.source.licenseScope !== "unconfirmed" ||
    value.source.distributionState !== "unconfirmed" ||
    value.source.checkedAt !== null ||
    !isRecord(value.integrity) ||
    !hasExactKeys(value.integrity, ["state", "summaryCode", "checkedAt"]) ||
    value.integrity.summaryCode !== "integrity_not_checked" ||
    value.integrity.checkedAt !== null ||
    !isRecord(value.preflight) ||
    !hasExactKeys(value.preflight, [
      "state",
      "reasonCode",
      "checks",
      "checkedAt",
    ]) ||
    value.preflight.reasonCode !== "preflight_not_run" ||
    !Array.isArray(value.preflight.checks) ||
    value.preflight.checkedAt !== null ||
    !isRecord(value.plan) ||
    !hasExactKeys(value.plan, [
      "state",
      "reasonCode",
      "snapshotId",
      "snapshotStale",
    ]) ||
    value.plan.reasonCode !== "plan_not_created" ||
    value.plan.snapshotId !== null ||
    value.plan.snapshotStale !== null
  ) {
    throw new Error("Agent install readiness is unavailable");
  }

  const automationReason = value.automation.reasonCode;
  const installMode = value.source.installMode;
  const matchesAgentPolicy =
    expectedAgentId === "codex"
      ? automationReason === "managed_by_codex_desktop" &&
        installMode === "managed_package"
      : isOneOf(expectedAgentId, [
            "qoderwork",
            "trae-work",
            "workbuddy",
            "grokbuild",
          ] as const)
        ? automationReason === "official_guide_only" &&
          installMode === "official_guide"
        : automationReason === "executor_not_implemented" &&
          installMode === "unsupported";
  if (!matchesAgentPolicy) {
    throw new Error("Agent install readiness is unavailable");
  }

  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    agentId: expectedAgentId,
    reviewedAt: value.reviewedAt as string,
    automation: {
      state: "unavailable",
      reasonCode: automationReason,
    },
    source: {
      state: parseLayerState(value.source.state),
      reasonCode: "source_review_not_refreshed",
      installMode,
      licenseScope: "unconfirmed",
      distributionState: "unconfirmed",
      checkedAt: null,
    },
    integrity: {
      state: parseLayerState(value.integrity.state),
      summaryCode: "integrity_not_checked",
      checkedAt: null,
    },
    preflight: {
      state: parseLayerState(value.preflight.state),
      reasonCode: "preflight_not_run",
      checks: value.preflight.checks.map(parsePreflightCheck),
      checkedAt: null,
    },
    plan: {
      state: parseLayerState(value.plan.state),
      reasonCode: "plan_not_created",
      snapshotId: null,
      snapshotStale: null,
    },
  };
}

export function assertAgentInstallReadinessId(
  agentId: AgentCatalogId,
): AgentCatalogId {
  if (!isOneOf(agentId, AGENT_CATALOG_IDS)) {
    throw new Error("Agent install readiness request is invalid");
  }
  return agentId;
}
