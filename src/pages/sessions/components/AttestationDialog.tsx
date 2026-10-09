import { useState } from "react";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";
import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";

import { Dialog } from "../../../shared/ui/Dialog";
import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";
import { parseMigrationError } from "../../../shared/features/session-migration";
import type {
  RestoreAttempt,
  RestoreStage,
} from "../../../shared/features/session-migration";

export interface AttestationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  attempt: RestoreAttempt | null;
  onRecord: (
    attemptId: string,
    claimedStage: RestoreStage,
    note?: string,
  ) => Promise<void>;
  originRef: DialogOriginRef | undefined;
}

export function AttestationDialog({
  open,
  onOpenChange,
  attempt,
  onRecord,
  originRef,
}: AttestationDialogProps) {
  const [note, setNote] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!attempt) return null;

  const handleSubmit = async () => {
    setError(null);
    setSubmitting(true);
    try {
      await onRecord(
        attempt.attemptId,
        "nextTurnReplyVerified",
        note.trim() || undefined,
      );
      onOpenChange(false);
    } catch (err) {
      setError(parseMigrationError(err).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={onOpenChange}
      size="comfortable"
      title="记录：我已手动续聊"
      description="记录你在目标软件中完成的下一轮对话状态。"
      actions={
        <>
          <Button disabled={submitting} onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={submitting}
            onClick={() => void handleSubmit()}
          >
            <CheckIcon size={14} weight="bold" />
            <span>{submitting ? "正在记录…" : "确认记录"}</span>
          </Button>
        </>
      }
    >
      <div className="fy-attestation-dialog-body">
        <div className="fy-attestation-warning" role="note">
          <InfoIcon size={20} weight="bold" />
          <div className="fy-attestation-warning-text">
            此操作记录你在目标软件中手动发送新消息并收到回复的说明。
            系统恢复阶段仍以实际回执为准，记录不会提升系统验证状态。
          </div>
        </div>

        <div className="fy-import-field-group">
          <label htmlFor="attestation-note-input" className="fy-field-label">
            备注信息（可选）：
          </label>
          <input
            id="attestation-note-input"
            type="text"
            className="fy-input-text"
            placeholder="如：在 Codex 终端发问并成功收到回复"
            value={note}
            onChange={(e) => setNote(e.target.value)}
          />
        </div>

        {error && (
          <div className="fy-field-error" role="alert">
            {error}
          </div>
        )}
      </div>
    </Dialog>
  );
}
