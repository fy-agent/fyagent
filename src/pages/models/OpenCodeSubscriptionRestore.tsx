import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { FileWriteDisclosure } from "../../shared/features/controls/FileWriteDisclosure";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import {
  isManagedOpenCodeProvider,
  type ModelWriteTarget,
} from "../../shared/features/models";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice } from "../../shared/ui/primitives";

export function OpenCodeSubscriptionRestore({
  disabled,
  writeTargets,
  onBeginWrite,
  onEndWrite,
  onUnconfirmed,
  onRestored,
}: {
  disabled: boolean;
  writeTargets: readonly ModelWriteTarget[];
  onBeginWrite: () => boolean;
  onEndWrite: () => void;
  onUnconfirmed: () => void;
  onRestored?: () => void;
}) {
  const { ports } = useFeatures();
  const queries = useQueryClient();
  const [open, setOpen] = useState(false);
  const [unconfirmed, setUnconfirmed] = useState(false);
  const mounted = useRef(true);
  const origin = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const restore = async () => {
    if (disabled || !onBeginWrite()) return;
    setOpen(false);
    setUnconfirmed(false);
    const markUnconfirmed = () => {
      if (mounted.current) setUnconfirmed(true);
      onUnconfirmed();
    };
    try {
      await ports.opencodeModels.restoreManagedProxy();
      await Promise.all([
        queries.invalidateQueries({
          queryKey: featureKeys.openCodeModelSnapshot,
          refetchType: "none",
        }),
        queries.invalidateQueries({
          queryKey: featureKeys.managedAuthOverview,
          refetchType: "none",
        }),
      ]);
      const [snapshot] = await Promise.all([
        queries.fetchQuery({
          queryKey: featureKeys.openCodeModelSnapshot,
          queryFn: ports.opencodeModels.getSnapshot,
        }),
        queries.fetchQuery({
          queryKey: featureKeys.managedAuthOverview,
          queryFn: ports.managedAuth.getOverview,
        }),
      ]);
      if (snapshot.providers.some((item) => isManagedOpenCodeProvider(item.id)))
        markUnconfirmed();
      else onRestored?.();
    } catch {
      markUnconfirmed();
    } finally {
      // Parent ownership survives unmount/target switches; only local state
      // is guarded. A missing readback must still block further target writes.
      if (mounted.current) setOpen(false);
      onEndWrite();
    }
  };

  return (
    <>
      <InlineNotice tone="info">
        OpenCode 已配置订阅来源。使用 API Key 前，请先恢复之前的模型配置。
        <Button ref={origin} disabled={disabled} onClick={() => setOpen(true)}>
          恢复之前的模型配置
        </Button>
      </InlineNotice>
      {unconfirmed && (
        <InlineNotice tone="warning">
          <p>
            未能确认原配置已恢复。当前文件或备份可能已有变化；请先退出
            OpenCode，
            对比现有配置和备份，保留后续改动，再重新打开模型页面检查。
          </p>
          {writeTargets.map((target) => (
            <div key={target.path}>
              <CopyablePath label="现有配置" value={target.path} />
              <CopyablePath label="备份位置" value={target.backupPath} />
            </div>
          ))}
        </InlineNotice>
      )}
      <Dialog
        open={open}
        onOpenChange={setOpen}
        originRef={origin}
        title="恢复 OpenCode 模型配置"
        description="停止为 OpenCode 使用此订阅，并恢复启用订阅前的模型配置。其他 Agent 的订阅设置保持原样。"
        actions={
          <>
            <Button onClick={() => setOpen(false)}>取消</Button>
            <Button disabled={disabled} onClick={() => void restore()}>
              确认恢复
            </Button>
          </>
        }
      >
        <FileWriteDisclosure targets={writeTargets} />
        <p>
          恢复完成后，请重新打开 OpenCode 或新建会话，让软件读取恢复后的配置。
        </p>
      </Dialog>
    </>
  );
}
