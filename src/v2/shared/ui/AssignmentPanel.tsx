import { getSkillTargetIcon } from "../assets/apps";
import type { SkillTargetId } from "../features/types";
import { Switch } from "./primitives";

type TargetOption<T extends SkillTargetId> = { id: T; label: string };

type SwitchAssignmentPanelProps<T extends SkillTargetId> = {
  mode?: "switch";
  apps: Record<string, boolean | undefined>;
  disabled?: boolean;
  labelSuffix: string;
  onToggle: (app: T, enabled: boolean) => void;
  targets: ReadonlyArray<TargetOption<T>>;
};

type RadioAssignmentPanelProps<T extends SkillTargetId> = {
  mode: "radio";
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
  ariaLabel: string;
  targets: ReadonlyArray<TargetOption<T>>;
};

export type AssignmentPanelProps<T extends SkillTargetId> =
  | SwitchAssignmentPanelProps<T>
  | RadioAssignmentPanelProps<T>;

function TargetIcon({ id }: { id: SkillTargetId }) {
  return (
    <img
      className="fy-feature-assignment-icon"
      src={getSkillTargetIcon(id)}
      alt=""
      aria-hidden="true"
    />
  );
}

export function AssignmentPanel<T extends SkillTargetId>(
  props: AssignmentPanelProps<T>,
) {
  if (props.mode === "radio") {
    return (
      <div
        className="fy-feature-target-picker"
        role="radiogroup"
        aria-label={props.ariaLabel}
      >
        {props.targets.map((app) => {
          const selected = app.id === props.value;
          return (
            <button
              key={app.id}
              type="button"
              role="radio"
              aria-checked={selected}
              className="fy-feature-target-option"
              disabled={props.disabled}
              onClick={() => props.onChange(app.id)}
            >
              <TargetIcon id={app.id} />
              <span>{app.label}</span>
            </button>
          );
        })}
      </div>
    );
  }

  return (
    <div className="fy-feature-assignments">
      <h3>应用分配</h3>
      {props.targets.map((app) => (
        <label key={app.id} className="fy-feature-assignment">
          <span className="fy-feature-assignment-label">
            <TargetIcon id={app.id} />
            <span>{app.label}</span>
          </span>
          <Switch
            checked={Boolean(props.apps[app.id])}
            onCheckedChange={(checked) => props.onToggle(app.id, checked)}
            label={`${app.label} ${props.labelSuffix}`}
            disabled={props.disabled}
          />
        </label>
      ))}
    </div>
  );
}
