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
import { BrandIconFrame } from "../../shared/ui/catalog";
import { Button, EmptyState, InlineNotice } from "../../shared/ui/primitives";
import type {
  AgentDirectoryScanController,
  AgentDirectoryScanState,
} from "./useAgentDirectoryScan";

function isInstalledState(state: AgentInstallState | undefined): boolean {
  return state === "installed" || state === "installed_not_runnable";
}

function projectInstalledEntries(
  entries: readonly AgentCatalogEntry[],
  scan: AgentDirectoryScanState,
): AgentCatalogEntry[] {
  return entries.filter((entry) => {
    const result = scan.results[entry.id];
    return isInstalledState(result?.installState);
  });
}

function AgentDirectoryCard({
  entry,
  scanning,
  onConfigure,
}: {
  entry: AgentCatalogEntry;
  scanning: boolean;
  onConfigure: (agentId: AgentCatalogId) => void;
}) {
  return (
    <article className="fy-agent-directory-card" data-agent-id={entry.id}>
      <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
      <div className="fy-agent-directory-card-copy">
        <div className="fy-agent-directory-card-heading">
          <h2>{entry.displayName}</h2>
        </div>
        <p className="fy-agent-directory-description">{entry.description}</p>
      </div>
      <Button disabled={scanning} onClick={() => onConfigure(entry.id)}>
        进行配置
      </Button>
    </article>
  );
}

function AgentDirectorySkeletonCard({ keyIndex }: { keyIndex: number }) {
  return (
    <div
      className="fy-agent-directory-card is-skeleton"
      aria-hidden="true"
      data-testid={`skeleton-card-${keyIndex}`}
    >
      <div className="fy-agent-skeleton-icon" />
      <div className="fy-agent-directory-card-copy">
        <div className="fy-agent-skeleton-title" />
        <div className="fy-agent-skeleton-desc" />
      </div>
    </div>
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
  const complete = state.status === "complete";
  const hasSuccessfulResults = Object.keys(state.results).length > 0;

  const currentReadiness = state.currentSuccessIds
    .map((id) => state.results[id])
    .filter((item): item is AgentInstallReadiness => Boolean(item));

  const discoveredCount = currentReadiness.filter((item) =>
    isInstalledState(item.installState),
  ).length;

  const allFailed = complete && state.currentSuccessIds.length === 0;
  const hasTechnicalError = state.currentFailureIds.length > 0;

  const installedEntries = complete || scanning
    ? projectInstalledEntries(entries, state)
    : [];

  const remainingSkeletonCount = scanning
    ? Math.max(0, AGENT_CATALOG_IDS.length - state.settledIds.length)
    : 0;

  return (
    <section className="fy-agent-directory" aria-label="AI 软件目录">
      <header className="fy-agent-directory-header">
        <div className="fy-agent-directory-title-row">
          <div className="fy-agent-directory-title-group">
            <h1>我的 AI 软件</h1>
          </div>
          <Button
            className="fy-control-button-primary fy-agent-scan-button"
            disabled={scanning}
            onClick={start}
          >
            {scanning
              ? "扫描中…"
              : complete && hasSuccessfulResults
                ? "重新扫描"
                : "开始扫描"}
          </Button>
        </div>
      </header>

      {scanning ? (
        <div
          className="fy-agent-directory-progress"
          role="status"
          aria-live="polite"
        >
          <div className="fy-agent-directory-progress-labels">
            <span>正在扫描本机 AI 软件</span>
            <span>已发现 {discoveredCount} 个</span>
          </div>
          <progress
            max={AGENT_CATALOG_IDS.length}
            value={state.settledIds.length}
            aria-label="扫描进度"
          />
        </div>
      ) : null}

      {allFailed ? (
        <InlineNotice tone="error">
          本次扫描未能读取任何软件状态。{hasSuccessfulResults ? "已保留上次成功结果，请重试。" : "请重试。"}
        </InlineNotice>
      ) : hasTechnicalError && complete ? (
        <InlineNotice tone="warning">
          {state.currentFailureIds.length} 个软件状态读取失败，成功结果已更新。
        </InlineNotice>
      ) : complete && installedEntries.length === 0 ? (
        <EmptyState
          title="未发现已安装的 AI 软件"
        />
      ) : null}

      <div className="fy-agent-directory-list">
        {installedEntries.map((entry) => (
          <AgentDirectoryCard
            key={entry.id}
            entry={entry}
            scanning={scanning}
            onConfigure={onConfigure}
          />
        ))}
        {Array.from({ length: remainingSkeletonCount }).map((_, index) => (
          <AgentDirectorySkeletonCard key={`skeleton-${index}`} keyIndex={index} />
        ))}
      </div>
    </section>
  );
}
