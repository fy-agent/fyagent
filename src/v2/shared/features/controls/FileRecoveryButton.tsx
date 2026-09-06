import { useRef, useState } from "react";

import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { InlineNotice, Spinner } from "../../ui/primitives";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import {
  CONFIG_RECOVERY_TARGET_LABELS,
  type ConfigRecoveryTarget,
} from "../config-recovery";
import { useFeatures } from "../provider";
import { useConfigRecoveries } from "../queries";
import { CopyablePath } from "./CopyablePath";

import "./FileWriteDisclosure.css";

export function FileRecoveryButton({
  targets,
  disabled = false,
  onRestored,
}: {
  targets: readonly ConfigRecoveryTarget[];
  disabled?: boolean;
  onRestored: () => void | Promise<void>;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const applying = useRef(false);
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<ConfigRecoveryTarget | null>(null);
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState<{ error: boolean; text: string } | null>(
    null,
  );
  const query = useConfigRecoveries(targets, open);
  const entry = query.data?.find((item) => item.target === selected);
  const canRestore =
    !disabled &&
    visible &&
    !pending &&
    !query.isError &&
    !query.isFetching &&
    entry?.state === "available" &&
    entry.receiptId !== null;

  const restore = async () => {
    if (!canRestore || !entry?.receiptId || applying.current) return;
    applying.current = true;
    setPending(true);
    setNotice(null);
    try {
      await ports.configRecovery.restore({
        target: entry.target,
        receiptId: entry.receiptId,
      });
      setSelected(null);
      setNotice({
        error: false,
        text: entry.restoresExistingFile
          ? "文件已恢复到上次修改前。请重新打开相关软件，检查配置和登录状态。"
          : "上次创建的文件已删除。请重新打开相关软件，检查配置和登录状态。",
      });
      await onRestored();
    } catch {
      setNotice({
        error: true,
        text: "未能确认文件已恢复。文件或备份可能已变化；请刷新检查，不要直接覆盖现有内容。",
      });
    } finally {
      setSelected(null);
      setPending(false);
      applying.current = false;
      await query.refetch();
    }
  };

  return (
    <>
      <Button
        ref={triggerRef}
        disabled={disabled || targets.length === 0}
        onClick={() => {
          setSelected(null);
          setNotice(null);
          setOpen(true);
        }}
      >
        撤回文件修改
      </Button>
      <Dialog
        open={open}
        originRef={triggerRef}
        initialFocusRef={cancelRef}
        onOpenChange={(next) => {
          if (!pending) setOpen(next);
        }}
        title="撤回文件修改"
        description="每个文件仅保留最近一次修改前的内容。只恢复文件，不删除 FyAgent 中已保存的账号或模型条目；其他程序的后续修改不会被自动覆盖。"
        actions={
          <>
            <Button
              ref={cancelRef}
              disabled={pending}
              onClick={() => setOpen(false)}
            >
              关闭
            </Button>
            <Button
              disabled={pending || query.isFetching}
              onClick={() => {
                setSelected(null);
                void query.refetch();
              }}
            >
              刷新备份
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={!canRestore}
              onClick={() => void restore()}
            >
              {pending
                ? "正在恢复…"
                : entry?.restoresExistingFile === false
                  ? "确认删除新文件"
                  : "确认恢复文件"}
            </Button>
          </>
        }
      >
        <div className="fy-file-write-disclosure">
          {query.isFetching ? <Spinner label="正在检查文件备份" /> : null}
          {query.isError ? (
            <InlineNotice tone="warning">
              无法读取备份状态，请刷新后重试。
            </InlineNotice>
          ) : null}
          {query.data?.map((item) => (
            <div className="fy-file-write-target" key={item.target}>
              <label>
                <input
                  type="radio"
                  name="config-recovery-target"
                  checked={selected === item.target}
                  disabled={pending || item.state !== "available"}
                  onChange={() => setSelected(item.target)}
                />
                {CONFIG_RECOVERY_TARGET_LABELS[item.target]}
              </label>
              <CopyablePath
                label="待恢复的文件路径"
                value={item.writeTarget.path}
              />
              <CopyablePath
                label="备份文件路径"
                value={item.writeTarget.backupPath}
              />
              <p>
                {item.state === "available"
                  ? item.restoresExistingFile
                    ? "确认后将用备份恢复原文件。"
                    : "此前没有此文件，确认后将删除上次新创建的文件。"
                  : item.state === "conflict"
                    ? "文件或备份已有变化，不能直接撤回。请先对比现有文件和备份。"
                    : item.state === "manual_backup"
                      ? "保留了备份，但没有可验证的修改记录。请先对比文件，再决定是否手动恢复。"
                      : item.state === "none"
                        ? "没有可撤回的文件修改。"
                        : "无法确认备份是否有效，请保留现有文件并检查备份。"}
              </p>
            </div>
          ))}
          {notice ? (
            <InlineNotice tone={notice.error ? "warning" : "info"}>
              {notice.text}
            </InlineNotice>
          ) : null}
          <p>
            认证备份可能包含登录凭证，请勿分享。备份失败时不会开始修改原文件。
          </p>
        </div>
      </Dialog>
    </>
  );
}
