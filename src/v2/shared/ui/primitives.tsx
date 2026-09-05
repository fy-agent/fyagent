import {
  forwardRef,
  useRef,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type RefObject,
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

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  actions,
  size = "standard",
  initialFocusRef,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children?: ReactNode;
  actions?: ReactNode;
  size?: "standard" | "comfortable" | "wide";
  initialFocusRef?: RefObject<HTMLElement>;
}) {
  const visible = usePersistentVisibility();
  const presented = open && visible;
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  return (
    <DialogPrimitive.Root
      open={presented}
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
            size !== "standard" && `fy-control-dialog-${size}`,
          )}
          {...(!description ? { "aria-describedby": undefined } : {})}
          onOpenAutoFocus={(event) => {
            const focused = document.activeElement;
            if (focused instanceof HTMLElement && focused !== document.body) {
              restoreFocusRef.current = focused;
            }
            const initial = initialFocusRef?.current;
            if (initial && !initial.matches(":disabled")) {
              event.preventDefault();
              initial.focus();
            }
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            const node = restoreFocusRef.current;
            window.requestAnimationFrame(() => {
              if (node?.isConnected && !node.closest("[hidden], [inert]")) {
                node.focus();
              }
            });
          }}
        >
          <div className="fy-control-dialog-content">
            <header className="fy-control-dialog-header">
              <DialogPrimitive.Title className="fy-control-dialog-title">
                {title}
              </DialogPrimitive.Title>
              {description && (
                <DialogPrimitive.Description className="fy-control-dialog-description">
                  {description}
                </DialogPrimitive.Description>
              )}
            </header>
            {children != null && (
              <div className="fy-control-dialog-body">{children}</div>
            )}
          </div>
          {actions && (
            <footer className="fy-control-dialog-actions">{actions}</footer>
          )}
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
  const cancelRef = useRef<HTMLButtonElement>(null);
  return (
    <Dialog
      open={open}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={title}
      description={description}
      actions={
        <>
          <Button ref={cancelRef} onClick={onCancel} disabled={pending}>
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
    />
  );
}
