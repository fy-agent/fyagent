import { useEffect } from "react";

import {
  signalFrontendReady,
  windowFramePort,
  type WindowFramePort,
} from "../../shared/platform";
import { TooltipProvider } from "../../shared/ui/primitives";
import { ContentViewport } from "./ContentViewport";
import { TopBar } from "./TopBar";

interface AppShellProps {
  frame?: WindowFramePort;
}

export function AppShell({ frame = windowFramePort }: AppShellProps) {
  useEffect(() => {
    void Promise.all([frame.prepareFrame(), signalFrontendReady()]).catch(
      (error: unknown) => {
        console.error("FyAgent V2 native frame preparation failed", error);
      },
    );
  }, [frame]);

  return (
    <TooltipProvider delayDuration={250} skipDelayDuration={100}>
      <div className="fy-app-shell" data-testid="app-shell">
        <TopBar frame={frame} />
        <ContentViewport />
      </div>
    </TooltipProvider>
  );
}
