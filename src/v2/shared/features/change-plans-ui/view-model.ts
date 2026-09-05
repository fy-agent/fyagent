import type { ChangeJobSnapshot, ChangePlan } from "../change-plans";

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

export type ApplyRiskPresentation = {
  readonly key: string;
  readonly label: string;
  readonly levelLabel: string;
};

export type ApplyPreviewModel = {
  readonly semantic: {
    readonly summary: string;
    readonly targetName: string;
    readonly operationLabel: string;
    readonly confirmationLabel: string;
  };
  readonly risk: {
    readonly restartLabel: string;
    readonly items: readonly ApplyRiskPresentation[];
    readonly empty: boolean;
  };
  readonly scope: {
    readonly readLabels: readonly string[];
    readonly writeLabels: readonly string[];
    readonly secretLabel: string;
    readonly expiresLabel: string;
  };
  readonly recovery: {
    readonly rollbackLabel: string;
    readonly interruptionLabel: string;
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
  precheck: "检查当前配置",
  snapshot: "准备恢复信息",
  managed_write: "写入配置",
  readback: "确认保存结果",
  finalize: "完成",
} as const satisfies Record<ChangeJobSnapshot["steps"][number]["kind"], string>;

const STEP_STATUS_LABELS = {
  pending: "等待中",
  running: "进行中",
  succeeded: "已完成",
  failed: "失败",
  compensating: "正在恢复原设置",
  compensated: "已恢复原设置",
  skipped: "已跳过",
} as const satisfies Record<
  ChangeJobSnapshot["steps"][number]["status"],
  string
>;

const RESOURCE_LABELS = {
  provider_db_current: "FyAgent 当前 Provider",
  device_current: "Codex 当前 Provider",
  target_definition: "目标 Provider 设置",
  codex_live_projection: "Codex 配置文件",
  work_buddy_models_config: "WorkBuddy 模型设置",
  work_buddy_backup: "WorkBuddy 配置备份",
} as const satisfies Record<
  ChangeJobSnapshot["resources"][number]["kind"],
  string
>;

const MANUAL_ACTION_LABELS = {
  retry_readback: "重新检查当前配置",
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
      return "不会新增或更改登录凭据";
    case "secret_dependency_unavailable":
      return "缺少可用凭据，请先返回补充";
    default:
      return assertNever(value);
  }
}

function riskLabel(code: string): string {
  switch (code) {
    case "local_configuration_write":
      return "将修改本机配置文件";
    case "save_provider_then_set_current":
      return "保存后会设为当前 Provider";
    case "existing_model_ids_will_be_updated":
      return "同名模型会更新现有设置";
    default:
      return "存在需要留意的配置变化";
  }
}

function riskLevelLabel(severity: string): string {
  switch (severity) {
    case "warning":
      return "注意";
    case "danger":
    case "error":
      return "高风险";
    default:
      return "提示";
  }
}

function previewValidityLabel(plan: ChangePlan): string {
  const seconds = Math.max(0, plan.expiresAt - plan.createdAt);
  const minutes = Math.max(1, Math.ceil(seconds / 60));
  return `${String(minutes)} 分钟内有效`;
}

function createPreviewModel(plan: ChangePlan): ApplyPreviewModel {
  const semantic =
    plan.operation === "workbuddy_models_save"
      ? {
          summary: `保存 WorkBuddy 模型设置，服务地址为 ${plan.targetProviderName}。`,
          operationLabel: "保存 WorkBuddy 模型设置",
        }
      : plan.operation === "codex_provider_upsert_and_switch"
        ? {
            summary: `保存 ${plan.targetProviderName} 并设为 Codex 当前 Provider。`,
            operationLabel: "保存并启用 Codex Provider",
          }
        : {
            summary: `将 Codex 当前 Provider 切换为 ${plan.targetProviderName}。`,
            operationLabel: "切换 Codex Provider",
          };
  return {
    semantic: {
      summary: semantic.summary,
      targetName: plan.targetProviderName,
      operationLabel: semantic.operationLabel,
      confirmationLabel:
        plan.status === "ready" ? "等待确认" : "请重新生成预览",
    },
    risk: {
      restartLabel: restartExpectationLabel(plan.restartExpectation),
      items: plan.risks.map((risk, index) => ({
        key: `${risk.code}-${String(index)}`,
        label: riskLabel(risk.code),
        levelLabel: riskLevelLabel(risk.severity),
      })),
      empty: plan.risks.length === 0,
    },
    scope: {
      readLabels: plan.adapter.readSet.map((kind) => RESOURCE_LABELS[kind]),
      writeLabels: plan.adapter.writeSet.map((kind) => RESOURCE_LABELS[kind]),
      secretLabel: secretCapabilityLabel(plan.secretCapability),
      expiresLabel: previewValidityLabel(plan),
    },
    recovery: {
      rollbackLabel: "保存失败时会恢复修改前的设置。",
      interruptionLabel:
        "如果操作中断，FyAgent 只检查当前设置，不会自动再次修改。",
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
        statusLabel: "已确认",
        tone: "success",
      };
    case "pending":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "等待确认",
        tone: "neutral",
      };
    case "mismatched":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "与预期不一致",
        tone: "danger",
      };
    case "unavailable":
      return {
        key: `${resource.kind}-${String(index)}`,
        label: RESOURCE_LABELS[resource.kind],
        statusLabel: "无法确认",
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
      title: "无法确认配置结果",
      description:
        "当前设置与预期不一致。为避免覆盖已有配置，FyAgent 已停止继续修改。请重新打开页面并检查当前设置。",
      statusLabel: "需要检查",
    };
  }

  switch (job.status) {
    case "planned":
      return {
        mode: "running",
        tone: "neutral",
        title: "准备应用配置",
        description: "即将开始修改本机设置。",
        statusLabel: "等待开始",
      };
    case "running":
      return {
        mode: "running",
        tone: "neutral",
        title: "正在应用配置",
        description: "请勿关闭 FyAgent，完成后会显示结果。",
        statusLabel: "进行中",
      };
    case "succeeded":
      switch (job.resultCode) {
        case "applied":
          return {
            mode: "succeeded",
            tone: "success",
            title: "配置已应用",
            description: "设置已保存，并已确认当前配置。",
            statusLabel: "已完成",
          };
        case "applied_restart_recommended":
        case "applied_with_warning":
          return {
            mode: "warning",
            tone: "warning",
            title: "配置已应用，请完成后续操作",
            description: "请按提示重启或重新打开目标应用。",
            statusLabel: "需要操作",
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
            description: "请重新打开页面并检查当前配置。",
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
          title: "配置已生效",
          description: "操作中断后已确认目标设置。请重新打开目标应用检查。",
          statusLabel: "已确认",
        };
      }
      return {
        mode: "warning",
        tone: "warning",
        title: "配置已应用，请完成后续操作",
        description: "请按页面提示检查当前设置。",
        statusLabel: "需要操作",
      };
    case "failed":
      if (job.resultCode === "interrupted_before_write") {
        return {
          mode: "failed",
          tone: "danger",
          title: "未修改配置",
          description: "操作在保存前中断，原设置未变。",
          statusLabel: "未保存",
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
            ? "配置未应用"
            : "无法确认配置结果",
        description:
          job.resultCode === "writer_failed_baseline_restored"
            ? "保存失败，已恢复修改前的设置。"
            : "为避免覆盖现有设置，FyAgent 已停止继续修改。请重新打开页面并检查。",
        statusLabel:
          job.resultCode === "writer_failed_baseline_restored"
            ? "失败"
            : "需要检查",
      };
    case "cancelled":
      return {
        mode: "failed",
        tone: "neutral",
        title: "已取消",
        description: "配置尚未写入。",
        statusLabel: "未保存",
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
          title: "缺少可用凭据",
          description: "请返回连接设置补充 API Key 或登录信息，然后重试。",
          statusLabel: "需要补充凭据",
        }
      : mustRegenerate
        ? {
            mode: "regenerate" as const,
            tone: "warning" as const,
            title: "预览已过期",
            description: "请重新生成预览后再确认。",
            statusLabel: "需要更新预览",
          }
        : error
          ? {
              mode: "unknown" as const,
              tone: "unknown" as const,
              title: "暂时无法继续",
              description: "请刷新页面后重试。",
              statusLabel: "暂时不可用",
            }
          : {
              mode: "preview" as const,
              tone: "neutral" as const,
              title: "确认配置更改",
              description: "确认前不会修改配置。请检查以下内容。",
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
        ? "配置已保存，但尚未在目标应用中测试。请新建会话检查。"
        : null,
    plan,
    preview: plan ? createPreviewModel(plan) : null,
    partialTruth: projectPartialTruth(job?.partialResult ?? null),
    steps,
    resources,
    canConfirm,
    canRegenerate: !job && (mustRegenerate || secretBlocked),
    confirmLabel: busy ? "正在应用…" : "应用更改",
  };
}
