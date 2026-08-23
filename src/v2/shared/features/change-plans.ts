export const CHANGE_PLAN_CONTRACT_VERSION = "fyagent-change-plan/v1" as const;

export type ChangePlanErrorCode =
  | "unsupported_operation"
  | "invalid_target"
  | "target_not_found"
  | "target_already_current"
  | "secret_dependency_unavailable"
  | "invalid_digest"
  | "expired"
  | "consumed"
  | "stale"
  | "plan_not_found"
  | "job_not_found"
  | "internal";

export type ChangePlan = {
  readonly planId: string;
  readonly operation: "codex_provider_switch";
  readonly targetProviderId: string;
  readonly targetProviderName: string;
  readonly planDigest: string;
  readonly baselineDigest: string;
  readonly dbBaselineProviderId: string | null;
  readonly deviceBaselineProviderId: string | null;
  readonly secretCapability:
    | "no_new_credential_material"
    | "secret_dependency_unavailable";
  readonly createdAt: number;
  readonly expiresAt: number;
  readonly status: "ready" | "consumed";
  readonly currentProviderCode: string;
  readonly targetProviderCode: string;
  readonly restartExpectation: "not_required" | "recommended" | "unknown";
  readonly risks: ReadonlyArray<{
    readonly code: string;
    readonly severity: string;
  }>;
  readonly evidenceNote: string;
};

export type ChangeJobStep = {
  readonly kind: "precheck" | "apply" | "readback" | "reconcile";
  readonly status: "pending" | "running" | "succeeded" | "failed" | "skipped";
  readonly code: string;
};

export type ChangeResourceResult = {
  readonly kind:
    | "provider_db_current"
    | "device_current"
    | "target_definition"
    | "codex_live_projection";
  readonly status: "pending" | "matched" | "mismatched" | "unavailable";
  readonly code: string;
};

export type ChangeJobSnapshot = {
  readonly jobId: string;
  readonly planId: string;
  readonly targetProviderId: string;
  readonly revision: number;
  readonly eventSeq: number;
  readonly status: "planned" | "running" | "succeeded" | "warning" | "failed";
  readonly resultCode:
    | "planned"
    | "running"
    | "applied"
    | "applied_restart_recommended"
    | "applied_with_warning"
    | "writer_failed_baseline_restored"
    | "writer_error_target_reached"
    | "post_write_mismatch"
    | "readback_unavailable"
    | "recovery_required";
  readonly steps: readonly ChangeJobStep[];
  readonly resources: readonly ChangeResourceResult[];
  readonly events: ReadonlyArray<{
    readonly sequence: number;
    readonly phase: ChangeJobStep["kind"];
    readonly reasonCode: string;
    readonly createdAt: number;
  }>;
  readonly restartRequirement: "not_required" | "recommended" | "unknown";
  readonly usageEvidence: "not_observed";
  readonly recoveryState: "not_needed" | "succeeded" | "recovery_required";
  readonly diagnosticCode: string | null;
  readonly liveConfigChanged: boolean;
  readonly createdAt: number;
  readonly updatedAt: number;
};

export type ApplyChangePlanOutcome =
  | { readonly kind: "admitted"; readonly job: ChangeJobSnapshot }
  | { readonly kind: "rejected"; readonly errorCode: ChangePlanErrorCode };

export interface ChangePlansPort {
  createCodexProviderSwitchPlan(targetProviderId: string): Promise<ChangePlan>;
  applyChangePlan(input: {
    readonly planId: string;
    readonly planDigest: string;
  }): Promise<ApplyChangePlanOutcome>;
  getChangeJob(jobId: string): Promise<ChangeJobSnapshot>;
  listRecoverableChangeJobs(): Promise<ChangeJobSnapshot[]>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]) {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
  );
}

function isOneOf<T extends string>(
  value: unknown,
  values: readonly T[],
): value is T {
  return typeof value === "string" && values.includes(value as T);
}

function isInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function parseStep(value: unknown): ChangeJobStep {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "status", "code"]) ||
    !isOneOf(value.kind, [
      "precheck",
      "apply",
      "readback",
      "reconcile",
    ] as const) ||
    !isOneOf(value.status, [
      "pending",
      "running",
      "succeeded",
      "failed",
      "skipped",
    ] as const) ||
    typeof value.code !== "string"
  )
    throw new Error("Change Job is unavailable");
  return { kind: value.kind, status: value.status, code: value.code };
}

function parseResource(value: unknown): ChangeResourceResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "status", "code"]) ||
    !isOneOf(value.kind, [
      "provider_db_current",
      "device_current",
      "target_definition",
      "codex_live_projection",
    ] as const) ||
    !isOneOf(value.status, [
      "pending",
      "matched",
      "mismatched",
      "unavailable",
    ] as const) ||
    typeof value.code !== "string"
  )
    throw new Error("Change Job is unavailable");
  return { kind: value.kind, status: value.status, code: value.code };
}

export function parseChangePlan(value: unknown): ChangePlan {
  const keys = [
    "planId",
    "operation",
    "targetProviderId",
    "targetProviderName",
    "planDigest",
    "baselineDigest",
    "dbBaselineProviderId",
    "deviceBaselineProviderId",
    "secretCapability",
    "createdAt",
    "expiresAt",
    "status",
    "currentProviderCode",
    "targetProviderCode",
    "restartExpectation",
    "risks",
    "evidenceNote",
  ] as const;
  if (
    !isRecord(value) ||
    !hasExactKeys(value, keys) ||
    value.operation !== "codex_provider_switch" ||
    typeof value.targetProviderId !== "string" ||
    !value.targetProviderId ||
    typeof value.targetProviderName !== "string" ||
    typeof value.planId !== "string" ||
    !value.planId ||
    typeof value.planDigest !== "string" ||
    !/^[0-9a-f]{64}$/u.test(value.planDigest) ||
    typeof value.baselineDigest !== "string" ||
    !/^[0-9a-f]{64}$/u.test(value.baselineDigest) ||
    !isNullableString(value.dbBaselineProviderId) ||
    !isNullableString(value.deviceBaselineProviderId) ||
    !isOneOf(value.secretCapability, [
      "no_new_credential_material",
      "secret_dependency_unavailable",
    ] as const) ||
    !isInteger(value.createdAt) ||
    !isInteger(value.expiresAt) ||
    value.expiresAt <= value.createdAt ||
    !isOneOf(value.status, ["ready", "consumed"] as const) ||
    typeof value.currentProviderCode !== "string" ||
    typeof value.targetProviderCode !== "string" ||
    !isOneOf(value.restartExpectation, [
      "not_required",
      "recommended",
      "unknown",
    ] as const) ||
    !Array.isArray(value.risks) ||
    typeof value.evidenceNote !== "string"
  )
    throw new Error("Change Plan is unavailable");
  const risks = value.risks.map((risk) => {
    if (
      !isRecord(risk) ||
      !hasExactKeys(risk, ["code", "severity"]) ||
      typeof risk.code !== "string" ||
      typeof risk.severity !== "string"
    )
      throw new Error("Change Plan is unavailable");
    return { code: risk.code, severity: risk.severity };
  });
  return { ...value, risks } as unknown as ChangePlan;
}

export function parseChangeJobSnapshot(value: unknown): ChangeJobSnapshot {
  const keys = [
    "jobId",
    "planId",
    "targetProviderId",
    "revision",
    "eventSeq",
    "status",
    "resultCode",
    "steps",
    "resources",
    "events",
    "restartRequirement",
    "usageEvidence",
    "recoveryState",
    "diagnosticCode",
    "liveConfigChanged",
    "createdAt",
    "updatedAt",
  ] as const;
  if (
    !isRecord(value) ||
    !hasExactKeys(value, keys) ||
    ![value.jobId, value.planId, value.targetProviderId].every(
      (item) => typeof item === "string" && item.length > 0,
    ) ||
    !isInteger(value.revision) ||
    !isInteger(value.eventSeq) ||
    !isOneOf(value.status, [
      "planned",
      "running",
      "succeeded",
      "warning",
      "failed",
    ] as const) ||
    !isOneOf(value.resultCode, [
      "planned",
      "running",
      "applied",
      "applied_restart_recommended",
      "applied_with_warning",
      "writer_failed_baseline_restored",
      "writer_error_target_reached",
      "post_write_mismatch",
      "readback_unavailable",
      "recovery_required",
    ] as const) ||
    !Array.isArray(value.steps) ||
    !Array.isArray(value.resources) ||
    !Array.isArray(value.events) ||
    !isOneOf(value.restartRequirement, [
      "not_required",
      "recommended",
      "unknown",
    ] as const) ||
    value.usageEvidence !== "not_observed" ||
    !isOneOf(value.recoveryState, [
      "not_needed",
      "succeeded",
      "recovery_required",
    ] as const) ||
    !isNullableString(value.diagnosticCode) ||
    typeof value.liveConfigChanged !== "boolean" ||
    !isInteger(value.createdAt) ||
    !isInteger(value.updatedAt)
  )
    throw new Error("Change Job is unavailable");
  const events = value.events.map((event) => {
    if (
      !isRecord(event) ||
      !hasExactKeys(event, ["sequence", "phase", "reasonCode", "createdAt"]) ||
      !isInteger(event.sequence) ||
      !isOneOf(event.phase, [
        "precheck",
        "apply",
        "readback",
        "reconcile",
      ] as const) ||
      typeof event.reasonCode !== "string" ||
      !isInteger(event.createdAt)
    )
      throw new Error("Change Job is unavailable");
    return {
      sequence: event.sequence,
      phase: event.phase,
      reasonCode: event.reasonCode,
      createdAt: event.createdAt,
    };
  });
  return {
    ...value,
    steps: value.steps.map(parseStep),
    resources: value.resources.map(parseResource),
    events,
  } as unknown as ChangeJobSnapshot;
}

const ERROR_CODES = [
  "unsupported_operation",
  "invalid_target",
  "target_not_found",
  "target_already_current",
  "secret_dependency_unavailable",
  "invalid_digest",
  "expired",
  "consumed",
  "stale",
  "plan_not_found",
  "job_not_found",
  "internal",
] as const;

export function parseApplyChangePlanOutcome(
  value: unknown,
): ApplyChangePlanOutcome {
  if (
    !isRecord(value) ||
    !isOneOf(value.kind, ["admitted", "rejected"] as const)
  )
    throw new Error("Change Plan Apply is unavailable");
  if (value.kind === "admitted") {
    if (!hasExactKeys(value, ["kind", "job"]))
      throw new Error("Change Plan Apply is unavailable");
    return { kind: "admitted", job: parseChangeJobSnapshot(value.job) };
  }
  if (
    !hasExactKeys(value, ["kind", "errorCode"]) ||
    !isOneOf(value.errorCode, ERROR_CODES)
  )
    throw new Error("Change Plan Apply is unavailable");
  return { kind: "rejected", errorCode: value.errorCode };
}

export function parseRecoverableChangeJobs(
  value: unknown,
): ChangeJobSnapshot[] {
  if (!Array.isArray(value))
    throw new Error("Recoverable Change Jobs are unavailable");
  return value.map(parseChangeJobSnapshot);
}
