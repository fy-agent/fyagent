import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

const withUnknown = <T extends [string, ...string[]]>(values: T) =>
  z.enum(values).or(z.string().transform(() => "unknown" as const));

export const changeOperationSchema = withUnknown(["codex_provider_switch"]);
export const changePlanStatusSchema = withUnknown(["ready", "consumed"]);
export const restartRequirementSchema = withUnknown([
  "not_required",
  "recommended",
  "unknown",
]);
export const changeJobStatusSchema = withUnknown([
  "planned",
  "running",
  "succeeded",
  "warning",
  "failed",
]);
export const changeResultCodeSchema = withUnknown([
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
]);
export const recoveryStateSchema = withUnknown([
  "not_needed",
  "succeeded",
  "recovery_required",
]);
export const changePlanErrorCodeSchema = withUnknown([
  "unsupported_operation",
  "target_not_found",
  "target_already_current",
  "invalid_digest",
  "expired",
  "consumed",
  "stale",
  "plan_not_found",
  "job_not_found",
  "internal",
]);

export const changePlanSchema = z.object({
  planId: z.string().min(1),
  operation: changeOperationSchema,
  targetProviderId: z.string().min(1),
  targetProviderName: z.string(),
  planDigest: z.string().min(1),
  baselineDigest: z.string().min(1),
  createdAt: z.number().int(),
  expiresAt: z.number().int(),
  status: changePlanStatusSchema,
  currentProviderCode: z.string(),
  targetProviderCode: z.string(),
  restartExpectation: restartRequirementSchema,
  risks: z.array(
    z.object({
      code: z.string(),
      severity: z.string(),
    }),
  ),
  evidenceNote: z.string(),
});

const changeJobStepSchema = z.object({
  kind: withUnknown(["precheck", "apply", "readback", "reconcile"]),
  status: withUnknown(["pending", "running", "succeeded", "failed", "skipped"]),
  code: z.string(),
});

const changeResourceResultSchema = z.object({
  kind: withUnknown([
    "provider_db_current",
    "device_current",
    "target_definition",
    "codex_live_projection",
  ]),
  status: withUnknown(["pending", "matched", "mismatched", "unavailable"]),
  code: z.string(),
});

export const changeJobSnapshotSchema = z.object({
  jobId: z.string().min(1),
  planId: z.string().min(1),
  targetProviderId: z.string().min(1),
  revision: z.number().int().nonnegative(),
  eventSeq: z.number().int().nonnegative(),
  status: changeJobStatusSchema,
  resultCode: changeResultCodeSchema,
  steps: z.array(changeJobStepSchema),
  resources: z.array(changeResourceResultSchema),
  restartRequirement: restartRequirementSchema,
  usageEvidence: withUnknown(["not_observed"]),
  recoveryState: recoveryStateSchema,
  diagnosticCode: z.string().nullable().optional(),
  liveConfigChanged: z.boolean(),
  createdAt: z.number().int(),
  updatedAt: z.number().int(),
});

export const applyChangePlanOutcomeSchema = z.object({
  kind: withUnknown(["admitted", "rejected"]),
  job: changeJobSnapshotSchema.optional(),
  errorCode: changePlanErrorCodeSchema.optional(),
});

export const changeJobUpdatedEventSchema = z.object({
  jobId: z.string().min(1),
  eventSeq: z.number().int().nonnegative(),
});

export type ChangePlan = z.infer<typeof changePlanSchema>;
export type ChangeJobSnapshot = z.infer<typeof changeJobSnapshotSchema>;
export type ApplyChangePlanOutcome = z.infer<
  typeof applyChangePlanOutcomeSchema
>;
export type ChangeJobUpdatedEvent = z.infer<typeof changeJobUpdatedEventSchema>;

export const changePlanApi = {
  async createCodexProviderSwitchPlan(
    targetProviderId: string,
  ): Promise<ChangePlan> {
    return changePlanSchema.parse(
      await invoke("create_codex_provider_switch_plan", { targetProviderId }),
    );
  },

  async apply(
    planId: string,
    planDigest: string,
  ): Promise<ApplyChangePlanOutcome> {
    return applyChangePlanOutcomeSchema.parse(
      await invoke("apply_change_plan", { planId, planDigest }),
    );
  },

  async getJob(jobId: string): Promise<ChangeJobSnapshot> {
    return changeJobSnapshotSchema.parse(
      await invoke("get_change_job", { jobId }),
    );
  },

  async listRecoverableJobs(): Promise<ChangeJobSnapshot[]> {
    return z
      .array(changeJobSnapshotSchema)
      .parse(await invoke("list_recoverable_change_jobs"));
  },
};
