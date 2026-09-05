import { useState } from "react";

import { SKILL_TARGETS, type SkillTargetId } from "../directory";
import { AssignmentPanel } from "../../ui/AssignmentPanel";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import type { DialogOriginRef } from "../../ui/dialogOrigin";

export function skillTargetLabel(id: SkillTargetId): string {
  return SKILL_TARGETS.find((app) => app.id === id)?.label ?? "Claude Code";
}

export function InstallPathPreview({
  path,
  note,
}: {
  path: string;
  note?: string;
}) {
  return (
    <>
      <p className="fy-feature-description">将写入以下路径：</p>
      <div className="fy-feature-path">
        <code className="fy-feature-path-value" title={path}>
          {path}
        </code>
      </div>
      {note ? <p className="fy-feature-description">{note}</p> : null}
    </>
  );
}

export function InstallTargetDialog({
  originRef,
  title,
  description = "选择要安装到的应用。",
  busy,
  defaultTarget,
  confirmVerb = "确认安装",
  pathForTarget,
  pathNote,
  onCancel,
  onConfirm,
}: {
  title: string;
  originRef?: DialogOriginRef;
  description?: string;
  busy: boolean;
  defaultTarget: SkillTargetId;
  confirmVerb?: string;
  pathForTarget: (target: SkillTargetId) => string;
  pathNote?: string;
  onCancel: () => void;
  onConfirm: (target: SkillTargetId) => void;
}) {
  const [chosenTarget, setChosenTarget] = useState(defaultTarget);
  const [step, setStep] = useState<"pick" | "path">("pick");
  const picking = step === "pick";

  return (
    <Dialog
      open
      originRef={originRef}
      title={title}
      description={
        picking
          ? description
          : `将安装到 ${skillTargetLabel(chosenTarget)}。确认路径后再写入。`
      }
      onOpenChange={(open) => {
        if (!open && !busy) onCancel();
      }}
      actions={
        picking ? (
          <>
            <Button disabled={busy} onClick={onCancel}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={busy}
              onClick={() => setStep("path")}
            >
              下一步
            </Button>
          </>
        ) : (
          <>
            <Button disabled={busy} onClick={() => setStep("pick")}>
              返回
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={busy}
              onClick={() => onConfirm(chosenTarget)}
            >
              {busy ? "安装中…" : confirmVerb}
            </Button>
          </>
        )
      }
    >
      {picking ? (
        <AssignmentPanel
          mode="radio"
          ariaLabel="安装目标"
          disabled={busy}
          onChange={setChosenTarget}
          targets={SKILL_TARGETS}
          value={chosenTarget}
        />
      ) : (
        <InstallPathPreview
          path={pathForTarget(chosenTarget)}
          note={pathNote}
        />
      )}
    </Dialog>
  );
}
