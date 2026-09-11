import {
  AGENT_CATALOG_IDS,
  PRODUCT_DIRECTORY,
  type AgentCatalogId,
} from "./directory";
import {
  HEALTH_STALE_AFTER_MS,
  type AgentHealthSnapshot,
  type HealthAction,
  type HealthCheckId,
  type HealthCheckState,
  type HealthReasonCode,
  type HealthStatus,
} from "./health";

export const HEALTH_STATUS_LABELS: Record<HealthStatus, string> = {
  ready: "本机检查正常",
  needs_attention: "需要处理",
  blocked: "暂不可用",
  not_configured: "尚未配置",
  stale: "需要重新检查",
};
export const HEALTH_STATE_LABELS: Record<HealthCheckState, string> = {
  ok: "正常",
  attention: "需要处理",
  blocked: "暂不可用",
  not_configured: "尚未配置",
  unknown: "待确认",
  not_supported: "暂不支持检查",
};
export const HEALTH_CHECK_LABELS: Record<HealthCheckId, string> = {
  installation: "版本与安装来源",
  helper: "安装助手",
  conflicts: "安装冲突",
  configuration: "配置文件",
  secret: "凭据状态",
  endpoint: "服务地址",
  auth: "登录状态",
  model: "当前模型",
  drift: "配置一致性",
  proxy: "本机转发",
  restart: "重启需求",
  last_request: "最近一次请求",
};
export const HEALTH_REASON_LABELS: Record<HealthReasonCode, string> = {
  installation_found: "已找到本机安装。",
  installation_not_found: "尚未找到安装，请先安装软件。",
  installation_unknown: "暂时无法确认安装位置，请到软件页面检查。",
  installation_not_runnable: "已找到安装，但当前无法运行。",
  multiple_installations: "发现多个安装，请确认要使用哪一个。",
  installation_conflict: "安装来源存在冲突，请检查安装位置。",
  single_installation: "未发现多个安装造成的冲突。",
  helper_available: "安装助手可用。",
  helper_unavailable: "安装助手暂不可用，请到安装页面处理。",
  helper_not_required: "当前安装方式无需安装助手。",
  configuration_present: "已找到可读取的配置。",
  configuration_missing: "尚未找到配置，请先完成模型配置。",
  configuration_unreadable: "无法读取配置，请检查文件格式。",
  credential_available: "已找到所需凭据，远端有效性尚未测试。",
  credential_missing: "缺少所需凭据，请登录或补充 API Key。",
  credential_unknown: "暂时无法确认凭据，请检查账号或模型配置。",
  credential_not_required: "当前配置无需单独保存凭据。",
  endpoint_configured: "服务地址已配置，连通性尚未测试。",
  endpoint_missing: "尚未配置服务地址。",
  endpoint_invalid: "服务地址格式有误，请检查模型配置。",
  endpoint_not_checked: "本次检查无法确认服务地址。",
  auth_logged_in: "本机记录显示已登录，远端会话有效性尚未测试。",
  auth_logged_out: "本机记录显示未登录，请先登录。",
  auth_unknown: "本次本机检查无法确认登录状态。",
  auth_managed: "本机认证材料已配置，服务端授权仍需实际请求确认。",
  auth_handoff: "登录由软件管理，请在软件中确认。",
  model_configured: "已设置当前模型，实际请求可用性尚未测试。",
  model_missing: "尚未选择模型。",
  model_unknown: "暂时无法确认当前模型。",
  configuration_in_sync: "当前请求来源与已保存设置一致。",
  configuration_drifted: "配置已在其他位置改变，请检查后再使用。",
  configuration_drift_unknown: "缺少可比较的配置，暂时无法确认是否发生变化。",
  proxy_running: "当前配置使用的本机转发正在运行。",
  proxy_stopped: "当前配置需要本机转发，但转发尚未运行。",
  proxy_not_used: "当前配置无需本机转发。",
  proxy_unknown: "暂时无法确认本机转发状态。",
  restart_required: "配置已改变，请重启软件或新建会话后再试。",
  restart_not_required: "未发现待完成的重启要求。",
  restart_unknown: "暂时无法确认是否需要重启。",
  request_succeeded: "最近一次已记录的请求成功；不代表当前额度或服务仍然可用。",
  request_failed: "最近一次已记录的请求失败，可到模型配置中测试。",
  request_not_recorded:
    "尚无此软件的本机代理请求记录，可在模型配置中自行测试。",
  not_supported: "暂不支持检查此项，请在软件中确认。",
  read_failed: "本次读取失败，请重新检查。",
  check_timeout: "本次检查超时，请重新检查。",
};
export const HEALTH_ACTION_LABELS: Record<HealthAction, string> = {
  installation: "查看安装",
  configuration: "检查配置",
  authentication: "查看登录",
  model_test: "前往模型测试",
  refresh: "重新检查",
};

const CORE_CHECKS: ReadonlySet<HealthCheckId> = new Set([
  "installation",
  "configuration",
  "secret",
  "auth",
  "model",
  "drift",
  "proxy",
]);

export function healthStatus(
  snapshot: AgentHealthSnapshot,
  now: number,
  readFailed = false,
): HealthStatus {
  if (
    readFailed ||
    now - Date.parse(snapshot.checkedAt) >= HEALTH_STALE_AFTER_MS
  )
    return "stale";
  if (snapshot.checks.some((check) => check.state === "blocked"))
    return "blocked";
  if (snapshot.checks.some((check) => check.state === "not_configured"))
    return "not_configured";
  if (
    snapshot.checks.some(
      (check) =>
        check.state === "attention" ||
        (check.state === "unknown" && CORE_CHECKS.has(check.id)),
    )
  )
    return "needs_attention";
  return "ready";
}

export function healthPrimaryReason(snapshot: AgentHealthSnapshot): string {
  const check =
    snapshot.checks.find((item) => item.state === "blocked") ??
    snapshot.checks.find((item) => item.state === "not_configured") ??
    snapshot.checks.find((item) => item.state === "attention") ??
    snapshot.checks.find(
      (item) => item.state === "unknown" && CORE_CHECKS.has(item.id),
    );
  return check
    ? HEALTH_REASON_LABELS[check.reasonCode]
    : "本次支持的本机检查已通过，远端服务可用性需单独测试。";
}

export function healthAgentFromSearch(search: string): AgentCatalogId | null {
  const values = new URLSearchParams(search).getAll("agent");
  return values.length === 1
    ? (AGENT_CATALOG_IDS.find((id) => id === values[0]) ?? null)
    : null;
}

/** Closed destinations only. Entering one never invokes a repair or a model. */
export function healthActionPath(
  action: HealthAction,
  agentId: AgentCatalogId,
): string | null {
  if (action === "refresh") return null;
  if (action === "installation") return "/agents";
  if (action === "authentication") {
    if (agentId === "codex" || agentId === "grokbuild") {
      return `/auth?view=connections&consumer=${agentId}`;
    }
    return `/agents?target=${agentId}&section=models`;
  }
  const target = PRODUCT_DIRECTORY.find(
    (entry) => entry.agentId === agentId,
  )?.modelTarget;
  return target ? `/models?target=${target}` : "/agents";
}
