import { lazy, Suspense, useRef, useState } from "react";
import { classNames } from "../../shared/design-system/classNames";
import { shouldShowMacOverlayDragStrip } from "../../shared/platform";
import { Button } from "../../shared/ui/Button";
import { Brand } from "./Brand";
import { ThemeToggle } from "./ThemeToggle";
import "./top-bar-actions.css";

const AboutDialog = lazy(() => import("./AboutDialog"));

export function TopBar() {
  const showMacOverlayDragStrip = shouldShowMacOverlayDragStrip();
  const [aboutOpen, setAboutOpen] = useState<boolean | null>(null);
  const aboutOrigin = useRef<HTMLElement | null>(null);

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
        <div className="fy-top-bar-actions">
          <ThemeToggle />
          <Button
            className="fy-about-trigger"
            aria-label="关于 FyAgent"
            aria-haspopup="dialog"
            dialogOriginRef={aboutOrigin}
            onClick={() => {
              setAboutOpen(true);
            }}
          >
            关于
          </Button>
        </div>
      </div>
      {aboutOpen !== null ? (
        <Suspense fallback={null}>
          <AboutDialog
            open={aboutOpen}
            onOpenChange={setAboutOpen}
            originRef={aboutOrigin}
          />
        </Suspense>
      ) : null}
    </header>
  );
}
