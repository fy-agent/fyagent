import { EyeIcon } from "@phosphor-icons/react/dist/csr/Eye";
import { EyeSlashIcon } from "@phosphor-icons/react/dist/csr/EyeSlash";
import { forwardRef, useRef, useState, type InputHTMLAttributes } from "react";

import { classNames } from "../design-system/classNames";
import { PressableButton } from "./Button";

export type SecretInputProps = Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type"
> & {
  revealLabel?: string;
  hideLabel?: string;
};

export const SecretInput = forwardRef<HTMLInputElement, SecretInputProps>(
  (
    { className, disabled, revealLabel = "显示", hideLabel = "隐藏", ...props },
    ref,
  ) => {
    const [visible, setVisible] = useState(false);
    const visualRef = useRef<HTMLSpanElement>(null);
    const toggleLabel = visible ? hideLabel : revealLabel;

    return (
      <div className="fy-control-secret">
        <input
          ref={ref}
          className={classNames("fy-control-input", className)}
          disabled={disabled}
          {...props}
          type={visible ? "text" : "password"}
        />
        <PressableButton
          pressVisualRef={visualRef}
          type="button"
          className="fy-control-secret-toggle"
          aria-label={toggleLabel}
          aria-pressed={visible}
          disabled={disabled}
          onClick={() => setVisible((current) => !current)}
        >
          <span ref={visualRef} className="fy-control-icon-feedback">
            {visible ? (
              <EyeSlashIcon size={16} weight="regular" aria-hidden />
            ) : (
              <EyeIcon size={16} weight="regular" aria-hidden />
            )}
          </span>
        </PressableButton>
      </div>
    );
  },
);

SecretInput.displayName = "SecretInput";
