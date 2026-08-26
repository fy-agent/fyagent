import type {
  CodexDesktopProgress,
  InstallerErrorDto,
  InstallerPrimaryAction,
  InstallerViewState,
} from "@/shared/codex-desktop";

import type { AgentLifecyclePrimaryAction } from "./useAgentLifecycleAction";

export type CodexDirectoryActionSource = {
  primaryAction: InstallerPrimaryAction;
  primaryDisabled: boolean;
  isActing: boolean;
  state: InstallerViewState;
  progress: CodexDesktopProgress | undefined;
  error: InstallerErrorDto | null;
  canCancel: boolean;
  operationFailed: boolean;
};

export type CodexDirectoryActionProjection = {
  primaryAction: AgentLifecyclePrimaryAction | null;
  busy: boolean;
  percent: number | null;
  state: InstallerViewState;
  error: InstallerErrorDto | null;
  canRun: boolean;
  canRetry: boolean;
  canCancel: boolean;
};

function projectPercent(
  progress: CodexDesktopProgress | undefined,
): number | null {
  const percent = progress?.percent;
  return typeof percent === "number" && Number.isFinite(percent)
    ? percent
    : null;
}

function projectPrimaryAction(
  action: InstallerPrimaryAction,
): AgentLifecyclePrimaryAction | null {
  return action === "install" || action === "update" ? action : null;
}

export function projectCodexDirectoryAction(
  source: CodexDirectoryActionSource,
): CodexDirectoryActionProjection {
  const primaryAction = projectPrimaryAction(source.primaryAction);
  const busy = source.isActing || source.state.startsWith("job_");
  return {
    primaryAction,
    busy,
    percent: projectPercent(source.progress),
    state: source.state,
    error: source.error,
    canRun: primaryAction !== null && !source.primaryDisabled && !busy,
    canRetry: source.primaryAction === "retry" || source.operationFailed,
    canCancel: source.canCancel,
  };
}
