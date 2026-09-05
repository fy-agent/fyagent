import { useRef, useState } from "react";

import type {
  ManagedAuthAccountRemovalPreview,
  ManagedAuthAccountSummary,
  ManagedAuthConnectionAction,
  ManagedAuthConnectionSummary,
  ManagedAuthOverview,
} from "../../shared/features/managed-auth";
import {
  Button,
  Dialog,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { ProviderMark } from "./common";
import {
  managedAuthConsumerLabel,
  managedAuthProviderLabel,
  managedAuthReasonCopy,
  requestModeLabel,
} from "./presentation";

export function RemoveAccountDialog({
  account,
  preview,
  loading,
  pending,
  error,
  onCancel,
  onConfirm,
}: {
  account: ManagedAuthAccountSummary | null;
  preview: ManagedAuthAccountRemovalPreview | null;
  loading: boolean;
  pending: boolean;
  error: string | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  return (
    <Dialog
      open={account !== null}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={account ? `移除 ${account.login}？` : "移除账号"}
      description="账号只有在所有连接都安全处理后才会从列表中移除。"
      actions={
        <>
          <Button ref={cancelRef} disabled={pending} onClick={onCancel}>
            取消
          </Button>
          <Button
            className="fy-control-button-danger"
            disabled={pending || loading || preview?.canApply !== true}
            onClick={onConfirm}
          >
            {pending ? "正在移除…" : "移除账号"}
          </Button>
        </>
      }
    >
      {loading ? (
        <div className="fy-auth-dialog-loading">
          <Spinner label="正在检查账号影响" />
          <span>正在检查受影响的软件连接</span>
        </div>
      ) : preview ? (
        <div className="fy-auth-impact-preview">
          <section>
            <h3>将断开</h3>
            {preview.disconnects.length === 0 ? (
              <p>没有软件连接会被断开。</p>
            ) : (
              <ul>
                {preview.disconnects.map((impact) => (
                  <li
                    key={`${impact.consumer}:${impact.targetLabel ?? "default"}`}
                  >
                    <strong>{managedAuthConsumerLabel(impact.consumer)}</strong>
                    <span>
                      {impact.targetLabel ? `${impact.targetLabel} · ` : ""}
                      {requestModeLabel(impact.requestMode, null)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </section>
          {preview.preserved.length > 0 ? (
            <section>
              <h3>不会改变</h3>
              <ul>
                {preview.preserved.map((impact) => (
                  <li
                    key={`${impact.consumer}:${impact.targetLabel ?? "default"}`}
                  >
                    <strong>{managedAuthConsumerLabel(impact.consumer)}</strong>
                    <span>{requestModeLabel(impact.requestMode, null)}</span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {preview.reasonCodes.length > 0 ? (
            <InlineNotice tone="warning">
              {preview.reasonCodes.map(managedAuthReasonCopy).join("；")}
            </InlineNotice>
          ) : null}
        </div>
      ) : error ? (
        <InlineNotice tone="warning">{error}</InlineNotice>
      ) : null}
    </Dialog>
  );
}

function connectionActionCopy(
  connection: ManagedAuthConnectionSummary,
  action: ManagedAuthConnectionAction,
): { title: string; description: string } {
  const consumer = managedAuthConsumerLabel(connection.consumer);
  switch (action) {
    case "connect_account":
      return {
        title: `连接 ${consumer} 账号`,
        description: "选择一个用途匹配的官方账号。",
      };
    case "switch_account":
      return {
        title: `切换 ${consumer} 账号`,
        description: "切换前会再次确认软件状态；需要时会提示重新启动。",
      };
    case "disconnect":
      return {
        title: `断开 ${consumer} 的账号连接？`,
        description: "账号仍会保存在“账号”中，其他软件连接不受影响。",
      };
    case "switch_to_official":
      return {
        title: `切回 ${consumer} 官方模式？`,
        description:
          "将停止使用当前第三方 API，并继续使用已保存的官方账号登录。",
      };
    case "refresh":
    case "restart":
    case "open_consumer":
      return { title: consumer, description: "" };
  }
}

interface ConnectionActionDialogProps {
  connection: ManagedAuthConnectionSummary | null;
  action: ManagedAuthConnectionAction | null;
  overview: ManagedAuthOverview;
  pending: boolean;
  preferredAccountId?: string | null;
  onCancel: () => void;
  onConfirm: (accountId: string | null) => void;
}

export function ConnectionActionDialog(props: ConnectionActionDialogProps) {
  if (!props.connection || !props.action) return null;
  const key = `${props.connection.connectionId}:${props.action}:${props.preferredAccountId ?? ""}`;
  return (
    <ConnectionActionDialogContent
      key={key}
      {...props}
      connection={props.connection}
      action={props.action}
    />
  );
}

function ConnectionActionDialogContent({
  connection,
  action,
  overview,
  pending,
  preferredAccountId,
  onCancel,
  onConfirm,
}: Omit<ConnectionActionDialogProps, "connection" | "action"> & {
  connection: ManagedAuthConnectionSummary;
  action: ManagedAuthConnectionAction;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  const requiresAccount =
    action === "connect_account" || action === "switch_account";
  const candidates = connection.provider
    ? overview.accounts.filter(
        (account) =>
          account.provider === connection.provider &&
          account.health === "ready",
      )
    : overview.accounts.filter((account) => account.health === "ready");
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(
    requiresAccount
      ? (candidates.find((account) => account.accountId === preferredAccountId)
          ?.accountId ??
          candidates.find(
            (account) => account.accountId !== connection.accountId,
          )?.accountId ??
          candidates[0]?.accountId ??
          null)
      : null,
  );
  const copy = connectionActionCopy(connection, action);
  return (
    <Dialog
      open
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={copy.title}
      description={copy.description}
      actions={
        <>
          <Button ref={cancelRef} disabled={pending} onClick={onCancel}>
            取消
          </Button>
          <Button
            className={
              action === "disconnect" ? "fy-control-button-danger" : undefined
            }
            disabled={
              pending || (requiresAccount && selectedAccountId === null)
            }
            onClick={() => onConfirm(selectedAccountId)}
          >
            {pending
              ? "正在处理…"
              : action === "disconnect"
                ? "断开"
                : action === "switch_to_official"
                  ? "切换"
                  : "确认"}
          </Button>
        </>
      }
    >
      {requiresAccount ? (
        candidates.length === 0 ? (
          <InlineNotice tone="warning">
            没有可用的
            {connection.provider
              ? ` ${managedAuthProviderLabel(connection.provider)} `
              : " "}
            账号，请先添加或重新登录。
          </InlineNotice>
        ) : (
          <fieldset className="fy-auth-account-choice">
            <legend>选择账号</legend>
            {candidates.map((account) => (
              <label key={account.accountId}>
                <input
                  type="radio"
                  name="managed-auth-connection-account"
                  checked={selectedAccountId === account.accountId}
                  onChange={() => setSelectedAccountId(account.accountId)}
                />
                <ProviderMark provider={account.provider} />
                <span>
                  <strong>{account.login}</strong>
                  <small>{managedAuthProviderLabel(account.provider)}</small>
                </span>
              </label>
            ))}
          </fieldset>
        )
      ) : action === "disconnect" ? (
        <p>
          当前账号连接将被移除；
          {connection.requestMode === "third_party_api"
            ? "当前第三方 API 配置不会被删除。"
            : "其他软件的账号连接不会改变。"}
        </p>
      ) : action === "switch_to_official" ? (
        <p>
          当前模型来源：
          {requestModeLabel(
            connection.requestMode,
            connection.requestProviderLabel,
          )}
        </p>
      ) : null}
    </Dialog>
  );
}
