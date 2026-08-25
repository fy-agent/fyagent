import { getAgentBrand } from "../../shared/assets/agents";
import type {
  AgentInstallReadiness,
  AgentInstallState,
} from "../../shared/features/agent-install-readiness";
import {
  AGENT_CATALOG_IDS,
  type AgentCatalogEntry,
  type AgentCatalogId,
} from "../../shared/features/types";
import { BrandIconFrame, CatalogOfficialLinks } from "../../shared/ui/catalog";
import { Button, InlineNotice, Spinner } from "../../shared/ui/primitives";
import type {
  AgentDirectoryScanController,
  AgentDirectoryScanState,
} from "./useAgentDirectoryScan";

function readinessCopy(state: AgentInstallState): string {
  switch (state) {
    case "installed":
      return "已发现 · 已安装";
    case "installed_not_runnable":
      return "已发现 · 当前不可运行";
    case "not_installed":
      return "未安装";
    case "unavailable":
      return "当前环境不可用";
    case "unknown":
      return "未确认";
  }
}

function readinessTone(state: AgentInstallState): string {
  switch (state) {
    case "installed":
      return "installed";
    case "installed_not_runnable":
    case "unavailable":
      return "warning";
    case "unknown":
      return "unknown";
    case "not_installed":
      return "not-installed";
  }
}

function formatScanTime(timestamp: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(new Date(timestamp));
}

function AgentReadinessStatus({
  agentId,
  scan,
}: {
  agentId: AgentCatalogId;
  scan: AgentDirectoryScanState;
}) {
  const data = scan.results[agentId];
  const failed = scan.currentFailureIds.includes(agentId);
  const settled = scan.settledIds.includes(agentId);

  if (scan.status === "scanning" && !settled) {
    return (
      <span className="fy-agent-directory-status" data-status="scanning">
        <Spinner label={`正在扫描 ${agentId}`} />
        扫描中
      </span>
    );
  }
  if (failed) {
    return (
      <span className="fy-agent-directory-status" data-status="error">
        {data
          ? `本次读取失败 · ${readinessCopy(data.installState)}`
          : "读取失败"}
      </span>
    );
  }
  if (data) {
    return (
      <span
        className="fy-agent-directory-status"
        data-status={readinessTone(data.installState)}
      >
        {readinessCopy(data.installState)}
      </span>
    );
  }
  return (
    <span className="fy-agent-directory-status" data-status="idle">
      尚未扫描
    </span>
  );
}

function AgentDirectoryCard({
  entry,
  scan,
  onConfigure,
}: {
  entry: AgentCatalogEntry;
  scan: AgentDirectoryScanState;
  onConfigure: (agentId: AgentCatalogId) => void;
}) {
  const productCapability = entry.capabilities.find(
    (candidate) => candidate.id === "product.open",
  );
  const scanned = Boolean(scan.results[entry.id]);
  const scanning = scan.status === "scanning";

  return (
    <article className="fy-agent-directory-card" data-agent-id={entry.id}>
      <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
      <div className="fy-agent-directory-card-copy">
        <div className="fy-agent-directory-card-heading">
          <h2>{entry.displayName}</h2>
          <AgentReadinessStatus agentId={entry.id} scan={scan} />
        </div>
        <p className="fy-agent-directory-description">{entry.description}</p>
        <div className="fy-agent-directory-meta">
          {scanned && scan.lastSuccessfulScanAt ? (
            <span>
              上次扫描：
              <time
                dateTime={new Date(scan.lastSuccessfulScanAt).toISOString()}
              >
                {formatScanTime(scan.lastSuccessfulScanAt)}
              </time>
            </span>
          ) : (
            <span>等待本机扫描结果</span>
          )}
          <details className="fy-agent-directory-more">
            <summary>查看完整介绍</summary>
            <p>{entry.description}</p>
            <CatalogOfficialLinks
              links={entry.officialLinks}
              disabled={
                productCapability?.mode !== "direct" &&
                productCapability?.mode !== "assisted"
              }
            />
          </details>
        </div>
      </div>
      <Button disabled={scanning} onClick={() => onConfigure(entry.id)}>
        进行配置
      </Button>
    </article>
  );
}

export function AgentDirectory({
  entries,
  scanController,
  onConfigure,
}: {
  entries: readonly AgentCatalogEntry[];
  scanController: AgentDirectoryScanController;
  onConfigure: (agentId: AgentCatalogId) => void;
}) {
  const { state, start } = scanController;
  const scanning = state.status === "scanning";
  const hasSuccessfulResults = Object.keys(state.results).length > 0;
  const currentReadiness = state.currentSuccessIds
    .map((id) => state.results[id])
    .filter((item): item is AgentInstallReadiness => Boolean(item));
  const discoveredCount = currentReadiness.filter(
    (item) =>
      item.installState === "installed" ||
      item.installState === "installed_not_runnable",
  ).length;
  const unknownCount = currentReadiness.filter(
    (item) => item.installState === "unknown",
  ).length;
  const allFailed =
    state.status === "complete" && state.currentSuccessIds.length === 0;
  const partiallyFailed =
    state.status === "complete" &&
    state.currentSuccessIds.length > 0 &&
    state.currentFailureIds.length > 0;
  const empty =
    state.status === "complete" &&
    state.currentSuccessIds.length === AGENT_CATALOG_IDS.length &&
    discoveredCount === 0 &&
    unknownCount === 0;

  return (
    <section className="fy-agent-directory" aria-label="AI 软件目录">
      <header className="fy-agent-directory-header">
        <div>
          <p className="fy-agent-directory-eyebrow">AI 软件配置</p>
          <h1>我的 AI 软件</h1>
          <p>扫描本机已知 Agent，并从真实准备度结果进入对应配置。</p>
        </div>
        <Button
          className="fy-control-button-primary"
          disabled={scanning}
          onClick={start}
        >
          {scanning
            ? "扫描中…"
            : hasSuccessfulResults
              ? "重新扫描"
              : "开始扫描"}
        </Button>
      </header>

      {scanning ? (
        <div
          className="fy-agent-directory-progress"
          role="status"
          aria-live="polite"
        >
          <div>
            <span>
              正在扫描本机 AI 软件 · 已完成 {state.settledIds.length} /{" "}
              {AGENT_CATALOG_IDS.length}
            </span>
            <span>已发现 {discoveredCount} 个</span>
          </div>
          <progress
            max={AGENT_CATALOG_IDS.length}
            value={state.settledIds.length}
            aria-label="扫描进度"
          />
          <p>请等待扫描完成；当前没有可安全取消的后台扫描任务。</p>
        </div>
      ) : null}

      {allFailed ? (
        <InlineNotice tone="error">
          本次扫描未能读取任何软件状态。
          {hasSuccessfulResults
            ? "正在保留上次成功结果，请重试。"
            : "请检查桌面能力后重试。"}
        </InlineNotice>
      ) : partiallyFailed ? (
        <InlineNotice tone="warning">
          {state.currentFailureIds.length}{" "}
          个软件状态读取失败；成功结果已更新，失败项保留上次结果。
        </InlineNotice>
      ) : empty ? (
        <InlineNotice>
          本次未发现已安装的 AI 软件。目录仍保留明确的“未安装”结果，可重新扫描。
        </InlineNotice>
      ) : state.status === "complete" && unknownCount > 0 ? (
        <InlineNotice tone="warning">
          {unknownCount} 个软件状态尚未确认；“未确认”不等于“未安装”。
        </InlineNotice>
      ) : null}

      <div className="fy-agent-directory-list">
        {entries.map((entry) => (
          <AgentDirectoryCard
            key={entry.id}
            entry={entry}
            scan={state}
            onConfigure={onConfigure}
          />
        ))}
      </div>
    </section>
  );
}
