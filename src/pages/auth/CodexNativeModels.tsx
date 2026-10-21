import { useId } from "react";

import { useCodexNativeModels } from "../../shared/features/queries";
import type {
  ManagedAuthAccountSummary,
  ManagedAuthCodexNativeModels,
} from "../../shared/features/managed-auth";
import { Button } from "../../shared/ui/Button";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";
import {
  codexNativeBindingCopy,
  codexNativeFailureCopy,
  managedAuthCommandErrorCopy,
} from "./presentation";

function readAt(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function CatalogResult({
  result,
  accounts,
}: {
  result: ManagedAuthCodexNativeModels;
  accounts: ManagedAuthAccountSummary[];
}) {
  if (result.accountBinding === "mismatch") {
    const native = accounts.find(
      (account) => account.accountId === result.nativeAccountId,
    );
    return (
      <InlineNotice tone="warning">
        本机 Codex 当前登录的是
        {native ? `「${native.login}」` : "另一个 ChatGPT 账号"}
        ，不是此账号，所以没有读取目录。把此账号连接到 Codex 后再获取。
      </InlineNotice>
    );
  }
  if (result.accountBinding === "unknown") {
    return (
      <InlineNotice tone="warning">
        {codexNativeBindingCopy(result.bindingDetail!)}
      </InlineNotice>
    );
  }
  if (result.catalogStatus === "failed") {
    return (
      <InlineNotice tone="warning">
        {codexNativeFailureCopy(result.failure!)}
      </InlineNotice>
    );
  }
  const official = result.providerRoute === "official";
  return (
    <>
      <p className="fy-auth-native-models-meta">
        来源：
        {official
          ? "本机 Codex 原生官方目录"
          : "本机 Codex app-server 返回的目录"}{" "}
        · Codex CLI {result.cliVersion ?? "版本未知"} · 读取于{" "}
        {readAt(result.checkedAt)}
      </p>
      {official ? null : (
        <InlineNotice tone="warning">
          {result.providerRoute === "custom"
            ? "Codex 当前把模型请求发给自定义模型服务（config.toml 的 model_provider），未确认这些模型对应此 ChatGPT 账号的官方可用模型。"
            : "无法确认 Codex 当前的模型请求路由，未确认这些模型对应此 ChatGPT 账号的官方可用模型。"}
        </InlineNotice>
      )}
      {result.models.length === 0 ? (
        <InlineNotice tone="warning">
          本机 Codex 没有返回可选模型。可以在 Codex 模型配置中手动填写。
        </InlineNotice>
      ) : (
        <ul className="fy-auth-native-model-list">
          {result.models.map((model) => (
            <li key={model.id}>
              <span>{model.displayName ?? model.id}</span>
              {model.displayName && model.displayName !== model.id ? (
                <code>{model.id}</code>
              ) : null}
              {model.isDefault ? <em>默认</em> : null}
            </li>
          ))}
        </ul>
      )}
      {result.truncated ? (
        <p className="fy-auth-native-models-meta">
          目录超过 {result.models.length} 个模型，只显示前{" "}
          {result.models.length} 个。
        </p>
      ) : null}
    </>
  );
}

/**
 * The catalog of the user's own Codex CLI for the account bound to the Codex
 * connection. Keyed by account and connection revision, so a result never
 * outlives an account switch or a Codex file change.
 */
export function CodexNativeModels({
  accountId,
  revision,
  accounts,
  disabled,
}: {
  accountId: string;
  revision: string;
  accounts: ManagedAuthAccountSummary[];
  disabled: boolean;
}) {
  const headingId = useId();
  const query = useCodexNativeModels(accountId, revision);
  const result = query.data;
  const stale = result !== undefined && result.revision !== revision;
  return (
    <section className="fy-auth-native-models" aria-labelledby={headingId}>
      <h4 id={headingId}>本机 Codex 模型目录</h4>
      <p className="fy-auth-native-models-meta">
        通过本机 codex 命令读取，只在本机 Codex
        当前登录的就是此账号时显示；与订阅转发的模型目录分开。
      </p>
      <div className="fy-feature-actions">
        <Button
          disabled={disabled || query.isFetching}
          onClick={() => void query.refetch()}
        >
          {result || query.isError ? "重新获取模型" : "获取模型"}
        </Button>
      </div>
      <div role="status">
        {query.isFetching ? (
          <Spinner label="正在读取本机 Codex 模型目录" />
        ) : query.isError ? (
          <InlineNotice tone="warning">
            {managedAuthCommandErrorCopy(query.error)}
          </InlineNotice>
        ) : stale ? (
          <InlineNotice tone="warning">
            读取后 Codex 连接状态已变化，这次结果不再适用。请重新获取。
          </InlineNotice>
        ) : result ? (
          <CatalogResult result={result} accounts={accounts} />
        ) : null}
      </div>
    </section>
  );
}
