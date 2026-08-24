import { useEffect, useState } from "react";

import type {
  AgentInstallReadiness,
  AgentInstallReadinessPort,
  ReadinessLayerState,
} from "../../shared/features/agent-install-readiness";
import type { AgentCatalogId } from "../../shared/features/types";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";

type ReadinessLoad = AgentInstallReadinessPort["get"];

const NATIVE_ONLY_COPY = "安装准备度仅在桌面应用接线后可读取";

const unavailableAgentInstallReadiness: ReadinessLoad = async () => {
  throw new Error(NATIVE_ONLY_COPY);
};

function layerStateCopy(state: ReadinessLayerState): string {
  switch (state) {
    case "ok":
      return "已确认";
    case "warn":
      return "需留意";
    case "fail":
      return "未通过";
    case "unknown":
      return "未确认";
  }
}

function ReadinessSummary({ data }: { data: AgentInstallReadiness }) {
  const managedByCodex =
    data.automation.reasonCode === "managed_by_codex_desktop";
  const officialGuideOnly =
    data.automation.reasonCode === "official_guide_only";
  return (
    <>
      <p className="fy-agent-install-readiness-summary">
        {managedByCodex
          ? "安装与更新由现有 Codex Desktop 安装器管理。"
          : officialGuideOnly
            ? "当前仅提供官方指引，通用自动安装尚不可用。"
            : "自动安装执行器尚未实现，请参考官方安装说明。"}
      </p>
      <dl className="fy-agent-install-readiness-grid">
        <div>
          <dt>自动化</dt>
          <dd>不可用</dd>
        </div>
        <div>
          <dt>来源与许可</dt>
          <dd>{layerStateCopy(data.source.state)}</dd>
        </div>
        <div>
          <dt>完整性</dt>
          <dd>{layerStateCopy(data.integrity.state)}</dd>
        </div>
        <div>
          <dt>环境检查</dt>
          <dd>{layerStateCopy(data.preflight.state)}</dd>
        </div>
        <div>
          <dt>安装计划</dt>
          <dd>{layerStateCopy(data.plan.state)}</dd>
        </div>
      </dl>
      <p className="fy-agent-install-readiness-note">
        未创建安装计划；未知或未确认信息不会被视为可安装。
      </p>
    </>
  );
}

function AgentInstallReadinessContent({
  agentId,
  load,
}: {
  agentId: AgentCatalogId;
  load: ReadinessLoad;
}) {
  const [state, setState] = useState<
    | { status: "loading" }
    | { status: "ready"; data: AgentInstallReadiness }
    | { status: "unavailable" }
  >({ status: "loading" });

  useEffect(() => {
    let active = true;
    void load(agentId).then(
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
  }, [agentId, load]);

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
          <ReadinessSummary data={state.data} />
        )}
      </div>
    </section>
  );
}

export function AgentInstallReadinessSection({
  agentId,
  load = unavailableAgentInstallReadiness,
}: {
  agentId: AgentCatalogId;
  load?: ReadinessLoad;
}) {
  return (
    <AgentInstallReadinessContent key={agentId} agentId={agentId} load={load} />
  );
}
