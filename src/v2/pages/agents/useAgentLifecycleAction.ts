import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  AGENT_REASON_CODES,
  type AgentActionId,
  type AgentActionJobStage,
  type AgentInstallationInventory,
  type AgentInstallationTarget,
  type AgentInstallReadiness,
  type AgentInstallReadinessPort,
  type AgentReasonCode,
} from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";

export const AGENT_LIFECYCLE_JOB_POLL_MS = 800;
export const AGENT_LIFECYCLE_MAX_JOB_POLLS = 2250;
export const AGENT_LIFECYCLE_INCOMPLETE_COPY =
  "操作未能完成。此区域不会推断安装成功。";
export const AGENT_LIFECYCLE_SUCCEEDED_COPY =
  "操作已完成。下面是再次读取的状态，不是推断。";
export const AGENT_LIFECYCLE_TIMEOUT_COPY =
  "安装仍在进行。超时不会被当成成功或失败。";

export type AgentLifecyclePrimaryAction = "install" | "update";

export type AgentLifecycleActionView = {
  primaryAction: AgentLifecyclePrimaryAction | null;
  busy: boolean;
  stage: AgentActionJobStage | null;
  percent: number | null;
  error: string | null;
  reasonCode: AgentReasonCode | null;
  success: string | null;
  canCancel: boolean;
  canRetry: boolean;
  run: (
    action: AgentActionId,
    targetOverride?: AgentInstallationTarget | null,
  ) => Promise<void>;
  runPrimary: () => Promise<void>;
  retry: () => Promise<void>;
  cancel: () => Promise<void>;
};

export function isTerminalAgentJobStage(stage: AgentActionJobStage): boolean {
  return stage === "succeeded" || stage === "failed" || stage === "cancelled";
}

export function deriveAgentLifecyclePrimaryAction(
  readiness: AgentInstallReadiness | null,
): AgentLifecyclePrimaryAction | null {
  if (!readiness) return null;
  const allowed = new Set(readiness.allowedActions);
  if (readiness.installState === "not_installed" && allowed.has("install")) {
    return "install";
  }
  if (
    (readiness.installState === "installed" ||
      readiness.installState === "installed_not_runnable") &&
    readiness.updateState === "update_available" &&
    allowed.has("update")
  ) {
    return "update";
  }
  return null;
}

export function reasonCopy(code: AgentReasonCode): string | null {
  switch (code) {
    case "managed_by_codex_desktop":
      return "安装与更新由现有 Codex Desktop 安装器管理。";
    case "interactive_user_unavailable":
      return "当前 Windows 提升环境不会代为执行安装命令。";
    case "platform_unsupported":
      return "当前平台没有可用的官方安装包。";
    case "source_not_verified":
      return "官方来源当前不可用，请改用产品页面。";
    case "official_page_only":
      return "请改用官方产品下载页。不会使用固定的历史版本地址。";
    case "provider_connection_required":
      return "OpenCode 需要连接 Provider，而不是全局登录。";
    case "auth_state_unknown":
      return null;
    case "operation_conflict":
      return "已有安装任务进行中，请等待当前任务结束。";
    case "refresh_required":
      return "来源已变化，请刷新后再试。";
    case "target_selection_required":
      return "请选择本次要管理的安装目标。";
    case "target_changed":
    case "inventory_expired":
      return "安装目标已变化，请刷新安装清单后再试。";
    case "target_not_executable":
      return "所选安装当前不可启动。";
    case "target_scope_unsupported":
      return "当前操作不支持所选安装范围。";
    case "candidate_conflict":
      return "检测到相互冲突的安装证据，已停止自动操作。";
    case "authorization_required":
      return "所选系统安装位置需要授权。当前不会自动改装到用户目录。";
    case "permission_denied":
      return "没有权限更新所选位置。原应用保持不变。";
    case "application_running":
      return "应用仍在运行。请先完全退出，再重新执行。";
    case "installation_verification_failed":
      return "安装后验证未通过，未确认新版本可用。";
    case "rollback_restored":
      return "新版本验证未通过，已恢复原应用。";
    case "recovery_required":
      return "安装恢复无法确认完成。请停止重试并检查应用安装状态。";
    case "cancelled":
      return "操作已取消。";
    case "executor_not_implemented":
      return "当前无法完成安装步骤。";
    case "installed_not_runnable":
      return "已写入安装位置，但还不能确认可以运行。";
    default:
      return null;
  }
}

export function jobStageCopy(stage: AgentActionJobStage): string {
  switch (stage) {
    case "checking":
      return "正在检查来源";
    case "downloading":
      return "正在下载安装包";
    case "staging":
      return "正在准备并验证应用";
    case "installing":
      return "正在安装";
    case "verifying_installation":
      return "正在确认安装结果";
    case "succeeded":
      return "正在读取安装结果";
    case "failed":
      return "操作失败";
    case "cancelled":
      return "操作已取消";
  }
}

export function agentLifecycleFailureCopy(
  code: AgentReasonCode | null,
): string {
  return (code && reasonCopy(code)) || AGENT_LIFECYCLE_INCOMPLETE_COPY;
}

export function actionErrorReason(error: unknown): AgentReasonCode | null {
  if (!error || typeof error !== "object" || !("reasonCode" in error)) {
    return null;
  }
  const code = error.reasonCode;
  return typeof code === "string" &&
    (AGENT_REASON_CODES as readonly string[]).includes(code)
    ? (code as AgentReasonCode)
    : null;
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

export function useAgentLifecycleAction({
  agentId,
  port,
  readiness,
  target,
  onReadinessChange,
  onInventoryChange,
  pollIntervalMs = AGENT_LIFECYCLE_JOB_POLL_MS,
  maxPolls = AGENT_LIFECYCLE_MAX_JOB_POLLS,
}: {
  agentId: AgentCatalogId;
  port: AgentInstallReadinessPort;
  readiness: AgentInstallReadiness | null;
  target?: AgentInstallationTarget | null;
  onReadinessChange?: (data: AgentInstallReadiness) => void;
  onInventoryChange?: (data: AgentInstallationInventory) => void;
  pollIntervalMs?: number;
  maxPolls?: number;
}): AgentLifecycleActionView {
  const [busy, setBusy] = useState(false);
  const [stage, setStage] = useState<AgentActionJobStage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reasonCode, setReasonCode] = useState<AgentReasonCode | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [cancellable, setCancellable] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);

  const generationRef = useRef(0);
  const runningRef = useRef(false);
  const lastActionRef = useRef<AgentActionId | null>(null);
  const readinessRef = useRef(readiness);
  const targetRef = useRef(target);
  const onReadinessChangeRef = useRef(onReadinessChange);
  const onInventoryChangeRef = useRef(onInventoryChange);
  const portRef = useRef(port);
  const agentIdRef = useRef(agentId);
  const pollIntervalMsRef = useRef(pollIntervalMs);
  const maxPollsRef = useRef(maxPolls);

  useEffect(() => {
    readinessRef.current = readiness;
    targetRef.current = target;
    onReadinessChangeRef.current = onReadinessChange;
    onInventoryChangeRef.current = onInventoryChange;
    portRef.current = port;
    agentIdRef.current = agentId;
    pollIntervalMsRef.current = pollIntervalMs;
    maxPollsRef.current = maxPolls;
  });

  useEffect(() => {
    return () => {
      generationRef.current += 1;
      runningRef.current = false;
    };
  }, []);

  const primaryAction = useMemo(
    () => deriveAgentLifecyclePrimaryAction(readiness),
    [readiness],
  );

  const reread = useCallback(async (generation: number): Promise<boolean> => {
    try {
      const [data, inventory] = await Promise.all([
        portRef.current.get(agentIdRef.current),
        portRef.current.getInventory(agentIdRef.current),
      ]);
      if (generationRef.current !== generation) return false;
      onReadinessChangeRef.current?.(data);
      onInventoryChangeRef.current?.(inventory);
      return true;
    } catch {
      return false;
    }
  }, []);

  const run = useCallback(
    async (
      action: AgentActionId,
      targetOverride?: AgentInstallationTarget | null,
    ) => {
      const current = readinessRef.current;
      if (!current || runningRef.current) return;
      if (!current.allowedActions.includes(action)) return;
      const selectedTarget = targetOverride ?? targetRef.current;
      const targetRequired =
        action === "install" ||
        action === "update" ||
        (current.requiresTargetSelection &&
          (action === "launch" || action === "auth_login"));
      if (
        targetRequired &&
        (!selectedTarget || !selectedTarget.eligibleActions.includes(action))
      ) {
        setReasonCode("target_selection_required");
        setError(reasonCopy("target_selection_required"));
        return;
      }

      const generation = generationRef.current + 1;
      generationRef.current = generation;
      runningRef.current = true;
      lastActionRef.current = action;
      setJobId(null);
      setBusy(true);
      setStage("checking");
      setCancellable(false);
      setError(null);
      setReasonCode(null);
      setSuccess(null);

      let outcome: "succeeded" | "failed" | "cancelled" | "timeout" | "error";
      let outcomeReason: AgentReasonCode | null;

      try {
        const result = await portRef.current.startAction({
          agentId: agentIdRef.current,
          action,
          expectedReleaseId: current.releaseId ?? undefined,
          ...(selectedTarget?.eligibleActions.includes(action)
            ? {
                inventoryId: selectedTarget.inventoryId,
                targetId: selectedTarget.targetId,
                expectedTargetRevision: selectedTarget.expectedTargetRevision,
              }
            : {}),
        });
        if (generationRef.current !== generation) return;
        setStage(result.stage);

        if (result.jobId) {
          setJobId(result.jobId);
          let snapshot = await portRef.current.getActionJob(result.jobId);
          if (generationRef.current !== generation) return;
          setStage(snapshot.stage);
          setCancellable(snapshot.cancellable);

          for (let attempt = 0; attempt < maxPollsRef.current; attempt += 1) {
            if (isTerminalAgentJobStage(snapshot.stage)) {
              break;
            }
            await wait(pollIntervalMsRef.current);
            if (generationRef.current !== generation) return;
            snapshot = await portRef.current.getActionJob(result.jobId);
            if (generationRef.current !== generation) return;
            setStage(snapshot.stage);
            setCancellable(snapshot.cancellable);
          }

          if (!isTerminalAgentJobStage(snapshot.stage)) {
            outcome = "timeout";
            outcomeReason = null;
          } else if (snapshot.stage === "succeeded") {
            outcome = "succeeded";
            outcomeReason = snapshot.reasonCode;
          } else if (snapshot.stage === "cancelled") {
            outcome = "cancelled";
            outcomeReason = snapshot.reasonCode;
          } else {
            outcome = "failed";
            outcomeReason = snapshot.reasonCode;
          }
        } else if (result.stage === "succeeded") {
          outcome = "succeeded";
          outcomeReason = result.reasonCode;
        } else if (result.stage === "cancelled") {
          outcome = "cancelled";
          outcomeReason = result.reasonCode;
        } else {
          outcome = "failed";
          outcomeReason = result.reasonCode;
        }
      } catch (caught) {
        if (generationRef.current !== generation) return;
        outcome = "error";
        outcomeReason = actionErrorReason(caught);
      }

      const readbackOk = await reread(generation);
      if (generationRef.current !== generation) return;

      setReasonCode(outcomeReason);
      if (outcome === "succeeded" && readbackOk) {
        setSuccess(AGENT_LIFECYCLE_SUCCEEDED_COPY);
        setError(null);
      } else if (outcome === "timeout") {
        setSuccess(null);
        setError(AGENT_LIFECYCLE_TIMEOUT_COPY);
      } else {
        setSuccess(null);
        setError(agentLifecycleFailureCopy(outcomeReason));
      }
      runningRef.current = false;
      setBusy(false);
      setStage(null);
      setCancellable(false);
      setJobId(null);
    },
    [reread],
  );

  const runPrimary = useCallback(async () => {
    const next = deriveAgentLifecyclePrimaryAction(readinessRef.current);
    if (!next) return;
    await run(next);
  }, [run]);

  const retry = useCallback(async () => {
    const last = lastActionRef.current;
    if (last && readinessRef.current?.allowedActions.includes(last)) {
      await run(last);
      return;
    }
    await runPrimary();
  }, [run, runPrimary]);

  const cancel = useCallback(async () => {
    if (!jobId || !cancellable) return;
    const requestedJobId = jobId;
    const generation = generationRef.current;
    try {
      const snapshot = await portRef.current.cancelAction(requestedJobId);
      if (generationRef.current !== generation) return;
      setStage(snapshot.stage);
      setCancellable(snapshot.cancellable);
    } catch (caught) {
      if (generationRef.current !== generation) return;
      setReasonCode(actionErrorReason(caught));
      setError(agentLifecycleFailureCopy(actionErrorReason(caught)));
    }
  }, [cancellable, jobId]);

  return {
    primaryAction,
    busy,
    stage,
    percent: null,
    error,
    reasonCode,
    success,
    canCancel: busy && jobId !== null && cancellable,
    canRetry: !busy && error !== null,
    run,
    runPrimary,
    retry,
    cancel,
  };
}
