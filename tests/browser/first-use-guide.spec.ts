import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

for (const choice of [
  { label: "日常办公", names: ["QoderWork CN", "TRAE Work CN", "WorkBuddy"] },
  { label: "编程开发", names: ["Codex", "Claude Code", "OpenCode"] },
  { label: "两者都用", names: ["WorkBuddy", "Codex"] },
]) {
  test(`first use recommends ${choice.label} and persists completion`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page, {
      firstUseGuideState: "pending",
    });
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/agents");
    const guide = page.getByRole("region", { name: "首次使用引导" });
    await expect(
      guide.getByRole("heading", { name: "你主要想用 AI 做什么？" }),
    ).toBeFocused();
    await guide
      .getByRole("button", { name: choice.label, exact: true })
      .click();
    await expect(guide.getByRole("heading", { level: 2 })).toHaveText(
      choice.names,
    );
    const calls = await featureFixtureCalls(page);
    expect(calls.map((call) => call.command)).not.toContain(
      "get_agent_install_readiness",
    );
    expect(calls.map((call) => call.command)).not.toContain(
      "start_agent_action",
    );
    expect(calls.map((call) => call.command)).not.toContain("save_settings");
    await guide.getByRole("button", { name: "重新选择" }).click();
    await expect(
      guide.getByRole("heading", { name: "你主要想用 AI 做什么？" }),
    ).toBeFocused();
    await guide
      .getByRole("button", { name: choice.label, exact: true })
      .click();
    await guide.getByRole("button", { name: "查看全部软件" }).click();
    await expect(
      page.getByRole("heading", { name: "我的 AI 软件" }),
    ).toBeFocused();
    await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
    const descriptions = await page
      .locator(".fy-agent-directory-description")
      .allTextContents();
    expect(
      descriptions.every((text) => !/不支持|不安装|暂无法确认/u.test(text)),
    ).toBe(true);
    await page.reload();
    await expect(
      page.getByRole("heading", { name: "我的 AI 软件" }),
    ).toBeVisible();
    await expect(guide).toHaveCount(0);
    await expectHealthyPage(page, health);
  });
}

test("skip works before and after a recommendation, and unfinished onboarding resumes", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { firstUseGuideState: "pending" });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/agents");
  await page.getByRole("button", { name: "日常办公", exact: true }).click();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "你主要想用 AI 做什么？" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "跳过引导" }).click();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "我的 AI 软件" }),
  ).toBeVisible();
  await expectHealthyPage(page, health);
});

for (const theme of ["light", "dark"] as const) {
  test(`first-use guide stays reachable with keyboard and viewport changes in ${theme}`, async ({
    page,
    browserName,
  }, testInfo) => {
    // WebKit's all-controls navigation respects the host keyboard-access policy.
    // Option-Tab reaches buttons even when ordinary Tab skips them on macOS.
    const tabKey = browserName === "webkit" ? "Alt+Tab" : "Tab";
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await installRichTauriFeatureFixture(page, {
      firstUseGuideState: "pending",
    });
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/agents");
    const guide = page.getByRole("region", { name: "首次使用引导" });
    await expect(guide).toBeVisible();
    for (const viewport of [
      { width: 1232, height: 700 },
      { width: 900, height: 600 },
      { width: 1232, height: 700 },
    ]) {
      await page.setViewportSize(viewport);
      await expectNoHorizontalOverflow(page);
      for (const name of ["日常办公", "编程开发", "两者都用", "跳过引导"]) {
        await guide
          .getByRole("button", { name, exact: true })
          .click({ trial: true });
      }
    }
    await page.screenshot({
      path: testInfo.outputPath(`guide-question-${theme}.png`),
    });
    await guide.getByRole("heading", { level: 1 }).focus();
    await page.keyboard.press(tabKey);
    await expect(
      guide.getByRole("button", { name: "日常办公", exact: true }),
    ).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(
      guide.getByRole("heading", { name: "推荐你从这些软件开始" }),
    ).toBeFocused();
    await page.screenshot({
      path: testInfo.outputPath(`guide-recommendations-${theme}.png`),
    });
    await page.keyboard.press(tabKey);
    await expect(guide.getByRole("button", { name: "重新选择" })).toBeFocused();
    await page.keyboard.press(tabKey);
    await page.keyboard.press(tabKey);
    await expect(guide.getByRole("button", { name: "跳过引导" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("heading", { name: "我的 AI 软件" }),
    ).toBeFocused();
    await page.reload();
    await expect(guide).toHaveCount(0);
    await expectHealthyPage(page, health);
  });
}
