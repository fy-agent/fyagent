import { classNames } from "../../shared/design-system/classNames";
import { shouldShowMacOverlayDragStrip } from "../../shared/platform";
import { Brand } from "./Brand";
import { PrimaryNav } from "./PrimaryNav";
import { ToolCluster } from "./ToolCluster";

export function TopBar() {
  const showMacOverlayDragStrip = shouldShowMacOverlayDragStrip();

  return (
    <header
      className={classNames(
        "fy-top-bar",
        showMacOverlayDragStrip && "fy-top-bar-macos-overlay",
      )}
      data-testid="top-bar"
    >
      {showMacOverlayDragStrip ? (
        <div
          className="fy-titlebar-drag-strip"
          data-testid="titlebar-drag-region"
        >
          <div className="fy-titlebar-traffic-light-space" aria-hidden="true" />
          <div className="fy-titlebar-drag-surface" data-tauri-drag-region />
        </div>
      ) : null}
      <div className="fy-top-bar-chrome">
        <div className="fy-top-bar-leading">
          <Brand />
        </div>

        <PrimaryNav />

        <div className="fy-top-bar-trailing">
          <ToolCluster />
        </div>
      </div>
    </header>
  );
}
