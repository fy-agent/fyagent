import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

import { classNames } from "../design-system/classNames";
import { TooltipPrimitive } from "./vendor";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement>;

export const GlassButton = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, type = "button", ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={classNames("fy-glass-button", className)}
      {...props}
    />
  ),
);

GlassButton.displayName = "GlassButton";

export const IconButton = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, type = "button", ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={classNames("fy-icon-button", className)}
      {...props}
    />
  ),
);

IconButton.displayName = "IconButton";

interface TooltipProps {
  label: ReactNode;
  children: ReactNode;
  testId?: string;
}

export function Tooltip({ label, children, testId }: TooltipProps) {
  return (
    <TooltipPrimitive.Root delayDuration={250}>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          className="fy-tooltip"
          sideOffset={8}
          data-testid={testId}
        >
          {label}
          <TooltipPrimitive.Arrow className="fy-tooltip-arrow" />
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}

export const TooltipProvider = TooltipPrimitive.Provider;
