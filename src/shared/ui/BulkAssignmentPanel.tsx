import type { SkillTargetId } from "../features/types";
import { Button } from "./Button";
import type { DialogOriginRef } from "./dialogOrigin";

/** Presentation only: the calling page still serializes writes and rereads authority. */
export function BulkAssignmentPanel<T extends SkillTargetId>({
  targets,
  disabled,
  onToggle,
  dialogOriginRef,
}: {
  targets: ReadonlyArray<{ id: T; label: string }>;
  disabled?: boolean;
  onToggle: (target: T, enabled: boolean) => void;
  dialogOriginRef?: DialogOriginRef;
}) {
  return (
    <section className="fy-feature-bulk-assignments" aria-label="全量分配">
      <h3>全量分配</h3>
      {targets.map((target) => (
        <div
          key={target.id}
          className="fy-feature-assignment fy-feature-bulk-row"
          role="group"
          aria-label={target.label}
        >
          <span className="fy-feature-bulk-label">{target.label}</span>
          <span className="fy-feature-assignment-actions">
            <Button
              disabled={disabled}
              dialogOriginRef={dialogOriginRef}
              onClick={() => onToggle(target.id, true)}
            >
              全开
            </Button>
            <Button
              disabled={disabled}
              dialogOriginRef={dialogOriginRef}
              onClick={() => onToggle(target.id, false)}
            >
              全关
            </Button>
          </span>
        </div>
      ))}
    </section>
  );
}
