import { describe, expect, it } from "vitest";
import {
  parseCodexInstallPreflight,
  parseCodexStagingSelection,
  type InstallerErrorDto,
} from "@/domain/codex-desktop";
import {
  codexInstallerErrorCopy,
  codexInstallerErrorPresentation,
} from "@/shared/codex-desktop/installerErrorCopy";
import { codexInstallPreflightFixture } from "../fixtures/codexInstallPreflight";

describe("Codex install confirmation", () => {
  it("accepts only the checked release, opaque confirmation and bounded summary", () => {
    const release = `v1:${"a".repeat(64)}`;
    const checked = codexInstallPreflightFixture(release);
    expect(parseCodexInstallPreflight(checked, release)).toEqual(checked);
    for (const invalid of [
      { ...checked, expectedReleaseId: `v1:${"b".repeat(64)}` },
      { ...checked, confirmationId: "/Applications/Codex.app" },
      { ...checked, targetLabel: "bad\nlabel" },
      { ...checked, stagingLabel: "bad\nlabel" },
      { ...checked, customStaging: "yes" },
      { ...checked, availableBytes: 0 },
      {
        ...checked,
        downloadUrl: "https://user:secret@example.test/Codex.msix",
      },
      { ...checked, downloadUrl: "file:///tmp/Codex.msix" },
      { ...checked, bypass: true },
    ])
      expect(() => parseCodexInstallPreflight(invalid, release)).toThrow();
  });

  it("accepts only opaque bounded staging capabilities", () => {
    const selected = {
      contractVersion: 1,
      capabilityId: `s1:${"d".repeat(32)}`,
      label: "用户选择的下载临时目录",
      availableBytes: 8 * 1024 ** 3,
    };
    expect(parseCodexStagingSelection(selected)).toEqual(selected);
    expect(parseCodexStagingSelection(null)).toBeNull();
    for (const invalid of [
      { ...selected, capabilityId: "C:\\Downloads" },
      { ...selected, label: "bad\nlabel" },
      { ...selected, availableBytes: 0 },
      { ...selected, path: "C:\\Downloads" },
    ]) {
      expect(() => parseCodexStagingSelection(invalid)).toThrow();
    }
  });

  it("offers specific recovery without rendering backend diagnostic text", () => {
    const error = {
      code: "MAC_COPY_FAILED",
      details: {
        platformErrorCode: "target_permission_denied",
        redactedMessage: "/Users/private secret",
      },
    } as InstallerErrorDto;
    expect(codexInstallerErrorCopy(error)).toContain("目录权限");
    expect(codexInstallerErrorCopy(error)).not.toContain("private");
    expect(
      codexInstallerErrorCopy({
        ...error,
        code: "INSUFFICIENT_DISK_SPACE",
        details: { ...error.details, platformErrorCode: null },
      }),
    ).toContain("清理");
  });

  it.each([
    ["staging_permission_denied", "文件创建"],
    ["staging_directory_unavailable", "身份安全检查"],
    ["staging_space_unavailable", "磁盘连接"],
    ["staging_insufficient_space", "空间不足"],
  ])("maps %s to staging-specific recovery", (platformErrorCode, copy) => {
    const error = {
      code:
        platformErrorCode === "staging_insufficient_space"
          ? "INSUFFICIENT_DISK_SPACE"
          : "INTERNAL_ERROR",
      details: {
        platformErrorCode,
        redactedMessage: "C:\\private",
      },
    } as InstallerErrorDto;
    expect(codexInstallerErrorCopy(error)).toContain(copy);
    expect(codexInstallerErrorCopy(error)).not.toContain("private");
  });

  it("classifies recovery copy, title and actions from one platform taxonomy", () => {
    const staging = codexInstallerErrorPresentation({
      code: "INTERNAL_ERROR",
      details: {
        platformErrorCode: "staging_cleanup_failed",
        redactedMessage: "private",
      },
    } as InstallerErrorDto);
    expect(staging).toMatchObject({
      title: "下载临时目录不可用",
      recoverable: true,
      offerStagingPicker: true,
    });
    expect(staging.message).toContain("探测文件");

    const target = codexInstallerErrorPresentation({
      code: "INSUFFICIENT_DISK_SPACE",
      details: {
        platformErrorCode: null,
        redactedMessage: "private",
      },
    } as InstallerErrorDto);
    expect(target).toMatchObject({
      title: "磁盘空间不足",
      recoverable: true,
      offerStagingPicker: false,
    });
  });

  it("maps helper and bridge recovery codes to retry copy", () => {
    const base = {
      code: "WINDOWS_DEPLOYMENT_FAILED",
      details: {
        platformErrorCode: "helper_busy",
        redactedMessage: "C:\\ProgramData\\private",
      },
    } as InstallerErrorDto;
    expect(codexInstallerErrorCopy(base)).toContain("稍后重试");
    expect(
      codexInstallerErrorCopy({
        ...base,
        details: {
          ...base.details,
          platformErrorCode: "shell_desktop_unavailable",
        },
      }),
    ).toContain("资源管理器");
    const identity = codexInstallerErrorCopy({
      ...base,
      code: "PACKAGE_IDENTITY_MISMATCH",
      details: { ...base.details, platformErrorCode: null },
    });
    expect(identity).toContain("安全检查");
    expect(identity).toContain("查看日志");
    expect(identity).not.toContain("private");
  });
});
