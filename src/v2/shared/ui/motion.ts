export {
  animate,
  motion,
  useMotionValue,
  useReducedMotion,
} from "framer-motion";

export const fySpringTransition = {
  type: "spring",
  stiffness: 520,
  damping: 42,
  mass: 0.62,
} as const;

export function fyMotionTransition(reduceMotion: boolean) {
  return reduceMotion ? { duration: 0 } : fySpringTransition;
}
