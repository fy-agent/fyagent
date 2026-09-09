import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { getAgentBrand } from "../../shared/assets/agents";
import {
  AGENT_CATALOG_IDS,
  PRODUCT_DIRECTORY,
  type AgentCatalogId,
} from "../../shared/features/directory";
import {
  HEALTH_ACTION_LABELS,
  HEALTH_CHECK_LABELS,
  HEALTH_REASON_LABELS,
  HEALTH_STATE_LABELS,
  HEALTH_STATUS_LABELS,
  healthActionPath,
  healthAgentFromSearch,
  healthPrimaryReason,
  healthStatus,
} from "../../shared/features/health-presentation";
import {
  HEALTH_STATUSES,
  type HealthAction,
  type HealthCheck,
  type HealthStatus,
} from "../../shared/features/health";
import { useFrontendReady } from "../../shared/platform/useFrontendReady";
import { Button } from "../../shared/ui/Button";
import {
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";
import {
  usePersistentSearchParams,
  useStickyVisibleValue,
} from "../../shared/ui/usePersistentSearchParams";
import { useHealthChecks } from "./useHealthChecks";
import "./page.css";

type StatusFilter = HealthStatus | "all" | "unchecked";
const STATUS_PRIORITY: Record<HealthStatus | "unchecked", number> = {
  blocked: 0,
  not_configured: 1,
  needs_attention: 2,
  stale: 3,
  unchecked: 4,
  ready: 5,
};
const CHECK_GROUPS = [
  {
    title: "安装与启动",
    ids: ["installation", "helper", "conflicts", "restart"],
  },
  {
    title: "账号与配置",
    ids: ["configuration", "secret", "auth", "model", "drift"],
  },
  { title: "连接与使用", ids: ["endpoint", "proxy", "last_request"] },
] as const;

function formatTime(value: string): string {
  return new Date(value).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function CheckRow({
  check,
  disabled,
  onAction,
}: {
  check: HealthCheck;
  disabled: boolean;
  onAction: (action: HealthAction) => void;
}) {
  const action = check.action;
  return (
    <li
      className="fy-health-check"
      data-check-id={check.id}
      data-severity={check.severity}
    >
      <div className="fy-health-check-copy">
        <div className="fy-health-check-heading">
          <h4>{HEALTH_CHECK_LABELS[check.id]}</h4>
          <span className="fy-health-state" data-state={check.state}>
            {HEALTH_STATE_LABELS[check.state]}
          </span>
        </div>
        <p>{HEALTH_REASON_LABELS[check.reasonCode]}</p>
        {check.value ? <p className="fy-health-value">{check.value}</p> : null}
        <p className="fy-health-time">
          检查时间{" "}
          <time dateTime={check.checkedAt}>{formatTime(check.checkedAt)}</time>
          {check.evidenceAt ? (
            <>
              {" "}
              · 记录时间{" "}
              <time dateTime={check.evidenceAt}>
                {formatTime(check.evidenceAt)}
              </time>
            </>
          ) : null}
        </p>
      </div>
      {action ? (
        <Button
          disabled={disabled}
          onClick={() => onAction(action)}
          aria-label={`${HEALTH_ACTION_LABELS[action]}：${HEALTH_CHECK_LABELS[check.id]}`}
        >
          {HEALTH_ACTION_LABELS[action]}
        </Button>
      ) : null}
    </li>
  );
}

export function HealthPage() {
  const { visible, searchParams, setSearchParams } =
    usePersistentSearchParams();
  const navigate = useNavigate();
  const selected = useStickyVisibleValue(
    visible,
    healthAgentFromSearch(searchParams.toString()),
    "codex",
  );
  const checks = useHealthChecks(selected, visible);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<StatusFilter>("all");
  const selectedQuery = checks.queries[AGENT_CATALOG_IDS.indexOf(selected)];
  const snapshot = selectedQuery.data;
  const readFailed =
    selectedQuery.isError ||
    selectedQuery.errorUpdatedAt > selectedQuery.dataUpdatedAt;
  const selectedStatus = snapshot
    ? healthStatus(snapshot, checks.now, readFailed)
    : null;
  const selectedName =
    PRODUCT_DIRECTORY.find((entry) => entry.agentId === selected)
      ?.displayName ?? selected;
  useFrontendReady(snapshot !== undefined || selectedQuery.isError);
  const entries = PRODUCT_DIRECTORY.map((entry) => {
    const query = checks.queries[AGENT_CATALOG_IDS.indexOf(entry.agentId)];
    const status: HealthStatus | "unchecked" = query.data
      ? healthStatus(
          query.data,
          checks.now,
          query.isError || query.errorUpdatedAt > query.dataUpdatedAt,
        )
      : "unchecked";
    return { ...entry, query, status };
  });
  const filtered = entries
    .filter(
      (entry) =>
        entry.displayName
          .toLocaleLowerCase()
          .includes(search.trim().toLocaleLowerCase()) &&
        (filter === "all" || entry.status === filter),
    )
    .sort((a, b) => STATUS_PRIORITY[a.status] - STATUS_PRIORITY[b.status]);
  const processed = entries.filter(
    (entry) => entry.query.data !== undefined,
  ).length;
  const attention = entries.filter((entry) =>
    ["blocked", "not_configured", "needs_attention"].includes(entry.status),
  ).length;
  const stale = entries.filter((entry) => entry.status === "stale").length;
  const onAction = (action: HealthAction) => {
    if (!visible) return;
    if (action === "refresh") {
      void checks.refreshSelected();
      return;
    }
    const path = healthActionPath(action, selected);
    if (path) {
      checks.stop();
      void navigate(path);
    }
  };
  const selectAgent = (agentId: AgentCatalogId) => {
    if (!visible) return;
    setSearchParams({ agent: agentId }, { replace: true });
  };
  const progress = checks.progress;
  const checkingName = PRODUCT_DIRECTORY.find(
    (entry) => entry.agentId === progress?.current,
  )?.displayName;

  return (
    <div
      className="fy-feature-page fy-split-page fy-catalog-page fy-health-page"
      data-testid="health-page"
      aria-label="运行状态"
    >
      <header className="fy-health-header">
        <div>
          <h1>运行状态</h1>
          <p>查看本机安装、账号与配置，找到需要处理的地方。</p>
        </div>
        <div className="fy-health-header-actions">
          <Button
            className="fy-control-button-primary"
            disabled={!visible || checks.busy}
            onClick={() => void checks.refreshAll()}
          >
            检查全部软件
          </Button>
          {checks.busy ? (
            <Button
              disabled={!visible || progress?.state === "stopping"}
              onClick={checks.stop}
            >
              停止检查
            </Button>
          ) : null}
        </div>
      </header>
      <div className="fy-health-overview" aria-label="检查概览">
        <span>
          已检查 {processed} / {entries.length}
        </span>
        <span>需要处理 {attention}</span>
        <span>需要重新检查 {stale}</span>
        <span>尚未检查 {entries.length - processed}</span>
      </div>
      <p className="fy-health-progress" role="status" aria-live="polite">
        {progress?.state === "running"
          ? `正在检查 ${checkingName ?? "软件"} · ${progress.completed} / ${progress.total}`
          : progress?.state === "stopping"
            ? "正在完成当前检查，之后停止。"
            : progress?.state === "stopped"
              ? `检查已停止 · 已完成 ${progress.completed} / ${progress.total}${progress.failed ? `，${progress.failed} 个读取失败` : ""}`
              : progress?.state === "complete"
                ? `检查完成 · ${progress.completed - progress.failed} 个已更新${progress.failed ? `，${progress.failed} 个读取失败，可单独重试` : ""}`
                : "选择软件即可检查，也可以检查全部软件。"}
      </p>
      <CatalogMasterDetail>
        <CatalogRail
          ariaLabel="软件运行状态"
          title="选择软件"
          meta="需要处理的软件优先显示"
        >
          <div className="fy-health-filters">
            <FeatureSearch
              value={search}
              onValueChange={setSearch}
              ariaLabel="搜索软件"
              placeholder="搜索软件名称"
            />
            <label className="fy-health-filter">
              筛选状态
              <select
                className="fy-control-input"
                value={filter}
                onChange={(event) => {
                  const value = event.target.value;
                  if (
                    value === "all" ||
                    value === "unchecked" ||
                    HEALTH_STATUSES.some((status) => status === value)
                  )
                    setFilter(value as StatusFilter);
                }}
              >
                <option value="all">全部状态</option>
                {HEALTH_STATUSES.map((status) => (
                  <option key={status} value={status}>
                    {HEALTH_STATUS_LABELS[status]}
                  </option>
                ))}
                <option value="unchecked">尚未检查</option>
              </select>
            </label>
          </div>
          <CatalogList
            layoutKey={filtered.map((entry) => entry.agentId).join(",")}
          >
            {filtered.map((entry) => (
              <CatalogListItem
                key={entry.agentId}
                asset={getAgentBrand(entry.agentId)}
                label={entry.displayName}
                selected={selected === entry.agentId}
                onSelect={() => selectAgent(entry.agentId)}
                testId={`health-agent-${entry.agentId}`}
                summary={
                  entry.query.isFetching
                    ? "正在检查…"
                    : entry.status === "unchecked"
                      ? entry.query.isError
                        ? "读取失败 · 请重试"
                        : "尚未检查"
                      : HEALTH_STATUS_LABELS[entry.status]
                }
              />
            ))}
          </CatalogList>
          {filtered.length === 0 ? (
            <EmptyState
              title="没有匹配的软件"
              description="请修改名称或状态筛选。"
              actions={
                <Button
                  onClick={() => {
                    setSearch("");
                    setFilter("all");
                  }}
                >
                  清除筛选
                </Button>
              }
            />
          ) : null}
        </CatalogRail>
        <CatalogDetail
          ariaLabel={`${selectedName} 运行状态`}
          className="fy-health-detail"
        >
          <div className="fy-health-detail-heading">
            <div>
              <h2>{selectedName}</h2>
              {selectedStatus ? (
                <span className="fy-health-status" data-status={selectedStatus}>
                  {HEALTH_STATUS_LABELS[selectedStatus]}
                </span>
              ) : null}
            </div>
            <Button
              disabled={!visible || checks.busy}
              onClick={() => void checks.refreshSelected()}
            >
              重新检查此软件
            </Button>
          </div>
          <p className="fy-health-scope">
            本机检查不会测试远端服务或额度。检查结果超过 5 分钟后需要重新检查。
          </p>
          {snapshot ? (
            <>
              {selectedStatus === "stale" ? (
                <InlineNotice tone="warning">
                  {readFailed
                    ? "本次读取失败，以下保留上次结果。请重新检查。"
                    : "检查结果已超过 5 分钟，请重新检查后再判断。"}
                </InlineNotice>
              ) : null}
              <p className="fy-health-summary">
                {healthPrimaryReason(snapshot)}
              </p>
              <p className="fy-health-time">
                最后检查{" "}
                <time dateTime={snapshot.checkedAt}>
                  {formatTime(snapshot.checkedAt)}
                </time>
              </p>
              {CHECK_GROUPS.map((group) => (
                <section
                  key={group.title}
                  className="fy-health-group"
                  aria-label={group.title}
                >
                  <h3>{group.title}</h3>
                  <ul>
                    {group.ids.map((id) => {
                      const check = snapshot.checks.find(
                        (item) => item.id === id,
                      );
                      return check ? (
                        <CheckRow
                          key={id}
                          check={check}
                          disabled={
                            !visible ||
                            (check.action === "refresh" && checks.busy)
                          }
                          onAction={onAction}
                        />
                      ) : null;
                    })}
                  </ul>
                </section>
              ))}
            </>
          ) : selectedQuery.isError ? (
            <EmptyState
              title="无法读取运行状态"
              description="请在 FyAgent 桌面应用中重新检查。当前尚无可显示的结果。"
            />
          ) : (
            <EmptyState
              title="正在读取本机状态"
              description="读取完成后，会显示检查结果和处理入口。"
            >
              <Spinner label="正在读取本机状态" />
            </EmptyState>
          )}
        </CatalogDetail>
      </CatalogMasterDetail>
    </div>
  );
}
