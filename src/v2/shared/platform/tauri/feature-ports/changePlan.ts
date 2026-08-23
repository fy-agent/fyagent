import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  CHANGE_STEP_KINDS,
  type ApplyChangePlanOutcome,
  type CancelChangeJobOutcome,
  type ChangeAdapterDescriptor,
  type ChangeJobSnapshot,
  type ChangeJobStep,
  type ChangeJobUpdatedEvent,
  type ChangePartialResult,
  type ChangePlan,
  type ChangePlanErrorCode,
  type ChangeResourceResult,
} from "../../../features/change-plan";
import type { FeaturePorts } from "../../../features/ports";
import type { WorkBuddyChangePlanRequest } from "../../../features/types";
import { assertQuickSetupRequest } from "./models";
import {
  hasExactKeys,
  hasRequiredAndOptionalKeys,
  isOneOf,
  isRecord,
  isStringArray,
} from "./validation";

const JOB_UPDATED_EVENT = "change-job://updated";
const JOB_STATUSES = [
  "planned",
  "running",
  "succeeded",
  "warning",
  "failed",
  "cancelled",
] as const;
const STEP_STATUSES = [
  "not_started",
  "running",
  "succeeded",
  "failed",
  "compensating",
  "compensated",
  "skipped",
] as const;
const RESOURCE_KINDS = [
  "provider_db_current",
  "device_current",
  "target_definition",
  "codex_live_projection",
  "work_buddy_models_config",
  "work_buddy_backup",
] as const;
const RESOURCE_STATUSES = [
  "pending",
  "matched",
  "mismatched",
  "unavailable",
] as const;
const RESTART_REQUIREMENTS = [
  "not_required",
  "recommended",
  "unknown",
] as const;
const RECOVERY_STATES = [
  "not_needed",
  "succeeded",
  "recovery_required",
] as const;
const ADAPTER_ERRORS = [
  "precondition_failed",
  "transient",
  "permanent",
  "unknown_outcome",
  "verify_failed",
  "compensation_failed",
  "unsupported",
] as const;
const RESULT_CODES = [
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
] as const;
const PLAN_ERRORS = [
  "unsupported_operation",
  "invalid_request",
  "target_not_found",
  "target_already_current",
  "baseline_unavailable",
  "invalid_digest",
  "expired",
  "consumed",
  "stale",
  "plan_not_found",
  "job_not_found",
  "internal",
] as const satisfies readonly ChangePlanErrorCode[];
const CANCEL_CODES = [
  "accepted",
  "commit_point_passed",
  "already_terminal",
  "not_active",
  "job_not_found",
] as const;

function isSafeId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.trim() === value &&
    value.length <= 256
  );
}

function isInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function parseStringList(value: unknown, error: string): string[] {
  if (!isStringArray(value) || value.some((item) => item.length > 256))
    throw new Error(error);
  return [...value];
}

function parseAdapter(value: unknown): ChangeAdapterDescriptor {
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
    !isSafeId(value.adapterId) ||
    !isSafeId(value.adapterVersion) ||
    !isOneOf(value.operationType, [
      "codex_provider_switch",
      "codex_provider_upsert_and_switch",
      "work_buddy_models_update",
    ] as const) ||
    !Array.isArray(value.phases) ||
    value.phases.length !== CHANGE_STEP_KINDS.length ||
    !value.phases.every((item, index) => item === CHANGE_STEP_KINDS[index]) ||
    !Array.isArray(value.readSet) ||
    !value.readSet.every((item) => isOneOf(item, RESOURCE_KINDS)) ||
    !Array.isArray(value.writeSet) ||
    !value.writeSet.every((item) => isOneOf(item, RESOURCE_KINDS)) ||
    value.idempotencyScope !== "plan" ||
    value.cancelMode !== "before_managed_write" ||
    value.compensationMode !== "writer_owned_rollback" ||
    !Array.isArray(value.faultPoints) ||
    !value.faultPoints.every((item) =>
      isOneOf(item, [
        "before_managed_write",
        "after_managed_write_before_record",
      ] as const),
    )
  )
    throw new Error("Change Plan adapter is unavailable");
  return {
    adapterId: value.adapterId,
    adapterVersion: value.adapterVersion,
    operationType: value.operationType,
    phases: [...value.phases],
    readSet: [...value.readSet],
    writeSet: [...value.writeSet],
    idempotencyScope: value.idempotencyScope,
    cancelMode: value.cancelMode,
    compensationMode: value.compensationMode,
    faultPoints: [...value.faultPoints],
  };
}

function parsePlan(value: unknown): ChangePlan {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(
      value,
      [
        "planId",
        "operation",
        "targetProviderId",
        "targetProviderName",
        "planDigest",
        "baselineDigest",
        "actor",
        "sourceVersion",
        "revision",
        "createdAt",
        "expiresAt",
        "status",
        "businessSteps",
        "adapter",
        "currentProviderCode",
        "targetProviderCode",
        "restartExpectation",
        "risks",
        "evidenceNote",
      ],
      ["credential"],
    ) ||
    !isSafeId(value.planId) ||
    !isOneOf(value.operation, [
      "codex_provider_switch",
      "codex_provider_upsert_and_switch",
      "work_buddy_models_update",
    ] as const) ||
    !isSafeId(value.targetProviderId) ||
    typeof value.targetProviderName !== "string" ||
    value.targetProviderName.length === 0 ||
    value.targetProviderName.length > 80 ||
    !isSafeId(value.planDigest) ||
    !isSafeId(value.baselineDigest) ||
    !isRecord(value.actor) ||
    !hasExactKeys(value.actor, ["type"]) ||
    value.actor.type !== "direct_user" ||
    !isSafeId(value.sourceVersion) ||
    !isInteger(value.revision) ||
    !isInteger(value.createdAt) ||
    !isInteger(value.expiresAt) ||
    !isOneOf(value.status, ["ready", "consumed"] as const) ||
    !Array.isArray(value.businessSteps) ||
    typeof value.currentProviderCode !== "string" ||
    typeof value.targetProviderCode !== "string" ||
    !isOneOf(value.restartExpectation, RESTART_REQUIREMENTS) ||
    !Array.isArray(value.risks) ||
    value.evidenceNote !== "usage_not_observed"
  )
    throw new Error("Change Plan is unavailable");
  const expectedSteps =
    value.operation === "codex_provider_upsert_and_switch"
      ? ["save_provider", "set_current_provider"]
      : value.operation === "work_buddy_models_update"
        ? ["save_work_buddy_models"]
        : ["set_current_provider"];
  if (
    value.businessSteps.length !== expectedSteps.length ||
    !value.businessSteps.every(
      (step, index) => step === expectedSteps[index],
    ) ||
    (value.operation === "codex_provider_upsert_and_switch" &&
      (!isRecord(value.credential) ||
        !hasExactKeys(value.credential, ["secretRefDisplay", "backend"]) ||
        typeof value.credential.secretRefDisplay !== "string" ||
        !/^sec_…[0-9a-f]{4}$/.test(value.credential.secretRefDisplay) ||
        value.credential.backend !== "os_keyring")) ||
    (value.operation !== "codex_provider_upsert_and_switch" &&
      value.credential !== undefined)
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
  const adapter = parseAdapter(value.adapter);
  if (adapter.operationType !== value.operation)
    throw new Error("Change Plan is unavailable");
  return {
    planId: value.planId,
    operation: value.operation,
    targetProviderId: value.targetProviderId,
    targetProviderName: value.targetProviderName,
    planDigest: value.planDigest,
    baselineDigest: value.baselineDigest,
    actor: { type: "direct_user" },
    sourceVersion: value.sourceVersion,
    revision: value.revision,
    createdAt: value.createdAt,
    expiresAt: value.expiresAt,
    status: value.status,
    businessSteps: [...value.businessSteps],
    credential:
      value.operation === "codex_provider_upsert_and_switch"
        ? {
            secretRefDisplay: (value.credential as { secretRefDisplay: string })
              .secretRefDisplay,
            backend: "os_keyring",
          }
        : undefined,
    adapter,
    currentProviderCode: value.currentProviderCode,
    targetProviderCode: value.targetProviderCode,
    restartExpectation: value.restartExpectation,
    risks,
    evidenceNote: value.evidenceNote,
  };
}

function parseStep(value: unknown): ChangeJobStep {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "status", "code"]) ||
    !isOneOf(value.kind, CHANGE_STEP_KINDS) ||
    !isOneOf(value.status, STEP_STATUSES) ||
    typeof value.code !== "string"
  )
    throw new Error("Change job is unavailable");
  return { kind: value.kind, status: value.status, code: value.code };
}

function parseResource(value: unknown): ChangeResourceResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "status", "code"]) ||
    !isOneOf(value.kind, RESOURCE_KINDS) ||
    !isOneOf(value.status, RESOURCE_STATUSES) ||
    typeof value.code !== "string"
  )
    throw new Error("Change job is unavailable");
  return { kind: value.kind, status: value.status, code: value.code };
}

function parsePartial(value: unknown): ChangePartialResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "succeededSteps",
      "compensatedSteps",
      "unverifiedSteps",
      "remainingEffects",
      "manualActions",
    ])
  )
    throw new Error("Change job is unavailable");
  const parseKinds = (
    input: unknown,
  ): ChangePartialResult["succeededSteps"] => {
    if (
      !Array.isArray(input) ||
      !input.every((item) => isOneOf(item, CHANGE_STEP_KINDS))
    )
      throw new Error("Change job is unavailable");
    return [...input];
  };
  return {
    succeededSteps: parseKinds(value.succeededSteps),
    compensatedSteps: parseKinds(value.compensatedSteps),
    unverifiedSteps: parseKinds(value.unverifiedSteps),
    remainingEffects: parseStringList(
      value.remainingEffects,
      "Change job is unavailable",
    ),
    manualActions: parseStringList(
      value.manualActions,
      "Change job is unavailable",
    ),
  };
}

function parseJob(value: unknown): ChangeJobSnapshot {
  const required = [
    "jobId",
    "executionId",
    "idempotencyKey",
    "planId",
    "targetProviderId",
    "revision",
    "eventSeq",
    "status",
    "resultCode",
    "steps",
    "resources",
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
    !hasRequiredAndOptionalKeys(value, required, [
      "adapterErrorCode",
      "partialResult",
    ]) ||
    !isSafeId(value.jobId) ||
    value.executionId !== value.jobId ||
    !isSafeId(value.idempotencyKey) ||
    !isSafeId(value.planId) ||
    value.idempotencyKey !== value.planId ||
    !isSafeId(value.targetProviderId) ||
    !isInteger(value.revision) ||
    !isInteger(value.eventSeq) ||
    !isOneOf(value.status, JOB_STATUSES) ||
    !isOneOf(value.resultCode, RESULT_CODES) ||
    !Array.isArray(value.steps) ||
    value.steps.length !== CHANGE_STEP_KINDS.length ||
    !Array.isArray(value.resources) ||
    !isOneOf(value.restartRequirement, RESTART_REQUIREMENTS) ||
    value.usageEvidence !== "not_observed" ||
    !isOneOf(value.recoveryState, RECOVERY_STATES) ||
    (value.adapterErrorCode !== undefined &&
      !isOneOf(value.adapterErrorCode, ADAPTER_ERRORS)) ||
    (value.diagnosticCode !== null &&
      typeof value.diagnosticCode !== "string") ||
    typeof value.liveConfigChanged !== "boolean" ||
    !isInteger(value.createdAt) ||
    !isInteger(value.updatedAt)
  )
    throw new Error("Change job is unavailable");
  const steps = value.steps.map(parseStep);
  if (!steps.every((step, index) => step.kind === CHANGE_STEP_KINDS[index]))
    throw new Error("Change job is unavailable");
  const resources = value.resources.map(parseResource);
  return {
    jobId: value.jobId,
    executionId: value.executionId,
    idempotencyKey: value.idempotencyKey,
    planId: value.planId,
    targetProviderId: value.targetProviderId,
    revision: value.revision,
    eventSeq: value.eventSeq,
    status: value.status,
    resultCode: value.resultCode,
    steps,
    resources,
    restartRequirement: value.restartRequirement,
    usageEvidence: value.usageEvidence,
    recoveryState: value.recoveryState,
    ...(value.adapterErrorCode === undefined
      ? {}
      : { adapterErrorCode: value.adapterErrorCode }),
    ...(value.partialResult === undefined
      ? {}
      : { partialResult: parsePartial(value.partialResult) }),
    diagnosticCode: value.diagnosticCode,
    liveConfigChanged: value.liveConfigChanged,
    createdAt: value.createdAt,
    updatedAt: value.updatedAt,
  };
}

function parseApplyOutcome(value: unknown): ApplyChangePlanOutcome {
  if (!isRecord(value) || typeof value.kind !== "string")
    throw new Error("Change Plan apply result is unavailable");
  if (isOneOf(value.kind, ["admitted", "idempotent_replay"] as const)) {
    if (!hasExactKeys(value, ["kind", "job"]))
      throw new Error("Change Plan apply result is unavailable");
    return { kind: value.kind, job: parseJob(value.job) };
  }
  if (
    value.kind !== "rejected" ||
    !hasExactKeys(value, ["kind", "errorCode"]) ||
    !isOneOf(value.errorCode, PLAN_ERRORS)
  )
    throw new Error("Change Plan apply result is unavailable");
  return { kind: "rejected", errorCode: value.errorCode };
}

function parseCancelOutcome(value: unknown): CancelChangeJobOutcome {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["accepted", "code", "jobId"]) ||
    typeof value.accepted !== "boolean" ||
    !isOneOf(value.code, CANCEL_CODES) ||
    !isSafeId(value.jobId) ||
    value.accepted !== (value.code === "accepted")
  )
    throw new Error("Change job cancellation result is unavailable");
  return { accepted: value.accepted, code: value.code, jobId: value.jobId };
}

function parseUpdatedEvent(value: unknown): ChangeJobUpdatedEvent {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["jobId", "eventSeq"]) ||
    !isSafeId(value.jobId) ||
    !isInteger(value.eventSeq)
  )
    throw new Error("Change job update is unavailable");
  return { jobId: value.jobId, eventSeq: value.eventSeq };
}

function assertId(value: string): string {
  if (!isSafeId(value)) throw new Error("Change Plan request is invalid");
  return value;
}

function assertWorkBuddyPlanRequest(
  request: WorkBuddyChangePlanRequest,
): WorkBuddyChangePlanRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      [
        "baseUrl",
        "apiKey",
        "allowNoApiKey",
        "selectedModelIds",
        "manualModelIds",
        "clearExistingApiKeys",
        "expectedRevision",
      ],
      ["removedModelIds"],
    ) ||
    typeof request.baseUrl !== "string" ||
    typeof request.apiKey !== "string" ||
    typeof request.allowNoApiKey !== "boolean" ||
    !isStringArray(request.selectedModelIds) ||
    !isStringArray(request.manualModelIds) ||
    (request.removedModelIds !== undefined &&
      !isStringArray(request.removedModelIds)) ||
    typeof request.clearExistingApiKeys !== "boolean" ||
    (request.expectedRevision !== null &&
      typeof request.expectedRevision !== "string")
  )
    throw new Error("WorkBuddy Change Plan request is invalid");
  return {
    baseUrl: request.baseUrl,
    apiKey: request.apiKey,
    allowNoApiKey: request.allowNoApiKey,
    selectedModelIds: [...request.selectedModelIds],
    manualModelIds: [...request.manualModelIds],
    ...(request.removedModelIds === undefined
      ? {}
      : { removedModelIds: [...request.removedModelIds] }),
    clearExistingApiKeys: request.clearExistingApiKeys,
    expectedRevision: request.expectedRevision,
  };
}

export function createChangePlanPort(): FeaturePorts["changePlan"] {
  return {
    createCodexProviderSwitchPlan: async (targetProviderId) =>
      parsePlan(
        await invoke<unknown>("create_codex_provider_switch_plan", {
          targetProviderId: assertId(targetProviderId),
        }),
      ),
    createCodexProviderUpsertPlan: async (request) =>
      parsePlan(
        await invoke<unknown>("create_codex_provider_upsert_plan", {
          request: assertQuickSetupRequest(request),
        }),
      ),
    createWorkBuddyModelsPlan: async (request) =>
      parsePlan(
        await invoke<unknown>("create_workbuddy_models_plan", {
          request: assertWorkBuddyPlanRequest(request),
        }),
      ),
    apply: async (planId, planDigest) =>
      parseApplyOutcome(
        await invoke<unknown>("apply_change_plan", {
          planId: assertId(planId),
          planDigest: assertId(planDigest),
        }),
      ),
    getJob: async (jobId) =>
      parseJob(
        await invoke<unknown>("get_change_job", { jobId: assertId(jobId) }),
      ),
    listRecoverableJobs: async () => {
      const value = await invoke<unknown>("list_recoverable_change_jobs");
      if (!Array.isArray(value)) throw new Error("Change jobs are unavailable");
      return value.map(parseJob);
    },
    cancelJob: async (jobId) =>
      parseCancelOutcome(
        await invoke<unknown>("cancel_change_job", {
          jobId: assertId(jobId),
        }),
      ),
    subscribeJobUpdates: async (onEvent) =>
      listen<unknown>(JOB_UPDATED_EVENT, (event) => {
        try {
          onEvent(parseUpdatedEvent(event.payload));
        } catch {
          // Event payloads are hints only. A malformed hint is ignored and can
          // never construct renderer state or trigger a write.
        }
      }),
  };
}
