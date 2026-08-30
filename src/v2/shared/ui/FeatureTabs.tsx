import type { HTMLAttributes, ReactNode } from "react";

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
  id,
  label,
  value,
  onChange,
  options,
  activationMode = "automatic",
  className,
}: {
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
        if (option) onChange(option.id);
      }}
    >
      <SelectionLensTrack id={id} className="fy-feature-tabs-list">
        <Tabs.List className="fy-feature-tabs-list-semantic" aria-label={label}>
          {options.map((option) => {
            const selected = option.id === value;
            return (
              <Tabs.Trigger
                key={option.id}
                id={featureTabTriggerId(id, option.id)}
                value={option.id}
                className="fy-feature-tab"
                aria-controls={featureTabPanelId(id, option.id)}
              >
                <SelectionLens active={selected} />
                {typeof option.label === "string" ? (
                  <span>{option.label}</span>
                ) : (
                  option.label
                )}
              </Tabs.Trigger>
            );
          })}
        </Tabs.List>
      </SelectionLensTrack>
    </Tabs.Root>
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
      className={className}
      hidden={!active}
      tabIndex={0}
      {...props}
    >
      {children}
    </div>
  );
}
