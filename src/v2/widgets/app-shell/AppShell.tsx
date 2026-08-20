import { useEffect } from "react";

import { classNames } from "../../shared/design-system/classNames";
import {
  shouldShowMacOverlayDragStrip,
  signalFrontendReady,
} from "../../shared/platform";
import { PrimaryBlockerProvider } from "../../shared/ui/PrimaryBlocker";
import { TooltipProvider } from "../../shared/ui/primitives";
import { ContentViewport } from "./ContentViewport";
import { TopBar } from "./TopBar";

export function AppShell() {
  const macosOverlay = shouldShowMacOverlayDragStrip();

  useEffect(() => {
    void signalFrontendReady().catch((error: unknown) => {
      console.error("FyAgent V2 frontend lifecycle readiness failed", error);
    });
  }, []);

  return (
    <TooltipProvider delayDuration={250} skipDelayDuration={100}>
      <div
        className={classNames(
          "fy-app-shell",
          macosOverlay && "fy-app-shell-macos-overlay",
        )}
        data-testid="app-shell"
      >
        <TopBar />
        <PrimaryBlockerProvider>
          <ContentViewport />
        </PrimaryBlockerProvider>
      </div>
    </TooltipProvider>
  );
}
