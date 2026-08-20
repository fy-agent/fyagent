import type { ReactNode } from "react";

import { classNames } from "../design-system/classNames";
import { SelectionLens, SelectionLensTrack } from "./SelectionLens";

export type FeatureTabOption<T extends string> = {
  id: T;
  label: ReactNode;
};

export function FeatureTabs<T extends string>({
  id,
  label,
  value,
  onChange,
  options,
  className,
}: {
  id: string;
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: ReadonlyArray<FeatureTabOption<T>>;
  className?: string;
}) {
  return (
    <SelectionLensTrack
      id={id}
      className={classNames("fy-feature-tabs", className)}
      role="tablist"
      aria-label={label}
    >
      {options.map((option) => {
        const selected = option.id === value;
        return (
          <button
            key={option.id}
            type="button"
            className="fy-feature-tab"
            role="tab"
            aria-selected={selected}
            onClick={() => onChange(option.id)}
          >
            <SelectionLens active={selected} />
            {typeof option.label === "string" ? (
              <span>{option.label}</span>
            ) : (
              option.label
            )}
          </button>
        );
      })}
    </SelectionLensTrack>
  );
}
