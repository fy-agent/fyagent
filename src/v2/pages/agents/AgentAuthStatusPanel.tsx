import { useMemo, useRef, useState } from "react";

import type {
  AgentAuthIntent,
  AgentAuthObservation,
  AgentAuthReasonCode,
  AgentAuthSessionSnapshot,
} from "../../shared/features/agent-auth";
import {
  installationTargetsForAction,
  type AgentInstallationTarget,
} from "../../shared/features/agent-install-readiness";
import {
  useAgentAuthObservation,
  useAgentInstallationInventory,
} from "../../shared/features/queries";
import { useFeatures } from "../../shared/features/provider";
import type { AgentCatalogId } from "../../shared/features/types";
import { LifecycleTargetPicker } from "../../shared/ui/LifecycleTargetPicker";
import { Button, InlineNotice, Spinner } from "../../shared/ui/primitives";

import {
  isAgentAuthSessionTerminal,
  useAgentAuthSession,
} from "./useAgentAuthSession";

const DESKTOP_HANDOFF_AGENTS = new Set<AgentCatalogId>([
  "qoderwork",
  "trae-work",
  "workbuddy",
]);

function observationSummary(observation: AgentAuthObservation): string {
  switch (observation.kind) {
    case "account":
      if (observation.authority !== "verified") return "登录状态待确认";
      if (observation.state === "logged_in") return "已登录";
      if (observation.state === "logged_out") return "未登录";
      return "暂时无法确认登录状态";
    case "provider_connections":
      if (observation.authority !== "verified")
        return "Provider 连接状态待确认";
      return observation.providers.length === 0
        ? "尚未连接 Provider"
        : `已连接 ${observation.providers.length} 个 Provider`;
    case "handoff_only":
      return "请在官方应用中登录";
    case "fyagent_managed":
      return "请在 FyAgent 认证中心管理";
    case "unavailable":
      return "当前无法读取认证状态";
  }
}

function observationDescription(observation: AgentAuthObservation): string {
  switch (observation.kind) {
    case "account":
      return observation.authority === "verified"
        ? "这是官方应用当前返回的登录状态。"
        : "暂时无法确认登录结果。完成登录后请刷新状态。";
    case "provider_connections":
      return "OpenCode 分别连接各个 Provider，没有统一登录状态。";
    case "handoff_only":
      return "FyAgent 会打开官方应用或 CLI。完成登录后请返回并刷新状态。";
    case "fyagent_managed":
      return "Codex 账号请在现有认证中心管理。";
    case "unavailable":
      return "暂时无法检查登录状态。FyAgent 不会读取厂商保存的凭据。";
  }
}

function intentLabel(intent: AgentAuthIntent): string {
  switch (intent) {
    case "login":
      return "登录";
    case "logout":
      return "退出登录";
    case "connect_provider":
      return "连接 Provider";
  }
}

function stageCopy(snapshot: AgentAuthSessionSnapshot): string {
  switch (snapshot.stage) {
    case "preparing":
      return "正在准备认证";
    case "launching":
      return "正在打开官方认证入口";
    case "awaiting_user":
      return "等待你完成官方认证";
    case "verifying":
      return "正在检查登录状态";
    case "verified":
      return "登录状态已更新";
    case "handoff_complete":
      return "已交给官方认证入口";
    case "failed":
      return "认证操作失败";
    case "cancelled":
      return "已停止等待认证结果";
    case "timed_out":
      return "等待登录结果超时";
  }
}

function reasonCopy(reason: AgentAuthReasonCode | null): string | null {
  switch (reason) {
    case null:
      return null;
    case "auth_state_unknown":
      return "暂时无法确认登录状态。请刷新后重试。";
    case "auth_observer_unavailable":
      return "暂时无法检查登录状态。";
    case "auth_output_invalid":
      return "无法识别官方应用返回的登录状态。请更新应用或重试。";
    case "interactive_user_unavailable":
      return "当前无法打开登录窗口。请回到桌面后重试。";
    case "operation_conflict":
      return "此应用已有登录操作正在进行。";
    case "provider_selection_required":
      return "请选择要断开的 Provider。";
    case "provider_changed":
      return "Provider 列表已变化，请刷新后重试。";
    case "monitoring_stopped":
      return "已停止等待；已打开的登录窗口仍会保留。";
    case "timed_out":
      return "等待登录结果超时。完成登录后请刷新状态。";
    case "handoff_only":
      return "登录入口已打开。完成后请刷新状态。";
    case "managed_by_auth_center":
      return "请在现有认证中心管理此账号。";
    case "target_selection_required":
      return "检测到多个安装，请选择要登录的应用。";
    case "target_changed":
    case "inventory_expired":
      return "安装位置已变化，请刷新后重新选择。";
    case "target_not_executable":
      return "所选应用无法启动。请检查安装位置或重新安装。";
    case "command_failed":
      return "登录操作未能完成。请检查官方应用后重试。";
    case "cancelled":
      return "认证会话已取消。";
    case "executor_not_implemented":
      return "FyAgent 暂时无法在当前环境中完成登录。";
  }
}

function errorReason(error: unknown): string {
  if (typeof error === "object" && error !== null && "reasonCode" in error) {
    const reason = (error as { reasonCode?: AgentAuthReasonCode }).reasonCode;
    if (reason) return reasonCopy(reason) ?? "认证操作未完成。";
  }
  return "认证操作未完成，请刷新状态后重试。";
}

function terminalTone(snapshot: AgentAuthSessionSnapshot): "info" | "warning" {
  return snapshot.stage === "verified" ? "info" : "warning";
}

export function AgentAuthStatusPanel(props: {
  agentId: AgentCatalogId;
  mode?: "compact" | "detail";
  enabled?: boolean;
}) {
  return (
    <AgentAuthStatusPanelInner
      key={`${props.agentId}:${props.enabled !== false ? "enabled" : "disabled"}`}
      {...props}
    />
  );
}

function AgentAuthStatusPanelInner({
  agentId,
  mode = "detail",
  enabled = true,
}: {
  agentId: AgentCatalogId;
  mode?: "compact" | "detail";
  enabled?: boolean;
}) {
  const { ports } = useFeatures();
  const observationQuery = useAgentAuthObservation(agentId, enabled);
  const desktopTarget = DESKTOP_HANDOFF_AGENTS.has(agentId);
  const inventoryQuery = useAgentInstallationInventory(
    agentId,
    enabled && mode === "detail" && desktopTarget,
  );
  const [selectedTarget, setSelectedTarget] =
    useState<AgentInstallationTarget | null>(null);
  const [selectionError, setSelectionError] = useState<string | null>(null);
  const lastTerminalSession = useRef<string | null>(null);
  const session = useAgentAuthSession({
    agentId,
    port: ports.agentAuth,
    enabled,
    onTerminal: (snapshot) => {
      if (lastTerminalSession.current === snapshot.sessionId) return;
      lastTerminalSession.current = snapshot.sessionId;
      void observationQuery.refetch();
    },
  });
  const targetOptions = useMemo(
    () =>
      inventoryQuery.data
        ? installationTargetsForAction(
            inventoryQuery.data,
            "auth_login",
          ).filter((target) => target.eligibleActions.includes("auth_login"))
        : [],
    [inventoryQuery.data],
  );
  const effectiveSelectedTarget =
    selectedTarget &&
    inventoryQuery.data &&
    selectedTarget.inventoryId === inventoryQuery.data.inventoryId &&
    targetOptions.some((option) => option.targetId === selectedTarget.targetId)
      ? selectedTarget
      : null;

  if (!enabled) return null;
  if (observationQuery.isPending) {
    return mode === "compact" ? (
      <span className="fy-agent-auth-compact" role="status">
        认证：读取中
      </span>
    ) : (
      <section className="fy-agent-section" aria-label="认证状态">
        <h3>认证状态</h3>
        <div className="fy-agent-auth-panel-loading">
          <Spinner label="正在读取认证状态" />
          <span>正在读取认证状态</span>
        </div>
      </section>
    );
  }
  if (observationQuery.isError || !observationQuery.data) {
    return mode === "compact" ? (
      <span className="fy-agent-auth-compact">认证：不可读取</span>
    ) : (
      <section className="fy-agent-section" aria-label="认证状态">
        <h3>认证状态</h3>
        <InlineNotice tone="warning">
          暂时无法检查登录状态。请重试。
        </InlineNotice>
        <Button onClick={() => void observationQuery.refetch()}>重试</Button>
      </section>
    );
  }

  const observation = session.snapshot?.observation ?? observationQuery.data;
  if (mode === "compact") {
    return (
      <span className="fy-agent-auth-compact" data-auth-kind={observation.kind}>
        认证：{observationSummary(observation)}
      </span>
    );
  }

  const runIntent = async (intent: AgentAuthIntent, providerId?: string) => {
    setSelectionError(null);
    let target: AgentInstallationTarget | null = null;
    if (desktopTarget && intent === "login") {
      target =
        targetOptions.length === 1
          ? targetOptions[0]
          : effectiveSelectedTarget
            ? (targetOptions.find(
                (option) =>
                  option.targetId === effectiveSelectedTarget.targetId,
              ) ?? null)
            : null;
      if (!target) {
        setSelectionError("请选择要登录的应用。");
        return;
      }
    }
    await session.start({
      agentId,
      intent,
      ...(providerId ? { providerId } : {}),
      ...(target
        ? {
            inventoryId: target.inventoryId,
            targetId: target.targetId,
            expectedTargetRevision: target.expectedTargetRevision,
          }
        : {}),
    });
  };

  const genericIntents = observation.allowedIntents.filter(
    (intent) =>
      !(observation.kind === "provider_connections" && intent === "logout"),
  );
  const refreshObservation = async () => {
    const result = await observationQuery.refetch();
    if (result.data) session.resetTerminal();
  };
  return (
    <section className="fy-agent-section" aria-label="认证状态">
      <div className="fy-agent-section-heading">
        <div>
          <h3>认证状态</h3>
          <p>{observationDescription(observation)}</p>
        </div>
        <Button
          disabled={session.busy}
          onClick={() => void refreshObservation()}
        >
          刷新状态
        </Button>
      </div>
      <div className="fy-agent-auth-panel" data-auth-kind={observation.kind}>
        <strong>{observationSummary(observation)}</strong>
        {observation.kind === "provider_connections" &&
        observation.providers.length > 0 ? (
          <div className="fy-agent-auth-provider-list">
            {observation.providers.map((provider) => (
              <div key={provider.providerId}>
                <span>{provider.label}</span>
                <Button
                  disabled={session.busy}
                  onClick={() => void runIntent("logout", provider.providerId)}
                >
                  断开
                </Button>
              </div>
            ))}
          </div>
        ) : null}
        {desktopTarget ? (
          <LifecycleTargetPicker
            id={`agent-auth-target-${agentId}`}
            action="auth_login"
            targets={targetOptions}
            value={effectiveSelectedTarget?.targetId ?? null}
            onChange={setSelectedTarget}
            loading={inventoryQuery.isPending}
            error={
              inventoryQuery.isError
                ? "暂时无法读取安装位置。请刷新后重试。"
                : null
            }
            disabled={session.busy}
            onRefresh={() => void inventoryQuery.refetch()}
          />
        ) : null}
        {selectionError ? (
          <InlineNotice tone="warning">{selectionError}</InlineNotice>
        ) : null}
        {genericIntents.length > 0 ? (
          <div className="fy-agent-action-row">
            {genericIntents.map((intent) => (
              <Button
                key={intent}
                disabled={session.busy}
                onClick={() => void runIntent(intent)}
              >
                {intentLabel(intent)}
              </Button>
            ))}
          </div>
        ) : null}
        {session.snapshot ? (
          <InlineNotice
            tone={
              isAgentAuthSessionTerminal(session.snapshot)
                ? terminalTone(session.snapshot)
                : "info"
            }
          >
            <span className="fy-agent-auth-session-copy">
              {!isAgentAuthSessionTerminal(session.snapshot) ? (
                <Spinner label={stageCopy(session.snapshot)} />
              ) : null}
              {stageCopy(session.snapshot)}
            </span>
          </InlineNotice>
        ) : null}
        {session.snapshot?.canStopWaiting ? (
          <Button
            disabled={session.submitting}
            onClick={() => void session.stopWaiting()}
          >
            停止等待
          </Button>
        ) : null}
        {session.snapshot?.reasonCode ? (
          <p className="fy-agent-auth-reason">
            {reasonCopy(session.snapshot.reasonCode)}
          </p>
        ) : null}
        {session.error ? (
          <InlineNotice tone="warning">
            {errorReason(session.error)}
          </InlineNotice>
        ) : null}
      </div>
    </section>
  );
}
