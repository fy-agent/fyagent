import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { useRecoverableErrorDialog } from "../ui/useRecoverableErrorDialog";
import { codexInstallerErrorPresentation } from "./installerErrorCopy";
import type { CodexDesktopInstallerViewModel } from "./useCodexDesktopInstaller";

export function CodexInstallConfirmation({
  installer,
}: {
  installer: CodexDesktopInstallerViewModel;
}) {
  const checked = installer.preflight;
  const error = installer.error;
  const presentation = error ? codexInstallerErrorPresentation(error) : null;
  const errorDialog = useRecoverableErrorDialog(
    error && presentation?.recoverable
      ? {
          key: `${error.code}:${error.details.platformErrorCode ?? ""}:${error.details.redactedMessage ?? ""}`,
          title: presentation.title,
          message: presentation.message,
        }
      : null,
    {
      rearm: installer.isActing,
      suppress: checked !== null || installer.isActing,
    },
  );
  return (
    <>
      <Dialog
        open={checked !== null}
        originRef={undefined}
        size="comfortable"
        title={`${checked?.updating ? "更新" : "安装"} Codex Desktop`}
        onOpenChange={(open) => {
          if (!open) void installer.dismissPreflight();
        }}
        actions={
          checked ? (
            <>
              <Button onClick={() => void installer.dismissPreflight()}>
                暂不开始
              </Button>
              {checked.platform === "windows" && (
                <Button
                  disabled={installer.isActing}
                  onClick={() => void installer.selectStagingDirectory()}
                >
                  {checked.customStaging
                    ? "更改下载临时目录"
                    : "选择下载临时目录"}
                </Button>
              )}
              <Button
                className="fy-control-button-primary"
                onClick={() => void installer.confirmInstall()}
              >
                {checked.updating ? "开始更新" : "开始安装"}
              </Button>
            </>
          ) : undefined
        }
      >
        {checked ? (
          <div className="fy-agent-install-readiness">
            <p>
              <strong>软件：</strong>Codex Desktop · {checked.displayVersion} ·
              Stable
            </p>
            <p>
              <strong>位置：</strong>
              {checked.targetLabel}
              {checked.platform === "windows" && (
                <>
                  {" "}
                  Windows 管理 MSIX 的安装位置，FyAgent 不能选择或更改该位置。
                </>
              )}
              {checked.platform === "macos" && (
                <>
                  {" "}
                  {checked.updating
                    ? "更新保留现有位置，暂不支持更改目录。"
                    : "当前安装方式暂不支持选择其他目录。"}
                </>
              )}
            </p>
            <p>
              <strong>下载临时目录：</strong>
              {checked.stagingLabel}
              {checked.platform === "windows"
                ? "。仅下载暂存使用；不会更改 Windows 管理的 MSIX 安装位置。"
                : "。当前平台使用 FyAgent 的安全临时目录。"}
            </p>
            <p>
              <strong>本次变化：</strong>
              {checked.platform === "windows"
                ? checked.updating
                  ? "由 Windows 更新当前用户的 Codex Desktop，保留现有配置。"
                  : "由 Windows 为当前用户安装 Codex Desktop，保留现有配置。"
                : checked.updating
                  ? "更新现有位置的 Codex Desktop，保留现有配置。"
                  : "为当前用户安装 Codex Desktop，保留现有配置。"}
            </p>
            <p>
              <strong>运行环境：</strong>
              {checked.platform === "macos" ? "macOS" : "Windows"} ·{" "}
              {checked.architecture === "aarch64" ? "ARM64" : "x64"}
            </p>
            <p>
              <strong>可用空间：</strong>
              {(checked.availableBytes / 1024 ** 3).toFixed(1)} GB。
              {checked.downloadSizeHint
                ? `下载约 ${(checked.downloadSizeHint / 1024 ** 2).toFixed(0)} MB；已检查安装所需的预留空间。`
                : "来源未提供安装大小，实际空间需求由安装过程确认。"}
            </p>
            <p>
              尚未开始本次操作。关闭此说明不会启动任务；开始后关闭页面不会取消后台任务，请使用进度区可用的取消按钮。
            </p>
            <p>
              <strong>下载来源域名：</strong>
              {new URL(checked.downloadUrl).hostname}
              。这是安装器返回的来源信息；开始后仍会检查安装包。
            </p>
            <details>
              <summary>查看本次安装包来源</summary>
              <p className="fy-install-source-url">{checked.downloadUrl}</p>
              <p>这是本次所选系统和架构的下载入口；安装时仍会检查下载内容。</p>
            </details>
          </div>
        ) : null}
      </Dialog>
      <Dialog
        open={errorDialog.open}
        originRef={undefined}
        title={errorDialog.content?.title ?? "安装未完成"}
        onOpenChange={(open) => {
          if (!open) errorDialog.dismiss();
        }}
        actions={
          <>
            <Button onClick={errorDialog.dismiss}>关闭</Button>
            {presentation?.offerStagingPicker && (
              <Button
                disabled={installer.primaryDisabled}
                onClick={() => void installer.selectStagingDirectory()}
              >
                重新选择下载临时目录
              </Button>
            )}
            <Button
              className="fy-control-button-primary"
              disabled={installer.primaryDisabled}
              onClick={() => void installer.runPrimaryAction()}
            >
              重新检查
            </Button>
          </>
        }
      >
        {errorDialog.content && <p>{errorDialog.content.message}</p>}
      </Dialog>
    </>
  );
}
