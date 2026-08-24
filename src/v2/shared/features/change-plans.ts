export const CHANGE_PLAN_CONTRACT_VERSION = "fyagent-change-plan/v2" as const;

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

export type ChangeStepKind =
  | "precheck"
  | "snapshot"
  | "managed_write"
  | "readback"
  | "finalize";

export type ChangeResourceKind =
  | "provider_db_current"
  | "device_current"
  | "target_definition"
  | "codex_live_projection";

export type ChangeAdapterDescriptor = {
  readonly adapterId: "codex_provider_switch";
  readonly adapterVersion: "1";
  readonly operationType: "codex_provider_switch";
  readonly phases: readonly ChangeStepKind[];
  readonly readSet: readonly ChangeResourceKind[];
  readonly writeSet: readonly ChangeResourceKind[];
  readonly idempotencyScope: "plan";
  readonly cancelMode: "before_managed_write";
  readonly compensationMode: "writer_owned_rollback";
  readonly faultPoints: readonly (
    | "before_managed_write"
    | "after_managed_write_before_record"
  )[];
};

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
  readonly adapter: ChangeAdapterDescriptor;
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
  readonly kind: ChangeStepKind;
  readonly status:
    | "pending"
    | "running"
    | "succeeded"
    | "failed"
    | "compensating"
    | "compensated"
    | "skipped";
  readonly code: string;
};

export type ChangeResourceResult = {
  readonly kind: ChangeResourceKind;
  readonly status: "pending" | "matched" | "mismatched" | "unavailable";
  readonly code: string;
};

export type ChangePartialResult = {
  readonly succeededSteps: readonly ChangeStepKind[];
  readonly compensatedSteps: readonly ChangeStepKind[];
  readonly unverifiedSteps: readonly ChangeStepKind[];
  readonly remainingEffects: readonly ChangeResourceKind[];
  readonly manualActions: readonly (
    | "retry_readback"
    | "review_configuration"
  )[];
};

export type ChangeJobSnapshot = {
  readonly jobId: string;
  readonly executionId: string;
  readonly planId: string;
  readonly idempotencyKey: string;
  readonly targetProviderId: string;
  readonly revision: number;
  readonly eventSeq: number;
  readonly status:
    | "planned"
    | "running"
    | "succeeded"
    | "warning"
    | "failed"
    | "cancelled";
  readonly resultCode:
    | "planned"
    | "running"
    | "applied"
    | "applied_restart_recommended"
    | "applied_with_warning"
    | "cancelled_before_write"
    | "interrupted_before_write"
    | "recovered_target_reached"
    | "writer_failed_baseline_restored"
    | "writer_error_target_reached"
    | "post_write_mismatch"
    | "readback_unavailable"
    | "recovery_required";
  readonly adapterErrorCode:
    | "precondition_failed"
    | "writer_failed"
    | "unknown_outcome"
    | "verify_failed"
    | "compensation_failed"
    | "unsupported"
    | null;
  readonly steps: readonly ChangeJobStep[];
  readonly resources: readonly ChangeResourceResult[];
  readonly partialResult: ChangePartialResult | null;
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
  | { readonly kind: "idempotent_replay"; readonly job: ChangeJobSnapshot }
  | { readonly kind: "rejected"; readonly errorCode: ChangePlanErrorCode };

export type CancelChangeJobOutcome = {
  readonly accepted: boolean;
  readonly code:
    | "accepted"
    | "commit_point_passed"
    | "already_terminal"
    | "not_active"
    | "job_not_found";
  readonly jobId: string;
};

export interface ChangePlansPort {
  createCodexProviderSwitchPlan(targetProviderId: string): Promise<ChangePlan>;
  applyChangePlan(input: {
    readonly planId: string;
    readonly planDigest: string;
  }): Promise<ApplyChangePlanOutcome>;
  cancelChangeJob(jobId: string): Promise<CancelChangeJobOutcome>;
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

const STEP_KINDS = [
  "precheck",
  "snapshot",
  "managed_write",
  "readback",
  "finalize",
] as const;

const RESOURCE_KINDS = [
  "provider_db_current",
  "device_current",
  "target_definition",
  "codex_live_projection",
] as const;

function isExactSequence(
  value: unknown,
  expected: readonly string[],
): value is string[] {
  return (
    Array.isArray(value) &&
    value.length === expected.length &&
    value.every((item, index) => item === expected[index])
  );
}

function parseEnumArray<T extends string>(
  value: unknown,
  allowed: readonly T[],
  label: string,
): T[] {
  if (
    !Array.isArray(value) ||
    !value.every((item): item is T => isOneOf(item, allowed)) ||
    new Set(value).size !== value.length
  ) {
    throw new Error(label);
  }
  return value;
}

function parseAdapterDescriptor(value: unknown): ChangeAdapterDescriptor {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "adapterId",
      "adapterVersion",
      "operationType",
      "phases",
      "readSet",
      "writeSet",
      "idempotencyScope",
      "cancelMode",
      "compensationMode",
      "faultPoints",
    ]) ||
    value.adapterId !== "codex_provider_switch" ||
    value.adapterVersion !== "1" ||
    value.operationType !== "codex_provider_switch" ||
    !isExactSequence(value.phases, STEP_KINDS) ||
    !isExactSequence(value.readSet, RESOURCE_KINDS) ||
    !isExactSequence(value.writeSet, [
      "provider_db_current",
      "device_current",
      "codex_live_projection",
    ]) ||
    value.idempotencyScope !== "plan" ||
    value.cancelMode !== "before_managed_write" ||
    value.compensationMode !== "writer_owned_rollback" ||
    !isExactSequence(value.faultPoints, [
      "before_managed_write",
      "after_managed_write_before_record",
    ])
  ) {
    throw new Error("Change Plan is unavailable");
  }
  return value as unknown as ChangeAdapterDescriptor;
}

function parseStep(value: unknown): ChangeJobStep {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "status", "code"]) ||
    !isOneOf(value.kind, STEP_KINDS) ||
    !isOneOf(value.status, [
      "pending",
      "running",
      "succeeded",
      "failed",
      "compensating",
      "compensated",
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
    !isOneOf(value.kind, RESOURCE_KINDS) ||
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

function parsePartialResult(value: unknown): ChangePartialResult | null {
  if (value === null) return null;
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "succeededSteps",
      "compensatedSteps",
      "unverifiedSteps",
      "remainingEffects",
      "manualActions",
    ])
  ) {
    throw new Error("Change Job is unavailable");
  }
  return {
    succeededSteps: parseEnumArray(
      value.succeededSteps,
      STEP_KINDS,
      "Change Job is unavailable",
    ),
    compensatedSteps: parseEnumArray(
      value.compensatedSteps,
      STEP_KINDS,
      "Change Job is unavailable",
    ),
    unverifiedSteps: parseEnumArray(
      value.unverifiedSteps,
      STEP_KINDS,
      "Change Job is unavailable",
    ),
    remainingEffects: parseEnumArray(
      value.remainingEffects,
      RESOURCE_KINDS,
      "Change Job is unavailable",
    ),
    manualActions: parseEnumArray(
      value.manualActions,
      ["retry_readback", "review_configuration"] as const,
      "Change Job is unavailable",
    ),
  };
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
    "adapter",
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
    !isRecord(value.adapter) ||
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
  return {
    ...value,
    adapter: parseAdapterDescriptor(value.adapter),
    risks,
  } as unknown as ChangePlan;
}

export function parseChangeJobSnapshot(value: unknown): ChangeJobSnapshot {
  const keys = [
    "jobId",
    "executionId",
    "planId",
    "idempotencyKey",
    "targetProviderId",
    "revision",
    "eventSeq",
    "status",
    "resultCode",
    "adapterErrorCode",
    "steps",
    "resources",
    "partialResult",
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
    ![
      value.jobId,
      value.executionId,
      value.planId,
      value.idempotencyKey,
      value.targetProviderId,
    ].every((item) => typeof item === "string" && item.length > 0) ||
    value.executionId !== value.jobId ||
    value.idempotencyKey !== value.planId ||
    !isInteger(value.revision) ||
    !isInteger(value.eventSeq) ||
    !isOneOf(value.status, [
      "planned",
      "running",
      "succeeded",
      "warning",
      "failed",
      "cancelled",
    ] as const) ||
    !isOneOf(value.resultCode, [
      "planned",
      "running",
      "applied",
      "applied_restart_recommended",
      "applied_with_warning",
      "cancelled_before_write",
      "interrupted_before_write",
      "recovered_target_reached",
      "writer_failed_baseline_restored",
      "writer_error_target_reached",
      "post_write_mismatch",
      "readback_unavailable",
      "recovery_required",
    ] as const) ||
    !(
      value.adapterErrorCode === null ||
      isOneOf(value.adapterErrorCode, [
        "precondition_failed",
        "writer_failed",
        "unknown_outcome",
        "verify_failed",
        "compensation_failed",
        "unsupported",
      ] as const)
    ) ||
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
      !isOneOf(event.phase, STEP_KINDS) ||
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
  if (
    events.length === 0 ||
    events[events.length - 1]?.sequence !== value.eventSeq ||
    events.some(
      (event, index) =>
        index > 0 && event.sequence <= events[index - 1]!.sequence,
    )
  ) {
    throw new Error("Change Job is unavailable");
  }
  const steps = value.steps.map(parseStep);
  const resources = value.resources.map(parseResource);
  if (
    !isExactSequence(
      steps.map((step) => step.kind),
      STEP_KINDS,
    ) ||
    !isExactSequence(
      resources.map((resource) => resource.kind),
      RESOURCE_KINDS,
    )
  ) {
    throw new Error("Change Job is unavailable");
  }
  return {
    ...value,
    steps,
    resources,
    partialResult: parsePartialResult(value.partialResult),
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
    !isOneOf(value.kind, ["admitted", "idempotent_replay", "rejected"] as const)
  )
    throw new Error("Change Plan Apply is unavailable");
  if (value.kind === "admitted" || value.kind === "idempotent_replay") {
    if (!hasExactKeys(value, ["kind", "job"]))
      throw new Error("Change Plan Apply is unavailable");
    return { kind: value.kind, job: parseChangeJobSnapshot(value.job) };
  }
  if (
    !hasExactKeys(value, ["kind", "errorCode"]) ||
    !isOneOf(value.errorCode, ERROR_CODES)
  )
    throw new Error("Change Plan Apply is unavailable");
  return { kind: "rejected", errorCode: value.errorCode };
}

export function parseCancelChangeJobOutcome(
  value: unknown,
): CancelChangeJobOutcome {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["accepted", "code", "jobId"]) ||
    typeof value.accepted !== "boolean" ||
    !isOneOf(value.code, [
      "accepted",
      "commit_point_passed",
      "already_terminal",
      "not_active",
      "job_not_found",
    ] as const) ||
    typeof value.jobId !== "string" ||
    !value.jobId ||
    value.accepted !== (value.code === "accepted")
  ) {
    throw new Error("Change Job Cancel is unavailable");
  }
  return value as CancelChangeJobOutcome;
}

export function parseRecoverableChangeJobs(
  value: unknown,
): ChangeJobSnapshot[] {
  if (!Array.isArray(value))
    throw new Error("Recoverable Change Jobs are unavailable");
  return value.map(parseChangeJobSnapshot);
}
