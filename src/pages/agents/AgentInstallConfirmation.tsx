import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice } from "../../shared/ui/primitives";
import type { AgentLifecycleActionView } from "./useAgentLifecycleAction";

export function AgentInstallConfirmation({
  name,
  lifecycle,
}: {
  name: string;
  lifecycle: AgentLifecycleActionView;
}) {
  const checked = lifecycle.preflight;
  const updating = checked?.request.action === "update";
  return (
    <Dialog
      open={checked !== null}
      size="comfortable"
      originRef={undefined}
      title={`${updating ? "更新" : "安装"} ${name}`}
      onOpenChange={(open) => {
        if (!open) lifecycle.dismissPreflight();
      }}
      actions={
        checked ? (
          <>
            <Button onClick={lifecycle.dismissPreflight}>取消</Button>
            <Button
              className="fy-control-button-primary"
              onClick={() => void lifecycle.confirm()}
            >
              {updating ? "确认更新" : "确认安装"}
            </Button>
          </>
        ) : undefined
      }
    >
      {checked ? (
        <div className="fy-agent-install-readiness">
          <p>
            <strong>软件：</strong>
            {name} · {checked.versionOrChannel}
          </p>
          <p>
            <strong>位置：</strong>
            {checked.targetLabel}
          </p>
          <p>
            <strong>本次变化：</strong>
            {checked.execution === "vendor_wizard"
              ? "下载并打开官方安装窗口，安装位置和最终变更由该窗口确认。"
              : updating
                ? "更新所选位置的软件，保留现有配置。"
                : "在所选位置安装软件，保留现有配置。"}
          </p>
          <p>
            <strong>运行环境：</strong>
            {checked.platform === "macos" ? "macOS" : "Windows"} ·{" "}
            {checked.architecture === "aarch64" ? "ARM64" : "x64"}；
            {checked.runtime === "node_npm"
              ? "Node.js 与 npm 已检查"
              : checked.runtime === "existing_cli"
                ? "使用现有 CLI 更新方式"
                : "使用系统安装工具"}
          </p>
          <p>
            <strong>可用空间：</strong>
            {(checked.availableBytes / 1024 ** 3).toFixed(1)}{" "}
            GB。来源未提供安装大小，实际空间需求由安装过程确认。
          </p>
          {checked.execution === "system_authorization" ? (
            <InlineNotice tone="warning">
              安装时需要管理员授权。拒绝授权会停止本次安装。
            </InlineNotice>
          ) : null}
          {checked.execution === "vendor_wizard" ? (
            <InlineNotice tone="info">
              请在官方窗口完成安装，需要时由 Windows
              请求授权。打开窗口后，FyAgent 不会中止它；完成后请刷新安装状态。
            </InlineNotice>
          ) : null}
        </div>
      ) : null}
    </Dialog>
  );
}
