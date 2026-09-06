import {
  Content as CollapsibleContentPrimitive,
  Root as CollapsibleRootPrimitive,
  Trigger as CollapsibleTriggerPrimitive,
} from "@radix-ui/react-collapsible";
import type { HTMLAttributes, ReactNode } from "react";

import { classNames } from "../design-system/classNames";
import {
  fyMotionTransition,
  fySelectionTransition,
  motion,
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

  const closedProps: CollapsibleClosedProps = open
    ? {}
    : { inert: "", "aria-hidden": true };

  return (
    <motion.div
      className="fy-collapsible-panel"
      initial={false}
      animate={{ height: open ? "auto" : 0 }}
      transition={reduceMotion ? { duration: 0 } : fySelectionTransition}
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
