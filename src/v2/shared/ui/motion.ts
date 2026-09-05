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

export const fySpringTransition = {
  type: "spring",
  visualDuration: 0.26,
  bounce: 0.07,
} as const;

export const fyPressRecovery = {
  type: "spring",
  stiffness: 280,
  damping: 18,
  mass: 0.7,
} as const;

export const fySurfaceEase = [0.16, 1, 0.3, 1] as const;
export const fyPressScale = {
  target: 0.975,
  minimum: 0.96,
  maximum: 1.004,
} as const;

/** CSS owns duration tokens; callers read them when an interaction starts. */
export function motionDuration(
  role: "press" | "dialog-enter" | "dialog-exit" | "content" | "toast",
): number {
  if (typeof document === "undefined") return 0;
  const milliseconds = Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue(
      `--fy-motion-${role}`,
    ),
  );
  return Number.isFinite(milliseconds) ? Math.max(0, milliseconds) / 1000 : 0;
}

export function fyMotionTransition(reduceMotion: boolean) {
  return reduceMotion ? { duration: 0 } : fySpringTransition;
}
