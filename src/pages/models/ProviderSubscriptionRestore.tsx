import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import type {
  ProviderAppId,
  ProviderProxyRestorePreview,
} from "../../shared/features/models";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice } from "../../shared/ui/primitives";

const labels: Record<ProviderAppId, string> = {
  claude: "Claude Code",
  codex: "Codex",
  grokbuild: "Grok Build",
};

function samePreview(
  before: ProviderProxyRestorePreview,
  current: ProviderProxyRestorePreview,
) {
  return (
    current.app === before.app &&
    current.enabled &&
    current.canRestore &&
    current.targets.length === before.targets.length &&
    current.targets.every(
      (target, index) =>
        target.path === before.targets[index].path &&
        target.exists === before.targets[index].exists,
    )
  );
}

export function ProviderSubscriptionRestore({
  app,
  active,
  disabled,
  onBeginWrite,
  onEndWrite,
  onUnconfirmed,
  onRecoveryConfirmed,
}: {
  app: ProviderAppId;
  active: boolean;
  disabled: boolean;
  onBeginWrite: () => boolean;
  onEndWrite: () => void;
  onUnconfirmed: () => void;
  onRecoveryConfirmed?: (app: ProviderAppId) => void;
}) {
  const { ports } = useFeatures();
  const queries = useQueryClient();
  const previewKey = featureKeys.providerProxyRestorePreview(app);
  const preview = useQuery({
    queryKey: previewKey,
    queryFn: () => ports.providers.getProxyRestorePreview(app),
    enabled: active,
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
  });
  const [pending, setPending] = useState<ProviderProxyRestorePreview | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [retryReady, setRetryReady] = useState(false);
  const [notice, setNotice] = useState<
    "restored" | "unconfirmed" | "changed" | null
  >(null);
  const lock = useRef(false);
  const mounted = useRef(true);
  const previousDisabled = useRef(disabled);
  const origin = useRef<HTMLButtonElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const refetch = preview.refetch;
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    // A save/source plan elsewhere in this panel may have enabled takeover.
    if (previousDisabled.current && !disabled && active) void refetch();
    previousDisabled.current = disabled;
  }, [active, disabled, refetch]);

  const confirmRestored = async (current?: ProviderProxyRestorePreview) => {
    const [next, summary, overview] = await Promise.all([
      current ?? ports.providers.getProxyRestorePreview(app),
      ports.providers.getSummary(app),
      ports.managedAuth.getOverview(),
    ]);
    if (
      next.app !== app ||
      next.enabled ||
      !summary.live ||
      summary.live.target !== app ||
      summary.live.state === "unreadable"
    )
      throw new Error("Restore readback is unconfirmed");
    queries.setQueryData(previewKey, next);
    queries.setQueryData(featureKeys.providerSummary(app), summary);
    queries.setQueryData(featureKeys.managedAuthOverview, overview);
    onRecoveryConfirmed?.(app);
    if (mounted.current) {
      setRetryReady(false);
      setNotice("restored");
    }
  };

  const recheck = async () => {
    if (!active || disabled || busy || lock.current) return;
    lock.current = true;
    if (!onBeginWrite()) {
      lock.current = false;
      return;
    }
    setBusy(true);
    setRetryReady(false);
    try {
      const current = await ports.providers.getProxyRestorePreview(app);
      if (current.app !== app)
        throw new Error("Restore preview target changed");
      queries.setQueryData(previewKey, current);
      if (current.enabled) {
        // Retain unknown write state while admitting only a new restore preview.
        if (mounted.current) setRetryReady(current.canRestore);
      } else {
        // The previous mutation may have completed before its readback failed.
        await confirmRestored(current);
      }
    } catch {
      if (mounted.current) setNotice("unconfirmed");
      onUnconfirmed();
    } finally {
      lock.current = false;
      onEndWrite();
      if (mounted.current) setBusy(false);
    }
  };

  const restore = async () => {
    if (!pending || pending.app !== app || !active || disabled || lock.current)
      return;
    lock.current = true;
    if (!onBeginWrite()) {
      lock.current = false;
      return;
    }
    const confirmed = pending;
    setPending(null);
    setBusy(true);
    setRetryReady(false);
    setNotice(null);
    let attempted = false;
    try {
      // Re-read before mutation so a changed path set requires new confirmation.
      const current = await ports.providers.getProxyRestorePreview(app);
      if (current.app === app) queries.setQueryData(previewKey, current);
      if (!samePreview(confirmed, current)) {
        if (mounted.current) setNotice("changed");
        return;
      }
      attempted = true;
      await ports.providers.restoreManagedProxy(app);
      await confirmRestored();
    } catch {
      if (mounted.current) setNotice(attempted ? "unconfirmed" : "changed");
      if (attempted) onUnconfirmed();
      // Do not reuse a previous success or automatically retry a mutation.
      await Promise.all([
        queries.invalidateQueries({
          queryKey: previewKey,
          refetchType: "none",
        }),
        queries.invalidateQueries({
          queryKey: featureKeys.providerSummary(app),
          refetchType: "none",
        }),
        queries.invalidateQueries({
          queryKey: featureKeys.managedAuthOverview,
          refetchType: "none",
        }),
      ]);
    } finally {
      lock.current = false;
      onEndWrite();
      if (mounted.current) setBusy(false);
    }
  };

  const current = preview.isError ? undefined : preview.data;
  const blocked = current?.enabled && !current.canRestore;
  const uncertain = notice === "unconfirmed";
  return (
    <>
      {current?.enabled && (!uncertain || retryReady) ? (
        <InlineNotice tone={blocked ? "warning" : "info"}>
          <p>
            {labels[app]} 已启用 FyAgent
            代理配置。退出时将使用接管前保存的配置恢复。
          </p>
          {blocked ? (
            <p>
              暂时无法确认这些配置可安全恢复。请先退出相关软件，保留后续修改后重新检查。
            </p>
          ) : null}
          <Button
            ref={origin}
            disabled={
              !active ||
              disabled ||
              busy ||
              !current.canRestore ||
              preview.isFetching
            }
            onClick={() => setPending(current)}
          >
            退出 {labels[app]} 代理并恢复配置
          </Button>
        </InlineNotice>
      ) : null}
      {preview.isError || blocked || notice ? (
        <InlineNotice tone={notice === "restored" ? "info" : "warning"}>
          <p>
            {notice === "restored"
              ? `已退出 ${labels[app]} 代理并回读恢复后的配置。请重新打开软件或新建会话。`
              : uncertain
                ? "未能确认配置已完整恢复，已暂停此目标的后续修改。请先退出相关软件并保留外部改动，再重新检查恢复状态。"
                : notice === "changed"
                  ? "恢复条件已变化或暂时无法读取，请重新检查后再次预览。"
                  : "暂时无法确认代理恢复状态。请保留当前配置并重新检查。"}
          </p>
          {(uncertain || blocked) &&
            current?.targets.map((target) => (
              <CopyablePath
                key={target.path}
                label="配置位置"
                value={target.path}
              />
            ))}
          {notice !== "restored" ? (
            <Button
              disabled={!active || disabled || busy || preview.isFetching}
              onClick={() => void recheck()}
            >
              重新检查恢复状态
            </Button>
          ) : null}
        </InlineNotice>
      ) : null}
      <Dialog
        open={active && pending !== null}
        onOpenChange={(open) => {
          if (!open && !lock.current) setPending(null);
        }}
        originRef={origin}
        initialFocusRef={cancel}
        title={`退出 ${labels[app]} 代理`}
        description="使用接管前保存的配置恢复。其他 Agent 的代理设置保持原样。"
        actions={
          <>
            <Button
              ref={cancel}
              disabled={busy}
              onClick={() => setPending(null)}
            >
              取消
            </Button>
            <Button
              disabled={!active || disabled || busy}
              onClick={() => void restore()}
            >
              确认退出并恢复
            </Button>
          </>
        }
      >
        <p>将恢复以下配置文件：</p>
        {pending?.targets.map((target) => (
          <div key={target.path}>
            <CopyablePath label="恢复目标" value={target.path} />
            <p className="fy-models-muted">
              {target.exists ? "当前文件存在" : "当前文件不存在"}
            </p>
          </div>
        ))}
        <p>完成后请重新打开软件或新建会话，让软件读取恢复后的配置。</p>
      </Dialog>
    </>
  );
}
