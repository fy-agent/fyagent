/**
 * Beginner first-install copy and the recommended-install version filter.
 * Presentation lives in the agents/prompts pages. This module has no React.
 *
 * External installer questionnaires that were not recorded stay pending.
 * Do not invent those steps here.
 */

export type FirstInstallEvidence = "product" | "pending";

export interface FirstInstallStep {
  step: number;
  asks: string;
  installs: string;
  evidence: FirstInstallEvidence;
}

export const FIRST_INSTALL_STEPS: readonly FirstInstallStep[] = [
  {
    step: 1,
    asks: "你主要想用 AI 做什么？可以选日常办公、编程开发，或两者都用。也可以跳过引导。",
    installs: "这一步不安装软件。",
    evidence: "product",
  },
  {
    step: 2,
    asks: "从推荐里选一个软件，或回到上一步重新选用途。",
    installs: "点「开始配置」只进入该软件，仍不安装。",
    evidence: "product",
  },
  {
    step: 3,
    asks: "如果这台电脑上已经有配置，会问保留现有配置还是替换。读不到配置时，说明无法确认，可以重试或进入软件详情。",
    installs: "不安装，也不测试连接。",
    evidence: "product",
  },
  {
    step: 4,
    asks: "安装前检查会问安装位置、版本，以及这次会改什么。标为推荐的是当前用户位置。只有选择系统文件夹时才要求管理员授权。覆盖安装保留现有配置。",
    installs:
      "只有明确点「开始安装」「开始更新」「开始命令行安装」「开始命令行更新」或「下载并打开安装窗口」之后才开始。",
    evidence: "product",
  },
  {
    step: 5,
    asks: "Windows 官方安装窗口里还会再问哪几句，材料没有逐条记下。",
    installs: "窗口打开不等于已经装好。完成窗口里的步骤后，再刷新安装状态。",
    evidence: "pending",
  },
];

export const PENDING_EXTERNAL_STEPS =
  "外部安装器当时具体问了哪几步，材料没有逐条记下，标为待补证。这里不把没记下的问句编成清单。";

export const FREE_LANE_NOTICE =
  "zen-gate 1.8.0 删掉了它自己的免费车道、专用端点和探测。引导不再提供那个入口。暂无免费车道。这里只确认 zen-gate 自己的这次删除。";

export const OPENCODE_UPDATE_NOTICE =
  "OpenCode 不再自动安装更新。要更新时，在 OpenCode 里输入 /update。升到 OpenCode 2 要另作处理。";

export const PARENT_RULE_NOTICE =
  "qwen-code 从 0.25.1 起，仓库上一级目录里的 QWEN.md 和 AGENTS.md 不再被读进去。共用规则请挪进仓库。本产品只使用各个应用自己的提示词文件，父目录规则文件不再当作有效。";

export const WINDOWS_LOGIN_SPIN_HINT =
  "登录窗口如果一直转圈，先留在已经打开的官方窗口里等它完成，然后回到这里刷新状态。窗口关掉了还在转圈，先停止等待，再登录一次。";

export const WINDOWS_ENV_HINT =
  "装完后如果终端提示找不到命令，先完全退出并重新打开终端或 FyAgent。旧窗口往往还记着安装前的环境变量。保持当前用户的安装方式即可，不必改整台电脑的环境变量。";

export const PRERELEASE_EXCLUSION_NOTICE =
  "带 alpha、beta、preview、nightly、rc 标记的预发版不进入推荐安装名单。";

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
