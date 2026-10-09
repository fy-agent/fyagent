import { useEffect, useRef } from "react";
import { usePersistentVisibility } from "../../../shared/ui/PersistentSurface";
import { Button } from "../../../shared/ui/Button";
import type { SessionFailureFeedback } from "../failure-feedback";

export function FailureFeedback({
  feedback,
  onEdit,
  onReview,
  reviewing,
}: {
  feedback: SessionFailureFeedback;
  onEdit?: () => void;
  onReview?: () => void;
  reviewing?: boolean;
}) {
  const visible = usePersistentVisibility();
  const alertRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!visible) return;
    alertRef.current?.focus({ preventScroll: true });
    alertRef.current?.scrollIntoView?.({ block: "nearest" });
  }, [feedback.phase, feedback.message, feedback.writeSummary, visible]);
  return (
    <div
      className="fy-field-error fy-session-failure-feedback"
      role="alert"
      tabIndex={-1}
      ref={alertRef}
    >
      <strong>{feedback.phase}</strong>
      {feedback.message && <div>{feedback.message}</div>}
      <div>{feedback.writeSummary}</div>
      <div>{feedback.nextStep}</div>
      {(onEdit || onReview) && (
        <div className="fy-banner-action-row">
          {onEdit && (
            <Button type="button" onClick={onEdit}>
              返回编辑位置
            </Button>
          )}
          {onReview && (
            <Button type="button" disabled={reviewing} onClick={onReview}>
              {reviewing ? "正在核对…" : "核对恢复回执"}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
