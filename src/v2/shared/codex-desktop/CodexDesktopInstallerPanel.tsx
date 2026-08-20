import type {
  InstallerPrimaryAction,
  InstallerViewState,
  LocalVersionState,
  RemoteVersionState,
} from "@/shared/codex-desktop";
import { Button, InlineNotice, Spinner } from "../ui/primitives";
import { useCodexDesktopInstaller } from "./useCodexDesktopInstaller";

const stateLabels: Readonly<Record<InstallerViewState, string>> = {
  hidden: "当前平台暂不支持安装 Codex Desktop。",
  checking: "正在检查安装状态和可用版本。",
  unsupported_architecture: "当前系统或处理器架构不受支持。",
  ambiguous: "发现多个 Codex Desktop 安装，暂时无法确定要使用的版本。",
  ready_install: "Codex Desktop 尚未安装，可以开始安装。",
  ready_update: "有可用的新版本。",
  ready_launch: "Codex Desktop 已是最新版本。",
  local_newer: "本机版本比当前可用版本更新。",
  remote_unavailable: "暂时无法读取可安装版本。",
  remote_unavailable_installed: "暂时无法读取最新版本，仍可启动本机版本。",
  job_checking: "正在确认版本信息。",
  job_preflight: "正在执行安装前检查。",
  job_downloading: "正在下载 Codex Desktop。",
  job_installing: "正在安装 Codex Desktop。",
  job_verifying_installation: "正在验证安装结果。",
  succeeded: "Codex Desktop 已安装完成。",
  failed: "Codex Desktop 安装未完成。",
  cancelled: "Codex Desktop 安装已取消。",
};

function localVersionLabel(version: LocalVersionState): string {
  switch (version.kind) {
    case "loading":
      return "读取中";
    case "installed":
      return version.version;
    case "not_installed":
      return "未安装";
    case "error":
      return "无法确认";
  }
}

function remoteVersionLabel(version: RemoteVersionState): string {
  switch (version.kind) {
    case "loading":
      return "读取中";
    case "available":
      return version.version;
    case "refreshing":
      return `${version.version}（刷新中）`;
    case "refetch_error":
      return `${version.version}（刷新失败）`;
    case "initial_network_error":
      return "网络不可用";
    case "platform_unavailable":
      return "当前平台暂无版本";
    case "metadata_error":
      return "版本信息无效";
  }
}

function actionLabel(
  action: InstallerPrimaryAction,
  state: InstallerViewState,
  pending: boolean,
): string | null {
  if (!action) return null;
  if (pending) return "处理中…";
  switch (action) {
    case "install":
      return "安装 Codex Desktop";
    case "update":
      return "更新 Codex Desktop";
    case "launch":
      return "启动 Codex Desktop";
    case "refresh":
      return "刷新版本";
    case "retry":
      return state === "failed" ? "重试安装" : "重试读取";
  }
}

function formatBytes(value: number | null): string | null {
  if (value === null || !Number.isFinite(value) || value < 0) return null;
  const units = ["B", "KB", "MB", "GB", "TB"] as const;
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const digits = amount >= 100 || unit === 0 ? 0 : amount >= 10 ? 1 : 2;
  return `${amount.toFixed(digits)} ${units[unit]}`;
}

export function CodexDesktopInstallerPanel() {
  const installer = useCodexDesktopInstaller();
  const primaryLabel = actionLabel(
    installer.primaryAction,
    installer.state,
    installer.isActing,
  );
  const showDownloadBytes = installer.state === "job_downloading";
  const jobBusy = installer.state.startsWith("job_");
  const currentBytes = showDownloadBytes
    ? formatBytes(installer.progress?.current ?? null)
    : null;
  const totalBytes = showDownloadBytes
    ? formatBytes(installer.progress?.total ?? null)
    : null;
  const speed = showDownloadBytes
    ? formatBytes(installer.progress?.bytesPerSecond ?? null)
    : null;
  const percent = installer.progress?.percent;
  const validPercent =
    percent !== null && percent !== undefined && Number.isFinite(percent)
      ? Math.max(0, Math.min(100, percent))
      : null;
  const downloadComplete =
    showDownloadBytes && validPercent !== null && validPercent >= 100;
  const statusText = installer.authorityUnavailable
    ? "暂时无法读取安装状态。"
    : downloadComplete
      ? "下载已完成，正在校验并准备安装。"
      : stateLabels[installer.state];
  const jobSpinnerLabel = downloadComplete
    ? "正在校验并准备安装 Codex Desktop"
    : "正在处理 Codex Desktop 安装";

  return (
    <section
      className="fy-agent-section fy-codex-installer"
      aria-label="Codex Desktop 安装器"
    >
      <div className="fy-codex-installer-heading">
        <div>
          <h3>Codex Desktop</h3>
          <p>在 FyAgent 中安装、更新或启动桌面应用。</p>
        </div>
        {(installer.state === "checking" || installer.isRefreshing) && (
          <Spinner label="正在读取 Codex Desktop 状态" />
        )}
      </div>

      <div className="fy-codex-installer-body" aria-live="polite">
        <div className="fy-codex-installer-status-row">
          {jobBusy && <Spinner label={jobSpinnerLabel} />}
          <p
            className={
              jobBusy
                ? "fy-codex-installer-status is-working"
                : "fy-codex-installer-status"
            }
          >
            {statusText}
          </p>
        </div>

        <dl className="fy-codex-installer-versions">
          <div>
            <dt>本机版本</dt>
            <dd>{localVersionLabel(installer.localVersion)}</dd>
          </div>
          <div>
            <dt>可用版本</dt>
            <dd>{remoteVersionLabel(installer.remoteVersion)}</dd>
          </div>
        </dl>

        {validPercent !== null && (
          <div className="fy-codex-installer-progress">
            <div
              className="fy-codex-installer-progress-track"
              role="progressbar"
              aria-label={
                showDownloadBytes ? "Codex Desktop 下载进度" : "安装进度"
              }
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={validPercent}
            >
              <span style={{ width: `${validPercent}%` }} />
            </div>
            <span>{Math.round(validPercent)}%</span>
          </div>
        )}

        {showDownloadBytes && (currentBytes || totalBytes) && (
          <p className="fy-codex-installer-download">
            {downloadComplete
              ? `已下载 ${currentBytes ?? "—"} / ${totalBytes ?? "—"}。文件较大，校验可能需要一点时间。`
              : `已下载 ${currentBytes ?? "—"} / ${totalBytes ?? "—"}`}
            {!downloadComplete && speed ? ` · ${speed}/s` : ""}
          </p>
        )}

        {installer.error && (
          <InlineNotice tone="error">
            {installer.error.code === "METADATA_CHANGED"
              ? "版本信息已更新，请刷新后重新确认安装。"
              : "安装未完成，请重试。"}
          </InlineNotice>
        )}
        {installer.operationFailed && !installer.error && (
          <InlineNotice tone="error">
            安装未完成，请重试。若问题持续，请查看日志。
          </InlineNotice>
        )}
        {installer.liveUpdatesUnavailable && (
          <InlineNotice tone="warning">
            暂时无法显示实时进度，请稍后刷新。
          </InlineNotice>
        )}

        <div className="fy-codex-installer-actions">
          {primaryLabel && (
            <Button
              className="fy-control-button-primary"
              disabled={installer.primaryDisabled}
              onClick={() => void installer.runPrimaryAction()}
            >
              {primaryLabel}
            </Button>
          )}
          <Button
            disabled={installer.isActing || installer.state.startsWith("job_")}
            onClick={() => void installer.refresh()}
          >
            刷新状态
          </Button>
          {installer.canCancel && (
            <Button onClick={() => void installer.cancel()}>取消安装</Button>
          )}
          {installer.canOpenLogs && (
            <Button onClick={() => void installer.openLogs()}>
              打开日志目录
            </Button>
          )}
        </div>
      </div>
    </section>
  );
}
