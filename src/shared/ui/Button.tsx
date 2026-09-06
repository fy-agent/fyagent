import {
  forwardRef,
  useImperativeHandle,
  useRef,
  type ButtonHTMLAttributes,
} from "react";
import { classNames } from "../design-system/classNames";
import type { DialogOriginRef } from "./dialogOrigin";
import { usePressFeedback } from "./usePressFeedback";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  dialogOriginRef?: DialogOriginRef;
}

/** Unstyled semantic button, also composed under Radix's asChild controls. */
export const PressableButton = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    { dialogOriginRef, onClick, type = "button", disabled, ...props },
    forwardedRef,
  ) => {
    const ref = useRef<HTMLButtonElement>(null);
    useImperativeHandle(forwardedRef, () => ref.current!);
    usePressFeedback(ref, disabled);
    return (
      <button
        {...props}
        ref={ref}
        type={type}
        disabled={disabled}
        data-pressable="true"
        onClick={(event) => {
          if (disabled) return;
          if (dialogOriginRef) dialogOriginRef.current = event.currentTarget;
          onClick?.(event);
        }}
      />
    );
  },
);
PressableButton.displayName = "PressableButton";

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, ...props }, ref) => (
    <PressableButton
      ref={ref}
      className={classNames("fy-control-button", className)}
      {...props}
    />
  ),
);
Button.displayName = "Button";
export const GlassButton = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, ...props }, ref) => (
    <PressableButton
      ref={ref}
      className={classNames("fy-glass-button", className)}
      {...props}
    />
  ),
);
GlassButton.displayName = "GlassButton";
export const IconButton = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, ...props }, ref) => (
    <PressableButton
      ref={ref}
      className={classNames("fy-icon-button", className)}
      {...props}
    />
  ),
);
IconButton.displayName = "IconButton";
