import { Glass, type GlassOptics } from "@samasante/liquid-glass";
import { useMemo, type ReactNode } from "react";

import { classNames } from "../design-system/classNames";

const surfaceOptics: Partial<GlassOptics> = {
  strength: 0.012,
  dispersion: 0,
  brightness: 0,
  glow: 0,
  sheen: 0.12,
  specular: 0.3,
};

/** A decorative backing only: never clone a page, credential form or live text. */
export function FrostedSurface() {
  const optics = useMemo(() => {
    const blur =
      typeof document === "undefined"
        ? 0
        : Number.parseFloat(
            getComputedStyle(document.documentElement).getPropertyValue(
              "--fy-surface-blur",
            ),
          );
    return {
      ...surfaceOptics,
      frost: Number.isFinite(blur) ? Math.max(0, blur) : 0,
    };
  }, []);
  const canRenderLens =
    typeof ResizeObserver !== "undefined" &&
    typeof CanvasRenderingContext2D !== "undefined";
  if (!canRenderLens) return <div className="fy-frosted-surface" aria-hidden />;
  return (
    <Glass className="fy-frosted-surface" optics={optics} aria-hidden>
      {/* Bare wrapping selects the library's material mode, not DOM-copy mode. */}
      <span />
    </Glass>
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
