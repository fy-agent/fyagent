import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  AGENT_REASON_CODES,
  installationTargetsForAction,
  type AgentActionId,
  type AgentActionJobSnapshot,
  type AgentActionJobStage,
  type AgentInstallationInventory,
  type AgentInstallationTarget,
  type AgentInstallReadiness,
  type AgentInstallReadinessPort,
  type AgentReasonCode,
  type AgentSourceKind,
  type AgentSurface,
} from "../../shared/features/agent-install-readiness";
import {
  agentJobToSpeedSample,
  createDownloadSpeedState,
  projectAgentJobTransfer,
  updateDownloadSpeedFromSample,
} from "../../shared/features/transfer-projection";
import { canOfferDirectoryUpdate } from "../../shared/features/agent-lifecycle-capabilities";
import type { AgentCatalogId } from "../../shared/features/types";

export const AGENT_LIFECYCLE_JOB_POLL_MS = 800;
export const AGENT_LIFECYCLE_MAX_JOB_POLLS = 2250;
export const AGENT_LIFECYCLE_INCOMPLETE_COPY =
  "无法确认操作结果。请刷新安装状态后再试。";
export const AGENT_LIFECYCLE_SUCCEEDED_COPY = "操作已完成。";
export const AGENT_LIFECYCLE_VENDOR_HANDOFF_COPY =
  "官方安装窗口已打开。完成安装后请刷新安装状态。";
export const AGENT_LIFECYCLE_TIMEOUT_COPY =
  "安装仍在进行。可稍后刷新安装状态。";

export type AgentLifecyclePrimaryAction = "install" | "update";

export type AgentLifecycleActionView = {
  primaryAction: AgentLifecyclePrimaryAction | null;
  busy: boolean;
  stage: AgentActionJobStage | null;
  percent: number | null;
  progressLabel: string | null;
  error: string | null;
  reasonCode: AgentReasonCode | null;
  success: string | null;
  activeSurface: AgentSurface | null;
  canCancel: boolean;
  canRetry: boolean;
  run: (
    action: AgentActionId,
    targetOverride?: AgentInstallationTarget | null,
    surface?: AgentSurface,
  ) => Promise<void>;
  runPrimary: () => Promise<void>;
  retry: () => Promise<void>;
  cancel: () => Promise<void>;
};

export function isTerminalAgentJobStage(stage: AgentActionJobStage): boolean {
  return (
    stage === "succeeded" ||
    stage === "failed" ||
    stage === "cancelled" ||
    stage === "incomplete"
  );
}

export function resolveLifecycleReadiness(
  readiness: AgentInstallReadiness,
  surface?: AgentSurface,
): {
  allowedActions: readonly AgentActionId[];
  releaseId: string | null;
  requiresTargetSelection: boolean;
  sourceKind: AgentSourceKind;
} {
  if (surface && readiness.surfaces && readiness.surfaces.length > 0) {
    const item = readiness.surfaces.find((entry) => entry.surface === surface);
    if (!item) {
      return {
        allowedActions: [],
        releaseId: null,
        requiresTargetSelection: false,
        sourceKind: "managed_desktop",
      };
    }
    return {
      allowedActions: item.allowedActions,
      releaseId: item.releaseId,
      requiresTargetSelection: item.requiresTargetSelection,
      sourceKind: item.sourceKind,
    };
  }
  return {
    allowedActions: readiness.allowedActions,
    releaseId: readiness.releaseId,
    requiresTargetSelection: readiness.requiresTargetSelection,
    sourceKind: readiness.sourceKind,
  };
}

function isCliBound(
  surface: AgentSurface | undefined,
  sourceKind: AgentSourceKind,
): boolean {
  return surface === "cli" || sourceKind === "cli_tooling";
}

export function bindLiveInventoryTarget(
  inventory: AgentInstallationInventory,
  action: AgentActionId,
  previous: AgentInstallationTarget | null,
): AgentInstallationTarget | null {
  const eligible = installationTargetsForAction(inventory, action).filter(
    (target) => target.eligibleActions.includes(action),
  );
  if (eligible.length === 1) return eligible[0];
  if (!previous) return null;
  return (
    eligible.find(
      (target) =>
        target.kind === previous.kind &&
        target.scope === previous.scope &&
        target.label === previous.label,
    ) ?? null
  );
}

export function deriveAgentLifecyclePrimaryAction(
  agentId: AgentCatalogId,
  readiness: AgentInstallReadiness | null,
): AgentLifecyclePrimaryAction | null {
  if (!readiness) return null;
  const allowed = new Set(readiness.allowedActions);
  if (readiness.installState === "not_installed" && allowed.has("install")) {
    return "install";
  }
  if (canOfferDirectoryUpdate(agentId, readiness)) {
    return "update";
  }
  return null;
}

export function reasonCopy(code: AgentReasonCode): string | null {
  switch (code) {
    case "managed_by_codex_desktop":
      return "Codex Desktop 的安装和更新请在现有安装器中完成。";
    case "interactive_user_unavailable":
      return "当前无法显示 Windows 管理员确认窗口。请回到桌面后重试。";
    case "platform_unsupported":
      return "当前系统没有可用的官方安装包。";
    case "source_not_verified":
      return "暂时无法访问官方下载来源。请打开产品官网下载安装。";
    case "official_page_only":
      return "请从产品官网下载安装。";
    case "provider_connection_required":
      return "请先为 OpenCode 连接 Provider。";
    case "auth_state_unknown":
      return null;
    case "operation_conflict":
      return "另一个安装任务正在进行，请完成后再试。";
    case "refresh_required":
      return "下载信息已更新，请刷新后重试。";
    case "target_selection_required":
      return "请选择要安装或更新的应用。";
    case "target_changed":
    case "inventory_expired":
      return "安装位置已变化，请刷新后重新选择。";
    case "target_not_executable":
      return "所选应用无法启动。请检查安装位置或重新安装。";
    case "target_scope_unsupported":
      return "无法在所选安装位置执行此操作。";
    case "candidate_conflict":
      return "检测到互相冲突的安装信息。请检查安装位置后再试。";
    case "authorization_required":
      return "系统应用程序文件夹目前不可用于一键安装。请使用当前用户的应用程序目录，不会改装到其他目录。";
    case "permission_denied":
      return "没有权限更新所选位置，原应用未改动。";
    case "application_running":
      return "应用仍在运行。请先完全退出，再重新执行。";
    case "installer_artifact_unavailable":
      return "安装包准备失败。请检查磁盘空间和文件权限后重试。";
    case "installation_verification_failed":
      return "无法确认新版本已正确安装。请检查安装位置。";
    case "installer_user_cancelled":
      return "你取消了 Windows 安装向导或 UAC 请求。";
    case "installer_process_unobservable":
      return "安装向导已打开，但 FyAgent 无法读取其进度。完成向导后请刷新安装状态。";
    case "installer_timed_out":
      return "安装向导仍未结束。请完成或关闭向导后刷新安装状态。";
    case "installer_exited_nonzero":
      return "Windows 安装向导未能完成。请检查向导中的错误后重试。";
    case "rollback_restored":
      return "新版本无法使用，已恢复之前的应用。";
    case "recovery_required":
      return "无法确认应用是否已恢复。请停止重试并检查安装位置。";
    case "cancelled":
      return "操作已取消。";
    case "executor_not_implemented":
      return "FyAgent 暂时无法在当前环境中完成安装。";
    case "application_launch_failed":
      return "无法打开该软件。请确认应用仍在安装位置后再试。";
    case "surface_not_supported":
      return "当前安装方式不可用。";
    case "action_not_supported":
      return "当前产品不支持此操作。";
    case "helper_not_packaged":
    case "helper_signature_invalid":
      return "系统文件夹一键安装当前不可用，不会改到其他目录。";
    case "helper_install_authorization_cancelled":
    case "operation_authorization_cancelled":
      return "你取消了管理员授权，未更改现有应用。";
    case "helper_install_failed":
    case "helper_peer_rejected":
    case "helper_protocol_incompatible":
      return "无法完成系统级安装组件准备。";
    case "helper_update_required":
      return "需要更新 FyAgent 后才能继续。";
    case "helper_downgrade_rejected":
      return "当前系统组件版本更新，请升级 FyAgent。";
    case "operation_authorization_invalid":
      return "管理员授权无效或已过期，未更改现有应用。";
    case "source_capability_invalid":
    case "source_changed":
      return "安装包在提交前已变化，未更改现有应用。";
    case "target_slot_invalid":
      return "无法在该系统位置安装此应用。";
    case "helper_removal_failed":
      return "无法移除系统安装组件。";
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
      return "正在准备安装包";
    case "launching_installer":
      return "正在打开官方安装窗口，请在该窗口中完成安装";
    case "awaiting_user":
      return "请在弹出的官方安装窗口中完成安装";
    case "installing":
      return "正在安装";
    case "verifying_installation":
      return "正在确认安装结果";
    case "succeeded":
      return "正在更新安装状态";
    case "failed":
      return "操作失败";
    case "cancelled":
      return "操作已取消";
    case "incomplete":
      return "安装结果待检查";
  }
}

export function isVendorInstallerHandoffStage(
  stage: AgentActionJobStage,
): boolean {
  return stage === "launching_installer" || stage === "awaiting_user";
}

export function lifecycleSuccessCopy(vendorHandoff: boolean): string {
  return vendorHandoff
    ? AGENT_LIFECYCLE_VENDOR_HANDOFF_COPY
    : AGENT_LIFECYCLE_SUCCEEDED_COPY;
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
  surface,
}: {
  agentId: AgentCatalogId;
  port: AgentInstallReadinessPort;
  readiness: AgentInstallReadiness | null;
  target?: AgentInstallationTarget | null;
  onReadinessChange?: (data: AgentInstallReadiness) => void;
  onInventoryChange?: (data: AgentInstallationInventory) => void;
  pollIntervalMs?: number;
  maxPolls?: number;
  surface?: AgentSurface;
}): AgentLifecycleActionView {
  const [busy, setBusy] = useState(false);
  const [stage, setStage] = useState<AgentActionJobStage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reasonCode, setReasonCode] = useState<AgentReasonCode | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [cancellable, setCancellable] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const [jobSnapshot, setJobSnapshot] = useState<AgentActionJobSnapshot | null>(
    null,
  );
  const [activeSurface, setActiveSurface] = useState<AgentSurface | null>(null);
  const [downloadSpeed, setDownloadSpeed] = useState(createDownloadSpeedState);

  const generationRef = useRef(0);
  const runningRef = useRef(false);
  const lastActionRef = useRef<AgentActionId | null>(null);
  const lastSurfaceRef = useRef<AgentSurface | undefined>(undefined);
  const vendorHandoffRef = useRef(false);
  const speedRef = useRef(createDownloadSpeedState());
  const readinessRef = useRef(readiness);
  const targetRef = useRef(target);
  const surfaceRef = useRef(surface);
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
    surfaceRef.current = surface;
  });

  useEffect(() => {
    return () => {
      generationRef.current += 1;
      runningRef.current = false;
    };
  }, []);

  const primaryAction = useMemo(
    () => deriveAgentLifecyclePrimaryAction(agentId, readiness),
    [agentId, readiness],
  );

  const reread = useCallback(async (generation: number): Promise<boolean> => {
    try {
      const [data, inventory] = await Promise.all([
        portRef.current.get(agentIdRef.current),
        portRef.current.getInventory(
          agentIdRef.current,
          lastSurfaceRef.current ?? surfaceRef.current,
        ),
      ]);
      if (generationRef.current !== generation) return false;
      onReadinessChangeRef.current?.(data);
      onInventoryChangeRef.current?.(inventory);
      return true;
    } catch {
      return false;
    }
  }, []);

  const resetTransfer = useCallback(() => {
    speedRef.current = createDownloadSpeedState();
    setDownloadSpeed(speedRef.current);
    setJobSnapshot(null);
  }, []);

  const applyJobSnapshot = useCallback((snapshot: AgentActionJobSnapshot) => {
    if (isVendorInstallerHandoffStage(snapshot.stage)) {
      vendorHandoffRef.current = true;
    }
    setStage(snapshot.stage);
    setCancellable(snapshot.cancellable);
    setJobSnapshot(snapshot);
    const nextSpeed = updateDownloadSpeedFromSample(
      speedRef.current,
      agentJobToSpeedSample(snapshot),
    );
    speedRef.current = nextSpeed;
    setDownloadSpeed(nextSpeed);
  }, []);

  const run = useCallback(
    async (
      action: AgentActionId,
      targetOverride?: AgentInstallationTarget | null,
      nextSurface?: AgentSurface,
    ) => {
      const current = readinessRef.current;
      if (!current || runningRef.current) {
        return;
      }
      if (action !== "install" && action !== "update" && action !== "launch") {
        return;
      }
      const resolvedSurface = nextSurface ?? surfaceRef.current;
      const gate = resolveLifecycleReadiness(current, resolvedSurface);
      if (!gate.allowedActions.includes(action)) {
        return;
      }
      const previousTarget = targetOverride ?? targetRef.current ?? null;
      const cliBound = isCliBound(resolvedSurface, gate.sourceKind);
      const targetRequired =
        action === "install" || action === "update"
          ? !cliBound || gate.requiresTargetSelection
          : gate.requiresTargetSelection && action === "launch";
      const shouldRefreshBinding =
        action === "install" ||
        action === "update" ||
        (action === "launch" && (targetRequired || previousTarget !== null));

      const generation = generationRef.current + 1;
      generationRef.current = generation;
      runningRef.current = true;
      lastActionRef.current = action;
      lastSurfaceRef.current = resolvedSurface;
      setActiveSurface(resolvedSurface ?? null);
      setJobId(null);
      setBusy(true);
      setStage("checking");
      setCancellable(false);
      setError(null);
      setReasonCode(null);
      setSuccess(null);
      vendorHandoffRef.current = false;
      resetTransfer();

      let selectedTarget = previousTarget;
      if (shouldRefreshBinding) {
        try {
          const inventory = await portRef.current.getInventory(
            agentIdRef.current,
            resolvedSurface,
          );
          if (generationRef.current !== generation) return;
          if (inventory.agentId === agentIdRef.current) {
            onInventoryChangeRef.current?.(inventory);
            selectedTarget = bindLiveInventoryTarget(
              inventory,
              action,
              previousTarget,
            );
          } else {
            selectedTarget = null;
          }
          targetRef.current = selectedTarget;
        } catch (caught) {
          if (generationRef.current !== generation) return;
          runningRef.current = false;
          setBusy(false);
          setStage(null);
          const reason = actionErrorReason(caught) ?? "inventory_expired";
          setReasonCode(reason);
          setError(agentLifecycleFailureCopy(reason));
          return;
        }
      }

      if (
        targetRequired &&
        (!selectedTarget || !selectedTarget.eligibleActions.includes(action))
      ) {
        runningRef.current = false;
        setBusy(false);
        setStage(null);
        setReasonCode("target_selection_required");
        setError(reasonCopy("target_selection_required"));
        return;
      }

      let outcome: "succeeded" | "failed" | "cancelled" | "timeout" | "error";
      let outcomeReason: AgentReasonCode | null;

      try {
        const result = await portRef.current.startAction({
          agentId: agentIdRef.current,
          action,
          expectedReleaseId: gate.releaseId ?? undefined,
          ...(selectedTarget?.eligibleActions.includes(action)
            ? {
                inventoryId: selectedTarget.inventoryId,
                targetId: selectedTarget.targetId,
                expectedTargetRevision: selectedTarget.expectedTargetRevision,
              }
            : {}),
          ...(lastSurfaceRef.current
            ? { surface: lastSurfaceRef.current }
            : {}),
        });
        if (generationRef.current !== generation) return;
        setStage(result.stage);
        if (isVendorInstallerHandoffStage(result.stage)) {
          vendorHandoffRef.current = true;
        }

        if (result.jobId) {
          setJobId(result.jobId);
          let snapshot = await portRef.current.getActionJob(result.jobId);
          if (generationRef.current !== generation) return;
          applyJobSnapshot(snapshot);

          for (let attempt = 0; attempt < maxPollsRef.current; attempt += 1) {
            if (isTerminalAgentJobStage(snapshot.stage)) {
              break;
            }
            await wait(pollIntervalMsRef.current);
            if (generationRef.current !== generation) return;
            snapshot = await portRef.current.getActionJob(result.jobId);
            if (generationRef.current !== generation) return;
            applyJobSnapshot(snapshot);
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
        const successCopy = lifecycleSuccessCopy(vendorHandoffRef.current);
        setSuccess(successCopy);
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
      resetTransfer();
    },
    [applyJobSnapshot, reread, resetTransfer],
  );

  const runPrimary = useCallback(async () => {
    const next = deriveAgentLifecyclePrimaryAction(
      agentIdRef.current,
      readinessRef.current,
    );
    if (!next) return;
    await run(next);
  }, [run]);

  const retry = useCallback(async () => {
    const last = lastActionRef.current;
    const current = readinessRef.current;
    if (!last || !current) {
      await runPrimary();
      return;
    }
    const gate = resolveLifecycleReadiness(current, lastSurfaceRef.current);
    if (gate.allowedActions.includes(last)) {
      await run(last, undefined, lastSurfaceRef.current);
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
      applyJobSnapshot(snapshot);
    } catch (caught) {
      if (generationRef.current !== generation) return;
      setReasonCode(actionErrorReason(caught));
      setError(agentLifecycleFailureCopy(actionErrorReason(caught)));
    }
  }, [applyJobSnapshot, cancellable, jobId]);

  const transferView = projectAgentJobTransfer(
    stage,
    jobSnapshot,
    downloadSpeed,
  );

  return {
    primaryAction,
    busy,
    stage,
    percent: transferView.percent,
    progressLabel: transferView.downloadLine,
    error,
    reasonCode,
    success,
    activeSurface,
    canCancel: busy && jobId !== null && cancellable,
    canRetry: !busy && error !== null,
    run,
    runPrimary,
    retry,
    cancel,
  };
}
