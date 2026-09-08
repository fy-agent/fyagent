import { Glass } from "@samasante/liquid-glass";
import type { ReactNode } from "react";

import { classNames } from "../design-system/classNames";

/** Stable CSS material for large business surfaces. The same backing and rim
 * survive the motion handoff; no displacement-map rebuild or DOM snapshot. */
export function FrostedSurface({ enhanced = true }: { enhanced?: boolean }) {
  return (
    <div className="fy-frosted-surface" data-enhanced={enhanced} aria-hidden>
      <span className="fy-glass-rim" />
    </div>
  );
}

interface LiquidGlassLensProps {
  children: ReactNode;
  className?: string;
}

export function LiquidGlassLens({ children, className }: LiquidGlassLensProps) {
  return (
    <Glass
      className={classNames("fy-liquid-glass-lens", className)}
      data-testid="liquid-glass-lens"
      optics={{ dispersion: 0 }}
      live={false}
      filterResolution={1}
    >
      {children}
    </Glass>
  );
}
