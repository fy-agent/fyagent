import { useState } from "react";
import { FolderOpenIcon } from "@phosphor-icons/react/dist/csr/FolderOpen";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";

import { Dialog } from "../../../shared/ui/Dialog";
import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";

export interface RemapWorkspaceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentPath: string;
  onSave: (newPath: string) => Promise<void>;
  onPickDirectory: () => Promise<string | null>;
  originRef: DialogOriginRef | undefined;
}

export function RemapWorkspaceDialog({
  open,
  onOpenChange,
  currentPath,
  onSave,
  onPickDirectory,
  originRef,
}: RemapWorkspaceDialogProps) {
  const [path, setPath] = useState(currentPath);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handlePickFolder = async () => {
    try {
      const dir = await onPickDirectory();
      if (dir) {
        setPath(dir);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSave = async () => {
    if (!path.trim()) {
      setError("工作区路径不能为空");
      return;
    }
    setError(null);
    setSaving(true);
    try {
      await onSave(path.trim());
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={onOpenChange}
      size="comfortable"
      title="映射目标工作区目录"
      description="跨设备时原机器的绝对路径在当前系统不可用。请指定当前机器的对应目录。"
      actions={
        <>
          <Button disabled={saving} onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={saving || !path.trim()}
            onClick={() => void handleSave()}
          >
            {saving ? "正在绑定…" : "确认绑定"}
          </Button>
        </>
      }
    >
      <div className="fy-remap-dialog-body">
        <div className="fy-import-field-group">
          <label htmlFor="remap-workspace-input" className="fy-field-label">
            目标本机目录绝对路径：
          </label>
          <div className="fy-input-with-button">
            <input
              id="remap-workspace-input"
              type="text"
              className="fy-input-text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/Users/username/workspace/project"
            />
            <Button type="button" onClick={() => void handlePickFolder()}>
              <FolderOpenIcon size={16} />
              <span>选择目录</span>
            </Button>
          </div>
        </div>

        {error && (
          <div className="fy-field-error" role="alert">
            {error}
          </div>
        )}

        <div className="fy-remap-fidelity-note" role="note">
          <InfoIcon size={18} weight="bold" />
          <div className="fy-note-content">
            <strong>正文绝对保真说明：</strong>
            系统仅在目标会话元数据中更新工作区绑定；正文中的所有历史路径均保持原文，不做篡改消除。
          </div>
        </div>
      </div>
    </Dialog>
  );
}
