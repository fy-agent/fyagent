export {
  animate,
  AnimatePresence,
  motion,
  press,
  styleEffect,
  usePresence,
  useIsPresent,
  useMotionValue,
  useTransform,
} from "framer-motion";
import { useMediaQuery } from "./useMediaQuery";

// The adopted Motion version reads this preference once. The existing native
// media-store pattern keeps it live without creating another animation engine.
export function useReducedMotion(): boolean {
  return useMediaQuery("(prefers-reduced-motion: reduce)");
}

export const fySpatialEase = [0.32, 0.72, 0, 1] as const;
export const fySpatialEasing = `cubic-bezier(${fySpatialEase.join(",")})`;
/** Full-window radius travel; the spatial curve crawls the last corner. */
export const fyThemeRevealEase = [0.25, 0.08, 0.25, 1] as const;
export const fyThemeRevealEasing = `cubic-bezier(${fyThemeRevealEase.join(",")})`;
export const fySelectionTransition = {
  type: "tween",
  duration: 0.3,
  ease: fySpatialEase,
} as const;

export const fyPressRecovery = {
  type: "spring",
  stiffness: 260,
  damping: 17,
  mass: 0.72,
} as const;

export const fyPressScale = {
  target: 0.96,
  minimum: 0.95,
  maximum: 1.004,
} as const;

/** CSS optimizers may serialize 420ms as .42s. A single CSS time, not a bare
 * parseFloat, is the boundary; missing, compound and nonfinite values fail closed. */
export function parseMotionDuration(value: string): number {
  const match = /^([+]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(ms|s)$/i.exec(
    value.trim(),
  );
  if (!match) return 0;
  const seconds =
    Number(match[1]) / (match[2].toLowerCase() === "ms" ? 1000 : 1);
  return Number.isFinite(seconds) ? seconds : 0;
}

/** CSS owns duration tokens; callers read them when an interaction starts. */
export function motionDuration(
  role:
    | "press"
    | "dialog-enter"
    | "dialog-exit"
    | "dialog-resize"
    | "content"
    | "toast"
    | "theme",
): number {
  if (typeof document === "undefined") return 0;
  return parseMotionDuration(
    getComputedStyle(document.documentElement).getPropertyValue(
      `--fy-motion-${role}`,
    ),
  );
}

export function fyMotionTransition(reduceMotion: boolean) {
  return reduceMotion ? { duration: 0 } : fySelectionTransition;
}
