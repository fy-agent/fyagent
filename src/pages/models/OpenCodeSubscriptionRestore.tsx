import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { FileWriteDisclosure } from "../../shared/features/controls/FileWriteDisclosure";
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
        onUnconfirmed();
      else onRestored?.();
    } catch {
      onUnconfirmed();
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
      </Dialog>
    </>
  );
}
