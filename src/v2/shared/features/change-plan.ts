export const CHANGE_STEP_KINDS = [
  "precheck",
  "snapshot",
  "managed_write",
  "readback",
  "finalize",
] as const;

export type ChangeStepKind = (typeof CHANGE_STEP_KINDS)[number];
export type ChangeOperation =
  | "codex_provider_switch"
  | "codex_provider_upsert_and_switch";
export type ChangeBusinessStepKind = "save_provider" | "set_current_provider";
export type ChangePlanStatus = "ready" | "consumed";
export type ChangeJobStatus =
  | "planned"
  | "running"
  | "succeeded"
  | "warning"
  | "failed"
  | "cancelled";
export type ChangeStepStatus =
  | "not_started"
  | "running"
  | "succeeded"
  | "failed"
  | "compensating"
  | "compensated"
  | "skipped";
export type ChangeResourceKind =
  | "provider_db_current"
  | "device_current"
  | "target_definition"
  | "codex_live_projection";
export type ChangeResourceStatus =
  | "pending"
  | "matched"
  | "mismatched"
  | "unavailable";
export type RestartRequirement = "not_required" | "recommended" | "unknown";
export type ChangeRecoveryState =
  | "not_needed"
  | "succeeded"
  | "recovery_required";
export type ChangeAdapterErrorCode =
  | "precondition_failed"
  | "transient"
  | "permanent"
  | "unknown_outcome"
  | "verify_failed"
  | "compensation_failed"
  | "unsupported";
export type ChangeResultCode =
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
export type ChangePlanErrorCode =
  | "unsupported_operation"
  | "invalid_request"
  | "target_not_found"
  | "target_already_current"
  | "baseline_unavailable"
  | "invalid_digest"
  | "expired"
  | "consumed"
  | "stale"
  | "plan_not_found"
  | "job_not_found"
  | "internal";

export type ChangeAdapterDescriptor = {
  adapterId: string;
  adapterVersion: string;
  operationType: ChangeOperation;
  phases: ChangeStepKind[];
  readSet: ChangeResourceKind[];
  writeSet: ChangeResourceKind[];
  idempotencyScope: "plan";
  cancelMode: "before_managed_write";
  compensationMode: "writer_owned_rollback";
  faultPoints: Array<
    "before_managed_write" | "after_managed_write_before_record"
  >;
};

export type ChangePlan = {
  planId: string;
  operation: ChangeOperation;
  targetProviderId: string;
  targetProviderName: string;
  planDigest: string;
  baselineDigest: string;
  actor: { type: "direct_user" };
  sourceVersion: string;
  revision: number;
  createdAt: number;
  expiresAt: number;
  status: ChangePlanStatus;
  businessSteps: ChangeBusinessStepKind[];
  credential?: {
    secretRefDisplay: string;
    backend: "os_keyring";
  };
  adapter: ChangeAdapterDescriptor;
  currentProviderCode: string;
  targetProviderCode: string;
  restartExpectation: RestartRequirement;
  risks: Array<{ code: string; severity: string }>;
  evidenceNote: "usage_not_observed";
};

export type ChangeJobStep = {
  kind: ChangeStepKind;
  status: ChangeStepStatus;
  code: string;
};

export type ChangeResourceResult = {
  kind: ChangeResourceKind;
  status: ChangeResourceStatus;
  code: string;
};

export type ChangePartialResult = {
  succeededSteps: ChangeStepKind[];
  compensatedSteps: ChangeStepKind[];
  unverifiedSteps: ChangeStepKind[];
  remainingEffects: string[];
  manualActions: string[];
};

export type ChangeJobSnapshot = {
  jobId: string;
  executionId: string;
  idempotencyKey: string;
  planId: string;
  targetProviderId: string;
  revision: number;
  eventSeq: number;
  status: ChangeJobStatus;
  resultCode: ChangeResultCode;
  steps: ChangeJobStep[];
  resources: ChangeResourceResult[];
  restartRequirement: RestartRequirement;
  usageEvidence: "not_observed";
  recoveryState: ChangeRecoveryState;
  adapterErrorCode?: ChangeAdapterErrorCode;
  partialResult?: ChangePartialResult;
  diagnosticCode: string | null;
  liveConfigChanged: boolean;
  createdAt: number;
  updatedAt: number;
};

export type ApplyChangePlanOutcome =
  | {
      kind: "admitted" | "idempotent_replay";
      job: ChangeJobSnapshot;
      errorCode?: never;
    }
  | {
      kind: "rejected";
      errorCode: ChangePlanErrorCode;
      job?: never;
    };

export type ChangeJobUpdatedEvent = {
  jobId: string;
  eventSeq: number;
};

export type CancelChangeJobOutcome = {
  accepted: boolean;
  code:
    | "accepted"
    | "commit_point_passed"
    | "already_terminal"
    | "not_active"
    | "job_not_found";
  jobId: string;
};

export function isTerminalChangeJob(status: ChangeJobStatus): boolean {
  return ["succeeded", "warning", "failed", "cancelled"].includes(status);
}
