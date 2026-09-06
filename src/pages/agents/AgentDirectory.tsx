import { useCallback, type ReactNode } from "react";

import { useQueryClient } from "@tanstack/react-query";

import { getAgentBrand } from "../../shared/assets/agents";
import { useCodexDesktopInstaller } from "../../shared/codex-desktop/useCodexDesktopInstaller";
import {
  installationTargetsForAction,
  type AgentInstallationTarget,
  type AgentInstallReadiness,
} from "../../shared/features/agent-install-readiness";
import {
  featureKeys,
  useAgentInstallationInventory,
} from "../../shared/features/queries";
import { useFeatures } from "../../shared/features/provider";
import { formatTransferPercent } from "../../shared/features/transfer-projection";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogEntry,
  type AgentCatalogId,
} from "../../shared/features/types";
import { BrandIconFrame } from "../../shared/ui/catalog";
import { Button } from "../../shared/ui/Button";
import { InlineNotice } from "../../shared/ui/primitives";

import {
  AgentLifecycleActionSlot,
  type AgentLifecycleActionSlotView,
} from "./AgentLifecycleActionSlot";
import { applyCommittedAgentDirectoryOrder } from "./agentDirectoryOrder";
import {
  observeAgentDirectoryRow,
  type AgentDirectoryRowObservation,
} from "./agentDirectoryScanProjection";
import {
  projectCodexDirectoryAction,
  type CodexDirectoryActionProjection,
} from "./codexDirectoryActionProjection";
import type { AgentDirectoryScanController } from "./useAgentDirectoryScan";
import {
  deriveAgentLifecyclePrimaryAction,
  jobStageCopy,
  useAgentLifecycleAction,
  type AgentLifecycleActionView,
} from "./useAgentLifecycleAction";
import { AgentAuthStatusPanel } from "./AgentAuthStatusPanel";

function isInstalledReadiness(
  data: AgentInstallReadiness | undefined,
): boolean {
  return (
    data?.installState === "installed" ||
    data?.installState === "installed_not_runnable"
  );
}

function scanButtonLabel(
  status: AgentDirectoryScanController["state"]["status"],
): string {
  if (status === "scanning") return "扫描中…";
  if (status === "complete") return "重新扫描";
  return "开始扫描";
}

function rowKindCopy(observation: AgentDirectoryRowObservation): string | null {
  if (observation.refreshing) return "正在刷新";
  if (observation.kind === "pending") return null;
  if (observation.readFailed && !isInstalledReadiness(observation.readiness)) {
    return "读取失败";
  }
  if (observation.kind === "error") return "读取失败";
  if (observation.kind === "unknown") return "状态未知";
  if (observation.kind === "unavailable") return "当前不可用";
  return null;
}

function directoryBusyCopy(
  observation: AgentDirectoryRowObservation,
): string | null {
  if (observation.kind === "pending" && !observation.refreshing) {
    return "正在扫描";
  }
  if (observation.refreshing) return "正在刷新";
  return null;
}

function genericBusyCopy(lifecycle: AgentLifecycleActionView): string {
  return (
    lifecycle.progressLabel ??
    (lifecycle.stage ? jobStageCopy(lifecycle.stage) : "处理中…")
  );
}

function codexBusyCopy(projection: CodexDirectoryActionProjection): string {
  if (projection.state === "job_downloading") {
    const percentLabel =
      projection.percent !== null
        ? formatTransferPercent(projection.percent)
        : null;
    if (percentLabel && projection.speedLabel) {
      return `正在下载 ${percentLabel} · ${projection.speedLabel}`;
    }
    if (percentLabel) return `正在下载 ${percentLabel}`;
    if (projection.speedLabel) return `正在下载 ${projection.speedLabel}`;
    return "正在下载";
  }
  switch (projection.state) {
    case "job_checking":
      return "正在检查来源";
    case "job_preflight":
      return "正在执行安装前检查";
    case "job_installing":
      return "正在安装";
    case "job_verifying_installation":
      return "正在确认安装结果";
    default:
      return "处理中…";
  }
}

function DirectoryCardShell({
  entry,
  observation,
  lifecycleBusy,
  lifecycleSlot,
  authSlot,
  error,
  success,
  onConfigure,
}: {
  entry: AgentCatalogEntry;
  observation: AgentDirectoryRowObservation;
  lifecycleBusy: boolean;
  lifecycleSlot: ReactNode;
  authSlot: ReactNode;
  error: string | null;
  success?: string | null;
  onConfigure: (agentId: AgentCatalogId) => void;
}) {
  const kindCopy = rowKindCopy(observation);
  const configurable = observation.configurable && !lifecycleBusy;
  return (
    <article
      className="fy-agent-directory-card"
      data-agent-id={entry.id}
      data-row-kind={observation.kind}
    >
      <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
      <div className="fy-agent-directory-card-copy">
        <div className="fy-agent-directory-card-heading">
          <h2>{entry.displayName}</h2>
          {kindCopy ? (
            <span
              className="fy-agent-directory-status"
              data-status={
                observation.kind === "error" || observation.readFailed
                  ? "error"
                  : observation.kind === "unknown" ||
                      observation.kind === "unavailable"
                    ? "unknown"
                    : "warning"
              }
            >
              {kindCopy}
            </span>
          ) : null}
        </div>
        <p className="fy-agent-directory-description">{entry.description}</p>
        {authSlot}
        <DirectoryActionFeedback error={error} success={success ?? null} />
      </div>
      <div className="fy-agent-directory-card-actions">
        {lifecycleSlot}
        <Button disabled={!configurable} onClick={() => onConfigure(entry.id)}>
          进行配置
        </Button>
      </div>
    </article>
  );
}

function DirectoryActionFeedback({
  error,
  success,
}: {
  error: string | null;
  success: string | null;
}) {
  const text = error ?? success;
  if (!text) return null;
  return (
    <p
      className="fy-agent-directory-card-feedback"
      data-tone={error ? "error" : "info"}
      role="status"
    >
      {text}
    </p>
  );
}

function GenericDirectoryCard({
  entry,
  observation,
  onConfigure,
  onReadinessChange,
}: {
  entry: AgentCatalogEntry;
  observation: AgentDirectoryRowObservation;
  onConfigure: (agentId: AgentCatalogId) => void;
  onReadinessChange: (data: AgentInstallReadiness) => void;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const primaryAction = deriveAgentLifecyclePrimaryAction(
    entry.id,
    observation.readiness ?? null,
  );
  const inventory = useAgentInstallationInventory(
    entry.id,
    primaryAction !== null,
  );
  const eligibleTargets =
    primaryAction && inventory.data
      ? installationTargetsForAction(inventory.data, primaryAction).filter(
          (target) => target.eligibleActions.includes(primaryAction),
        )
      : [];
  const primaryTarget: AgentInstallationTarget | null =
    eligibleTargets.length === 1 ? eligibleTargets[0] : null;
  const lifecycle = useAgentLifecycleAction({
    agentId: entry.id,
    port: ports.agentInstallReadiness,
    readiness: observation.readiness ?? null,
    target: primaryTarget,
    onReadinessChange,
    onInventoryChange: (data) => {
      queryClient.setQueryData(
        featureKeys.agentInstallationInventory(entry.id),
        data,
      );
    },
  });
  const scanningCopy = directoryBusyCopy(observation);
  const targetStatus =
    primaryAction === null
      ? "not_needed"
      : inventory.isPending
        ? "loading"
        : inventory.isError
          ? "unavailable"
          : eligibleTargets.length === 1
            ? "single"
            : "selection_required";
  return (
    <DirectoryCardShell
      entry={entry}
      observation={observation}
      lifecycleBusy={lifecycle.busy}
      error={lifecycle.error}
      success={lifecycle.success}
      authSlot={
        <AgentAuthStatusPanel
          agentId={entry.id}
          mode="compact"
          enabled={observation.configurable}
        />
      }
      onConfigure={onConfigure}
      lifecycleSlot={
        <GenericLifecycleSlot
          observation={observation}
          scanningCopy={scanningCopy}
          lifecycle={lifecycle}
          targetStatus={targetStatus}
          onConfigure={() => onConfigure(entry.id)}
        />
      }
    />
  );
}

function genericLifecycleSlotView(
  observation: AgentDirectoryRowObservation,
  scanningCopy: string | null,
  lifecycle: AgentLifecycleActionView,
  targetStatus:
    | "not_needed"
    | "loading"
    | "unavailable"
    | "single"
    | "selection_required",
  onConfigure: () => void,
): AgentLifecycleActionSlotView {
  if (lifecycle.busy) {
    return { kind: "status", label: genericBusyCopy(lifecycle) };
  }
  if (scanningCopy && !lifecycle.primaryAction) {
    return { kind: "status", label: scanningCopy };
  }
  if (lifecycle.primaryAction) {
    if (targetStatus === "loading") {
      return { kind: "status", label: "正在读取安装目标" };
    }
    if (targetStatus !== "single") {
      return { kind: "select_target", onClick: onConfigure };
    }
    return {
      kind: "primary",
      action: lifecycle.primaryAction,
      onClick: () => void lifecycle.runPrimary(),
    };
  }
  if (lifecycle.canRetry) {
    return { kind: "retry", onClick: () => void lifecycle.retry() };
  }
  if (scanningCopy) {
    return { kind: "status", label: scanningCopy };
  }
  if (observation.kind === "pending") {
    return { kind: "status", label: "正在扫描" };
  }
  return { kind: "empty" };
}

function GenericLifecycleSlot({
  observation,
  scanningCopy,
  lifecycle,
  targetStatus,
  onConfigure,
}: {
  observation: AgentDirectoryRowObservation;
  scanningCopy: string | null;
  lifecycle: AgentLifecycleActionView;
  targetStatus:
    | "not_needed"
    | "loading"
    | "unavailable"
    | "single"
    | "selection_required";
  onConfigure: () => void;
}) {
  return (
    <AgentLifecycleActionSlot
      view={genericLifecycleSlotView(
        observation,
        scanningCopy,
        lifecycle,
        targetStatus,
        onConfigure,
      )}
    />
  );
}

function CodexDirectoryCard({
  entry,
  observation,
  projection,
  onConfigure,
  onRun,
}: {
  entry: AgentCatalogEntry;
  observation: AgentDirectoryRowObservation;
  projection: CodexDirectoryActionProjection;
  onConfigure: (agentId: AgentCatalogId) => void;
  onRun: () => Promise<void>;
}) {
  const scanningCopy = directoryBusyCopy(observation);
  const error = projection.error?.details.redactedMessage ?? null;
  return (
    <DirectoryCardShell
      entry={entry}
      observation={observation}
      lifecycleBusy={projection.busy}
      error={error}
      authSlot={
        <AgentAuthStatusPanel
          agentId={entry.id}
          mode="compact"
          enabled={observation.configurable}
        />
      }
      onConfigure={onConfigure}
      lifecycleSlot={
        <CodexLifecycleSlot
          scanningCopy={scanningCopy}
          observation={observation}
          projection={projection}
          onRun={onRun}
        />
      }
    />
  );
}

function codexLifecycleSlotView(
  scanningCopy: string | null,
  observation: AgentDirectoryRowObservation,
  projection: CodexDirectoryActionProjection,
  onRun: () => Promise<void>,
): AgentLifecycleActionSlotView {
  if (projection.busy) {
    return { kind: "status", label: codexBusyCopy(projection) };
  }
  if (observation.kind === "pending" && !observation.refreshing) {
    return { kind: "status", label: "正在扫描" };
  }
  if (projection.canRun && projection.primaryAction) {
    return {
      kind: "primary",
      action: projection.primaryAction,
      onClick: () => void onRun(),
    };
  }
  if (projection.canRetry) {
    return { kind: "retry", onClick: () => void onRun() };
  }
  if (scanningCopy) {
    return { kind: "status", label: scanningCopy };
  }
  return { kind: "empty" };
}

function CodexLifecycleSlot({
  scanningCopy,
  observation,
  projection,
  onRun,
}: {
  scanningCopy: string | null;
  observation: AgentDirectoryRowObservation;
  projection: CodexDirectoryActionProjection;
  onRun: () => Promise<void>;
}) {
  return (
    <AgentLifecycleActionSlot
      view={codexLifecycleSlotView(
        scanningCopy,
        observation,
        projection,
        onRun,
      )}
    />
  );
}

export function AgentDirectory({
  entries,
  scanController,
  onConfigure,
}: {
  entries: readonly AgentCatalogEntry[];
  scanController: AgentDirectoryScanController;
  onConfigure: (agentId: AgentCatalogId) => void;
}) {
  const { ports } = useFeatures();
  const installer = useCodexDesktopInstaller();
  const { state, start, applyReadiness } = scanController;
  const scanning = state.status === "scanning";
  const complete = state.status === "complete";
  const visibleEntries = applyCommittedAgentDirectoryOrder(
    entries,
    state.committedOrderIds,
  );
  const hasSuccessfulResults = Object.keys(state.results).length > 0;
  const currentReadiness = state.currentSuccessIds
    .map((id) => state.results[id])
    .filter((item): item is AgentInstallReadiness => Boolean(item));
  const discoveredCount = currentReadiness.filter((item) =>
    isInstalledReadiness(item),
  ).length;
  const allFailed = complete && state.currentSuccessIds.length === 0;
  const hasTechnicalError = state.currentFailureIds.length > 0;
  const codexProjection = projectCodexDirectoryAction(installer);

  const runCodexAndReread = useCallback(async () => {
    await installer.runPrimaryAction();
    try {
      applyReadiness("codex", await ports.agentInstallReadiness.get("codex"));
    } catch {
      /* Keep the last scan observation until an authoritative reread succeeds. */
    }
  }, [applyReadiness, installer, ports.agentInstallReadiness]);

  return (
    <section className="fy-agent-directory" aria-label="AI 软件目录">
      <header className="fy-agent-directory-header">
        <div className="fy-agent-directory-title-row">
          <div className="fy-agent-directory-title-group">
            <h1>我的 AI 软件</h1>
          </div>
          <Button
            className="fy-control-button-primary fy-agent-scan-button"
            disabled={scanning}
            onClick={start}
          >
            {scanButtonLabel(state.status)}
          </Button>
        </div>
      </header>

      {scanning ? (
        <div
          className="fy-agent-directory-progress"
          role="status"
          aria-live="polite"
        >
          <div className="fy-agent-directory-progress-labels">
            <span>正在扫描本机 AI 软件</span>
            <span>已发现 {discoveredCount} 个</span>
          </div>
          <progress
            max={AGENT_CATALOG_IDS.length}
            value={state.settledIds.length}
            aria-label="扫描进度"
          />
        </div>
      ) : null}

      {allFailed ? (
        <InlineNotice tone="error">
          本次扫描未能读取任何软件状态。
          {hasSuccessfulResults ? "已保留上次成功结果，请重试。" : "请重试。"}
        </InlineNotice>
      ) : hasTechnicalError && complete ? (
        <InlineNotice tone="warning">
          {state.currentFailureIds.length} 个软件状态读取失败，成功结果已更新。
        </InlineNotice>
      ) : null}

      <div className="fy-agent-directory-list">
        {visibleEntries.map((entry) => {
          const observation = observeAgentDirectoryRow(entry.id, state);
          if (entry.id === "codex") {
            return (
              <CodexDirectoryCard
                key={entry.id}
                entry={entry}
                observation={observation}
                projection={codexProjection}
                onConfigure={onConfigure}
                onRun={runCodexAndReread}
              />
            );
          }
          return (
            <GenericDirectoryCard
              key={entry.id}
              entry={entry}
              observation={observation}
              onConfigure={onConfigure}
              onReadinessChange={(data) => applyReadiness(entry.id, data)}
            />
          );
        })}
      </div>
    </section>
  );
}
