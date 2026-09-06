import { useRef, type HTMLAttributes, type ReactNode } from "react";
import { usePressFeedback } from "./usePressFeedback";
import type { DialogOriginRef } from "./dialogOrigin";

import { classNames } from "../design-system/classNames";
import { SelectionLens, SelectionLensTrack } from "./SelectionLens";
import { TabsPrimitive as Tabs } from "./vendor";

export type FeatureTabOption<T extends string> = {
  id: T;
  label: ReactNode;
};

function safeIdPart(value: string): string {
  return value.replace(/[^a-zA-Z0-9_-]/gu, "-");
}

export function featureTabTriggerId(tabsId: string, value: string): string {
  return `${tabsId}-trigger-${safeIdPart(value)}`;
}

export function featureTabPanelId(tabsId: string, value: string): string {
  return `${tabsId}-panel-${safeIdPart(value)}`;
}

export function FeatureTabs<T extends string>({
  originRef,
  id,
  label,
  value,
  onChange,
  options,
  activationMode = "automatic",
  className,
}: {
  originRef?: DialogOriginRef;
  id: string;
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: ReadonlyArray<FeatureTabOption<T>>;
  activationMode?: "automatic" | "manual";
  className?: string;
}) {
  return (
    <Tabs.Root
      className={classNames("fy-feature-tabs", className)}
      value={value}
      activationMode={activationMode}
      orientation="horizontal"
      onValueChange={(nextValue) => {
        const option = options.find((candidate) => candidate.id === nextValue);
        if (option) {
          if (originRef)
            originRef.current = document.getElementById(
              featureTabTriggerId(id, option.id),
            );
          onChange(option.id);
        }
      }}
    >
      <SelectionLensTrack id={id} className="fy-feature-tabs-list">
        <Tabs.List className="fy-feature-tabs-list-semantic" aria-label={label}>
          {options.map((option) => {
            const selected = option.id === value;
            return (
              <FeatureTabTrigger
                key={option.id}
                id={featureTabTriggerId(id, option.id)}
                value={option.id}
                className="fy-feature-tab"
                aria-controls={featureTabPanelId(id, option.id)}
                selected={selected}
              >
                {option.label}
              </FeatureTabTrigger>
            );
          })}
        </Tabs.List>
      </SelectionLensTrack>
    </Tabs.Root>
  );
}

function FeatureTabTrigger({
  selected,
  children,
  ...props
}: {
  selected: boolean;
  children: ReactNode;
  id: string;
  value: string;
  className: string;
  "aria-controls": string;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  usePressFeedback(ref, false, labelRef);
  return (
    <Tabs.Trigger {...props} ref={ref}>
      <SelectionLens active={selected} />
      <span ref={labelRef} className="fy-feature-tab-label">
        {children}
      </span>
    </Tabs.Trigger>
  );
}

export function FeatureTabPanel<T extends string>({
  tabsId,
  value,
  active,
  unmountOnExit = false,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id" | "role"> & {
  tabsId: string;
  value: T;
  active: boolean;
  unmountOnExit?: boolean;
  children: ReactNode;
}) {
  if (!active && unmountOnExit) {
    return null;
  }
  return (
    <div
      id={featureTabPanelId(tabsId, value)}
      role="tabpanel"
      aria-labelledby={featureTabTriggerId(tabsId, value)}
      className={classNames("fy-feature-tab-panel", className)}
      hidden={!active}
      tabIndex={0}
      {...props}
    >
      {children}
    </div>
  );
}
