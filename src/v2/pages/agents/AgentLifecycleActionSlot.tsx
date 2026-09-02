import type { ReactNode } from "react";

import { Button, Spinner } from "../../shared/ui/primitives";

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

export type AgentLifecycleActionSlotView =
  | { kind: "status"; label: string }
  | { kind: "primary"; action: "install" | "update"; onClick: () => void }
  | { kind: "retry"; onClick: () => void }
  | { kind: "select_target"; onClick: () => void }
  | { kind: "empty" };

export function AgentLifecycleActionSlot({
  view,
}: {
  view: AgentLifecycleActionSlotView;
}): ReactNode {
  switch (view.kind) {
    case "status":
      return <StatusSlot label={view.label} />;
    case "primary":
      return (
        <Button onClick={view.onClick}>
          {directoryPrimaryActionLabel(view.action)}
        </Button>
      );
    case "retry":
      return <Button onClick={view.onClick}>重试</Button>;
    case "select_target":
      return <Button onClick={view.onClick}>选择安装目标</Button>;
    case "empty":
      return null;
  }
}
