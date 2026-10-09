import type {
  ManagedAuthConnectionSummary,
  ManagedAuthOverview,
} from "../../shared/features/managed-auth";
import {
  accountHealthPresentation,
  connectionStatusPresentation,
  type AuthTone,
} from "./presentation";

type SummaryState = ManagedAuthConnectionSummary["authStatus"] | "saved";

function connectionSummaryState(
  connection: ManagedAuthConnectionSummary,
): SummaryState {
  if (connection.authStatus === "connected") {
    if (connection.reasonCodes.includes("native_projection_unavailable"))
      return "saved";
    if (connection.pendingRestart) return "pending_restart";
  }
  if (connection.authStatus === "disconnected" && connection.accountId !== null)
    return "saved";
  return connection.authStatus;
}

function statePresentation(state: SummaryState) {
  return state === "saved"
    ? { label: "账号已保存", tone: "warning" as const }
    : connectionStatusPresentation(state);
}

export function connectionPresentation(
  connection: ManagedAuthConnectionSummary,
) {
  return statePresentation(connectionSummaryState(connection));
}

export function summarizeConnections(
  connections: readonly ManagedAuthConnectionSummary[],
) {
  const total = connections.length;
  const states = connections.map(connectionSummaryState);
  const connected = states.filter((state) => state === "connected").length;
  const needsAttention = states.some(
    (state) => statePresentation(state).tone === "warning",
  );
  const countsLabel = `${total} 个连接位置 · ${connected} 个已连接`;
  let label: string;
  let tone: AuthTone = "neutral";
  const remaining = [
    ...new Set(states.filter((state) => state !== "connected")),
  ];
  if (total === 0) {
    label = "暂时没有可管理的连接";
  } else if (connected === total) {
    label = `已连接 ${connected}/${total}`;
    tone = "accent";
  } else if (connected > 0) {
    label = `部分连接 ${connected}/${total}`;
    tone = needsAttention ? "warning" : "neutral";
  } else {
    const priority: SummaryState[] = [
      "requires_reauth",
      "pending_restart",
      "unavailable",
      "checking",
      "saved",
      "disconnected",
    ];
    const primary =
      priority.find((state) => states.includes(state)) ?? "unavailable";
    ({ label, tone } = statePresentation(primary));
  }
  const remainingLabels = remaining
    .map((state) => statePresentation(state).label)
    .filter((item) => item !== label);
  const primarySummary =
    label === "账号已保存" ? "账号已保存，尚未确认软件连接" : label;
  const summary =
    remainingLabels.length > 0
      ? `${primarySummary} · ${remainingLabels.join("、")}`
      : primarySummary;
  return {
    total,
    connected,
    countsLabel,
    summary,
    label,
    tone,
    needsAttention,
  };
}

export function summarizeAuthOverview(overview: ManagedAuthOverview) {
  const connections = summarizeConnections(overview.connections);
  const accountStates = [
    ...new Set(
      overview.accounts
        .filter((account) => account.health !== "ready")
        .map((account) => account.health),
    ),
  ];
  const accountsLabel =
    overview.accounts.length === 0
      ? "尚未保存受管账号"
      : accountStates.length > 0
        ? `受管账号：${accountStates.map((state) => accountHealthPresentation(state).label).join("、")}`
        : "受管账号状态正常";
  const unknownSources = overview.connections.filter(
    (connection) => connection.requestMode === "unknown",
  ).length;
  const sourceLabel =
    unknownSources > 0
      ? ` · ${unknownSources} 个位置的请求来源暂时无法确认`
      : "";
  return {
    connections,
    label: `${accountsLabel} · ${connections.summary}${sourceLabel}`,
    needsAttention:
      accountStates.length > 0 ||
      connections.needsAttention ||
      overview.reasonCodes.length > 0,
  };
}
