import { EyeIcon } from "@phosphor-icons/react/dist/csr/Eye";
import { EyeSlashIcon } from "@phosphor-icons/react/dist/csr/EyeSlash";
import { forwardRef, useState, type InputHTMLAttributes } from "react";

import { classNames } from "../design-system/classNames";

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
        <button
          type="button"
          className="fy-control-secret-toggle"
          aria-label={toggleLabel}
          aria-pressed={visible}
          disabled={disabled}
          onClick={() => setVisible((current) => !current)}
        >
          {visible ? (
            <EyeSlashIcon size={16} weight="regular" aria-hidden />
          ) : (
            <EyeIcon size={16} weight="regular" aria-hidden />
          )}
        </button>
      </div>
    );
  },
);

SecretInput.displayName = "SecretInput";
