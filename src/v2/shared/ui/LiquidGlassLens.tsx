import { Glass } from "@samasante/liquid-glass";
import type { ReactNode } from "react";

import { classNames } from "../design-system/classNames";

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
