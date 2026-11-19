import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import { Button } from "../../shared/ui/Button";
import "./codex-reload-banner.css";

function isCodexMutationKey(queryKey: readonly unknown[]): boolean {
  if (queryKey[0] !== "fyagent") return false;
  if (queryKey[1] === "providers" && queryKey[2] === "codex") return true;
  if (
    (queryKey[1] === "provider-mode" ||
      queryKey[1] === "provider-stack" ||
      queryKey[1] === "provider-list") &&
    queryKey[2] === "codex"
  ) {
    return true;
  }
  if (queryKey[1] === "change-plans" && queryKey[2] === "job") return true;
  return false;
}

export default function CodexReloadBanner() {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const [dismissed, setDismissed] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  const statusQuery = useQuery({
    queryKey: featureKeys.codexReloadStatus,
    queryFn: async () => {
      try {
        return await ports.providers.getCodexReloadStatus();
      } catch {
        return null;
      }
    },
    retry: false,
    refetchOnWindowFocus: true,
  });

  useEffect(() => {
    const cache = queryClient.getQueryCache();
    return cache.subscribe((event) => {
      const codexCacheMutated =
        (event.type === "added" && event.query.state.status === "success") ||
        (event.type === "updated" &&
          (event.action.type === "success" ||
            event.action.type === "invalidate" ||
            event.action.type === "setState"));
      if (codexCacheMutated && isCodexMutationKey(event.query.queryKey)) {
        setDismissed(false);
        setFeedback(null);
        void queryClient.invalidateQueries({
          queryKey: featureKeys.codexReloadStatus,
        });
      }
    });
  }, [queryClient]);

  const status = statusQuery.data;
  const hasStale = Boolean(status && (status.daemon || status.others));
  if ((!hasStale && !feedback) || dismissed) return null;

  const handleRestartDaemon = async () => {
    if (restarting) return;
    setRestarting(true);
    setFeedback(null);
    try {
      const outcome = await ports.providers.restartCodexDaemon();
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: featureKeys.codexReloadStatus,
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.providerStack("codex"),
        }),
      ]);
      setFeedback(
        outcome === "restarted"
          ? "已重启 Codex 命令行后台服务，新模型已生效。"
          : "Codex 命令行后台服务当前未运行，无需重启。",
      );
    } catch {
      setFeedback("重启 Codex 命令行服务失败，请稍后在终端手动重试。");
    } finally {
      setRestarting(false);
    }
  };

  const message = feedback
    ? feedback
    : status?.daemon && status?.others
      ? "检测到运行中的 Codex 客户端仍在使用旧模型配置：可一键重启命令行服务；Codex 桌面端或插件请完全退出后重开（macOS 按 ⌘Q，Windows 从托盘退出）。"
      : status?.daemon
        ? "检测到 Codex 命令行后台服务仍在使用旧模型配置，重启后立即生效。"
        : "检测到运行中的 Codex 桌面端或插件仍在使用旧模型配置，请完全退出后重新打开（macOS 按 ⌘Q，Windows 从托盘退出）。";

  return (
    <div
      className="fy-codex-reload-banner"
      role="region"
      aria-label="Codex 配置生效提示"
      data-testid="codex-reload-banner"
    >
      <p className="fy-codex-reload-banner-copy">{message}</p>
      <div className="fy-codex-reload-banner-actions">
        {status?.daemon ? (
          <Button
            className="fy-control-button-primary"
            disabled={restarting}
            onClick={() => void handleRestartDaemon()}
          >
            {restarting ? "重启中…" : "一键重启命令行服务"}
          </Button>
        ) : null}
        <Button
          onClick={() => {
            setDismissed(true);
            setFeedback(null);
          }}
        >
          知道了
        </Button>
      </div>
    </div>
  );
}
