import { useRef, useState } from "react";

import type {
  ManagedAuthAccountRemovalPreview,
  ManagedAuthAccountSummary,
  ManagedAuthConnectionAction,
  ManagedAuthConnectionSummary,
  ManagedAuthOverview,
} from "../../shared/features/managed-auth";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";
import { ProviderMark } from "./common";
import { FileWriteDisclosure } from "../../shared/features/controls/FileWriteDisclosure";
import { useManagedAuthConnectionPreview } from "../../shared/features/queries";
import {
  managedAuthConsumerLabel,
  managedAuthProviderLabel,
  managedAuthReasonCopy,
  requestModeLabel,
} from "./presentation";

export function RemoveAccountDialog({
  originRef,
  account,
  preview,
  loading,
  pending,
  error,
  onCancel,
  onConfirm,
}: {
  account: ManagedAuthAccountSummary | null;
  originRef?: DialogOriginRef;
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
      originRef={originRef}
      open={account !== null}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={account ? `移除 ${account.login}？` : "移除账号"}
      description="将删除 FyAgent 保存的账号凭证及其连接记录。其他软件的认证和配置文件保持不变；重新使用此账号需要再次登录。"
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
        description:
          "请选择账号，并确认本次认证文件、模型来源和备份位置的影响。",
      };
    case "disconnect":
      return {
        title:
          connection.authStatus === "disconnected"
            ? `恢复 ${consumer} 的第三方模型来源？`
            : `断开 ${consumer} 的账号连接？`,
        description:
          connection.authStatus === "disconnected"
            ? "将取消注释顶层 model_provider，恢复原来的第三方模型来源。账号仍会保存在“账号”中。"
            : "账号仍会保存在“账号”中，其他软件连接不受影响。",
      };
    case "switch_to_official":
      return {
        title: `切回 ${consumer} 官方模式？`,
        description: "请通过模型来源设置切换；账号连接不会自动修改模型配置。",
      };
    case "refresh":
    case "restart":
    case "open_consumer":
      return { title: consumer, description: "" };
  }
}

interface ConnectionActionDialogProps {
  originRef?: DialogOriginRef;
  connection: ManagedAuthConnectionSummary | null;
  action: ManagedAuthConnectionAction | null;
  overview: ManagedAuthOverview;
  pending: boolean;
  preferredAccountId?: string | null;
  onCancel: () => void;
  onConfirm: (accountId: string | null, previewId: string) => void;
}

export function ConnectionActionDialog(props: ConnectionActionDialogProps) {
  return <ConnectionActionDialogContent {...props} />;
}

function ConnectionActionDialogContent({
  originRef,
  connection,
  action,
  overview,
  pending,
  preferredAccountId,
  onCancel,
  onConfirm,
}: ConnectionActionDialogProps) {
  const open = connection !== null && action !== null;
  const cancelRef = useRef<HTMLButtonElement>(null);
  const requiresAccount =
    action === "connect_account" || action === "switch_account";
  const candidates = connection?.provider
    ? overview.accounts.filter(
        (account) =>
          account.provider === connection.provider &&
          account.health === "ready",
      )
    : overview.accounts.filter((account) => account.health === "ready");
  const initialAccountId = requiresAccount
    ? (candidates.find((account) => account.accountId === preferredAccountId)
        ?.accountId ??
      candidates.find((account) => account.accountId !== connection?.accountId)
        ?.accountId ??
      candidates[0]?.accountId ??
      null)
    : null;
  const selectionScope = JSON.stringify([
    connection?.connectionId ?? null,
    action,
    preferredAccountId ?? null,
  ]);
  const [previousScope, setPreviousScope] = useState(selectionScope);
  const [selectedAccountId, setSelectedAccountId] = useState(initialAccountId);
  // Reset only the local selection, not the Dialog's presentation lifetime.
  // The guarded render adjustment prevents committing a stale preview request.
  if (previousScope !== selectionScope) {
    setPreviousScope(selectionScope);
    setSelectedAccountId(initialAccountId);
  }
  const copy =
    connection && action
      ? connectionActionCopy(connection, action)
      : { title: "软件连接", description: "" };
  const preview = useManagedAuthConnectionPreview(
    {
      connectionId: connection?.connectionId ?? "closed",
      expectedRevision: connection?.revision ?? "0",
      action: action ?? "refresh",
      accountId: selectedAccountId,
    },
    open && !pending && (!requiresAccount || selectedAccountId !== null),
  );
  const [consumedPreviewId, setConsumedPreviewId] = useState<string | null>(
    null,
  );
  const consumedPreviewRef = useRef<string | null>(null);
  // A preview is single-use across close/reopen too. A fresh native preview,
  // rather than clearing this guard, authorizes another confirmation.
  const canConfirm =
    open &&
    !pending &&
    !preview.isError &&
    !preview.isFetching &&
    preview.data?.canApply === true &&
    preview.data.previewId !== consumedPreviewId &&
    (!requiresAccount || selectedAccountId !== null);
  const confirmLabel = pending
    ? "正在处理…"
    : action === "disconnect"
      ? connection?.authStatus === "disconnected"
        ? "恢复"
        : "断开"
      : action === "switch_to_official"
        ? "切换"
        : "确认";
  return (
    <Dialog
      originRef={originRef}
      open={open}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={copy.title}
      description={copy.description || undefined}
      actions={
        open ? (
          <>
            <Button ref={cancelRef} disabled={pending} onClick={onCancel}>
              取消
            </Button>
            <Button
              className={
                action === "disconnect" ? "fy-control-button-danger" : undefined
              }
              disabled={!canConfirm}
              onClick={() => {
                if (
                  !canConfirm ||
                  !preview.data ||
                  consumedPreviewRef.current === preview.data.previewId
                )
                  return;
                consumedPreviewRef.current = preview.data.previewId;
                setConsumedPreviewId(preview.data.previewId);
                onConfirm(selectedAccountId, preview.data.previewId);
              }}
            >
              {confirmLabel}
            </Button>
          </>
        ) : undefined
      }
    >
      {open && connection && action ? (
        <>
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
                      disabled={pending}
                      checked={selectedAccountId === account.accountId}
                      onChange={() => setSelectedAccountId(account.accountId)}
                    />
                    <ProviderMark provider={account.provider} />
                    <span>
                      <strong>{account.login}</strong>
                      <small>
                        {managedAuthProviderLabel(account.provider)}
                      </small>
                    </span>
                  </label>
                ))}
              </fieldset>
            )
          ) : action === "disconnect" ? (
            <p>
              {connection.authStatus === "disconnected"
                ? "当前并未连接官方登录；确认后只恢复第三方模型来源，不会删除账号。"
                : connection.requestMode === "third_party_api"
                  ? "当前账号连接将被移除；当前第三方 API 配置不会被删除。"
                  : "当前账号连接将被移除；其他软件的账号连接不会改变。"}
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
          {preview.isFetching ? (
            <div className="fy-auth-dialog-loading">
              <Spinner label="正在检查文件影响" />
              <span>正在检查文件和备份位置</span>
            </div>
          ) : preview.data ? (
            <>
              <FileWriteDisclosure
                targets={preview.data.writeTargets}
                preservedPaths={preview.data.preservedPaths}
              />
              {connection.consumer === "codex" ? (
                preview.data.writeTargets.some((target) =>
                  target.path.endsWith("config.toml"),
                ) ? (
                  <p>
                    {action === "disconnect"
                      ? "断开后会取消注释顶层 model_provider，恢复原来的第三方模型来源；[model_providers.*] 表保持不变。"
                      : "官方连接会注释顶层 model_provider，让 Codex 使用内建官方登录；[model_providers.*] 表保持不变。"}
                  </p>
                ) : action === "connect_account" ||
                  action === "switch_account" ? (
                  <p>
                    顶层 model_provider 已是注释状态，Codex
                    将走官方登录；提供商表和其它配置保持不变。
                  </p>
                ) : null
              ) : null}
              {preview.data.previewId === consumedPreviewId && !pending ? (
                <InlineNotice tone="warning">
                  此次确认已使用。请关闭后重新打开，检查最新状态再操作。
                </InlineNotice>
              ) : null}
            </>
          ) : preview.isError ? (
            <InlineNotice tone="warning">
              无法确认文件影响。请检查账号用途及软件状态，关闭后重新打开；尚未修改任何文件。
            </InlineNotice>
          ) : null}
        </>
      ) : null}
    </Dialog>
  );
}
