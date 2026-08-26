import { useEffect, useState } from "react";

import {
  AGENT_REASON_CODES,
  type AgentActionId,
  type AgentActionJobStage,
  type AgentInstallReadiness,
  type AgentInstallReadinessPort,
  type AgentInstallState,
  type AgentReasonCode,
  type AgentUpdateState,
} from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";
import { Button, InlineNotice, Spinner } from "../../shared/ui/primitives";

type ReadinessPort = AgentInstallReadinessPort;

const NATIVE_ONLY_COPY = "安装准备度仅在桌面应用接线后可读取";
const JOB_POLL_MS = 800;
const MAX_JOB_POLLS = 2250;
const ACTION_INCOMPLETE_COPY = "操作未能完成。此区域不会推断安装成功。";
const ACTION_SUCCEEDED_COPY = "操作已完成。下面是再次读取的状态，不是推断。";

const unavailablePort: ReadinessPort = {
  get: async () => {
    throw new Error(NATIVE_ONLY_COPY);
  },
  startAction: async () => {
    throw new Error(NATIVE_ONLY_COPY);
  },
  cancelAction: async () => {
    throw new Error(NATIVE_ONLY_COPY);
  },
  getActionJob: async () => {
    throw new Error(NATIVE_ONLY_COPY);
  },
};

function installStateCopy(state: AgentInstallState): string {
  switch (state) {
    case "installed":
      return "已安装";
    case "not_installed":
      return "未安装";
    case "installed_not_runnable":
      return "已安装但不可运行";
    case "unavailable":
      return "当前环境不可用";
    case "unknown":
      return "未确认";
  }
}

function updateStateCopy(state: AgentUpdateState): string {
  switch (state) {
    case "up_to_date":
      return "已是最新";
    case "update_available":
      return "可更新";
    case "latest_unknown":
      return "远端版本未知";
    case "unavailable":
      return "更新不可用";
    case "unknown":
      return "未确认";
  }
}

function actionLabel(action: AgentActionId): string {
  switch (action) {
    case "install":
      return "安装";
    case "update":
      return "更新到最新版";
    case "launch":
      return "打开应用";
    case "auth_login":
      return "登录";
    case "auth_logout":
      return "退出登录";
    case "auth_connect_provider":
      return "连接 Provider";
  }
}

function reasonCopy(code: AgentReasonCode): string | null {
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

function jobStageCopy(stage: AgentActionJobStage): string {
  switch (stage) {
    case "checking":
      return "正在检查来源";
    case "downloading":
      return "正在下载安装包";
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

function isTerminalJobStage(stage: AgentActionJobStage): boolean {
  return stage === "succeeded" || stage === "failed" || stage === "cancelled";
}

function failureCopy(code: AgentReasonCode | null): string {
  return (code && reasonCopy(code)) || ACTION_INCOMPLETE_COPY;
}

function actionErrorReason(error: unknown): AgentReasonCode | null {
  if (!error || typeof error !== "object" || !("reasonCode" in error)) {
    return null;
  }
  const code = error.reasonCode;
  return typeof code === "string" &&
    (AGENT_REASON_CODES as readonly string[]).includes(code)
    ? (code as AgentReasonCode)
    : null;
}

function ReadinessSummary({
  data,
  busy,
  jobStage,
  error,
  success,
  onAction,
}: {
  data: AgentInstallReadiness;
  busy: boolean;
  jobStage: AgentActionJobStage | null;
  error: string | null;
  success: string | null;
  onAction: (action: AgentActionId) => void;
}) {
  const managedByCodex = data.reasonCodes.includes("managed_by_codex_desktop");
  const notices = data.reasonCodes
    .map(reasonCopy)
    .filter((text): text is string => Boolean(text));
  return (
    <>
      {notices.map((text) => (
        <p key={text} className="fy-agent-install-readiness-summary">
          {text}
        </p>
      ))}
      <dl className="fy-agent-install-readiness-grid">
        <div>
          <dt>安装状态</dt>
          <dd>{installStateCopy(data.installState)}</dd>
        </div>
        <div>
          <dt>更新</dt>
          <dd>{updateStateCopy(data.updateState)}</dd>
        </div>
        <div>
          <dt>本地版本</dt>
          <dd>{data.localVersion ?? "未确认"}</dd>
        </div>
        <div>
          <dt>远端版本</dt>
          <dd>{data.remoteVersion ?? "未确认"}</dd>
        </div>
      </dl>
      {busy && jobStage ? (
        <div className="fy-agent-install-readiness-loading">
          <Spinner label={jobStageCopy(jobStage)} />
          <span>{jobStageCopy(jobStage)}</span>
        </div>
      ) : null}
      {error ? <InlineNotice tone="warning">{error}</InlineNotice> : null}
      {success ? <InlineNotice tone="info">{success}</InlineNotice> : null}
      {!managedByCodex && data.allowedActions.length > 0 ? (
        <div className="fy-agent-action-row">
          {data.allowedActions.map((action) => (
            <Button
              key={action}
              disabled={busy}
              onClick={() => onAction(action)}
            >
              {actionLabel(action)}
            </Button>
          ))}
        </div>
      ) : null}
    </>
  );
}

function AgentInstallReadinessContent({
  agentId,
  port,
}: {
  agentId: AgentCatalogId;
  port: ReadinessPort;
}) {
  const [state, setState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: AgentInstallReadiness }
    | { status: "unavailable" }
  >({ status: "loading" });
  const [busy, setBusy] = useState(false);
  const [jobStage, setJobStage] = useState<AgentActionJobStage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void port.get(agentId).then(
      (data) => {
        if (active) setState({ status: "ready", data });
      },
      () => {
        if (active) setState({ status: "unavailable" });
      },
    );
    return () => {
      active = false;
    };
  }, [agentId, port]);

  const runAction = async (action: AgentActionId) => {
    if (state.status !== "ready") return;
    setBusy(true);
    setJobStage("checking");
    setError(null);
    setSuccess(null);
    try {
      const result = await port.startAction({
        agentId,
        action,
        expectedReleaseId: state.data.releaseId ?? undefined,
      });
      setJobStage(result.stage);
      if (result.jobId) {
        let snapshot = await port.getActionJob(result.jobId);
        setJobStage(snapshot.stage);
        for (let attempt = 0; attempt < MAX_JOB_POLLS; attempt += 1) {
          if (isTerminalJobStage(snapshot.stage)) {
            break;
          }
          await new Promise((resolve) =>
            window.setTimeout(resolve, JOB_POLL_MS),
          );
          snapshot = await port.getActionJob(result.jobId);
          setJobStage(snapshot.stage);
        }
        if (!isTerminalJobStage(snapshot.stage)) {
          setError("安装仍在进行。超时不会被当成成功或失败。");
        } else if (snapshot.stage === "succeeded") {
          setSuccess(ACTION_SUCCEEDED_COPY);
        } else {
          setError(failureCopy(snapshot.reasonCode));
        }
      } else if (result.stage === "succeeded") {
        setSuccess(ACTION_SUCCEEDED_COPY);
      } else {
        setError(failureCopy(result.reasonCode));
      }
      const data = await port.get(agentId);
      setState({ status: "ready", data });
    } catch (error) {
      setError(failureCopy(actionErrorReason(error)));
    } finally {
      setBusy(false);
      setJobStage(null);
    }
  };

  return (
    <section className="fy-agent-section" aria-label="安装方式">
      <h3>安装方式</h3>
      <div className="fy-agent-install-readiness">
        {state.status === "loading" ? (
          <div className="fy-agent-install-readiness-loading">
            <Spinner label="正在读取安装准备度" />
            <span>正在读取安装准备度</span>
          </div>
        ) : state.status === "unavailable" ? (
          <InlineNotice tone="warning">
            当前无法读取安装准备度。此区域不会推断安装可用性。
          </InlineNotice>
        ) : (
          <ReadinessSummary
            data={state.data}
            busy={busy}
            jobStage={jobStage}
            error={error}
            success={success}
            onAction={(action) => void runAction(action)}
          />
        )}
      </div>
    </section>
  );
}

export function AgentInstallReadinessSection({
  agentId,
  port = unavailablePort,
}: {
  agentId: AgentCatalogId;
  port?: ReadinessPort;
}) {
  return (
    <AgentInstallReadinessContent key={agentId} agentId={agentId} port={port} />
  );
}
