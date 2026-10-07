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
  const startLabel =
    checked?.execution === "vendor_wizard"
      ? "下载并打开安装窗口"
      : checked?.runtime === "node_npm" || checked?.runtime === "existing_cli"
        ? updating
          ? "开始命令行更新"
          : "开始命令行安装"
        : updating
          ? "开始更新"
          : "开始安装";
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
            <Button onClick={lifecycle.dismissPreflight}>暂不开始</Button>
            <Button
              className="fy-control-button-primary"
              onClick={() => void lifecycle.confirm()}
            >
              {startLabel}
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
            <strong>空间预检：</strong>
            {checked.requiredBytes !== null
              ? `需预留 ${(checked.requiredBytes / 1024 ** 3).toFixed(1)} GiB；`
              : "安装预留预算尚未核定；"}
            下载临时目录和目标目录所在磁盘最少可用{" "}
            {(checked.availableBytes / 1024 ** 3).toFixed(1)} GiB。
          </p>
          <p>
            {checked.spaceBudgetBasis === "source_size"
              ? "按此安装包元数据大小的 3 倍预留下载、临时文件和安装空间；这是保守预算，不是厂商精确安装需求。"
              : checked.spaceBudgetBasis === "package_reserve"
                ? "按已核实 npm 包及依赖大小的 3 倍预留 FyAgent 保守预算；这是预留空间，不是厂商保证的完整峰值。"
                : checked.spaceBudgetBasis === "download_limit"
                  ? "未取得此安装包的准确大小，按下载器的 2 GiB 上限预留 3 倍空间；这是保守预算，不是厂商精确安装需求。"
                  : "CLI 安装和依赖大小尚未核定；可用空间数值不能证明容量足够，安装工具仍可能报告空间不足。"}
            确认时会重新检查可用空间。
          </p>
          <p>
            尚未开始本次操作。关闭此说明不会启动任务；开始后关闭页面不会取消后台任务，请使用进度区可用的取消按钮。
          </p>
          {checked.downloadUrl ? (
            <>
              <p>
                <strong>下载来源域名：</strong>
                {new URL(checked.downloadUrl).hostname}
                。这是安装器返回的来源信息；开始后仍会检查安装包。
              </p>
              <details>
                <summary>查看本次安装包来源</summary>
                <p className="fy-install-source-url">{checked.downloadUrl}</p>
                <p>
                  这是本次所选系统和架构的下载入口；安装时仍会检查下载内容。
                </p>
              </details>
            </>
          ) : null}
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
