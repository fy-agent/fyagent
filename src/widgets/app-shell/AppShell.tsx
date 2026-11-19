import { lazy, Suspense } from "react";

import { classNames } from "../../shared/design-system/classNames";
import { shouldShowMacOverlayDragStrip } from "../../shared/platform";
import { PrimaryBlockerProvider } from "../../shared/ui/PrimaryBlocker";
import { TooltipProvider } from "../../shared/ui/primitives";
import { ContentViewport } from "./ContentViewport";
import { SideNavigation } from "./SideNavigation";
import { TopBar } from "./TopBar";

const CodexReloadBanner = lazy(() => import("./CodexReloadBanner"));

export function AppShell() {
  const macosOverlay = shouldShowMacOverlayDragStrip();

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
        <Suspense fallback={null}>
          <CodexReloadBanner />
        </Suspense>
        <PrimaryBlockerProvider>
          <div className="fy-shell-body">
            <SideNavigation />
            <ContentViewport />
          </div>
        </PrimaryBlockerProvider>
      </div>
    </TooltipProvider>
  );
}
