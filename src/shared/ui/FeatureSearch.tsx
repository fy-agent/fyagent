import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { XIcon } from "@phosphor-icons/react/dist/csr/X";
import { useRef } from "react";

import { classNames } from "../design-system/classNames";
import { Input } from "./primitives";
import { PressableButton } from "./Button";

export function FeatureSearch({
  value,
  onValueChange,
  placeholder,
  ariaLabel,
  clearLabel = "清除搜索",
  className,
  disabled,
  id,
}: {
  value: string;
  onValueChange: (value: string) => void;
  placeholder: string;
  ariaLabel: string;
  clearLabel?: string;
  className?: string;
  disabled?: boolean;
  id?: string;
}) {
  const clearVisualRef = useRef<HTMLSpanElement>(null);
  return (
    <div role="search" className={classNames("fy-feature-search", className)}>
      <MagnifyingGlassIcon
        className="fy-feature-search-icon"
        size={16}
        weight="regular"
        aria-hidden
      />
      <Input
        id={id}
        type="search"
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        aria-label={ariaLabel}
        autoComplete="off"
        spellCheck={false}
        onChange={(event) => onValueChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape" && value) {
            event.stopPropagation();
            onValueChange("");
          }
        }}
      />
      {value ? (
        <PressableButton
          pressVisualRef={clearVisualRef}
          type="button"
          className="fy-feature-search-clear"
          aria-label={clearLabel}
          title={clearLabel}
          disabled={disabled}
          onClick={() => onValueChange("")}
        >
          <span ref={clearVisualRef} className="fy-control-icon-feedback">
            <XIcon size={14} aria-hidden />
          </span>
        </PressableButton>
      ) : null}
    </div>
  );
}
