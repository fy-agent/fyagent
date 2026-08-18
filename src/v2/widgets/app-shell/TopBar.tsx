import type { WindowFramePort } from "../../shared/platform";
import { Brand } from "./Brand";
import { PrimaryNav } from "./PrimaryNav";
import { ToolCluster } from "./ToolCluster";
import { WindowControls } from "./WindowControls";

interface TopBarProps {
  frame: WindowFramePort;
}

export function TopBar({ frame }: TopBarProps) {
  return (
    <header className="fy-top-bar" data-testid="top-bar">
      <div className="fy-top-bar-leading">
        <Brand />
        <span
          className="fy-titlebar-drag-region"
          data-tauri-drag-region
          data-testid="titlebar-drag-region"
          aria-hidden="true"
        />
      </div>

      <PrimaryNav />

      <div className="fy-top-bar-trailing">
        <ToolCluster />
        <WindowControls frame={frame} />
      </div>
    </header>
  );
}
