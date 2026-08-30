import type {
  ChangeJobSnapshot,
  ChangePlan,
} from "../../../shared/features/change-plans";

export type ApplyTone =
  | "neutral"
  | "success"
  | "warning"
  | "danger"
  | "unknown";

export type ApplyViewMode =
  | "preview"
  | "running"
  | "succeeded"
  | "warning"
  | "failed"
  | "recovery"
  | "regenerate"
  | "blocked"
  | "unknown";

export type ApplyWorkspaceError = {
  readonly code: string;
  readonly message?: string;
};

export type ApplyStepPresentation = {
  readonly key: string;
  readonly label: string;
  readonly detail: string;
  readonly status: "pending" | "running" | "succeeded" | "failed" | "skipped";
  readonly current: boolean;
};

export type ApplyResourcePresentation = {
  readonly key: string;
  readonly label: string;
  readonly statusLabel: string;
  readonly tone: ApplyTone;
};

export type ApplyPreviewModel = {
  readonly semantic: {
    readonly summary: string;
    readonly currentCode: string;
    readonly targetCode: string;
    readonly targetName: string;
    readonly operationLabel: string;
    readonly planStatusLabel: string;
  };
  readonly risk: {
    readonly restartLabel: string;
    readonly items: ChangePlan["risks"];
    readonly empty: boolean;
  };
  readonly scope: {
    readonly readLabels: readonly string[];
    readonly writeLabels: readonly string[];
    readonly secretLabel: string;
    readonly dbBaselineLabel: string;
    readonly deviceBaselineLabel: string;
    readonly expiresLabel: string;
  };
  readonly recovery: {
    readonly evidenceLabel: string;
    readonly compensationLabel: string;
    readonly readbackLabel: string;
  };
};

export type ApplyPartialTruth = {
  readonly succeededCount: number;
  readonly compensatedCount: number;
  readonly unverifiedCount: number;
  readonly remainingEffects: readonly string[];
  readonly manualActions: readonly string[];
};

export type ApplyPresentation = {
  readonly mode: ApplyViewMode;
  readonly tone: ApplyTone;
  readonly title: string;
  readonly description: string;
  readonly statusLabel: string;
  readonly usageEvidenceCopy: string | null;
  readonly plan: ChangePlan | null;
  readonly preview: ApplyPreviewModel | null;
  readonly partialTruth: ApplyPartialTruth | null;
  readonly eventSeq: number | null;
  readonly steps: readonly ApplyStepPresentation[];
  readonly resources: readonly ApplyResourcePresentation[];
  readonly canConfirm: boolean;
  readonly canRegenerate: boolean;
  readonly confirmLabel: string;
};

type ViewOptions = {
  readonly busy: boolean;
  readonly error: ApplyWorkspaceError | null;
  readonly nowMs?: number;
};

const REGENERATE_ERRORS = new Set([
  "expired",
  "stale",
  "consumed",
  "invalid_digest",
  "plan_not_found",
]);

const STEP_LABELS = {
  precheck: "核对计划",
  snapshot: "冻结执行快照",
  managed_write: "应用配置",
  readback: "回读核对",
  finalize: "确认最终状态",
} as const satisfies Record<ChangeJobSnapshot["steps"][number]["kind"], string>;

const STEP_STATUS_LABELS = {
  pending: "等待中",
  running: "进行中",
  succeeded: "已完成",
  failed: "失败",
  compensating: "正在补偿",
  compensated: "已补偿",
  skipped: "已跳过",
} as const satisfies Record<
  ChangeJobSnapshot["steps"][number]["status"],
  string
>;

const RESOURCE_LABELS = {
  provider_db_current: "数据库当前 Provider",
  device_current: "设备当前 Provider",
  target_definition: "目标 Provider 定义",
  codex_live_projection: "Codex 实时配置投影",
  work_buddy_models_config: "WorkBuddy 模型配置",
  work_buddy_backup: "WorkBuddy 备份",
} as const satisfies Record<
  ChangeJobSnapshot["resources"][number]["kind"],
  string
>;

const MANUAL_ACTION_LABELS = {
  retry_readback: "重新回读",
  review_configuration: "检查配置",
} as const satisfies Record<
  NonNullable<ChangeJobSnapshot["partialResult"]>["manualActions"][number],
  string
>;

function assertNever(value: never): never {
  throw new Error(`Unhandled Apply state: ${String(value)}`);
}

function restartExpectationLabel(
  value: ChangePlan["restartExpectation"],
): string {
  switch (value) {
    case "recommended":
      return "建议重启 Codex";
    case "not_required":
      return "无需重启";
    case "unknown":
      return "尚未确认";
    default:
      return assertNever(value);
  }
}

function secretCapabilityLabel(value: ChangePlan["secretCapability"]): string {
  switch (value) {
    case "no_new_credential_material":
      return "不写入新的凭据材料";
    case "secret_dependency_unavailable":
      return "缺少可用凭据依赖，无法安全应用";
    default:
      return assertNever(value);
  }
}

function evidenceNoteLabel(note: string): string {
  return note === "usage_not_observed"
    ? "本次不把真实使用观察当作成功证据。"
    : "仅采用计划内证据说明，不把未观察的使用当成成功。";
}

function createPreviewModel(plan: ChangePlan): ApplyPreviewModel {
  const semantic =
    plan.operation === "workbuddy_models_save"
      ? {
          summary: `保存 WorkBuddy 模型配置 ${plan.targetProviderName}（${plan.targetProviderCode}）。`,
          operationLabel: "WorkBuddy 模型保存并应用",
        }
      : plan.operation === "codex_provider_upsert_and_switch"
        ? {
            summary: `保存 Codex Provider ${plan.targetProviderName}（${plan.targetProviderCode}）并设为当前配置。`,
            operationLabel: "Codex Provider 保存并设为当前",
          }
        : {
            summary: `将 Codex 当前 Provider ${plan.currentProviderCode} 切换到 ${plan.targetProviderName}（${plan.targetProviderCode}）。`,
            operationLabel: "Codex Provider 切换",
          };
  return {
    semantic: {
      summary: semantic.summary,
      currentCode: plan.currentProviderCode,
      targetCode: plan.targetProviderCode,
      targetName: plan.targetProviderName,
      operationLabel: semantic.operationLabel,
      planStatusLabel: plan.status === "ready" ? "可确认" : "不可再次使用",
    },
    risk: {
      restartLabel: restartExpectationLabel(plan.restartExpectation),
      items: plan.risks,
      empty: plan.risks.length === 0,
    },
    scope: {
      readLabels: plan.adapter.readSet.map((kind) => RESOURCE_LABELS[kind]),
      writeLabels: plan.adapter.writeSet.map((kind) => RESOURCE_LABELS[kind]),
      secretLabel: secretCapabilityLabel(plan.secretCapability),
      dbBaselineLabel: plan.dbBaselineProviderId ?? "无数据库基线",
      deviceBaselineLabel: plan.deviceBaselineProviderId ?? "无设备基线",
      expiresLabel: new Date(plan.expiresAt * 1000).toISOString(),
    },
    recovery: {
      evidenceLabel: evidenceNoteLabel(plan.evidenceNote),
      compensationLabel: "失败时由写入方回滚到原基线，而不是另一套撤销引擎。",
      readbackLabel: "中断后只做只读回读，不重放写入。",
    },
  };
}

function projectPartialTruth(
  partial: ChangeJobSnapshot["partialResult"],
): ApplyPartialTruth | null {
  if (!partial) return null;
  return {
    succeededCount: partial.succeededSteps.length,
    compensatedCount: partial.compensatedSteps.length,
    unverifiedCount: partial.unverifiedSteps.length,
    remainingEffects: partial.remainingEffects.map(
      (kind) => RESOURCE_LABELS[kind],
    ),
    manualActions: partial.manualActions.map(
      (action) => MANUAL_ACTION_LABELS[action],
    ),
  };
}

function stepStatus(
  status: ChangeJobSnapshot["steps"][number]["status"],
): ApplyStepPresentation["status"] {
  switch (status) {
    case "pending":
    case "running":
    case "compensating":
      return "running";
    case "succeeded":
    case "compensated":
      return "succeeded";
    case "failed":
    case "skipped":
      return status;
    default:
      return assertNever(status);
  }
}

function resourcePresentation(
  resource: ChangeJobSnapshot["resources"][number],
  index: number,
): ApplyResourcePresentation {
  switch (resource.status) {
    case "matched":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "回读一致",
        tone: "success",
      };
    case "pending":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "等待回读",
        tone: "neutral",
      };
    case "mismatched":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "回读不一致",
        tone: "danger",
      };
    case "unavailable":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "状态无法确认",
        tone: "unknown",
      };
    default:
      return assertNever(resource.status);
  }
}

export function hasUnconfirmedAuthority(job: ChangeJobSnapshot): boolean {
  if (
    job.recoveryState === "succeeded" &&
    (job.resultCode === "writer_failed_baseline_restored" ||
      job.resultCode === "interrupted_before_write")
  ) {
    return false;
  }
  if (job.recoveryState === "recovery_required") {
    return true;
  }
  return job.resources.some(
    (resource) =>
      resource.status === "mismatched" || resource.status === "unavailable",
  );
}

function terminalPresentation(
  job: ChangeJobSnapshot,
): Pick<
  ApplyPresentation,
  "mode" | "tone" | "title" | "description" | "statusLabel"
> {
  if (hasUnconfirmedAuthority(job)) {
    return {
      mode: "recovery",
      tone: "unknown",
      title: "配置结果需要人工确认",
      description: "回读无法建立单一可信状态。系统不会自动重复写入。",
      statusLabel: "需要恢复确认",
    };
  }

  switch (job.status) {
    case "planned":
      return {
        mode: "running",
        tone: "neutral",
        title: "应用任务已创建",
        description: "正在等待真实任务开始执行。",
        statusLabel: "已计划",
      };
    case "running":
      return {
        mode: "running",
        tone: "neutral",
        title: "正在应用配置",
        description: "以下进度来自真实 Change Job 事件与回读。",
        statusLabel: "进行中",
      };
    case "succeeded":
      switch (job.resultCode) {
        case "applied":
          return {
            mode: "succeeded",
            tone: "success",
            title: "配置已应用",
            description: "本机配置写入与回读已完成。",
            statusLabel: "已完成",
          };
        case "applied_restart_recommended":
        case "applied_with_warning":
          return {
            mode: "warning",
            tone: "warning",
            title: "配置已应用，但仍需留意",
            description: "请按提示完成后续操作，再验证真实使用情况。",
            statusLabel: "需留意",
          };
        case "recovered_target_reached":
        case "cancelled_before_write":
        case "interrupted_before_write":
        case "planned":
        case "running":
        case "writer_failed_baseline_restored":
        case "writer_error_target_reached":
        case "post_write_mismatch":
        case "readback_unavailable":
        case "recovery_required":
          return {
            mode: "unknown",
            tone: "unknown",
            title: "配置结果无法确认",
            description: "任务状态与结果不一致，不能视为成功。",
            statusLabel: "状态未知",
          };
        default:
          return assertNever(job.resultCode);
      }
    case "warning":
      if (job.resultCode === "recovered_target_reached") {
        return {
          mode: "warning",
          tone: "warning",
          title: "配置已通过恢复回读确认",
          description: "进程中断后已从真实目标确认配置生效；系统没有重放写入。",
          statusLabel: "恢复后已确认",
        };
      }
      return {
        mode: "warning",
        tone: "warning",
        title: "配置已应用，但仍需留意",
        description: "存在明确警告，请完成提示的后续检查。",
        statusLabel: "需留意",
      };
    case "failed":
      if (job.resultCode === "interrupted_before_write") {
        return {
          mode: "failed",
          tone: "danger",
          title: "执行在写入前中断",
          description: "回读已确认未进入托管写入，系统没有重放配置写入。",
          statusLabel: "未执行写入",
        };
      }
      return {
        mode:
          job.resultCode === "recovery_required" ||
          job.resultCode === "writer_error_target_reached" ||
          job.resultCode === "recovered_target_reached" ||
          job.resultCode === "post_write_mismatch" ||
          job.resultCode === "readback_unavailable"
            ? "recovery"
            : "failed",
        tone:
          job.resultCode === "writer_failed_baseline_restored"
            ? "danger"
            : "unknown",
        title:
          job.resultCode === "writer_failed_baseline_restored"
            ? "配置应用失败，原基线已确认"
            : "配置结果需要人工确认",
        description:
          job.resultCode === "writer_failed_baseline_restored"
            ? "写入失败，回读已确认仍处于原基线。"
            : "当前权威状态不明确，系统不会自动重复写入。",
        statusLabel:
          job.resultCode === "writer_failed_baseline_restored"
            ? "失败"
            : "需要恢复确认",
      };
    case "cancelled":
      return {
        mode: "failed",
        tone: "neutral",
        title: "执行已取消",
        description: "取消发生在托管写入提交点之前，没有执行配置写入。",
        statusLabel: "写入前已取消",
      };
    default:
      return assertNever(job.status);
  }
}

export function createApplyViewModel(
  plan: ChangePlan | null,
  job: ChangeJobSnapshot | null,
  { busy, error, nowMs = Date.now() }: ViewOptions,
): ApplyPresentation {
  const expired = plan ? plan.expiresAt * 1000 <= nowMs : false;
  const mustRegenerate =
    expired ||
    plan?.status === "consumed" ||
    (error ? REGENERATE_ERRORS.has(error.code) : false);
  const secretBlocked = error?.code === "secret_dependency_unavailable";

  const base = job
    ? terminalPresentation(job)
    : secretBlocked
      ? {
          mode: "blocked" as const,
          tone: "warning" as const,
          title: "无法安全生成变更计划",
          description:
            "目标还缺可用凭据。如果这是 SuperGrok，请先去认证中心扫码，再生成切换计划。Apply 不会接收或写入钥匙。",
          statusLabel: "凭据条件不满足",
        }
      : mustRegenerate
        ? {
            mode: "regenerate" as const,
            tone: "warning" as const,
            title: "计划已失效",
            description: "请重新生成计划；当前计划不会被应用。",
            statusLabel: "需要重新生成",
          }
        : error
          ? {
              mode: "unknown" as const,
              tone: "unknown" as const,
              title: "无法确认 Apply 状态",
              description: error.message ?? "发生受控错误，未建立成功证据。",
              statusLabel: "状态未知",
            }
          : {
              mode: "preview" as const,
              tone: "neutral" as const,
              title: "确认配置变更",
              description:
                "预览仅用于核对；点击确认后才会提交真实 Change Plan。",
              statusLabel: "待确认",
            };

  const steps = (job?.steps ?? []).map((step, index) => ({
    key: `${step.kind}-${String(index)}`,
    label: STEP_LABELS[step.kind],
    detail: STEP_STATUS_LABELS[step.status],
    status: stepStatus(step.status),
    current: step.status === "running" || step.status === "compensating",
  }));

  const resources = (job?.resources ?? []).map(resourcePresentation);
  const canConfirm =
    !!plan &&
    !job &&
    !busy &&
    !error &&
    !expired &&
    plan.status === "ready" &&
    plan.secretCapability === "no_new_credential_material";

  return {
    ...base,
    usageEvidenceCopy:
      job &&
      (job.status === "succeeded" || job.status === "warning") &&
      job.usageEvidence === "not_observed"
        ? "配置结果已记录，尚无真实使用证据。"
        : null,
    plan,
    preview: plan ? createPreviewModel(plan) : null,
    partialTruth: projectPartialTruth(job?.partialResult ?? null),
    eventSeq: job?.eventSeq ?? null,
    steps,
    resources,
    canConfirm,
    canRegenerate: !job && (mustRegenerate || secretBlocked),
    confirmLabel: busy ? "正在提交…" : "确认应用",
  };
}
