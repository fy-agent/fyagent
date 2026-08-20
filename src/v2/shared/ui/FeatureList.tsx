import { type HTMLAttributes, type ReactNode } from "react";

import { classNames } from "../design-system/classNames";
import { SelectionLens, SelectionLensTrack } from "./SelectionLens";

export function FeatureList({
  id,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & { id: string }) {
  return (
    <SelectionLensTrack
      id={id}
      className={classNames("fy-feature-list", className)}
      {...props}
    >
      {children}
    </SelectionLensTrack>
  );
}

export function FeatureListItem({
  selected,
  onSelect,
  title,
  children,
  ariaLabel,
}: {
  selected: boolean;
  onSelect: () => void;
  title: ReactNode;
  children?: ReactNode;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      className="fy-feature-list-item"
      aria-current={selected ? true : undefined}
      aria-label={ariaLabel}
      onClick={onSelect}
    >
      <SelectionLens active={selected} />
      <strong>{title}</strong>
      {children}
    </button>
  );
}
