import {
  forwardRef,
  useImperativeHandle,
  useRef,
  type ButtonHTMLAttributes,
  type RefObject,
} from "react";
import { classNames } from "../design-system/classNames";
import { captureDialogOrigin, type DialogOriginRef } from "./dialogOrigin";
import { usePressFeedback } from "./usePressFeedback";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  dialogOriginRef?: DialogOriginRef;
  /** Transient/async-reflow controls supply their explicit persistent return anchor. */
  dialogReturnRef?: RefObject<HTMLElement>;
  /** Keep positioned or measured hosts stable; animate only their visual child. */
  pressVisualRef?: RefObject<HTMLElement>;
}

/** Unstyled semantic button, also composed under Radix's asChild controls. */
export const PressableButton = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      dialogOriginRef,
      dialogReturnRef,
      pressVisualRef,
      onClick,
      type = "button",
      disabled,
      ...props
    },
    forwardedRef,
  ) => {
    const ref = useRef<HTMLButtonElement>(null);
    useImperativeHandle(forwardedRef, () => ref.current!);
    usePressFeedback(ref, disabled, pressVisualRef);
    return (
      <button
        {...props}
        ref={ref}
        type={type}
        disabled={disabled}
        data-pressable="true"
        onClick={(event) => {
          if (disabled) return;
          if (dialogOriginRef)
            captureDialogOrigin(
              dialogOriginRef,
              event.currentTarget,
              dialogReturnRef?.current,
            );
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
