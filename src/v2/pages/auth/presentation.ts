import { errorMessage } from "../../shared/features/helpers";
import {
  parseManagedAuthCommandError,
  type ManagedAuthAccountSummary,
  type ManagedAuthConnectionAction,
  type ManagedAuthConnectionState,
  type ManagedAuthConnectionSummary,
  type ManagedAuthConsumer,
  type ManagedAuthCredentialManager,
  type ManagedAuthHealth,
  type ManagedAuthLoginSessionSnapshot,
  type ManagedAuthLoginStage,
  type ManagedAuthProvider,
  type ManagedAuthReasonCode,
  type ManagedAuthRequestMode,
} from "../../shared/features/managed-auth";

export type AuthTone = "neutral" | "accent" | "warning";

const providerLabels: Record<ManagedAuthProvider, string> = {
  openai: "OpenAI",
  xai: "xAI",
  github_copilot: "GitHub Copilot",
};

const consumerLabels: Record<ManagedAuthConsumer, string> = {
  codex: "Codex",
  grokbuild: "Grok Build",
  opencode: "OpenCode Desktop",
  fyagent_proxy: "FyAgent Local Proxy",
};

const managerLabels: Record<ManagedAuthCredentialManager, string> = {
  fyagent: "由 FyAgent 自动续期",
  codex: "由 Codex 自动续期",
  grokbuild: "由 Grok Build 自动续期",
  opencode: "由 OpenCode 自动续期",
  unavailable: "暂时无法确认续期方式",
};

const reasonCopies: Record<ManagedAuthReasonCode, string> = {
  native_only: "此状态只能在 FyAgent 桌面应用中读取。",
  observer_unavailable: "暂时无法读取最新状态，请稍后重试。",
  operation_conflict: "已有账号操作正在进行，请先完成或取消。",
  requires_reauth: "登录凭据已失效，需要重新登录。",
  migration_blocked: "旧账号数据尚未完成安全迁移。",
  secret_unavailable: "系统凭据库暂时不可用。",
  connection_unavailable: "暂时无法确认软件连接状态。",
  native_projection_unavailable:
    "账号已保存在 FyAgent。还不能改写该软件的本地登录和模型来源，所以本机配置不会变。",
  target_selection_required: "检测到多个安装实例，请先选择要管理的软件。",
  target_changed: "软件安装状态已变化，请刷新后重试。",
  pending_restart: "凭据已更新，软件需要重新启动后才能使用。",
  external_change_detected:
    "检测到软件在 FyAgent 外部修改了登录信息，请刷新确认。",
  provider_not_supported: "此官方账号暂时不能连接到目标软件。",
  callback_unavailable: "本地回调不可用，可以改用设备码登录。",
  device_code_expired: "设备码已过期，请生成新的设备码。",
  identity_mismatch: "登录的不是原账号，请返回后单独添加。",
  partial_completion: "账号已保存，但仍有软件连接需要处理。",
  cancelled: "登录已取消。",
  timed_out: "等待官方登录结果超时。",
  login_failed: "官方登录未完成，请重试。",
  invalid_response: "无法识别返回的账号状态，请更新 FyAgent 后重试。",
};

export function managedAuthProviderLabel(
  provider: ManagedAuthProvider,
): string {
  return providerLabels[provider];
}

export function managedAuthConsumerLabel(
  consumer: ManagedAuthConsumer,
): string {
  return consumerLabels[consumer];
}

export function managedAuthManagerLabel(
  manager: ManagedAuthCredentialManager,
): string {
  return managerLabels[manager];
}

export function managedAuthReasonCopy(
  reason: ManagedAuthReasonCode | null,
): string | null {
  return reason === null ? null : reasonCopies[reason];
}

export function managedAuthCommandErrorCopy(error: unknown): string {
  const reason = parseManagedAuthCommandError(error);
  return reason === null ? errorMessage(error) : reasonCopies[reason];
}

export function uniqueManagedAuthReasonCopies(
  reasons: ManagedAuthReasonCode[],
): string[] {
  const copies: string[] = [];
  const seen = new Set<string>();
  for (const reason of reasons) {
    const copy = managedAuthReasonCopy(reason);
    if (copy === null || seen.has(copy)) continue;
    seen.add(copy);
    copies.push(copy);
  }
  return copies;
}

export function accountHealthPresentation(health: ManagedAuthHealth): {
  label: string;
  tone: AuthTone;
} {
  switch (health) {
    case "ready":
      return { label: "正常", tone: "accent" };
    case "checking":
      return { label: "正在确认", tone: "neutral" };
    case "requires_reauth":
      return { label: "需要重新登录", tone: "warning" };
    case "migration_blocked":
      return { label: "需要完成迁移", tone: "warning" };
    case "unavailable":
      return { label: "状态不可用", tone: "warning" };
  }
}

export function connectionStatusPresentation(
  state: ManagedAuthConnectionState,
  reasonCodes: readonly ManagedAuthReasonCode[] = [],
): { label: string; tone: AuthTone } {
  if (
    state === "connected" &&
    reasonCodes.includes("native_projection_unavailable")
  ) {
    return { label: "账号已保存", tone: "warning" };
  }
  switch (state) {
    case "connected":
      return { label: "已连接", tone: "accent" };
    case "disconnected":
      return { label: "未连接", tone: "neutral" };
    case "checking":
      return { label: "正在确认", tone: "neutral" };
    case "requires_reauth":
      return { label: "需要重新登录", tone: "warning" };
    case "pending_restart":
      return { label: "等待重启", tone: "warning" };
    case "unavailable":
      return { label: "状态不可用", tone: "warning" };
  }
}

export function requestModeLabel(
  mode: ManagedAuthRequestMode,
  providerLabel: string | null,
): string {
  if (providerLabel) return providerLabel;
  switch (mode) {
    case "official_subscription":
      return "官方订阅";
    case "third_party_api":
      return "第三方 API";
    case "provider_connections":
      return "Provider 连接";
    case "none":
      return "未配置";
    case "unknown":
      return "暂时无法确认";
  }
}

export function formatAuthenticatedAt(value: string | null): string {
  if (!value) return "暂时没有记录";
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return "暂时没有记录";
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

const accountHealthRank: Record<ManagedAuthHealth, number> = {
  requires_reauth: 0,
  migration_blocked: 1,
  unavailable: 2,
  checking: 3,
  ready: 4,
};

export function sortManagedAuthAccounts(
  accounts: ManagedAuthAccountSummary[],
  connections: ManagedAuthConnectionSummary[],
  preferredConsumer: ManagedAuthConsumer | null,
): ManagedAuthAccountSummary[] {
  const preferredAccountIds = new Set(
    preferredConsumer === null
      ? []
      : connections
          .filter(
            (connection) =>
              connection.consumer === preferredConsumer &&
              connection.accountId !== null,
          )
          .map((connection) => connection.accountId as string),
  );
  return accounts.slice().sort((left, right) => {
    const health =
      accountHealthRank[left.health] - accountHealthRank[right.health];
    if (health !== 0) return health;
    const preferred =
      Number(preferredAccountIds.has(right.accountId)) -
      Number(preferredAccountIds.has(left.accountId));
    if (preferred !== 0) return preferred;
    const defaults = Number(right.isDefault) - Number(left.isDefault);
    if (defaults !== 0) return defaults;
    const leftTime = left.lastAuthenticatedAt
      ? Date.parse(left.lastAuthenticatedAt)
      : 0;
    const rightTime = right.lastAuthenticatedAt
      ? Date.parse(right.lastAuthenticatedAt)
      : 0;
    if (leftTime !== rightTime) return rightTime - leftTime;
    return left.login.localeCompare(right.login);
  });
}

export function loginStagePresentation(stage: ManagedAuthLoginStage): {
  title: string;
  description: string;
} {
  switch (stage) {
    case "preparing":
      return {
        title: "正在准备官方登录",
        description: "正在创建受控的登录会话。",
      };
    case "opening_browser":
      return {
        title: "正在打开官方登录页",
        description: "请在浏览器中继续。",
      };
    case "awaiting_user":
      return {
        title: "等待你完成官方登录",
        description: "此窗口可以暂时关闭，登录会在后台继续。",
      };
    case "exchanging_code":
      return {
        title: "已收到授权，正在验证",
        description: "正在向官方服务确认本次登录。",
      };
    case "saving_account":
      return {
        title: "正在安全保存账号",
        description: "账号凭据不会显示在此页面。",
      };
    case "connecting_consumer":
      return {
        title: "正在连接软件",
        description: "正在更新目标软件的登录连接。",
      };
    case "verifying":
      return {
        title: "正在确认最终状态",
        description: "只有软件重新读取成功后才会显示完成。",
      };
    case "completed":
      return {
        title: "账号已添加并连接",
        description: "最终状态已经确认。",
      };
    case "partial":
      return {
        title: "账号已安全保存",
        description: "仍有软件连接需要处理。",
      };
    case "failed":
      return {
        title: "登录未完成",
        description: "没有确认成功的连接不会被标记为已完成。",
      };
    case "cancelled":
      return {
        title: "登录已取消",
        description: "未完成的登录不会改变现有账号连接。",
      };
    case "expired":
      return {
        title: "登录已过期",
        description: "请重新发起登录。",
      };
  }
}

export function connectionActionLabel(
  action: ManagedAuthConnectionAction,
): string {
  switch (action) {
    case "connect_account":
      return "连接账号";
    case "switch_account":
      return "切换账号";
    case "disconnect":
      return "断开";
    case "refresh":
      return "刷新状态";
    case "restart":
      return "立即重启";
    case "open_consumer":
      return "打开软件";
    case "switch_to_official":
      return "切回官方";
  }
}

export function accountPageConnectionActionLabel(
  action: ManagedAuthConnectionAction,
): string {
  switch (action) {
    case "connect_account":
      return "用此账号连接";
    case "switch_account":
      return "切换到此账号";
    case "switch_to_official":
      return "切回官方";
    default:
      return connectionActionLabel(action);
  }
}

const ACCOUNT_PAGE_CONNECTION_ACTIONS: ManagedAuthConnectionAction[] = [
  "switch_to_official",
  "connect_account",
  "switch_account",
];

export function accountPageConnectionActions(
  connection: ManagedAuthConnectionSummary,
  accountId: string,
): ManagedAuthConnectionAction[] {
  return ACCOUNT_PAGE_CONNECTION_ACTIONS.filter((action) => {
    if (!connection.allowedActions.includes(action)) return false;
    if (action === "switch_to_official") {
      return connection.accountId === accountId;
    }
    return connection.accountId !== accountId;
  });
}

export function connectableConnectionsForAccount(
  account: ManagedAuthAccountSummary,
  connections: ManagedAuthConnectionSummary[],
): ManagedAuthConnectionSummary[] {
  if (account.health !== "ready") return [];
  return connections.filter((connection) => {
    if (connection.provider !== account.provider) return false;
    if (connection.accountId === account.accountId) return false;
    return (
      connection.allowedActions.includes("connect_account") ||
      connection.allowedActions.includes("switch_account")
    );
  });
}

export function loginRequiredConnectionsForAccount(
  account: ManagedAuthAccountSummary,
  connections: ManagedAuthConnectionSummary[],
): ManagedAuthConnectionSummary[] {
  if (account.health !== "ready") return [];
  const connectableIds = new Set(
    connectableConnectionsForAccount(account, connections).map(
      (connection) => connection.connectionId,
    ),
  );
  return connections.filter((connection) => {
    if (connection.provider !== account.provider) return false;
    if (connection.accountId !== null) return false;
    if (connectableIds.has(connection.connectionId)) return false;
    return true;
  });
}

export function sessionSummary(
  session: ManagedAuthLoginSessionSnapshot,
): string {
  const stage = loginStagePresentation(session.stage);
  const reason = managedAuthReasonCopy(session.reasonCode);
  return reason ? `${stage.title}：${reason}` : stage.title;
}
