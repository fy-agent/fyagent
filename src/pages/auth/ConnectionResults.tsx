import { useState } from "react";
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

export function ConnectionResults({
  results,
  connections,
  disabled,
  originRef,
  onRetry,
  onRefresh,
  onDismiss,
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
  onDismiss: (connectionId: string) => void;
}) {
  const entries = Object.entries(results);
  const [openId, setOpenId] = useState<string | null>(null);
  if (entries.length === 0) return null;
  return (
    <section className="fy-auth-results" aria-label="软件连接操作结果">
      <p className="fy-auth-results-note">
        各软件分开保存。一个失败不会撤回其他软件。
      </p>
      {entries.map(([id, result]) => {
        const current = connections.find((item) => item.connectionId === id);
        const label = `${managedAuthConsumerLabel(result.connection.consumer)}${result.connection.provider ? ` · ${managedAuthProviderLabel(result.connection.provider)}` : ""}`;
        const failed = result.state === "partial" || result.state === "failed";
        const open = openId === id;
        return (
          <article
            key={id}
            aria-label={`${label} 操作结果`}
            className="fy-auth-result"
            data-state={result.state}
          >
            <p className="fy-auth-result-line">
              <strong>{label}</strong>
              <span role="status">{result.message}</span>
            </p>
            <div className="fy-auth-result-actions">
              {failed && current?.allowedActions.includes(result.action) ? (
                <Button
                  disabled={disabled}
                  dialogOriginRef={originRef}
                  onClick={() => onRetry(current, result)}
                >
                  重试此连接
                </Button>
              ) : null}
              {failed ? (
                <Button
                  disabled={disabled}
                  onClick={() => void onRefresh().catch(() => {})}
                >
                  重新读取连接
                </Button>
              ) : null}
              <Button
                aria-expanded={open}
                onClick={() => setOpenId(open ? null : id)}
              >
                {open ? "收起" : "详情"}
              </Button>
              <Button onClick={() => onDismiss(id)}>
                {`关闭${label}结果`}
              </Button>
            </div>
            {open ? (
              <p className="fy-auth-result-detail">{result.message}</p>
            ) : null}
          </article>
        );
      })}
    </section>
  );
}
