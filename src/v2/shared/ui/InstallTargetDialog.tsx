import { useState } from "react";

import { SKILL_TARGETS, type SkillTargetId } from "../features/types";
import { AssignmentPanel } from "./AssignmentPanel";
import { Button, Dialog } from "./primitives";

export function skillTargetLabel(id: SkillTargetId): string {
  return SKILL_TARGETS.find((app) => app.id === id)?.label ?? "Claude Code";
}

export function InstallTargetDialog({
  title,
  description = "选择要安装到的应用。",
  busy,
  defaultTarget,
  confirmVerb = "安装到",
  onCancel,
  onConfirm,
}: {
  title: string;
  description?: string;
  busy: boolean;
  defaultTarget: SkillTargetId;
  confirmVerb?: string;
  onCancel: () => void;
  onConfirm: (target: SkillTargetId) => void;
}) {
  const [chosenTarget, setChosenTarget] = useState(defaultTarget);
  return (
    <Dialog
      open
      title={title}
      description={description}
      onOpenChange={(open) => {
        if (!open && !busy) onCancel();
      }}
      actions={
        <>
          <Button disabled={busy} onClick={onCancel}>
            取消
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={busy}
            onClick={() => onConfirm(chosenTarget)}
          >
            {confirmVerb} {skillTargetLabel(chosenTarget)}
          </Button>
        </>
      }
    >
      <AssignmentPanel
        mode="radio"
        ariaLabel="安装目标"
        disabled={busy}
        onChange={setChosenTarget}
        targets={SKILL_TARGETS}
        value={chosenTarget}
      />
    </Dialog>
  );
}
