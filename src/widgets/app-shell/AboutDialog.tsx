import { useRef } from "react";

import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { useAppVersion } from "../../shared/features/useAppVersion";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import "./about-dialog.css";

const projectUrl = "https://github.com/fy-agent/fyagent";

export default function AboutDialog({
  open,
  onOpenChange,
  originRef,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  originRef: DialogOriginRef;
}) {
  const version = useAppVersion(open);
  const closeRef = useRef<HTMLButtonElement>(null);

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      originRef={originRef}
      initialFocusRef={closeRef}
      title="关于 FyAgent"
      description="在一个地方安装 AI 软件、连接模型，并管理项目所需的配置。"
      actions={
        <Button ref={closeRef} onClick={() => onOpenChange(false)}>
          关闭
        </Button>
      }
    >
      <div className="fy-about-content">
        <div className="fy-about-version" aria-live="polite">
          <span>当前版本</span>
          {version.data ? (
            <strong>{version.data}</strong>
          ) : version.isError ? (
            <>
              <span>版本信息暂不可用</span>
              <Button
                onClick={() => void version.refetch()}
                disabled={version.isFetching}
              >
                {version.isFetching ? "正在读取…" : "重新读取"}
              </Button>
            </>
          ) : (
            <span>正在读取…</span>
          )}
        </div>
        <div className="fy-about-links">
          <ExternalLinkButton url={`${projectUrl}/releases`}>
            查看更新
          </ExternalLinkButton>
          <ExternalLinkButton url={`${projectUrl}/issues`}>
            帮助与反馈
          </ExternalLinkButton>
        </div>
        <details className="fy-about-details">
          <summary>发布与许可</summary>
          <p>更新页面提供各版本的变更说明、安装包和发布信息。</p>
          <div className="fy-about-links">
            <ExternalLinkButton url={projectUrl}>项目主页</ExternalLinkButton>
            <ExternalLinkButton url={`${projectUrl}/blob/main/LICENSING.md`}>
              软件许可
            </ExternalLinkButton>
            <ExternalLinkButton
              url={`${projectUrl}/blob/main/THIRD_PARTY_NOTICES.md`}
            >
              第三方声明
            </ExternalLinkButton>
          </div>
        </details>
      </div>
    </Dialog>
  );
}
