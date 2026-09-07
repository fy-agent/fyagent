import { useRef, type HTMLAttributes, type ReactNode } from "react";
import { usePressFeedback } from "./usePressFeedback";
import type { DialogOriginRef } from "./dialogOrigin";

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
  originRef,
  selected,
  onSelect,
  title,
  children,
  ariaLabel,
}: {
  originRef?: DialogOriginRef;
  selected: boolean;
  onSelect: () => void;
  title: ReactNode;
  children?: ReactNode;
  ariaLabel?: string;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  usePressFeedback(ref, false, labelRef);
  return (
    <button
      ref={ref}
      type="button"
      className="fy-feature-list-item"
      aria-current={selected ? true : undefined}
      aria-label={ariaLabel}
      onClick={(event) => {
        if (originRef) originRef.current = event.currentTarget;
        onSelect();
      }}
    >
      <SelectionLens active={selected} />
      <span ref={labelRef} className="fy-feature-list-copy">
        <strong>{title}</strong>
        {children}
      </span>
    </button>
  );
}
