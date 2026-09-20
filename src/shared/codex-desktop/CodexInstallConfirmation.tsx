import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import type { CodexDesktopInstallerViewModel } from "./useCodexDesktopInstaller";

export function CodexInstallConfirmation({
  installer,
}: {
  installer: CodexDesktopInstallerViewModel;
}) {
  const checked = installer.preflight;
  return (
    <Dialog
      open={checked !== null}
      originRef={undefined}
      size="comfortable"
      title={`${checked?.updating ? "更新" : "安装"} Codex Desktop`}
      onOpenChange={(open) => {
        if (!open) installer.dismissPreflight();
      }}
      actions={
        checked ? (
          <>
            <Button onClick={installer.dismissPreflight}>取消</Button>
            <Button
              className="fy-control-button-primary"
              onClick={() => void installer.confirmInstall()}
            >
              {checked.updating ? "确认更新" : "确认安装"}
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
          </p>
          <p>
            <strong>本次变化：</strong>
            {checked.updating
              ? "更新所选位置的 Codex Desktop，保留现有配置。"
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
        </div>
      ) : null}
    </Dialog>
  );
}
