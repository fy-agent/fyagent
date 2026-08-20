import {
  forwardRef,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
} from "react";

import { classNames } from "../design-system/classNames";
import { usePersistentVisibility } from "./PersistentSurface";
import {
  CheckboxPrimitive,
  DialogPrimitive,
  SwitchPrimitive,
  TooltipPrimitive,
} from "./vendor";

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

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, type = "button", ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={classNames("fy-control-button", className)}
      {...props}
    />
  ),
);
Button.displayName = "Button";

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
      className="fy-control-switch"
      checked={checked}
      onCheckedChange={onCheckedChange}
      aria-label={label}
      disabled={disabled}
    >
      <SwitchPrimitive.Thumb className="fy-control-switch-thumb" />
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
      className="fy-control-checkbox"
      checked={checked}
      onCheckedChange={(value) => onCheckedChange(value === true)}
      aria-label={label}
      disabled={disabled}
    >
      <CheckboxPrimitive.Indicator>✓</CheckboxPrimitive.Indicator>
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
  description: string;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="fy-control-empty">
      <h2>{title}</h2>
      <p>{description}</p>
      {children}
      {actions && <div>{actions}</div>}
    </div>
  );
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  actions,
  large = false,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children: ReactNode;
  actions?: ReactNode;
  large?: boolean;
}) {
  const visible = usePersistentVisibility();
  return (
    <DialogPrimitive.Root
      open={open && visible}
      onOpenChange={(next) => {
        if (!visible) return;
        onOpenChange(next);
      }}
    >
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fy-control-dialog-overlay" />
        <DialogPrimitive.Content
          className={classNames(
            "fy-control-dialog",
            large && "fy-control-dialog-large",
          )}
        >
          <header>
            <DialogPrimitive.Title>{title}</DialogPrimitive.Title>
            {description && (
              <DialogPrimitive.Description>
                {description}
              </DialogPrimitive.Description>
            )}
          </header>
          <div className="fy-control-dialog-body">{children}</div>
          {actions && <footer>{actions}</footer>}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

export function ConfirmDialog({
  open,
  title,
  description,
  pending,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  description: string;
  pending?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={title}
      description={description}
      actions={
        <>
          <Button autoFocus onClick={onCancel} disabled={pending}>
            取消
          </Button>
          <Button
            className="fy-control-button-danger"
            onClick={onConfirm}
            disabled={pending}
          >
            {pending ? "处理中…" : "确认"}
          </Button>
        </>
      }
    >
      <p>此操作需要你的明确确认。</p>
    </Dialog>
  );
}
