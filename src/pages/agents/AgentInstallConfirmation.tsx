import {
  recommendedInstallLabels,
  WINDOWS_ENV_HINT,
  WINDOWS_LOGIN_SPIN_HINT,
} from "../../domain/first-install-path";
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
  const recommendedVersion = checked
    ? recommendedInstallLabels([checked.versionOrChannel])[0]
    : undefined;
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
            {name}
            {recommendedVersion
              ? ` · ${recommendedVersion}`
              : ` · ${checked.versionOrChannel}（预发版，不在推荐安装名单）`}
          </p>
          {recommendedVersion ? null : (
            <InlineNotice tone="warning">
              该版本带有预发标记，如需稳定版请返回重新检查来源。
            </InlineNotice>
          )}
          <p>
            <strong>位置：</strong>
            {checked.targetLabel}
          </p>
          <p>
            <strong>操作：</strong>
            {checked.execution === "vendor_wizard"
              ? "下载并打开官方安装程序，安装位置由官方窗口确认。"
              : updating
                ? "更新所选位置，保留现有配置。"
                : "安装到所选位置，保留现有配置。"}
          </p>
          <p>
            <strong>空间：</strong>
            {checked.requiredBytes !== null
              ? `预计需预留 ${(checked.requiredBytes / 1024 ** 3).toFixed(1)} GiB`
              : "安装体积未知"}
            {" · "}当前可用 {(checked.availableBytes / 1024 ** 3).toFixed(1)}{" "}
            GiB
          </p>
          {checked.spaceBudgetBasis === "cli_unknown" ? (
            <InlineNotice tone="warning">
              CLI 依赖体积无法提前核定，磁盘剩余空间仅供参考。
            </InlineNotice>
          ) : null}
          <details>
            <summary>安装详情与来源</summary>
            <p>
              <strong>运行环境：</strong>
              {checked.platform === "macos" ? "macOS" : "Windows"} ·{" "}
              {checked.architecture === "aarch64" ? "ARM64" : "x64"} ·{" "}
              {checked.runtime === "node_npm"
                ? "Node.js / npm"
                : checked.runtime === "existing_cli"
                  ? "现有 CLI 更新"
                  : "系统安装工具"}
            </p>
            {checked.spaceBudgetBasis !== "cli_unknown" ? (
              <p>
                {checked.spaceBudgetBasis === "source_size"
                  ? "按安装包大小的 3 倍预留临时与解压空间，确认时会重新检查。"
                  : checked.spaceBudgetBasis === "package_reserve"
                    ? "按 npm 包及依赖大小的 3 倍预留安装空间，确认时会重新检查。"
                    : "未获取到精确包体大小，暂按 2 GiB 上限的 3 倍预留空间。"}
              </p>
            ) : null}
            {checked.downloadUrl ? (
              <>
                <p>
                  <strong>下载域名：</strong>
                  {new URL(checked.downloadUrl).hostname}
                </p>
                <p className="fy-install-source-url">{checked.downloadUrl}</p>
              </>
            ) : null}
          </details>
          {checked.execution === "system_authorization" ? (
            <InlineNotice tone="warning">
              安装需要管理员授权，拒绝授权将取消安装。
            </InlineNotice>
          ) : null}
          {checked.execution === "vendor_wizard" ? (
            <InlineNotice tone="info">
              请在官方安装窗口完成安装，完成后回到此页刷新状态。
            </InlineNotice>
          ) : null}
          {checked.platform === "windows" ? (
            <InlineNotice tone="info">
              {WINDOWS_LOGIN_SPIN_HINT}
              {WINDOWS_ENV_HINT}
            </InlineNotice>
          ) : null}
        </div>
      ) : null}
    </Dialog>
  );
}
