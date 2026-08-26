import { useEffect, useState } from "react";

import type {
  AgentActionId,
  AgentActionJobStage,
  AgentInstallReadiness,
  AgentInstallReadinessPort,
  AgentInstallState,
  AgentUpdateState,
} from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";
import { Button, InlineNotice, Spinner } from "../../shared/ui/primitives";

import {
  jobStageCopy,
  reasonCopy,
  useAgentLifecycleAction,
} from "./useAgentLifecycleAction";

type ReadinessPort = AgentInstallReadinessPort;

const NATIVE_ONLY_COPY = "安装准备度仅在桌面应用接线后可读取";

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
  const lifecycle = useAgentLifecycleAction({
    agentId,
    port,
    readiness: state.status === "ready" ? state.data : null,
    onReadinessChange: (data) => {
      setState({ status: "ready", data });
    },
  });

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
            busy={lifecycle.busy}
            jobStage={lifecycle.stage}
            error={lifecycle.error}
            success={lifecycle.success}
            onAction={(action) => void lifecycle.run(action)}
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
