import type {
  ManagedAuthAccountSummary,
  ManagedAuthConnectionAction,
  ManagedAuthConnectionSummary,
  ManagedAuthConsumer,
  ManagedAuthOverview,
  ManagedAuthProvider,
} from "../../shared/features/managed-auth";
import {
  CatalogDetail,
  CatalogList,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { EmptyState, Input, InlineNotice } from "../../shared/ui/primitives";
import {
  AccountHealthBadge,
  AuthListItem,
  DefinitionRow,
  ProviderMark,
  ReasonList,
  StatusBadge,
} from "./common";
import {
  accountHealthPresentation,
  accountPageConnectionActionLabel,
  accountPageConnectionActions,
  connectableConnectionsForAccount,
  connectionStatusPresentation,
  loginRequiredConnectionsForAccount,
  formatAuthenticatedAt,
  managedAuthConsumerLabel,
  managedAuthManagerLabel,
  managedAuthProviderLabel,
  requestModeLabel,
  sortManagedAuthAccounts,
} from "./presentation";

const PROVIDER_FILTERS: Array<ManagedAuthProvider | "all"> = [
  "all",
  "openai",
  "xai",
  "github_copilot",
];

function AccountConnection({
  originRef,
  connection,
  mutationBusy,
  actions,
  onAction,
  loginConnectLabel,
  onLoginConnect,
}: {
  connection: ManagedAuthConnectionSummary;
  mutationBusy: boolean;
  actions: ManagedAuthConnectionAction[];
  onAction: (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
  ) => void;
  loginConnectLabel?: string;
  originRef?: DialogOriginRef;
  onLoginConnect?: () => void;
}) {
  const status = connectionStatusPresentation(
    connection.authStatus,
    connection.reasonCodes,
  );
  return (
    <article className="fy-auth-connection-card">
      <div className="fy-auth-connection-card-heading">
        <div>
          <h4>{managedAuthConsumerLabel(connection.consumer)}</h4>
          {connection.targetLabel ? <p>{connection.targetLabel}</p> : null}
        </div>
        <StatusBadge {...status} />
      </div>
      <dl className="fy-feature-definition">
        <DefinitionRow label="当前请求">
          {requestModeLabel(
            connection.requestMode,
            connection.requestProviderLabel,
          )}
        </DefinitionRow>
        <DefinitionRow label="官方登录">
          {connection.officialSessionPreserved === null
            ? "不适用"
            : connection.officialSessionPreserved
              ? "已保留"
              : "未确认保留"}
        </DefinitionRow>
        <DefinitionRow label="自动续期">
          {managedAuthManagerLabel(connection.credentialManager)}
        </DefinitionRow>
      </dl>
      <ReasonList reasons={connection.reasonCodes} />
      {actions.length > 0 || loginConnectLabel ? (
        <div className="fy-feature-actions">
          {actions.map((action) => (
            <Button
              key={action}
              dialogOriginRef={originRef}
              disabled={mutationBusy}
              onClick={() => onAction(connection, action)}
            >
              {accountPageConnectionActionLabel(action)}
            </Button>
          ))}
          {loginConnectLabel ? (
            <Button
              dialogOriginRef={originRef}
              disabled={mutationBusy}
              onClick={onLoginConnect}
            >
              {loginConnectLabel}
            </Button>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

function AccountDetail({
  originRef,
  account,
  connections,
  mutationBusy,
  onBack,
  onReauthenticate,
  onSetDefault,
  onRemove,
  onConnectionAction,
  onConnectViaLogin,
}: {
  account: ManagedAuthAccountSummary;
  originRef?: DialogOriginRef;
  connections: ManagedAuthConnectionSummary[];
  mutationBusy: boolean;
  onBack: () => void;
  onReauthenticate: (account: ManagedAuthAccountSummary) => void;
  onSetDefault: (account: ManagedAuthAccountSummary) => void;
  onRemove: (account: ManagedAuthAccountSummary) => void;
  onConnectionAction: (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
  ) => void;
  onConnectViaLogin: (connection: ManagedAuthConnectionSummary) => void;
}) {
  const canReauthenticate = account.allowedActions.includes("reauthenticate");
  const canSetDefault = account.allowedActions.includes("set_default");
  const canRemove = account.allowedActions.includes("remove");
  const linkedConnections = connections.filter(
    (connection) => connection.accountId === account.accountId,
  );
  const connectableConnections = connectableConnectionsForAccount(
    account,
    connections,
  );
  const loginRequiredConnections = loginRequiredConnectionsForAccount(
    account,
    connections,
  );
  return (
    <CatalogDetail
      ariaLabel={`${account.login} 账号详情`}
      className="fy-auth-detail"
    >
      <Button className="fy-auth-mobile-back" onClick={onBack}>
        返回账号列表
      </Button>
      <header className="fy-auth-detail-header">
        <ProviderMark provider={account.provider} size="detail" />
        <div className="fy-auth-detail-title">
          <div className="fy-auth-detail-title-line">
            <h2 title={account.login}>{account.login}</h2>
            {account.isDefault ? (
              <StatusBadge label="默认账号" tone="neutral" />
            ) : null}
            <AccountHealthBadge health={account.health} />
          </div>
          <p>
            {managedAuthProviderLabel(account.provider)}
            {account.planSummary ? ` · ${account.planSummary}` : ""}
          </p>
        </div>
      </header>

      <ReasonList reasons={account.reasonCodes} />

      <section
        className="fy-auth-section"
        aria-labelledby="fy-auth-account-status"
      >
        <div className="fy-auth-section-heading">
          <div>
            <h3 id="fy-auth-account-status">账号状态</h3>
            <p>登录状态与软件连接分别管理。</p>
          </div>
          <div className="fy-feature-actions">
            {canReauthenticate ? (
              <Button
                disabled={mutationBusy}
                onClick={() => onReauthenticate(account)}
                dialogOriginRef={originRef}
              >
                重新登录
              </Button>
            ) : null}
            {!account.isDefault && canSetDefault ? (
              <Button
                disabled={mutationBusy}
                onClick={() => onSetDefault(account)}
              >
                设为默认
              </Button>
            ) : null}
          </div>
        </div>
        <dl className="fy-feature-definition fy-auth-definition">
          <DefinitionRow label="账号名称">
            {account.displayName ?? "未设置"}
          </DefinitionRow>
          <DefinitionRow label="上次认证">
            {formatAuthenticatedAt(account.lastAuthenticatedAt)}
          </DefinitionRow>
          <DefinitionRow label="软件连接">
            {account.connectedConsumerCount === 0
              ? "尚未连接软件"
              : `已连接 ${account.connectedConsumerCount} 个软件`}
          </DefinitionRow>
          <DefinitionRow label="额度状态">
            {account.quotaSummary ?? "暂时没有额度信息"}
          </DefinitionRow>
        </dl>
      </section>

      <section
        className="fy-auth-section"
        aria-labelledby="fy-auth-account-connections"
      >
        <div className="fy-auth-section-heading">
          <div>
            <h3 id="fy-auth-account-connections">已连接软件</h3>
            <p>这里显示账号连接和软件当前请求来源，两者可能不同。</p>
          </div>
        </div>
        {linkedConnections.length === 0 ? (
          <InlineNotice>此账号尚未连接任何软件。</InlineNotice>
        ) : (
          <div className="fy-auth-connection-grid">
            {linkedConnections.map((connection) => (
              <AccountConnection
                key={connection.connectionId}
                connection={connection}
                mutationBusy={mutationBusy}
                actions={accountPageConnectionActions(
                  connection,
                  account.accountId,
                )}
                onAction={onConnectionAction}
                originRef={originRef}
              />
            ))}
          </div>
        )}
      </section>

      {connectableConnections.length > 0 ||
      loginRequiredConnections.length > 0 ? (
        <section
          className="fy-auth-section"
          aria-labelledby="fy-auth-account-connect"
        >
          <div className="fy-auth-section-heading">
            <div>
              <h3 id="fy-auth-account-connect">连接到软件</h3>
              <p>
                登录成功后不会自动改写软件，需要在这里选择要使用此账号的软件。
              </p>
            </div>
          </div>
          <div className="fy-auth-connection-grid">
            {connectableConnections.map((connection) => (
              <AccountConnection
                key={connection.connectionId}
                connection={connection}
                mutationBusy={mutationBusy}
                actions={accountPageConnectionActions(
                  connection,
                  account.accountId,
                )}
                onAction={onConnectionAction}
                originRef={originRef}
              />
            ))}
            {loginRequiredConnections.map((connection) => (
              <AccountConnection
                key={connection.connectionId}
                connection={connection}
                mutationBusy={mutationBusy}
                actions={[]}
                onAction={onConnectionAction}
                loginConnectLabel={`连接 ${managedAuthConsumerLabel(connection.consumer)}`}
                originRef={originRef}
                onLoginConnect={() => onConnectViaLogin(connection)}
              />
            ))}
          </div>
        </section>
      ) : null}

      <section
        className="fy-auth-section fy-auth-danger-section"
        aria-labelledby="fy-auth-danger-zone"
      >
        <div className="fy-auth-section-heading">
          <div>
            <h3 id="fy-auth-danger-zone">危险操作</h3>
            <p>移除账号前会先展示受影响的软件连接。</p>
          </div>
          {canRemove ? (
            <Button
              className="fy-control-button-danger"
              disabled={mutationBusy}
              onClick={() => onRemove(account)}
              dialogOriginRef={originRef}
            >
              移除账号
            </Button>
          ) : null}
        </div>
      </section>
    </CatalogDetail>
  );
}

export function AccountView({
  originRef,
  overview,
  selectedAccountId,
  preferredConsumer,
  search,
  providerFilter,
  mutationBusy,
  onSearchChange,
  onProviderFilterChange,
  onSelectAccount,
  onClearSelection,
  onAddAccount,
  onReauthenticate,
  onSetDefault,
  onRemove,
  onConnectionAction,
  onConnectViaLogin,
}: {
  overview: ManagedAuthOverview;
  originRef?: DialogOriginRef;
  selectedAccountId: string | null;
  preferredConsumer: ManagedAuthConsumer | null;
  search: string;
  providerFilter: ManagedAuthProvider | "all";
  mutationBusy: boolean;
  onSearchChange: (value: string) => void;
  onProviderFilterChange: (value: ManagedAuthProvider | "all") => void;
  onSelectAccount: (accountId: string) => void;
  onClearSelection: () => void;
  onAddAccount: () => void;
  onReauthenticate: (account: ManagedAuthAccountSummary) => void;
  onSetDefault: (account: ManagedAuthAccountSummary) => void;
  onRemove: (account: ManagedAuthAccountSummary) => void;
  onConnectionAction: (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
  ) => void;
  onConnectViaLogin: (connection: ManagedAuthConnectionSummary) => void;
}) {
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const sortedAccounts = sortManagedAuthAccounts(
    overview.accounts,
    overview.connections,
    preferredConsumer,
  );
  const accounts = sortedAccounts.filter((account) => {
    if (providerFilter !== "all" && account.provider !== providerFilter) {
      return false;
    }
    if (!normalizedSearch) return true;
    return [account.login, account.displayName, account.planSummary]
      .filter((value): value is string => value !== null)
      .some((value) => value.toLocaleLowerCase().includes(normalizedSearch));
  });
  const selectedAccount = overview.accounts.find(
    (account) => account.accountId === selectedAccountId,
  );

  if (overview.accounts.length === 0) {
    return (
      <EmptyState
        title={
          preferredConsumer === "codex"
            ? "Codex 还没有可用的 OpenAI 账号"
            : "还没有官方账号"
        }
        description={
          preferredConsumer === "codex"
            ? "添加账号后可以直接连接到 Codex。"
            : "添加 OpenAI、xAI 或 GitHub Copilot 账号后，可以在支持的软件之间管理连接。"
        }
        actions={
          <Button dialogOriginRef={originRef} onClick={onAddAccount}>
            添加账号
          </Button>
        }
      />
    );
  }

  return (
    <CatalogMasterDetail className="fy-auth-master-detail">
      <CatalogRail
        ariaLabel="官方账号列表"
        title="账号"
        meta={`${accounts.length} / ${overview.accounts.length}`}
        className="fy-auth-rail"
      >
        <div className="fy-auth-account-toolbar">
          <Input
            type="search"
            value={search}
            onChange={(event) => onSearchChange(event.currentTarget.value)}
            placeholder="搜索账号"
            aria-label="搜索账号"
          />
          <div className="fy-auth-filter-row" aria-label="按账号类型筛选">
            {PROVIDER_FILTERS.map((provider) => (
              <button
                key={provider}
                type="button"
                aria-pressed={providerFilter === provider}
                onClick={() => onProviderFilterChange(provider)}
              >
                {provider === "all"
                  ? "全部"
                  : managedAuthProviderLabel(provider)}
              </button>
            ))}
          </div>
        </div>
        {accounts.length === 0 ? (
          <InlineNotice>没有符合当前筛选条件的账号。</InlineNotice>
        ) : (
          <CatalogList>
            {accounts.map((account) => {
              const health = accountHealthPresentation(account.health);
              const consumers = overview.connections
                .filter(
                  (connection) => connection.accountId === account.accountId,
                )
                .map((connection) =>
                  managedAuthConsumerLabel(connection.consumer),
                );
              return (
                <AuthListItem
                  key={account.accountId}
                  selected={account.accountId === selectedAccountId}
                  label={account.login}
                  leading={<ProviderMark provider={account.provider} />}
                  summary={
                    account.health === "ready"
                      ? consumers.length > 0
                        ? `已连接：${[...new Set(consumers)].join("、")}`
                        : "尚未连接软件"
                      : health.label
                  }
                  trailing={
                    account.isDefault ? (
                      <StatusBadge label="默认" tone="neutral" />
                    ) : undefined
                  }
                  onSelect={() => onSelectAccount(account.accountId)}
                  testId={`managed-auth-account-${account.accountId}`}
                />
              );
            })}
          </CatalogList>
        )}
      </CatalogRail>
      {selectedAccount ? (
        <AccountDetail
          originRef={originRef}
          account={selectedAccount}
          connections={overview.connections}
          mutationBusy={mutationBusy}
          onBack={onClearSelection}
          onReauthenticate={onReauthenticate}
          onSetDefault={onSetDefault}
          onRemove={onRemove}
          onConnectionAction={onConnectionAction}
          onConnectViaLogin={onConnectViaLogin}
        />
      ) : (
        <CatalogDetail ariaLabel="账号详情" className="fy-auth-detail">
          <EmptyState
            title="选择一个账号"
            description="查看登录状态、软件连接和账号操作。"
          />
        </CatalogDetail>
      )}
    </CatalogMasterDetail>
  );
}
