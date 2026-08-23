import type {
  ChangeJobSnapshot,
  ChangeJobStatus,
  ChangePlanErrorCode,
  ChangeResourceKind,
  ChangeResourceStatus,
  ChangeResultCode,
  ChangeStepKind,
  ChangeStepStatus,
  RestartRequirement,
} from "../../../shared/features/change-plan";

export const STEP_LABELS: Readonly<Record<ChangeStepKind, string>> = {
  precheck: "核对计划与当前基线",
  snapshot: "绑定变更前状态",
  managed_write: "写入受管配置",
  readback: "回读本机配置",
  finalize: "收口执行结果",
};

export const STEP_STATUS_LABELS: Readonly<Record<ChangeStepStatus, string>> = {
  not_started: "未开始",
  running: "进行中",
  succeeded: "已完成",
  failed: "失败",
  compensating: "正在恢复",
  compensated: "已恢复",
  skipped: "未执行",
};

export const JOB_STATUS_LABELS: Readonly<Record<ChangeJobStatus, string>> = {
  planned: "已计划",
  running: "执行中",
  succeeded: "已完成",
  warning: "需留意",
  failed: "失败",
  cancelled: "已取消",
};

export const RESOURCE_LABELS: Readonly<Record<ChangeResourceKind, string>> = {
  provider_db_current: "Provider 当前状态",
  device_current: "本机当前选择",
  target_definition: "目标 Provider 定义",
  codex_live_projection: "Codex 本机配置投影",
};

export const RESOURCE_STATUS_LABELS: Readonly<
  Record<ChangeResourceStatus, string>
> = {
  pending: "待核对",
  matched: "回读一致",
  mismatched: "回读不一致",
  unavailable: "无法确认",
};

export const RESTART_LABELS: Readonly<Record<RestartRequirement, string>> = {
  not_required: "无需重启",
  recommended: "建议重启或新建会话",
  unknown: "重启要求尚无法确认",
};

const RESULT_COPY: Readonly<Record<ChangeResultCode, string>> = {
  planned: "任务已创建，尚未开始写入。",
  running: "正在按后端持久快照执行。",
  applied: "配置已应用，可直接开始使用。",
  applied_restart_recommended:
    "配置已应用，可直接开始使用；建议重启或新建会话。",
  applied_with_warning: "配置已应用，但有需要留意的本机状态。",
  cancelled_before_write: "已在首笔受管写入前取消，目标配置未写入。",
  interrupted_before_write: "执行在首笔写入前中断，目标配置未写入。",
  recovered_target_reached:
    "中断后已通过真实回读确认目标配置，但执行过程需要留意。",
  writer_failed_baseline_restored:
    "写入未完成，已通过回读确认此前状态得到恢复。",
  writer_error_target_reached: "写入返回异常，但真实回读显示目标配置已生效。",
  post_write_mismatch: "写入后的本机状态不一致，需要人工检查。",
  readback_unavailable: "无法完成真实回读，当前结果不能确认。",
  recovery_required: "缺少足够证据确认结果，需要人工恢复。",
};

const PLAN_ERROR_COPY: Readonly<Record<ChangePlanErrorCode, string>> = {
  unsupported_operation: "当前操作暂不受支持。",
  target_not_found: "目标 Provider 已不存在，请刷新后重试。",
  target_already_current: "目标 Provider 已是当前配置。",
  baseline_unavailable: "当前基线无法安全读取，请修复本机配置后重试。",
  invalid_digest: "计划身份校验失败，请重新生成计划。",
  expired: "计划已过期，请重新预览。",
  consumed: "计划已经执行过，请查看已有任务结果。",
  stale: "计划生成后配置已变化，请重新预览。",
  plan_not_found: "计划已不存在，请重新生成。",
  job_not_found: "执行任务已不存在，请刷新状态。",
  internal: "本机执行暂时不可用，请检查日志后重试。",
};

export function resultCopy(result: ChangeResultCode): string {
  return RESULT_COPY[result];
}

export function planErrorCopy(error: ChangePlanErrorCode): string {
  return PLAN_ERROR_COPY[error];
}

export function unknownPlanErrorCopy(error: unknown): string {
  return typeof error === "string" && error in PLAN_ERROR_COPY
    ? PLAN_ERROR_COPY[error as ChangePlanErrorCode]
    : "无法完成本机变更，请刷新状态后重试。";
}

export function canRequestCancellation(job: ChangeJobSnapshot): boolean {
  if (job.status !== "planned" && job.status !== "running") return false;
  const write = job.steps.find((step) => step.kind === "managed_write");
  return write?.status === "not_started" || write?.status === "skipped";
}

export function manualRecoveryCopy(job: ChangeJobSnapshot): string[] {
  if (!job.partialResult) return [];
  return job.partialResult.manualActions.map((action) => {
    switch (action) {
      case "restore_readback_authority":
        return "恢复本机配置的可读状态后重新回读。";
      case "inspect_and_resolve":
        return "检查本机 Provider 与 Codex 配置并人工消除不一致。";
      default:
        return "请人工检查本机状态后再继续修改。";
    }
  });
}
