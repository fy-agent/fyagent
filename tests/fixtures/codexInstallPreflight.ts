import type { CodexInstallPreflight } from "@/domain/codex-desktop";

export const confirmationId = `i1:${"c".repeat(32)}`;

export function codexInstallPreflightFixture(
  expectedReleaseId: string,
): CodexInstallPreflight {
  return {
    contractVersion: 1,
    confirmationId,
    expectedReleaseId,
    platform: "windows",
    architecture: "x86_64",
    displayVersion: "1.2.3.4",
    targetLabel: "当前桌面用户的 Windows 应用目录（由系统管理）",
    updating: false,
    availableBytes: 10 * 1024 ** 3,
    downloadSizeHint: 4096,
  };
}
