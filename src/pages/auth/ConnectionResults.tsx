import { useId, useState } from "react";
import type {
  ManagedAuthConnectionAction,
  ManagedAuthConnectionSummary,
} from "../../shared/features/managed-auth";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import {
  managedAuthConsumerLabel,
  managedAuthProviderLabel,
} from "./presentation";

export interface ConnectionResult {
  connection: ManagedAuthConnectionSummary;
  action: ManagedAuthConnectionAction;
  accountId: string | null;
  state: "pending" | "completed" | "partial" | "failed";
  message: string;
}

type ResultItem = {
  id: string;
  result: ConnectionResult;
  current: ManagedAuthConnectionSummary | undefined;
  label: string;
  failed: boolean;
  restart: boolean;
};

function summaryText(items: ResultItem[]) {
  const count = (match: (item: ResultItem) => boolean) =>
    items.filter(match).length;
  const failed = count((item) => item.failed);
  const pending = count((item) => item.result.state === "pending");
  const restart = count((item) => item.restart);
  const completed = count(
    (item) => item.result.state === "completed" && !item.restart,
  );
  return [
    failed ? `${failed} 个需要处理` : null,
    pending ? `${pending} 个处理中` : null,
    restart ? `${restart} 个需重新启动软件` : null,
    completed ? `${completed} 个已完成` : null,
  ]
    .filter(Boolean)
    .join("，");
}

export function ConnectionResults({
  results,
  connections,
  disabled,
  originRef,
  onRetry,
  onRefresh,
  onDismissCompleted,
}: {
  results: Record<string, ConnectionResult>;
  connections: ManagedAuthConnectionSummary[];
  disabled: boolean;
  originRef: DialogOriginRef;
  onRetry: (
    connection: ManagedAuthConnectionSummary,
    result: ConnectionResult,
  ) => void;
  onRefresh: () => Promise<unknown>;
  onDismissCompleted: (connectionIds: string[]) => void;
}) {
  const completedListId = useId();
  const [showCompleted, setShowCompleted] = useState(false);
  const items: ResultItem[] = Object.entries(results).map(([id, result]) => {
    const current = connections.find((item) => item.connectionId === id);
    return {
      id,
      result,
      current,
      label: `${managedAuthConsumerLabel(result.connection.consumer)}${result.connection.provider ? ` · ${managedAuthProviderLabel(result.connection.provider)}` : ""}`,
      failed: result.state === "partial" || result.state === "failed",
      restart: result.state === "completed" && current?.pendingRestart === true,
    };
  });
  if (items.length === 0) return null;
  // Failures, in-flight work and pending restarts stay visible; only plain
  // completions fold away.
  const completed = items.filter(
    (item) => item.result.state === "completed" && !item.restart,
  );
  const attention = items.filter((item) => !completed.includes(item));
  const renderItem = (item: ResultItem) => (
    <article
      key={item.id}
      aria-label={`${item.label} 操作结果`}
      className="fy-auth-result-item"
      data-tone={item.failed ? "warning" : "info"}
    >
      <div className="fy-auth-result-copy">
        <h3>{item.label}</h3>
        <p role={completed.includes(item) ? undefined : "status"}>
          {item.result.message}
        </p>
      </div>
      {item.failed ? (
        <div className="fy-auth-result-actions">
          {item.current?.allowedActions.includes(item.result.action) ? (
            <Button
              disabled={disabled}
              dialogOriginRef={originRef}
              onClick={() => onRetry(item.current!, item.result)}
            >
              重试此连接
            </Button>
          ) : null}
          <Button
            disabled={disabled}
            onClick={() => void onRefresh().catch(() => {})}
          >
            重新读取连接
          </Button>
        </div>
      ) : null}
    </article>
  );
  return (
    <section className="fy-auth-results" aria-label="软件连接操作结果">
      <div className="fy-auth-results-bar">
        <h2>软件连接操作结果</h2>
        <span role="status">{summaryText(items)}</span>
        {completed.length > 0 ? (
          <div className="fy-auth-result-actions">
            <Button
              aria-expanded={showCompleted}
              aria-controls={completedListId}
              onClick={() => setShowCompleted((open) => !open)}
            >
              {showCompleted
                ? "收起已完成"
                : `查看已完成（${completed.length}）`}
            </Button>
            <Button
              onClick={() =>
                onDismissCompleted(completed.map((item) => item.id))
              }
            >
              关闭已完成
            </Button>
          </div>
        ) : null}
      </div>
      {attention.map(renderItem)}
      <div
        id={completedListId}
        className="fy-auth-results-completed"
        hidden={!showCompleted}
      >
        <p className="fy-auth-results-note">
          每个软件分别保存并检查。一个软件失败不会撤回其他软件已完成的连接。
        </p>
        {completed.map(renderItem)}
      </div>
    </section>
  );
}
