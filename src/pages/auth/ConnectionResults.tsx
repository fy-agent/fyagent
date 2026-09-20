import type { ConfigRecoveryTarget } from "../../shared/features/config-recovery";
import { FileRecoveryButton } from "../../shared/features/controls/FileRecoveryButton";
import type {
  ManagedAuthConnectionAction,
  ManagedAuthConnectionSummary,
  ManagedAuthConsumer,
} from "../../shared/features/managed-auth";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { InlineNotice } from "../../shared/ui/primitives";
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

const recoveryTargets: Partial<
  Record<ManagedAuthConsumer, readonly ConfigRecoveryTarget[]>
> = {
  codex: ["codex_auth", "codex_config"],
  opencode: ["opencode_auth"],
};

export function ConnectionResults({
  results,
  connections,
  disabled,
  originRef,
  onRetry,
  onRefresh,
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
}) {
  if (Object.keys(results).length === 0) return null;
  return (
    <section className="fy-auth-section" aria-label="软件连接操作结果">
      <h2>软件连接操作结果</h2>
      <p>每个软件分别保存并检查。一个软件失败不会撤回其他软件已完成的连接。</p>
      {Object.entries(results).map(([id, result]) => {
        const current = connections.find((item) => item.connectionId === id);
        const targets = recoveryTargets[result.connection.consumer];
        const label = `${managedAuthConsumerLabel(result.connection.consumer)}${result.connection.provider ? ` · ${managedAuthProviderLabel(result.connection.provider)}` : ""}`;
        const failed = result.state === "partial" || result.state === "failed";
        return (
          <article
            key={id}
            aria-label={`${label} 操作结果`}
            className="fy-auth-connection-card"
          >
            <h3>{label}</h3>
            <InlineNotice tone={failed ? "warning" : "info"}>
              <span role="status">{result.message}</span>
            </InlineNotice>
            <div className="fy-feature-actions">
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
              {targets && result.state !== "pending" ? (
                <FileRecoveryButton
                  targets={targets}
                  disabled={disabled}
                  onRestored={async () => {
                    await onRefresh();
                  }}
                />
              ) : null}
            </div>
          </article>
        );
      })}
    </section>
  );
}
