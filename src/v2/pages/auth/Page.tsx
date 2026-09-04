import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";

import { errorMessage } from "../../shared/features/helpers";
import {
  MANAGED_AUTH_CONSUMERS,
  MANAGED_AUTH_PROVIDERS,
  type ManagedAuthAccountRemovalPreview,
  type ManagedAuthAccountSummary,
  type ManagedAuthConnectionAction,
  type ManagedAuthConnectionSummary,
  type ManagedAuthConsumer,
  type ManagedAuthMutationResult,
  type ManagedAuthProvider,
} from "../../shared/features/managed-auth";
import { useFeatures } from "../../shared/features/provider";
import {
  featureKeys,
  useManagedAuthOverview,
} from "../../shared/features/queries";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { usePersistentSearchParams } from "../../shared/ui/usePersistentSearchParams";
import {
  Button,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { AccountView } from "./AccountView";
import { ConnectionsView } from "./ConnectionsView";
import { LoginDialog } from "./LoginDialog";
import { ConnectionActionDialog, RemoveAccountDialog } from "./MutationDialogs";
import { ReasonList } from "./common";
import {
  managedAuthCommandErrorCopy,
  managedAuthConsumerLabel,
  sessionSummary,
} from "./presentation";
import { useManagedAuthLoginSession } from "./useManagedAuthLoginSession";
import "./page.css";

type AuthView = "accounts" | "connections";

const AUTH_VIEWS: ReadonlyArray<{ id: AuthView; label: string }> = [
  { id: "accounts", label: "账号" },
  { id: "connections", label: "软件连接" },
];

function authView(value: string | null): AuthView | null {
  return value === "accounts" || value === "connections" ? value : null;
}

function authConsumer(value: string | null): ManagedAuthConsumer | null {
  return value !== null &&
    MANAGED_AUTH_CONSUMERS.includes(value as ManagedAuthConsumer)
    ? (value as ManagedAuthConsumer)
    : null;
}

function consumerForAgent(value: string | null): ManagedAuthConsumer | null {
  switch (value) {
    case "codex":
      return "codex";
    case "grokbuild":
      return "grokbuild";
    case "opencode":
      return "opencode";
    default:
      return null;
  }
}

export function AuthPage() {
  const queryClient = useQueryClient();
  const { ports, notify } = useFeatures();
  const { visible, searchParams, setSearchParams } =
    usePersistentSearchParams();
  const overviewQuery = useManagedAuthOverview(visible);
  const requestedConsumer =
    authConsumer(searchParams.get("consumer")) ??
    consumerForAgent(searchParams.get("agentReturn"));
  const requestedView = authView(searchParams.get("view"));
  const view: AuthView =
    requestedView ?? (requestedConsumer ? "connections" : "accounts");
  const requestedAccountId = searchParams.get("account");
  const [accountSearch, setAccountSearch] = useState("");
  const [providerFilter, setProviderFilter] = useState<
    ManagedAuthProvider | "all"
  >("all");
  const [loginOpen, setLoginOpen] = useState(false);
  const [loginAccount, setLoginAccount] =
    useState<ManagedAuthAccountSummary | null>(null);
  const [loginConsumer, setLoginConsumer] =
    useState<ManagedAuthConsumer | null>(null);
  const [mutationBusy, setMutationBusy] = useState(false);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [removalAccount, setRemovalAccount] =
    useState<ManagedAuthAccountSummary | null>(null);
  const [removalPreview, setRemovalPreview] =
    useState<ManagedAuthAccountRemovalPreview | null>(null);
  const [removalPreviewLoading, setRemovalPreviewLoading] = useState(false);
  const [connectionAction, setConnectionAction] = useState<{
    connection: ManagedAuthConnectionSummary;
    action: ManagedAuthConnectionAction;
    preferredAccountId?: string | null;
  } | null>(null);

  const refetchOverview = overviewQuery.refetch;
  const handleLoginTerminal = useCallback(() => {
    void refetchOverview();
  }, [refetchOverview]);
  const loginController = useManagedAuthLoginSession({
    port: ports.managedAuth,
    active: visible,
    onTerminal: handleLoginTerminal,
  });
  const resumableSession = overviewQuery.data?.activeSessions[0] ?? null;
  const resumeLoginSession = loginController.resume;
  const currentLoginSessionId = loginController.snapshot?.sessionId;
  const currentLoginStage = loginController.snapshot?.stage;

  useEffect(() => {
    if (!resumableSession) return;
    if (
      currentLoginSessionId === resumableSession.sessionId &&
      currentLoginStage === resumableSession.stage
    ) {
      return;
    }
    resumeLoginSession(resumableSession);
  }, [
    currentLoginSessionId,
    currentLoginStage,
    resumableSession,
    resumeLoginSession,
  ]);

  const updateRoute = useCallback(
    (updates: Record<string, string | null>) => {
      setSearchParams((current) => {
        const next = new URLSearchParams(current);
        for (const [key, value] of Object.entries(updates)) {
          if (value === null) next.delete(key);
          else next.set(key, value);
        }
        return next;
      });
    },
    [setSearchParams],
  );

  const commitMutationResult = useCallback(
    (result: ManagedAuthMutationResult, successTitle: string) => {
      queryClient.setQueryData(
        featureKeys.managedAuthOverview,
        result.overview,
      );
      setMutationError(null);
      notify({
        tone: result.outcome === "completed" ? "success" : "info",
        title: successTitle,
        description:
          result.pendingRestartConsumers.length > 0
            ? `${result.pendingRestartConsumers
                .map(managedAuthConsumerLabel)
                .join("、")} 需要重新启动。`
            : undefined,
      });
    },
    [notify, queryClient],
  );

  const runMutation = async (
    action: () => Promise<ManagedAuthMutationResult>,
    successTitle: string,
  ) => {
    setMutationBusy(true);
    setMutationError(null);
    try {
      const result = await action();
      commitMutationResult(result, successTitle);
      return result;
    } catch (cause) {
      const message = managedAuthCommandErrorCopy(cause);
      setMutationError(message);
      notify({ tone: "error", title: "账号操作未完成", description: message });
      return null;
    } finally {
      setMutationBusy(false);
    }
  };

  const openLogin = (
    account: ManagedAuthAccountSummary | null,
    consumer: ManagedAuthConsumer | null,
  ) => {
    if (loginController.snapshot && !loginController.snapshot.terminal) {
      setLoginOpen(true);
      return;
    }
    loginController.reset();
    setLoginAccount(account);
    setLoginConsumer(consumer);
    setLoginOpen(true);
  };

  const closeLogin = (nextOpen: boolean) => {
    setLoginOpen(nextOpen);
    if (!nextOpen && loginController.snapshot?.terminal) {
      loginController.reset();
      setLoginAccount(null);
      setLoginConsumer(null);
    }
  };

  const finishLogin = () => {
    setLoginOpen(false);
    setLoginAccount(null);
    setLoginConsumer(null);
    void overviewQuery.refetch();
  };

  const setDefaultAccount = async (account: ManagedAuthAccountSummary) => {
    await runMutation(
      () =>
        ports.managedAuth.setDefaultAccount(
          account.accountId,
          account.revision,
        ),
      `${account.login} 已设为默认账号`,
    );
  };

  const beginRemoveAccount = async (account: ManagedAuthAccountSummary) => {
    setRemovalAccount(account);
    setRemovalPreview(null);
    setRemovalPreviewLoading(true);
    setMutationError(null);
    try {
      const preview = await ports.managedAuth.previewAccountRemoval(
        account.accountId,
        account.revision,
      );
      setRemovalPreview(preview);
    } catch (cause) {
      setMutationError(managedAuthCommandErrorCopy(cause));
    } finally {
      setRemovalPreviewLoading(false);
    }
  };

  const confirmRemoveAccount = async () => {
    if (!removalAccount || !removalPreview) return;
    const result = await runMutation(
      () =>
        ports.managedAuth.removeAccount(
          removalPreview.previewId,
          removalAccount.accountId,
          removalPreview.expectedRevision,
        ),
      `${removalAccount.login} 已移除`,
    );
    if (!result) return;
    setRemovalAccount(null);
    setRemovalPreview(null);
    if (requestedAccountId === removalAccount.accountId) {
      updateRoute({ account: null });
    }
  };

  const applyConnectionAction = async (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
    accountId: string | null,
  ) => {
    const result = await runMutation(
      () =>
        ports.managedAuth.applyConnectionAction({
          connectionId: connection.connectionId,
          expectedRevision: connection.revision,
          action,
          accountId,
        }),
      `${managedAuthConsumerLabel(connection.consumer)} 状态已更新`,
    );
    if (result) setConnectionAction(null);
  };

  const requestConnectionAction = (
    connection: ManagedAuthConnectionSummary,
    action: ManagedAuthConnectionAction,
    preferredAccountId?: string | null,
  ) => {
    if (
      action === "refresh" ||
      action === "restart" ||
      action === "open_consumer"
    ) {
      void applyConnectionAction(connection, action, null);
      return;
    }
    setConnectionAction({ connection, action, preferredAccountId });
  };

  if (overviewQuery.isPending) {
    return (
      <div
        className="fy-feature-page fy-auth-page"
        aria-label="账号与认证"
        data-testid="auth-page"
      >
        <EmptyState
          title="正在加载账号与认证"
          description="正在读取账号和软件连接"
        >
          <Spinner label="正在加载账号与认证" />
        </EmptyState>
      </div>
    );
  }

  if (overviewQuery.isError || !overviewQuery.data) {
    return (
      <div
        className="fy-feature-page fy-auth-page"
        aria-label="账号与认证"
        data-testid="auth-page"
      >
        <EmptyState
          title="无法加载账号与认证"
          description={errorMessage(overviewQuery.error)}
          actions={
            <Button onClick={() => void overviewQuery.refetch()}>重试</Button>
          }
        />
      </div>
    );
  }

  const overview = overviewQuery.data;
  const selectedAccountId =
    overview.accounts.find(
      (account) => account.accountId === requestedAccountId,
    )?.accountId ??
    overview.accounts[0]?.accountId ??
    null;
  const selectedConsumer =
    requestedConsumer ?? MANAGED_AUTH_CONSUMERS[0] ?? null;
  const needsAttention =
    overview.accounts.some((account) => account.health !== "ready") ||
    overview.connections.some(
      (connection) =>
        connection.authStatus !== "connected" &&
        connection.authStatus !== "disconnected",
    );
  const mobileDetailSelected =
    view === "accounts"
      ? requestedAccountId !== null && selectedAccountId !== null
      : searchParams.get("consumer") !== null && selectedConsumer !== null;

  return (
    <div
      className="fy-feature-page fy-split-page fy-auth-page"
      aria-label="账号与认证"
      data-testid="auth-page"
      data-view={view}
      data-mobile-detail={mobileDetailSelected ? "true" : "false"}
    >
      <header className="fy-feature-header fy-auth-page-header">
        <div>
          <h1>账号与认证</h1>
          <p>管理官方账号、软件连接和每个软件当前使用的模型来源。</p>
        </div>
        <div className="fy-feature-actions">
          <Button
            disabled={mutationBusy || loginController.busy}
            onClick={() => openLogin(null, requestedConsumer)}
          >
            添加账号
          </Button>
        </div>
      </header>

      <div className="fy-auth-overview-strip">
        <FeatureTabs
          id="managed-auth-views"
          label="账号与认证视图"
          value={view}
          options={AUTH_VIEWS.map((option) => ({
            ...option,
            label:
              option.id === "accounts"
                ? `账号 ${overview.accounts.length}`
                : `软件连接 ${overview.connections.length}`,
          }))}
          onChange={(next) =>
            updateRoute({
              view: next,
              account: next === "accounts" ? requestedAccountId : null,
              consumer:
                next === "connections" ? (requestedConsumer ?? "codex") : null,
            })
          }
        />
        <span data-attention={needsAttention ? "true" : undefined}>
          {needsAttention ? "有状态需要处理" : "账号状态正常"}
        </span>
      </div>

      {mutationError ? (
        <InlineNotice tone="warning">{mutationError}</InlineNotice>
      ) : null}
      {overview.reasonCodes.length > 0 ? (
        <InlineNotice tone="warning">
          <span className="fy-auth-session-banner">
            <ReasonList reasons={overview.reasonCodes} />
            <Button onClick={() => void refetchOverview()}>刷新状态</Button>
          </span>
        </InlineNotice>
      ) : null}
      {loginController.snapshot && !loginOpen ? (
        <InlineNotice
          tone={loginController.snapshot.terminal ? "warning" : "info"}
        >
          <span className="fy-auth-session-banner">
            <span>{sessionSummary(loginController.snapshot)}</span>
            <Button onClick={() => setLoginOpen(true)}>
              {loginController.snapshot.terminal ? "查看结果" : "继续登录"}
            </Button>
          </span>
        </InlineNotice>
      ) : null}

      <FeatureTabPanel
        tabsId="managed-auth-views"
        value="accounts"
        active={view === "accounts"}
        className="fy-auth-view-panel"
      >
        <AccountView
          overview={overview}
          selectedAccountId={selectedAccountId}
          preferredConsumer={requestedConsumer}
          search={accountSearch}
          providerFilter={providerFilter}
          mutationBusy={mutationBusy || loginController.busy}
          onSearchChange={setAccountSearch}
          onProviderFilterChange={(next) => {
            if (next === "all" || MANAGED_AUTH_PROVIDERS.includes(next)) {
              setProviderFilter(next);
            }
          }}
          onSelectAccount={(accountId) => updateRoute({ account: accountId })}
          onClearSelection={() => updateRoute({ account: null })}
          onAddAccount={() => openLogin(null, requestedConsumer)}
          onReauthenticate={(account) => openLogin(account, null)}
          onSetDefault={(account) => void setDefaultAccount(account)}
          onRemove={(account) => void beginRemoveAccount(account)}
          onConnectionAction={(connection, action) =>
            requestConnectionAction(connection, action, selectedAccountId)
          }
          onConnectViaLogin={(connection) => {
            openLogin(null, connection.consumer);
          }}
        />
      </FeatureTabPanel>

      <FeatureTabPanel
        tabsId="managed-auth-views"
        value="connections"
        active={view === "connections"}
        className="fy-auth-view-panel"
      >
        <ConnectionsView
          overview={overview}
          selectedConsumer={selectedConsumer}
          mutationBusy={mutationBusy || loginController.busy}
          onSelectConsumer={(consumer) =>
            updateRoute({ consumer, view: "connections" })
          }
          onClearSelection={() => updateRoute({ consumer: null })}
          onAction={requestConnectionAction}
        />
      </FeatureTabPanel>

      <LoginDialog
        open={loginOpen}
        providers={overview.providers}
        initialConsumer={loginConsumer}
        reauthenticateAccount={loginAccount}
        controller={loginController}
        onOpenChange={closeLogin}
        onFinished={finishLogin}
      />
      <RemoveAccountDialog
        account={removalAccount}
        preview={removalPreview}
        loading={removalPreviewLoading}
        pending={mutationBusy}
        error={mutationError}
        onCancel={() => {
          setRemovalAccount(null);
          setRemovalPreview(null);
          setMutationError(null);
        }}
        onConfirm={() => void confirmRemoveAccount()}
      />
      <ConnectionActionDialog
        connection={connectionAction?.connection ?? null}
        action={connectionAction?.action ?? null}
        overview={overview}
        pending={mutationBusy}
        preferredAccountId={connectionAction?.preferredAccountId}
        onCancel={() => setConnectionAction(null)}
        onConfirm={(accountId) => {
          if (!connectionAction) return;
          void applyConnectionAction(
            connectionAction.connection,
            connectionAction.action,
            accountId,
          );
        }}
      />
    </div>
  );
}

export default AuthPage;
