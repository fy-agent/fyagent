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

import { requestOpenAuthCenter } from "../../shared/features/auth-center-handoff";

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
      if (observation.authority !== "verified") return "账号状态尚未验证";
      if (observation.state === "logged_in") return "已验证登录";
      if (observation.state === "logged_out") return "已验证退出";
      return "账号状态未知";
    case "provider_connections":
      if (observation.authority !== "verified")
        return "Provider 连接状态尚未验证";
      return observation.providers.length === 0
        ? "尚未连接 Provider"
        : `已连接 ${observation.providers.length} 个 Provider`;
    case "handoff_only":
      return observation.agentId === "grokbuild"
        ? "仅支持打开官方认证入口（终端 grok login）"
        : "仅支持打开官方认证入口";
    case "fyagent_managed":
      return "由 FyAgent 认证中心管理";
    case "unavailable":
      return "当前无法读取认证状态";
  }
}

function observationDescription(observation: AgentAuthObservation): string {
  switch (observation.kind) {
    case "account":
      return observation.authority === "verified"
        ? "状态来自官方结构化命令的回读。"
        : "没有可确认的官方状态；不会把打开登录入口当作成功。";
    case "provider_connections":
      return "OpenCode 按 Provider 管理连接，不提供全局登录布尔值。";
    case "handoff_only":
      return observation.agentId === "grokbuild"
        ? "软件只能开门，不能验证是否已经登录，所以这里不会出现已登录。终端里自己运行 grok login / grok logout 也可以。这条路只给 Grok Build 自己用，不会写进 Codex。SuperGrok 扫码请去认证中心。"
        : "FyAgent 只能把操作交给官方应用或 CLI，无法验证最终账号状态。";
    case "fyagent_managed":
      return "Codex 托管账号请去认证中心管理。SuperGrok 扫码也在认证中心，不要在这里登录，也不要去终端跑 grok login。";
    case "unavailable":
      return "认证观察器不可用；不会读取厂商凭据文件或推断登录状态。";
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
      return "正在回读认证状态";
    case "verified":
      return "认证结果已验证";
    case "handoff_complete":
      return "已交给官方认证入口";
    case "failed":
      return "认证操作失败";
    case "cancelled":
      return "已停止等待认证结果";
    case "timed_out":
      return "认证验证等待超时";
  }
}

function reasonCopy(
  reason: AgentAuthReasonCode | null,
  agentId?: AgentCatalogId,
): string | null {
  switch (reason) {
    case null:
      return null;
    case "auth_state_unknown":
      return "官方认证状态仍未知。";
    case "auth_observer_unavailable":
      return "当前无法运行官方认证状态命令。";
    case "auth_output_invalid":
      return "官方状态输出无法安全解析。";
    case "interactive_user_unavailable":
      return "当前无法以交互用户身份打开认证入口。";
    case "operation_conflict":
      return "此软件已有认证会话正在进行。";
    case "provider_selection_required":
      return "请选择要断开的 Provider。";
    case "provider_changed":
      return "Provider 列表已变化，请刷新后重试。";
    case "monitoring_stopped":
      return "已停止等待；官方认证窗口不会因此被关闭。";
    case "timed_out":
      return "在限定时间内没有获得可验证结果。";
    case "handoff_only":
      return agentId === "grokbuild"
        ? "已打开终端入口。请完成 grok login 或 grok logout。没有权威状态可验证，不会显示已登录。"
        : "已完成入口交接，但没有权威状态可验证。";
    case "managed_by_auth_center":
      return "请在现有认证中心管理此账号。SuperGrok 扫码也在那里，不是 grok login。";
    case "target_selection_required":
      return "检测到多份安装，请选择认证目标。";
    case "target_changed":
    case "inventory_expired":
      return "安装目标已变化，请刷新后重试。";
    case "target_not_executable":
      return "所选安装当前不可启动。";
    case "command_failed":
      return "官方认证命令未成功完成。";
    case "cancelled":
      return "认证会话已取消。";
    case "executor_not_implemented":
      return "当前认证动作尚不可执行。";
  }
}

function errorReason(error: unknown, agentId?: AgentCatalogId): string {
  if (typeof error === "object" && error !== null && "reasonCode" in error) {
    const reason = (error as { reasonCode?: AgentAuthReasonCode }).reasonCode;
    if (reason) return reasonCopy(reason, agentId) ?? "认证操作未完成。";
  }
  return "认证操作未完成，请刷新状态后重试。";
}

function terminalTone(snapshot: AgentAuthSessionSnapshot): "info" | "warning" {
  return snapshot.stage === "verified" || snapshot.stage === "handoff_complete"
    ? "info"
    : "warning";
}

function canRefreshObservation(observation: AgentAuthObservation): boolean {
  return (
    observation.kind === "account" ||
    observation.kind === "provider_connections"
  );
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
          当前无法读取认证状态，不会推断为已登录。
        </InlineNotice>
        <Button onClick={() => void observationQuery.refetch()}>
          重新读取
        </Button>
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
        setSelectionError("请选择要打开认证入口的安装目标。");
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
        <div className="fy-agent-action-row">
          {observation.agentId === "grokbuild" ||
          observation.agentId === "workbuddy" ||
          observation.kind === "fyagent_managed" ? (
            <Button onClick={() => requestOpenAuthCenter()}>
              打开认证中心
            </Button>
          ) : null}
          {canRefreshObservation(observation) ? (
            <Button
              disabled={session.busy}
              onClick={() => void refreshObservation()}
            >
              刷新状态
            </Button>
          ) : null}
        </div>
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
                ? "当前无法读取安装目标；不会猜测要打开哪一份安装。"
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
            {reasonCopy(session.snapshot.reasonCode, agentId)}
          </p>
        ) : null}
        {session.error ? (
          <InlineNotice tone="warning">
            {errorReason(session.error, agentId)}
          </InlineNotice>
        ) : null}
      </div>
    </section>
  );
}
