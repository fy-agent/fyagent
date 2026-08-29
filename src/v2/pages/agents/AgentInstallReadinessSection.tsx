import { useEffect, useMemo, useState, type ReactNode } from "react";

import type {
  AgentActionId,
  AgentActionJobStage,
  AgentInstallationInventory,
  AgentInstallationTarget,
  AgentInstallReadiness,
  AgentInstallReadinessPort,
  AgentInstallState,
  AgentUpdateState,
} from "../../shared/features/agent-install-readiness";
import { installationTargetsForAction } from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";
import { LifecycleTargetPicker } from "../../shared/ui/LifecycleTargetPicker";
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
  getInventory: async () => {
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

function inventoryStateCopy(
  state: AgentInstallReadiness["inventoryState"],
): string {
  switch (state) {
    case "not_observed":
      return "未发现";
    case "single":
      return "单一安装";
    case "multiple":
      return "多份安装";
    case "unsupported":
      return "当前平台未支持";
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
  targetPicker,
  onAction,
}: {
  data: AgentInstallReadiness;
  busy: boolean;
  jobStage: AgentActionJobStage | null;
  error: string | null;
  success: string | null;
  targetPicker?: ReactNode;
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
          <dt>安装清单</dt>
          <dd>{inventoryStateCopy(data.inventoryState)}</dd>
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
      {targetPicker}
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
  const [inventoryState, setInventoryState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: AgentInstallationInventory }
    | { status: "unavailable" }
  >({ status: "loading" });
  const [targetAction, setTargetAction] = useState<AgentActionId | null>(null);
  const [selectedTarget, setSelectedTarget] =
    useState<AgentInstallationTarget | null>(null);

  const targetOptions = useMemo(
    () =>
      targetAction && inventoryState.status === "ready"
        ? installationTargetsForAction(inventoryState.data, targetAction)
        : [],
    [inventoryState, targetAction],
  );

  const lifecycle = useAgentLifecycleAction({
    agentId,
    port,
    readiness: state.status === "ready" ? state.data : null,
    target: selectedTarget,
    onReadinessChange: (data) => {
      setState({ status: "ready", data });
    },
    onInventoryChange: (data) => {
      setInventoryState({ status: "ready", data });
      setSelectedTarget((current) =>
        current?.inventoryId === data.inventoryId ? current : null,
      );
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

  useEffect(() => {
    let active = true;
    void port.getInventory(agentId).then(
      (data) => {
        if (active) setInventoryState({ status: "ready", data });
      },
      () => {
        if (active) setInventoryState({ status: "unavailable" });
      },
    );
    return () => {
      active = false;
    };
  }, [agentId, port]);

  const refreshInventory = () => {
    setInventoryState({ status: "loading" });
    setSelectedTarget(null);
    void port.getInventory(agentId).then(
      (data) => setInventoryState({ status: "ready", data }),
      () => setInventoryState({ status: "unavailable" }),
    );
  };

  const handleAction = (action: AgentActionId) => {
    if (state.status !== "ready") return;
    const needsTarget =
      action === "install" ||
      action === "update" ||
      (state.data.requiresTargetSelection &&
        (action === "launch" || action === "auth_login"));
    if (!needsTarget) {
      void lifecycle.run(action, null);
      return;
    }
    setTargetAction(action);
    if (inventoryState.status !== "ready") return;
    const options = installationTargetsForAction(inventoryState.data, action);
    const eligible = options.filter((target) =>
      target.eligibleActions.includes(action),
    );
    const current = selectedTarget
      ? eligible.find((target) => target.targetId === selectedTarget.targetId)
      : undefined;
    if (current) {
      void lifecycle.run(action, current);
      return;
    }
    if (eligible.length === 1) {
      setSelectedTarget(eligible[0]);
      void lifecycle.run(action, eligible[0]);
      return;
    }
    setSelectedTarget(null);
  };

  const targetPicker = targetAction ? (
    <>
      <LifecycleTargetPicker
        id={`agent-lifecycle-target-${agentId}-${targetAction}`}
        action={targetAction}
        targets={targetOptions}
        value={selectedTarget?.targetId ?? null}
        onChange={setSelectedTarget}
        loading={inventoryState.status === "loading"}
        error={
          inventoryState.status === "unavailable"
            ? "当前无法读取安装目标。不会退回到路径猜测。"
            : null
        }
        disabled={lifecycle.busy}
        onRefresh={refreshInventory}
      />
      {targetOptions.length > 1 && !selectedTarget ? (
        <p className="fy-agent-install-readiness-note">
          请选择目标，然后再次点击“{actionLabel(targetAction)}”。
        </p>
      ) : null}
    </>
  ) : null;

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
            targetPicker={targetPicker}
            onAction={handleAction}
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
