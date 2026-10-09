import type { ReactNode } from "react";
import {
  MANAGED_AUTH_CONSUMERS,
  type ManagedAuthConnectionAction,
  type ManagedAuthConnectionSummary,
  type ManagedAuthConsumer,
  type ManagedAuthOverview,
} from "../../shared/features/managed-auth";
import {
  CatalogDetail,
  CatalogList,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { EmptyState } from "../../shared/ui/primitives";
import { PersistentSurface } from "../../shared/ui/PersistentSurface";
import { connectionPresentation, summarizeConnections } from "./summary";
import { GrokOfficialLogin } from "./GrokOfficialLogin";
import {
  AuthListItem,
  DefinitionRow,
  ProviderMark,
  ReasonList,
  StatusBadge,
} from "./common";
import {
  connectionActionLabel,
  managedAuthConsumerLabel,
  managedAuthManagerLabel,
  managedAuthProviderLabel,
  requestModeLabel,
} from "./presentation";

function ConnectionCard({
  originRef,
  connection,
  overview,
  mutationBusy,
  onAction,
}: {
  connection: ManagedAuthConnectionSummary;
  originRef?: DialogOriginRef;
  overview: ManagedAuthOverview;
  mutationBusy: boolean;
  onAction: (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
  ) => void;
}) {
  const account = connection.accountId
    ? overview.accounts.find(
        (candidate) => candidate.accountId === connection.accountId,
      )
    : null;
  const status = connectionPresentation(connection);
  return (
    <article className="fy-auth-consumer-connection">
      <div className="fy-auth-connection-card-heading">
        <div className="fy-auth-connection-identity">
          {connection.provider ? (
            <ProviderMark provider={connection.provider} />
          ) : null}
          <div>
            <h3>
              {connection.provider
                ? managedAuthProviderLabel(connection.provider)
                : "软件状态"}
            </h3>
            {connection.targetLabel ? <p>{connection.targetLabel}</p> : null}
          </div>
        </div>
        <StatusBadge {...status} />
      </div>

      <dl className="fy-feature-definition fy-auth-definition">
        <DefinitionRow label="受管账号绑定">
          {account
            ? `${managedAuthProviderLabel(account.provider)} · ${account.login}`
            : "尚未绑定受管账号"}
        </DefinitionRow>
        <DefinitionRow label="当前模型来源">
          {requestModeLabel(
            connection.requestMode,
            connection.requestProviderLabel,
          )}
        </DefinitionRow>
        {connection.officialSessionPreserved !== null ? (
          <DefinitionRow label="官方登录">
            {connection.officialSessionPreserved ? "已保留" : "未确认保留"}
          </DefinitionRow>
        ) : null}
        <DefinitionRow label="自动续期">
          {managedAuthManagerLabel(connection.credentialManager)}
        </DefinitionRow>
      </dl>

      <ReasonList reasons={connection.reasonCodes} />

      {connection.allowedActions.length > 0 ? (
        <div className="fy-feature-actions">
          {connection.allowedActions.map((action) => (
            <Button
              key={action}
              dialogOriginRef={originRef}
              className={
                action === "disconnect"
                  ? "fy-control-button-danger-subtle"
                  : undefined
              }
              disabled={mutationBusy}
              onClick={() => onAction(connection, action)}
            >
              {connectionActionLabel(action, connection)}
            </Button>
          ))}
        </div>
      ) : null}
    </article>
  );
}

export function ConnectionsView({
  active = true,
  originRef,
  overview,
  selectedConsumer,
  mutationBusy,
  onSelectConsumer,
  onClearSelection,
  onAction,
  codexSourceControls,
}: {
  active?: boolean;
  overview: ManagedAuthOverview;
  selectedConsumer: ManagedAuthConsumer | null;
  originRef?: DialogOriginRef;
  mutationBusy: boolean;
  onSelectConsumer: (consumer: ManagedAuthConsumer) => void;
  onClearSelection: () => void;
  codexSourceControls?: ReactNode;
  onAction: (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
  ) => void;
}) {
  const selectedConnections = selectedConsumer
    ? overview.connections.filter(
        (connection) => connection.consumer === selectedConsumer,
      )
    : [];
  return (
    <CatalogMasterDetail className="fy-auth-master-detail">
      <CatalogRail
        ariaLabel="软件连接列表"
        title="软件连接"
        meta={summarizeConnections(overview.connections).countsLabel}
        className="fy-auth-rail"
      >
        <CatalogList>
          {MANAGED_AUTH_CONSUMERS.map((consumer) => {
            const connections = overview.connections.filter(
              (connection) => connection.consumer === consumer,
            );
            const status = summarizeConnections(connections);
            return (
              <AuthListItem
                key={consumer}
                selected={consumer === selectedConsumer}
                label={managedAuthConsumerLabel(consumer)}
                leading={
                  <span className="fy-auth-consumer-mark" aria-hidden>
                    {managedAuthConsumerLabel(consumer).slice(0, 1)}
                  </span>
                }
                summary={status.summary}
                trailing={<StatusBadge {...status} />}
                onSelect={() => onSelectConsumer(consumer)}
                testId={`managed-auth-consumer-${consumer}`}
              />
            );
          })}
        </CatalogList>
      </CatalogRail>
      <CatalogDetail
        ariaLabel={
          selectedConsumer
            ? `${managedAuthConsumerLabel(selectedConsumer)} 连接详情`
            : "软件连接详情"
        }
        className="fy-auth-detail"
      >
        {selectedConsumer ? (
          <>
            <Button className="fy-auth-mobile-back" onClick={onClearSelection}>
              返回软件连接列表
            </Button>
            <header className="fy-auth-consumer-header">
              <div>
                <h2>{managedAuthConsumerLabel(selectedConsumer)}</h2>
              </div>
              <StatusBadge {...summarizeConnections(selectedConnections)} />
            </header>
            {selectedConnections.length === 0 ? (
              <EmptyState
                title="暂时没有可管理的连接"
                description="安装或配置状态更新后，请刷新此页面。"
              />
            ) : (
              <div className="fy-auth-consumer-connections">
                {selectedConnections.map((connection) => (
                  <ConnectionCard
                    originRef={originRef}
                    key={connection.connectionId}
                    connection={connection}
                    overview={overview}
                    mutationBusy={mutationBusy}
                    onAction={onAction}
                  />
                ))}
              </div>
            )}
          </>
        ) : (
          <EmptyState title="选择一个软件" />
        )}
        <PersistentSurface active={selectedConsumer === "codex"}>
          {codexSourceControls}
        </PersistentSurface>
        <PersistentSurface active={active && selectedConsumer === "grokbuild"}>
          <GrokOfficialLogin />
        </PersistentSurface>
      </CatalogDetail>
    </CatalogMasterDetail>
  );
}
