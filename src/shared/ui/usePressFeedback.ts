import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import {
  animate,
  fyPressRecovery,
  fyPressScale,
  motionDuration,
  press,
  styleEffect,
  useMotionValue,
  useReducedMotion,
  useTransform,
} from "./motion";
import { usePersistentVisibility } from "./PersistentSurface";

/** Motion owns filtering, spring solving and rendering. A separate visual target
 * lets navigation labels press without disturbing measured selection geometry. */
export function usePressFeedback<T extends HTMLElement>(
  ref: RefObject<T>,
  disabled = false,
  visualRef?: RefObject<HTMLElement>,
) {
  const reduce = useReducedMotion();
  const visible = usePersistentVisibility();
  const scale = useMotionValue(1);
  const boundedScale = useTransform(scale, (value) =>
    Math.max(fyPressScale.minimum, Math.min(fyPressScale.maximum, value)),
  );
  const gate = useRef({ disabled, reduce, visible });
  const animation = useRef<ReturnType<typeof animate> | null>(null);
  const epoch = useRef(0);
  const held = useRef(false);
  const releaseRef = useRef<(() => void) | null>(null);

  useLayoutEffect(() => {
    gate.current = { disabled, reduce, visible };
    if (reduce || !visible) {
      epoch.current += 1;
      animation.current?.stop();
      held.current = false;
      scale.set(1);
    } else if (disabled && held.current) releaseRef.current?.();
  }, [disabled, reduce, visible, scale]);

  useEffect(() => {
    const node = ref.current;
    const visual = visualRef?.current ?? node;
    if (!node || !visual) return;
    const detachStyle = styleEffect(visual, { scale: boundedScale });
    const release = () => {
      held.current = false;
      const revision = ++epoch.current;
      animation.current?.stop();
      if (gate.current.reduce || !gate.current.visible) {
        scale.set(1);
        return;
      }
      const recover = () => {
        if (epoch.current === revision)
          animation.current = animate(scale, 1, fyPressRecovery);
      };
      // Down/up within one frame still receives a short dip. The native click
      // and business action never wait for this decorative recovery.
      if (scale.get() > 0.985) {
        animation.current = animate(scale, fyPressScale.target, {
          duration: motionDuration("press"),
          ease: "easeOut",
        });
        void animation.current.then(recover);
      } else recover();
    };
    releaseRef.current = release;
    const begin = () => {
      if (
        gate.current.disabled ||
        gate.current.reduce ||
        !gate.current.visible ||
        node.matches(':disabled, [aria-disabled="true"]') ||
        node.closest("[hidden], [inert]")
      )
        return;
      epoch.current += 1;
      held.current = true;
      animation.current?.stop();
      animation.current = animate(scale, fyPressScale.target, {
        duration: motionDuration("press"),
        ease: "easeOut",
      });
      return release;
    };
    // Register once per host. Disabled/visibility changes only update admission,
    // not a new gesture subscription and its keyboard listeners.
    const cancelPress = press(node, begin);
    const keyDown = (event: KeyboardEvent) => {
      if (node.tagName === "BUTTON" && event.key === " " && !event.repeat)
        begin();
    };
    const keyUp = (event: KeyboardEvent) => {
      if (event.key === " " && held.current) release();
    };
    const blur = () => {
      if (held.current) release();
    };
    node.addEventListener("keydown", keyDown);
    node.addEventListener("keyup", keyUp);
    node.addEventListener("blur", blur);
    return () => {
      epoch.current += 1;
      animation.current?.stop();
      releaseRef.current = null;
      cancelPress();
      detachStyle();
      node.removeEventListener("keydown", keyDown);
      node.removeEventListener("keyup", keyUp);
      node.removeEventListener("blur", blur);
    };
  }, [ref, visualRef, boundedScale, scale]);
}
