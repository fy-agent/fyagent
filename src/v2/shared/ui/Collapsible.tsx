import {
  Content as CollapsibleContentPrimitive,
  Root as CollapsibleRootPrimitive,
  Trigger as CollapsibleTriggerPrimitive,
} from "@radix-ui/react-collapsible";
import {
  useLayoutEffect,
  useRef,
  type HTMLAttributes,
  type ReactNode,
} from "react";

import { classNames } from "../design-system/classNames";
import {
  animate,
  fyMotionTransition,
  fySpringTransition,
  motion,
  useMotionValue,
  useReducedMotion,
} from "./motion";

import "./collapsible.css";

type CollapsibleClosedProps = {
  inert?: "";
  "aria-hidden"?: boolean;
};

export function Collapsible({
  open,
  onOpenChange,
  children,
  className,
  asChild,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
  className?: string;
  asChild?: boolean;
}) {
  return (
    <CollapsibleRootPrimitive
      open={open}
      onOpenChange={onOpenChange}
      className={className}
      asChild={asChild}
    >
      {children}
    </CollapsibleRootPrimitive>
  );
}

export const CollapsibleTrigger = CollapsibleTriggerPrimitive;

function CollapsibleMotionPanel({
  open,
  children,
}: {
  open: boolean;
  children: ReactNode;
}) {
  const reduceMotion = useReducedMotion() === true;
  const panelRef = useRef<HTMLDivElement>(null);
  const height = useMotionValue<number | "auto">(open ? "auto" : 0);
  const lastOpenHeightRef = useRef(0);
  const hasMountedRef = useRef(false);
  const generationRef = useRef(0);

  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }

    if (open) {
      const measured = panel.scrollHeight;
      if (measured > 0) {
        lastOpenHeightRef.current = measured;
      }
    }

    if (!hasMountedRef.current) {
      hasMountedRef.current = true;
      height.set(open ? "auto" : 0);
      return;
    }

    if (reduceMotion) {
      height.set(open ? "auto" : 0);
      return;
    }

    const generation = ++generationRef.current;

    if (open) {
      const target = panel.scrollHeight || lastOpenHeightRef.current;
      if (height.get() === "auto") {
        height.set(0);
      }
      const controls = animate(height, target, fySpringTransition);
      void controls.then(() => {
        if (generationRef.current !== generation) {
          return;
        }
        height.set("auto");
      });
      return () => {
        controls.stop();
      };
    }

    const current = height.get();
    height.set(current === "auto" ? lastOpenHeightRef.current : current);
    const controls = animate(height, 0, fySpringTransition);
    return () => {
      controls.stop();
    };
  }, [height, open, reduceMotion]);

  const closedProps: CollapsibleClosedProps = open
    ? {}
    : { inert: "", "aria-hidden": true };

  return (
    <motion.div
      ref={panelRef}
      className="fy-collapsible-panel"
      style={{ height }}
      {...closedProps}
    >
      {children}
    </motion.div>
  );
}

export function CollapsibleContent({
  open,
  children,
  className,
  ...props
}: {
  open: boolean;
  children: ReactNode;
  className?: string;
} & Omit<HTMLAttributes<HTMLDivElement>, "children">) {
  return (
    <CollapsibleContentPrimitive forceMount className={className} {...props}>
      <CollapsibleMotionPanel open={open}>{children}</CollapsibleMotionPanel>
    </CollapsibleContentPrimitive>
  );
}

export function CollapsibleCaret({
  open,
  children,
  className,
}: {
  open: boolean;
  children: ReactNode;
  className?: string;
}) {
  const reduceMotion = useReducedMotion() === true;

  return (
    <motion.span
      className={classNames("fy-collapsible-caret", className)}
      initial={false}
      animate={{ rotate: open ? 180 : 0 }}
      transition={fyMotionTransition(reduceMotion)}
      aria-hidden
    >
      {children}
    </motion.span>
  );
}
