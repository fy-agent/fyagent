import { describe, expect, it } from "vitest";

import {
  WINDOWS_ENV_HINT,
  WINDOWS_LOGIN_SPIN_HINT,
  isPrereleaseInstallLabel,
  recommendedInstallLabels,
} from "@/domain/first-install-path";

describe("first install path", () => {
  it("keeps Windows login and environment recovery hints URL-free", () => {
    expect(WINDOWS_LOGIN_SPIN_HINT).toContain("刷新状态");
    expect(WINDOWS_LOGIN_SPIN_HINT).not.toMatch(/https?:\/\//u);
    expect(WINDOWS_ENV_HINT).toContain("环境变量");
    expect(WINDOWS_ENV_HINT).not.toMatch(/https?:\/\//u);
  });

  it("keeps prerelease labels out of the recommended install list", () => {
    for (const label of [
      "1.2.3-alpha.1",
      "1.2.3-beta.1",
      "0.45.0-nightly.1",
      "2.0.0-preview.2",
      "2.0.0-rc.1",
      "v1.8.0-alpha",
      "nightly",
      "preview",
      "OpenCode 1.18.27-beta",
    ]) {
      expect(isPrereleaseInstallLabel(label), label).toBe(true);
    }
    for (const label of [
      "2.0.20",
      "1.2.4",
      "5.3.14.36279234",
      "官方最新渠道",
      "stable",
    ]) {
      expect(isPrereleaseInstallLabel(label), label).toBe(false);
    }
    expect(
      recommendedInstallLabels([
        "2.0.20",
        "0.45.0-nightly.1",
        "官方最新渠道",
        "1.2.3-preview.1",
        " ",
      ]),
    ).toEqual(["2.0.20", "官方最新渠道"]);
  });
});
