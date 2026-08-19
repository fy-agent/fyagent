import type {
  ApplyEffect,
  ApplyEvidence,
  ApplyJobStatus,
  ApplyRecoveryAction,
  ApplySnapshot,
  ApplyStepId,
  ApplyStepStatus,
} from "./fixtures";

export type {
  ApplyEffect,
  ApplyJobStatus,
  ApplySnapshot,
  ApplyStepStatus,
} from "./fixtures";

export type ApplyActionKind =
  | ApplyRecoveryAction
  | "complete_and_use"
  | "request_cancel"
  | "return_to_plan";

export type ApplyActionPresentation = {
  readonly kind: ApplyActionKind;
  readonly label: string;
  readonly primary: boolean;
};

export type ApplyStepPresentation = {
  readonly stepId: ApplyStepId;
  readonly label: string;
  readonly status: ApplyStepStatus;
  readonly statusLabel: string;
  readonly current: boolean;
};

export type ApplyPresentation = {
  readonly status: ApplyJobStatus;
  readonly statusLabel: string;
  readonly effect: ApplyEffect;
  readonly effectLabel: string;
  readonly title: string;
  readonly subtitle: string;
  readonly observedUsageCopy: string | null;
  readonly backupAvailable: boolean;
  readonly steps: readonly ApplyStepPresentation[];
  readonly evidence: readonly string[];
  readonly notices: readonly string[];
  readonly failureLabel: string | null;
  readonly actions: readonly ApplyActionPresentation[];
  readonly plan: ApplySnapshot["plan"];
};

const STEP_LABELS = {
  verify_plan: "核对计划",
  backup_resources: "备份资源",
  write_managed_config: "写入受管配置",
  readback_verify: "回读核对",
  refresh_local_state: "刷新本机状态",
} as const satisfies Record<ApplyStepId, string>;

const STATUS_LABELS = {
  not_started: "未开始",
  planned: "已就绪",
  running: "进行中",
  succeeded: "已完成",
  warning: "需留意",
  failed: "失败",
  cancelled: "已取消",
} as const satisfies Record<ApplyJobStatus, string>;

const EFFECT_LABELS = {
  none: "无变更",
  applied: "已应用",
  partial: "部分变更",
  unknown: "结果未确认",
} as const satisfies Record<ApplyEffect, string>;

const RECOVERY_LABELS = {
  retry_readback: "重试回读",
  restore_backup: "恢复备份",
  retry_refresh: "重试辅助刷新",
  regenerate_plan: "重新生成计划",
} as const satisfies Record<ApplyRecoveryAction, string>;

const EVIDENCE_COPY = {
  plan_verified: "已核对",
  backup_ready: "已备份",
  managed_write_completed: "已写入",
  readback_matched: "回读一致 ·",
  local_state_refreshed: "已刷新",
} as const satisfies Record<ApplyEvidence["kind"], string>;

const EVIDENCE_UNIT = {
  plan_verified: "项逻辑资源",
  backup_ready: "项资源",
  managed_write_completed: "项受管配置",
  readback_matched: "项资源",
  local_state_refreshed: "个本机组件",
} as const satisfies Record<ApplyEvidence["kind"], string>;

const OUTCOME_COPY = {
  running_before_write: ["正在应用配置", "正在核对计划并保护可恢复性"],
  running_after_write: ["正在确认配置结果", "配置写入已返回，正在进行本机回读"],
  succeeded: ["配置已应用，可直接开始使用", "已完成本机写入与回读核对"],
  warning: [
    "配置已应用，可直接开始使用",
    "核心配置已回读一致；仍有一项本机辅助状态待处理",
  ],
  failed_none: [
    "配置尚未应用",
    "写入前已安全停止；请按提示重新生成计划或修复条件",
  ],
  failed_partial: ["配置未能完整应用", "已检测到部分变更；请先查看恢复选项"],
  failed_unknown: [
    "无法确认配置结果",
    "不会自动重复写入；请先重试回读或恢复备份",
  ],
  cancelled: ["已取消应用", "尚未开始受管配置写入"],
  planned: ["准备应用配置", "任务已创建，尚未开始写入"],
  not_started: ["尚未开始应用", "确认计划后开始本机写入"],
} as const;

type OutcomeKey = keyof typeof OUTCOME_COPY;

export function assertNever(value: never): never {
  throw new Error(`Unhandled apply presentation value: ${String(value)}`);
}

function resolveOutcome(snapshot: ApplySnapshot): OutcomeKey {
  switch (snapshot.status) {
    case "running":
      return snapshot.effect === "unknown"
        ? "running_after_write"
        : "running_before_write";
    case "succeeded":
      return "succeeded";
    case "warning":
      return "warning";
    case "failed":
      switch (snapshot.effect) {
        case "none":
          return "failed_none";
        case "partial":
          return "failed_partial";
        case "unknown":
        case "applied":
          return "failed_unknown";
        default:
          return assertNever(snapshot.effect);
      }
    case "cancelled":
      return "cancelled";
    case "planned":
      return "planned";
    case "not_started":
      return "not_started";
    default:
      return assertNever(snapshot.status);
  }
}

function presentActions(
  snapshot: ApplySnapshot,
): readonly ApplyActionPresentation[] {
  switch (snapshot.status) {
    case "running":
    case "planned":
      return snapshot.canCancel
        ? [{ kind: "request_cancel", label: "请求取消", primary: false }]
        : [];
    case "succeeded":
      return [
        { kind: "complete_and_use", label: "完成并开始使用", primary: true },
      ];
    case "warning":
      return [
        { kind: "complete_and_use", label: "完成并开始使用", primary: true },
        ...snapshot.recoveryActions.map((kind) => ({
          kind,
          label: RECOVERY_LABELS[kind],
          primary: false,
        })),
      ];
    case "failed":
      return snapshot.recoveryActions.map((kind, index) => ({
        kind,
        label: RECOVERY_LABELS[kind],
        primary: index === 0,
      }));
    case "cancelled":
    case "not_started":
      return [{ kind: "return_to_plan", label: "返回计划", primary: true }];
    default:
      return assertNever(snapshot.status);
  }
}

export function createApplyViewModel(
  snapshot: ApplySnapshot,
): ApplyPresentation {
  const [title, subtitle] = OUTCOME_COPY[resolveOutcome(snapshot)];
  const runningStepId =
    snapshot.steps.find((step) => step.status === "running")?.stepId ?? null;

  return {
    status: snapshot.status,
    statusLabel: STATUS_LABELS[snapshot.status],
    effect: snapshot.effect,
    effectLabel: EFFECT_LABELS[snapshot.effect],
    title,
    subtitle,
    observedUsageCopy:
      snapshot.effect === "applied" && snapshot.observedUsage === "not_observed"
        ? "配置已应用，尚无真实使用证据"
        : null,
    backupAvailable: snapshot.backupAvailable,
    steps: snapshot.steps.map((item) => ({
      stepId: item.stepId,
      label: STEP_LABELS[item.stepId],
      status: item.status,
      statusLabel: STATUS_LABELS[item.status],
      current: item.stepId === runningStepId,
    })),
    evidence: snapshot.steps.flatMap((item) =>
      item.evidence.map(
        (entry) =>
          `${EVIDENCE_COPY[entry.kind]} ${String(entry.count)} ${EVIDENCE_UNIT[entry.kind]}`,
      ),
    ),
    notices: snapshot.notices.map(() => "本机索引刷新未完成"),
    failureLabel: snapshot.failure ? "回读不一致" : null,
    actions: presentActions(snapshot),
    plan: snapshot.plan,
  };
}
