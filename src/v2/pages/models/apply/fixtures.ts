export const APPLY_STEP_IDS = [
  "verify_plan",
  "backup_resources",
  "write_managed_config",
  "readback_verify",
  "refresh_local_state",
] as const;

export type ApplyStepId = (typeof APPLY_STEP_IDS)[number];
export type ApplyJobStatus =
  | "not_started"
  | "planned"
  | "running"
  | "succeeded"
  | "warning"
  | "failed"
  | "cancelled";
export type ApplyStepStatus = ApplyJobStatus;
export type ApplyEffect = "none" | "applied" | "partial" | "unknown";
export type ApplyRecoveryAction =
  | "retry_readback"
  | "restore_backup"
  | "retry_refresh"
  | "regenerate_plan";
export type ObservedUsageState = "not_observed" | "observed";

export type ApplyEvidence = {
  readonly kind:
    | "plan_verified"
    | "backup_ready"
    | "managed_write_completed"
    | "readback_matched"
    | "local_state_refreshed";
  readonly count: number;
};

export type ApplyStepSnapshot = {
  readonly stepId: ApplyStepId;
  readonly status: ApplyStepStatus;
  readonly blocking: boolean;
  readonly evidence: readonly ApplyEvidence[];
};

export type ApplySnapshot = {
  readonly jobId: string;
  readonly status: ApplyJobStatus;
  readonly effect: ApplyEffect;
  readonly canCancel: boolean;
  readonly cancelRequested: boolean;
  readonly backupAvailable: boolean;
  readonly observedUsage: ObservedUsageState;
  readonly steps: readonly ApplyStepSnapshot[];
  readonly notices: readonly {
    readonly code: "local_index_refresh_failed";
    readonly stepId: ApplyStepId;
  }[];
  readonly failure: {
    readonly code: "readback_mismatch";
    readonly stepId: ApplyStepId;
    readonly retryable: boolean;
  } | null;
  readonly recoveryActions: readonly ApplyRecoveryAction[];
  readonly plan: {
    readonly actionLabel: string;
    readonly resourceCount: number;
    readonly baselineValid: boolean;
  };
};

export const APPLY_SCENARIOS = [
  "running",
  "succeeded",
  "warning",
  "failed",
  "cancelled",
] as const;

export type ApplyScenario = (typeof APPLY_SCENARIOS)[number];

const CODEX_PLAN = {
  actionLabel: "Codex Provider 切换",
  resourceCount: 2,
  baselineValid: true,
} as const;

function evidence(
  kind: ApplyEvidence["kind"],
  count: number,
): readonly ApplyEvidence[] {
  return [{ kind, count }];
}

function step(
  index: 0 | 1 | 2 | 3 | 4,
  status: ApplyStepStatus,
  items: readonly ApplyEvidence[] = [],
): ApplyStepSnapshot {
  const stepId = APPLY_STEP_IDS[index];
  return {
    stepId,
    status,
    blocking: stepId !== "refresh_local_state",
    evidence: items,
  };
}

const running: ApplySnapshot = {
  jobId: "job-running",
  status: "running",
  effect: "none",
  canCancel: true,
  cancelRequested: false,
  backupAvailable: false,
  observedUsage: "not_observed",
  notices: [],
  failure: null,
  recoveryActions: [],
  plan: CODEX_PLAN,
  steps: [
    step(0, "succeeded", evidence("plan_verified", 2)),
    step(1, "running"),
    step(2, "not_started"),
    step(3, "not_started"),
    step(4, "not_started"),
  ],
};

const succeeded: ApplySnapshot = {
  jobId: "job-succeeded",
  status: "succeeded",
  effect: "applied",
  canCancel: false,
  cancelRequested: false,
  backupAvailable: true,
  observedUsage: "not_observed",
  notices: [],
  failure: null,
  recoveryActions: [],
  plan: CODEX_PLAN,
  steps: [
    step(0, "succeeded", evidence("plan_verified", 2)),
    step(1, "succeeded", evidence("backup_ready", 2)),
    step(2, "succeeded", evidence("managed_write_completed", 2)),
    step(3, "succeeded", evidence("readback_matched", 2)),
    step(4, "succeeded", evidence("local_state_refreshed", 3)),
  ],
};

const warning: ApplySnapshot = {
  jobId: "job-warning",
  status: "warning",
  effect: "applied",
  canCancel: false,
  cancelRequested: false,
  backupAvailable: true,
  observedUsage: "not_observed",
  notices: [{ code: "local_index_refresh_failed", stepId: "refresh_local_state" }],
  failure: null,
  recoveryActions: ["retry_refresh"],
  plan: CODEX_PLAN,
  steps: [
    step(0, "succeeded", evidence("plan_verified", 2)),
    step(1, "succeeded", evidence("backup_ready", 2)),
    step(2, "succeeded", evidence("managed_write_completed", 2)),
    step(3, "succeeded", evidence("readback_matched", 2)),
    step(4, "warning"),
  ],
};

const failed: ApplySnapshot = {
  jobId: "job-failed",
  status: "failed",
  effect: "unknown",
  canCancel: false,
  cancelRequested: false,
  backupAvailable: true,
  observedUsage: "not_observed",
  notices: [],
  failure: {
    code: "readback_mismatch",
    stepId: "readback_verify",
    retryable: true,
  },
  recoveryActions: ["retry_readback", "restore_backup"],
  plan: CODEX_PLAN,
  steps: [
    step(0, "succeeded", evidence("plan_verified", 2)),
    step(1, "succeeded", evidence("backup_ready", 2)),
    step(2, "succeeded", evidence("managed_write_completed", 2)),
    step(3, "failed"),
    step(4, "not_started"),
  ],
};

const cancelled: ApplySnapshot = {
  jobId: "job-cancelled",
  status: "cancelled",
  effect: "none",
  canCancel: false,
  cancelRequested: true,
  backupAvailable: false,
  observedUsage: "not_observed",
  notices: [],
  failure: null,
  recoveryActions: [],
  plan: CODEX_PLAN,
  steps: [
    step(0, "succeeded", evidence("plan_verified", 2)),
    step(1, "cancelled"),
    step(2, "not_started"),
    step(3, "not_started"),
    step(4, "not_started"),
  ],
};

export const applyFixtures = {
  running,
  succeeded,
  warning,
  failed,
  cancelled,
} as const satisfies Record<ApplyScenario, ApplySnapshot>;
