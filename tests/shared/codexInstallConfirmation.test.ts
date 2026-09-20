import { describe, expect, it } from "vitest";
import {
  parseCodexInstallPreflight,
  type InstallerErrorDto,
} from "@/domain/codex-desktop";
import { codexInstallerErrorCopy } from "@/shared/codex-desktop/installerErrorCopy";
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
      { ...checked, availableBytes: 0 },
      { ...checked, bypass: true },
    ])
      expect(() => parseCodexInstallPreflight(invalid, release)).toThrow();
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
});
