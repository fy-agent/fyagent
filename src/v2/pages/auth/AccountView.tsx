import type {
  ManagedAuthAccountSummary,
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
import {
  Button,
  EmptyState,
  Input,
  InlineNotice,
} from "../../shared/ui/primitives";
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
  connectionStatusPresentation,
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
  connection,
}: {
  connection: ManagedAuthConnectionSummary;
}) {
  const status = connectionStatusPresentation(connection.authStatus);
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
    </article>
  );
}

function AccountDetail({
  account,
  connections,
  mutationBusy,
  onBack,
  onReauthenticate,
  onSetDefault,
  onRemove,
}: {
  account: ManagedAuthAccountSummary;
  connections: ManagedAuthConnectionSummary[];
  mutationBusy: boolean;
  onBack: () => void;
  onReauthenticate: (account: ManagedAuthAccountSummary) => void;
  onSetDefault: (account: ManagedAuthAccountSummary) => void;
  onRemove: (account: ManagedAuthAccountSummary) => void;
}) {
  const canReauthenticate = account.allowedActions.includes("reauthenticate");
  const canSetDefault = account.allowedActions.includes("set_default");
  const canRemove = account.allowedActions.includes("remove");
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
        {connections.length === 0 ? (
          <InlineNotice>此账号尚未连接任何软件。</InlineNotice>
        ) : (
          <div className="fy-auth-connection-grid">
            {connections.map((connection) => (
              <AccountConnection
                key={connection.connectionId}
                connection={connection}
              />
            ))}
          </div>
        )}
      </section>

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
}: {
  overview: ManagedAuthOverview;
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
  const selectedConnections = selectedAccount
    ? overview.connections.filter(
        (connection) => connection.accountId === selectedAccount.accountId,
      )
    : [];

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
        actions={<Button onClick={onAddAccount}>添加账号</Button>}
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
          account={selectedAccount}
          connections={selectedConnections}
          mutationBusy={mutationBusy}
          onBack={onClearSelection}
          onReauthenticate={onReauthenticate}
          onSetDefault={onSetDefault}
          onRemove={onRemove}
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
