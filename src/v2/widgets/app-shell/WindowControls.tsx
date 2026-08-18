import { MinusIcon } from "@phosphor-icons/react/dist/csr/Minus";
import { SquareIcon } from "@phosphor-icons/react/dist/csr/Square";
import { XIcon } from "@phosphor-icons/react/dist/csr/X";

import type { WindowFramePort } from "../../shared/platform";

interface WindowControlsProps {
  frame: WindowFramePort;
}

function invokeWindowAction(action: () => Promise<void>): void {
  void action().catch((error: unknown) => {
    console.error("FyAgent V2 window action failed", error);
  });
}

export function WindowControls({ frame }: WindowControlsProps) {
  return (
    <div
      className="fy-window-controls"
      role="group"
      aria-label="窗口控制"
      data-testid="window-controls"
    >
      <button
        className="fy-window-control"
        type="button"
        aria-label="最小化"
        data-testid="window-minimize"
        onClick={() => invokeWindowAction(() => frame.minimize())}
      >
        <MinusIcon size={16} weight="regular" aria-hidden />
      </button>
      <button
        className="fy-window-control"
        type="button"
        aria-label="最大化/还原"
        data-testid="window-maximize"
        onClick={() => invokeWindowAction(() => frame.toggleMaximize())}
      >
        <SquareIcon size={13} weight="regular" aria-hidden />
      </button>
      <button
        className="fy-window-control fy-window-control-close"
        type="button"
        aria-label="关闭"
        data-testid="window-close"
        onClick={() => invokeWindowAction(() => frame.close())}
      >
        <XIcon size={16} weight="regular" aria-hidden />
      </button>
    </div>
  );
}
