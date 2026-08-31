import { useEffect, useMemo, useState, type ReactNode } from "react";

import type {
  AgentActionId,
  AgentActionJobStage,
  AgentInstallationInventory,
  AgentInstallationTarget,
  AgentInstallReadiness,
  AgentInstallReadinessPort,
  AgentInstallState,
  AgentReasonCode,
  AgentSurfaceReadiness,
  AgentUpdateState,
} from "../../shared/features/agent-install-readiness";
import { installationTargetsForAction } from "../../shared/features/agent-install-readiness";
import {
  grokLatestLabel,
  grokOwnerCopy,
  type GrokToolSnapshot,
  type GrokToolingPort,
} from "../../shared/features/grok-tooling";
import type { AgentCatalogId } from "../../shared/features/types";
import { LifecycleTargetPicker } from "../../shared/ui/LifecycleTargetPicker";
import { Button, InlineNotice, Spinner } from "../../shared/ui/primitives";

import {
  jobStageCopy,
  reasonCopy,
  useAgentLifecycleAction,
} from "./useAgentLifecycleAction";

type ReadinessPort = AgentInstallReadinessPort;
type SurfaceKind = "cli" | "desktop";

const NATIVE_ONLY_COPY = "安装状态仅可在 FyAgent 桌面应用中读取";
const LAUNCH_COPY = "打开软件";
const LIFECYCLE_ACTION_ORDER = ["install", "update", "launch"] as const;

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

type InventorySlot =
  | { status: "loading" }
  | { status: "ready"; data: AgentInstallationInventory }
  | { status: "unavailable" };

type SurfaceProjection = {
  surface: SurfaceKind;
  title: string;
  showTitle: boolean;
  localVersion: string | null;
  remoteVersion: string | null;
  installState: AgentInstallState;
  updateState: AgentUpdateState;
  inventoryState: AgentInstallReadiness["inventoryState"];
  allowedActions: AgentActionId[];
  reasonCodes: AgentReasonCode[];
  requiresTargetSelection: boolean;
  releaseId: string | null;
  sourceKind: AgentInstallReadiness["sourceKind"];
  hideLaunch: boolean;
  emptyNote: string | null;
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
      return "暂时无法确认";
  }
}

function updateStateCopy(state: AgentUpdateState): string {
  switch (state) {
    case "up_to_date":
      return "已是最新";
    case "update_available":
      return "可更新";
    case "latest_unknown":
      return "暂时无法检查";
    case "unavailable":
      return "更新不可用";
    case "unknown":
      return "暂时无法确认";
  }
}

function inventoryStateCopy(
  state: AgentInstallReadiness["inventoryState"],
): string {
  switch (state) {
    case "not_observed":
      return "未发现安装";
    case "single":
      return "找到 1 个安装";
    case "multiple":
      return "找到多个安装";
    case "unsupported":
      return "当前平台未支持";
    case "unknown":
      return "暂时无法确认";
  }
}

function actionLabel(
  action: AgentActionId,
  surface: SurfaceKind | "product",
): string {
  switch (action) {
    case "install":
      return "安装";
    case "update":
      return surface === "desktop" ? "更新当前位置" : "更新到最新版";
    case "launch":
      return LAUNCH_COPY;
    case "auth_login":
      return "登录";
    case "auth_logout":
      return "退出登录";
    case "auth_connect_provider":
      return "连接 Provider";
  }
}

function isLifecycleAction(
  action: AgentActionId,
): action is "install" | "update" | "launch" {
  return action === "install" || action === "update" || action === "launch";
}

function orderedLifecycleActions(
  allowed: readonly AgentActionId[],
  hideLaunch: boolean,
): Array<"install" | "update" | "launch"> {
  return LIFECYCLE_ACTION_ORDER.filter((action) => {
    if (!allowed.includes(action) || !isLifecycleAction(action)) return false;
    if (hideLaunch && action === "launch") return false;
    return true;
  });
}

function componentTitle(
  agentId: AgentCatalogId,
  surface: SurfaceKind,
): { title: string; showTitle: boolean } {
  if (agentId === "claude-code") {
    return { title: "Claude Desktop", showTitle: true };
  }
  return {
    title: surface === "cli" ? "命令行" : "桌面应用",
    showTitle: false,
  };
}

function projectionFromSurface(
  agentId: AgentCatalogId,
  item: AgentSurfaceReadiness,
): SurfaceProjection {
  const { title, showTitle } = componentTitle(agentId, item.surface);
  return {
    surface: item.surface,
    title,
    showTitle,
    localVersion: item.localVersion,
    remoteVersion: item.remoteVersion,
    installState: item.installState,
    updateState: item.updateState,
    inventoryState: item.inventoryState,
    allowedActions: item.allowedActions,
    reasonCodes: item.reasonCodes,
    requiresTargetSelection: item.requiresTargetSelection,
    releaseId: item.releaseId,
    sourceKind: item.sourceKind,
    hideLaunch: item.surface === "cli",
    emptyNote:
      item.surface === "desktop" &&
      item.installState === "not_installed" &&
      !item.allowedActions.includes("install") &&
      !item.allowedActions.includes("launch")
        ? "未发现桌面应用"
        : null,
  };
}

function detectSurfaces(
  agentId: AgentCatalogId,
  data: AgentInstallReadiness,
): SurfaceProjection[] | null {
  if (!data.surfaces || data.surfaces.length === 0) return null;
  return data.surfaces.map((item) => projectionFromSurface(agentId, item));
}

function surfacesForProduct(
  agentId: AgentCatalogId,
  data: AgentInstallReadiness,
): SurfaceProjection[] {
  const detected = detectSurfaces(agentId, data);
  const hideLaunch =
    data.sourceKind === "cli_tooling" ||
    (detected?.[0]?.surface ?? "desktop") === "cli";
  if (detected?.length === 1) {
    return [
      { ...detected[0], hideLaunch: detected[0].hideLaunch || hideLaunch },
    ];
  }
  const surface: SurfaceKind = hideLaunch ? "cli" : "desktop";
  const { title, showTitle } = componentTitle(agentId, surface);
  return [
    {
      surface,
      title,
      showTitle,
      localVersion: data.localVersion,
      remoteVersion: data.remoteVersion,
      installState: data.installState,
      updateState: data.updateState,
      inventoryState: data.inventoryState,
      allowedActions: data.allowedActions,
      reasonCodes: data.reasonCodes,
      requiresTargetSelection: data.requiresTargetSelection,
      releaseId: data.releaseId,
      sourceKind: data.sourceKind,
      hideLaunch,
      emptyNote: null,
    },
  ];
}

function LifecycleProgress({
  busy,
  stage,
  progressLabel,
  canCancel,
  onCancel,
}: {
  busy: boolean;
  stage: AgentActionJobStage | null;
  progressLabel: string | null;
  canCancel: boolean;
  onCancel: () => void;
}) {
  if (!busy) return null;
  const label = progressLabel ?? (stage ? jobStageCopy(stage) : "处理中…");
  return (
    <div className="fy-agent-install-readiness-loading">
      <Spinner label={label} />
      <span className="fy-agent-transfer-progress">{label}</span>
      {canCancel ? <Button onClick={onCancel}>取消</Button> : null}
    </div>
  );
}

function ReadinessSummary({
  projection,
  busy,
  jobStage,
  progressLabel,
  error,
  success,
  canCancel,
  targetPicker,
  onAction,
  onCancel,
  extra,
  showJobProgress = true,
}: {
  projection: SurfaceProjection;
  busy: boolean;
  jobStage: AgentActionJobStage | null;
  progressLabel: string | null;
  error: string | null;
  success: string | null;
  canCancel: boolean;
  targetPicker?: ReactNode;
  onAction: (action: AgentActionId) => void;
  onCancel: () => void;
  extra?: ReactNode;
  showJobProgress?: boolean;
}) {
  const managedByCodex = projection.reasonCodes.includes(
    "managed_by_codex_desktop",
  );
  const lifecycleActions = orderedLifecycleActions(
    projection.allowedActions,
    projection.hideLaunch,
  );
  const notices = projection.reasonCodes
    .map(reasonCopy)
    .filter((text): text is string => Boolean(text));
  return (
    <div data-surface={projection.surface}>
      {projection.showTitle ? <h4>{projection.title}</h4> : null}
      {notices.map((text) => (
        <p key={text} className="fy-agent-install-readiness-summary">
          {text}
        </p>
      ))}
      {projection.emptyNote ? (
        <p className="fy-agent-install-readiness-note">
          {projection.emptyNote}
        </p>
      ) : null}
      <dl className="fy-agent-install-readiness-grid">
        <div>
          <dt>安装状态</dt>
          <dd>{installStateCopy(projection.installState)}</dd>
        </div>
        <div>
          <dt>更新</dt>
          <dd>{updateStateCopy(projection.updateState)}</dd>
        </div>
        <div>
          <dt>发现的安装</dt>
          <dd>{inventoryStateCopy(projection.inventoryState)}</dd>
        </div>
        <div>
          <dt>本地版本</dt>
          <dd>{projection.localVersion ?? "未确认"}</dd>
        </div>
        <div>
          <dt>最新版本</dt>
          <dd>{projection.remoteVersion ?? "未确认"}</dd>
        </div>
      </dl>
      {showJobProgress ? (
        <LifecycleProgress
          busy={busy}
          stage={jobStage}
          progressLabel={progressLabel}
          canCancel={canCancel}
          onCancel={onCancel}
        />
      ) : null}
      {error ? <InlineNotice tone="warning">{error}</InlineNotice> : null}
      {success ? <InlineNotice tone="info">{success}</InlineNotice> : null}
      {extra}
      {targetPicker}
      {!managedByCodex && lifecycleActions.length > 0 ? (
        <div className="fy-agent-action-row">
          {lifecycleActions.map((action) => (
            <Button
              key={action}
              disabled={busy}
              onClick={() => onAction(action)}
            >
              {actionLabel(action, projection.surface)}
            </Button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function GrokOwnerPanel({
  port,
  nativeFailed,
}: {
  port: GrokToolingPort;
  nativeFailed: boolean;
}) {
  const [snapshot, setSnapshot] = useState<GrokToolSnapshot | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const refresh = () => {
    void port.getSnapshot().then(
      (data) => {
        setSnapshot(data);
        setUnavailable(false);
      },
      () => {
        setUnavailable(true);
      },
    );
  };

  useEffect(() => {
    let active = true;
    void port.getSnapshot().then(
      (data) => {
        if (!active) return;
        setSnapshot(data);
        setUnavailable(false);
      },
      () => {
        if (active) setUnavailable(true);
      },
    );
    return () => {
      active = false;
    };
  }, [port]);

  const runOfficialNpm = async () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    setSuccess(null);
    try {
      await port.installOfficialNpm();
      setSuccess("官方 npm 包已安装，安装状态已更新。");
      refresh();
    } catch {
      setError("官方 npm 安装未完成。原安装方式未改动。");
    } finally {
      setBusy(false);
    }
  };

  if (unavailable) {
    return (
      <p className="fy-agent-install-readiness-note">
        暂时无法确认 Grok Build 的安装方式。
      </p>
    );
  }
  if (!snapshot) return null;

  const notInstalled =
    snapshot.localVersion === null && !snapshot.installedButBroken;
  const showNpmChoice =
    notInstalled ||
    (nativeFailed && snapshot.distributionOwner !== "official_npm");

  return (
    <div className="fy-agent-grok-owner">
      <dl className="fy-agent-install-readiness-grid">
        <div>
          <dt>安装方式</dt>
          <dd>{grokOwnerCopy(snapshot.distributionOwner)}</dd>
        </div>
        <div>
          <dt>本地版本</dt>
          <dd>{snapshot.localVersion ?? "未安装"}</dd>
        </div>
        <div>
          <dt>{grokLatestLabel(snapshot.latestSource)}</dt>
          <dd>{snapshot.latestVersion ?? "未确认"}</dd>
        </div>
      </dl>
      {notInstalled ? (
        <p className="fy-agent-install-readiness-note">
          首次安装建议使用官方命令行。也可改用官方 npm
          包，两种方式不会自动切换。
        </p>
      ) : null}
      {busy ? (
        <div className="fy-agent-install-readiness-loading">
          <Spinner label="正在安装官方 npm 包" />
          <span>正在安装官方 npm 包</span>
        </div>
      ) : null}
      {error ? <InlineNotice tone="warning">{error}</InlineNotice> : null}
      {success ? <InlineNotice tone="info">{success}</InlineNotice> : null}
      {showNpmChoice && !busy ? (
        <div className="fy-agent-action-row">
          <Button onClick={() => void runOfficialNpm()}>
            {nativeFailed && !notInstalled
              ? "改用官方 npm 方式"
              : "使用官方 npm 方式"}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function AgentInstallReadinessContent({
  agentId,
  port,
  grokTooling,
}: {
  agentId: AgentCatalogId;
  port: ReadinessPort;
  grokTooling?: GrokToolingPort;
}) {
  const [state, setState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: AgentInstallReadiness }
    | { status: "unavailable" }
  >({ status: "loading" });
  const [inventoryState, setInventoryState] = useState<InventorySlot>({
    status: "loading",
  });
  const [targetAction, setTargetAction] = useState<{
    action: AgentActionId;
    surface: SurfaceKind | "product";
  } | null>(null);
  const [selectedTarget, setSelectedTarget] =
    useState<AgentInstallationTarget | null>(null);

  const targetOptions = useMemo(
    () =>
      targetAction && inventoryState.status === "ready"
        ? installationTargetsForAction(inventoryState.data, targetAction.action)
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
    setSelectedTarget(null);
    setInventoryState({ status: "loading" });
    void port.getInventory(agentId).then(
      (data) => setInventoryState({ status: "ready", data }),
      () => setInventoryState({ status: "unavailable" }),
    );
  };

  const handleAction = (
    action: AgentActionId,
    projection: SurfaceProjection,
  ) => {
    if (state.status !== "ready") return;
    if (!isLifecycleAction(action)) return;
    if (projection.hideLaunch && action === "launch") return;
    const cliBound =
      projection.surface === "cli" || projection.sourceKind === "cli_tooling";
    const launchEligibleCount =
      inventoryState.status === "ready"
        ? inventoryState.data.candidates.filter(
            (candidate) => candidate.launchEligible,
          ).length
        : 0;
    const needsTarget =
      (!cliBound &&
        (action === "install" || action === "update" || action === "launch")) ||
      (projection.requiresTargetSelection &&
        (action === "install" || action === "update" || action === "launch")) ||
      (action === "launch" && launchEligibleCount > 1);
    if (!needsTarget) {
      void lifecycle.run(action, null);
      return;
    }
    setTargetAction({ action, surface: projection.surface });
    if (inventoryState.status !== "ready") return;
    const options = installationTargetsForAction(inventoryState.data, action);
    const eligible = options.filter((target) =>
      target.eligibleActions.includes(action),
    );
    const hasDisabledSystem = options.some((target) =>
      target.reasonCodes.includes("authorization_required"),
    );
    const current = selectedTarget
      ? eligible.find((target) => target.targetId === selectedTarget.targetId)
      : undefined;
    if (current) {
      void lifecycle.run(action, current);
      return;
    }
    if (eligible.length === 1 && !hasDisabledSystem) {
      setSelectedTarget(eligible[0]);
      void lifecycle.run(action, eligible[0]);
      return;
    }
    setSelectedTarget(null);
  };

  const targetPicker = (surface: SurfaceKind | "product") =>
    targetAction && targetAction.surface === surface ? (
      <>
        <LifecycleTargetPicker
          id={`agent-lifecycle-target-${agentId}-${targetAction.action}-${surface}`}
          action={targetAction.action}
          targets={targetOptions}
          value={selectedTarget?.targetId ?? null}
          onChange={setSelectedTarget}
          loading={inventoryState.status === "loading"}
          error={
            inventoryState.status === "unavailable"
              ? "暂时无法读取安装位置。请刷新后重试。"
              : null
          }
          disabled={lifecycle.busy}
          onRefresh={() => refreshInventory()}
        />
        {targetOptions.some((target) =>
          target.reasonCodes.includes("authorization_required"),
        ) ? (
          <p className="fy-agent-install-readiness-note">
            {reasonCopy("authorization_required")}
          </p>
        ) : null}
        {targetOptions.length > 1 && !selectedTarget ? (
          <p className="fy-agent-install-readiness-note">
            请选择目标，然后再次点击“
            {actionLabel(
              targetAction.action,
              surface === "product" ? "desktop" : surface,
            )}
            ”。
          </p>
        ) : null}
      </>
    ) : null;

  const projections =
    state.status === "ready" ? surfacesForProduct(agentId, state.data) : [];

  return (
    <section className="fy-agent-section" aria-label="安装与更新">
      <h3>安装与更新</h3>
      <div className="fy-agent-install-readiness">
        {state.status === "loading" ? (
          <div className="fy-agent-install-readiness-loading">
            <Spinner label="正在检查安装状态" />
            <span>正在检查安装状态</span>
          </div>
        ) : state.status === "unavailable" ? (
          <InlineNotice tone="warning">
            暂时无法检查安装状态。请重新打开此页面。
          </InlineNotice>
        ) : projections[0] ? (
          <ReadinessSummary
            projection={projections[0]}
            busy={lifecycle.busy}
            jobStage={lifecycle.stage}
            progressLabel={lifecycle.progressLabel}
            error={lifecycle.error}
            success={lifecycle.success}
            canCancel={lifecycle.canCancel}
            targetPicker={targetPicker(projections[0]?.surface ?? "product")}
            onAction={(action) =>
              projections[0] ? handleAction(action, projections[0]) : undefined
            }
            onCancel={() => void lifecycle.cancel()}
            extra={
              agentId === "grokbuild" && grokTooling ? (
                <GrokOwnerPanel
                  port={grokTooling}
                  nativeFailed={Boolean(
                    lifecycle.error && lifecycle.reasonCode !== "cancelled",
                  )}
                />
              ) : null
            }
          />
        ) : null}
      </div>
    </section>
  );
}

export function AgentInstallReadinessSection({
  agentId,
  port = unavailablePort,
  grokTooling,
}: {
  agentId: AgentCatalogId;
  port?: ReadinessPort;
  grokTooling?: GrokToolingPort;
}) {
  return (
    <AgentInstallReadinessContent
      key={agentId}
      agentId={agentId}
      port={port}
      grokTooling={grokTooling}
    />
  );
}
