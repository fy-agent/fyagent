import { describe, expect, it } from "vitest";

import {
  FIRST_INSTALL_STEPS,
  FREE_LANE_NOTICE,
  PARENT_RULE_NOTICE,
  PENDING_EXTERNAL_STEPS,
  isPrereleaseInstallLabel,
  recommendedInstallLabels,
} from "@/domain/first-install-path";

describe("first install path", () => {
  it("states what each recorded step asks and installs", () => {
    expect(FIRST_INSTALL_STEPS.map((step) => step.step)).toEqual([
      1, 2, 3, 4, 5,
    ]);
    for (const step of FIRST_INSTALL_STEPS) {
      expect(step.asks.trim().length).toBeGreaterThan(0);
      expect(step.installs.trim().length).toBeGreaterThan(0);
    }
    expect(FIRST_INSTALL_STEPS[0]?.installs).toContain("不安装");
    expect(FIRST_INSTALL_STEPS[4]?.evidence).toBe("pending");
    expect(PENDING_EXTERNAL_STEPS).toContain("待补证");
  });

  it("does not point the guide at the removed zen-gate free lane", () => {
    expect(FREE_LANE_NOTICE).toContain("暂无免费车道");
    expect(FREE_LANE_NOTICE).toContain("zen-gate");
    expect(FREE_LANE_NOTICE).not.toMatch(/https?:\/\//u);
    expect(PARENT_RULE_NOTICE).toContain("不再被读进去");
    expect(PARENT_RULE_NOTICE).not.toMatch(/https?:\/\//u);
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
