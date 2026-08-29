import type {
  AgentActionId,
  AgentInstallationTarget,
} from "../features/agent-install-readiness";
import { Button, InlineNotice, Spinner } from "./primitives";

function scopeCopy(scope: AgentInstallationTarget["scope"]): string {
  switch (scope) {
    case "current_user":
      return "当前用户";
    case "all_users":
      return "所有用户";
    case "custom":
      return "自定义位置";
    case "unknown":
      return "范围未确认";
  }
}

export function LifecycleTargetPicker({
  id,
  action,
  targets,
  value,
  onChange,
  loading = false,
  error = null,
  disabled = false,
  onRefresh,
}: {
  id: string;
  action: AgentActionId;
  targets: readonly AgentInstallationTarget[];
  value: string | null;
  onChange: (target: AgentInstallationTarget) => void;
  loading?: boolean;
  error?: string | null;
  disabled?: boolean;
  onRefresh?: () => void;
}) {
  if (loading) {
    return (
      <div className="fy-agent-target-picker-status">
        <Spinner label="正在读取安装目标" />
        <span>正在读取安装目标</span>
      </div>
    );
  }

  if (error) {
    return (
      <InlineNotice tone="warning">
        <span>{error}</span>
        {onRefresh ? (
          <Button type="button" onClick={onRefresh} disabled={disabled}>
            重新读取
          </Button>
        ) : null}
      </InlineNotice>
    );
  }

  if (targets.length === 0) {
    return (
      <InlineNotice tone="warning">
        当前没有经过验证、可用于此操作的安装目标。
      </InlineNotice>
    );
  }

  return (
    <fieldset
      className="fy-agent-target-picker"
      aria-label="安装目标"
      disabled={disabled}
    >
      <legend>本次操作目标</legend>
      <div className="fy-agent-target-options">
        {targets.map((target) => {
          const eligible = target.eligibleActions.includes(action);
          const inputId = `${id}-${target.targetId}`;
          return (
            <label
              key={target.targetId}
              htmlFor={inputId}
              className="fy-agent-target-option"
              data-selected={value === target.targetId ? "true" : "false"}
              data-disabled={!eligible ? "true" : "false"}
            >
              <input
                id={inputId}
                name={id}
                type="radio"
                value={target.targetId}
                checked={value === target.targetId}
                disabled={disabled || !eligible}
                onChange={() => onChange(target)}
              />
              <span className="fy-agent-target-option-copy">
                <strong>{target.label}</strong>
                <small>{scopeCopy(target.scope)}</small>
              </span>
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}
