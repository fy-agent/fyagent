import { forwardRef, type InputHTMLAttributes, type ReactNode } from "react";
import { classNames } from "../design-system/classNames";
import { PressableButton } from "./Button";
import { CheckboxPrimitive, SwitchPrimitive, TooltipPrimitive } from "./vendor";

export function Tooltip({
  label,
  children,
  testId,
}: {
  label: ReactNode;
  children: ReactNode;
  testId?: string;
}) {
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

export const Input = forwardRef<
  HTMLInputElement,
  InputHTMLAttributes<HTMLInputElement>
>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={classNames("fy-control-input", className)}
    {...props}
  />
));
Input.displayName = "Input";
export { SecretInput, type SecretInputProps } from "./SecretInput";

export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "warning";
}) {
  return (
    <span className={`fy-control-badge fy-control-badge-${tone}`}>
      {children}
    </span>
  );
}

export function Switch({
  checked,
  onCheckedChange,
  label,
  disabled,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <SwitchPrimitive.Root
      asChild
      className="fy-control-switch"
      checked={checked}
      onCheckedChange={onCheckedChange}
      aria-label={label}
      disabled={disabled}
    >
      <PressableButton>
        <SwitchPrimitive.Thumb className="fy-control-switch-thumb" />
      </PressableButton>
    </SwitchPrimitive.Root>
  );
}

export function Checkbox({
  checked,
  onCheckedChange,
  label,
  disabled,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <CheckboxPrimitive.Root
      asChild
      className="fy-control-checkbox"
      checked={checked}
      onCheckedChange={(value) => onCheckedChange(value === true)}
      aria-label={label}
      disabled={disabled}
    >
      <PressableButton>
        <CheckboxPrimitive.Indicator>✓</CheckboxPrimitive.Indicator>
      </PressableButton>
    </CheckboxPrimitive.Root>
  );
}

export function InlineNotice({
  children,
  tone = "info",
}: {
  children: ReactNode;
  tone?: "info" | "error" | "warning";
}) {
  return (
    <div
      className={`fy-control-notice fy-control-notice-${tone}`}
      role={tone === "error" ? "alert" : "status"}
    >
      {children}
    </div>
  );
}

export function Spinner({ label = "加载中" }: { label?: string }) {
  return (
    <span className="fy-control-spinner" role="status" aria-label={label} />
  );
}

export function EmptyState({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="fy-control-empty">
      <h2>{title}</h2>
      {description ? <p>{description}</p> : null}
      {children}
      {actions && <div>{actions}</div>}
    </div>
  );
}
