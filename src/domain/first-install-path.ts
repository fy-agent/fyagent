/**
 * Windows installation/login hints and the recommended-install version filter.
 * Presentation lives in the agents pages. This module has no React.
 */

export const WINDOWS_LOGIN_SPIN_HINT =
  "登录窗口如果一直转圈，先留在已经打开的官方窗口里等它完成，然后回到这里刷新状态。窗口关掉了还在转圈，先停止等待，再登录一次。";

export const WINDOWS_ENV_HINT =
  "装完后如果终端提示找不到命令，先完全退出并重新打开终端或 FyAgent。旧窗口往往还记着安装前的环境变量。保持当前用户的安装方式即可，不必改整台电脑的环境变量。";

const PRERELEASE_MARKER =
  /(?:^|[^0-9A-Za-z])(?:alpha|beta|preview|nightly|rc)(?:[^0-9A-Za-z]|$)/iu;

const SEMVER_PRERELEASE = /^v?\d+\.\d+\.\d+-/u;

export function isPrereleaseInstallLabel(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return false;
  if (SEMVER_PRERELEASE.test(trimmed)) return true;
  return PRERELEASE_MARKER.test(trimmed);
}

/** Stable labels only. A prerelease label is omitted, not rewritten into a version. */
export function recommendedInstallLabels(labels: readonly string[]): string[] {
  return labels.filter(
    (label) => label.trim().length > 0 && !isPrereleaseInstallLabel(label),
  );
}
