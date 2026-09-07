import { useRef, type ReactNode, type RefObject } from "react";

import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { Spinner } from "../../shared/ui/primitives";

function directoryPrimaryActionLabel(action: "install" | "update"): string {
  return action === "install" ? "一键安装" : "一键更新";
}

function StatusSlot({ label }: { label: string }) {
  return (
    <span className="fy-agent-directory-lifecycle-status" role="status">
      <Spinner label={label} />
      {label}
    </span>
  );
}

function SelectTargetButton({
  originRef,
  returnRef,
  onClick,
}: {
  originRef?: DialogOriginRef;
  returnRef: RefObject<HTMLElement>;
  onClick: () => void;
}) {
  return (
    <Button
      dialogOriginRef={originRef}
      dialogReturnRef={returnRef}
      onClick={onClick}
    >
      选择安装目标
    </Button>
  );
}

export type AgentLifecycleActionSlotView =
  | { kind: "status"; label: string }
  | { kind: "primary"; action: "install" | "update"; onClick: () => void }
  | { kind: "retry"; onClick: () => void }
  | { kind: "select_target"; onClick: () => void; originRef?: DialogOriginRef }
  | { kind: "empty" };

export function AgentLifecycleActionSlot({
  view,
}: {
  view: AgentLifecycleActionSlotView;
}): ReactNode {
  const hostRef = useRef<HTMLDivElement>(null);
  if (view.kind === "empty") return null;
  return (
    <div ref={hostRef} className="fy-agent-directory-lifecycle-host">
      {view.kind === "status" ? (
        <StatusSlot label={view.label} />
      ) : view.kind === "primary" ? (
        <Button onClick={view.onClick}>
          {directoryPrimaryActionLabel(view.action)}
        </Button>
      ) : view.kind === "retry" ? (
        <Button onClick={view.onClick}>重试</Button>
      ) : (
        <SelectTargetButton
          originRef={view.originRef}
          returnRef={hostRef}
          onClick={view.onClick}
        />
      )}
    </div>
  );
}
